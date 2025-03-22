use std::path::PathBuf;
use std::time::Instant;
use std::sync::Arc;

use drapto_core::{
    config::Config,
    encoding::pipeline::{EncodingPipeline, PipelineOptions},
    error::Result,
};

fn progress_callback(progress: f32, message: &str) {
    println!("[{:.1}%] {}", progress * 100.0, message);
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init();
    
    // Parse command-line arguments
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <input_file> <output_file>", args[0]);
        std::process::exit(1);
    }
    
    let input_file = PathBuf::from(&args[1]);
    let output_file = PathBuf::from(&args[2]);
    
    if !input_file.exists() {
        eprintln!("Input file not found: {}", input_file.display());
        std::process::exit(1);
    }
    
    // Ensure output directory exists
    if let Some(parent) = output_file.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // Create pipeline configuration
    let mut config = Config::default();
    config.parallel_jobs = num_cpus::get();
    config.use_segmentation = false;
    
    let options = PipelineOptions {
        config,
        working_dir: std::env::temp_dir(),
        disable_crop: false,
        progress_callback: Some(Arc::new(progress_callback)),
    };
    
    // Create and run the pipeline
    let start_time = Instant::now();
    println!("Starting encoding pipeline");
    
    let mut pipeline = EncodingPipeline::with_options(options);
    let stats = pipeline.process_file(&input_file, &output_file)?;
    
    // Print results
    let elapsed = start_time.elapsed();
    println!("\nEncoding completed in {:.2}s", elapsed.as_secs_f64());
    println!("Input:  {} ({} bytes)", stats.input_file, stats.input_size);
    println!("Output: {} ({} bytes)", stats.output_file, stats.output_size);
    println!("Size reduction: {:.2}%", stats.reduction_percent);
    println!("Dolby Vision: {}", if stats.is_dolby_vision { "Yes" } else { "No" });
    println!("Segments: {}", stats.segment_count);
    println!("Audio tracks: {}", stats.audio_track_count);
    
    // Validation results
    println!("\nValidation Results:");
    println!("Errors: {}", stats.validation_summary.error_count);
    println!("Warnings: {}", stats.validation_summary.warning_count);
    println!("Video validation: {}", if stats.validation_summary.video_passed { "Passed" } else { "Failed" });
    println!("Audio validation: {}", if stats.validation_summary.audio_passed { "Passed" } else { "Failed" });
    println!("A/V sync validation: {}", if stats.validation_summary.sync_passed { "Passed" } else { "Failed" });
    
    Ok(())
}