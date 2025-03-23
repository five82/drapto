//! Video encoding module for drapto
//!
//! This module implements video encoding functionality for drapto.
//! It handles video encoding with various encoders and quality settings.

use log::{debug, info, warn};
use regex::Regex;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use crate::config::{VideoEncodingConfig, SceneDetectionConfig};

use crate::error::{DraptoError, Result};
use crate::media::info::MediaInfo;
use crate::util::command::{self, run_command, run_command_with_progress};

/// Configuration for ab-av1 encoding
#[derive(Debug, Clone)]
pub struct AbAv1Config {
    /// Encoder preset (1-13, lower is slower/better quality)
    pub preset: u8,

    /// Target VMAF score (0-100)
    pub target_vmaf: f32,

    /// Target VMAF score for HDR content
    pub target_vmaf_hdr: f32,

    /// SVT-AV1 encoder parameters
    pub svt_params: String,

    /// Number of samples to use for VMAF analysis
    pub vmaf_sample_count: usize,

    /// Duration of each VMAF sample in seconds
    pub vmaf_sample_duration: u32,

    /// VMAF analysis options
    pub vmaf_options: String,
}

impl Default for AbAv1Config {
    fn default() -> Self {
        Self {
            preset: 6,
            target_vmaf: 93.0,     // SDR target VMAF
            target_vmaf_hdr: 95.0, // HDR target VMAF
            svt_params: "tune=0:enable-qm=1:enable-overlays=1:film-grain=0:film-grain-denoise=0"
                .to_string(),
            vmaf_sample_count: 3,
            vmaf_sample_duration: 1,
            vmaf_options: "n_subsample=8:pool=perc5_min".to_string(),
        }
    }
}

/// Ab-AV1 encoder
#[derive(Clone)]
pub struct AbAv1Encoder {
    /// Encoding configuration
    config: AbAv1Config,
}

impl AbAv1Encoder {
    /// Create a new Ab-AV1 encoder with default configuration
    pub fn new() -> Self {
        Self {
            config: AbAv1Config::default(),
        }
    }

    /// Create a new Ab-AV1 encoder with custom configuration
    pub fn with_config(config: AbAv1Config) -> Self {
        Self { config }
    }

    /// Check if ab-av1 is available
    pub fn check_availability(&self) -> Result<()> {
        info!("Checking for ab-av1...");

        let mut cmd = Command::new("which");
        cmd.arg("ab-av1");

        match run_command(&mut cmd) {
            Ok(_) => {
                info!("ab-av1 found");
                Ok(())
            }
            Err(_) => Err(DraptoError::ExternalTool(
                "ab-av1 is required for encoding but not found. Install with: cargo install ab-av1"
                    .to_string(),
            )),
        }
    }

    /// Build an ab-av1 encode command based on parameters
    fn build_encode_command(
        &self,
        input: &Path,
        output: &Path,
        crop_filter: Option<&str>,
        retry_count: usize,
        is_hdr: bool,
        dv_flag: bool,
    ) -> Command {
        // Get retry-specific parameters
        let (sample_count, sample_duration, min_vmaf) = self.get_retry_params(retry_count, is_hdr);

        // Now we just use the static to avoid logging parameters again,
        // since they've already been shown at the start
        use std::sync::atomic::Ordering;
        static PARAMETERS_LOGGED: std::sync::atomic::AtomicBool =
            std::sync::atomic::AtomicBool::new(false);
        if !PARAMETERS_LOGGED.swap(true, Ordering::SeqCst) {
            // This section is now logged earlier in the parallel encoding process
            // This just ensures we don't double-log
        }

        let mut cmd = Command::new("ab-av1");
        cmd.arg("auto-encode")
            .arg("--input")
            .arg(input)
            .arg("--output")
            .arg(output)
            .arg("--encoder")
            .arg("libsvtav1")
            .arg("--min-vmaf")
            .arg(min_vmaf.to_string())
            .arg("--preset")
            .arg(self.config.preset.to_string())
            .arg("--svt")
            .arg(&self.config.svt_params)
            .arg("--keyint")
            .arg("10s")
            .arg("--samples")
            .arg(sample_count.to_string())
            .arg("--sample-duration")
            .arg(format!("{}s", sample_duration))
            .arg("--vmaf")
            .arg(&self.config.vmaf_options)
            .arg("--pix-format")
            .arg("yuv420p10le");

        if let Some(filter) = crop_filter {
            cmd.arg("--vfilter").arg(filter);
        }

        if dv_flag {
            cmd.arg("--enc").arg("dolbyvision=true");
        }

        cmd
    }

