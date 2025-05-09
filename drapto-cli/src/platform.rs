// ============================================================================
// drapto-cli/src/platform.rs
// ============================================================================
//
// PLATFORM: Platform-specific functionality
//
// This module provides platform-specific functionality and abstractions for
// the drapto-cli application. It encapsulates platform detection, hardware
// acceleration capabilities, and other OS-specific features.
//
// KEY COMPONENTS:
// - Platform detection functions
// - Hardware acceleration capability detection
// - Platform-specific formatting and output
//
// DESIGN PHILOSOPHY:
// This module centralizes platform-specific code to make the rest of the
// application more portable and easier to maintain. It provides a clean
// abstraction over platform differences.
//
// AI-ASSISTANT-INFO: Platform-specific functionality and detection

// ---- Standard library imports ----
use std::env;

// ---- External crate imports ----
use colored::*;
use log::info;

// ============================================================================
// PLATFORM DETECTION
// ============================================================================

/// Checks if the current platform is macOS.
///
/// This function uses the `std::env::consts::OS` constant to determine
/// if the current operating system is macOS.
///
/// # Returns
///
/// * `true` - If the current platform is macOS
/// * `false` - Otherwise
///
/// # Examples
///
/// ```rust
/// use drapto_cli::platform::is_macos;
///
/// if is_macos() {
///     println!("Running on macOS, can use VideoToolbox");
/// } else {
///     println!("Not running on macOS");
/// }
/// ```
pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}

// ============================================================================
// HARDWARE ACCELERATION
// ============================================================================

/// Represents hardware acceleration capabilities for the current platform.
#[derive(Debug, Clone, Copy)]
pub struct HardwareAcceleration {
    /// Whether VideoToolbox hardware decoding is available (macOS only)
    pub videotoolbox_decode_available: bool,
}

impl Default for HardwareAcceleration {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareAcceleration {
    /// Detects hardware acceleration capabilities for the current platform.
    ///
    /// # Returns
    ///
    /// * `HardwareAcceleration` - The detected hardware acceleration capabilities
    pub fn detect() -> Self {
        // Currently, we only support VideoToolbox on macOS
        let videotoolbox_decode_available = is_macos();

        Self {
            videotoolbox_decode_available,
        }
    }

    /// Logs information about hardware acceleration capabilities.
    ///
    /// This function logs information about the available hardware acceleration
    /// capabilities to the info log level.
    pub fn log_capabilities(&self) {
        if self.videotoolbox_decode_available {
            info!("{} {}", "Hardware:".cyan(), "VideoToolbox hardware decoding available".green().bold());
        } else {
            info!("{} {}", "Hardware:".cyan(), "Using software decoding (hardware acceleration not available on this platform)".yellow());
        }
    }

    /// Gets FFmpeg hardware acceleration arguments for the current platform.
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - The FFmpeg hardware acceleration arguments
    pub fn get_ffmpeg_hwaccel_args(&self) -> Vec<String> {
        let mut args = Vec::new();
        
        if self.videotoolbox_decode_available {
            args.push("-hwaccel".to_string());
            args.push("videotoolbox".to_string());
        }
        
        args
    }
}
