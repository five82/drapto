use assert_cmd::Command;
use std::path::PathBuf;
use predicates::str::contains; // Import only what's needed
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

    // TODO: Implement mocking for ffmpeg/ffprobe execution or use real binaries with dummy files.
    // For now, this test primarily checks if the command runs without panicking
    // and accepts basic arguments. A future step will involve verifying the
    // generated ffmpeg command or the output file.

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_file.to_str().unwrap())
        .arg("--preset")
        .arg("6"); // Use a valid integer for preset

    // With the "test-mock-ffmpeg" feature enabled via dev-dependencies,
    // the drapto binary should use the MockFfmpegSpawner and Mock ffprobe,
    // allowing the command to succeed even with dummy files.
    cmd.assert().success(); // Expect success now with mocks

    Ok(())
}

// TODO: Add more tests:
// - Different presets
// - Specifying config file (Done below)
// - Handling non-existent input
// - Handling invalid arguments (Done below)
// - Verifying mocked ffmpeg command generation or output file properties

// Removed test_encode_command_with_config_file as --config flag doesn't exist

#[test]
fn test_encode_command_non_existent_input() -> Result<(), Box<dyn Error>> {
    let output_dir = tempdir()?;
    let non_existent_input = PathBuf::from("surely/this/does/not/exist/input.mkv");

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(non_existent_input.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.path().to_str().unwrap());

    // Expect failure because the input path is invalid
    cmd.assert()
       .failure()
       .stderr(contains("Invalid input path")); // Use imported function directly

    Ok(())
}

#[test]
fn test_encode_command_different_preset() -> Result<(), Box<dyn Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;

    // Create dummy input file
    let input_file = input_dir.path().join("preset_test.mkv");
    std::fs::write(&input_file, "dummy content")?;

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.path().to_str().unwrap())
        .arg("--preset")
        .arg("4"); // Different valid preset

    // Expect success because mocks are enabled
    cmd.assert().success();

    // TODO: Could enhance mock spawner to capture args and verify preset "4" was used internally

    Ok(())
}

#[test]
fn test_encode_command_invalid_quality() -> Result<(), Box<dyn Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;

    // Create dummy input file
    let input_file = input_dir.path().join("invalid_quality.mkv");
    std::fs::write(&input_file, "dummy content")?;

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.path().to_str().unwrap())
        .arg("--quality-sd")
        .arg("300"); // Invalid quality value (likely outside u8 range or clap validation if added)

    // Expect failure due to clap parsing/validation
    cmd.assert()
        .failure()
        .stderr(contains("invalid value '300'")); // Check clap's error message for out-of-range or parse error

    Ok(())
}

#[test]
fn test_encode_command_with_quality_args() -> Result<(), Box<dyn Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;

    // Create dummy input file
    let input_file = input_dir.path().join("quality_test.mkv");
    std::fs::write(&input_file, "dummy content")?;

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.path().to_str().unwrap())
        .arg("--quality-sd")
        .arg("28")
        .arg("--quality-hd")
        .arg("26");
        // Not specifying UHD, should use default

    // Expect success because mocks are enabled
    cmd.assert().success();

    // TODO: Could enhance mock spawner/CLI logging to verify CRF values were used internally

    Ok(())
}

#[test]
fn test_encode_command_invalid_preset() -> Result<(), Box<dyn Error>> {
    let input_dir = tempdir()?;
    let output_dir = tempdir()?;

    // Create dummy input file
    let input_file = input_dir.path().join("invalid_preset.mkv");
    std::fs::write(&input_file, "dummy content")?;

    let mut cmd = drapto_cmd();
    cmd.arg("encode")
        .arg("--input")
        .arg(input_file.to_str().unwrap())
        .arg("--output")
        .arg(output_dir.path().to_str().unwrap())
        .arg("--preset")
        .arg("99"); // Invalid preset value (0-13 allowed)

    // Expect failure due to clap validation
    cmd.assert()
        .failure()
        .stderr(contains("invalid value '99'")); // Check clap's error message

    Ok(())
}