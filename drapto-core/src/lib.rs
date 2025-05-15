//! # drapto-core
//!
//! Core library for video processing and encoding tasks using ffmpeg and ffprobe.
//!
//! ## Overview
//!
//! This crate provides the core logic for video processing tasks, primarily focusing
//! on interacting with ffmpeg for encoding and ffprobe for analysis. It handles
//! video file discovery, property detection, crop analysis, grain/noise analysis,
//! and encoding with optimized parameters.
//!
//! ## Architecture
//!
//! The library follows a modular design with dependency injection patterns to allow
//! for flexible configuration and easier testing. It uses traits to define interfaces
//! for external tool interactions, file system operations, and notifications.
//!
//! ## Module Structure
//!
//! The crate is organized into several modules:
//! - `config`: Defines configuration structures (`CoreConfig`) used throughout the library.
//! - `discovery`: Contains functions for finding processable video files (`find_processable_files`).
//! - `error`: Defines custom error types (`CoreError`) and results (`CoreResult`) for the library.
//! - `external`: Handles interactions with external command-line tools like ffmpeg and ffprobe.
//! - `processing`: Contains the main video processing logic, including encoding orchestration
//!   (`process_videos`) and detection algorithms for crop and grain analysis.
//! - `utils`: Provides common utility functions (e.g., `format_bytes`, `format_duration`).
//! - `notifications`: Handles sending notifications about encoding progress.
//!
//! ## Public API
//!
//! This `lib.rs` file re-exports the primary public interface elements from the internal
//! modules, making them directly accessible to users of the `drapto-core` crate.
//! It also defines the `EncodeResult` struct, which is returned to report the outcome
//! of individual file encoding operations.
//!
//! ## Usage Example
//!
//! ```rust,no_run
//! use drapto_core::{CoreConfig, process_videos};
//! use drapto_core::external::{SidecarSpawner, CrateFfprobeExecutor, StdFsMetadataProvider};
//! use drapto_core::notifications::NtfyNotificationSender;
//! use drapto_core::processing::detection::GrainLevel;
//! use std::path::PathBuf;
//!
//! // Create configuration using the builder pattern
//! let config = drapto_core::config::CoreConfigBuilder::new()
//!     .input_dir(PathBuf::from("/path/to/input"))
//!     .output_dir(PathBuf::from("/path/to/output"))
//!     .log_dir(PathBuf::from("/path/to/logs"))
//!     .enable_denoise(true)
//!     .encoder_preset(6)
//!     .quality_sd(24)
//!     .quality_hd(26)
//!     .quality_uhd(28)
//!     .crop_mode("auto")
//!     .ntfy_topic("https://ntfy.sh/my-topic")
//!     .film_grain_sample_duration(5)
//!     .film_grain_knee_threshold(0.8)
//!     .film_grain_max_level(GrainLevel::Moderate)
//!     .film_grain_refinement_points_count(5)
//!     .build();
//!
//! // Find files to process
//! let files = drapto_core::find_processable_files(&config.input_dir).unwrap();
//!
//! // Create dependencies
//! let spawner = SidecarSpawner;
//! let ffprobe_executor = CrateFfprobeExecutor::new();
//! let notification_sender = NtfyNotificationSender::new("https://ntfy.sh/my-topic").unwrap();
//! let metadata_provider = StdFsMetadataProvider;
//!
//! // Process videos
//! let results = process_videos(
//!     &spawner,
//!     &ffprobe_executor,
//!     Some(&notification_sender),
//!     &metadata_provider,
//!     &config,
//!     &files,
//!     None,
//! ).unwrap();
//! ```
//!
//! ## AI-ASSISTANT-INFO
//!
//! Core library for video encoding with ffmpeg, handles file discovery, analysis, and encoding

// ============================================================================
// MODULE DECLARATIONS
// ============================================================================

/// Configuration structures and constants used throughout the library
pub mod config;

