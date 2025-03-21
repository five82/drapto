use std::path::Path;
use log::{info, error};

use crate::error::{DraptoError, Result};
use crate::media::MediaInfo;

pub mod video;
pub mod audio;
pub mod sync;
pub mod report;
pub mod subtitles;
pub mod quality;

// Re-export from report module
pub use report::{ValidationMessage, ValidationLevel, ValidationReport};

/// Validate a media file
pub fn validate_media<P: AsRef<Path>>(path: P) -> Result<ValidationReport> {
    let mut report = ValidationReport::new();
    let media_info = MediaInfo::from_path(path)?;
    
    // Run various validations
    audio::validate_audio(&media_info, &mut report);
    video::validate_video(&media_info, &mut report);
    subtitles::validate_subtitles(&media_info, &mut report);
    sync::validate_sync(&media_info, &mut report);
    
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
    
    // Use the sync module to check AV synchronization
    sync::validate_sync(&media_info, &mut report);
    
    if !report.passed {
        error!("A/V sync validation failed: {}", report);
        return Err(DraptoError::Validation(
            format!("A/V sync validation failed: {} error(s)", report.errors().len())
        ));
    }
    
    info!("A/V sync validation passed");
    Ok(report)
}