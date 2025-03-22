use std::fmt::{self, Display};
use serde::{Serialize, Deserialize};

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
    
    /// Add a validation message with immediate logging
    pub fn add_message(&mut self, message: ValidationMessage) {
        // Log the validation message immediately
        match message.level {
            ValidationLevel::Info => log::info!("[{}] {}: {}", message.level, message.category, message.message),
            ValidationLevel::Warning => log::warn!("[{}] {}: {}", message.level, message.category, message.message),
            ValidationLevel::Error => log::error!("[{}] {}: {}", message.level, message.category, message.message),
        }
        
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
        
        // Create a more prominent header
        lines.push("=".repeat(80));
        lines.push(format!("VALIDATION REPORT SUMMARY - {}", 
            if self.passed { 
                "✅ PASSED" 
            } else { 
                "❌ FAILED" 
            }
        ));
        lines.push("=".repeat(80));
        
        // Group messages by category
        let mut categories = std::collections::HashMap::new();
        for msg in &self.messages {
            categories
                .entry(msg.category.clone())
                .or_insert_with(Vec::new)
                .push(msg);
        }
        
        // Count errors and warnings by category
        let mut category_stats = std::collections::HashMap::new();
        for (category, messages) in &categories {
            let error_count = messages.iter().filter(|m| m.level == ValidationLevel::Error).count();
            let warning_count = messages.iter().filter(|m| m.level == ValidationLevel::Warning).count();
            category_stats.insert(category.clone(), (error_count, warning_count));
        }
        
        // Print validation summary by category first
        lines.push("VALIDATION BY CATEGORY:".to_string());
        for (category, (errors, warnings)) in &category_stats {
            let status = if *errors > 0 {
                "❌ FAILED"
            } else if *warnings > 0 {
                "⚠️ PASSED WITH WARNINGS"
            } else {
                "✅ PASSED"
            };
            
            // Only show error and warning counts if there are any
            if *errors > 0 || *warnings > 0 {
                lines.push(format!("  {} - {}: {} error(s), {} warning(s)", 
                    status, category, errors, warnings));
            } else {
                lines.push(format!("  {} - {}", status, category));
            }
        }
        
        lines.push("".to_string());
        lines.push("DETAILED REPORT:".to_string());
        lines.push("-".repeat(80));
        
        // Print detailed messages by category
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
                let prefix = match msg.level {
                    ValidationLevel::Error => "❌",
                    ValidationLevel::Warning => "⚠️",
                    ValidationLevel::Info => "ℹ️",
                };
                lines.push(format!("  {} [{}] {}", prefix, msg.level, msg.message));
            }
            
            lines.push("".to_string());
        }
        
        // Add summary
        let error_count = self.errors().len();
        let warning_count = self.warnings().len();
        let info_count = self.infos().len();
        
        lines.push("=".repeat(80));
        lines.push(format!("OVERALL SUMMARY: {} error(s), {} warning(s), {} info message(s)",
            error_count, warning_count, info_count
        ));
        
        // Status conclusion
        if self.passed {
            if warning_count > 0 {
                lines.push("⚠️ VALIDATION PASSED WITH WARNINGS - Output may have minor issues".to_string());
            } else {
                lines.push("✅ VALIDATION PASSED - Output meets all quality criteria".to_string());
            }
        } else {
            lines.push("❌ VALIDATION FAILED - Output has quality issues that should be addressed".to_string());
        }
        lines.push("=".repeat(80));
        
        lines.join("\n")
    }
}

impl Display for ValidationReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.format())
    }
}