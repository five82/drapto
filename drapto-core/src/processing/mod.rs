//! Module for specific video processing steps like film grain optimization and the main processing loop.

// Declare submodules
pub mod film_grain;
pub mod video;

// Re-export public API functions
pub use film_grain::determine_optimal_grain;
pub use video::process_videos;

// Note: Functions like extract_and_test_sample and get_video_duration_secs are pub(crate)
// within film_grain::sampling and are passed via dependency injection where needed,
// so they are not re-exported here.