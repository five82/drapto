use super::*;
use std::path::PathBuf;

/// Create test parameters with common defaults
fn create_test_params() -> EncodeParams {
    EncodeParams {
        input_path: PathBuf::from("/test/input.mkv"),
        output_path: PathBuf::from("/test/output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![6], // 5.1 audio
        audio_streams: None,
        duration: 3600.0,
        // Actual values used in FFmpeg command
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    }
}

/// Create test parameters with crop filter
fn create_test_params_with_crop(crop: Option<&str>) -> EncodeParams {
    let mut params = create_test_params();
    params.crop_filter = crop.map(|c| c.to_string());
    params
}

#[test]
fn test_svt_av1_params_with_logical_processor_limit() {
    let mut params = create_test_params();
    params.logical_processors = Some(10);

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("lp=10"),
        "Command should include lp parameter when logical processors are limited: {}",
        cmd_string
    );
}

#[test]
fn test_audio_bitrate_calculation() {
    // Test that audio bitrates match what's displayed in configuration
    use crate::processing::audio::calculate_audio_bitrate;

    // Test common channel configurations
    assert_eq!(calculate_audio_bitrate(1), 64, "Mono should be 64kbps");
    assert_eq!(calculate_audio_bitrate(2), 128, "Stereo should be 128kbps");
    assert_eq!(calculate_audio_bitrate(6), 256, "5.1 should be 256kbps");
    assert_eq!(calculate_audio_bitrate(8), 384, "7.1 should be 384kbps");
}

#[test]
fn test_uhd_width_threshold_constant() {
    assert!(
        crate::config::UHD_WIDTH_THRESHOLD >= 3840,
        "UHD threshold should be >= 3840"
    );
}

#[test]
fn test_build_ffmpeg_command_with_crop_filter() {
    // Test that crop filters are correctly included in the full FFmpeg command
    let params = create_test_params_with_crop(Some("crop=1920:800:0:140"));
    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("-vf"),
        "Command should contain video filter argument"
    );
    assert!(
        cmd_string.contains("crop=1920:800:0:140"),
        "Command should contain crop filter"
    );
}

#[test]
fn test_build_ffmpeg_command_without_crop_filter() {
    // Test that FFmpeg command works correctly when no crop is needed
    let params = create_test_params_with_crop(None);
    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        !cmd_string.contains("crop="),
        "Command should not contain crop filter"
    );
}

#[test]
fn test_build_ffmpeg_command_with_denoise_filter() {
    let mut params = create_test_params();
    params.video_denoise_filter = Some("hqdn3d=1.5:1.5:3:3".to_string());

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("-vf"),
        "Command should contain video filter argument"
    );
    assert!(
        cmd_string.contains("hqdn3d=1.5:1.5:3:3"),
        "Command should contain hqdn3d denoise filter"
    );
}

#[test]
fn test_svt_av1_params_include_film_grain_when_set() {
    let mut params = create_test_params();
    params.film_grain = Some(6);
    params.film_grain_denoise = Some(false);

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("film-grain=6"),
        "Command should include film-grain when configured on"
    );
    assert!(
        cmd_string.contains("film-grain-denoise=0"),
        "Command should include film-grain-denoise=0 when configured off"
    );
}

#[test]
fn test_encode_params_match_ffmpeg_command() {
    // Test that EncodeParams fields accurately reflect what's used in the FFmpeg command
    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: true,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: Some(10),
        crop_filter: Some("crop=1920:1036:0:22".to_string()),
        audio_channels: vec![6], // 5.1 surround
        audio_streams: None,
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Validate video codec matches
    assert!(
        cmd_string.contains("-c:v"),
        "Command should contain video codec flag"
    );
    assert!(
        cmd_string.contains(&params.video_codec),
        "Command should contain video codec: {}",
        params.video_codec
    );

    // Validate pixel format matches
    assert!(
        cmd_string.contains("-pix_fmt"),
        "Command should contain pixel format flag"
    );
    assert!(
        cmd_string.contains(&params.pixel_format),
        "Command should contain pixel format: {}",
        params.pixel_format
    );

    // Validate audio codec matches
    assert!(
        cmd_string.contains("-c:a"),
        "Command should contain audio codec flag"
    );
    assert!(
        cmd_string.contains(&params.audio_codec),
        "Command should contain audio codec: {}",
        params.audio_codec
    );

    // Validate preset matches
    assert!(
        cmd_string.contains("-preset"),
        "Command should contain preset flag"
    );
    assert!(
        cmd_string.contains(&params.preset.to_string()),
        "Command should contain preset: {}",
        params.preset
    );

    // Validate quality (CRF) matches
    assert!(
        cmd_string.contains("-crf"),
        "Command should contain CRF flag"
    );
    assert!(
        cmd_string.contains(&params.quality.to_string()),
        "Command should contain quality: {}",
        params.quality
    );

    // Validate tune parameter matches (in svtav1-params)
    assert!(
        cmd_string.contains(&format!("tune={}", params.tune)),
        "Command should contain tune parameter: {}",
        params.tune
    );

    // Variance boost enabled should include strength/octile and thread limit
    assert!(
        cmd_string.contains("enable-variance-boost=1"),
        "Command should enable variance boost when configured on"
    );
    assert!(
        cmd_string.contains(&format!(
            "variance-boost-strength={}",
            params.variance_boost_strength
        )),
        "Variance boost strength should be emitted when feature is enabled"
    );
    assert!(
        cmd_string.contains(&format!("variance-octile={}", params.variance_octile)),
        "Variance octile should be emitted when feature is enabled"
    );
    assert!(
        cmd_string.contains("lp=10"),
        "Command should include logical processor limit"
    );

    // Validate audio bitrate is calculated correctly for 5.1 (6 channels)
    let expected_bitrate = crate::processing::audio::calculate_audio_bitrate(6); // 256 kbps for 5.1
    assert!(
        cmd_string.contains(&format!("{}k", expected_bitrate)),
        "Command should contain correct audio bitrate: {}k",
        expected_bitrate
    );
}

