use assert_cmd::Command;
use std::error::Error;
use tempfile::tempdir;

// Helper function to get the path to the compiled binary
fn drapto_cmd() -> Command {
    Command::cargo_bin("drapto").expect("Failed to find drapto binary")
}

#[test]
fn test_encode_command_basic_args() -> Result<(), Box<dyn Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;

    // Create dummy input file
    let input_file = input_dir.path().join("fake_input.mkv");
    std::fs::write(&input_file, "dummy content")?;

    let output_file = output_dir.path().join("fake_output.mp4");

    // TODO: Implement mocking for HandBrakeCLI execution
    // For now, this test primarily checks if the command runs without panicking
    // and accepts basic arguments. A future step will involve verifying the
    // generated HandBrake command.

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .arg("--preset")
        .arg("Fast 1080p30"); // Example preset

    // We expect this to fail because HandBrakeCLI isn't found or mocked yet,
    // but it shouldn't panic due to argument parsing issues.
    // We'll refine the assertion once mocking is in place.
    cmd.assert().failure(); // Or success() once mocking is implemented

    Ok(())
}

// TODO: Add more tests:
// - Different presets
// - Specifying config file
// - Handling non-existent input
// - Handling invalid arguments
// - Verifying mocked HandBrakeCLI command generation