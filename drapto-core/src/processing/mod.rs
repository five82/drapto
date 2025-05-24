// ============================================================================
// drapto-core/src/processing/mod.rs
// ============================================================================
//
// VIDEO PROCESSING: Core Video Processing Logic
//
// This module serves as the central hub for the core video processing logic
// within the drapto-core library. It organizes different processing steps
// into submodules and exposes the primary functions for initiating these tasks.
//
// KEY COMPONENTS:
// - Video encoding orchestration (video submodule)
// - Audio stream handling (audio submodule)
// - Video property detection (detection submodule)
// - Crop detection and analysis
//
// WORKFLOW:
// 1. Video properties are detected (resolution, frame rate, etc.)
// 2. Crop parameters are determined if auto-crop is enabled
// 3. Audio streams are analyzed and processed
// 4. Video encoding is performed with optimized parameters
// 5. Results are collected and returned
//
// PUBLIC API:
// - process_videos: Main function to process a list of video files
// - VideoProperties: Structure containing detected video properties
// - detect_crop: Function to detect optimal crop parameters
//
// DESIGN PHILOSOPHY:
// The processing module follows a modular design with clear separation of
// concerns between different aspects of video processing. It uses dependency
// injection for external tools and file system operations to facilitate testing.
//
// AI-ASSISTANT-INFO: Core video processing logic and orchestration

// ============================================================================
// SUBMODULES
// ============================================================================

/// Main video encoding orchestration logic
pub mod video;

/// Audio stream handling and processing
pub mod audio;

/// Video property detection and analysis
pub mod detection;

// ============================================================================
// PUBLIC API RE-EXPORTS
// ============================================================================

/// Main function to process a list of video files
pub use video::process_videos;

/// Structure containing detected video properties
pub use detection::VideoProperties;

/// Function to detect optimal crop parameters
pub use detection::detect_crop;
