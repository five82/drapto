//! Utility logging functions (Deprecated)
//!
//! Responsibilities:
//! - Re-export logging functions from main logging module
//! - Provide backwards compatibility for existing code
//! - Warn about deprecated usage in favor of main logging module
//! - Maintain interface stability during transition
//! - Support legacy code paths using the util::logging module
//!
//! This deprecated module serves as a compatibility layer, redirecting
//! to the main logging.rs module. New code should use drapto_core::logging directly.

// This file is being deprecated in favor of the main logging.rs module
// Future development should use drapto_core::logging instead

use log::warn;

/// Emit a deprecation warning for this module
pub fn deprecation_warning() {
    warn!("The util::logging module is deprecated. Use drapto_core::logging instead.");
}

// Re-export the main logging functions
pub use crate::logging::{
    init as init_logger,
    init_with_level,
    log_progress,
    log_command,
    log_section,
    log_subsection,
    log_status,
    log_memory_stats,
    log_segment_completion
};