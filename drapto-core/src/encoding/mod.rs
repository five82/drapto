//! Encoding module for drapto
//!
//! This module contains functionality for video and audio encoding,
//! including segmentation, parallel encoding, and media processing pipelines.

pub mod video;
pub mod audio;
pub mod pipeline;
pub mod segmentation;
pub mod memory;
pub mod parallel;
pub mod merger;
pub mod muxer;