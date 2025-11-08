//! Hardware decoding detection and configuration.
//!
//! This module provides centralized hardware decoding functionality.
//! Supports `VideoToolbox` hardware decoding on macOS and `VAAPI` on Linux.
//!
//! **Important**: This module is ONLY for hardware DECODING, not encoding.
//! We use software encoding (libsvtav1) exclusively for output.

use ffmpeg_sidecar::command::FfmpegCommand;
use std::env;
use std::fs;
use std::path::Path;

const VAAPI_DRIVER_DIRS: &[&str] = &["/usr/lib/x86_64-linux-gnu/dri", "/usr/lib/dri"];

/// Represents hardware decoding capabilities for the current platform.
#[derive(Debug, Clone)]
pub struct HardwareDecoding {
    /// Whether `VideoToolbox` hardware decoding is available (macOS only)
    pub videotoolbox_decode_available: bool,
    /// Whether `VAAPI` hardware decoding is available (Linux only)
    pub vaapi_decode_available: bool,
    /// Selected VAAPI render node path (Linux only)
    pub vaapi_device_path: Option<String>,
    /// VAAPI driver hint to export for FFmpeg (Linux only)
    pub vaapi_driver: Option<String>,
}

impl Default for HardwareDecoding {
    fn default() -> Self {
        Self::detect()
    }
}

