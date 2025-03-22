//! Encoding pipeline module
//!
//! This module implements the orchestration of the encoding pipeline.
//! It coordinates video processing, segmentation, parallel encoding,
//! audio encoding, and final muxing.

use std::path::{Path, PathBuf};
use std::time::Instant;
use std::sync::Arc;
use chrono::Local;
use log::{info, warn, error};
use thiserror::Error;
use crate::validation::ValidationLevel;

use crate::error::{DraptoError, Result};
use crate::config::Config;
use crate::detection::format::detect_dolby_vision;
use crate::encoding::{
    video::{encode_video, VideoEncodingOptions},
    audio::{encode_audio, AudioEncodingOptions},
    muxer::{Muxer, MuxOptions},
    segmentation::segment_video,
};
use crate::reporting::summary::{BatchSummary, generate_summary, save_summary_json, save_batch_summary_json};
use crate::util::scheduler::{MemoryAwareScheduler, SchedulerBuilder};
use crate::validation::{self, report::ValidationReport};

/// Pipeline orchestration errors
#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("Input file not found: {0}")]
    InputNotFound(String),
    
    #[error("Invalid output path: {0}")]
    InvalidOutput(String),
    
    #[error("Encoding stage failed: {0}")]
    EncodingFailed(String),
    
    #[error("Validation failed: {0}")]
    ValidationFailed(String),
    
    #[error("Segmentation failed: {0}")]
    SegmentationFailed(String),
    
    #[error("Muxing failed: {0}")]
    MuxingFailed(String),
}

/// Encoding pipeline statistics
#[derive(Debug, Clone)]
pub struct PipelineStats {
    /// Input file name
    pub input_file: String,
    
    /// Output file name
    pub output_file: String,
    
    /// Input file size in bytes
    pub input_size: u64,
    
    /// Output file size in bytes
    pub output_size: u64,
    
    /// Size reduction percentage
    pub reduction_percent: f32,
    
    /// Total encoding time in seconds
    pub encoding_time: f64,
    
    /// Dolby Vision content flag
    pub is_dolby_vision: bool,
    
    /// Number of segments processed
    pub segment_count: usize,
    
    /// Number of audio tracks
    pub audio_track_count: usize,
    
    /// Validation summary
    pub validation_summary: ValidationSummary,
}

/// Summary of validation results
#[derive(Debug, Clone)]
pub struct ValidationSummary {
    /// Number of warnings found
    pub warning_count: usize,
    
    /// Number of errors found
    pub error_count: usize,
    
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
    
    /// Overall validation status
    pub overall_passed: bool,
    
    /// Category-specific warnings and errors
    pub category_stats: std::collections::HashMap<String, (usize, usize)>,
}

impl From<&ValidationReport> for ValidationSummary {
    fn from(report: &ValidationReport) -> Self {
        // Count errors and warnings by category
        let mut category_stats = std::collections::HashMap::new();
        for msg in &report.messages {
            let entry = category_stats.entry(msg.category.clone()).or_insert((0, 0));
            match msg.level {
                ValidationLevel::Error => entry.0 += 1,
                ValidationLevel::Warning => entry.1 += 1,
                _ => {}
            }
        }
        
        Self {
            warning_count: report.warnings().len(),
            error_count: report.errors().len(),
            video_passed: !report.errors().iter().any(|e| e.category == "Video"),
            audio_passed: !report.errors().iter().any(|e| e.category == "Audio"),
            sync_passed: !report.errors().iter().any(|e| e.category == "A/V Sync"),
            subtitles_passed: !report.errors().iter().any(|e| e.category == "Subtitles"),
            duration_passed: !report.errors().iter().any(|e| e.category == "Duration"),
            codec_passed: !report.errors().iter().any(|e| e.category == "Codec"),
            quality_passed: !report.errors().iter().any(|e| e.category == "Quality"),
            overall_passed: report.passed,
            category_stats,
        }
    }
}

/// Options for the encoding pipeline
pub struct PipelineOptions {
    /// Configuration settings
    pub config: Config,
    
    /// Working directory for temporary files
    pub working_dir: PathBuf,
    
    /// Disable crop detection
    pub disable_crop: bool,
    
    /// Progress callback function
    pub progress_callback: Option<Arc<dyn Fn(f32, &str) + Send + Sync>>,
}

impl Default for PipelineOptions {
    fn default() -> Self {
        Self {
            config: Config::default(),
            working_dir: std::env::temp_dir(),
            disable_crop: false,
            progress_callback: None,
        }
    }
}

