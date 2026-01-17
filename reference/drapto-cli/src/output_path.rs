//! Output path resolution for the CLI.
//!
//! Handles the logic for determining whether the -o argument is a directory
//! or a specific output filename.

use drapto_core::CoreError;
use std::ffi::OsString;
use std::path::{Path, PathBuf};

/// Resolved output path information.
#[derive(Debug)]
pub struct OutputPathInfo {
    /// The directory where output files should be written.
    pub output_dir: PathBuf,
    /// Optional filename override (when user specifies output.mkv instead of a directory).
    pub filename_override: Option<OsString>,
}

/// Resolves the output path argument into a directory and optional filename.
///
/// When the input is a single file AND the output has a `.mkv` extension,
/// the output is treated as a filename. Otherwise, it's treated as a directory.
///
/// Returns an error if the output has a non-.mkv extension.
pub fn resolve_output_path(input_path: &Path, output_path: &Path) -> Result<OutputPathInfo, CoreError> {
    if input_path.is_file() && output_path.extension().is_some() {
        // Single file input with extension on output - treat as filename
        let ext = output_path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase());

        if ext.as_deref() != Some("mkv") {
            return Err(CoreError::PathError(
                "Output filename must have .mkv extension".to_string(),
            ));
        }

        let parent_dir = output_path
            .parent()
            .map(Path::to_path_buf)
            .filter(|p| !p.as_os_str().is_empty())
            .unwrap_or_else(|| PathBuf::from("."));
        let filename = output_path.file_name().map(OsString::from);

        Ok(OutputPathInfo {
            output_dir: parent_dir,
            filename_override: filename,
        })
    } else {
        // Directory input OR no extension - treat output as directory
        Ok(OutputPathInfo {
            output_dir: output_path.to_path_buf(),
            filename_override: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn single_file_with_mkv_extension_extracts_filename() {
        let temp = TempDir::new().unwrap();
        let input_file = temp.path().join("input.mp4");
        File::create(&input_file).unwrap();

        let output_path = PathBuf::from("/output/dir/myfile.mkv");
        let result = resolve_output_path(&input_file, &output_path).unwrap();

        assert_eq!(result.output_dir, PathBuf::from("/output/dir"));
        assert_eq!(result.filename_override, Some(OsString::from("myfile.mkv")));
    }

    #[test]
    fn single_file_with_mkv_uppercase_extension_works() {
        let temp = TempDir::new().unwrap();
        let input_file = temp.path().join("input.mp4");
        File::create(&input_file).unwrap();

        let output_path = PathBuf::from("/output/dir/myfile.MKV");
        let result = resolve_output_path(&input_file, &output_path).unwrap();

        assert_eq!(result.output_dir, PathBuf::from("/output/dir"));
        assert_eq!(result.filename_override, Some(OsString::from("myfile.MKV")));
    }

    #[test]
    fn single_file_with_non_mkv_extension_returns_error() {
        let temp = TempDir::new().unwrap();
        let input_file = temp.path().join("input.mp4");
        File::create(&input_file).unwrap();

        let output_path = PathBuf::from("/output/dir/myfile.mp4");
        let result = resolve_output_path(&input_file, &output_path);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains(".mkv extension"));
    }

    #[test]
    fn single_file_with_directory_output_no_override() {
        let temp = TempDir::new().unwrap();
        let input_file = temp.path().join("input.mp4");
        File::create(&input_file).unwrap();

        let output_path = PathBuf::from("/output/dir/");
        let result = resolve_output_path(&input_file, &output_path).unwrap();

        assert_eq!(result.output_dir, PathBuf::from("/output/dir/"));
        assert!(result.filename_override.is_none());
    }

    #[test]
    fn directory_input_treats_output_as_directory() {
        let temp = TempDir::new().unwrap();
        // temp.path() is a directory

        let output_path = PathBuf::from("/output/dir/something.mkv");
        let result = resolve_output_path(temp.path(), &output_path).unwrap();

        // Even though output has .mkv, input is a directory so treat as dir
        assert_eq!(result.output_dir, PathBuf::from("/output/dir/something.mkv"));
        assert!(result.filename_override.is_none());
    }

    #[test]
    fn filename_only_output_uses_current_dir() {
        let temp = TempDir::new().unwrap();
        let input_file = temp.path().join("input.mp4");
        File::create(&input_file).unwrap();

        let output_path = PathBuf::from("output.mkv");
        let result = resolve_output_path(&input_file, &output_path).unwrap();

        assert_eq!(result.output_dir, PathBuf::from("."));
        assert_eq!(result.filename_override, Some(OsString::from("output.mkv")));
    }
}
