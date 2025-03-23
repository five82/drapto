//! Summary reporting module
//!
//! This module handles generation of encoding summaries,
//! statistics, and report formatting.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use std::fmt;
use chrono::{DateTime, Local};
use serde::{Serialize, Deserialize};

/// Simplified validation message for summary reports
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationMessageSummary {
    /// Message text
    pub message: String,
    
    /// Validation severity level
    pub level: String,
    
    /// Validation category
    pub category: String,
}

use crate::error::{DraptoError, Result};
use crate::validation::{ValidationLevel, ValidationReport};
use crate::encoding::pipeline::PipelineStats;

/// Summary of an encoding job with detailed statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodingSummary {
    /// Input file path
    pub input_path: PathBuf,
    
    /// Output file path
    pub output_path: PathBuf,
    
    /// Input file size in bytes
    pub input_size: u64,
    
    /// Output file size in bytes
    pub output_size: u64,
    
    /// Size reduction percentage
    pub reduction_percent: f64,
    
    /// Encoding time in seconds
    pub encoding_time: f64,
    
    /// Number of segments used
    pub segment_count: usize,
    
    /// Number of audio tracks
    pub audio_track_count: usize,
    
    /// Timestamp when encoding completed
    pub completion_time: DateTime<Local>,
    
    /// Input file format
    pub input_format: Option<String>,
    
    /// Output file format
    pub output_format: Option<String>,
    
    /// Validation results
    pub validation: ValidationResult,
}

/// Summary results of validation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    /// Overall validation status
    pub passed: bool,
    
    /// Number of errors
    pub errors: usize,
    
    /// Number of warnings
    pub warnings: usize,
    
    /// Video validation passed
    pub video_passed: bool,
    
    /// Audio validation passed
    pub audio_passed: bool,
    
    /// A/V sync validation passed
    pub sync_passed: bool,
    
    /// Subtitles validation passed
    pub subtitles_passed: bool,
    
    /// Duration validation passed
    pub duration_passed: bool,
    
    /// Codec validation passed
    pub codec_passed: bool,
    
    /// Quality validation passed
    pub quality_passed: bool,
    
    /// Category-specific validation details
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub categories: Option<std::collections::HashMap<String, (usize, usize)>>,
    
    /// Detailed validation messages
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<ValidationMessageSummary>,
}

impl EncodingSummary {
    /// Create a new encoding summary from pipeline stats
    pub fn from_pipeline_stats(stats: &PipelineStats) -> Self {
        let completion_time = Local::now();
        
        Self {
            input_path: PathBuf::from(&stats.input_file),
            output_path: PathBuf::from(&stats.output_file),
            input_size: stats.input_size,
            output_size: stats.output_size,
            reduction_percent: stats.reduction_percent as f64,
            encoding_time: stats.encoding_time,
            segment_count: stats.segment_count,
            audio_track_count: stats.audio_track_count,
            completion_time,
            input_format: None,
            output_format: None,
            validation: ValidationResult {
                passed: stats.validation_summary.overall_passed,
                errors: stats.validation_summary.error_count,
                warnings: stats.validation_summary.warning_count,
                video_passed: stats.validation_summary.video_passed,
                audio_passed: stats.validation_summary.audio_passed,
                sync_passed: stats.validation_summary.sync_passed,
                subtitles_passed: stats.validation_summary.subtitles_passed,
                duration_passed: stats.validation_summary.duration_passed,
                codec_passed: stats.validation_summary.codec_passed,
                quality_passed: stats.validation_summary.quality_passed,
                categories: Some(stats.validation_summary.category_stats.clone()),
                messages: if let Some(messages) = &stats.validation_messages {
                    messages.iter()
                        .filter(|msg| matches!(msg.level, ValidationLevel::Error | ValidationLevel::Warning))
                        .map(|msg| ValidationMessageSummary {
                            message: msg.message.clone(),
                            level: msg.level.to_string(),
                            category: msg.category.clone(),
                        })
                        .collect()
                } else {
                    Vec::new()
                },
            },
        }
    }
    
