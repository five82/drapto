//! Audio encoding module for drapto
//!
//! This module provides functionality for encoding audio streams to Opus format
//! with appropriate bitrates based on channel count.

use std::path::{Path, PathBuf};
use std::process::Command;
use log::{info, warn, error, debug};
use thiserror::Error;

use crate::error::{DraptoError, Result};
use crate::media::info::MediaInfo;
use crate::validation::audio as audio_validation;
use crate::validation::report::ValidationReport;
use crate::util::command;

/// Audio encoding errors
#[derive(Error, Debug)]
pub enum AudioEncodingError {
    #[error("Failed to encode audio: {0}")]
    EncodingFailed(String),
    
    #[error("Failed to validate audio: {0}")]
    ValidationFailed(String),
    
    #[error("No audio streams found")]
    NoAudioStreams,
    
    #[error("Invalid audio track index: {0}")]
    InvalidTrackIndex(usize),
}

/// Audio track information used for encoding
#[derive(Debug, Clone)]
pub struct AudioTrackInfo {
    /// Original track index
    pub index: usize,
    
    /// Number of channels
    pub channels: u32,
    
    /// Duration in seconds
    pub duration: Option<f64>,
    
    /// Original codec
    pub codec: String,
    
    /// Channel layout
    pub layout: String,
    
    /// Target bitrate for encoding
    pub target_bitrate: String,
}

/// Audio encoder configuration
#[derive(Debug, Clone)]
pub struct AudioEncoderConfig {
    /// Global configuration reference
    pub global_config: crate::config::Config,
    
    /// Working directory for temporary files
    pub temp_dir: PathBuf,
}

impl Default for AudioEncoderConfig {
    fn default() -> Self {
        Self {
            global_config: crate::config::Config::default(),
            temp_dir: std::env::temp_dir(),
        }
    }
}

/// Audio encoder for libopus
pub struct OpusEncoder {
    config: AudioEncoderConfig,
}

impl OpusEncoder {
    /// Create a new OpusEncoder with default configuration
    pub fn new() -> Self {
        Self {
            config: AudioEncoderConfig {
                global_config: crate::config::Config::default(),
                temp_dir: std::env::temp_dir(),
            },
        }
    }
    
    /// Create a new OpusEncoder with custom configuration
    pub fn with_config(config: AudioEncoderConfig) -> Self {
        Self { config }
    }
    
