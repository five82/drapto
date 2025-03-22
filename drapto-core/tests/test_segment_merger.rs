use std::path::Path;
use std::ffi::OsString;

use drapto_core::encoding::merger::{self, SegmentMerger, MergeOptions};

#[test]
fn test_build_concat_command() {
    let concat_file = Path::new("/tmp/test/concat.txt");
    let output_file = Path::new("/tmp/test/output.mkv");
    
    let cmd = merger::build_concat_command(concat_file, output_file);
    let args: Vec<_> = cmd.get_args().collect();
    
    // Check the command includes these arguments
    let f_arg = OsString::from("-f");
    let concat_arg = OsString::from("concat");
    let c_arg = OsString::from("-c");
    let copy_arg = OsString::from("copy");
    let y_arg = OsString::from("-y");
    
    assert!(args.contains(&f_arg));
    assert!(args.contains(&concat_arg));
    assert!(args.contains(&c_arg));
    assert!(args.contains(&copy_arg));
    assert!(args.contains(&y_arg));
    
    // Check that we have all the expected arguments
    let expected_args = [
        "-hide_banner", "-loglevel", "warning",
        "-f", "concat", "-safe", "0", 
        "-c", "copy",
        "-movflags", "+faststart",
        "-fflags", "+genpts",
        "-map_metadata", "0",
        "-y"
    ];
    
    for arg in expected_args {
        let os_arg = OsString::from(arg);
        assert!(args.contains(&os_arg), "Missing argument: {}", arg);
    }
}

#[test]
fn test_merger_options() {
    // Test default options
    let default_options = MergeOptions::default();
    assert!(default_options.copy_streams);
    assert!(default_options.faststart);
    assert!(default_options.generate_pts);
    assert!(default_options.copy_metadata);
    assert_eq!(default_options.expected_codec, Some("av1".to_string()));
    assert_eq!(default_options.duration_tolerance, 1.0);
    assert_eq!(default_options.start_time_tolerance, 0.2);
    
    // Test custom options
    let custom_options = MergeOptions {
        copy_streams: false,
        faststart: false,
        expected_codec: Some("h264".to_string()),
        duration_tolerance: 2.0,
        ..Default::default()
    };
    
    let merger = SegmentMerger::with_options(custom_options);
    assert!(!merger.options.copy_streams);
    assert!(!merger.options.faststart);
    assert_eq!(merger.options.expected_codec, Some("h264".to_string()));
    assert!(merger.options.generate_pts);
}