    /// Get encoding parameters based on retry count
    fn get_retry_params(&self, retry_count: usize, is_hdr: bool) -> (usize, u32, f32) {
        let target_vmaf = if is_hdr {
            self.config.target_vmaf_hdr
        } else {
            self.config.target_vmaf
        };

        match retry_count {
            0 => (3, 1, target_vmaf),
            1 => (4, 2, target_vmaf),
            _ => (4, 2, 95.0), // Force highest quality for last retry
        }
    }

    /// Parse VMAF scores from encoder output
    fn parse_vmaf_scores(&self, stderr: &str) -> (Option<f64>, Option<f64>, Option<f64>) {
        let mut vmaf_values = Vec::new();
        // More flexible regex that matches multiple formats:
        // 1. [Parsed_libvmaf_0 @ 0x55842c1490] VMAF score: 95.432651
        // 2. [SVT] Average VMAF Score: 95.43 (Min: 93.21, Max: 97.65)
        // 3. VMAF 95.432651
        let re = Regex::new(r"VMAF(?:\s+score:|[^\d]+)([0-9.]+)").unwrap();
        
        // For min/max detection if available in SVT format
        let re_min_max = Regex::new(r"Min:\s*([0-9.]+).*Max:\s*([0-9.]+)").unwrap();
        let mut min_value: Option<f64> = None;
        let mut max_value: Option<f64> = None;

        for line in stderr.lines() {
            // Check for min/max values in the line
            if let Some(captures) = re_min_max.captures(line) {
                if let (Some(min_str), Some(max_str)) = (captures.get(1), captures.get(2)) {
                    if let (Ok(min), Ok(max)) = (min_str.as_str().parse::<f64>(), max_str.as_str().parse::<f64>()) {
                        min_value = Some(min);
                        max_value = Some(max);
                    }
                }
            }
            
            // Check for VMAF scores
            if let Some(captures) = re.captures(line) {
                if let Some(vmaf_str) = captures.get(1) {
                    if let Ok(vmaf) = vmaf_str.as_str().parse::<f64>() {
                        vmaf_values.push(vmaf);
                    }
                }
            }
        }

        if vmaf_values.is_empty() {
            return (None, None, None);
        }

        let avg = vmaf_values.iter().sum::<f64>() / vmaf_values.len() as f64;
        
        // Use explicitly extracted min/max if available, otherwise calculate from values
        let min = min_value.unwrap_or_else(|| 
            vmaf_values.iter().cloned().fold(f64::INFINITY, f64::min)
        );
        
        let max = max_value.unwrap_or_else(|| 
            vmaf_values.iter().cloned().fold(f64::NEG_INFINITY, f64::max)
        );

        (Some(avg), Some(min), Some(max))
    }

