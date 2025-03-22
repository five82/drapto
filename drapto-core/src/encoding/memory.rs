use std::path::Path;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
#[allow(unused_imports)]
use log::{info, debug};

use crate::error::Result;
use crate::media::MediaInfo;

/// Resolution categories for memory weight calculation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolutionCategory {
    SD,
    HD,
    UHD,
}

impl ResolutionCategory {
    /// Convert resolution category to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::SD => "SDR",
            Self::HD => "1080p",
            Self::UHD => "4k",
        }
    }
    
    /// Convert string to resolution category
    pub fn from_str(s: &str) -> Self {
        match s {
            "4k" => Self::UHD,
            "1080p" => Self::HD,
            _ => Self::SD,
        }
    }
}

/// Determine the resolution category of a video file
///
/// # Arguments
///
/// * `path` - Path to the video file
///
/// # Returns
///
/// * Resolution category and width of the video
pub fn get_resolution_category<P: AsRef<Path>>(path: P) -> Result<(ResolutionCategory, u32)> {
    let info = MediaInfo::from_path(path)?;
    
    let (width, _height) = info.video_dimensions()
        .unwrap_or((1280, 720)); // Default to 720p if dimensions unavailable
    
    let category = if width >= 3840 {
        ResolutionCategory::UHD
    } else if width >= 1920 {
        ResolutionCategory::HD
    } else {
        ResolutionCategory::SD
    };
    
    Ok((category, width))
}

/// Estimate memory weight for encoding a video segment
///
/// # Arguments
///
/// * `path` - Path to the video segment
/// * `resolution_weights` - Map of resolution categories to memory weights
///
/// # Returns
///
/// * Memory weight for the segment (number of tokens needed)
pub fn estimate_memory_weight<P: AsRef<Path>>(
    path: P,
    resolution_weights: &HashMap<String, usize>,
) -> Result<usize> {
    let (category, _) = get_resolution_category(&path)?;
    
    let weight = resolution_weights
        .get(category.as_str())
        .copied()
        .unwrap_or_else(|| {
            // Default weights if not specified
            match category {
                ResolutionCategory::UHD => 4,
                ResolutionCategory::HD => 2,
                ResolutionCategory::SD => 1,
            }
        });
    
    Ok(weight)
}

/// Default memory weights for different resolutions
pub fn default_memory_weights() -> HashMap<String, usize> {
    let mut weights = HashMap::new();
    weights.insert("SDR".to_string(), 1);
    weights.insert("1080p".to_string(), 2);
    weights.insert("4k".to_string(), 4);
    weights
}

/// Memory management for parallel encoding jobs
pub struct MemoryTracker {
    /// Total system memory in MB
    system_memory_mb: usize,
    /// Memory required per job in MB
    memory_per_job: usize,
    /// Number of memory units currently in use
    used_memory: Arc<AtomicUsize>,
    /// Maximum memory units allowed
    max_memory: usize,
}

impl MemoryTracker {
    /// Create a new memory tracker
    pub fn new(memory_per_job: usize) -> Self {
        // Get system memory
        let system_memory = sysinfo::System::new_all()
            .total_memory() / 1024 / 1024; // Convert to MB
            
        // Protect against very small values
        let memory_per_job = memory_per_job.max(256); // Minimum 256MB per job
        
        // Reserve 25% of system memory for OS and other processes
        let available_memory = (system_memory as f64 * 0.75) as usize;
        let max_memory = available_memory / memory_per_job;
        
        info!("Memory tracker: System memory: {}MB, Available: {}MB, Memory per job: {}MB, Max concurrent jobs: {}",
              system_memory, available_memory, memory_per_job, max_memory);
        
        Self {
            system_memory_mb: system_memory as usize,
            memory_per_job,
            used_memory: Arc::new(AtomicUsize::new(0)),
            max_memory,
        }
    }
    
    /// Get the maximum number of concurrent jobs based on available memory
    pub fn max_concurrent_jobs(&self) -> usize {
        self.max_memory
    }
    
    /// Acquire memory units for a job
    pub fn acquire_memory(&self) -> Result<MemoryHandle> {
        // Try to acquire memory
        loop {
            let current = self.used_memory.load(Ordering::SeqCst);
            if current >= self.max_memory {
                // No memory available, sleep and retry
                debug!("Waiting for memory to become available. Used: {}/{}", current, self.max_memory);
                std::thread::sleep(std::time::Duration::from_millis(100));
                continue;
            }
            
            // Try to update the counter atomically
            if self.used_memory.compare_exchange(
                current, 
                current + 1,
                Ordering::SeqCst,
                Ordering::SeqCst
            ).is_ok() {
                // Successfully acquired memory
                debug!("Acquired memory. Now using: {}/{}", current + 1, self.max_memory);
                return Ok(MemoryHandle {
                    tracker: Arc::clone(&self.used_memory),
                });
            }
            
            // If compare_exchange failed, someone else modified the counter
            // Try again
        }
    }
    
    /// Get the current memory usage (number of units in use)
    pub fn current_usage(&self) -> usize {
        self.used_memory.load(Ordering::SeqCst)
    }
    
    /// Get the total system memory in MB
    pub fn system_memory_mb(&self) -> usize {
        self.system_memory_mb
    }
    
    /// Get the memory required per job in MB
    pub fn memory_per_job_mb(&self) -> usize {
        self.memory_per_job
    }
}

/// RAII handle for memory allocation
pub struct MemoryHandle {
    tracker: Arc<AtomicUsize>,
}

impl Drop for MemoryHandle {
    fn drop(&mut self) {
        let prev = self.tracker.fetch_sub(1, Ordering::SeqCst);
        debug!("Released memory. Now using: {}/{}", prev - 1, "?");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolution_category_conversion() {
        assert_eq!(ResolutionCategory::SD.as_str(), "SDR");
        assert_eq!(ResolutionCategory::HD.as_str(), "1080p");
        assert_eq!(ResolutionCategory::UHD.as_str(), "4k");
        
        assert_eq!(ResolutionCategory::from_str("SDR"), ResolutionCategory::SD);
        assert_eq!(ResolutionCategory::from_str("1080p"), ResolutionCategory::HD);
        assert_eq!(ResolutionCategory::from_str("4k"), ResolutionCategory::UHD);
        assert_eq!(ResolutionCategory::from_str("unknown"), ResolutionCategory::SD);
    }
    
    #[test]
    fn test_default_memory_weights() {
        let weights = default_memory_weights();
        
        assert_eq!(*weights.get("SDR").unwrap(), 1);
        assert_eq!(*weights.get("1080p").unwrap(), 2);
        assert_eq!(*weights.get("4k").unwrap(), 4);
    }
    
    #[test]
    fn test_memory_tracker() {
        let tracker = MemoryTracker::new(1024);
        
        // Verify initial state
        assert_eq!(tracker.current_usage(), 0);
        assert!(tracker.max_concurrent_jobs() > 0);
        
        // Acquire memory
        let handle1 = tracker.acquire_memory().unwrap();
        assert_eq!(tracker.current_usage(), 1);
        
        // Acquire more memory
        let handle2 = tracker.acquire_memory().unwrap();
        assert_eq!(tracker.current_usage(), 2);
        
        // Release memory by dropping handles
        drop(handle1);
        assert_eq!(tracker.current_usage(), 1);
        
        drop(handle2);
        assert_eq!(tracker.current_usage(), 0);
    }
}