    /// Build a command for encoding an audio track
    pub fn build_encode_command(
        &self,
        input_file: impl AsRef<Path>,
        output_file: impl AsRef<Path>,
        track_index: usize,
        bitrate: &str,
    ) -> Command {
        // Get media info to find the actual stream index
        let media_info = match MediaInfo::from_path(input_file.as_ref()) {
            Ok(info) => info,
            Err(_) => {
                // If we can't get media info, use the track_index directly
                // Get audio config from global config
                let audio_config = &self.config.global_config.audio;
                
                let mut cmd = Command::new("ffmpeg");
                cmd.args(["-hide_banner", "-loglevel", "warning"])
                   .arg("-i").arg(input_file.as_ref())
                   .arg("-map").arg(format!("0:a:{}", track_index))
                   .arg("-c:a").arg("libopus")
                   .arg("-af").arg("aformat=channel_layouts=7.1|5.1|stereo|mono")
                   .arg("-application").arg(&audio_config.application)
                   .arg("-vbr").arg(if audio_config.vbr { "on" } else { "off" })
                   .arg("-compression_level").arg(audio_config.compression_level.to_string())
                   .arg("-frame_duration").arg(audio_config.frame_duration.to_string())
                   .arg("-b:a").arg(bitrate)
                   .arg("-avoid_negative_ts").arg("make_zero")
                   .arg("-y").arg(output_file.as_ref());
                return cmd;
            }
        };
        
        let audio_streams = media_info.audio_streams();
        
        // If the track_index is out of bounds or we have no audio streams, use the simple mapping
        if track_index >= audio_streams.len() {
            // Get audio config from global config
            let audio_config = &self.config.global_config.audio;
            
            let mut cmd = Command::new("ffmpeg");
            cmd.args(["-hide_banner", "-loglevel", "warning"])
               .arg("-i").arg(input_file.as_ref())
               .arg("-map").arg(format!("0:a:{}", track_index))
               .arg("-c:a").arg("libopus")
               .arg("-af").arg("aformat=channel_layouts=7.1|5.1|stereo|mono")
               .arg("-application").arg(&audio_config.application)
               .arg("-vbr").arg(if audio_config.vbr { "on" } else { "off" })
               .arg("-compression_level").arg(audio_config.compression_level.to_string())
               .arg("-frame_duration").arg(audio_config.frame_duration.to_string())
               .arg("-b:a").arg(bitrate)
               .arg("-avoid_negative_ts").arg("make_zero")
               .arg("-y").arg(output_file.as_ref());
            return cmd;
        }
        
        // Get the actual stream index
        let actual_index = audio_streams[track_index].index;
        
        // Get audio config from global config
        let audio_config = &self.config.global_config.audio;
        
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "warning"])
           .arg("-i").arg(input_file.as_ref())
           .arg("-map").arg(format!("0:{}", actual_index))  // Use the actual stream index
           .arg("-c:a").arg("libopus")
           .arg("-af").arg("aformat=channel_layouts=7.1|5.1|stereo|mono")
           .arg("-application").arg(&audio_config.application)
           .arg("-vbr").arg(if audio_config.vbr { "on" } else { "off" })
           .arg("-compression_level").arg(audio_config.compression_level.to_string())
           .arg("-frame_duration").arg(audio_config.frame_duration.to_string())
           .arg("-b:a").arg(bitrate)
           .arg("-avoid_negative_ts").arg("make_zero")
           .arg("-y").arg(output_file.as_ref());
        
        cmd
    }
    
    /// Determine appropriate bitrate based on channel count
    pub fn determine_bitrate(&self, channels: u32) -> (String, String) {
        // Get the bitrate from config, or use default if not specified
        let config = &self.config;
        
        match channels {
            1 => {
                let bitrate = config.global_config
                    .audio.bitrates.mono
                    .unwrap_or(64);
                (format!("{}k", bitrate), "mono".to_string())
            },
            2 => {
                let bitrate = config.global_config
                    .audio.bitrates.stereo
                    .unwrap_or(128);
                (format!("{}k", bitrate), "stereo".to_string())
            },
            6 => {
                let bitrate = config.global_config
                    .audio.bitrates.surround_5_1
                    .unwrap_or(256);
                (format!("{}k", bitrate), "5.1".to_string())
            },
            8 => {
                let bitrate = config.global_config
                    .audio.bitrates.surround_7_1
                    .unwrap_or(384);
                (format!("{}k", bitrate), "7.1".to_string())
            },
            _ => {
                // Default to per-channel bitrate for other configurations
                let per_channel = config.global_config
                    .audio.bitrates.per_channel
                    .unwrap_or(48);
                (format!("{}k", channels * per_channel), "custom".to_string())
            }
        }
    }
    
    /// Get audio track information from the input file
    pub fn get_audio_track_info(
        &self,
        input_file: impl AsRef<Path>,
        track_index: usize
    ) -> Result<AudioTrackInfo> {
        let media_info = MediaInfo::from_path(input_file.as_ref())?;
        let audio_streams = media_info.audio_streams();
        
        if audio_streams.is_empty() {
            return Err(DraptoError::Encoding(
                AudioEncodingError::NoAudioStreams.to_string()
            ));
        }
        
        // Check if the track_index is within bounds of the audio streams
        if track_index >= audio_streams.len() {
            return Err(DraptoError::Encoding(
                AudioEncodingError::InvalidTrackIndex(track_index).to_string()
            ));
        }
        
        // Get stream by position in the audio streams array, not by its index property
        let stream = &audio_streams[track_index];
        
        let channels = stream.properties.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(2) as u32;
        
        let duration = stream.properties.get("duration")
            .and_then(|d| d.as_str())
            .and_then(|d| d.parse::<f64>().ok());
        
        let (bitrate, layout) = self.determine_bitrate(channels);
        
        Ok(AudioTrackInfo {
            index: track_index,
            channels,
            duration,
            codec: stream.codec_name.clone(),
            layout,
            target_bitrate: bitrate,
        })
    }
    
    /// Encode a single audio track
    pub fn encode_audio_track(
        &self,
        input_file: impl AsRef<Path>,
        track_index: usize
    ) -> Result<PathBuf> {
        // Log subsection header
        info!("");
        crate::logging::log_subsection(&format!("AUDIO TRACK {}", track_index));
        info!("");
        
        // Get track information for encoding
        let track_info = self.get_audio_track_info(&input_file, track_index)?;
        
        // Create output file path
        let output_file = self.config.temp_dir.join(format!("audio-{}.mkv", track_index));
        
        // Build encoding command
        let mut cmd = self.build_encode_command(
            &input_file,
            &output_file,
            track_index,
            &track_info.target_bitrate
        );
        
        // Get the command arguments for logging
        let args: Vec<String> = cmd.get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
        
        // Log audio encoding parameters section
        crate::logging::log_subsection("AUDIO ENCODING PARAMETERS");
        info!("");
        info!("Opus audio encoding parameters:");
        info!("  Codec: libopus");
        info!("  Track: {}", track_index);
        info!("  Channels: {}", track_info.channels);
        info!("  Layout: {}", track_info.layout);
        info!("  Bitrate: {}", track_info.target_bitrate);
        
        // Extract and log all actual parameters from the command
        for i in 0..args.len() {
            if args[i] == "-af" && i + 1 < args.len() {
                info!("  Audio Filter: {}", args[i+1]);
            }
            if args[i] == "-application" && i + 1 < args.len() {
                info!("  Application: {}", args[i+1]);
            }
            if args[i] == "-vbr" && i + 1 < args.len() {
                info!("  VBR: {}", args[i+1]);
            }
            if args[i] == "-compression_level" && i + 1 < args.len() {
                info!("  Compression Level: {}", args[i+1]);
            }
            if args[i] == "-frame_duration" && i + 1 < args.len() {
                info!("  Frame Duration: {}ms", args[i+1]);
            }
            if args[i] == "-avoid_negative_ts" && i + 1 < args.len() {
                info!("  Avoid Negative TS: {}", args[i+1]);
            }
        }
        
        // Create command string for debugging
        let cmd_str = format!("ffmpeg {}", args.join(" "));
        debug!("FFmpeg audio encoding command:");
        debug!("  {}", cmd_str.replace(" -", "\n  -"));
        
        // Progress callback for logging
        let progress_callback = if let Some(duration) = track_info.duration {
            let track_idx = track_index;
            Some(Box::new(move |progress: f32| {
                if progress > 0.0 && (progress * 100.0).round() % 10.0 == 0.0 {
                    info!(
                        "Audio track {} encoding: {:.0}% complete ({:.1}/{:.1}s)",
                        track_idx,
                        progress * 100.0,
                        progress * duration as f32,
                        duration
                    );
                }
            }) as command::ProgressCallback)
        } else {
            None
        };
        
        // Execute command with progress reporting
        command::run_command_with_progress(&mut cmd, progress_callback, None)
            .map_err(|e| DraptoError::Encoding(
                format!("Failed to encode audio track {}: {}", track_index, e)
            ))?;
        
        // Validate the encoded audio
        self.validate_encoded_audio(&output_file, track_index)?;
        
        Ok(output_file)
    }
    
    /// Encode all audio tracks from an input file
    pub fn encode_audio_tracks(
        &self,
        input_file: impl AsRef<Path>
    ) -> Result<Vec<PathBuf>> {
        // Log section header
        crate::logging::log_section("AUDIO ENCODING");
        
        // Validate input audio first
        self.validate_input_audio(&input_file)?;
        
        // Get media info for track detection
        let media_info = MediaInfo::from_path(&input_file)?;
        let audio_streams = media_info.audio_streams();
        
        if audio_streams.is_empty() {
            warn!("No audio tracks found");
            return Ok(Vec::new());
        }
        
        info!("Found {} audio streams in input", audio_streams.len());
        
        let mut encoded_tracks = Vec::new();
        // Iterate through audio streams by their position in the array (0, 1, 2, etc.)
        for (i, _stream) in audio_streams.iter().enumerate() {
            match self.encode_audio_track(&input_file, i) {
                Ok(output_track) => {
                    encoded_tracks.push(output_track);
                },
                Err(e) => {
                    error!("Failed to encode audio track {}: {}", i, e);
                    return Err(DraptoError::Encoding(
                        format!("Failed to encode audio track {}: {}", i, e)
                    ));
                }
            }
        }
        
        // Show audio encoding summary
        crate::logging::log_subsection("AUDIO ENCODING SUMMARY");
        if encoded_tracks.is_empty() {
            warn!("No audio tracks were encoded");
        } else {
            for (i, track) in encoded_tracks.iter().enumerate() {
                let size = std::fs::metadata(track)
                    .map(|m| m.len())
                    .unwrap_or(0);
                
                let size_mb = size as f64 / (1024.0 * 1024.0);
                info!("  Track {}: Opus audio, {:.2} MB", i, size_mb);
            }
            info!("");
            info!("Successfully encoded {} audio tracks to Opus format", encoded_tracks.len());
        }
        
        Ok(encoded_tracks)
    }
    
    /// Validate input audio before encoding
    fn validate_input_audio(&self, input_file: impl AsRef<Path>) -> Result<()> {
        let media_info = MediaInfo::from_path(input_file.as_ref())?;
        let audio_streams = media_info.audio_streams();
        
        if audio_streams.is_empty() {
            return Err(DraptoError::Validation(
                "Input file contains no audio streams".to_string()
            ));
        }
        
        info!("Found {} audio streams in input", audio_streams.len());
        
        for (i, stream) in audio_streams.iter().enumerate() {
            if stream.codec_name.is_empty() {
                return Err(DraptoError::Validation(
                    format!("Audio stream {} has invalid codec", i)
                ));
            }
        }
        
        Ok(())
    }
    
    /// Validate encoded audio after encoding
    fn validate_encoded_audio(
        &self,
        audio_file: impl AsRef<Path>,
        original_index: usize
    ) -> Result<()> {
        let path = audio_file.as_ref();
        
        // Basic file validation
        if !path.exists() {
            return Err(DraptoError::Validation(
                format!("Encoded audio track {} missing", original_index)
            ));
        }
        
        // Size check
        let metadata = std::fs::metadata(path).map_err(|e| {
            DraptoError::Validation(format!("Failed to get file metadata: {}", e))
        })?;
        
        if metadata.len() < 1024 {
            return Err(DraptoError::Validation(
                format!("Encoded audio track {} too small", original_index)
            ));
        }
        
        // Media info validation
        let media_info = MediaInfo::from_path(path)?;
        let audio_streams = media_info.audio_streams();
        
        if audio_streams.is_empty() {
            return Err(DraptoError::Validation(
                format!("Encoded audio track {} contains no audio streams", original_index)
            ));
        }
        
        // Codec validation
        let stream = &audio_streams[0];
        if stream.codec_name != "opus" {
            return Err(DraptoError::Validation(
                format!("Encoded track {} has wrong codec: {}", 
                    original_index, stream.codec_name)
            ));
        }
        
        // Channel validation
        let channels = stream.properties.get("channels")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;
            
        if channels < 1 {
            return Err(DraptoError::Validation(
                format!("Encoded track {} has invalid channel count", original_index)
            ));
        }
        
        info!("Successfully validated encoded audio track {}", original_index);
        
        Ok(())
    }
    
    /// Validate audio streams in final output
    pub fn validate_audio_streams(
        &self,
        input_file: impl AsRef<Path>,
        output_file: impl AsRef<Path>
    ) -> Result<ValidationReport> {
        let input_info = MediaInfo::from_path(input_file.as_ref())?;
        let output_info = MediaInfo::from_path(output_file.as_ref())?;
        
        let mut report = ValidationReport::new();
        
        let input_audio = input_info.audio_streams();
        let output_audio = output_info.audio_streams();
        
        // Track count validation
        if input_audio.len() != output_audio.len() {
            report.add_error(
                format!(
                    "Audio track count mismatch: input {} vs output {}",
                    input_audio.len(), output_audio.len()
                ),
                "Audio Tracks"
            );
        } else {
            report.add_info(
                format!("Found {} audio tracks", output_audio.len()),
                "Audio Tracks"
            );
        }
        
        // Detailed audio validation
        audio_validation::validate_audio(&output_info, &mut report, None);
        
        // Codec validation
        for (i, stream) in output_audio.iter().enumerate() {
            if stream.codec_name != "opus" {
                report.add_error(
                    format!("Track {} has wrong codec: {}", i, stream.codec_name),
                    "Audio Codec"
                );
            }
        }
        
        report.add_info(
            format!("{} validated Opus streams", output_audio.len()),
            "Audio"
        );
        
        Ok(report)
    }
}

