//! Utility functions for logging
//!
//! This module contains utility functions for logging used throughout the codebase.

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