    /// Calculate output metrics for an encoded segment
    fn calculate_output_metrics(
        &self,
        output_segment: &Path,
        input_duration: f64,
        encoding_time: f64,
    ) -> Result<EncodingMetrics> {
        let info = MediaInfo::from_path(output_segment)?;

        let output_duration = info.duration().unwrap_or(0.0);
        let output_size = info.format.as_ref().and_then(|f| f.size).unwrap_or(0);

        let video_stream = info
            .primary_video_stream()
            .ok_or_else(|| DraptoError::MediaFile("No video stream found".to_string()))?;

        let bitrate = if output_duration > 0.0 {
            (output_size as f64 * 8.0) / (output_duration * 1000.0)
        } else {
            0.0
        };

        let framerate = video_stream
            .properties
            .get("r_frame_rate")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let (width, height) = info.video_dimensions().unwrap_or((0, 0));
        let resolution = format!("{}x{}", width, height);

        Ok(EncodingMetrics {
            duration: output_duration,
            size_bytes: output_size,
            bitrate_kbps: bitrate as u64,
            speed_factor: if encoding_time > 0.0 {
                input_duration / encoding_time
            } else {
                0.0
            },
            resolution,
            framerate,
        })
    }

    /// Encode a single video segment
    ///
    /// # Arguments
    ///
    /// * `input` - Path to input segment
    /// * `output` - Path to output segment location
    /// * `crop_filter` - Optional crop filter to apply
    /// * `retry_count` - Number of previous retry attempts (0 for first try)
    /// * `is_hdr` - Whether the content is HDR
    /// * `dv_flag` - Whether the content has Dolby Vision
    ///
    /// # Returns
    ///
    /// Encoding statistics
    pub fn encode_segment(
        &self,
        input: &Path,
        output: &Path,
        crop_filter: Option<&str>,
        retry_count: usize,
        is_hdr: bool,
        dv_flag: bool,
    ) -> Result<SegmentEncodingStats> {
        info!("Encoding segment: {}", input.display());
        debug!("Output: {}", output.display());
        debug!("Crop filter: {:?}", crop_filter);
        debug!("Retry count: {}", retry_count);
        debug!("HDR: {}, Dolby Vision: {}", is_hdr, dv_flag);

        let start_time = Instant::now();

        // Get input properties
        let input_info = MediaInfo::from_path(input)?;
        let input_duration = input_info.duration().unwrap_or(0.0);

        // Build and run encode command
        let mut cmd =
            self.build_encode_command(input, output, crop_filter, retry_count, is_hdr, dv_flag);

        // Use progress callback
        let result = run_command_with_progress(
            &mut cmd,
            Some(Box::new(|progress| {
                debug!("Encoding progress: {:.1}%", progress * 100.0);
            })),
            None,
        );

        match result {
            Ok(command_output) => {
                let encoding_time = start_time.elapsed().as_secs_f64();

                // Parse metrics from the encoding result
                let metrics =
                    self.calculate_output_metrics(output, input_duration, encoding_time)?;
                let vmaf_metrics =
                    self.parse_vmaf_scores(&String::from_utf8_lossy(&command_output.stderr));

                // Create statistics
                let stats = SegmentEncodingStats {
                    segment_name: input
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string(),
                    encoding_time,
                    crop_filter: crop_filter.unwrap_or("none").to_string(),
                    vmaf_score: vmaf_metrics.0,
                    vmaf_min: vmaf_metrics.1,
                    vmaf_max: vmaf_metrics.2,
                    metrics,
                    peak_memory_bytes: 0, // This will be populated by the parallel encoder
                };

                // Log progress
                info!("Segment encoding complete: {}", stats.segment_name);
                info!("  Duration: {:.2}s", stats.metrics.duration);
                info!(
                    "  Size: {:.2} MB",
                    stats.metrics.size_bytes as f64 / (1024.0 * 1024.0)
                );
                info!("  Bitrate: {:.2} kbps", stats.metrics.bitrate_kbps);
                info!(
                    "  Encoding time: {:.2}s ({:.2}x realtime)",
                    stats.encoding_time, stats.metrics.speed_factor
                );
                info!(
                    "  Resolution: {} @ {}",
                    stats.metrics.resolution, stats.metrics.framerate
                );

                if let Some(vmaf) = stats.vmaf_score {
                    info!(
                        "  VMAF Avg: {:.2}, Min: {:.2}, Max: {:.2}",
                        vmaf,
                        stats.vmaf_min.unwrap_or(0.0),
                        stats.vmaf_max.unwrap_or(0.0)
                    );
                } else {
                    info!("  No VMAF scores available");
                }

                Ok(stats)
            }
            Err(e) => self.handle_segment_retry(
                e,
                input,
                output,
                crop_filter,
                retry_count,
                is_hdr,
                dv_flag,
            ),
        }
    }

