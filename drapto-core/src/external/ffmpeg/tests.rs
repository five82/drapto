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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![6], // 5.1 audio
        audio_streams: None,
        duration: 3600.0,
        hqdn3d_params: Some("2:1.5:3:2.5".to_string()), // Example SDR denoising params
        // Actual values used in FFmpeg command
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: crate::config::MIN_FILM_GRAIN_VALUE,
    }
}

/// Create test parameters with crop filter
fn create_test_params_with_crop(crop: Option<&str>) -> EncodeParams {
    let mut params = create_test_params();
    params.crop_filter = crop.map(|c| c.to_string());
    params
}

#[test]
fn test_video_filter_chain_with_sdr_denoising() {
    // Test that SDR denoising parameters are correctly included in video filters
    let params = create_test_params();

    let hqdn3d_to_use = params.hqdn3d_params.as_deref();
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hqdn3d_to_use.unwrap_or(""))
        .add_crop(params.crop_filter.as_deref().unwrap_or(""))
        .build();

    assert!(
        filter_chain.is_some(),
        "Should have video filters when denoising is enabled"
    );
    let filters = filter_chain.unwrap();
    assert!(
        filters.contains("hqdn3d=2:1.5:3:2.5"),
        "Video filters should contain test SDR HQDN3D parameters"
    );
}

#[test]
fn test_video_filter_chain_with_hdr_denoising() {
    // Test that HDR denoising parameters are correctly used
    let hdr_params = Some("1:0.8:2.5:2".to_string()); // Example HDR denoising params

    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise(hdr_params.as_deref().unwrap_or(""))
        .build();

    assert!(
        filter_chain.is_some(),
        "Should have video filters for HDR denoising"
    );
    let filters = filter_chain.unwrap();
    assert!(
        filters.contains("hqdn3d=1:0.8:2.5:2"),
        "Video filters should contain test HDR HQDN3D parameters"
    );
}

#[test]
fn test_svt_av1_params_with_film_grain() {
    // Test that film grain synthesis is correctly applied when denoising is enabled
    let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
        .with_tune(3)
        .with_film_grain(crate::config::MIN_FILM_GRAIN_VALUE)
        .build();

    assert!(
        svtav1_params.contains("tune=3"),
        "SVT-AV1 params should contain tune=3"
    );
    assert!(
        svtav1_params.contains(&format!(
            "film-grain={}",
            crate::config::MIN_FILM_GRAIN_VALUE
        )),
        "SVT-AV1 params should contain film-grain={}",
        crate::config::MIN_FILM_GRAIN_VALUE
    );
    assert!(
        svtav1_params.contains("film-grain-denoise=0"),
        "SVT-AV1 params should disable built-in film grain denoising"
    );
}

#[test]
fn test_svt_av1_params_with_logical_processor_limit() {
    let mut params = create_test_params();
    params.logical_processors = Some(10);

    let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("lp=10"),
        "Command should include lp parameter when logical processors are limited: {}",
        cmd_string
    );
}

