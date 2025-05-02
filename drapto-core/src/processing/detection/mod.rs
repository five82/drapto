// drapto-core/src/processing/detection/mod.rs

// Declare submodules
pub mod properties; // Make public
pub mod crop_analysis; // Make public

// Re-export public items
pub use properties::VideoProperties;
pub use crop_analysis::detect_crop; // Assuming detect_crop moves to crop_analysis.rs