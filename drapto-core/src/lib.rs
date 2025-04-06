//! # drapto-core
//!
//! This crate provides the core logic for video processing tasks, primarily focusing
//! on interacting with HandBrakeCLI for encoding and potentially other tools for
//! analysis and optimization (like film grain detection).
//!
//! ## Structure
//!
//! The crate is organized into several modules:
//! - `config`: Defines configuration structures (`CoreConfig`) used throughout the library.
//! - `discovery`: Contains functions for finding processable video files (`find_processable_files`).
//! - `error`: Defines custom error types (`CoreError`) and results (`CoreResult`) for the library.
//! - `external`: Handles interactions with external command-line tools like HandBrakeCLI.
//! - `processing`: Contains the main video processing logic, including encoding orchestration
//!   (`process_videos`) and specific optimizations like film grain analysis
//!   (`determine_optimal_grain`).
//! - `utils`: Provides common utility functions (e.g., `format_bytes`, `format_duration`).
//!
//! ## Public API
//!
//! This `lib.rs` file re-exports the primary public interface elements from the internal
//! modules, making them directly accessible to users of the `drapto-core` crate.
//! It also defines the `EncodeResult` struct, which is returned to report the outcome
//! of individual file encoding operations.

// --- Modules ---
// Declare all the top-level modules as per the refactoring plan.
pub mod config;
pub mod discovery;
pub mod error;
pub mod external; // Note: This is pub but contains pub(crate) items. Fine for now.
pub mod processing;
pub mod utils;
pub mod notifications; // Added for ntfy support

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

// From notifications module
pub use notifications::send_ntfy;

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