#[test]
fn test_svt_av1_params_without_film_grain() {
    // Test that film grain is not applied when denoising is disabled
    let svtav1_params = crate::external::SvtAv1ParamsBuilder::new()
        .with_tune(3)
        .with_film_grain(0) // No film grain
        .build();

    assert!(
        svtav1_params.contains("tune=3"),
        "SVT-AV1 params should contain tune=3"
    );
    assert!(
        !svtav1_params.contains("film-grain="),
        "SVT-AV1 params should not contain film-grain when disabled"
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
fn test_video_filter_chain_with_crop_and_denoise() {
    // Test that both crop and denoise filters are properly combined
    let filter_chain = crate::external::VideoFilterChain::new()
        .add_denoise("2:1.5:3:2.5")
        .add_crop("crop=1920:800:0:140")
        .build();

    assert!(filter_chain.is_some(), "Should have video filters");
    let filters = filter_chain.unwrap();
    assert!(filters.contains("hqdn3d="), "Should contain HQDN3D filter");
    assert!(
        filters.contains("crop=1920:800:0:140"),
        "Should contain crop filter"
    );
    assert!(filters.contains(","), "Filters should be comma-separated");
}

#[test]
fn test_configuration_constants_consistency() {
    // Test that our configuration constants are reasonable values

    // Film grain constants should be reasonable
    assert!(
        crate::config::MIN_FILM_GRAIN_VALUE <= 50,
        "Min film grain should be <= 50"
    );
    assert!(
        crate::config::MIN_FILM_GRAIN_VALUE > 0,
        "Min film grain should be > 0"
    );
    assert!(
        crate::config::MAX_FILM_GRAIN_VALUE <= 50,
        "Max film grain should be <= 50"
    );
    assert!(
        crate::config::MAX_FILM_GRAIN_VALUE > crate::config::MIN_FILM_GRAIN_VALUE,
        "Max should be > min"
    );

    // Resolution thresholds should be reasonable
    assert!(
        crate::config::HD_WIDTH_THRESHOLD < crate::config::UHD_WIDTH_THRESHOLD,
        "HD threshold should be < UHD threshold"
    );
    assert!(
        crate::config::HD_WIDTH_THRESHOLD >= 1920,
        "HD threshold should be >= 1920"
    );
    assert!(
        crate::config::UHD_WIDTH_THRESHOLD >= 3840,
        "UHD threshold should be >= 3840"
    );
}

#[test]
fn test_build_ffmpeg_command_with_crop_filter() {
    // Test that crop filters are correctly included in the full FFmpeg command
    let params = create_test_params_with_crop(Some("crop=1920:800:0:140"));
    let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("-vf"),
        "Command should contain video filter argument"
    );
    assert!(
        cmd_string.contains("crop=1920:800:0:140"),
        "Command should contain crop filter"
    );
    assert!(
        cmd_string.contains("hqdn3d="),
        "Command should contain denoise filter when denoising enabled"
    );
}

#[test]
fn test_build_ffmpeg_command_with_crop_and_denoise() {
    // Test that crop and denoise filters are properly chained
    let params = create_test_params_with_crop(Some("crop=1920:1036:0:22"));
    let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Should contain both filters properly chained
    assert!(
        cmd_string.contains("crop=1920:1036:0:22"),
        "Command should contain crop filter"
    );
    assert!(
        cmd_string.contains("hqdn3d="),
        "Command should contain denoise filter"
    );
    assert!(
        cmd_string.contains("hqdn3d=2:1.5:3:2.5,crop=1920:1036:0:22"),
        "Filters should be properly chained in denoise,crop order"
    );
}

#[test]
fn test_build_ffmpeg_command_without_crop_filter() {
    // Test that FFmpeg command works correctly when no crop is needed
    let params = create_test_params_with_crop(None);
    let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        !cmd_string.contains("crop="),
        "Command should not contain crop filter"
    );
    assert!(
        cmd_string.contains("hqdn3d="),
        "Command should still contain denoise filter"
    );
}

