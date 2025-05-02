// drapto-core/src/external/mocks.rs

// --- Mocking Infrastructure (for testing) ---

// This module is only compiled when the "test-mocks" feature is enabled.
#![cfg(feature = "test-mocks")]

use super::*;
use crate::error::{CoreError, CoreResult};
use crate::processing::detection::VideoProperties; // Import VideoProperties
use ffmpeg_sidecar::command::FfmpegCommand;
use ffmpeg_sidecar::event::FfmpegEvent;
use std::cell::RefCell;
use std::collections::HashMap;
use std::os::unix::process::ExitStatusExt; // For ExitStatus::from_raw
use std::path::{Path, PathBuf};
use std::process::ExitStatus;
use std::rc::Rc;
use log; // Import log crate

/// Mock implementation of FfmpegProcess.
#[derive(Clone)]
pub struct MockFfmpegProcess {
    /// Events to emit when handle_events is called.
    pub events_to_emit: Rc<RefCell<Vec<FfmpegEvent>>>,
    /// Exit status to return when wait is called.
    pub exit_status: ExitStatus,
}

impl FfmpegProcess for MockFfmpegProcess {
    fn handle_events<F>(&mut self, mut handler: F) -> CoreResult<()>
        where F: FnMut(FfmpegEvent) -> CoreResult<()>
    {
        let events = self.events_to_emit.borrow().clone();
        for event in events {
            handler(event)?;
        }
        Ok(())
    }

    fn wait(&mut self) -> CoreResult<ExitStatus> {
        Ok(self.exit_status)
    }
}

/// Represents an expected ffmpeg command call and its mock result.
pub struct MockFfmpegExpectation {
    pub arg_pattern: String,
    pub result: CoreResult<MockFfmpegProcess>,
    pub create_dummy_output: bool,
}

/// Mock implementation of FfmpegSpawner supporting multiple expectations.
#[derive(Clone, Default)]
pub struct MockFfmpegSpawner {
    expectations: Rc<RefCell<Vec<MockFfmpegExpectation>>>,
    received_calls: Rc<RefCell<Vec<Vec<String>>>>,
}

impl MockFfmpegSpawner {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add_expectation(
        &self,
        arg_pattern: &str,
        result: CoreResult<MockFfmpegProcess>,
        create_dummy_output: bool,
    ) {
        self.expectations.borrow_mut().push(MockFfmpegExpectation {
            arg_pattern: arg_pattern.to_string(),
            result,
            create_dummy_output,
        });
    }

    pub fn add_success_expectation(
        &self,
        arg_pattern: &str,
        events: Vec<FfmpegEvent>,
        create_dummy_output: bool,
    ) {
        let process = MockFfmpegProcess {
            events_to_emit: Rc::new(RefCell::new(events)),
            exit_status: ExitStatus::from_raw(0),
        };
        self.add_expectation(arg_pattern, Ok(process), create_dummy_output);
    }

     pub fn add_spawn_error_expectation(&self, arg_pattern: &str, error: CoreError) {
         self.add_expectation(arg_pattern, Err(error), false);
     }

     pub fn add_exit_error_expectation(
         &self,
         arg_pattern: &str,
         events: Vec<FfmpegEvent>,
         exit_code: i32,
     ) {
         let process = MockFfmpegProcess {
             events_to_emit: Rc::new(RefCell::new(events)),
             exit_status: ExitStatus::from_raw(exit_code),
         };
         self.add_expectation(arg_pattern, Ok(process), false);
     }

    pub fn get_received_calls(&self) -> Vec<Vec<String>> {
        self.received_calls.borrow().clone()
    }
}

impl FfmpegSpawner for MockFfmpegSpawner {
    type Process = MockFfmpegProcess;

    fn spawn(&self, cmd: FfmpegCommand) -> CoreResult<Self::Process> {
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().into_owned()).collect();
        self.received_calls.borrow_mut().push(args.clone());

        let mut expectations = self.expectations.borrow_mut();

        let found_index = expectations.iter().position(|exp| {
            args.iter().any(|arg| arg.contains(&exp.arg_pattern))
        });

