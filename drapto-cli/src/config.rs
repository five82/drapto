// drapto-cli/src/config.rs
//
// Defines default configuration constants for the `drapto-cli` application,
// primarily related to encoding parameters.


pub const DEFAULT_ENCODER_PRESET: i32 = 6;
pub const DEFAULT_CROP_MODE: &str = "auto";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_values() {
        assert_eq!(DEFAULT_ENCODER_PRESET, 6);
        assert_eq!(DEFAULT_CROP_MODE, "auto");
    }
}