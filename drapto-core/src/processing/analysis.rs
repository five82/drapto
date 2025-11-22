//! Helper wrappers for crop detection orchestration.
//!
//! These functions keep reporting logic close to crop detection so that the main
//! workflow can stay focused on encoding decisions.

use crate::config::CoreConfig;
use crate::processing::crop_detection;
use crate::processing::video_properties::VideoProperties;
use crate::reporting::{CropSummary, Reporter, StageProgress};
use std::path::Path;

/// Run crop detection with reporting and graceful error handling.
/// Returns `(crop_filter, is_hdr_flag)`.
pub fn run_crop_detection(
    input_path: &Path,
    video_props: &VideoProperties,
    config: &CoreConfig,
    reporter: Option<&dyn Reporter>,
    input_filename: &str,
) -> (Option<String>, bool) {
    let disable_crop = config.crop_mode == "none";

    if let Some(rep) = reporter {
        rep.stage_progress(&StageProgress {
            stage: "analysis".to_string(),
            percent: 5.0,
            message: "Detecting black bars".to_string(),
            eta: None,
        });
    }

    if disable_crop {
        if let Some(rep) = reporter {
            rep.crop_result(&CropSummary {
                message: "Crop detection skipped".to_string(),
                crop: Some("disabled".to_string()),
                required: false,
                disabled: true,
            });
        }
        return (None, false);
    }

    match crop_detection::detect_crop(input_path, video_props, disable_crop) {
        Ok(result) => {
            if let Some(rep) = reporter {
                rep.crop_result(&CropSummary {
                    message: "Crop detection complete".to_string(),
                    crop: result.0.clone(),
                    required: result.0.is_some(),
                    disabled: false,
                });
            }
            result
        }
        Err(e) => {
            if let Some(rep) = reporter {
                rep.warning(&format!(
                    "Crop detection failed for {input_filename}: {e}. Proceeding without cropping."
                ));
                rep.crop_result(&CropSummary {
                    message: "Crop detection failed".to_string(),
                    crop: None,
                    required: false,
                    disabled: false,
                });
            }
            (None, false)
        }
    }
}
