// ============================================================================
// drapto-core/src/hardware_accel.rs
// ============================================================================
//
// HARDWARE ACCELERATION: Centralized hardware acceleration detection and configuration
//
// This module provides a centralized place for all hardware acceleration related
// functionality. Currently, it only supports VideoToolbox hardware decoding on macOS.
//
// KEY COMPONENTS:
// - Hardware acceleration detection
// - FFmpeg command configuration for hardware acceleration
// - Reporting of hardware acceleration capabilities
//
// DESIGN PHILOSOPHY:
// This module follows a minimalist approach, focusing only on VideoToolbox hardware
// decoding on macOS. It provides a simple API for detecting and configuring hardware
// acceleration.

use crate::progress_reporting::report_hardware_acceleration;
use ffmpeg_sidecar::command::FfmpegCommand;
use std::env;

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
        report_hardware_acceleration(self.videotoolbox_decode_available, "VideoToolbox");
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

/// Checks if the current platform is macOS.
///
/// This function uses the `std::env::consts::OS` constant to determine
/// if the current operating system is macOS.
///
/// # Returns
///
/// * `true` - If the current platform is macOS
/// * `false` - Otherwise
pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}

/// Checks if hardware acceleration is available on the current platform.
///
/// Currently, this only checks for VideoToolbox on macOS.
///
/// # Returns
///
/// * `true` - If hardware acceleration is available
/// * `false` - Otherwise
pub fn is_hardware_acceleration_available() -> bool {
    is_macos()
}

/// Adds hardware acceleration options to an FFmpeg command.
///
/// IMPORTANT: This must be called BEFORE adding the input file to the command.
///
/// # Arguments
///
/// * `cmd` - The FFmpeg command to add hardware acceleration options to
/// * `use_hw_decode` - Whether to use hardware acceleration
/// * `is_grain_analysis_sample` - Whether this is a grain analysis sample (hardware acceleration is disabled for grain analysis)
///
/// # Returns
///
/// * `bool` - Whether hardware acceleration was added
pub fn add_hardware_acceleration_to_command(
    cmd: &mut FfmpegCommand,
    use_hw_decode: bool,
    is_grain_analysis_sample: bool,
) -> bool {
    let hw_accel_available = is_hardware_acceleration_available();

    if use_hw_decode && hw_accel_available && !is_grain_analysis_sample {
        // IMPORTANT: This call is adding hw acceleration options to the command
        // but is NOT logging anything. The "Hardware: VideoToolbox hardware decoding available"
        // message in the output is coming from another source when this function is called.
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");

        // Note: This function doesn't directly log anything.
        // When it returns true, the caller usually logs a message.
        return true;
    }

    false
}

/// Logs hardware acceleration status.
///
/// This function logs information about the available hardware acceleration
/// capabilities to the info log level.
pub fn log_hardware_acceleration_status() {
    let hw_accel_available = is_hardware_acceleration_available();

    if hw_accel_available {
        report_hardware_acceleration(true, "VideoToolbox");
    } else {
        report_hardware_acceleration(false, "VideoToolbox");
    }
}

/// Gets a human-readable string describing the hardware acceleration capabilities.
///
/// This function is useful for displaying hardware acceleration information
/// in user interfaces.
///
/// # Returns
///
/// * `Option<String>` - A string describing the hardware acceleration capabilities,
///   or None if no hardware acceleration is available
pub fn get_hardware_accel_info() -> Option<String> {
    let hw_accel_available = is_hardware_acceleration_available();

    if hw_accel_available {
        Some("VideoToolbox".to_string())
    } else {
        None
    }
}
