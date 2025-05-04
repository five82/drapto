// drapto-core/src/processing/detection/properties.rs

// Removed unused imports: CoreError, CoreResult, Deserialize, Path, Command, Stdio

// --- Struct to hold extracted properties ---
// This struct remains as it defines the data structure returned by the FfprobeExecutor trait.

#[derive(Debug, Clone, Default)]
pub struct VideoProperties { // Keep public as it's re-exported
    pub width: u32,
    pub height: u32,
    pub duration_secs: f64, // Renamed from duration for clarity
    pub color_space: Option<String>,
    // color_transfer and color_primaries removed as they are not available in ffprobe crate v0.3.3
}

// --- Implementation ---
// The implementation logic (get_video_properties_impl) and the internal ffprobe
// JSON parsing structs (FfprobeOutput, FfprobeFormat, FfprobeStream) have been removed.
// The responsibility for executing ffprobe and parsing its output now lies within
// the CrateFfprobeExecutor implementation in src/external/ffprobe_executor.rs,
// which utilizes the `ffprobe` crate.