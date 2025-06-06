// ============================================================================
// drapto-core/src/notifications/abstraction.rs
// ============================================================================
//
// NOTIFICATION TYPES: Notification System Type Definitions
//
// This module defines the notification types used throughout the application.
// It provides a simple enum for different notification scenarios without
// unnecessary abstraction layers.
//
// KEY COMPONENTS:
// - NotificationType: Enum of different notification types
//
// DESIGN PHILOSOPHY:
// This module follows a minimalist approach, focusing on the data structures
// needed for notifications without adding unnecessary abstraction layers.
//
// AI-ASSISTANT-INFO: Notification system type definitions

// ---- Standard library imports ----
use std::path::PathBuf;
use std::time::Duration;

// ============================================================================
// NOTIFICATION TYPES
// ============================================================================

/// Represents different types of notifications that can be sent.
///
/// This enum defines the various notifications that can be sent during the
/// encoding process, such as encoding start, completion, and errors.
#[derive(Debug, Clone)]
pub enum NotificationType {
    /// Encoding process has started for a file
    EncodeStart {
        /// Path to the input file
        input_path: PathBuf,
        /// Path to the output file
        output_path: PathBuf,
        /// Hostname of the machine performing the encoding
        hostname: String,
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
        /// Hostname of the machine performing the encoding
        hostname: String,
    },

    /// An error occurred during encoding
    EncodeError {
        /// Path to the input file
        input_path: PathBuf,
        /// Error message
        message: String,
        /// Hostname of the machine performing the encoding
        hostname: String,
    },

    /// A custom notification message
    Custom {
        /// Title of the notification
        title: String,
        /// Message body
        message: String,
        /// Priority level (1-5, with 5 being highest)
        priority: u8,
    },
}

impl NotificationType {
    /// Gets the title for this notification type.
    ///
    /// # Returns
    ///
    /// * A string representing the title for this notification
    pub fn get_title(&self) -> String {
        match self {
            NotificationType::EncodeStart { .. } => "Encoding Started".to_string(),
            NotificationType::EncodeComplete { .. } => "Encoding Complete".to_string(),
            NotificationType::EncodeError { .. } => "Encoding Error".to_string(),
            NotificationType::Custom { title, .. } => title.clone(),
        }
    }

    /// Gets the message body for this notification type.
    ///
    /// # Returns
    ///
    /// * A string representing the message body for this notification
    pub fn get_message(&self) -> String {
        match self {
            NotificationType::EncodeStart {
                input_path,
                hostname,
                ..
            } => {
                let filename = input_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.to_string_lossy().to_string());
                format!("Started encoding {} on {}", filename, hostname)
            }
            NotificationType::EncodeComplete {
                input_path,
                input_size,
                output_size,
                duration,
                hostname,
                ..
            } => {
                let filename = input_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.to_string_lossy().to_string());

                // Calculate size reduction percentage
                let reduction = if *input_size > 0 {
                    100 - ((output_size * 100) / input_size)
                } else {
                    0
                };

                let duration_secs = duration.as_secs();
                let duration_str = if duration_secs >= 3600 {
                    format!(
                        "{}h {}m {}s",
                        duration_secs / 3600,
                        (duration_secs % 3600) / 60,
                        duration_secs % 60
                    )
                } else if duration_secs >= 60 {
                    format!("{}m {}s", duration_secs / 60, duration_secs % 60)
                } else {
                    format!("{}s", duration_secs)
                };

                format!(
                    "Completed encoding {} on {} in {}. Reduced by {}%",
                    filename, hostname, duration_str, reduction
                )
            }
            NotificationType::EncodeError {
                input_path,
                message,
                hostname,
            } => {
                let filename = input_path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| input_path.to_string_lossy().to_string());
                format!("Error encoding {} on {}: {}", filename, hostname, message)
            }
            NotificationType::Custom { message, .. } => message.clone(),
        }
    }

    /// Gets the priority level for this notification type.
    ///
    /// # Returns
    ///
    /// * A priority level (1-5, with 5 being highest)
    pub fn get_priority(&self) -> u8 {
        match self {
            NotificationType::EncodeStart { .. } => 3,
            NotificationType::EncodeComplete { .. } => 4,
            NotificationType::EncodeError { .. } => 5,
            NotificationType::Custom { priority, .. } => *priority,
        }
    }
}
