//! Core library for video processing and encoding tasks using ffmpeg and ffprobe.
//!
//! This crate provides video file discovery, property detection, crop analysis,
//! and encoding with optimized parameters.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use drapto_core::{CoreConfig, process_videos};
//! use std::path::PathBuf;
//!
//! // Create configuration
//! let mut config = CoreConfig::new(
//!     PathBuf::from("/path/to/input"),
//!     PathBuf::from("/path/to/output"),
//!     PathBuf::from("/path/to/logs")
//! );
//!
//! // Configure encoding settings
//! config.svt_av1_preset = 6;
//! config.quality_hd = 27;
//!
//! // Validate configuration
//! config.validate().unwrap();
//!
//! // Find video files to process
//! let files = drapto_core::find_processable_files(&config.input_dir).unwrap();
//!
//! // Process videos
//! let results = process_videos(
//!     &config,
//!     &files,
//!     None,
//!     None, // No event dispatcher for this example
//! ).unwrap();
//! ```

pub mod config;
pub mod discovery;
pub mod error;
pub mod events;
pub mod external;
pub mod file_logging;
pub mod hardware_decode;
pub mod presentation;
pub mod processing;
pub mod system_info;
pub mod temp_files;
pub mod utils;

// Re-exports for public API
pub use config::CoreConfig;
pub use discovery::find_processable_files;
pub use error::{CoreError, CoreResult};
pub use external::{
    MediaInfo, get_audio_channels, get_file_size, get_media_info, get_video_properties,
};
pub use hardware_decode::{
    HardwareDecoding, add_hardware_decoding_to_command, is_hardware_decoding_available, is_macos,
};
pub use processing::process_videos;
pub use temp_files::{create_temp_dir, create_temp_file, create_temp_file_path};
pub use utils::{format_bytes, format_duration, parse_ffmpeg_time};

use std::time::Duration;

/// Encoding statistics returned by process_videos for each successful file.
#[derive(Debug, Clone)]
pub struct EncodeResult {
    pub filename: String,
    pub duration: Duration,
    pub input_size: u64,
    pub output_size: u64,
    pub video_duration_secs: f64,
    pub encoding_speed: f32,
    pub validation_passed: bool,
    pub validation_steps: Vec<(String, bool, String)>, // (step_name, passed, details)
}
