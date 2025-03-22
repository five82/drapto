use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::thread;
use std::sync::{Arc, Mutex};
// Silence unused import warnings
#[allow(unused_imports)]
use log::{info};
use tokio::runtime::Runtime;
use sysinfo::System;

/// Token-based memory-aware scheduler for parallel encoding tasks
///
/// This scheduler:
/// 1. Limits parallel encoding tasks based on estimated memory usage
/// 2. Assigns memory tokens to tasks based on their estimated memory requirement
/// 3. Monitors system memory to avoid exhaustion
/// 4. Provides backpressure when memory is constrained
///
/// The scheduler uses a token system where each task requires a certain number of
/// memory tokens. The total number of tokens is limited, preventing memory exhaustion.
pub struct MemoryAwareScheduler {
    /// Base memory per token in bytes
    base_mem_per_token: usize,
    
    /// Maximum number of tokens available
    max_tokens: usize,
    
    /// Delay between task submissions in milliseconds
    task_stagger_delay: u64,
    
    /// Currently running tasks: task_id -> (task_handle, token_weight)
    running_tasks: Arc<Mutex<HashMap<usize, (Arc<Mutex<TaskStatus>>, usize)>>>,
    
    /// Runtime for executing async tasks
    runtime: Runtime,
    
    /// System information for memory monitoring
    system: Arc<Mutex<System>>,
}

/// Status of a task managed by the scheduler
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    Running,
    Completed,
    Failed,
}

/// Detailed task status with result
pub struct TaskStatus {
    /// Current state of the task
    pub state: TaskState,
    
    /// Task result if available
    pub result: Option<Result<(), String>>,
    
    /// Start time
    pub start_time: Instant,
    
    /// End time if completed
    pub end_time: Option<Instant>,
}

impl MemoryAwareScheduler {
    /// Create a new memory-aware scheduler
    ///
    /// # Arguments
    ///
    /// * `base_mem_per_token` - Base memory per token in bytes
    /// * `max_tokens` - Maximum number of tokens available
    /// * `task_stagger_delay` - Delay between task submissions in milliseconds
    ///
    /// # Returns
    ///
    /// A new `MemoryAwareScheduler` instance
    pub fn new(base_mem_per_token: usize, max_tokens: usize, task_stagger_delay: u64) -> Self {
        Self {
            base_mem_per_token,
            max_tokens,
            task_stagger_delay,
            running_tasks: Arc::new(Mutex::new(HashMap::new())),
            runtime: Runtime::new().expect("Failed to create Tokio runtime"),
            system: Arc::new(Mutex::new(System::new_all())),
        }
    }
    
    /// Calculate the current token usage of all running tasks
    ///
    /// # Returns
    ///
    /// The sum of token weights for all running tasks
    pub fn current_token_usage(&self) -> usize {
        let tasks = self.running_tasks.lock().unwrap();
        tasks.values()
            .map(|(_, token_weight)| token_weight)
            .sum()
    }
    
    /// Check if a new task with the given estimated memory can be submitted
    ///
    /// This considers:
    /// 1. Current system memory availability
    /// 2. Current token usage
    /// 3. Task memory requirements
    ///
    /// # Arguments
    ///
    /// * `memory_weight` - Memory weight of the task in tokens
    ///
    /// # Returns
    ///
    /// `true` if the task can be submitted, `false` otherwise
    pub fn can_submit(&self, memory_weight: usize) -> bool {
        // Update system information for accurate memory stats
        {
            let mut system = self.system.lock().unwrap();
            system.refresh_all();
        }
        
        let system = self.system.lock().unwrap();
        let available_memory = system.available_memory() as u64 * 1024; // Convert to bytes
        let total_memory = system.total_memory() as u64 * 1024; // Convert to bytes
        
        // Reserve 20% of total memory
        let target_available = total_memory / 5;
        
        // Calculate current and new memory usage
        let current_usage = (self.current_token_usage() * self.base_mem_per_token) as u64;
        let new_task_memory = (memory_weight * self.base_mem_per_token) as u64;
        
        // Check if we have enough memory available
        if available_memory.saturating_sub(current_usage + new_task_memory) > target_available {
            // Check if we have enough tokens available
            if self.current_token_usage() + memory_weight <= self.max_tokens {
                return true;
            }
        }
        
        false
    }
    
