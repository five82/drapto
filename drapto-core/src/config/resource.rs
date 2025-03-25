//! Resource management configuration module
//!
//! Defines the configuration structure for parallel processing,
//! memory management, and system resource allocation.

use serde::{Deserialize, Serialize};
use super::utils::*;

/// Resource management configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceConfig {
    //
    // Parallelism settings
    //
    
    /// Number of parallel encoding jobs
    pub parallel_jobs: usize,
    
    /// Task stagger delay in seconds
    pub task_stagger_delay: f32,
    
    //
    // Memory management
    //

    /// Memory threshold as a fraction of total system memory
    pub memory_threshold: f32,

    /// Maximum memory tokens for concurrent operations
    pub max_memory_tokens: usize,
    
    /// Memory limit per encoding job in MB (0 = auto)
    pub memory_per_job: usize,
    
    //
    // Memory allocation settings
    //
    
    /// Reserve percentage of system memory (0.0-1.0)
    pub memory_reserve_percent: f32,
    
    /// Default memory token size in MB
    pub memory_token_size: usize,
    
    /// Memory allocation percentage of available memory (0.0-1.0)
    pub memory_allocation_percent: f32,
    
    /// Minimum allowed memory tokens
    pub min_memory_tokens: usize,
    
    /// Maximum allowed memory tokens
    pub max_memory_tokens_limit: usize,
}

impl Default for ResourceConfig {
    fn default() -> Self {
        Self {
            // Parallelism settings
            
            // Number of parallel encoding jobs
            // Controls how many encoding tasks can run simultaneously
            // Default: Number of CPU cores on the system
            parallel_jobs: get_env_usize("DRAPTO_PARALLEL_JOBS", num_cpus::get()),
            
            // Task stagger delay in seconds
            // Time to wait between starting consecutive tasks to prevent resource spikes
            // Default: 0.2 seconds - Helps prevent CPU/memory spikes
            task_stagger_delay: get_env_f32("DRAPTO_TASK_STAGGER_DELAY", 0.2),
            
            // Memory management
            
            // Memory threshold as a fraction of total system memory
            // Maximum percentage of system memory that can be used (0.0-1.0)
            // Default: 0.7 (70%) - Leaves some memory for other processes
            memory_threshold: get_env_f32("DRAPTO_MEMORY_THRESHOLD", 0.7),
            
            // Maximum memory tokens for concurrent operations
            // Used for token-based memory management and throttling
            // Default: 8 tokens - Good balance for most systems
            max_memory_tokens: get_env_usize("DRAPTO_MAX_MEMORY_TOKENS", 8),
            
            // Memory limit per encoding job in MB (0 = auto)
            // Specifies how much memory each job is allowed to use
            // Default: 2048 MB (2 GB) per job
            memory_per_job: get_env_usize("DRAPTO_MEMORY_PER_JOB", 2048),
            
            // Memory allocation settings
            
            // Reserve percentage of system memory (0.0-1.0)
            // Percentage of system memory to reserve for other processes
            // Default: 0.2 (20%) - Keeps some memory free for system stability
            memory_reserve_percent: get_env_f32("DRAPTO_MEMORY_RESERVE_PERCENT", 0.2),
            
            // Default memory token size in MB
            // Size of each memory token for allocation calculations
            // Default: 512 MB - Good balance between granularity and overhead
            memory_token_size: get_env_usize("DRAPTO_MEMORY_TOKEN_SIZE", 512),
            
            // Memory allocation percentage of available memory (0.0-1.0)
            // Percentage of available memory (after reserve) to allocate for encoding
            // Default: 0.6 (60%) - Conservative allocation to prevent memory pressure
            memory_allocation_percent: get_env_f32("DRAPTO_MEMORY_ALLOCATION_PERCENT", 0.6),
            
            // Minimum allowed memory tokens
            // Ensures at least this many tokens are available for encoding
            // Default: 1 token - Always allow at least one encoding job
            min_memory_tokens: get_env_usize("DRAPTO_MIN_MEMORY_TOKENS", 1),
            
            // Maximum allowed memory tokens
            // Upper limit on memory tokens to prevent excessive allocation
            // Default: 16 tokens - Prevents overcommitting memory on large systems
            max_memory_tokens_limit: get_env_usize("DRAPTO_MAX_MEMORY_TOKENS_LIMIT", 16),
        }
    }
}