//! Core video processing logic and orchestration.
//!
//! This module serves as the central hub for the core video processing logic
//! within the drapto-core library. It organizes different processing steps
//! into submodules and exposes the primary functions for initiating these tasks.

/// Main video encoding orchestration logic
pub mod video;

/// Audio stream handling and processing
pub mod audio;

/// Video property detection and analysis
pub mod video_properties;

/// Crop detection and analysis
pub mod crop_detection;

/// Post-encode validation
pub mod validation;

/// Encoding parameter helpers
pub mod encode_params;

/// Shared formatting helpers
pub mod formatting;

/// Event and notification helpers
pub mod reporting;

/// Analysis wrappers for workflow steps
pub mod analysis;

pub use crop_detection::detect_crop;
pub use validation::{ValidationResult, validate_output_video};
pub use video::process_videos;
pub use video_properties::VideoProperties;
