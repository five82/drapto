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
            Self::SD => "SD",  // Changed from "SDR" for consistency
            Self::HD => "HD",  // Changed from "1080p" for consistency
            Self::UHD => "UHD", // Changed from "4k" for consistency
        }
    }
    
    /// Convert string to resolution category
    pub fn from_str(s: &str) -> Self {
        match s {
            "UHD" | "4k" | "4K" => Self::UHD,
            "HD" | "1080p" => Self::HD,
            // Default to SD for all other values
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
    weights.insert("SD".to_string(), 1);
    weights.insert("HD".to_string(), 2);
    weights.insert("UHD".to_string(), 4);
    weights
}

/// Calculate per-job memory requirement based on encoder and resolution
/// 
/// This is used to determine how much memory to allocate for each encoding job
/// based on the specific encoder and resolution category.
pub fn calculate_memory_per_job(
    resolution_category: &ResolutionCategory,
    encoder_name: &str,
    base_memory_mb: usize,
) -> usize {
    // Get system memory to scale memory requirements for different system sizes
    let mut system = sysinfo::System::new_all();
    system.refresh_memory();
    let system_memory_mb = system.total_memory() / 1024 / 1024; // Convert to MB
    
    // Optimize values specifically for SVT-AV1 which is used by AB-AV1 tool
    // AB-AV1 is a tool that uses SVT-AV1 as its encoder
    // Set encoder-specific memory multipliers based on system size
    let encoder_multiplier = match encoder_name.to_lowercase().as_str() {
        // SVT-AV1 is memory intensive (also used by AB-AV1)
        // Scale multiplier with system memory
        "svtav1" | "libsvtav1" | "abav1" | "libabav1" | "ab-av1" => {
            if system_memory_mb < 8 * 1024 {
                // Small systems need much smaller per-job allocation
                1.5
            } else if system_memory_mb < 16 * 1024 {
                // Medium systems need moderate allocation
                2.0
            } else {
                // Larger systems can use larger allocation
                2.5
            }
        },
        // Default to same scaling since we always use SVT-AV1
        _ => {
            if system_memory_mb < 8 * 1024 {
                1.5
            } else if system_memory_mb < 16 * 1024 {
                2.0
            } else {
                2.5
            }
        },
    };
    
    // Apply resolution category multiplier, also scaled by system size
    let resolution_multiplier = match resolution_category {
        ResolutionCategory::UHD => {
            if system_memory_mb < 8 * 1024 {
                // Small systems - reduce UHD weight
                1.5
            } else if system_memory_mb < 16 * 1024 {
                // Medium systems - moderate UHD weight
                1.8
            } else {
                // Larger systems - full UHD weight
                2.0
            }
        },
        ResolutionCategory::HD => 1.0,
        ResolutionCategory::SD => 0.5,
    };
    
    // Calculate final memory requirement with upper limit that scales with system size
    let max_job_memory = if system_memory_mb < 8 * 1024 {
        // Small systems - cap per-job memory to avoid exhaustion
        4 * 1024 // 4GB max on small systems
    } else if system_memory_mb < 16 * 1024 {
        // Medium systems - moderate cap
        8 * 1024 // 8GB max on medium systems
    } else if system_memory_mb < 32 * 1024 {
        // Large systems - higher cap
        12 * 1024 // 12GB max on large systems
    } else {
        // Very large systems - highest cap
        16 * 1024 // 16GB max on very large systems
    };
    
    // Apply calculation with min/max limits
    let calculated = ((base_memory_mb as f64) * encoder_multiplier * resolution_multiplier) as usize;
    calculated.max(1024).min(max_job_memory)
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
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();
        let system_memory = system.total_memory() / 1024 / 1024; // Convert to MB
        
        // Protect against very small values
        let memory_per_job = memory_per_job.max(512); // Minimum 512MB per job
        
        // Adjust memory allocation based on total system memory
        // Use more conservative allocation on systems with less memory
        let memory_percentage = if system_memory < 8 * 1024 {
            // Less than 8GB - be very conservative (50%)
            0.5
        } else if system_memory < 16 * 1024 {
            // 8-16GB - conservative (60%) 
            0.6
        } else if system_memory < 32 * 1024 {
            // 16-32GB - moderate (65%)
            0.65
        } else {
            // 32GB+ - can be more generous (70%)
            0.7
        };
        
        // Calculate available memory based on system size
        let available_memory = (system_memory as f64 * memory_percentage) as usize;
        info!("Memory allocation: Using {}% of system memory based on system size", 
              (memory_percentage * 100.0) as usize);
        
        // Calculate max jobs but ensure we have at least 1
        let max_memory = (available_memory / memory_per_job).max(1);
        
        // Since we're always using the memory-intensive SVT-AV1 encoder, apply aggressive limits
        // Either directly or via the AB-AV1 tool
        
        // Determine minimum job count based on system memory
        let min_jobs = if system_memory < 8 * 1024 {
            // Small systems (< 8GB) - can only safely run 1 job
            1
        } else if system_memory < 16 * 1024 {
            // Medium systems (8-16GB) - at least 2 jobs
            2
        } else {
            // Larger systems (16GB+) - at least 3 jobs for better performance
            3
        };
        
        // Apply a more balanced limit for our AV1 encoding, scaling with system size
        let adjusted_max = if max_memory > min_jobs + 1 {
            // Moderate limit for memory-intensive encoding
            let scaling_factor = if system_memory < 16 * 1024 {
                // Smaller systems need more conservative scaling
                0.7
            } else {
                // Larger systems can scale more aggressively
                0.8
            };
            
            let reduced = (max_memory as f64 * scaling_factor) as usize;
            info!("SVT-AV1 encoder detected, balancing job count from {} to {}", 
                  max_memory, reduced.max(min_jobs));
            reduced.max(min_jobs) // Ensure minimum jobs based on system size
        } else {
            // If max_memory is already close to or below our minimum, use it
            max_memory.max(1) // Always ensure at least 1 job
        };
        
        info!("Memory tracker: System memory: {}MB, Available: {}MB, Memory per job: {}MB, Max concurrent jobs: {} (SVT-AV1 encoder)",
              system_memory, available_memory, memory_per_job, adjusted_max);
        
        Self {
            system_memory_mb: system_memory as usize,
            memory_per_job,
            used_memory: Arc::new(AtomicUsize::new(0)),
            max_memory: adjusted_max,
        }
    }
    
    /// Get the maximum number of concurrent jobs based on available memory
    pub fn max_concurrent_jobs(&self) -> usize {
        self.max_memory
    }
    
    /// Check current system memory and return safe job count
    ///
    /// This dynamically adjusts the allowed job count based on real-time 
    /// system memory availability to prevent memory exhaustion.
    pub fn current_safe_job_count(&self) -> usize {
        // Get current system memory state
        let mut system = sysinfo::System::new_all();
        system.refresh_memory();
        
        let total_memory_mb = system.total_memory() / 1024 / 1024; // Convert to MB
        let available_memory_mb = system.available_memory() / 1024 / 1024; // Convert to MB
        let used_percent = 100.0 - (available_memory_mb as f64 / total_memory_mb as f64 * 100.0);
        
        // Always using the memory-intensive SVT-AV1 encoder (either directly or via AB-AV1)
        
        // Set thresholds based on system memory size
        let (critical, high, moderate) = if total_memory_mb < 8 * 1024 {
            // Smaller systems need more aggressive thresholds
            (85.0, 75.0, 65.0)
        } else if total_memory_mb < 16 * 1024 {
            // Medium systems need moderate thresholds
            (87.0, 77.0, 67.0)
        } else {
            // Larger systems can use more balanced thresholds
            (90.0, 80.0, 70.0)
        };
        
        // Determine minimum job count based on system size
        let min_jobs = if total_memory_mb < 8 * 1024 {
            // Small systems - prefer reliability over parallelism
            1
        } else if total_memory_mb < 16 * 1024 {
            // Medium systems - need some parallelism
            2
        } else {
            // Larger systems - need good parallelism
            2
        };
        
        // Calculate safe job count based on memory pressure
        let safe_count = if used_percent > critical {
            // Critical memory pressure - minimum jobs
            min_jobs.min(1) // Force to 1 in critical conditions
        } else if used_percent > high {
            // High memory pressure - conservative scaling
            let count = if total_memory_mb < 16 * 1024 {
                (self.max_memory / 4).max(min_jobs)
            } else {
                (self.max_memory / 3).max(min_jobs)
            };
            
            // For smaller systems at high pressure, further limit
            if total_memory_mb < 8 * 1024 && count > 1 {
                1 // Limit to 1 job on small systems at high pressure
            } else {
                count
            }
        } else if used_percent > moderate {
            // Moderate memory pressure - balanced approach
            if total_memory_mb < 8 * 1024 {
                (self.max_memory / 3).max(min_jobs)
            } else {
                (self.max_memory / 2).max(min_jobs)
            }
        } else {
            // Normal memory pressure - scale with system size
            let scaling_factor = if total_memory_mb < 8 * 1024 {
                0.7 // Small systems - conservative scaling
            } else if total_memory_mb < 16 * 1024 {
                0.8 // Medium systems - moderate scaling
            } else {
                0.9 // Large systems - aggressive scaling
            };
            
            let limit = (self.max_memory as f64 * scaling_factor) as usize;
            limit.max(min_jobs)
        };
        
        info!("Memory status: Total: {}MB, Available: {}MB, Used: {:.1}%, Safe job count: {}/{} (SVT-AV1)",
              total_memory_mb, available_memory_mb, used_percent, safe_count, self.max_memory);
        
        safe_count
    }
    
    /// Acquire memory units for a job
    pub fn acquire_memory(&self) -> Result<MemoryHandle> {
        // Try to acquire memory
        loop {
            // Check current system memory pressure
            let safe_job_count = self.current_safe_job_count();
            let current = self.used_memory.load(Ordering::SeqCst);
            
            if current >= safe_job_count {
                // No memory available, sleep and retry
                debug!("Waiting for memory to become available. Used: {}/{} (safe limit: {})", 
                    current, self.max_memory, safe_job_count);
                std::thread::sleep(std::time::Duration::from_millis(500));
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
                debug!("Acquired memory. Now using: {}/{} (safe limit: {})", 
                    current + 1, self.max_memory, safe_job_count);
                return Ok(MemoryHandle {
                    tracker: Arc::clone(&self.used_memory),
                    id: None,
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
    /// Optional identifier for better logging
    id: Option<String>,
}

impl MemoryHandle {
    /// Set an identifier for this memory handle (for logging)
    pub fn with_id<S: Into<String>>(mut self, id: S) -> Self {
        self.id = Some(id.into());
        self
    }
}

impl Drop for MemoryHandle {
    fn drop(&mut self) {
        let prev = self.tracker.fetch_sub(1, Ordering::SeqCst);
        
        if let Some(id) = &self.id {
            debug!("Released memory for {}. Now using: {}", id, prev - 1);
        } else {
            debug!("Released memory. Now using: {}", prev - 1);
        }
        
        // Allow some time for memory to be properly released
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_resolution_category_conversion() {
        assert_eq!(ResolutionCategory::SD.as_str(), "SD");
        assert_eq!(ResolutionCategory::HD.as_str(), "HD");
        assert_eq!(ResolutionCategory::UHD.as_str(), "UHD");
        
        // Test primary names
        assert_eq!(ResolutionCategory::from_str("SD"), ResolutionCategory::SD);
        assert_eq!(ResolutionCategory::from_str("HD"), ResolutionCategory::HD);
        assert_eq!(ResolutionCategory::from_str("UHD"), ResolutionCategory::UHD);
        
        // Test legacy names
        assert_eq!(ResolutionCategory::from_str("SDR"), ResolutionCategory::SD);
        assert_eq!(ResolutionCategory::from_str("1080p"), ResolutionCategory::HD);
        assert_eq!(ResolutionCategory::from_str("4k"), ResolutionCategory::UHD);
        
        // Test fallback
        assert_eq!(ResolutionCategory::from_str("unknown"), ResolutionCategory::SD);
    }
    
    #[test]
    fn test_default_memory_weights() {
        let weights = default_memory_weights();
        
        assert_eq!(*weights.get("SD").unwrap(), 1);
        assert_eq!(*weights.get("HD").unwrap(), 2);
        assert_eq!(*weights.get("UHD").unwrap(), 4);
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