// drapto-core/src/processing/film_grain/types.rs
//
// This module defines the data structures specifically used within the
// film grain optimization logic (`processing::film_grain` module) to
// store and manage the results of testing different grain values.
//
// Types:
// - `GrainTest`: A struct representing the outcome of encoding a single video
//   sample with a specific `grain_value`. It stores the `grain_value` tested
//   and the resulting `file_size` in bytes. It includes a custom `Debug`
//   implementation to format the file size nicely (in MB) for logging.
// - `SampleResult`: A type alias for `Vec<GrainTest>`. It represents the
//   collection of all `GrainTest` results obtained from testing different
//   grain values at a single sample point within the video.
// - `AllResults`: A type alias for `Vec<SampleResult>`. It represents the
//   complete set of results across all sample points tested in the video.
//   Each element in the outer vector corresponds to a sample point, and the
//   inner `SampleResult` vector holds the tests for that point.

use std::fmt;
use std::vec::Vec; // Explicit import for clarity, though often prelude is enough

// --- Data Structures ---

/// Stores the result of a single grain test encode for a sample
// Keep Clone separate
#[derive(Clone)]
pub struct GrainTest { // Made pub
    pub grain_value: u8, // Made pub
    pub file_size: u64, // Bytes - Made pub
}

impl fmt::Debug for GrainTest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let file_size_mb = self.file_size as f64 / (1024.0 * 1024.0);
        f.debug_struct("GrainTest")
            .field("grain_value", &self.grain_value)
            // Format file_size to 2 decimal places and add " MB"
            .field("file_size", &format_args!("{:.2} MB", file_size_mb))
            .finish()
    }
}

/// Stores all test results for a single sample point
pub type SampleResult = Vec<GrainTest>; // Made pub

/// Stores all results across all sample points
pub type AllResults = Vec<SampleResult>; // Made pub