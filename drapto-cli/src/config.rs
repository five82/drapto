// ============================================================================
// drapto-cli/src/config.rs
// ============================================================================
//
// DEFAULT CONFIGURATION: Constants for the Drapto CLI Application
//
// This file defines default configuration constants used throughout the
// application. These values are used when the user doesn't explicitly
// override them via command-line arguments.
//
// KEY COMPONENTS:
// - Encoder preset defaults
// - Processing mode defaults
// - Other application-wide constants
//
// AI-ASSISTANT-INFO: Default configuration values for the application

/// Default encoder preset for the SVT-AV1 encoder (0-13).
/// Lower values are slower but produce better quality.
/// Higher values are faster but may reduce quality.
/// Value 6 provides a good balance between speed and quality.
pub const DEFAULT_ENCODER_PRESET: i32 = 6;

/// Default crop detection mode.
/// Options:
/// - "auto": Automatically detect and crop black bars
/// - "none": Disable cropping
pub const DEFAULT_CROP_MODE: &str = "auto";