/// Audio encoding options
#[derive(Debug, Clone)]
pub struct AudioEncodingOptions {
    /// Working directory for temporary files
    pub working_dir: PathBuf,
    
    /// Target quality (0-100%, codec-specific)
    pub quality: Option<f32>,
    
    /// Hardware acceleration options for FFmpeg (for decoding only)
    pub hw_accel_option: Option<String>,
}

/// Encode audio streams from a video file
///
/// # Arguments
///
/// * `input` - Path to input video file
/// * `options` - Audio encoding options
/// * `config` - Global configuration
///
/// # Returns
///
/// Paths to the encoded audio files
pub fn encode_audio(
    input: &Path, 
    options: &AudioEncodingOptions,
    config: &crate::config::Config
) -> Result<Vec<PathBuf>> {
    info!("Starting audio encoding: {}", input.display());
    debug!("Audio encoding options: {:?}", options);
    
    // Create encoder with proper configuration
    let encoder_config = AudioEncoderConfig {
        global_config: config.clone(),
        temp_dir: options.working_dir.clone(),
    };
    
    let encoder = OpusEncoder::with_config(encoder_config);
    
    // Encode all audio tracks
    match encoder.encode_audio_tracks(input) {
        Ok(tracks) => {
            info!("Audio encoding complete: {} tracks", tracks.len());
            Ok(tracks)
        },
        Err(e) => {
            error!("Audio encoding failed: {}", e);
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    
    #[test]
    fn test_determine_bitrate() {
        // Create a default config
        let mut config = crate::config::Config::default();
        
        // Set custom bitrates for testing
        config.audio.bitrates.mono = Some(64);
        config.audio.bitrates.stereo = Some(128);
        config.audio.bitrates.surround_5_1 = Some(256);
        config.audio.bitrates.surround_7_1 = Some(384);
        config.audio.bitrates.per_channel = Some(48);
        
        let encoder_config = AudioEncoderConfig {
            global_config: config,
            temp_dir: std::env::temp_dir(),
        };
        
        let encoder = OpusEncoder::with_config(encoder_config);
        
        let (bitrate1, layout1) = encoder.determine_bitrate(1);
        assert_eq!(bitrate1, "64k");
        assert_eq!(layout1, "mono");
        
        let (bitrate2, layout2) = encoder.determine_bitrate(2);
        assert_eq!(bitrate2, "128k");
        assert_eq!(layout2, "stereo");
        
        let (bitrate6, layout6) = encoder.determine_bitrate(6);
        assert_eq!(bitrate6, "256k");
        assert_eq!(layout6, "5.1");
        
        let (bitrate8, layout8) = encoder.determine_bitrate(8);
        assert_eq!(bitrate8, "384k");
        assert_eq!(layout8, "7.1");
        
        // Custom channel count
        let (bitrate4, layout4) = encoder.determine_bitrate(4);
        assert_eq!(bitrate4, "192k");
        assert_eq!(layout4, "custom");
    }
    
    #[test]
    fn test_build_encode_command() {
        let temp_dir = tempdir().unwrap();
        
        // Create a default config
        let mut config = crate::config::Config::default();
        
        // Set audio config for testing
        config.audio.compression_level = 10;
        config.audio.frame_duration = 20;
        config.audio.vbr = true;
        config.audio.application = "audio".to_string();
        
        let encoder_config = AudioEncoderConfig {
            global_config: config,
            temp_dir: temp_dir.path().to_path_buf(),
        };
        
        let encoder = OpusEncoder::with_config(encoder_config);
        
        let input_file = Path::new("/path/to/input.mkv");
        let output_file = Path::new("/path/to/output.mkv");
        
        let cmd = encoder.build_encode_command(input_file, output_file, 0, "128k");
        
        // Convert args to strings for easier assertion
        let args: Vec<String> = cmd.get_args()
            .map(|arg| arg.to_string_lossy().to_string())
            .collect();
            
        // Verify key command arguments
        assert!(args.contains(&"-c:a".to_string()));
        assert!(args.contains(&"libopus".to_string()));
        assert!(args.contains(&"-b:a".to_string()));
        assert!(args.contains(&"128k".to_string()));
        assert!(args.contains(&"-map".to_string()));
        assert!(args.contains(&"0:a:0".to_string()));
    }
}