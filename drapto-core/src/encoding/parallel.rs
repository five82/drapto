use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use log::{debug, info};
use rayon::prelude::*;

use crate::error::{Result, DraptoError};
use crate::encoding::memory::MemoryTracker;

/// Type for video encoding progress callback
pub type ProgressCallback = Box<dyn Fn(f32) + Send + Sync>;

/// Progress tracking for encoding operations
pub struct EncodingProgress {
    callback: Box<dyn Fn(f32) + Send + Sync>,
}

impl EncodingProgress {
    /// Create a new progress tracker with the given callback
    pub fn new<F>(callback: F) -> Self 
    where 
        F: Fn(f32) + Send + Sync + 'static,
    {
        Self {
            callback: Box::new(callback),
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
        
        // Setup memory tracker
        let memory_tracker = MemoryTracker::new(self.memory_per_job);
        let available_jobs = memory_tracker.max_concurrent_jobs().min(self.max_concurrent_jobs);
        
        info!("Starting parallel encoding with {} concurrent jobs (max allowed: {})",
              available_jobs, self.max_concurrent_jobs);
        
        // Set up thread pool with limited parallelism
        rayon::ThreadPoolBuilder::new()
            .num_threads(available_jobs)
            .build_global()
            .map_err(|e| DraptoError::Other(format!("Failed to initialize thread pool: {}", e)))?;
        
        // Progress tracking
        let total_segments = segments.len();
        let completed = Arc::new(Mutex::new(0usize));
        let results = Arc::new(Mutex::new(Vec::with_capacity(segments.len())));
        
        // Process segments in parallel
        segments.par_iter().try_for_each(|segment| -> Result<()> {
            // Acquire memory for this job
            let _memory_handle = memory_tracker.acquire_memory()?;
            
            // Generate output path
            let segment_name = segment.file_name()
                .ok_or_else(|| DraptoError::InvalidPath("Invalid segment filename".to_string()))?;
                
            let output_path = output_dir.join(segment_name);
            let _temp_file = temp_dir.join(format!("temp_{}", segment_name.to_string_lossy()));
            
            debug!("Encoding segment {:?} to {:?}", segment, output_path);
            
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
            
            // Run the encoding
            self.encoder.encode_video(
                segment, 
                &output_path,
                progress_cb
            )?;
            
            // Track completion
            {
                let mut completed_count = completed.lock().unwrap();
                *completed_count += 1;
                
                let mut results_vec = results.lock().unwrap();
                results_vec.push(output_path);
            }
            
            Ok(())
        })?;
        
        // Get the final results
        let encoded_segments = Arc::try_unwrap(results)
            .map_err(|_| DraptoError::Other("Failed to get encoding results".to_string()))?
            .into_inner()
            .map_err(|e| DraptoError::Other(format!("Mutex error: {}", e)))?;
        
        // Sort the segments by name to maintain order
        let mut sorted_segments = encoded_segments;
        sorted_segments.sort();
        
        info!("Successfully encoded {} segments", sorted_segments.len());
        
        Ok(sorted_segments)
    }
}