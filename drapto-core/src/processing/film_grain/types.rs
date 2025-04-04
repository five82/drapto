// drapto-core/src/processing/film_grain/types.rs
// Responsibility: Define data structures specific to film grain processing.

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