    /// Create a new encoding summary from file paths and encoding duration
    pub fn new(
        input_path: &Path,
        output_path: &Path,
        encoding_duration: Duration,
        validation_report: Option<&ValidationReport>
    ) -> Result<Self> {
        // Get file metadata
        let input_metadata = match std::fs::metadata(input_path) {
            Ok(m) => m,
            Err(e) => return Err(DraptoError::Io(e)),
        };
        
        let output_metadata = match std::fs::metadata(output_path) {
            Ok(m) => m,
            Err(e) => return Err(DraptoError::Io(e)),
        };
        
        let input_size = input_metadata.len();
        let output_size = output_metadata.len();
        
        // Calculate size reduction
        let reduction_percent = if input_size > 0 {
            ((input_size as f64 - output_size as f64) / input_size as f64) * 100.0
        } else {
            0.0
        };
        
        // Create validation result
        let validation = if let Some(report) = validation_report {
            // Count errors and warnings by category
            let mut categories = std::collections::HashMap::new();
            for msg in &report.messages {
                let entry = categories.entry(msg.category.clone()).or_insert((0, 0));
                match msg.level {
                    crate::validation::ValidationLevel::Error => entry.0 += 1,
                    crate::validation::ValidationLevel::Warning => entry.1 += 1,
                    _ => {}
                }
            }
            
            // Collect validation messages
            let messages = report.messages.iter()
                .filter(|msg| matches!(msg.level, ValidationLevel::Error | ValidationLevel::Warning))
                .map(|msg| ValidationMessageSummary {
                    message: msg.message.clone(),
                    level: msg.level.to_string(),
                    category: msg.category.clone(),
                })
                .collect();
            
            ValidationResult {
                passed: report.passed,
                errors: report.errors().len(),
                warnings: report.warnings().len(),
                video_passed: !report.errors().iter().any(|e| e.category == "Video"),
                audio_passed: !report.errors().iter().any(|e| e.category == "Audio"),
                sync_passed: !report.errors().iter().any(|e| e.category == "A/V Sync"),
                subtitles_passed: !report.errors().iter().any(|e| e.category == "Subtitles"),
                duration_passed: !report.errors().iter().any(|e| e.category == "Duration"),
                codec_passed: !report.errors().iter().any(|e| e.category == "Codec"),
                quality_passed: !report.errors().iter().any(|e| e.category == "Quality"),
                categories: Some(categories),
                messages,
            }
        } else {
            ValidationResult {
                passed: true,
                errors: 0,
                warnings: 0,
                video_passed: true,
                audio_passed: true,
                sync_passed: true,
                subtitles_passed: true,
                duration_passed: true,
                codec_passed: true,
                quality_passed: true,
                categories: None,
                messages: Vec::new(),
            }
        };
        
        Ok(Self {
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
            input_size,
            output_size,
            reduction_percent,
            encoding_time: encoding_duration.as_secs_f64(),
            segment_count: 0, // Will be filled in by pipeline
            audio_track_count: 0, // Will be filled in by pipeline
            completion_time: Local::now(),
            input_format: None,
            output_format: None,
            validation,
        })
    }
    
    /// Format file size with appropriate units
    pub fn format_size(size: u64) -> String {
        const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
        
        let mut size_f = size as f64;
        let mut unit_idx = 0;
        
        while size_f >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size_f /= 1024.0;
            unit_idx += 1;
        }
        
        if unit_idx == 0 {
            format!("{} {}", size_f, UNITS[unit_idx])
        } else {
            format!("{:.2} {}", size_f, UNITS[unit_idx])
        }
    }
    
    /// Format time duration as HH:MM:SS
    pub fn format_duration(seconds: f64) -> String {
        let total_seconds = seconds as u64;
        let hours = total_seconds / 3600;
        let minutes = (total_seconds % 3600) / 60;
        let secs = total_seconds % 60;
        
        if hours > 0 {
            format!("{:02}:{:02}:{:02}", hours, minutes, secs)
        } else {
            format!("{:02}:{:02}", minutes, secs)
        }
    }
    
    /// Get the summary as a compact one-line string
    pub fn as_compact_line(&self) -> String {
        let filename = self.input_path.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("unknown"));
            
        format!(
            "{}: {} → {} (-{:.1}%) in {}",
            filename,
            Self::format_size(self.input_size),
            Self::format_size(self.output_size),
            self.reduction_percent,
            Self::format_duration(self.encoding_time)
        )
    }
}

