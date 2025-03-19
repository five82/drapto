use std::path::Path;
use std::fmt::{self, Display};

use serde::{Serialize, Deserialize};
use log::{info, error};

use crate::error::{DraptoError, Result};
use crate::ffprobe::MediaInfo;

pub mod audio;
pub mod video;
pub mod subtitles;
pub mod quality;

/// Validation severity level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ValidationLevel {
    Info,
    Warning,
    Error,
}

impl Display for ValidationLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ValidationLevel::Info => write!(f, "INFO"),
            ValidationLevel::Warning => write!(f, "WARNING"),
            ValidationLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// A validation message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessage {
    /// Message text
    pub message: String,
    
    /// Validation severity level
    pub level: ValidationLevel,
    
    /// Validation category
    pub category: String,
}

impl ValidationMessage {
    /// Create a new info message
    pub fn info<S: Into<String>, C: Into<String>>(message: S, category: C) -> Self {
        Self {
            message: message.into(),
            level: ValidationLevel::Info,
            category: category.into(),
        }
    }
    
    /// Create a new warning message
    pub fn warning<S: Into<String>, C: Into<String>>(message: S, category: C) -> Self {
        Self {
            message: message.into(),
            level: ValidationLevel::Warning,
            category: category.into(),
        }
    }
    
    /// Create a new error message
    pub fn error<S: Into<String>, C: Into<String>>(message: S, category: C) -> Self {
        Self {
            message: message.into(),
            level: ValidationLevel::Error,
            category: category.into(),
        }
    }
}

impl Display for ValidationMessage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}: {}", self.level, self.category, self.message)
    }
}

/// Validation report
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValidationReport {
    /// Validation messages
    pub messages: Vec<ValidationMessage>,
    
    /// Whether validation passed (no errors)
    pub passed: bool,
}

impl ValidationReport {
    /// Create a new validation report
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            passed: true,
        }
    }
    
    /// Add a validation message
    pub fn add_message(&mut self, message: ValidationMessage) {
        if message.level == ValidationLevel::Error {
            self.passed = false;
        }
        self.messages.push(message);
    }
    
    /// Add an info message
    pub fn add_info<S: Into<String>, C: Into<String>>(&mut self, message: S, category: C) {
        self.add_message(ValidationMessage::info(message, category));
    }
    
    /// Add a warning message
    pub fn add_warning<S: Into<String>, C: Into<String>>(&mut self, message: S, category: C) {
        self.add_message(ValidationMessage::warning(message, category));
    }
    
    /// Add an error message
    pub fn add_error<S: Into<String>, C: Into<String>>(&mut self, message: S, category: C) {
        self.add_message(ValidationMessage::error(message, category));
        self.passed = false;
    }
    
    /// Get all error messages
    pub fn errors(&self) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.level == ValidationLevel::Error)
            .collect()
    }
    
    /// Get all warning messages
    pub fn warnings(&self) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.level == ValidationLevel::Warning)
            .collect()
    }
    
    /// Get all info messages
    pub fn infos(&self) -> Vec<&ValidationMessage> {
        self.messages
            .iter()
            .filter(|m| m.level == ValidationLevel::Info)
            .collect()
    }
    
    /// Generate a formatted validation report
    pub fn format(&self) -> String {
        let mut lines = Vec::new();
        
        lines.push(format!("Validation Report - {}", 
            if self.passed { "PASSED" } else { "FAILED" }
        ));
        lines.push("-".repeat(80));
        
        // Group messages by category
        let mut categories = std::collections::HashMap::new();
        for msg in &self.messages {
            categories
                .entry(msg.category.clone())
                .or_insert_with(Vec::new)
                .push(msg);
        }
        
        // Print messages by category
        for (category, messages) in categories {
            lines.push(format!("Category: {}", category));
            
            // Sort by level (errors first, then warnings, then info)
            let mut sorted_messages = messages.clone();
            sorted_messages.sort_by_key(|m| match m.level {
                ValidationLevel::Error => 0,
                ValidationLevel::Warning => 1,
                ValidationLevel::Info => 2,
            });
            
            for msg in sorted_messages {
                lines.push(format!("  [{}] {}", msg.level, msg.message));
            }
            
            lines.push("".to_string());
        }
        
        // Add summary
        let error_count = self.errors().len();
        let warning_count = self.warnings().len();
        let info_count = self.infos().len();
        
        lines.push("-".repeat(80));
        lines.push(format!("Summary: {} error(s), {} warning(s), {} info message(s)",
            error_count, warning_count, info_count
        ));
        
        lines.join("\n")
    }
}

impl Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}

/// Validate a media file
pub fn validate_media<P: AsRef<Path>>(path: P) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();
    let media_info = MediaInfo::from_path(path)?;
    
    // Run various validations
    audio::validate_audio(&media_info, &mut report);
    video::validate_video(&media_info, &mut report);
    subtitles::validate_subtitles(&media_info, &mut report);
    
    if !report.passed {
        error!("Validation failed: {}", report);
        return Err(DraptoError::Validation(
            format!("Media validation failed: {} error(s)", report.errors().len())
        ));
    }
    
    info!("Validation passed: {}", report);
    Ok(report)
}

/// Validate A/V synchronization
pub fn validate_av_sync<P: AsRef<Path>>(path: P) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();
    let media_info = MediaInfo::from_path(path)?;
    
    // Get primary video stream duration
    let video_duration = media_info.primary_video_stream()
        .and_then(|s| s.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
        );
    
    // Get primary audio stream duration
    let audio_duration = media_info.audio_streams()
        .first()
        .and_then(|s| s.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok())
        );
    
    // Check for duration mismatch
    match (video_duration, audio_duration) {
        (Some(vdur), Some(adur)) => {
            // Convert to milliseconds for more precise comparison
            let video_ms = (vdur * 1000.0).round() as i64;
            let audio_ms = (adur * 1000.0).round() as i64;
            
            // Calculate difference in milliseconds
            let diff_ms = (video_ms - audio_ms).abs();
            
            // Acceptable threshold is 500ms by default
            const THRESHOLD_MS: i64 = 500;
            
            if diff_ms <= THRESHOLD_MS {
                report.add_info(
                    format!("A/V sync OK. Difference: {}ms", diff_ms),
                    "AV Sync"
                );
            } else {
                report.add_error(
                    format!("A/V sync error. Difference: {}ms exceeds threshold of {}ms", 
                            diff_ms, THRESHOLD_MS),
                    "AV Sync"
                );
            }
        },
        (None, Some(_)) => {
            report.add_error("Video duration not available for A/V sync check", "AV Sync");
        },
        (Some(_), None) => {
            report.add_error("Audio duration not available for A/V sync check", "AV Sync");
        },
        (None, None) => {
            report.add_error("Neither video nor audio duration available for A/V sync check", "AV Sync");
        }
    }
    
    if !report.passed {
        error!("A/V sync validation failed: {}", report);
        return Err(DraptoError::Validation(
            format!("A/V sync validation failed: {} error(s)", report.errors().len())
        ));
    }
    
    info!("A/V sync validation passed");
    Ok(report)
}