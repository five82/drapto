// ============================================================================
// drapto-core/src/processing/detection/mod.rs
// ============================================================================
//
// VIDEO DETECTION: Analysis and Property Detection
//
// This module handles the detection and analysis of video properties, including
// resolution, color space, crop parameters, and grain/noise levels. It provides
// functions for analyzing video files to determine optimal encoding parameters.
//
// KEY COMPONENTS:
// - Video property detection (resolution, color space, etc.)
// - Crop detection for removing black bars
// - Grain/noise analysis for optimal denoising
//
// DESIGN PHILOSOPHY:
// The detection module follows a modular approach with separate submodules for
// different types of analysis. Each detection function is designed to be
// independent and testable, with clear inputs and outputs.
//
// AI-ASSISTANT-INFO: Video property detection and analysis for encoding optimization

// ============================================================================
// SUBMODULES
// ============================================================================

/// Video property detection (resolution, color space, etc.)
pub mod properties;

/// Crop detection for removing black bars
pub mod crop_analysis;

/// Grain/noise analysis for optimal denoising
pub mod grain_analysis;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

/// Structure containing detected video properties
pub use properties::VideoProperties;

/// Function to detect optimal crop parameters
pub use crop_analysis::detect_crop;

/// Grain analysis function and related types
pub use grain_analysis::{GrainAnalysisResult, GrainLevel, analyze_grain};
