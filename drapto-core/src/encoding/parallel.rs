//! Parallel encoding coordination module
//!
//! Responsibilities:
//! - Coordinate parallel encoding of video segments
//! - Track progress across multiple encoding operations
//! - Manage thread-safe access to shared resources
//! - Handle completion and error states of parallel tasks
//! - Optimize CPU utilization during encoding
//!
//! This module leverages Rayon to efficiently distribute encoding
//! tasks across available CPU cores, maximizing throughput.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use log::{info, warn};
use rayon::prelude::*;

use crate::error::{Result, DraptoError};
use crate::encoding::memory::MemoryTracker;

/// Type for video encoding progress callback
pub type ProgressCallback = Box<dyn Fn(f32) + Send + Sync>;

/// Progress tracking for encoding operations
#[derive(Clone)]
pub struct EncodingProgress {
    callback: Arc<dyn Fn(f32) + Send + Sync>,
}

impl EncodingProgress {
    /// Create a new progress tracker with the given callback
    pub fn new<F>(callback: F) -> Self 
    where 
        F: Fn(f32) + Send + Sync + 'static,
    {
        Self {
            callback: Arc::new(callback),
        }
    }
    
    /// Update progress
    pub fn update(&self, progress: f32) {
        (self.callback)(progress);
    }
}

/// Trait for video encoders
pub trait VideoEncoder {
    /// Encode a video file
    fn encode_video(
        &self,
        input: &Path,
        output: &Path,
        progress: Option<EncodingProgress>,
    ) -> Result<()>;
}

/// Configure and run parallel encoding jobs for video segments
pub struct ParallelEncoder {
    encoder: Arc<dyn VideoEncoder + Send + Sync>,
    max_concurrent_jobs: usize,
    memory_per_job: usize,
    on_progress: Option<Arc<dyn Fn(f32, usize, usize) + Send + Sync>>,
}

impl ParallelEncoder {
    /// Create a new parallel encoder
    pub fn new(encoder: Arc<dyn VideoEncoder + Send + Sync>) -> Self {
        Self {
            encoder,
            max_concurrent_jobs: num_cpus::get(),
            memory_per_job: 1024, // Default 1GB per job
            on_progress: None,
        }
    }
    
    /// Set maximum number of concurrent encoding jobs
    pub fn max_concurrent_jobs(mut self, jobs: usize) -> Self {
        self.max_concurrent_jobs = jobs;
        self
    }
    
    /// Set estimated memory required per job in MB
    pub fn memory_per_job(mut self, memory_mb: usize) -> Self {
        self.memory_per_job = memory_mb;
        self
    }
    
    /// Set progress callback function
    pub fn on_progress<F>(mut self, callback: F) -> Self 
    where
        F: Fn(f32, usize, usize) + Send + Sync + 'static,
    {
        self.on_progress = Some(Arc::new(callback));
        self
    }
    
