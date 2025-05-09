// ============================================================================
// drapto-core/src/progress.rs
// ============================================================================
//
// PROGRESS REPORTING: Encoding Progress Callbacks and Events
//
// This module provides abstractions for reporting encoding progress and events
// from the core library to consumers. It defines a set of event types and a
// callback mechanism that allows consumers to receive and handle these events.
//
// KEY COMPONENTS:
// - ProgressEvent: Enum of different progress event types
// - ProgressCallback: Trait for receiving progress events
// - NullProgressCallback: No-op implementation for when callbacks aren't needed
//
// DESIGN PHILOSOPHY:
// This module follows the observer pattern, allowing consumers to register
// callbacks that will be notified of progress events. This decouples the core
// library from presentation concerns, making it more flexible and testable.
//
// AI-ASSISTANT-INFO: Progress reporting abstractions and callback system

// ---- Standard library imports ----
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// PROGRESS EVENTS
// ============================================================================

/// Represents different types of progress events that can occur during encoding.
///
/// This enum defines the various events that can be reported during the encoding
/// process, such as encoding start, progress updates, and completion.
#[derive(Debug, Clone)]
pub enum ProgressEvent {
    /// Encoding process has started for a file
    EncodeStart {
        /// Path to the input file
        input_path: PathBuf,
        /// Path to the output file
        output_path: PathBuf,
        /// Whether hardware acceleration is being used
        using_hw_accel: bool,
    },
    
    /// Progress update during encoding
    EncodeProgress {
        /// Current progress percentage (0.0 to 100.0)
        percent: f32,
        /// Current time position in seconds
        current_secs: f64,
        /// Total duration in seconds
        total_secs: f64,
        /// Encoding speed (e.g., 2.5x means 2.5x realtime)
        speed: f32,
        /// Average frames per second
        fps: f32,
        /// Estimated time remaining
        eta: Duration,
    },
    
    /// Encoding process has completed for a file
    EncodeComplete {
        /// Path to the input file
        input_path: PathBuf,
        /// Path to the output file
        output_path: PathBuf,
        /// Size of the input file in bytes
        input_size: u64,
        /// Size of the output file in bytes
        output_size: u64,
        /// Total encoding time
        duration: Duration,
    },
    
    /// An error occurred during encoding
    EncodeError {
        /// Path to the input file
        input_path: PathBuf,
        /// Error message
        message: String,
    },
    
    /// Hardware acceleration status
    HardwareAcceleration {
        /// Whether hardware acceleration is available
        available: bool,
        /// Type of hardware acceleration (e.g., "VideoToolbox")
        acceleration_type: String,
    },
    
    /// General log message
    LogMessage {
        /// Log message
        message: String,
        /// Log level (info, warn, error, etc.)
        level: LogLevel,
    },
}

/// Log levels for progress events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    /// Debug-level message (verbose)
    Debug,
    /// Informational message
    Info,
    /// Warning message
    Warning,
    /// Error message
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warning => write!(f, "WARNING"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

// ============================================================================
// PROGRESS CALLBACK
// ============================================================================

/// Trait for receiving progress events during encoding.
///
/// This trait defines the interface for receiving progress events from the
/// encoding process. Consumers can implement this trait to handle these events
/// in a custom way, such as updating a UI or logging to a file.
pub trait ProgressCallback: Send + Sync {
    /// Called when a progress event occurs.
    ///
    /// # Arguments
    ///
    /// * `event` - The progress event that occurred
    fn on_progress(&self, event: ProgressEvent);
}

/// No-op implementation of ProgressCallback that does nothing.
///
/// This implementation is useful when progress reporting is not needed,
/// such as in tests or when running in a non-interactive environment.
#[derive(Debug, Clone, Default)]
pub struct NullProgressCallback;

impl ProgressCallback for NullProgressCallback {
    fn on_progress(&self, _event: ProgressEvent) {
        // Do nothing
    }
}
