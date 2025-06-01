//! Core library for video processing and encoding tasks using ffmpeg and ffprobe.
//!
//! This crate provides video file discovery, property detection, crop analysis,
//! denoising, and encoding with optimized parameters.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use drapto_core::{CoreConfig, process_videos};
//! use drapto_core::notifications::NtfyNotificationSender;
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
//! config.enable_denoise = true;
//! config.encoder_preset = 6;
//! config.quality_hd = 27;
//! 
//! // Validate configuration
//! config.validate().unwrap();
//!
//! // Find video files to process
//! let files = drapto_core::find_processable_files(&config.input_dir).unwrap();
//! 
//! // Optional: Set up notifications
//! let notification_sender = NtfyNotificationSender::new("https://ntfy.sh/my-topic").unwrap();
//!
//! // Process videos
//! let results = process_videos(
//!     Some(&notification_sender),
//!     &config,
//!     &files,
//!     None,
//! ).unwrap();
//! ```

pub mod config;
pub mod discovery;
pub mod error;
pub mod external;
pub mod processing;
pub mod utils;
pub mod notifications;
pub mod temp_files;
pub mod hardware_decode;
pub mod terminal;
pub mod progress_reporting;

// Re-exports for public API
pub use config::CoreConfig;
pub use discovery::find_processable_files;
pub use error::{CoreError, CoreResult};
pub use processing::process_videos;
pub use utils::{format_bytes, format_duration, parse_ffmpeg_time};
pub use external::{
    MediaInfo, get_audio_channels, get_file_size, get_media_info,
    get_video_properties,
};
pub use notifications::{Notification, NtfyNotificationSender};
pub use temp_files::{
    create_temp_dir,
    create_temp_file,
    create_temp_file_path,
};
pub use hardware_decode::{
    HardwareDecoding, add_hardware_decoding_to_command, is_hardware_decoding_available, is_macos,
};
pub use progress_reporting::{
    TerminalProgressReporter, set_progress_reporter,
    processing_debug, status_debug, info_debug
};

use std::time::Duration;

/// Encoding statistics returned by process_videos for each successful file.
#[derive(Debug, Clone)]
pub struct EncodeResult {
    pub filename: String,
    pub duration: Duration,
    pub input_size: u64,
    pub output_size: u64,
}
