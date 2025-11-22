use std::sync::Arc;
use std::time::Duration;

pub mod json_handler;

#[derive(Debug, Clone)]
pub enum Event {
    // System information events
    HardwareInfo {
        hostname: String,
        os: String,
        cpu: String,
        memory: String,
    },

    // Initialization events
    InitializationStarted {
        input_file: String,
        output_file: String,
        duration: String,
        resolution: String,
        category: String,          // HD, UHD, SD
        dynamic_range: String,     // HDR, SDR
        audio_description: String, // e.g., "5.1 surround", "stereo"
    },

    // Analysis events
    VideoAnalysisStarted,
    BlackBarDetectionStarted,
    BlackBarDetectionProgress {
        current: u64,
        total: u64,
    },
    BlackBarDetectionComplete {
        crop_required: bool,
        crop_params: Option<String>,
    },

    // Encoding events
    EncodingConfigurationDisplayed {
        encoder: String,
        preset: String,
        tune: String,
        quality: String,
        pixel_format: String,
        matrix_coefficients: String,
        audio_codec: String,
        audio_description: String,
    },

    EncodingStarted {
        total_frames: u64,
    },

    EncodingProgress {
        current_frame: u64,
        total_frames: u64,
        percent: f32,
        speed: f32,
        fps: f32,
        eta: Duration,
        bitrate: String,
    },

    StageProgress {
        stage: String,
        percent: f32,
        message: String,
        eta: Option<Duration>,
    },

    EncodingComplete {
        input_file: String,
        output_file: String,
        original_size: u64,
        encoded_size: u64,
        video_stream: String,
        audio_stream: String,
        total_time: Duration,
        average_speed: f32,
        output_path: String,
    },

    ValidationComplete {
        validation_passed: bool,
        validation_steps: Vec<(String, bool, String)>, // (step_name, passed, details)
    },

    // Error events
    Error {
        title: String,
        message: String,
        context: Option<String>,
        suggestion: Option<String>,
    },

    Warning {
        message: String,
    },

    // Generic events
    StatusUpdate {
        label: String,
        value: String,
        emphasize: bool,
    },

    ProcessingStep {
        message: String,
    },

    OperationComplete {
        message: String,
    },

    // Batch processing events
    BatchInitializationStarted {
        total_files: usize,
        file_list: Vec<String>,
        output_dir: String,
    },

    FileProgressContext {
        current_file: usize,
        total_files: usize,
    },

    BatchComplete {
        successful_count: usize,
        total_files: usize,
        total_original_size: u64,
        total_encoded_size: u64,
        total_duration: Duration,
        average_speed: f32,
        file_results: Vec<(String, f64)>, // (filename, reduction_percentage)
        validation_passed_count: usize,
        validation_failed_count: usize,
    },
}

pub trait EventHandler: Send + Sync {
    fn handle(&self, event: &Event);
}

pub struct EventDispatcher {
    handlers: Vec<Arc<dyn EventHandler>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            handlers: Vec::new(),
        }
    }

    pub fn add_handler(&mut self, handler: Arc<dyn EventHandler>) {
        self.handlers.push(handler);
    }

    pub fn emit(&self, event: Event) {
        for handler in &self.handlers {
            handler.handle(&event);
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
