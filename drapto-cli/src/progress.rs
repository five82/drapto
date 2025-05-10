// ============================================================================
// drapto-cli/src/progress.rs
// ============================================================================
//
// PROGRESS REPORTING: CLI-specific progress reporting utilities
//
// This module provides CLI-specific utilities for progress reporting.
// It contains helper functions and types for formatting and displaying
// progress information in a user-friendly way in the terminal.
//
// KEY COMPONENTS:
// - CliProgress: Simple struct for tracking CLI progress state
// - Formatting utilities for terminal output
//
// DESIGN PHILOSOPHY:
// This module uses the direct progress reporting functions from drapto-core
// and provides additional CLI-specific utilities as needed.
//
// AI-ASSISTANT-INFO: CLI-specific progress reporting utilities

// ---- External crate imports ----

// ---- Standard library imports ----


// ============================================================================
// CLI PROGRESS UTILITIES
// ============================================================================

/// Simple struct for tracking CLI progress state.
///
/// This struct provides a simple way to track progress state in the CLI.
/// It can be used to store state that needs to be maintained between
/// progress reporting calls.
#[derive(Debug, Clone, Default)]
pub struct CliProgress {
    /// Whether to use verbose output
    pub verbose: bool,
}

impl CliProgress {
    /// Creates a new CliProgress with the specified verbosity.
    ///
    /// # Arguments
    ///
    /// * `verbose` - Whether to use verbose output
    ///
    /// # Returns
    ///
    /// * A new CliProgress instance
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }

    /// Formats a duration in seconds as a human-readable string.
    ///
    /// # Arguments
    ///
    /// * `seconds` - Duration in seconds
    ///
    /// # Returns
    ///
    /// * A formatted string (e.g., "01:30:45")
    pub fn format_duration_seconds(seconds: f64) -> String {
        let hours = (seconds / 3600.0) as u64;
        let minutes = ((seconds % 3600.0) / 60.0) as u64;
        let secs = (seconds % 60.0) as u64;

        format!("{:02}:{:02}:{:02}", hours, minutes, secs)
    }
}