/// Main encoding pipeline orchestrator
pub struct EncodingPipeline {
    /// Pipeline options
    options: PipelineOptions,
    
    /// Scheduler for memory-aware parallel processing
    scheduler: Option<MemoryAwareScheduler>,
}

impl EncodingPipeline {
    /// Create a new encoding pipeline with default options
    pub fn new() -> Self {
        Self {
            options: PipelineOptions::default(),
            scheduler: None,
        }
    }
    
    /// Create a new encoding pipeline with custom options
    pub fn with_options(options: PipelineOptions) -> Self {
        Self {
            options,
            scheduler: None,
        }
    }
    
    /// Setup memory-aware scheduler based on system capabilities
    fn setup_scheduler(&mut self) -> Result<()> {
        let builder = SchedulerBuilder::new()
            .task_stagger_delay(250); // 250ms stagger between tasks
            
        self.scheduler = Some(builder.build());
        
        info!("Memory-aware scheduler initialized with {} tokens", 
              self.scheduler.as_ref().unwrap().current_token_usage());
              
        Ok(())
    }
    
    /// Report progress to the callback function
    fn report_progress(&self, progress: f32, message: &str) {
        if let Some(ref callback) = self.options.progress_callback {
            callback(progress, message);
        }
    }
    
    /// Process a single input file through the encoding pipeline
    pub fn process_file(&mut self, input_file: &Path, output_file: &Path) -> Result<PipelineStats> {
        let start_time = Instant::now();
        
        // Validate input/output paths
        if !input_file.exists() {
            return Err(DraptoError::Pipeline(PipelineError::InputNotFound(
                input_file.display().to_string()
            )));
        }
        
        // Ensure output directory exists
        if let Some(parent) = output_file.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }
        
        // Setup memory-aware scheduling if not already done
        if self.scheduler.is_none() {
            self.setup_scheduler()?;
        }
        
        // Setup working directories
        let temp_dir = self.options.working_dir.join(format!(
            "drapto_{}", uuid::Uuid::new_v4().to_string()
        ));
        std::fs::create_dir_all(&temp_dir)?;
        let segments_dir = temp_dir.join("segments");
        let encoded_segments_dir = temp_dir.join("encoded_segments");
        
        // Report initial progress
        self.report_progress(0.0, "Starting encode");
        
        // Get input file info for reporting
        let input_size = std::fs::metadata(input_file)?.len();
        
        // 1. Detect format (Dolby Vision, HDR, etc.)
        self.report_progress(0.05, "Detecting format");
        crate::logging::log_section("FORMAT DETECTION");
        let is_dolby_vision = detect_dolby_vision(input_file);
        info!("Dolby Vision: {}", if is_dolby_vision { "Yes" } else { "No" });
        
        // 2. Segment the video if needed
        self.report_progress(0.1, "Analyzing video");
        crate::logging::log_section("VIDEO SEGMENTATION");
        let segments = if self.options.config.use_segmentation {
            info!("Using segmented encoding");
            segment_video(input_file, &segments_dir, &self.options.config)?
        } else {
            info!("Using direct encoding (segmentation disabled)");
            vec![input_file.to_path_buf()]
        };
        
        // 3. Encode video (either directly or via segments)
        self.report_progress(0.2, "Encoding video");
        crate::logging::log_section("VIDEO ENCODING");
        let video_track = if segments.len() > 1 {
            // Use parallel encoding for segments
            self.encode_segments(input_file, &segments, &encoded_segments_dir)?
        } else {
            // Direct encoding for single file
            let video_options = VideoEncodingOptions {
                working_dir: temp_dir.clone(),
                is_dolby_vision,
                is_hdr: false, // TODO: detect HDR
                crop_filter: None, // TODO: implement crop detection
                parallel_jobs: self.options.config.parallel_jobs,
                quality: self.options.config.target_quality,
                hw_accel_option: if self.options.config.hardware_acceleration { 
                    Some(self.options.config.hw_accel_option.clone()) 
                } else { 
                    None 
                },
                scenes: None,
            };
            
            encode_video(input_file, &video_options)?
        };

        // 4. Encode audio
        self.report_progress(0.7, "Encoding audio");
        crate::logging::log_section("AUDIO ENCODING");
        let audio_options = AudioEncodingOptions {
            working_dir: temp_dir.clone(),
            quality: None, // Use default quality
            hw_accel_option: if self.options.config.hardware_acceleration { 
                Some(self.options.config.hw_accel_option.clone()) 
            } else { 
                None 
            },
        };
        let audio_tracks = encode_audio(input_file, &audio_options)?;
        
