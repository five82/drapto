pub mod encode;
pub mod validate;
pub mod info;

// Re-export common command functionality
pub use encode::execute_encode;
pub use validate::execute_validate;
pub use info::execute_ffmpeg_info;