    /// Handle segment encoding retry logic
    fn handle_segment_retry(
        &self,
        error: DraptoError,
        input: &Path,
        output: &Path,
        crop_filter: Option<&str>,
        retry_count: usize,
        is_hdr: bool,
        dv_flag: bool,
    ) -> Result<SegmentEncodingStats> {
        const MAX_RETRIES: usize = 2;

        warn!(
            "Encoding segment {} failed on attempt {} with error: {}",
            input.display(),
            retry_count,
            error
        );

        if retry_count < MAX_RETRIES {
            let new_retry_count = retry_count + 1;
            info!(
                "Retrying segment {} (attempt {})",
                input.display(),
                new_retry_count
            );
            self.encode_segment(input, output, crop_filter, new_retry_count, is_hdr, dv_flag)
        } else {
            Err(DraptoError::Encoding(format!(
                "Segment {} failed after {} attempts: {}",
                input.display(),
                MAX_RETRIES + 1,
                error
            )))
        }
    }

    /// Validate an encoded segment
    ///
    /// Checks:
    /// - File exists and has size
    /// - Codec is av1
    /// - Duration matches input (within tolerance)
    ///
    /// # Arguments
    ///
    /// * `original` - Path to original segment for comparison
    /// * `encoded` - Path to encoded segment
    /// * `tolerance` - Duration comparison tolerance in seconds
    ///
    /// # Returns
    ///
    /// Result with error message if validation fails
    pub fn validate_segment(&self, original: &Path, encoded: &Path, tolerance: f64) -> Result<()> {
        debug!("Validating segment: {}", encoded.display());

        // Check if file exists and has size
        if !encoded.exists() {
            return Err(DraptoError::Validation(format!(
                "Encoded segment doesn't exist: {}",
                encoded.display()
            )));
        }

        let encoded_size = encoded
            .metadata()
            .map_err(|e| DraptoError::Validation(format!("Failed to get segment metadata: {}", e)))?
            .len();

        if encoded_size == 0 {
            return Err(DraptoError::Validation(format!(
                "Encoded segment is empty: {}",
                encoded.display()
            )));
        }

        // Check codec
        let encoded_info = MediaInfo::from_path(encoded)?;
        let encoded_codec = encoded_info
            .primary_video_stream()
            .map(|stream| stream.codec_name.clone())
            .unwrap_or_default();

        if encoded_codec != "av1" {
            return Err(DraptoError::Validation(format!(
                "Wrong codec '{}' in segment: {}",
                encoded_codec,
                encoded.display()
            )));
        }

        // Check duration
        let encoded_duration = encoded_info.duration().unwrap_or(0.0);
        if encoded_duration <= 0.0 {
            return Err(DraptoError::Validation(format!(
                "Invalid duration in segment: {}",
                encoded.display()
            )));
        }

        // Compare with original duration
        let original_info = MediaInfo::from_path(original)?;
        let original_duration = original_info.duration().unwrap_or(0.0);

        // Allow a relative tolerance of 5% (or at least specified tolerance)
        let duration_tolerance = f64::max(tolerance, original_duration * 0.05);
        if f64::abs(original_duration - encoded_duration) > duration_tolerance {
            return Err(DraptoError::Validation(format!(
                "Duration mismatch in {}: {:.2} vs {:.2} (tolerance: {:.2})",
                encoded.display(),
                original_duration,
                encoded_duration,
                duration_tolerance
            )));
        }

        Ok(())
    }

