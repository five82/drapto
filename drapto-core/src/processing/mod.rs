// drapto-core/src/processing/mod.rs
//
// This module serves as the central hub for the core video processing logic
// within the `drapto-core` library. It organizes different processing steps
// into submodules and exposes the primary functions for initiating these tasks.
//
// Submodules:
// - `video`: Contains the main video encoding orchestration logic, handling the
//   processing of multiple video files based on the provided configuration.
//
// Public API Re-exports:
// This `mod.rs` file re-exports the main entry points from its submodules
// to simplify the public API of the `processing` module:
// - `process_videos`: From the `video` submodule, the main function to process
//   a list of video files according to the core configuration.
//
// Internal implementation details within the submodules (like specific sampling
// or analysis functions) are kept private to those submodules and are not
// re-exported here.

// Declare submodules
pub mod video;
pub mod audio; // Add the new audio module

// Re-export public API functions
pub use video::process_videos;

// Note: Functions related to video processing are kept internal or passed via
// dependency injection where needed, so they are not re-exported here.