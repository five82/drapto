//! Platform-specific functionality and detection.
//!
//! This module provides platform abstractions including OS detection
//! and hardware decoding capabilities.

// Re-export platform detection and hardware capabilities from core
pub use drapto_core::hardware_decode::{is_macos, HardwareDecoding};