impl fmt::Display for EncodingSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let input_filename = self.input_path.file_name()
            .map(|f| f.to_string_lossy().to_string())
            .unwrap_or_else(|| String::from("unknown"));
        
        writeln!(f, "\n=== Encoding Summary ===\n")?;
        writeln!(f, "File:         {}", input_filename)?;
        writeln!(f, "Input size:   {}", Self::format_size(self.input_size))?;
        writeln!(f, "Output size:  {}", Self::format_size(self.output_size))?;
        writeln!(f, "Reduction:    {:.2}%", self.reduction_percent)?;
        writeln!(f, "Duration:     {}", Self::format_duration(self.encoding_time))?;
        writeln!(f, "Segments:     {}", self.segment_count)?;
        writeln!(f, "Audio tracks: {}", self.audio_track_count)?;
        writeln!(f, "Completed:    {}", self.completion_time.format("%Y-%m-%d %H:%M:%S"))?;
        
        writeln!(f, "\n=== Validation Results ===")?;
        writeln!(f, "Status:       {}", if self.validation.passed { 
            if self.validation.warnings > 0 {
                "⚠ PASSED WITH WARNINGS"
            } else {
                "✓ PASSED" 
            }
        } else { 
            "✗ FAILED" 
        })?;
        writeln!(f, "Errors:       {}", self.validation.errors)?;
        writeln!(f, "Warnings:     {}", self.validation.warnings)?;
        
        writeln!(f, "\n--- Validation by Category ---")?;
        writeln!(f, "Video:        {}", if self.validation.video_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "Audio:        {}", if self.validation.audio_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "A/V Sync:     {}", if self.validation.sync_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "Subtitles:    {}", if self.validation.subtitles_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "Duration:     {}", if self.validation.duration_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "Codec:        {}", if self.validation.codec_passed { "✓ OK" } else { "✗ FAILED" })?;
        writeln!(f, "Quality:      {}", if self.validation.quality_passed { "✓ OK" } else { "✗ FAILED" })?;
        
        // Show detailed category stats if available
        if let Some(categories) = &self.validation.categories {
            if !categories.is_empty() {
                writeln!(f, "\n--- Validation Details ---")?;
                for (category, (errors, warnings)) in categories {
                    let status = if *errors > 0 {
                        "✗"
                    } else if *warnings > 0 {
                        "⚠"
                    } else {
                        "✓"
                    };
                    writeln!(f, "{} {}: {} error(s), {} warning(s)", 
                            status, category, errors, warnings)?;
                }
            }
        }
        
        Ok(())
    }
}

/// A collection of encoding summaries
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSummary {
    /// Individual encoding summaries
    pub summaries: Vec<EncodingSummary>,
    
    /// Start time of the batch process
    pub start_time: DateTime<Local>,
    
    /// End time of the batch process
    pub end_time: DateTime<Local>,
    
    /// Total input size of all files
    pub total_input_size: u64,
    
    /// Total output size of all files
    pub total_output_size: u64,
    
    /// Overall reduction percentage
    pub overall_reduction: f64,
    
    /// Number of succeeded encoding jobs
    pub success_count: usize,
    
    /// Number of failed encoding jobs
    pub failure_count: usize,
}

impl BatchSummary {
    /// Create a new batch summary from individual summaries
    pub fn new(summaries: Vec<EncodingSummary>, start_time: DateTime<Local>) -> Self {
        let end_time = Local::now();
        
        // Calculate total sizes
        let mut total_input_size = 0;
        let mut total_output_size = 0;
        let mut success_count = 0;
        let mut failure_count = 0;
        
        for summary in &summaries {
            total_input_size += summary.input_size;
            total_output_size += summary.output_size;
            
            if summary.validation.passed {
                success_count += 1;
            } else {
                failure_count += 1;
            }
        }
        
        // Calculate overall reduction
        let overall_reduction = if total_input_size > 0 {
            ((total_input_size as f64 - total_output_size as f64) / total_input_size as f64) * 100.0
        } else {
            0.0
        };
        
        Self {
            summaries,
            start_time,
            end_time,
            total_input_size,
            total_output_size,
            overall_reduction,
            success_count,
            failure_count,
        }
    }
}

