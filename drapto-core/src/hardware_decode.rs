//! Hardware decoding detection and configuration.
//!
//! This module provides centralized hardware decoding functionality.
//! Supports `VideoToolbox` hardware decoding on macOS and `VAAPI` on Linux.
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
    /// Whether `VAAPI` hardware decoding is available (Linux only)
    pub vaapi_decode_available: bool,
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
        let vaapi_decode_available = is_linux() && is_vaapi_available();

        Self {
            videotoolbox_decode_available,
            vaapi_decode_available,
        }
    }

    /// Logs available hardware decoding capabilities.
    pub fn log_capabilities(&self) {
        if self.videotoolbox_decode_available {
            log::info!("Hardware: VideoToolbox (decode only)");
        } else if self.vaapi_decode_available {
            log::info!("Hardware: VAAPI (decode only)");
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
        } else if self.vaapi_decode_available {
            args.push("-hwaccel".to_string());
            args.push("vaapi".to_string());
            args.push("-hwaccel_device".to_string());
            args.push("/dev/dri/renderD128".to_string());
        }

        args
    }
}

/// Returns true if running on macOS.
#[must_use] pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}

/// Returns true if running on Linux.
#[must_use] pub fn is_linux() -> bool {
    env::consts::OS == "linux"
}

/// Returns true if VAAPI hardware decoding is available on Linux.
#[must_use] pub fn is_vaapi_available() -> bool {
    use std::path::Path;
    
    // Check for DRI render device (most common VAAPI device)
    if Path::new("/dev/dri/renderD128").exists() {
        return true;
    }
    
    // Check for alternative DRI devices (renderD129, etc.)
    if let Ok(entries) = std::fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with("renderD") {
                    return true;
                }
            }
        }
    }
    
    false
}

/// Returns true if any hardware decoding is available.
#[must_use] pub fn is_hardware_decoding_available() -> bool {
    is_macos() || (is_linux() && is_vaapi_available())
}

/// Adds hardware decoding to FFmpeg command. Must be called BEFORE input file.
pub fn add_hardware_decoding_to_command(
    cmd: &mut FfmpegCommand,
    use_hw_decode: bool,
) -> bool {
    if !use_hw_decode {
        return false;
    }

    if is_macos() {
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");
        return true;
    } else if is_linux() && is_vaapi_available() {
        cmd.arg("-hwaccel");
        cmd.arg("vaapi");
        cmd.arg("-hwaccel_device");
        cmd.arg("/dev/dri/renderD128");
        return true;
    }

    false
}

/// Logs available hardware decoding status.
pub fn log_hardware_decoding_status() {
    if is_macos() {
        log::info!("Hardware: VideoToolbox (decode only)");
    } else if is_linux() && is_vaapi_available() {
        log::info!("Hardware: VAAPI (decode only)");
    } else {
        log::info!("Hardware: No hardware decoder available");
    }
}