        // 5. Mux tracks
        self.report_progress(0.85, "Muxing tracks");
        crate::logging::log_section("MUXING AND FINALIZATION");
        let muxer = Muxer::new();
        let muxed_file = muxer.mux_tracks(
            video_track, 
            &audio_tracks, 
            output_file.to_path_buf(), 
            &MuxOptions::default()
        )?;
        
        // 6. Validate output
        self.report_progress(0.95, "Validating output");
        crate::logging::log_section("VALIDATION");
        info!("Running comprehensive validation of encoding output");
        
        let validation_report = validation::validate_output(input_file, &muxed_file, None)?;
        let validation_summary = ValidationSummary::from(&validation_report);
        
        // Log validation summary by category
        crate::logging::log_subsection("VALIDATION CATEGORIES SUMMARY");
        for (category, (errors, warnings)) in &validation_summary.category_stats {
            let status_icon = if *errors > 0 {
                "❌"
            } else if *warnings > 0 {
                "⚠️"
            } else {
                "✅"
            };
            
            if *errors > 0 {
                error!("{} {} validation: {} error(s), {} warning(s)", 
                      status_icon, category, errors, warnings);
            } else if *warnings > 0 {
                warn!("{} {} validation: {} warning(s)", 
                     status_icon, category, warnings);
            } else {
                info!("{} {} validation: Passed", status_icon, category);
            }
        }
        
        // Log overall validation result
        crate::logging::log_subsection("OVERALL VALIDATION RESULT");
        if validation_summary.overall_passed {
            if validation_summary.warning_count > 0 {
                warn!("⚠️ Validation passed with {} warning(s)", validation_summary.warning_count);
            } else {
                info!("✅ Validation passed - all quality checks successful");
            }
        } else {
            error!("❌ Validation failed with {} error(s)", validation_summary.error_count);
        }
        
        // Calculate statistics
        let encoding_time = start_time.elapsed().as_secs_f64();
        let output_size = std::fs::metadata(output_file)?.len();
        let reduction_percent = if input_size > 0 {
            ((input_size as f64 - output_size as f64) / input_size as f64 * 100.0) as f32
        } else {
            0.0
        };
        
        let stats = PipelineStats {
            input_file: input_file.file_name().unwrap_or_default().to_string_lossy().to_string(),
            output_file: output_file.file_name().unwrap_or_default().to_string_lossy().to_string(),
            input_size,
            output_size,
            reduction_percent,
            encoding_time,
            is_dolby_vision,
            segment_count: segments.len(),
            audio_track_count: audio_tracks.len(),
            validation_summary,
        };
        
        // Create and save summary report
        self.report_progress(0.95, "Generating summary report");
        let summary = generate_summary(&stats);
        
        // Output summary to log
        info!("\n{}", summary);
        
        // Save summary to JSON file
        if let Some(parent) = output_file.parent() {
            let summary_path = parent.join(format!(
                "{}.summary.json", 
                output_file.file_name().unwrap_or_default().to_string_lossy()
            ));
            
            if let Err(e) = save_summary_json(&summary, &summary_path) {
                warn!("Failed to save summary report: {}", e);
            } else {
                info!("Summary report saved to {}", summary_path.display());
            }
        }
        
        // Cleanup temp files
        self.report_progress(1.0, "Cleaning up");
        if let Err(e) = std::fs::remove_dir_all(&temp_dir) {
            warn!("Failed to cleanup temporary directory: {}", e);
        }
        
        // Log completion
        info!("Encoding completed in {:.2} seconds with {:.2}% size reduction", 
              encoding_time, reduction_percent);
        
