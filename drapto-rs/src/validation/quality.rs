use std::path::Path;
use std::process::Command;

use log::error;
use serde_json::Value;

use crate::error::{DraptoError, Result};
use crate::logging;
use super::ValidationReport;

/// Validate encoding quality using VMAF
pub fn validate_vmaf<P: AsRef<Path>>(
    reference_path: P,
    encoded_path: P,
    target_score: f32,
    report: &mut ValidationReport
) -> Result<f32> {
    // Run FFmpeg with libvmaf filter to get VMAF score
    let vmaf_score = calculate_vmaf(reference_path, encoded_path)?;
    
    // Add VMAF score to report
    report.add_info(
        format!("VMAF score: {:.2}", vmaf_score),
        "Quality"
    );
    
    // Check if VMAF score meets target
    if vmaf_score < target_score {
        report.add_error(
            format!("VMAF score {:.2} is below target of {:.2}", vmaf_score, target_score),
            "Quality"
        );
    } else {
        report.add_info(
            format!("VMAF score {:.2} meets or exceeds target of {:.2}", vmaf_score, target_score),
            "Quality"
        );
    }
    
    Ok(vmaf_score)
}

/// Calculate VMAF score between reference and encoded video
fn calculate_vmaf<P: AsRef<Path>>(reference_path: P, encoded_path: P) -> Result<f32> {
    let reference = reference_path.as_ref();
    let encoded = encoded_path.as_ref();
    
    if !reference.exists() {
        return Err(DraptoError::Validation(
            format!("Reference file not found: {:?}", reference)
        ));
    }
    
    if !encoded.exists() {
        return Err(DraptoError::Validation(
            format!("Encoded file not found: {:?}", encoded)
        ));
    }
    
    // Create temporary file for JSON output
    let temp_dir = std::env::temp_dir();
    let json_output = temp_dir.join("vmaf_output.json");
    
    // Build FFmpeg command for VMAF calculation
    let mut cmd = Command::new("ffmpeg");
    cmd.args([
        "-i", encoded.to_str().unwrap_or_default(),
        "-i", reference.to_str().unwrap_or_default(),
        "-filter_complex", &format!(
            "[0:v]setpts=PTS-STARTPTS[distorted];\
             [1:v]setpts=PTS-STARTPTS[reference];\
             [distorted][reference]libvmaf=log_fmt=json:log_path={}:model_path=vmaf_v0.6.1.json",
            json_output.to_str().unwrap_or_default()
        ),
        "-f", "null", "-"
    ]);
    
    logging::log_command(&cmd);
    
    // Execute command
    let output = cmd.output()
        .map_err(|e| DraptoError::CommandExecution(
            format!("Failed to execute FFmpeg for VMAF calculation: {}", e)
        ))?;
    
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        error!("VMAF calculation failed: {}", stderr);
        return Err(DraptoError::CommandExecution(
            format!("VMAF calculation failed: {}", stderr)
        ));
    }
    
    // Read and parse JSON output
    if !json_output.exists() {
        return Err(DraptoError::Validation(
            "VMAF calculation completed but no output file was created".to_string()
        ));
    }
    
    let json_content = std::fs::read_to_string(&json_output)
        .map_err(|e| DraptoError::Validation(
            format!("Failed to read VMAF output file: {}", e)
        ))?;
    
    let json: Value = serde_json::from_str(&json_content)
        .map_err(|e| DraptoError::Validation(
            format!("Failed to parse VMAF JSON output: {}", e)
        ))?;
    
    // Extract pooled VMAF score
    let vmaf_score = json["pooled_metrics"]["vmaf"]["mean"]
        .as_f64()
        .ok_or_else(|| DraptoError::Validation(
            "Failed to extract VMAF score from JSON output".to_string()
        ))? as f32;
    
    // Clean up temporary file
    let _ = std::fs::remove_file(json_output);
    
    Ok(vmaf_score)
}

/// Check if libvmaf is available
pub fn is_vmaf_available() -> bool {
    let output = Command::new("ffmpeg")
        .arg("-filters")
        .output();
    
    match output {
        Ok(output) => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            stdout.contains("libvmaf")
        },
        Err(_) => false
    }
}