    /// Submit a task to be executed with the given memory weight
    ///
    /// # Arguments
    ///
    /// * `task_id` - Unique identifier for the task
    /// * `func` - Function to execute
    /// * `memory_weight` - Memory weight of the task in tokens
    ///
    /// # Returns
    ///
    /// A handle to the task's status
    pub fn submit_task<F, T>(&self, task_id: usize, func: F, memory_weight: usize) -> Arc<Mutex<TaskStatus>>
    where
        F: FnOnce() -> Result<T, String> + Send + 'static,
        T: Send + 'static,
    {
        // Create task status
        let task_status = Arc::new(Mutex::new(TaskStatus {
            state: TaskState::Running,
            result: None,
            start_time: Instant::now(),
            end_time: None,
        }));
        
        // Clone for move into closure
        let status_clone = task_status.clone();
        let running_tasks = self.running_tasks.clone();
        
        // Spawn the task
        let _handle = self.runtime.spawn(async move {
            // Execute the function and capture result
            let result = func();
            
            // Update task status
            let mut status = status_clone.lock().unwrap();
            match result {
                Ok(_) => {
                    status.state = TaskState::Completed;
                    status.result = Some(Ok(()));
                },
                Err(e) => {
                    status.state = TaskState::Failed;
                    status.result = Some(Err(e));
                }
            }
            status.end_time = Some(Instant::now());
            
            // Remove task from running tasks
            let mut tasks = running_tasks.lock().unwrap();
            tasks.remove(&task_id);
        });
        
        // Register the task
        {
            let mut tasks = self.running_tasks.lock().unwrap();
            tasks.insert(task_id, (task_status.clone(), memory_weight));
        }
        
        // Apply stagger delay
        if self.task_stagger_delay > 0 {
            thread::sleep(Duration::from_millis(self.task_stagger_delay));
        }
        
        task_status
    }
    
    /// Update the status of running tasks and remove completed ones
    ///
    /// # Returns
    ///
    /// The number of completed tasks removed
    pub fn update_completed(&self) -> usize {
        let mut tasks = self.running_tasks.lock().unwrap();
        let before_count = tasks.len();
        
        // Find completed tasks
        let completed_ids: Vec<usize> = tasks.iter()
            .filter(|(_, (status, _))| {
                let status = status.lock().unwrap();
                status.state != TaskState::Running
            })
            .map(|(id, _)| *id)
            .collect();
        
        // Remove completed tasks
        for id in completed_ids.iter() {
            tasks.remove(id);
        }
        
        before_count - tasks.len()
    }
    
    /// Wait for all running tasks to complete
    ///
    /// This will block until all tasks are completed
    pub fn wait_for_all(&self) {
        while !self.running_tasks.lock().unwrap().is_empty() {
            self.update_completed();
            thread::sleep(Duration::from_millis(100));
        }
    }
    
    /// Get the number of currently running tasks
    ///
    /// # Returns
    ///
    /// The number of running tasks
    pub fn running_task_count(&self) -> usize {
        self.running_tasks.lock().unwrap().len()
    }
    
    /// Get the available memory in bytes
    ///
    /// # Returns
    ///
    /// Available physical memory in bytes
    pub fn available_memory(&self) -> u64 {
        let mut system = self.system.lock().unwrap();
        system.refresh_all();
        system.available_memory() as u64 * 1024 // Convert to bytes
    }
    
    /// Get the memory usage percentage
    ///
    /// # Returns
    ///
    /// Memory usage as a percentage (0-100)
    pub fn memory_usage_percent(&self) -> f32 {
        let mut system = self.system.lock().unwrap();
        system.refresh_all();
        let used = system.used_memory() as f32;
        let total = system.total_memory() as f32;
        (used / total) * 100.0
    }
}

/// Builder for dynamically sizing a MemoryAwareScheduler based on system capabilities
pub struct SchedulerBuilder {
    /// Base memory per token in bytes (default: 512 MB)
    base_mem_per_token: usize,
    
    /// Maximum memory tokens (default: based on system memory)
    max_tokens: Option<usize>,
    
    /// Stagger delay in milliseconds (default: 250ms)
    task_stagger_delay: u64,
}

impl Default for SchedulerBuilder {
    fn default() -> Self {
        Self {
            base_mem_per_token: 512 * 1024 * 1024, // 512 MB default
            max_tokens: None,                      // Auto-calculate based on system
            task_stagger_delay: 250,               // 250ms default
        }
    }
}

impl SchedulerBuilder {
    /// Create a new scheduler builder with default values
    pub fn new() -> Self {
        Self::default()
    }
    
    /// Set the base memory per token
    pub fn base_mem_per_token(mut self, bytes: usize) -> Self {
        self.base_mem_per_token = bytes;
        self
    }
    
    /// Set the maximum number of tokens
    pub fn max_tokens(mut self, tokens: usize) -> Self {
        self.max_tokens = Some(tokens);
        self
    }
    
    /// Set the task stagger delay
    pub fn task_stagger_delay(mut self, milliseconds: u64) -> Self {
        self.task_stagger_delay = milliseconds;
        self
    }
    
    /// Build the scheduler based on current specifications
    pub fn build(self) -> MemoryAwareScheduler {
        // Calculate max tokens if not specified
        let max_tokens = self.max_tokens.unwrap_or_else(|| {
            let mut system = System::new();
            system.refresh_all();
            let total_memory = system.total_memory() as usize * 1024; // Convert to bytes
            
            // Prevent division by zero
            let available_tokens = if self.base_mem_per_token == 0 {
                8 // Default to 8 tokens if base_mem_per_token is 0
            } else {
                total_memory / self.base_mem_per_token
            };
            
            // Use 60% of available memory for encoding tasks
            let token_limit = (available_tokens as f32 * 0.6) as usize;
            
            // Ensure at least 1 token, at most 16 tokens
            token_limit.clamp(1, 16)
        });
        
        MemoryAwareScheduler::new(
            self.base_mem_per_token,
            max_tokens,
            self.task_stagger_delay
        )
    }
}

