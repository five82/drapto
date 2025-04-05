// drapto-core/src/processing/mod.rs
//
// This module serves as the central hub for the core video processing logic
// within the `drapto-core` library. It organizes different processing steps
// into submodules and exposes the primary functions for initiating these tasks.
//
// Submodules:
// - `film_grain`: Contains logic specifically related to analyzing and optimizing
//   film grain settings for video encodes.
// - `video`: Contains the main video encoding orchestration logic, handling the
//   processing of multiple video files based on the provided configuration.
//
// Public API Re-exports:
// This `mod.rs` file re-exports the main entry points from its submodules
// to simplify the public API of the `processing` module:
// - `determine_optimal_grain`: From the `film_grain` submodule, used to find
//   the best film grain value for a given video.
// - `process_videos`: From the `video` submodule, the main function to process
//   a list of video files according to the core configuration.
//
// Internal implementation details within the submodules (like specific sampling
// or analysis functions) are kept private to those submodules and are not
// re-exported here.

// Declare submodules
pub mod film_grain;
pub mod video;

// Re-export public API functions
pub use film_grain::determine_optimal_grain;
pub use video::process_videos;

// Note: Functions like extract_and_test_sample and get_video_duration_secs are pub(crate)
// within film_grain::sampling and are passed via dependency injection where needed,
// so they are not re-exported here.