#[test]
fn test_build_ffmpeg_command_crop_only_no_denoise() {
    // Test crop filter without denoising
    let params = create_test_params_with_crop(Some("crop=1920:800:0:140"));
    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    assert!(
        cmd_string.contains("-vf"),
        "Command should contain video filter argument"
    );
    assert!(
        cmd_string.contains("crop=1920:800:0:140"),
        "Command should contain crop filter"
    );
    assert!(
        !cmd_string.contains("hqdn3d"),
        "Command should not contain denoise filter when denoising disabled"
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
        enable_variance_boost: crate::config::DEFAULT_SVT_AV1_ENABLE_VARIANCE_BOOST,
        variance_boost_strength: crate::config::DEFAULT_SVT_AV1_VARIANCE_BOOST_STRENGTH,
        variance_octile: crate::config::DEFAULT_SVT_AV1_VARIANCE_OCTILE,
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: Some("crop=1920:1036:0:22".to_string()),
        audio_channels: vec![6], // 5.1 surround
        audio_streams: None,
        duration: 120.0,
        hqdn3d_params: Some("2:1.5:3:2.5".to_string()),
        // Test the actual values that should match FFmpeg command
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 4,
    };

    let cmd = build_ffmpeg_command(&params, None, false, true).unwrap();
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

    // Validate film grain level matches (in svtav1-params)
    assert!(
        cmd_string.contains("-svtav1-params"),
        "Command should contain SVT-AV1 params"
    );
    assert!(
        cmd_string.contains(&format!("film-grain={}", params.film_grain_level)),
        "Command should contain film grain level: {}",
        params.film_grain_level
    );

    // Validate tune parameter matches (in svtav1-params)
    assert!(
        cmd_string.contains(&format!("tune={}", params.tune)),
        "Command should contain tune parameter: {}",
        params.tune
    );

    // Validate hqdn3d params match
    if let Some(ref hqdn3d) = params.hqdn3d_params {
        assert!(
            cmd_string.contains(&format!("hqdn3d={}", hqdn3d)),
            "Command should contain hqdn3d params: {}",
            hqdn3d
        );
    }

    // Validate audio bitrate is calculated correctly for 5.1 (6 channels)
    let expected_bitrate = crate::processing::audio::calculate_audio_bitrate(6); // 256 kbps for 5.1
    assert!(
        cmd_string.contains(&format!("{}k", expected_bitrate)),
        "Command should contain correct audio bitrate: {}k",
        expected_bitrate
    );
}

#[test]
fn test_encode_params_film_grain_disabled() {
    // Test that film grain level 0 results in no film grain in FFmpeg command
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![2], // Stereo
        audio_streams: None,
        duration: 120.0,
        hqdn3d_params: None, // No denoising
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0, // Disabled
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
    let cmd_string = format!("{:?}", cmd);

    // Film grain should not appear in command when disabled (level 0)
    assert!(
        !cmd_string.contains("film-grain="),
        "Command should not contain film-grain parameter when disabled. Command: {}",
        cmd_string
    );

    // Should not contain hqdn3d when denoising is disabled
    assert!(
        !cmd_string.contains("hqdn3d"),
        "Command should not contain hqdn3d when denoising disabled"
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![spatial_stream]),
        duration: 120.0,
        hqdn3d_params: None,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0,
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8, 2],
        audio_streams: Some(vec![spatial_stream, commentary_stream]),
        duration: 120.0,
        hqdn3d_params: None,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0,
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![dtsx_stream]),
        duration: 120.0,
        hqdn3d_params: None,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0,
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![8],
        audio_streams: Some(vec![eac3_joc_stream]),
        duration: 120.0,
        hqdn3d_params: None,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0,
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
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
        use_hw_decode: false,
        logical_processors: None,
        crop_filter: None,
        audio_channels: vec![6], // 5.1 surround
        audio_streams: None,     // No detailed stream info
        duration: 120.0,
        hqdn3d_params: None,
        video_codec: "libsvtav1".to_string(),
        pixel_format: "yuv420p10le".to_string(),
        matrix_coefficients: "bt709".to_string(),
        audio_codec: "libopus".to_string(),
        film_grain_level: 0,
    };

    let cmd = build_ffmpeg_command(&params, None, false, false).unwrap();
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

#[test]
fn test_should_retry_without_hw_decode_detects_vaapi_failure() {
    let stderr = "[VAAPI @ 0x0] No VA display found for device /dev/dri/renderD128.";
    assert!(super::should_retry_without_hw_decode(stderr));
}

#[test]
fn test_should_retry_without_hw_decode_ignores_unrelated_errors() {
    let stderr = "Unknown ffmpeg failure";
    assert!(!super::should_retry_without_hw_decode(stderr));
}

#[test]
fn test_cleanup_partial_output_removes_file_if_present() {
    use std::env;
    use std::time::{SystemTime, UNIX_EPOCH};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards")
        .as_nanos();
    let path = env::temp_dir().join(format!("drapto_cleanup_partial_output_{unique}.tmp"));

    std::fs::write(&path, b"test").expect("Failed to create temp file");
    assert!(path.exists(), "Temp file should exist before cleanup");

    super::cleanup_partial_output(&path);
    assert!(!path.exists(), "Temp file should be removed by cleanup");
}