/// Returns hardware decoding info string if available.
#[must_use] pub fn get_hardware_decoding_info() -> Option<String> {
    if is_macos() {
        Some("VideoToolbox".to_string())
    } else if is_linux() && is_vaapi_available() {
        Some("VAAPI".to_string())
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ffmpeg_sidecar::command::FfmpegCommand;
    
    #[test]
    fn test_is_macos() {
        #[cfg(target_os = "macos")]
        assert!(is_macos());
        
        #[cfg(not(target_os = "macos"))]
        assert!(!is_macos());
    }
    
    #[test]
    fn test_is_linux() {
        #[cfg(target_os = "linux")]
        assert!(is_linux());
        
        #[cfg(not(target_os = "linux"))]
        assert!(!is_linux());
    }
    
    #[test]
    fn test_hardware_decoding_detection() {
        let hw = HardwareDecoding::detect();
        
        #[cfg(target_os = "macos")]
        {
            assert!(hw.videotoolbox_decode_available);
            assert!(!hw.vaapi_decode_available);
        }
        
        #[cfg(target_os = "linux")]
        {
            assert!(!hw.videotoolbox_decode_available);
            // VAAPI availability depends on system hardware
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            assert!(!hw.videotoolbox_decode_available);
            assert!(!hw.vaapi_decode_available);
        }
    }
    
    #[test]
    fn test_get_ffmpeg_hwdecode_args_macos() {
        #[cfg(target_os = "macos")]
        {
            let hw = HardwareDecoding { videotoolbox_decode_available: true, vaapi_decode_available: false };
            let args = hw.get_ffmpeg_hwdecode_args();
            assert_eq!(args, vec!["-hwaccel", "videotoolbox"]);
        }
    }
    
    #[test]
    fn test_get_ffmpeg_hwdecode_args_linux_with_vaapi() {
        #[cfg(target_os = "linux")]
        {
            let hw = HardwareDecoding { videotoolbox_decode_available: false, vaapi_decode_available: true };
            let args = hw.get_ffmpeg_hwdecode_args();
            assert_eq!(args, vec!["-hwaccel", "vaapi", "-hwaccel_device", "/dev/dri/renderD128"]);
        }
    }
    
    #[test]
    fn test_get_ffmpeg_hwdecode_args_no_hardware() {
        let hw = HardwareDecoding { videotoolbox_decode_available: false, vaapi_decode_available: false };
        let args = hw.get_ffmpeg_hwdecode_args();
        assert!(args.is_empty());
    }
    
    #[test]
    fn test_add_hardware_decoding_to_command_disabled() {
        let mut cmd = FfmpegCommand::new();
        let result = add_hardware_decoding_to_command(&mut cmd, false);
        assert!(!result);
    }
    
    #[test]
    fn test_add_hardware_decoding_to_command_macos() {
        #[cfg(target_os = "macos")]
        {
            let mut cmd = FfmpegCommand::new();
            let result = add_hardware_decoding_to_command(&mut cmd, true);
            assert!(result);
        }
    }
    
    #[test]
    fn test_add_hardware_decoding_to_command_linux_with_vaapi() {
        #[cfg(target_os = "linux")]
        {
            if is_vaapi_available() {
                let mut cmd = FfmpegCommand::new();
                let result = add_hardware_decoding_to_command(&mut cmd, true);
                assert!(result);
            }
        }
    }
    
    #[test]
    fn test_get_hardware_decoding_info() {
        #[cfg(target_os = "macos")]
        {
            let info = get_hardware_decoding_info();
            assert_eq!(info, Some("VideoToolbox".to_string()));
        }
        
        #[cfg(target_os = "linux")]
        {
            let info = get_hardware_decoding_info();
            if is_vaapi_available() {
                assert_eq!(info, Some("VAAPI".to_string()));
            } else {
                assert_eq!(info, None);
            }
        }
        
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            let info = get_hardware_decoding_info();
            assert_eq!(info, None);
        }
    }
    
    #[test]
    fn test_ffmpeg_command_integration_with_hardware_decoding() {
        // Test that FfmpegCommandBuilder properly integrates hardware decoding
        let mut cmd = FfmpegCommand::new();
        
        #[cfg(target_os = "linux")]
        {
            if is_vaapi_available() {
                // Manually add hardware args to verify they're correctly formatted
                add_hardware_decoding_to_command(&mut cmd, true);
                
                // Convert to string to inspect the actual command (this is a bit hacky but works for testing)
                let debug_output = format!("{:?}", cmd);
                assert!(debug_output.contains("-hwaccel"));
                assert!(debug_output.contains("vaapi"));
                assert!(debug_output.contains("-hwaccel_device"));
                assert!(debug_output.contains("/dev/dri/renderD128"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            add_hardware_decoding_to_command(&mut cmd, true);
            let debug_output = format!("{:?}", cmd);
            assert!(debug_output.contains("-hwaccel"));
            assert!(debug_output.contains("videotoolbox"));
        }
    }
    
    #[test]
    fn test_ffmpeg_builder_integration() {
        use crate::external::FfmpegCommandBuilder;
        
        // Test that the builder properly calls hardware decoding
        let cmd = FfmpegCommandBuilder::new()
            .with_hardware_accel(true)
            .build();
        
        let debug_output = format!("{:?}", cmd);
        
        #[cfg(target_os = "linux")]
        {
            if is_vaapi_available() {
                assert!(debug_output.contains("-hwaccel"));
                assert!(debug_output.contains("vaapi"));
            }
        }
        
        #[cfg(target_os = "macos")]
        {
            assert!(debug_output.contains("-hwaccel"));
            assert!(debug_output.contains("videotoolbox"));
        }
    }
}