impl fmt::Display for BatchSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let duration = self.end_time.timestamp() - self.start_time.timestamp();
        
        writeln!(f, "\n====== Batch Summary ======\n")?;
        writeln!(f, "Files processed: {}", self.summaries.len())?;
        writeln!(f, "Succeeded:       {}", self.success_count)?;
        writeln!(f, "Failed:          {}", self.failure_count)?;
        writeln!(f, "Total Duration:  {}", EncodingSummary::format_duration(duration as f64))?;
        writeln!(f, "Total Input:     {}", EncodingSummary::format_size(self.total_input_size))?;
        writeln!(f, "Total Output:    {}", EncodingSummary::format_size(self.total_output_size))?;
        writeln!(f, "Overall Reduction: {:.2}%", self.overall_reduction)?;
        
        if !self.summaries.is_empty() {
            writeln!(f, "\n--- Individual File Summaries ---")?;
            
            for (i, summary) in self.summaries.iter().enumerate() {
                write!(f, "{}. ", i + 1)?;
                let status_icon = if !summary.validation.passed {
                    "✗"
                } else if summary.validation.warnings > 0 {
                    "⚠"
                } else {
                    "✓"
                };
                writeln!(f, "{} {}", status_icon, summary.as_compact_line())?;
                
                // Display validation issues for this file if there are any
                if summary.validation.errors > 0 || summary.validation.warnings > 0 {
                    writeln!(f, "   Validation: {} error(s), {} warning(s)", 
                        summary.validation.errors, summary.validation.warnings)?;
                    
                    // Display detailed messages
                    for msg in &summary.validation.messages {
                        let icon = if msg.level == "ERROR" { "✗" } else { "⚠" };
                        writeln!(f, "   {} [{}] {}: {}", 
                            icon, msg.level, msg.category, msg.message)?;
                    }
                }
            }
        }
        
        Ok(())
    }
}

/// Generate a summary report from pipeline stats
pub fn generate_summary(stats: &PipelineStats) -> EncodingSummary {
    EncodingSummary::from_pipeline_stats(stats)
}

/// Save summary to JSON file
pub fn save_summary_json(summary: &EncodingSummary, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(summary)
        .map_err(|e| DraptoError::Other(format!("Failed to serialize summary: {}", e)))?;
        
    std::fs::write(output_path, json)
        .map_err(|e| DraptoError::Io(e))?;
        
    Ok(())
}

/// Save batch summary to JSON file
pub fn save_batch_summary_json(summary: &BatchSummary, output_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(summary)
        .map_err(|e| DraptoError::Other(format!("Failed to serialize batch summary: {}", e)))?;
        
    std::fs::write(output_path, json)
        .map_err(|e| DraptoError::Io(e))?;
        
    Ok(())
}

/// Utility to measure time and generate a summary
pub struct TimedSummary {
    /// Start time of the encoding process
    start_time: Instant,
    
    /// Input file path
    input_path: PathBuf,
    
    /// Output file path
    output_path: PathBuf,
}

impl TimedSummary {
    /// Start timing for a new encoding process
    pub fn start(input_path: &Path, output_path: &Path) -> Self {
        Self {
            start_time: Instant::now(),
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
        }
    }
    
