//! Encoding Summary and Statistics Example
//!
//! This example demonstrates the encoding summary and reporting capabilities:
//! 1. Generating detailed encoding summaries for individual files
//! 2. Processing an entire directory of files using batch processing
//! 3. Collecting and displaying statistics about the encoding process
//! 4. Formatting results in a human-readable way
//!
//! The example shows both single file processing and batch directory processing
//! modes with progress tracking and performance statistics.
//!
//! Run with:
//! ```
//! # Process a single file
//! cargo run --example summary_example <input_file> <output_file>
//!
//! # Process a directory of files
//! cargo run --example summary_example <input_dir> <output_dir> -d
//! ```

use std::env;
use std::path::PathBuf;
use std::time::Instant;

use drapto_core::{
    EncodingSummary,
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
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage:");
        println!("  {} <input_file> <output_file>    # Process a single file", args[0]);
        println!("  {} <input_dir> <output_dir> -d   # Process a directory of files", args[0]);
        return Ok(());
    }
    
    let input_path = PathBuf::from(&args[1]);
    let output_path = PathBuf::from(&args[2]);
    let is_directory = args.len() > 3 && args[3] == "-d";
    
    // Create pipeline configuration
    let mut config = Config::default();
    config.resources.parallel_jobs = num_cpus::get();
    config.video.use_segmentation = false;
    
    let options = PipelineOptions {
        config,
        working_dir: env::temp_dir(),
        disable_crop: false,
        progress_callback: Some(std::sync::Arc::new(progress_callback)),
    };
    
    // Create the pipeline
    let mut pipeline = EncodingPipeline::with_options(options);
    
    // Process file or directory
    let start_time = Instant::now();
    
    if is_directory {
        println!("Processing directory: {}", input_path.display());
        if !input_path.is_dir() {
            eprintln!("Error: {} is not a directory", input_path.display());
            std::process::exit(1);
        }
        
        // Process the directory
        let results = pipeline.process_directory(&input_path, &output_path)?;
        
        println!("\nDirectory Processing Completed");
        println!("============================");
        println!("Files processed: {}", results.len());
        println!("Time elapsed: {:.2}s", start_time.elapsed().as_secs_f64());
        
        // All summary reporting is done inside the pipeline, no need to duplicate here
    } else {
        println!("Processing file: {}", input_path.display());
        if !input_path.is_file() {
            eprintln!("Error: {} is not a file", input_path.display());
            std::process::exit(1);
        }
        
        // Process the single file
        let stats = pipeline.process_file(&input_path, &output_path)?;
        
        println!("\nIndividual File Processing Completed");
        println!("==================================");
        println!("Input:  {} ({} bytes)", stats.input_file, stats.input_size);
        println!("Output: {} ({} bytes)", stats.output_file, stats.output_size);
        println!("Reduction: {:.2}%", stats.reduction_percent);
        println!("Time elapsed: {:.2}s", stats.encoding_time);
        
        // Generate a standalone summary
        // (pipeline already generates one, but this demonstrates manual creation)
        let summary = EncodingSummary::from_pipeline_stats(&stats);
        println!("\nSummary information:");
        println!("{}", summary.as_compact_line());
    }
    
    Ok(())
}