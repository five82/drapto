// Core modules
pub mod error;
pub mod config;
pub mod logging;
pub mod ffprobe;
pub mod command;
pub mod validation;
pub mod video;

// Will be implemented in future phases
pub mod audio {
    // Placeholder for audio processing modules
}

pub mod processing {
    // Placeholder for processing pipeline modules
}

// Version information
pub const VERSION: &str = env!("CARGO_PKG_VERSION");