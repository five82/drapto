use drapto_core::config::Config;
use std::path::PathBuf;

#[test]
fn test_builder_pattern() {
    let config = Config::new()
        .with_input("input.mp4")
        .with_output("output.mp4");
    
    assert_eq!(config.input, PathBuf::from("input.mp4"));
    assert_eq!(config.output, PathBuf::from("output.mp4"));
}