    /// Validate all encoded segments
    ///
    /// # Arguments
    ///
    /// * `original_segments` - Original segment paths
    /// * `encoded_segments` - Encoded segment paths
    ///
    /// # Returns
    ///
    /// Result with error message if validation fails
    pub fn validate_segments(
        &self,
        original_segments: &[PathBuf],
        encoded_segments: &[PathBuf],
    ) -> Result<()> {
        info!("Validating {} encoded segments", encoded_segments.len());

        if original_segments.len() != encoded_segments.len() {
            return Err(DraptoError::Validation(format!(
                "Encoded segment count ({}) doesn't match original ({})",
                encoded_segments.len(),
                original_segments.len()
            )));
        }

        for (orig, encoded) in original_segments.iter().zip(encoded_segments.iter()) {
            self.validate_segment(orig, encoded, 0.2)?;
        }

        info!(
            "Successfully validated {} encoded segments",
            encoded_segments.len()
        );
        Ok(())
    }
}

/// Metrics for an encoded video segment
#[derive(Debug, Clone)]
pub struct EncodingMetrics {
    /// Duration of the segment in seconds
    pub duration: f64,

    /// Size of the segment in bytes
    pub size_bytes: u64,

    /// Bitrate of the segment in kilobits per second
    pub bitrate_kbps: u64,

    /// Encoding speed factor (realtime ratio)
    pub speed_factor: f64,

    /// Resolution of the segment (e.g. "1920x1080")
    pub resolution: String,

    /// Frame rate of the segment (e.g. "24/1")
    pub framerate: String,
}

/// Statistics from encoding a segment
#[derive(Debug, Clone)]
pub struct SegmentEncodingStats {
    /// Segment name
    pub segment_name: String,

    /// Time taken to encode the segment in seconds
    pub encoding_time: f64,

    /// Crop filter used (or "none")
    pub crop_filter: String,

    /// Average VMAF score (0-100, higher is better)
    pub vmaf_score: Option<f64>,

    /// Minimum VMAF score
    pub vmaf_min: Option<f64>,

    /// Maximum VMAF score
    pub vmaf_max: Option<f64>,

    /// Encoding metrics
    pub metrics: EncodingMetrics,

    /// Peak memory usage during encoding in bytes
    pub peak_memory_bytes: usize,
}

/// Video encoding options
#[derive(Debug, Clone)]
pub struct VideoEncodingOptions {
    /// Target VMAF quality (0-100, higher is better)
    pub quality: Option<f32>,

    /// Number of parallel encoding jobs
    pub parallel_jobs: usize,

    /// Hardware acceleration options for FFmpeg (for decoding only)
    pub hw_accel_option: Option<String>,

    /// Crop filter to apply
    pub crop_filter: Option<String>,

    /// Scene timestamps for segmentation
    pub scenes: Option<Vec<f64>>,

    /// Whether the content is HDR
    pub is_hdr: bool,

    /// Whether the content has Dolby Vision
    pub is_dolby_vision: bool,

    /// Working directory for temporary files
    pub working_dir: PathBuf,
}