        if let Some(index) = found_index {
            let expectation = expectations.remove(index);
            log::info!("MockFfmpegSpawner: Matched expectation with pattern '{}'", expectation.arg_pattern);

            match expectation.result {
                Ok(process) => {
                    if expectation.create_dummy_output {
                        if let Some(output_path_str) = args.last() {
                            let output_path = std::path::PathBuf::from(output_path_str);
                            if let Some(parent) = output_path.parent() {
                                if let Err(e) = std::fs::create_dir_all(parent) {
                                     log::error!("MockFfmpegSpawner failed to create parent dir {:?}: {}", parent, e);
                                }
                            }
                            match std::fs::File::create(&output_path) {
                                Ok(_) => log::info!("MockFfmpegSpawner created dummy output file: {:?}", output_path),
                                Err(e) => log::error!("MockFfmpegSpawner failed to create dummy output file {:?}: {}", output_path, e),
                            }
                        } else {
                            log::warn!("MockFfmpegSpawner couldn't find output path in args to create dummy file.");
                        }
                    }
                    Ok(process)
                }
                Err(err) => {
                    log::warn!("MockFfmpegSpawner simulating spawn error for pattern '{}': {:?}", expectation.arg_pattern, err);
                    Err(err)
                }
            }
        } else {
            log::error!("MockFfmpegSpawner: No expectation found for command args: {:?}", args);
            panic!("MockFfmpegSpawner: No expectation found for command args: {:?}", args);
        }
    }
}

/// Mock implementation of FfprobeExecutor.
#[derive(Clone, Default)]
pub struct MockFfprobeExecutor {
    audio_channel_results: Rc<RefCell<HashMap<PathBuf, CoreResult<Vec<u32>>>>>,
    /// Map of input path -> Result<VideoProperties> for get_video_properties
    video_properties_results: Rc<RefCell<HashMap<PathBuf, CoreResult<VideoProperties>>>>, // Add field
}

impl MockFfprobeExecutor {
    pub fn new() -> Self {
        Default::default()
    }

    /// Add an expected result for get_audio_channels for a specific input path.
    pub fn expect_audio_channels(&self, input_path: &Path, result: CoreResult<Vec<u32>>) {
        // Need to ensure CoreError is Clone if result is Err
        self.audio_channel_results.borrow_mut().insert(input_path.to_path_buf(), result);
    }

    /// Add an expected result for get_video_properties for a specific input path.
    pub fn expect_video_properties(&self, input_path: &Path, result: CoreResult<VideoProperties>) { // Add method
        // Need to ensure CoreError is Clone if result is Err
        // VideoProperties should be Clone
        self.video_properties_results.borrow_mut().insert(input_path.to_path_buf(), result);
    }
}

impl FfprobeExecutor for MockFfprobeExecutor {
    fn get_audio_channels(&self, input_path: &Path) -> CoreResult<Vec<u32>> {
        log::info!("MockFfprobeExecutor::get_audio_channels called for: {}", input_path.display());
        match self.audio_channel_results.borrow().get(input_path) {
             Some(Ok(channels)) => Ok(channels.clone()),
             Some(Err(err)) => {
                 log::warn!("MockFfprobeExecutor returning stored error (type might differ due to no Clone): {:?}", err);
                 Err(CoreError::FfprobeParse(format!("Mock ffprobe error for {}: {:?}", input_path.display(), err)))
             }
            None => {
                log::error!("MockFfprobeExecutor: No expectation set for get_audio_channels on path: {}", input_path.display());
                Err(CoreError::FfprobeParse(format!("MockFfprobeExecutor: No expectation set for path {}", input_path.display())))
            }
        }
    } // Close get_audio_channels method here

    // Add get_video_properties method implementation
    fn get_video_properties(&self, input_path: &Path) -> CoreResult<VideoProperties> {
        log::info!("MockFfprobeExecutor::get_video_properties called for: {}", input_path.display());
        match self.video_properties_results.borrow().get(input_path) {
            Some(Ok(props)) => Ok(props.clone()), // Clone VideoProperties if Ok
            Some(Err(err)) => {
                // Cannot clone CoreError, reconstruct
                log::warn!("MockFfprobeExecutor returning stored error (type might differ due to no Clone): {:?}", err);
                Err(CoreError::VideoInfoError(format!("Mock ffprobe error for {}: {:?}", input_path.display(), err)))
            },
            None => {
                log::error!("MockFfprobeExecutor: No expectation set for get_video_properties on path: {}", input_path.display());
                Err(CoreError::VideoInfoError(format!("MockFfprobeExecutor: No expectation set for path {}", input_path.display())))
            }
        }
    }
} // Close impl FfprobeExecutor block here