/// Calculate memory requirements based on warmup results
///
/// This function analyzes the memory usage of warmup encoding tasks and
/// determines appropriate token weights for different video resolutions.
///
/// # Arguments
///
/// * `warmup_results` - Results from warmup encoding tasks, including memory usage
///
/// # Returns
///
/// A tuple containing:
/// * Base memory size per token in bytes
/// * HashMap of resolution category to weight values
pub fn calculate_memory_requirements<T, E>(
    warmup_results: &[(Result<T, E>, usize, String)],
) -> (usize, HashMap<String, usize>) 
where 
    E: std::fmt::Display
{
    let mut memory_by_resolution: HashMap<String, Vec<usize>> = HashMap::new();
    
    // Collect memory usage by resolution category
    for (_, peak_memory, resolution_category) in warmup_results {
        memory_by_resolution
            .entry(resolution_category.clone())
            .or_insert_with(Vec::new)
            .push(*peak_memory);
    }
    
    // Calculate averages for each resolution category
    let mut averages: HashMap<String, usize> = HashMap::new();
    for (category, values) in &memory_by_resolution {
        if !values.is_empty() {
            let sum: usize = values.iter().sum();
            averages.insert(category.clone(), sum / values.len());
        }
    }
    
    // Calculate peak memory usage during warmup
    let max_peak = warmup_results.iter()
        .map(|(_, peak_memory, _)| *peak_memory)
        .max()
        .unwrap_or(512 * 1024 * 1024); // Default to 512MB if no data
    
    // Determine base token size
    let base_size = if let Some(min_average) = averages.values().min().copied() {
        // Use the larger of minimum average or peak/4
        std::cmp::max(min_average, max_peak / 4)
    } else {
        // Fallback to 512MB
        512 * 1024 * 1024
    };
    
    // Calculate relative weights
    let mut weights = HashMap::new();
    weights.insert("SDR".to_string(), 1); // Base weight
    
    // 1080p weight
    let weight_1080p = if let Some(avg_1080p) = averages.get("1080p") {
        std::cmp::max(1, avg_1080p / base_size)
    } else {
        2 // Default if no data
    };
    weights.insert("1080p".to_string(), weight_1080p);
    
    // 4K weight
    let weight_4k = if let Some(avg_4k) = averages.get("4k") {
        std::cmp::max(2, avg_4k / base_size)
    } else {
        4 // Default if no data
    };
    weights.insert("4k".to_string(), weight_4k);
    
    (base_size, weights)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    
    #[test]
    fn test_scheduler_token_usage() {
        let scheduler = MemoryAwareScheduler::new(100_000_000, 8, 10);
        assert_eq!(scheduler.current_token_usage(), 0);
        
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = counter.clone();
        
        // Submit a task with weight 2
        let _status = scheduler.submit_task(1, move || {
            counter_clone.fetch_add(1, Ordering::SeqCst);
            thread::sleep(Duration::from_millis(50));
            Ok(())
        }, 2);
        
        assert_eq!(scheduler.current_token_usage(), 2);
        
        scheduler.wait_for_all();
        assert_eq!(counter.load(Ordering::SeqCst), 1);
        assert_eq!(scheduler.current_token_usage(), 0);
    }
    
    #[test]
    fn test_calculate_memory_requirements() {
        // Mock warmup results with memory usage in bytes
        let warmup_results = vec![
            (Ok(()) as Result<(), &str>, 300_000_000, "SDR".to_string()),       // 300 MB for SD
            (Ok(()) as Result<(), &str>, 320_000_000, "SDR".to_string()),       // 320 MB for SD
            (Ok(()) as Result<(), &str>, 650_000_000, "1080p".to_string()),     // 650 MB for 1080p
            (Ok(()) as Result<(), &str>, 700_000_000, "1080p".to_string()),     // 700 MB for 1080p
            (Ok(()) as Result<(), &str>, 1_200_000_000, "4k".to_string()),      // 1.2 GB for 4K
        ];
        
        let (base_size, weights) = calculate_memory_requirements(&warmup_results);
        
        // The base size should be close to the SD average
        assert!(base_size >= 300_000_000, "Base size should be at least 300 MB");
        
        // Weights should be proportional
        assert_eq!(*weights.get("SDR").unwrap(), 1, "SDR weight should be 1");
        assert!(*weights.get("1080p").unwrap() >= 2, "1080p weight should be at least 2");
        assert!(*weights.get("4k").unwrap() >= 3, "4K weight should be at least 3");
    }
    
    #[test]
    fn test_builder_default_values() {
        let builder = SchedulerBuilder::default();
        assert_eq!(builder.base_mem_per_token, 512 * 1024 * 1024);
        assert_eq!(builder.task_stagger_delay, 250);
    }
}