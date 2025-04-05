// drapto-cli/src/config.rs
//
// This module defines default configuration constants specifically for the
// `drapto-cli` application. These values are used as fallbacks when
// corresponding settings are not provided via command-line arguments or
// potentially other configuration sources in the future.
//
// These defaults relate to HandbrakeCLI encoding parameters.

// Default HandbrakeCLI encoding parameters

pub const DEFAULT_ENCODER_PRESET: i32 = 6;
// pub const DEFAULT_QUALITY: i32 = 27; // Removed
// The following CLI defaults are no longer needed as fallbacks are handled in drapto-core
// pub const DEFAULT_QUALITY_SD: i32 = 28;
// pub const DEFAULT_QUALITY_HD: i32 = 27;
// pub const DEFAULT_QUALITY_UHD: i32 = 26;
pub const DEFAULT_CROP_MODE: &str = "auto"; // Keep this one for now

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        assert_eq!(DEFAULT_ENCODER_PRESET, 6);
        // assert_eq!(DEFAULT_QUALITY, 27); // Removed
        // assert_eq!(DEFAULT_QUALITY_SD, 28); // Removed
        // assert_eq!(DEFAULT_QUALITY_HD, 27); // Removed
        // assert_eq!(DEFAULT_QUALITY_UHD, 26); // Removed
        assert_eq!(DEFAULT_CROP_MODE, "auto");
    }
}