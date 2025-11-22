//! Helper wrappers for crop detection orchestration.
//!
//! These functions keep event/notification wiring close to the operations
//! while letting the main workflow stay slimmer.

use crate::config::CoreConfig;
use crate::events::{Event, EventDispatcher};
use crate::processing::crop_detection;
use crate::processing::reporting::emit_event;
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
