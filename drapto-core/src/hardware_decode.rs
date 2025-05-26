// ============================================================================
// drapto-core/src/hardware_decode.rs
// ============================================================================
//
// HARDWARE DECODING: Centralized hardware decoding detection and configuration
//
// This module provides a centralized place for all hardware decoding related
// functionality. Currently, it only supports VideoToolbox hardware decoding on macOS.
//
// IMPORTANT: This module is ONLY for hardware DECODING, not encoding. We use
// software encoding (libsvtav1) exclusively for output.
//
// KEY COMPONENTS:
// - Hardware decoding detection
// - FFmpeg command configuration for hardware decoding
// - Reporting of hardware decoding capabilities
//
// DESIGN PHILOSOPHY:
// This module follows a minimalist approach, focusing only on VideoToolbox hardware
// decoding on macOS. It provides a simple API for detecting and configuring hardware
// decoding.

// Hardware decoding reporting is now done inline
use ffmpeg_sidecar::command::FfmpegCommand;
use log;
use std::env;

/// Represents hardware decoding capabilities for the current platform.
#[derive(Debug, Clone, Copy)]
pub struct HardwareDecoding {
    /// Whether VideoToolbox hardware decoding is available (macOS only)
    pub videotoolbox_decode_available: bool,
}

impl Default for HardwareDecoding {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareDecoding {
    /// Detects hardware decoding capabilities for the current platform.
    ///
    /// # Returns
    ///
    /// * `HardwareDecoding` - The detected hardware decoding capabilities
    pub fn detect() -> Self {
        // Currently, we only support VideoToolbox on macOS
        let videotoolbox_decode_available = is_macos();

        Self {
            videotoolbox_decode_available,
        }
    }

    /// Logs information about hardware decoding capabilities.
    ///
    /// This function logs information about the available hardware decoding
    /// capabilities to the info log level.
    pub fn log_capabilities(&self) {
        if self.videotoolbox_decode_available {
            log::info!("Hardware decoding: VideoToolbox available");
        } else {
            log::info!("Hardware decoding: None");
        }
    }

    /// Gets FFmpeg hardware decoding arguments for the current platform.
    ///
    /// # Returns
    ///
    /// * `Vec<String>` - The FFmpeg hardware decoding arguments
    pub fn get_ffmpeg_hwdecode_args(&self) -> Vec<String> {
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

/// Checks if hardware decoding is available on the current platform.
///
/// Currently, this only checks for VideoToolbox on macOS.
///
/// # Returns
///
/// * `true` - If hardware decoding is available
/// * `false` - Otherwise
pub fn is_hardware_decoding_available() -> bool {
    is_macos()
}

/// Adds hardware decoding options to an FFmpeg command.
///
/// IMPORTANT: This must be called BEFORE adding the input file to the command.
///
/// # Arguments
///
/// * `cmd` - The FFmpeg command to add hardware decoding options to
/// * `use_hw_decode` - Whether to use hardware decoding
/// * `is_grain_analysis_sample` - Whether this is a grain analysis sample (hardware decoding is disabled for grain analysis)
///
/// # Returns
///
/// * `bool` - Whether hardware decoding was added
pub fn add_hardware_decoding_to_command(
    cmd: &mut FfmpegCommand,
    use_hw_decode: bool,
    is_grain_analysis_sample: bool,
) -> bool {
    let hw_decode_available = is_hardware_decoding_available();

    if use_hw_decode && hw_decode_available && !is_grain_analysis_sample {
        // IMPORTANT: This call is adding hw decoding options to the command
        // but is NOT logging anything. The hardware decoding status message
        // is logged separately by log_hardware_decoding_status().
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");

        // Note: This function doesn't directly log anything.
        // When it returns true, the caller usually logs a message.
        return true;
    }

    false
}

/// Logs hardware decoding status.
///
/// This function logs information about the available hardware decoding
/// capabilities to the info log level.
pub fn log_hardware_decoding_status() {
    let hw_decode_available = is_hardware_decoding_available();

    if hw_decode_available {
        log::info!("Hardware decoding: VideoToolbox available");
    } else {
        log::info!("Hardware decoding: None");
    }
}

/// Gets a human-readable string describing the hardware decoding capabilities.
///
/// This function is useful for displaying hardware decoding information
/// in user interfaces.
///
/// # Returns
///
/// * `Option<String>` - A string describing the hardware decoding capabilities,
///   or None if no hardware decoding is available
pub fn get_hardware_decoding_info() -> Option<String> {
    let hw_decode_available = is_hardware_decoding_available();

    if hw_decode_available {
        Some("VideoToolbox".to_string())
    } else {
        None
    }
}
