//! drapto-core: The core library for video processing tasks.

// --- Modules ---
// Declare all the top-level modules as per the refactoring plan.
pub mod config;
pub mod discovery;
pub mod error;
pub mod external; // Note: This is pub but contains pub(crate) items. Fine for now.
pub mod processing;
pub mod utils;

// --- Public API Re-exports ---
// Re-export items intended for public use from their respective modules.

// From config module
pub use config::{CoreConfig, FilmGrainMetricType};

// From discovery module
pub use discovery::find_processable_files;

// From error module
pub use error::{CoreError, CoreResult};

// From processing module (which itself re-exports from submodules)
pub use processing::{determine_optimal_grain, process_videos};

// From utils module (public helper functions)
pub use utils::{format_bytes, format_duration};

// --- Public Structs (defined directly in lib.rs) ---
// EncodeResult remains here as it's a simple data structure returned by the public API.
use std::time::Duration; // Keep only necessary imports for items defined here

#[derive(Debug, Clone)] // Clone might be useful for the CLI
pub struct EncodeResult {
    pub filename: String,
    pub duration: Duration,
    pub input_size: u64,
    pub output_size: u64,
}

// Note: Removed unused imports like std::fs, std::io, std::process, etc.
// Note: Path and PathBuf are no longer directly used in lib.rs's remaining code.
