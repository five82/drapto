use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use log::{info, warn, error};
use env_logger::Env;

use drapto_core::encoding::memory::MemoryTracker;
use drapto_core::encoding::parallel::{ParallelEncoder, EncodingProgress, VideoEncoder};
use drapto_core::error::Result;

// Simple video encoder implementation for testing
struct TestEncoder;

impl VideoEncoder for TestEncoder {
    fn encode_video(
        &self,
        input: &std::path::Path,
        output: &std::path::Path,
        progress: Option<EncodingProgress>,
    ) -> Result<()> {
        // Simulate encoding operation
        info!("Encoding {:?} to {:?}", input, output);
        
        // Use segment number to determine memory usage simulation
        let segment_name = input.file_name().unwrap().to_string_lossy();
        let segment_num = segment_name.parse::<u32>().unwrap_or(1);
        
        // Simulate different memory usage based on segment number
        let memory_usage_mb = segment_num * 100; // Each segment uses N*100MB for demo
        
        // Simulate memory allocation by sleeping according to segment number
        let total_steps = 10;
        let total_time = segment_num as u64 * 500; // segments take different times
        
        for i in 0..total_steps {
            // Report progress if callback provided
            if let Some(ref cb) = progress {
                cb.update(i as f32 / total_steps as f32);
            }
            
            // Simulate memory usage spike for segment 4 and 6 (error cases)
            if (segment_num == 4 || segment_num == 6) && i == 5 {
                info!("Segment {} simulating high memory usage ({}MB)", segment_num, memory_usage_mb * 10);
                std::thread::sleep(std::time::Duration::from_millis(100)); 
            }
            
            std::thread::sleep(std::time::Duration::from_millis(total_time / total_steps));
        }
        
        // Create output file with enough content to pass size check (>1KB)
        let content = format!("Test encoded segment {}\n", segment_name).repeat(500);
        std::fs::write(output, content)?;
        
        Ok(())
    }
}

fn main() -> Result<()> {
    // Initialize logger
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    
    info!("Testing parallel encoding with memory tracking");
    
    // Create test segments
    let temp_dir = std::env::temp_dir().join("drapto_test");
    let input_dir = temp_dir.join("input");
    let output_dir = temp_dir.join("output");
    
    // Create directories
    std::fs::create_dir_all(&input_dir)?;
    std::fs::create_dir_all(&output_dir)?;
    
    // Create test segment files
    let mut segments = Vec::new();
    for i in 1..=10 {
        let segment_path = input_dir.join(format!("{}.mkv", i));
        std::fs::write(&segment_path, format!("Test segment {}", i))?;
        segments.push(segment_path);
    }
    
    // Create parallel encoder with memory tracking
    let encoder = Arc::new(TestEncoder);
    let parallel_encoder = ParallelEncoder::new(encoder)
        .max_concurrent_jobs(4)
        .memory_per_job(512) // 512MB per job
        .on_progress(|progress, completed, total| {
            info!("Overall progress: {:.1}% ({}/{})", progress * 100.0, completed, total);
        });
    
    // Start timing
    let start = Instant::now();
    
    // Run parallel encoding
    let results = parallel_encoder.encode_segments(&segments, &output_dir, &temp_dir)?;
    
    let duration = start.elapsed();
    info!("Parallel encoding completed in {:.2?} with {} segments", duration, results.len());
    
    // Clean up test files
    std::fs::remove_dir_all(temp_dir)?;
    
    Ok(())
}