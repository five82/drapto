//! AV1 Video Encoding with Ab-AV1 Example
//!
//! This example demonstrates high-efficiency AV1 video encoding using the Ab-AV1 encoder:
//! 1. Creating an Ab-AV1 encoder with quality-targeted configuration
//! 2. Checking for the availability of the ab-av1 encoder on the system
//! 3. Encoding a video segment with specified VMAF quality target
//! 4. Collecting and displaying detailed encoding statistics
//! 5. Validating the encoded segment against the source
//! 6. Supporting HDR and Dolby Vision content
//!
//! Run with:
//! ```
//! cargo run --example ab_av1_encoding --input video.mp4 --output encoded.mkv --vmaf 95 --preset 8
//! ```
//!
//! Optional flags:
//! --hdr          Enable HDR processing
//! --dv           Enable Dolby Vision processing
//! --crop "..."   Specify cropping parameters

use std::path::PathBuf;
use drapto_core::encoding::video::{AbAv1Encoder, AbAv1Config};
use drapto_core::error::Result;
use std::time::Instant;
use clap::Parser;
use log::{info, error};

/// Command line arguments for the example
#[derive(Parser, Debug)]
#[clap(about = "Ab-AV1 encoding example")]
struct Args {
    /// Path to input video file
    #[clap(short, long)]
    input: Option<PathBuf>,
    
    /// Path to output video file
    #[clap(short, long)]
    output: Option<PathBuf>,
    
    /// Optional crop filter (e.g. "crop=1920:1080:0:0")
    #[clap(short, long)]
    crop: Option<String>,
    
    /// Whether the input is HDR content
    #[clap(long, action = clap::ArgAction::SetTrue)]
    hdr: bool,
    
    /// Whether the input has Dolby Vision
    #[clap(long, action = clap::ArgAction::SetTrue)]
    dv: bool,
    
    /// Encoder preset (1-13, lower is slower/better quality)
    #[clap(short, long, default_value = "8")]
    preset: u8,
    
    /// Target VMAF score (0-100)
    #[clap(short, long, default_value = "95.0")]
    vmaf: f32,
}

fn main() -> Result<()> {
    // Initialize logging
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("info"));
    
    // Parse command line arguments
    let args = Args::parse();
    
    // Setup paths
    let input_path = args.input.unwrap_or_else(|| {
        info!("No input specified, using a default test file path");
        PathBuf::from("test_input.mkv")
    });
    
    let output_path = args.output.unwrap_or_else(|| {
        let mut output = input_path.clone();
        output.set_file_name(format!(
            "{}_encoded.mkv",
            input_path.file_stem().unwrap_or_default().to_string_lossy()
        ));
        output
    });
    
    info!("Input: {}", input_path.display());
    info!("Output: {}", output_path.display());
    
    // Create Ab-AV1 configuration
    let config = AbAv1Config {
        preset: args.preset,
        target_vmaf: args.vmaf,
        target_vmaf_hdr: args.vmaf,
        ..AbAv1Config::default()
    };
    
    // Create encoder
    let encoder = AbAv1Encoder::with_config(config);
    
    // Check if ab-av1 is available
    match encoder.check_availability() {
        Ok(_) => info!("ab-av1 is available"),
        Err(e) => {
            error!("ab-av1 check failed: {}", e);
            return Err(e);
        }
    }
    
    // Check if input file exists
    if !input_path.exists() {
        error!("Input file does not exist: {}", input_path.display());
        return Err(drapto_core::error::DraptoError::InvalidPath(
            format!("Input file does not exist: {}", input_path.display())
        ));
    }
    
    // Create output directory if it doesn't exist
    if let Some(parent) = output_path.parent() {
        if !parent.exists() {
            std::fs::create_dir_all(parent)?;
        }
    }
    
    // Encode the segment
    info!("Starting encoding process...");
    let start_time = Instant::now();
    
    let stats = encoder.encode_segment(
        &input_path,
        &output_path,
        args.crop.as_deref(),
        0, // First attempt
        args.hdr,
        args.dv,
    )?;
    
    let elapsed = start_time.elapsed();
    info!("Encoding completed in {:.2} seconds", elapsed.as_secs_f64());
    
    // Print encoding statistics
    info!("Encoding statistics:");
    info!("  Duration: {:.2} seconds", stats.metrics.duration);
    info!("  Size: {:.2} MB", stats.metrics.size_bytes as f64 / (1024.0 * 1024.0));
    info!("  Bitrate: {:.2} kbps", stats.metrics.bitrate_kbps);
    info!("  Speed: {:.2}x realtime", stats.metrics.speed_factor);
    info!("  Resolution: {}", stats.metrics.resolution);
    
    if let Some(vmaf) = stats.vmaf_score {
        info!("  VMAF score: {:.2} (min: {:.2}, max: {:.2})",
             vmaf,
             stats.vmaf_min.unwrap_or(0.0),
             stats.vmaf_max.unwrap_or(0.0));
    } else {
        info!("  No VMAF scores available");
    }
    
    // Validate the encoded segment
    info!("Validating encoded segment...");
    encoder.validate_segment(&input_path, &output_path, 0.2)?;
    info!("Validation successful!");
    
    Ok(())
}