/// Functions for finding and filtering video files for processing
pub mod discovery;

/// Custom error types and result definitions
pub mod error;

/// Interactions with external tools like ffmpeg and ffprobe
pub mod external;

/// Core video processing logic including encoding and analysis
pub mod processing;

/// Utility functions for formatting and common operations
pub mod utils;

/// Notification services for encoding progress updates
pub mod notifications;

/// Progress reporting functions
pub mod progress_reporting;

/// Temporary file management utilities
pub mod temp_files;

/// Hardware acceleration detection and configuration
pub mod hardware_accel;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================
// These items are re-exported to make them directly accessible to users
// without requiring explicit imports from submodules

// ----- Configuration Types -----
/// Main configuration structure for the core library
pub use config::{CoreConfig, FilmGrainMetricType};

// ----- File Discovery -----
/// Function to find processable video files in a directory
pub use discovery::find_processable_files;

// ----- Error Handling -----
/// Custom error types and result type alias
pub use error::{CoreError, CoreResult};

// ----- Video Processing -----
/// Main function to process a list of video files
pub use processing::process_videos;

// ----- Utility Functions -----
/// Helper functions for formatting bytes and durations
pub use utils::{format_bytes, format_duration};

// ----- External Tool Interactions -----
/// Re-export external tool traits, implementations, and platform detection
pub use external::{
    CrateFfprobeExecutor, FfmpegProcess, FfmpegSpawner, FfprobeExecutor,
    FileMetadataProvider, SidecarProcess, SidecarSpawner, StdFsMetadataProvider,
};

// ----- Progress Reporting -----
/// Progress reporting functions
pub use progress_reporting::{
    report_encode_start, report_encode_progress, report_encode_complete,
    report_encode_error, report_hardware_acceleration, report_log_message,
    LogLevel,
};

// ----- Notification Services -----
/// Notification system types and implementations
pub use notifications::{NotificationType, NtfyNotificationSender};

// ----- Temporary File Management -----
/// Temporary file management utilities
pub use temp_files::{create_grain_analysis_dir, create_analysis_dir, cleanup_base_dirs, create_temp_file_path};

// ----- Hardware Acceleration -----
/// Hardware acceleration detection and configuration
pub use hardware_accel::{
    HardwareAcceleration, is_macos, is_hardware_acceleration_available,
    add_hardware_acceleration_to_command, log_hardware_acceleration_status
};

// ============================================================================
// PUBLIC STRUCTS
// ============================================================================

use std::time::Duration;

/// Result of an encoding operation, containing statistics about the process.
///
/// This structure is returned by the `process_videos` function for each
/// successfully processed video file. It contains information about the
/// encoding process, including the filename, duration of the encoding operation,
/// and file size statistics.
///
/// # Fields
///
/// * `filename` - The name of the processed file
/// * `duration` - How long the encoding process took
/// * `input_size` - Size of the original input file in bytes
/// * `output_size` - Size of the encoded output file in bytes
///
/// # Example
///
/// ```rust,no_run
/// use drapto_core::EncodeResult;
/// use std::time::Duration;
///
/// // Create a result for reporting or analysis
/// let result = EncodeResult {
///     filename: "video.mkv".to_string(),
///     duration: Duration::from_secs(3600), // 1 hour encoding time
///     input_size: 5_000_000_000, // 5 GB input
///     output_size: 1_000_000_000, // 1 GB output
/// };
///
/// // Calculate size reduction percentage
/// let reduction_percent = 100 - (result.output_size * 100 / result.input_size);
/// println!("Reduced file size by {}%", reduction_percent); // "Reduced file size by 80%"
/// ```
#[derive(Debug, Clone)]
pub struct EncodeResult {
    /// Name of the processed file
    pub filename: String,

    /// Duration of the encoding process
    pub duration: Duration,

    /// Size of the original input file in bytes
    pub input_size: u64,

    /// Size of the encoded output file in bytes
    pub output_size: u64,
}

