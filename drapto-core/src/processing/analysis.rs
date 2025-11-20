//! Helper wrappers for crop detection and noise analysis orchestration.
//!
//! These functions keep event/notification wiring close to the operations
//! while letting the main workflow stay slimmer.

use crate::config::CoreConfig;
use crate::error::CoreResult;
use crate::events::{Event, EventDispatcher};
use crate::notifications::NotificationSender;
use crate::processing::crop_detection;
use crate::processing::noise_analysis;
use crate::processing::reporting::{emit_event, send_notification_safe};
use crate::processing::video_properties::VideoProperties;
use std::path::Path;

/// Run crop detection with event reporting and graceful error handling.
/// Returns (crop_filter, is_hdr_flag)
pub fn run_crop_detection(
    input_path: &Path,
    video_props: &VideoProperties,
    config: &CoreConfig,
    event_dispatcher: Option<&EventDispatcher>,
    input_filename: &str,
) -> (Option<String>, bool) {
    emit_event(event_dispatcher, Event::VideoAnalysisStarted);

    let disable_crop = config.crop_mode == "none";

    if disable_crop {
        emit_event(
            event_dispatcher,
            Event::BlackBarDetectionComplete {
                crop_required: false,
                crop_params: Some("disabled".to_string()),
            },
        );
        return (None, false);
    }

    emit_event(event_dispatcher, Event::BlackBarDetectionStarted);

    match crop_detection::detect_crop(input_path, video_props, disable_crop, event_dispatcher) {
        Ok(result) => {
            emit_event(
                event_dispatcher,
                Event::BlackBarDetectionComplete {
                    crop_required: result.0.is_some(),
                    crop_params: result.0.clone(),
                },
            );
            result
        }
        Err(e) => {
            let warning_msg = format!(
                "Crop detection failed for {input_filename}: {e}. Proceeding without cropping."
            );
            emit_event(
                event_dispatcher,
                Event::Warning {
                    message: warning_msg,
                },
            );
            emit_event(
                event_dispatcher,
                Event::BlackBarDetectionComplete {
                    crop_required: false,
                    crop_params: None,
                },
            );
            (None, false)
        }
    }
}

/// Run noise analysis when enabled; emits events and handles notification on failure.
pub fn run_noise_analysis(
    input_path: &Path,
    video_props: &VideoProperties,
    config: &CoreConfig,
    notification_sender: Option<&dyn NotificationSender>,
    event_dispatcher: Option<&EventDispatcher>,
    input_filename: &str,
) -> CoreResult<Option<noise_analysis::NoiseAnalysis>> {
    if !config.enable_denoise {
        return Ok(None);
    }

    emit_event(event_dispatcher, Event::NoiseAnalysisStarted);
    match noise_analysis::analyze_noise(input_path, video_props, event_dispatcher) {
        Ok(analysis) => {
            emit_event(
                event_dispatcher,
                Event::NoiseAnalysisComplete {
                    average_noise: analysis.average_noise,
                    has_significant_noise: analysis.has_significant_noise,
                    recommended_params: analysis.recommended_hqdn3d.clone(),
                },
            );
            Ok(Some(analysis))
        }
        Err(e) => {
            let error_msg = format!("Noise analysis failed for {input_filename}: {e}");
            emit_event(
                event_dispatcher,
                Event::Error {
                    title: "Noise Analysis Failed".to_string(),
                    message: error_msg.clone(),
                    context: Some(format!("File: {}", input_path.display())),
                    suggestion: Some("Check if the video file is valid and accessible".to_string()),
                },
            );

            send_notification_safe(
                notification_sender,
                &format!("Error encoding {input_filename}: Noise analysis failed"),
                "error",
            );

            Err(e)
        }
    }
}