impl HardwareDecoding {
    /// Detects hardware decoding capabilities.
    #[must_use]
    pub fn detect() -> Self {
        let videotoolbox_decode_available = is_macos();
        let vaapi_info = if is_linux() {
            detect_vaapi_device()
        } else {
            None
        };
        let (vaapi_decode_available, vaapi_device_path, vaapi_driver) =
            if let Some(device) = vaapi_info {
                (true, Some(device.path), device.driver)
            } else {
                (false, None, None)
            };

        Self {
            videotoolbox_decode_available,
            vaapi_decode_available,
            vaapi_device_path,
            vaapi_driver,
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
    #[must_use]
    pub fn get_ffmpeg_hwdecode_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        if self.videotoolbox_decode_available {
            args.push("-hwaccel".to_string());
            args.push("videotoolbox".to_string());
        } else if self.vaapi_decode_available {
            args.push("-hwaccel".to_string());
            args.push("vaapi".to_string());
            args.push("-hwaccel_device".to_string());
            if let Some(path) = &self.vaapi_device_path {
                args.push(path.clone());
            }
        }

        args
    }
}

/// Returns true if running on macOS.
#[must_use]
pub fn is_macos() -> bool {
    env::consts::OS == "macos"
}

/// Returns true if running on Linux.
#[must_use]
pub fn is_linux() -> bool {
    env::consts::OS == "linux"
}

/// Returns true if VAAPI hardware decoding is available on Linux.
#[must_use]
pub fn is_vaapi_available() -> bool {
    detect_vaapi_device().is_some()
}

/// Returns true if any hardware decoding is available.
#[must_use]
pub fn is_hardware_decoding_available() -> bool {
    is_macos() || (is_linux() && is_vaapi_available())
}

/// Adds hardware decoding to FFmpeg command. Must be called BEFORE input file.
pub fn add_hardware_decoding_to_command(cmd: &mut FfmpegCommand, use_hw_decode: bool) -> bool {
    if !use_hw_decode {
        return false;
    }

    if is_macos() {
        cmd.arg("-hwaccel");
        cmd.arg("videotoolbox");
        return true;
    } else if is_linux() && is_vaapi_available() {
        if let Some(device) = detect_vaapi_device() {
            cmd.arg("-hwaccel");
            cmd.arg("vaapi");
            cmd.arg("-hwaccel_device");
            cmd.arg(&device.path);
            if let Some(driver) = device.driver {
                cmd.as_inner_mut().env("LIBVA_DRIVER_NAME", driver);
            }
            return true;
        }
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
#[must_use]
pub fn get_hardware_decoding_info() -> Option<String> {
    if is_macos() {
        Some("VideoToolbox".to_string())
    } else if is_linux() && is_vaapi_available() {
        Some("VAAPI".to_string())
    } else {
        None
    }
}

#[derive(Debug, Clone)]
struct VaapiDevice {
    path: String,
    driver: Option<String>,
}

#[derive(Debug, Clone)]
struct VaapiCandidate {
    path: String,
    render_node: String,
    driver: Option<&'static str>,
    driver_present: bool,
}

impl VaapiCandidate {
    fn matches_driver(&self, driver_name: &str) -> bool {
        self.driver
            .map(|driver| driver.eq_ignore_ascii_case(driver_name))
            .unwrap_or(false)
    }

    fn to_device(&self) -> VaapiDevice {
        VaapiDevice {
            path: self.path.clone(),
            driver: self.driver.map(|d| d.to_string()),
        }
    }
}

fn detect_vaapi_device() -> Option<VaapiDevice> {
    let candidates = gather_vaapi_candidates();
    if candidates.is_empty() {
        return None;
    }

    let env_driver = env::var("LIBVA_DRIVER_NAME").ok();

    if let Some(ref env_driver) = env_driver {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.driver_present && candidate.matches_driver(env_driver))
        {
            return Some(candidate.to_device());
        }
    }

    for preferred in ["nvidia", "radeonsi", "iHD"] {
        if let Some(candidate) = candidates
            .iter()
            .find(|candidate| candidate.driver_present && candidate.matches_driver(preferred))
        {
            return Some(candidate.to_device());
        }
    }

    candidates
        .iter()
        .find(|candidate| candidate.driver_present)
        .map(VaapiCandidate::to_device)
}

fn gather_vaapi_candidates() -> Vec<VaapiCandidate> {
    let mut candidates = Vec::new();

    if let Ok(entries) = fs::read_dir("/dev/dri") {
        for entry in entries.flatten() {
            let file_name = match entry.file_name().to_str() {
                Some(name) if name.starts_with("renderD") => name.to_string(),
                _ => continue,
            };

            let path = format!("/dev/dri/{file_name}");
            let driver = read_vendor_id(&file_name)
                .as_deref()
                .and_then(vendor_to_driver_name);
            let driver_present = driver.map(libva_driver_available).unwrap_or(true);

            candidates.push(VaapiCandidate {
                path,
                render_node: file_name,
                driver,
                driver_present,
            });
        }
    }

    candidates.sort_by(|a, b| a.render_node.cmp(&b.render_node));
    candidates
}

fn read_vendor_id(render_node: &str) -> Option<String> {
    let vendor_path = Path::new("/sys/class/drm")
        .join(render_node)
        .join("device/vendor");
    fs::read_to_string(vendor_path)
        .ok()
        .map(|content| content.trim().to_lowercase())
}

fn vendor_to_driver_name(vendor: &str) -> Option<&'static str> {
    match vendor {
        "0x10de" => Some("nvidia"),
        "0x1002" | "0x1022" => Some("radeonsi"),
        "0x8086" => Some("iHD"),
        _ => None,
    }
}

fn libva_driver_available(driver: &str) -> bool {
    let driver_file = format!("{driver}_drv_video.so");

    VAAPI_DRIVER_DIRS
        .iter()
        .map(Path::new)
        .map(|dir| dir.join(&driver_file))
        .any(|candidate| candidate.exists())
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
            let hw = HardwareDecoding {
                videotoolbox_decode_available: true,
                vaapi_decode_available: false,
                vaapi_device_path: None,
                vaapi_driver: None,
            };
            let args = hw.get_ffmpeg_hwdecode_args();
            assert_eq!(args, vec!["-hwaccel", "videotoolbox"]);
        }
    }

    #[test]
    fn test_get_ffmpeg_hwdecode_args_linux_with_vaapi() {
        #[cfg(target_os = "linux")]
        {
            let hw = HardwareDecoding {
                videotoolbox_decode_available: false,
                vaapi_decode_available: true,
                vaapi_device_path: Some("/dev/dri/renderD128".to_string()),
                vaapi_driver: Some("nvidia".to_string()),
            };
            let args = hw.get_ffmpeg_hwdecode_args();
            assert_eq!(
                args,
                vec![
                    "-hwaccel",
                    "vaapi",
                    "-hwaccel_device",
                    "/dev/dri/renderD128"
                ]
            );
        }
    }

    #[test]
    fn test_get_ffmpeg_hwdecode_args_no_hardware() {
        let hw = HardwareDecoding {
            videotoolbox_decode_available: false,
            vaapi_decode_available: false,
            vaapi_device_path: None,
            vaapi_driver: None,
        };
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
                assert!(debug_output.contains("/dev/dri/renderD"));
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
