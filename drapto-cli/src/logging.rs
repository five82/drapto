// ============================================================================
// drapto-cli/src/logging.rs
// ============================================================================
//
// LOGGING UTILITIES: Helper Functions for Logging
//
// This file contains utility functions related to logging in the Drapto CLI
// application. The main logging implementation uses the standard `log` crate
// with `env_logger` as the backend, configured in main.rs.
//
// KEY COMPONENTS:
// - Timestamp generation for log files and other time-based operations
// - Other logging-related utility functions
//
// USAGE:
// The application uses env_logger with the RUST_LOG environment variable:
// - RUST_LOG=info (default): Normal operation logs
// - RUST_LOG=debug: Detailed debugging information
// - RUST_LOG=trace: Very verbose debugging information
//
// AI-ASSISTANT-INFO: Logging utilities and helper functions

/// Returns the current local timestamp formatted as "YYYYMMDD_HHMMSS".
///
/// This function is used to generate unique timestamps for log files,
/// temporary directories, and other time-based operations.
///
/// # Returns
/// A string containing the formatted timestamp (e.g., "20240601_123045")
///
/// # Example
/// ```
/// let log_filename = format!("drapto_log_{}.txt", get_timestamp());
/// // Result: "drapto_log_20240601_123045.txt"
/// ```
pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// Note: The previous custom log callback system has been replaced by
// standard `log` macros and `env_logger` initialization in `main.rs`.
// This provides better integration with the Rust ecosystem and
// standardized logging levels (error, warn, info, debug, trace).