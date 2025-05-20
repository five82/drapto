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
    /// Whether to use interactive mode
    pub interactive: bool,
    /// Whether to use verbose output
    pub verbose: bool,
}

impl CliProgress {
    /// Creates a new CliProgress with the specified mode.
    ///
    /// # Arguments
    ///
    /// * `interactive` - Whether to use interactive mode
    ///
    /// # Returns
    ///
    /// * A new CliProgress instance
    pub fn new(interactive: bool) -> Self {
        Self { 
            interactive,
            verbose: false,
        }
    }
    
    /// Display progress information using the terminal module.
    ///
    /// This method is used to display progress information for encoding operations.
    /// It uses the terminal module to format and display the progress.
    ///
    /// # Arguments
    ///
    /// * `percent` - Progress percentage (0-100)
    /// * `current_secs` - Current time position in seconds
    /// * `total_secs` - Total duration in seconds
    /// * `speed` - Encoding speed
    /// * `fps` - Frames per second
    /// * `eta` - Estimated time remaining
    pub fn display_progress(
        &self,
        percent: f32,
        current_secs: f64,
        total_secs: f64,
        speed: f32,
        fps: f32,
        eta: std::time::Duration,
    ) {
        // Only display progress bar in interactive mode
        if self.interactive {
            // Use our terminal module to display progress
            crate::terminal::print_progress_bar(
                percent,
                current_secs,
                total_secs,
                Some(speed),
                Some(fps),
                Some(eta),
            );
        }
    }
    
    /// Process a progress update from the core library.
    ///
    /// This method is called when the core library reports progress during
    /// encoding operations. It forwards the progress information to the
    /// terminal module for display.
    ///
    /// # Arguments
    ///
    /// * `percent` - Progress percentage (0-100)
    /// * `current_secs` - Current time position in seconds
    /// * `total_secs` - Total duration in seconds
    /// * `speed` - Encoding speed
    /// * `fps` - Frames per second
    /// * `eta` - Estimated time remaining
    pub fn process_encode_progress(
        &self,
        percent: f32,
        current_secs: f64,
        total_secs: f64,
        speed: f32,
        fps: f32,
        eta: std::time::Duration,
    ) {
        // Forward to display_progress
        self.display_progress(percent, current_secs, total_secs, speed, fps, eta);
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