#[test]
fn test_audio_command_generation_transcodes_all() {
    use crate::external::AudioStreamInfo;

    // Test single audio track (previously spatial)
    let spatial_stream = AudioStreamInfo {
        channels: 8,
        codec_name: "truehd".to_string(),
        profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
        index: 0,
        is_spatial: false,
    };

    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![spatial_stream]),
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Should transcode to Opus with bitrate
    assert!(cmd_string.contains("-c:a:0") && cmd_string.contains("libopus"));
    assert!(cmd_string.contains("-b:a:0") && cmd_string.contains("384k"));
}

#[test]
fn test_multiple_audio_streams_all_transcoded() {
    use crate::external::AudioStreamInfo;

    // Mixed scenario: primary + commentary track (both transcoded)
    let spatial_stream = AudioStreamInfo {
        channels: 8,
        codec_name: "truehd".to_string(),
        profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
        index: 0,
        is_spatial: false,
    };

    let commentary_stream = AudioStreamInfo {
        channels: 2,
        codec_name: "aac".to_string(),
        profile: Some("LC".to_string()),
        index: 1,
        is_spatial: false,
    };

    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8, 2],
        audio_streams: Some(vec![spatial_stream, commentary_stream]),
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Both streams should be transcoded to Opus with bitrates
    assert!(cmd_string.contains("-c:a:0") && cmd_string.contains("libopus"));
    assert!(cmd_string.contains("-c:a:1") && cmd_string.contains("libopus"));
    assert!(cmd_string.contains("-b:a:0") && cmd_string.contains("384k"));
    assert!(cmd_string.contains("-b:a:1") && cmd_string.contains("128k"));
}

#[test]
fn test_dtsx_audio_command_generation_transcoded() {
    use crate::external::AudioStreamInfo;

    // Test DTS:X audio track (now transcoded)
    let dtsx_stream = AudioStreamInfo {
        channels: 8,
        codec_name: "dts".to_string(),
        profile: Some("DTS:X".to_string()),
        index: 0,
        is_spatial: false,
    };

    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![dtsx_stream]),
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Should transcode to Opus with appropriate bitrate
    assert!(cmd_string.contains("-c:a:0") && cmd_string.contains("libopus"));
    assert!(cmd_string.contains("-b:a:0") && cmd_string.contains("384k"));
}

#[test]
fn test_eac3_joc_audio_command_generation() {
    use crate::external::AudioStreamInfo;

    // Test E-AC-3 with JOC (Atmos) audio track (transcoded)
    let eac3_joc_stream = AudioStreamInfo {
        channels: 8,
        codec_name: "eac3".to_string(),
        profile: Some("Dolby Digital Plus + JOC".to_string()),
        index: 0,
        is_spatial: false,
    };

    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![eac3_joc_stream]),
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Should transcode to Opus with bitrate
    assert!(cmd_string.contains("-c:a:0") && cmd_string.contains("libopus"));
    assert!(cmd_string.contains("-b:a:0") && cmd_string.contains("384k"));
}

#[test]
fn test_fallback_behavior_without_detailed_streams() {
    // Test fallback when no detailed audio stream info is available
    let params = EncodeParams {
        input_path: std::path::PathBuf::from("test_input.mkv"),
        output_path: std::path::PathBuf::from("test_output.mkv"),
        quality: 27,
        preset: crate::config::DEFAULT_SVT_AV1_PRESET,
        tune: crate::config::DEFAULT_SVT_AV1_TUNE,
        ac_bias: crate::config::DEFAULT_SVT_AV1_AC_BIAS,
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        video_denoise_filter: None,
        film_grain: None,
        film_grain_denoise: None,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![6], // 5.1 surround
        audio_streams: None,     // No detailed stream info
        duration: 120.0,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
    };

    let cmd = build_ffmpeg_command(&params, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Should fall back to traditional behavior
    assert!(
        cmd_string.contains("-c:a") && cmd_string.contains("libopus"),
        "Command should use opus codec in fallback mode: {}",
        cmd_string
    );

    assert!(
        cmd_string.contains("-b:a:0") && cmd_string.contains("256k"),
        "Command should contain 5.1 bitrate in fallback mode: {}",
        cmd_string
    );

    assert!(
        cmd_string.contains("-map") && cmd_string.contains("0:a"),
        "Command should map all audio streams in fallback mode: {}",
        cmd_string
    );
}
