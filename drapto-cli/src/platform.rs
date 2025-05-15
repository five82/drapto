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

// ---- External crate imports ----

// ============================================================================
// PLATFORM DETECTION
// ============================================================================

/// Re-export is_macos from hardware_accel module
pub use drapto_core::hardware_accel::is_macos;

// ============================================================================
// HARDWARE ACCELERATION
// ============================================================================

/// Re-export HardwareAcceleration from hardware_accel module
pub use drapto_core::hardware_accel::HardwareAcceleration;
