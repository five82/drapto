//! Hardware decoding detection and configuration.
//!
//! This module provides centralized hardware decoding functionality.
//! Currently only supports `VideoToolbox` hardware decoding on macOS.
//!
//! **Important**: This module is ONLY for hardware DECODING, not encoding.
//! We use software encoding (libsvtav1) exclusively for output.

use ffmpeg_sidecar::command::FfmpegCommand;
use std::env;

/// Represents hardware decoding capabilities for the current platform.
#[derive(Debug, Clone, Copy)]
pub struct HardwareDecoding {
    /// Whether `VideoToolbox` hardware decoding is available (macOS only)
    pub videotoolbox_decode_available: bool,
}

impl Default for HardwareDecoding {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareDecoding {
    /// Detects hardware decoding capabilities.
    #[must_use] pub fn detect() -> Self {
        let videotoolbox_decode_available = is_macos();

        Self {
            videotoolbox_decode_available,
        }
    }

    /// Logs available hardware decoding capabilities.
    pub fn log_capabilities(&self) {
        if self.videotoolbox_decode_available {
            log::info!("Hardware: VideoToolbox (decode only)");
        } else {
            log::info!("Hardware: No hardware decoder available");
        }
    }

    /// Returns FFmpeg hardware decoding arguments for the platform.
    #[must_use] pub fn get_ffmpeg_hwdecode_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if self.videotoolbox_decode_available {
            args.push("-hwaccel".to_string());
            args.push("videotoolbox".to_string());
        }

        args
    }
}

/// Returns true if running on macOS.
#[must_use] pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}

/// Returns true if VideoToolbox hardware decoding is available (macOS only).
#[must_use] pub fn is_hardware_decoding_available() -> bool {
    is_macos()
}

/// Adds hardware decoding to FFmpeg command. Must be called BEFORE input file.
pub fn add_hardware_decoding_to_command(
    cmd: &mut FfmpegCommand,
    use_hw_decode: bool,
) -> bool {
    let hw_decode_available = is_hardware_decoding_available();

    if use_hw_decode && hw_decode_available {
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");
        return true;
    }

    false
}

/// Logs available hardware decoding status.
pub fn log_hardware_decoding_status() {
    let hw_decode_available = is_hardware_decoding_available();

    if hw_decode_available {
        log::info!("Hardware: VideoToolbox (decode only)");
    } else {
        log::info!("Hardware: No hardware decoder available");
    }
}

/// Returns "VideoToolbox" on macOS, None otherwise.
#[must_use] pub fn get_hardware_decoding_info() -> Option<String> {
    let hw_decode_available = is_hardware_decoding_available();

    if hw_decode_available {
        Some("VideoToolbox".to_string())
    } else {
        None
    }
}