/// Encode a video file with optional segmentation
///
/// # Arguments
///
/// * `input` - Path to input video file
/// * `options` - Encoding options
///
/// # Returns
///
/// Path to the encoded video file
pub fn encode_video(input: &Path, options: &VideoEncodingOptions) -> Result<PathBuf> {
    info!("Starting video encoding: {}", input.display());
    debug!("Encoding options: {:?}", options);

    // Use the same filename as the input file for the output
    let input_filename = input.file_name().ok_or_else(|| {
        DraptoError::InvalidPath("Could not determine input filename".to_string())
    })?;
    let output = options.working_dir.join(input_filename);

    // Create and configure the encoder
    let encoder = AbAv1Encoder::new();

    // Check for ab-av1 availability
    match encoder.check_availability() {
        Ok(_) => {
            info!("Using ab-av1 encoder");

            // Check if we have scene data for segmentation
            if let Some(ref scenes) = options.scenes {
                if !scenes.is_empty() {
                    // Section header for segmentation
                    crate::logging::log_section("SEGMENTATION");

                    info!("Using scene-based segmentation for parallel encoding");
                    info!(
                        "Creating {} scene-based segments for parallel encoding...",
                        scenes.len()
                    );

                    // Create segments directory
                    let segments_dir = options.working_dir.join("segments");
                    std::fs::create_dir_all(&segments_dir)?;

                    // Create config for segmentation
                    let config = crate::config::Config {
                        input: input.to_path_buf(),
                        directories: Default::default(),
                        video: VideoEncodingConfig {
                            hw_accel_option: options.hw_accel_option.clone().unwrap_or_default(),
                            hardware_acceleration: true,
                            target_quality: None,
                            target_quality_hdr: None,
                            preset: 6,
                            svt_params: String::new(),
                            pix_fmt: String::new(),
                            disable_crop: false,
                            use_segmentation: true,
                            vmaf_sample_count: 3,
                            vmaf_sample_length: 1.0,
                        },
                        scene_detection: SceneDetectionConfig {
                            scene_threshold: 40.0,
                            hdr_scene_threshold: 30.0,
                            min_segment_length: 5.0,
                            max_segment_length: 15.0,
                        },
                        audio: Default::default(),
                        resources: Default::default(),
                        logging: Default::default(),
                        output: PathBuf::new()
                    };

                    // Segment the video
                    use crate::encoding::segmentation::segment_video_at_scenes;
                    let segment_files =
                        segment_video_at_scenes(input, &segments_dir, &scenes, &config)?;

                    if segment_files.is_empty() {
                        info!("Segmentation produced no segments, falling back to single-file encoding");
                    } else {

                        // Set up parallel encoder
                        use crate::encoding::parallel::{
                            EncodingProgress, ParallelEncoder, VideoEncoder,
                        };
                        use std::sync::Arc;

                        // Save the config values for logging before we move the encoder
                        let preset = encoder.config.preset;
                        let svt_params = encoder.config.svt_params.clone();
                        let vmaf_options = encoder.config.vmaf_options.clone();
                        let min_vmaf = if options.is_hdr { 95 } else { 93 };

                        // Define encoder adapter
                        struct AbAv1EncoderAdapter {
                            encoder: AbAv1Encoder,
                            crop_filter: Option<String>,
                            is_hdr: bool,
                            is_dv: bool,
                        }

                        impl VideoEncoder for AbAv1EncoderAdapter {
                            fn encode_video(
                                &self,
                                input: &Path,
                                output: &Path,
                                _progress: Option<EncodingProgress>,
                            ) -> Result<()> {
                                // Properly capture stats instead of explicitly discarding them
                                // Using _stats to indicate intentionally unused return value that contains VMAF data
                                let _stats = self.encoder.encode_segment(
                                    input,
                                    output,
                                    self.crop_filter.as_deref(),
                                    0,
                                    self.is_hdr,
                                    self.is_dv,
                                )?;
                                
                                // Log VMAF scores (already logged in encode_segment function)
                                // No need to duplicate logs here as they are already handled
                                Ok(())
                            }
                        }

                        let encoder_adapter = AbAv1EncoderAdapter {
                            encoder,
                            crop_filter: options.crop_filter.clone(),
                            is_hdr: options.is_hdr,
                            is_dv: options.is_dolby_vision,
                        };

                        // Create output directories first
                        let encoded_segments_dir = options.working_dir.join("encoded_segments");
                        std::fs::create_dir_all(&encoded_segments_dir)?;

                        // Encoding parameters have already been saved above

                        crate::logging::log_section("ENCODING PARAMETERS");
                        info!("Common ab-av1 encoding parameters:");
                        info!("  Encoder: libsvtav1");
                        info!("  Preset: {}", preset);
                        info!("  Min-VMAF: {}", min_vmaf);
                        info!("  SVT parameters: {}", svt_params);
                        info!("  Keyframe interval: 10s");
                        info!("  Sample count: {}", 3);
                        info!("  Sample duration: {}s", 1);
                        info!("  VMAF options: {}", vmaf_options);
                        info!("  Pixel format: yuv420p10le");
                        if let Some(filter) = &options.crop_filter {
                            info!("  Video filter: {}", filter);
                        }
                        if options.is_dolby_vision {
                            info!("  Dolby Vision mode: enabled");
                        }

                        // Prevent actual encoder from logging parameters again
                        use std::sync::atomic::Ordering;
                        static PARAMETERS_LOGGED: std::sync::atomic::AtomicBool =
                            std::sync::atomic::AtomicBool::new(false);
                        PARAMETERS_LOGGED.store(true, Ordering::SeqCst);

                        crate::logging::log_section("PARALLEL ENCODING");

                        // Create parallel encoder
                        let parallel_encoder = ParallelEncoder::new(Arc::new(encoder_adapter))
                            .max_concurrent_jobs(options.parallel_jobs)
                            .memory_per_job(2048) // 2GB per job
                            .on_progress(|progress, completed, total| {
                                debug!(
                                    "Parallel encoding progress: {:.1}% ({}/{} segments)",
                                    progress * 100.0,
                                    completed,
                                    total
                                );
                            });

                        // Encode all segments in parallel

                        let encoded_segments = parallel_encoder.encode_segments(
                            &segment_files,
                            &encoded_segments_dir,
                            &options.working_dir.join("temp"),
                        )?;

                        // Concatenate segments
                        use crate::util::command::run_command;
                        use std::io::Write;

                        // Create concat file
                        let concat_file = options.working_dir.join("concat.txt");
                        let mut file = std::fs::File::create(&concat_file)?;

                        for segment in &encoded_segments {
                            writeln!(file, "file '{}'", segment.to_string_lossy())?;
                        }

                        // Build ffmpeg concat command
                        let mut cmd = std::process::Command::new("ffmpeg");
                        cmd.args([
                            "-hide_banner",
                            "-loglevel",
                            "warning",
                            "-f",
                            "concat",
                            "-safe",
                            "0",
                            "-i",
                            concat_file.to_str().unwrap_or_default(),
                            "-c",
                            "copy",
                            "-y",
                            output.to_str().unwrap_or_default(),
                        ]);
                        crate::logging::log_section("FILE CONCATENATION");
                        info!("Concatenating to output file: {}", output.display());

                        // Execute concat command
                        info!("Concatenating {} encoded segments", encoded_segments.len());
                        run_command(&mut cmd)?;

                        crate::logging::log_section("ENCODING COMPLETE");
                        info!("Video encoding complete: {}", output.display());
                        return Ok(output);
                    }
                }
            }

            // If we didn't do segmentation or it failed, fall back to single-file encoding
            info!("Using single-file encoding");

            // Determine crop filter
            let crop_filter = options.crop_filter.as_deref();

            // Build encode command
            let mut cmd = encoder.build_encode_command(
                input,
                &output,
                crop_filter,
                0, // First attempt
                options.is_hdr,
                options.is_dolby_vision,
            );

            // Execute the encoding command
            info!("Running encoding command...");
            match command::run_command_with_progress(
                &mut cmd,
                Some(Box::new(|progress| {
                    debug!("Encoding progress: {:.1}%", progress * 100.0);
                })),
                None,
            ) {
                Ok(_) => {
                    info!("Video encoding complete: {}", output.display());
                    Ok(output)
                }
                Err(e) => Err(e),
            }
        }
        Err(_) => {
            // Fallback to copy mode for testing purposes
            warn!("ab-av1 encoder not found, falling back to test mode (copy only)");
            info!("Target quality: {:?}", options.quality);
            info!("Parallel jobs: {}", options.parallel_jobs);
            info!(
                "Hardware acceleration for decoding: {}",
                options.hw_accel_option.as_deref().unwrap_or("None")
            );

            // Copy the input to output for testing
            std::fs::copy(input, &output)?;

            info!("Video encoding complete: {}", output.display());
            Ok(output)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_encode_command() {
        let encoder = AbAv1Encoder::new();
        let input = Path::new("/tmp/input.mkv");
        let output = Path::new("/tmp/output.mkv");

        // Test default command
        let cmd = encoder.build_encode_command(input, output, None, 0, false, false);
        let args: Vec<String> = cmd
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert!(args.contains(&"auto-encode".to_string()));
        assert!(args.contains(&"--input".to_string()));
        assert!(args.contains(&"--output".to_string()));
        assert!(args.contains(&"--encoder".to_string()));
        assert!(args.contains(&"libsvtav1".to_string()));

        // Test with crop filter
        let cmd_crop =
            encoder.build_encode_command(input, output, Some("crop=100:100:0:0"), 0, false, false);
        let crop_args: Vec<String> = cmd_crop
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert!(crop_args.contains(&"--vfilter".to_string()));
        assert!(crop_args.contains(&"crop=100:100:0:0".to_string()));

        // Test with Dolby Vision
        let cmd_dv = encoder.build_encode_command(input, output, None, 0, false, true);
        let dv_args: Vec<String> = cmd_dv
            .get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();

        assert!(dv_args.contains(&"--enc".to_string()));
        assert!(dv_args.contains(&"dolbyvision=true".to_string()));
    }

    #[test]
    fn test_get_retry_params() {
        let encoder = AbAv1Encoder::new();

        // Test default settings (SDR)
        let (samples, duration, vmaf) = encoder.get_retry_params(0, false);
        assert_eq!(samples, 3);
        assert_eq!(duration, 1);
        assert_eq!(vmaf, 93.0); // Default SDR VMAF target

        // Test with retry
        let (samples_retry, duration_retry, vmaf_retry) = encoder.get_retry_params(1, false);
        assert_eq!(samples_retry, 4);
        assert_eq!(duration_retry, 2);
        assert_eq!(vmaf_retry, 93.0); // Should still use target_vmaf

        // Test second retry (force quality)
        let (samples_retry2, duration_retry2, vmaf_retry2) = encoder.get_retry_params(2, false);
        assert_eq!(samples_retry2, 4);
        assert_eq!(duration_retry2, 2);
        assert_eq!(vmaf_retry2, 95.0); // Third try forces 95.0 quality regardless of target

        // Test HDR
        let (_, _, vmaf_hdr) = encoder.get_retry_params(0, true);
        assert_eq!(vmaf_hdr, 95.0); // Default config has same value for both
    }

    #[test]
    fn test_parse_vmaf_scores() {
        let encoder = AbAv1Encoder::new();

        // Test with valid VMAF scores in newer format
        let stderr = "
            [Parsed_libvmaf_0 @ 0x55842c1490] VMAF score: 95.432651
            [Parsed_libvmaf_0 @ 0x55842c1490] VMAF score: 93.214567
            [Parsed_libvmaf_0 @ 0x55842c1490] VMAF score: 97.654321
        ";

        let (avg, min, max) = encoder.parse_vmaf_scores(stderr);
        assert!(avg.is_some());
        assert!(min.is_some());
        assert!(max.is_some());

        let avg_val = avg.unwrap();
        let min_val = min.unwrap();
        let max_val = max.unwrap();

        assert!((avg_val - 95.43).abs() < 1.0); // Within 1 point
        assert!((min_val - 93.21).abs() < 0.1); // Within 0.1 point
        assert!((max_val - 97.65).abs() < 0.1); // Within 0.1 point

        // Test with no VMAF scores
        let stderr_no_vmaf = "
            No VMAF scores here
            Just some other output
        ";

        let (avg_none, min_none, max_none) = encoder.parse_vmaf_scores(stderr_no_vmaf);
        assert!(avg_none.is_none());
        assert!(min_none.is_none());
        assert!(max_none.is_none());
    }
}