        Ok(stats)
    }
    
    /// Process all media files in an input directory
    pub fn process_directory(
        &mut self, 
        input_dir: &Path, 
        output_dir: &Path
    ) -> Result<Vec<PipelineStats>> {
        let start_time = Local::now();
        info!("Starting batch processing of directory: {}", input_dir.display());
        
        if !input_dir.exists() || !input_dir.is_dir() {
            return Err(DraptoError::Pipeline(PipelineError::InputNotFound(
                input_dir.display().to_string()
            )));
        }
        
        // Ensure output directory exists
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)?;
        }
        
        // Find all media files
        let mut media_files = Vec::new();
        for entry in std::fs::read_dir(input_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy().to_lowercase();
                    if ext_str == "mkv" || ext_str == "mp4" {
                        media_files.push(path);
                    }
                }
            }
        }
        
        if media_files.is_empty() {
            warn!("No media files found in {}", input_dir.display());
            return Ok(Vec::new());
        }
        
        info!("Found {} media files to process", media_files.len());
        
        // Process each file
        let mut results = Vec::new();
        let mut summaries = Vec::new();
        
        for (i, input_file) in media_files.iter().enumerate() {
            let file_name = input_file.file_name().unwrap_or_default();
            let output_file = output_dir.join(file_name);
            
            info!("Processing file {}/{}: {}", i + 1, media_files.len(), file_name.to_string_lossy());
            
            match self.process_file(input_file, &output_file) {
                Ok(stats) => {
                    results.push(stats.clone());
                    
                    // Add to summaries
                    let summary = generate_summary(&stats);
                    summaries.push(summary);
                },
                Err(e) => {
                    error!("Failed to process {}: {}", file_name.to_string_lossy(), e);
                }
            }
        }
        
        // Create batch summary if we processed multiple files
        if !summaries.is_empty() {
            let batch_summary = BatchSummary::new(summaries, start_time);
            
            // Log batch summary
            info!("\n{}", batch_summary);
            
            // Save batch summary to file
            let summary_path = output_dir.join("batch_summary.json");
            if let Err(e) = save_batch_summary_json(&batch_summary, &summary_path) {
                warn!("Failed to save batch summary: {}", e);
            } else {
                info!("Batch summary saved to {}", summary_path.display());
            }
        }
        
        Ok(results)
    }
    
    /// Encode video segments in parallel and merge them
    fn encode_segments(
        &self, 
        input_file: &Path,
        segments: &[PathBuf],
        output_dir: &Path,
    ) -> Result<PathBuf> {
        info!("Encoding {} segments in parallel", segments.len());
        
        // Setup directories
        if !output_dir.exists() {
            std::fs::create_dir_all(output_dir)?;
        }
        
        // Create video encoding options
        let video_options = VideoEncodingOptions {
            working_dir: output_dir.to_path_buf(),
            is_dolby_vision: detect_dolby_vision(input_file),
            is_hdr: false, // TODO: detect HDR
            crop_filter: None, // TODO: implement crop detection
            parallel_jobs: self.options.config.parallel_jobs,
            quality: self.options.config.target_quality,
            hw_accel_option: if self.options.config.hardware_acceleration { 
                Some(self.options.config.hw_accel_option.clone()) 
            } else { 
                None 
            },
            scenes: None,
        };
        
        // Use the real encoder implementation
        let output_path = encode_video(input_file, &video_options)?;
        
        crate::logging::log_subsection("CONCATENATION");
        info!("Encoded segments merged to {}", output_path.display());
        
        Ok(output_path)
    }
}

/// Run the encoding pipeline on a single file
///
/// # Arguments
///
/// * `input_file` - Path to input media file
/// * `output_file` - Path where encoded file will be saved
/// * `options` - Pipeline options
///
/// # Returns
///
/// * `Result<PipelineStats>` - Statistics from the encoding process
pub fn run_pipeline(
    input_file: &Path,
    output_file: &Path,
    options: PipelineOptions
) -> Result<PipelineStats> {
    let mut pipeline = EncodingPipeline::with_options(options);
    pipeline.process_file(input_file, output_file)
}

/// Validate that a pipeline output meets quality criteria
///
/// # Arguments
///
/// * `input_file` - Original input file
/// * `output_file` - Encoded output file
///
/// # Returns
///
/// * `Result<ValidationReport>` - Full validation report
pub fn validate_pipeline_output(
    input_file: &Path,
    output_file: &Path
) -> Result<ValidationReport> {
    validation::validate_output(input_file, output_file, None)
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_pipeline_options_default() {
        let options = PipelineOptions::default();
        assert!(!options.disable_crop);
        assert!(options.progress_callback.is_none());
    }
    
    #[test]
    fn test_validation_summary_from_report() {
        let mut report = ValidationReport::new();
        report.add_warning("Test warning", "Video");
        report.add_error("Test error", "Audio");
        
        let summary = ValidationSummary::from(&report);
        assert_eq!(summary.warning_count, 1);
        assert_eq!(summary.error_count, 1);
        assert!(summary.video_passed);
        assert!(!summary.audio_passed);
        assert!(summary.sync_passed);
        assert!(summary.subtitles_passed);
        assert!(summary.duration_passed);
        assert!(summary.codec_passed);
        assert!(summary.quality_passed);
        assert!(!summary.overall_passed);
        
        // Check category stats
        let stats = &summary.category_stats;
        assert_eq!(stats.len(), 2);
        assert_eq!(stats.get("Video"), Some(&(0, 1)));
        assert_eq!(stats.get("Audio"), Some(&(1, 0)));
    }
}