    /// Complete the timing and generate a summary
    pub fn complete(self, validation_report: Option<&ValidationReport>) -> Result<EncodingSummary> {
        let duration = self.start_time.elapsed();
        EncodingSummary::new(&self.input_path, &self.output_path, duration, validation_report)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;
    use crate::validation::ValidationReport;
    
    #[test]
    fn test_format_size() {
        assert_eq!(EncodingSummary::format_size(500), "500 B");
        assert_eq!(EncodingSummary::format_size(1024), "1.00 KiB");
        assert_eq!(EncodingSummary::format_size(1024 * 1024), "1.00 MiB");
        assert_eq!(EncodingSummary::format_size(2 * 1024 * 1024), "2.00 MiB");
    }
    
    #[test]
    fn test_format_duration() {
        assert_eq!(EncodingSummary::format_duration(30.0), "00:30");
        assert_eq!(EncodingSummary::format_duration(90.0), "01:30");
        assert_eq!(EncodingSummary::format_duration(3661.0), "01:01:01");
    }
    
    #[test]
    fn test_summary_creation() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.mp4");
        let output_path = temp_dir.path().join("output.mkv");
        
        // Create dummy files
        let mut input_file = File::create(&input_path).unwrap();
        let mut output_file = File::create(&output_path).unwrap();
        
        // Write data to input (1MB)
        let input_data = vec![0; 1024 * 1024];
        input_file.write_all(&input_data).unwrap();
        
        // Write data to output (512KB)
        let output_data = vec![0; 512 * 1024];
        output_file.write_all(&output_data).unwrap();
        
        // Create a summary
        let summary = EncodingSummary::new(
            &input_path,
            &output_path,
            Duration::from_secs(60),
            None
        ).unwrap();
        
        // Check values
        assert_eq!(summary.input_size, 1024 * 1024);
        assert_eq!(summary.output_size, 512 * 1024);
        assert_eq!(summary.reduction_percent, 50.0);
        assert_eq!(summary.encoding_time, 60.0);
        assert!(summary.validation.passed);
    }
    
    #[test]
    fn test_summary_with_validation_messages() {
        let temp_dir = tempdir().unwrap();
        let input_path = temp_dir.path().join("input.mp4");
        let output_path = temp_dir.path().join("output.mkv");
        
        // Create dummy files
        let mut input_file = File::create(&input_path).unwrap();
        let mut output_file = File::create(&output_path).unwrap();
        
        // Write data to files
        input_file.write_all(&vec![0; 1024 * 1024]).unwrap();
        output_file.write_all(&vec![0; 512 * 1024]).unwrap();
        
        // Create a validation report with messages
        let mut report = ValidationReport::new();
        report.add_warning("Low bitrate detected", "Video");
        report.add_error("Audio sync issues", "Audio");
        
        // Create a summary with validation
        let summary = EncodingSummary::new(
            &input_path,
            &output_path,
            Duration::from_secs(60),
            Some(&report)
        ).unwrap();
        
        // Check validation values
        assert!(!summary.validation.passed);
        assert_eq!(summary.validation.errors, 1);
        assert_eq!(summary.validation.warnings, 1);
        assert!(summary.validation.video_passed);
        assert!(!summary.validation.audio_passed);
        
        // Check validation messages
        assert_eq!(summary.validation.messages.len(), 2);
        
        // Get message by content for easier testing
        let warning_msg = summary.validation.messages.iter()
            .find(|m| m.level == "WARNING").unwrap();
        let error_msg = summary.validation.messages.iter()
            .find(|m| m.level == "ERROR").unwrap();
            
        assert_eq!(warning_msg.category, "Video");
        assert_eq!(warning_msg.message, "Low bitrate detected");
        assert_eq!(error_msg.category, "Audio");
        assert_eq!(error_msg.message, "Audio sync issues");
    }
    
