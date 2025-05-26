// ============================================================================
// drapto-cli/src/error.rs
// ============================================================================
//
// CLI ERROR HANDLING: Error types and utilities for the CLI
//
// This module provides error handling utilities for the CLI that integrate
// with the drapto-core error types while adding CLI-specific error contexts.
//
// KEY COMPONENTS:
// - CliResult: Type alias for CLI operations
// - Error conversion utilities
//
// AI-ASSISTANT-INFO: CLI error handling utilities

// ---- Internal crate imports ----
use drapto_core::{CoreError, CoreResult};

// ---- Standard library imports ----
use std::fmt;

// ============================================================================
// RESULT TYPE ALIAS
// ============================================================================

/// Type alias for CLI results using CoreError.
///
/// This provides consistency with the core library while allowing
/// CLI-specific error handling when needed.
pub type CliResult<T> = CoreResult<T>;

// ============================================================================
// ERROR CONVERSION UTILITIES
// ============================================================================

/// Extension trait for adding context to errors in the CLI.
///
/// This trait provides methods similar to anyhow's context methods
/// but converts to CoreError instead.
pub trait CliErrorContext<T> {
    /// Add context to an error.
    fn cli_context<C>(self, context: C) -> CliResult<T>
    where
        C: fmt::Display;

    /// Add context using a closure (for lazy evaluation).
    fn cli_with_context<C, F>(self, f: F) -> CliResult<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C;
}

impl<T, E> CliErrorContext<T> for Result<T, E>
where
    E: Into<CoreError>,
{
    fn cli_context<C>(self, context: C) -> CliResult<T>
    where
        C: fmt::Display,
    {
        self.map_err(|e| {
            let core_error: CoreError = e.into();
            CoreError::OperationFailed(format!("{}: {}", context, core_error))
        })
    }

    fn cli_with_context<C, F>(self, f: F) -> CliResult<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C,
    {
        self.map_err(|e| {
            let core_error: CoreError = e.into();
            CoreError::OperationFailed(format!("{}: {}", f(), core_error))
        })
    }
}

impl<T> CliErrorContext<T> for Option<T> {
    fn cli_context<C>(self, context: C) -> CliResult<T>
    where
        C: fmt::Display,
    {
        self.ok_or_else(|| CoreError::OperationFailed(context.to_string()))
    }

    fn cli_with_context<C, F>(self, f: F) -> CliResult<T>
    where
        C: fmt::Display,
        F: FnOnce() -> C,
    {
        self.ok_or_else(|| CoreError::OperationFailed(f().to_string()))
    }
}

/// Creates a CLI error with a formatted message.
///
/// This is a convenience macro similar to anyhow! but creates a CoreError.
#[macro_export]
macro_rules! cli_error {
    ($($arg:tt)*) => {
        $crate::drapto_core::CoreError::OperationFailed(format!($($arg)*))
    };
}