    /// Encode multiple segments in parallel
    pub fn encode_segments<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        segments: &[PathBuf],
        output_dir: P,
        temp_dir: Q,
    ) -> Result<Vec<PathBuf>> {
        if segments.is_empty() {
            return Ok(Vec::new());
        }
        
        let output_dir = output_dir.as_ref().to_path_buf();
        let temp_dir = temp_dir.as_ref().to_path_buf();
        
        // Ensure directories exist
        if !output_dir.exists() {
            std::fs::create_dir_all(&output_dir)?;
        }
        if !temp_dir.exists() {
            std::fs::create_dir_all(&temp_dir)?;
        }
        
        // Setup memory tracker with SVT-AV1-specific requirements
        // AB-AV1 is a tool that uses SVT-AV1 encoder, not a separate encoder
        let encoder_type = "svtav1"; // Always using SVT-AV1 encoder (possibly via AB-AV1 tool)
        let adjusted_memory = if let Some(first_segment) = segments.first() {
            // SVT-AV1 is our encoder (could be used directly or via AB-AV1 tool)
            let encoder_str = "libsvtav1";
            
            // Get resolution category of first segment to estimate memory needs
            if let Ok((category, width)) = crate::encoding::memory::get_resolution_category(first_segment) {
                info!("Encoding video with {} encoder, resolution: {}, width: {}px", 
                      encoder_type, category.as_str(), width);
                      
                // Calculate memory per job based on resolution and encoder
                let adjusted = crate::encoding::memory::calculate_memory_per_job(&category, encoder_str, self.memory_per_job);
                
                // Provide more detailed logging about memory adjustments
                if adjusted != self.memory_per_job {
                    info!("Adjusted memory per job: {}MB → {}MB for {} encoding of {} content", 
                          self.memory_per_job, adjusted, encoder_type, category.as_str());
                }
                
                adjusted
            } else {
                // Default to base memory if resolution detection fails
                info!("Unable to detect resolution, using base memory per job");
                self.memory_per_job
            }
        } else {
            self.memory_per_job
        };
        
        let memory_tracker = MemoryTracker::new(adjusted_memory);
        
        // Determine job count based on memory and configured limit
        let safe_job_count = memory_tracker.current_safe_job_count();
        let available_jobs = safe_job_count.min(self.max_concurrent_jobs);
        
        // Log job configuration as a subsection
        crate::logging::log_subsection("JOB CONFIGURATION");
        info!("");
        info!("Starting parallel encoding with {} concurrent jobs (max configured: {}, memory per job: {}MB, encoder: {})",
              available_jobs, self.max_concurrent_jobs, adjusted_memory, encoder_type);
        
        // Set up thread pool with limited parallelism
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(available_jobs)
            .build()
            .map_err(|e| DraptoError::Other(format!("Failed to initialize thread pool: {}", e)))?;
        
        // Progress tracking
        let total_segments = segments.len();
        let completed = Arc::new(Mutex::new(0usize));
        let results = Arc::new(Mutex::new(Vec::with_capacity(segments.len())));
        
        // Process segments in parallel using our local pool
        pool.install(|| segments.par_iter().try_for_each(|segment| -> Result<()> {
            // Generate segment name for reporting
            let segment_name = segment.file_name()
                .ok_or_else(|| DraptoError::InvalidPath("Invalid segment filename".to_string()))?;
            let segment_name_str = segment_name.to_string_lossy().to_string();
                
            // Acquire memory for this job with segment identifier for better tracking
            let _memory_handle = memory_tracker.acquire_memory()?.with_id(format!("segment {}", segment_name_str));
            
            // Generate output path
            let output_path = output_dir.join(segment_name);
            let _temp_file = temp_dir.join(format!("temp_{}", segment_name_str));
            
            // Create progress callback if needed
            let progress_cb = if let Some(ref cb) = self.on_progress {
                let cb_clone = Arc::clone(cb);
                let completed_clone = Arc::clone(&completed);
                let total = total_segments;
                
                Some(EncodingProgress::new(move |progress: f32| {
                    let completed_count = *completed_clone.lock().unwrap();
                    let overall_progress = (completed_count as f32 + progress) / total as f32;
                    cb_clone(overall_progress, completed_count, total);
                }))
            } else {
                None
            };
            
            // Run the encoding with retry logic
            let max_retries = 2;
            let mut attempt = 0;
            
            loop {
                match self.encoder.encode_video(segment, &output_path, progress_cb.clone()) {
                    Ok(_) => {
                        // Successfully encoded
                        info!("Segment encoding complete: {}", segment_name_str);
                        
                        // Check output file exists and has valid size
                        if let Ok(metadata) = std::fs::metadata(&output_path) {
                            if metadata.len() > 1024 {  // Ensure output is at least 1KB
                                // Track completion
                                {
                                    let mut completed_count = completed.lock().unwrap();
                                    *completed_count += 1;
                                    
                                    let mut results_vec = results.lock().unwrap();
                                    results_vec.push(output_path.clone());
                                }
                                break;
                            } else {
                                warn!("Encoding result for {} is too small ({} bytes), retrying", 
                                      segment_name_str, metadata.len());
                            }
                        } else {
                            warn!("Cannot access output file for {}, retrying", segment_name_str);
                        }
                    },
                    Err(e) => {
                        // Encoding failed
                        warn!("Encoding segment {} failed on attempt {} with error: {}", 
                              segment_name_str, attempt, e);
                    }
                }
                
                // Handle retries or fail permanently
                attempt += 1;
                if attempt >= max_retries {
                    return Err(DraptoError::Other(format!(
                        "Failed to encode segment {} after {} attempts", 
                        segment_name_str, max_retries)));
                }
                
                // Wait before retry to avoid memory contention
                std::thread::sleep(std::time::Duration::from_millis(1000));
                info!("Retrying segment {} (attempt {})", segment_name_str, attempt);
            }
            
            Ok(())
        }))?;
        
        // Get the final results
        let encoded_segments = Arc::try_unwrap(results)
            .map_err(|_| DraptoError::Other("Failed to get encoding results".to_string()))?
            .into_inner()
            .map_err(|e| DraptoError::Other(format!("Mutex error: {}", e)))?;
        
        // Sort the segments by name to maintain order
        let mut sorted_segments = encoded_segments;
        sorted_segments.sort();
        
        crate::logging::log_section("SEGMENT ENCODING COMPLETE");
        info!("Successfully encoded {} segments", sorted_segments.len());
        
        Ok(sorted_segments)
    }
}