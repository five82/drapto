//! JSON progress handler for structured progress output
//!
//! This module provides a JSON-based event handler that outputs structured
//! progress information to stdout for consumption by external tools like spindle.

use super::{Event, EventHandler};
use serde_json::json;
use std::io::{self, Write};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

/// Event handler that outputs progress events as structured JSON to stdout
pub struct JsonProgressHandler {
    output: Mutex<Box<dyn Write + Send>>,
}

impl JsonProgressHandler {
    /// Create a new JSON progress handler that writes to stdout
    pub fn new() -> Self {
        Self {
            output: Mutex::new(Box::new(io::stdout())),
        }
    }

    /// Create a new JSON progress handler with a custom writer
    #[allow(dead_code)]
    pub fn with_writer(writer: Box<dyn Write + Send>) -> Self {
        Self {
            output: Mutex::new(writer),
        }
    }

    /// Get current timestamp as seconds since Unix epoch
    fn get_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }

    /// Write a JSON progress event to the output
    fn write_json(&self, value: serde_json::Value) {
        if let Ok(mut output) = self.output.lock() {
            if let Ok(json_str) = serde_json::to_string(&value) {
                let _ = writeln!(output, "{}", json_str);
                let _ = output.flush();
            }
        }
    }
}

impl EventHandler for JsonProgressHandler {
    fn handle(&self, event: &Event) {
        let timestamp = Self::get_timestamp();

        match event {
            Event::StageProgress {
                stage,
                percent,
                message,
                eta,
            } => {
                let progress = json!({
                    "type": "stage_progress",
                    "stage": stage,
                    "percent": percent,
                    "message": message,
                    "eta_seconds": eta.map(|d| d.as_secs()),
                    "timestamp": timestamp
                });
                self.write_json(progress);
            }

            Event::EncodingProgress {
                current_frame,
                total_frames,
                percent,
                speed,
                fps,
                eta,
                bitrate,
            } => {
                // Only output encoding progress every 5% to avoid interfering with interactive progress bars
                if (*percent as u32) % 5 == 0 || *percent >= 99.0 {
                    let progress = json!({
                        "type": "encoding_progress",
                        "stage": "encoding",
                        "current_frame": current_frame,
                        "total_frames": total_frames,
                        "percent": percent,
                        "speed": speed,
                        "fps": fps,
                        "eta_seconds": eta.as_secs(),
                        "bitrate": bitrate,
                        "timestamp": timestamp
                    });
                    self.write_json(progress);
                }
            }

            Event::InitializationStarted {
                input_file,
                output_file,
                duration,
                resolution,
                category,
                dynamic_range,
                audio_description,
            } => {
                let init = json!({
                    "type": "initialization",
                    "input_file": input_file,
                    "output_file": output_file,
                    "duration": duration,
                    "resolution": resolution,
                    "category": category,
                    "dynamic_range": dynamic_range,
                    "audio_description": audio_description,
                    "timestamp": timestamp
                });
                self.write_json(init);
            }

            Event::EncodingComplete {
                input_file,
                output_file,
                original_size,
                encoded_size,
                total_time,
                ..
            } => {
                let complete = json!({
                    "type": "encoding_complete",
                    "input_file": input_file,
                    "output_file": output_file,
                    "original_size": original_size,
                    "encoded_size": encoded_size,
                    "duration_seconds": total_time.as_secs(),
                    "size_reduction_percent": if *original_size > 0 {
                        (((*original_size as f64) - (*encoded_size as f64)) / (*original_size as f64) * 100.0).round()
                    } else {
                        0.0
                    },
                    "timestamp": timestamp
                });
                self.write_json(complete);
            }

            Event::ValidationComplete {
                validation_passed,
                validation_steps,
            } => {
                let validation = json!({
                    "type": "validation_complete",
                    "validation_passed": validation_passed,
                    "validation_steps": validation_steps.iter().map(|(step, passed, details)| {
                        json!({
                            "step": step,
                            "passed": passed,
                            "details": details
                        })
                    }).collect::<Vec<_>>(),
                    "timestamp": timestamp
                });
                self.write_json(validation);
            }

            Event::Error {
                title,
                message,
                context,
                suggestion,
            } => {
                let error = json!({
                    "type": "error",
                    "title": title,
                    "message": message,
                    "context": context,
                    "suggestion": suggestion,
                    "timestamp": timestamp
                });
                self.write_json(error);
            }

            Event::Warning { message } => {
                let warning = json!({
                    "type": "warning",
                    "message": message,
                    "timestamp": timestamp
                });
                self.write_json(warning);
            }

            Event::BatchComplete {
                successful_count,
                total_files,
                total_original_size,
                total_encoded_size,
                total_duration,
                ..
            } => {
                let batch_complete = json!({
                    "type": "batch_complete",
                    "successful_count": successful_count,
                    "total_files": total_files,
                    "total_original_size": total_original_size,
                    "total_encoded_size": total_encoded_size,
                    "total_duration_seconds": total_duration.as_secs(),
                    "total_size_reduction_percent": if *total_original_size > 0 {
                        (((*total_original_size as f64) - (*total_encoded_size as f64)) / (*total_original_size as f64) * 100.0).round()
                    } else {
                        0.0
                    },
                    "timestamp": timestamp
                });
                self.write_json(batch_complete);
            }

            // For other events that might be relevant for progress tracking
            _ => {
                // Optionally handle other events here if needed
            }
        }
    }
}

impl Default for JsonProgressHandler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    struct MockWriter {
        content: Arc<Mutex<Vec<u8>>>,
    }

    impl MockWriter {
        fn new() -> (Self, Arc<Mutex<Vec<u8>>>) {
            let content = Arc::new(Mutex::new(Vec::new()));
            (
                Self {
                    content: content.clone(),
                },
                content,
            )
        }
    }

    impl Write for MockWriter {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.content.lock().unwrap().extend_from_slice(buf);
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_stage_progress_json() {
        let (writer, content) = MockWriter::new();
        let handler = JsonProgressHandler::with_writer(Box::new(writer));

        let event = Event::StageProgress {
            stage: "analysis".to_string(),
            percent: 25.0,
            message: "Analyzing video properties".to_string(),
            eta: Some(Duration::from_secs(30)),
        };

        handler.handle(&event);

        let output = String::from_utf8(content.lock().unwrap().clone()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(parsed["type"], "stage_progress");
        assert_eq!(parsed["stage"], "analysis");
        assert_eq!(parsed["percent"], 25.0);
        assert_eq!(parsed["eta_seconds"], 30);
    }

    #[test]
    fn test_encoding_progress_json() {
        let (writer, content) = MockWriter::new();
        let handler = JsonProgressHandler::with_writer(Box::new(writer));

        let event = Event::EncodingProgress {
            current_frame: 1000,
            total_frames: 4000,
            percent: 25.0,
            speed: 1.5,
            fps: 30.0,
            eta: Duration::from_secs(120),
            bitrate: "2000kbps".to_string(),
        };

        handler.handle(&event);

        let output = String::from_utf8(content.lock().unwrap().clone()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(output.trim()).unwrap();

        assert_eq!(parsed["type"], "encoding_progress");
        assert_eq!(parsed["stage"], "encoding");
        assert_eq!(parsed["current_frame"], 1000);
        assert_eq!(parsed["total_frames"], 4000);
        assert_eq!(parsed["percent"], 25.0);
    }
}
