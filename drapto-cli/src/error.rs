//! Error handling utilities for the CLI.
//!
//! This module provides a type alias for consistency with the core library.

use drapto_core::CoreResult;

/// Type alias for CLI results using `CoreError`.
pub type CliResult<T> = CoreResult<T>;