    #[test]
    fn test_batch_summary_with_validation() {
        let temp_dir = tempdir().unwrap();
        
        // Create two summaries with validation issues
        let mut report1 = ValidationReport::new();
        report1.add_warning("Low bitrate detected", "Video");
        
        let mut report2 = ValidationReport::new();
        report2.add_error("Audio sync issues", "Audio");
        
        let file1 = temp_dir.path().join("file1.mp4");
        let file2 = temp_dir.path().join("file2.mp4");
        let out1 = temp_dir.path().join("out1.mkv");
        let out2 = temp_dir.path().join("out2.mkv");
        
        // Create dummy files
        File::create(&file1).unwrap().write_all(&vec![0; 1024 * 1024]).unwrap();
        File::create(&file2).unwrap().write_all(&vec![0; 2048 * 1024]).unwrap();
        File::create(&out1).unwrap().write_all(&vec![0; 512 * 1024]).unwrap();
        File::create(&out2).unwrap().write_all(&vec![0; 1024 * 1024]).unwrap();
        
        // Create summaries
        let summary1 = EncodingSummary::new(
            &file1, &out1, Duration::from_secs(60), Some(&report1)
        ).unwrap();
        
        let summary2 = EncodingSummary::new(
            &file2, &out2, Duration::from_secs(120), Some(&report2)
        ).unwrap();
        
        // Create batch summary
        let start_time = Local::now();
        let batch_summary = BatchSummary::new(vec![summary1, summary2], start_time);
        
        // Check batch stats
        assert_eq!(batch_summary.success_count, 1);  // One passed with warnings
        assert_eq!(batch_summary.failure_count, 1);  // One failed with errors
        assert_eq!(batch_summary.summaries.len(), 2);
        
        // Validate that summaries retain their validation messages
        let warning_summary = &batch_summary.summaries[0];
        let error_summary = &batch_summary.summaries[1];
        
        assert_eq!(warning_summary.validation.warnings, 1);
        assert_eq!(warning_summary.validation.errors, 0);
        assert_eq!(error_summary.validation.warnings, 0);
        assert_eq!(error_summary.validation.errors, 1);
        
        // Check that messages are preserved
        assert_eq!(warning_summary.validation.messages.len(), 1);
        assert_eq!(error_summary.validation.messages.len(), 1);
        assert_eq!(warning_summary.validation.messages[0].category, "Video");
        assert_eq!(error_summary.validation.messages[0].category, "Audio");
    }
    
    #[test]
    fn test_batch_summary_without_validation_issues() {
        let temp_dir = tempdir().unwrap();
        
        // Create clean validation reports with no issues
        let report1 = ValidationReport::new(); // Clean report
        let report2 = ValidationReport::new(); // Clean report
        
        let file1 = temp_dir.path().join("clean1.mp4");
        let file2 = temp_dir.path().join("clean2.mp4");
        let out1 = temp_dir.path().join("clean_out1.mkv");
        let out2 = temp_dir.path().join("clean_out2.mkv");
        
        // Create dummy files
        File::create(&file1).unwrap().write_all(&vec![0; 1024 * 1024]).unwrap();
        File::create(&file2).unwrap().write_all(&vec![0; 2048 * 1024]).unwrap();
        File::create(&out1).unwrap().write_all(&vec![0; 512 * 1024]).unwrap();
        File::create(&out2).unwrap().write_all(&vec![0; 1024 * 1024]).unwrap();
        
        // Create summaries with clean validation
        let summary1 = EncodingSummary::new(
            &file1, &out1, Duration::from_secs(45), Some(&report1)
        ).unwrap();
        
        let summary2 = EncodingSummary::new(
            &file2, &out2, Duration::from_secs(90), Some(&report2)
        ).unwrap();
        
        // Create batch summary
        let start_time = Local::now();
        let batch_summary = BatchSummary::new(vec![summary1, summary2], start_time);
        
        // Check batch stats
        assert_eq!(batch_summary.success_count, 2);  // Both should succeed
        assert_eq!(batch_summary.failure_count, 0);  // No failures
        assert_eq!(batch_summary.summaries.len(), 2);
        
        // Validate that there are no validation issues
        for summary in &batch_summary.summaries {
            assert!(summary.validation.passed);
            assert_eq!(summary.validation.errors, 0);
            assert_eq!(summary.validation.warnings, 0);
            assert!(summary.validation.messages.is_empty());
        }
        
        // Format the batch summary as a string
        let formatted = batch_summary.to_string();
        
        // Verify it contains success indicators but no validation details
        assert!(formatted.contains("✓"));
        assert!(!formatted.contains("✗"));
        assert!(!formatted.contains("⚠"));
        assert!(!formatted.contains("error(s)"));
        assert!(!formatted.contains("warning(s)"));
        assert!(!formatted.contains("Validation:"));
    }
}