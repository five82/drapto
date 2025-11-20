//! Shared formatting helpers for human-readable display strings.
//!
//! Keeping these helpers in one place avoids duplication across stages.

use crate::processing::audio;

/// Generate audio results description for encoding complete summary
pub fn generate_audio_results_description(
    audio_channels: &[u32],
    audio_streams: Option<&[crate::external::AudioStreamInfo]>,
) -> String {
    if audio_channels.is_empty() {
        return "No audio".to_string();
    }

    // If we don't have detailed stream info, fall back to basic Opus description
    let Some(streams) = audio_streams else {
        return generate_basic_audio_description(audio_channels);
    };

    if streams.len() == 1 {
        generate_single_stream_description(&streams[0])
    } else {
        generate_multi_stream_description(streams)
    }
}

/// Generate basic audio description assuming Opus (fallback)
fn generate_basic_audio_description(audio_channels: &[u32]) -> String {
    if audio_channels.len() == 1 {
        let channel_desc = channel_label(audio_channels[0]);
        format!(
            "{}, Opus, {} kb/s",
            channel_desc,
            audio::calculate_audio_bitrate(audio_channels[0])
        )
    } else {
        let track_descriptions: Vec<String> = audio_channels
            .iter()
            .enumerate()
            .map(|(i, &channels)| {
                let bitrate = audio::calculate_audio_bitrate(channels);
                let desc = channel_label(channels);
                format!("Track {}: {}, Opus, {} kb/s", i + 1, desc, bitrate)
            })
            .collect();
        track_descriptions.join("\n                     ")
    }
}

/// Generate description for a single audio stream
fn generate_single_stream_description(stream: &crate::external::AudioStreamInfo) -> String {
    let channel_desc = channel_label(stream.channels);
    let bitrate = audio::calculate_audio_bitrate(stream.channels);
    format!("{}, Opus, {} kb/s", channel_desc, bitrate)
}

/// Generate description for multiple audio streams
fn generate_multi_stream_description(streams: &[crate::external::AudioStreamInfo]) -> String {
    let track_descriptions: Vec<String> = streams
        .iter()
        .enumerate()
        .map(|(i, stream)| {
            let channel_desc = channel_label(stream.channels);
            let bitrate = audio::calculate_audio_bitrate(stream.channels);
            format!("Track {}: {}, Opus, {} kb/s", i + 1, channel_desc, bitrate)
        })
        .collect();
    track_descriptions.join("\n                     ")
}

/// Simple helper for channel count labels.
fn channel_label(channels: u32) -> String {
    match channels {
        1 => "Mono".to_string(),
        2 => "Stereo".to_string(),
        6 => "5.1 surround".to_string(),
        8 => "7.1 surround".to_string(),
        n => format!("{} channels", n),
    }
}

/// Basic audio description for initialization/logging (no bitrate)
pub fn format_audio_description_basic(audio_channels: &[u32]) -> String {
    if audio_channels.is_empty() {
        return "No audio".to_string();
    }

    if audio_channels.len() == 1 {
        channel_label(audio_channels[0])
    } else {
        let track_descriptions: Vec<String> = audio_channels
            .iter()
            .enumerate()
            .map(|(i, &channels)| format!("Track {}: {}", i + 1, channel_label(channels)))
            .collect();
        track_descriptions.join("\n                     ")
    }
}

/// Audio description for config display including bitrate and per-track detail.
pub fn format_audio_description_config(
    audio_channels: &[u32],
    audio_streams: Option<&[crate::external::AudioStreamInfo]>,
) -> String {
    if let Some(streams) = audio_streams {
        if streams.is_empty() {
            return "No audio".to_string();
        } else if streams.len() == 1 {
            let stream = &streams[0];
            let channel_desc = channel_label(stream.channels);
            let bitrate = audio::calculate_audio_bitrate(stream.channels);
            return format!("{channel_desc} @ {bitrate}kbps Opus");
        } else {
            let track_descriptions: Vec<String> = streams
                .iter()
                .map(|stream| {
                    let desc = channel_label(stream.channels);
                    let bitrate = audio::calculate_audio_bitrate(stream.channels);
                    format!(
                        "Track {}: {} @ {}kbps Opus",
                        stream.index + 1,
                        desc,
                        bitrate
                    )
                })
                .collect();
            return track_descriptions.join("\n                     ");
        }
    }

    // Fallback to simple channel info if detailed analysis failed
    format_audio_description_without_streams(audio_channels)
}

fn format_audio_description_without_streams(audio_channels: &[u32]) -> String {
    if audio_channels.is_empty() {
        "No audio".to_string()
    } else if audio_channels.len() == 1 {
        let bitrate = audio::calculate_audio_bitrate(audio_channels[0]);
        let channel_desc = channel_label(audio_channels[0]);
        format!("{channel_desc} @ {bitrate}kbps")
    } else {
        let track_descriptions: Vec<String> = audio_channels
            .iter()
            .enumerate()
            .map(|(i, &channels)| {
                let bitrate = audio::calculate_audio_bitrate(channels);
                let desc = channel_label(channels);
                format!("Track {}: {} @ {}kbps", i + 1, desc, bitrate)
            })
            .collect();
        track_descriptions.join("\n                     ")
    }
}

#[cfg(test)]
mod tests {
    use super::generate_audio_results_description;
    use crate::external::AudioStreamInfo;

    #[test]
    fn test_single_spatial_audio_description() {
        let streams = vec![AudioStreamInfo {
            channels: 8,
            codec_name: "truehd".to_string(),
            profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
            index: 0,
            is_spatial: false,
        }];
        let audio_channels = vec![8];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        assert_eq!(result, "7.1 surround, Opus, 384 kb/s");
    }

    #[test]
    fn test_single_non_spatial_audio_description() {
        let streams = vec![AudioStreamInfo {
            channels: 2,
            codec_name: "aac".to_string(),
            profile: Some("LC".to_string()),
            index: 0,
            is_spatial: false,
        }];
        let audio_channels = vec![2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        assert_eq!(result, "Stereo, Opus, 128 kb/s");
    }

    #[test]
    fn test_multiple_audio_tracks_mixed() {
        let streams = vec![
            AudioStreamInfo {
                channels: 8,
                codec_name: "truehd".to_string(),
                profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 1,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 2,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![8, 2, 2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 7.1 surround, Opus, 384 kb/s\n                     Track 2: Stereo, Opus, 128 kb/s\n                     Track 3: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_spatial_audio_tracks() {
        let streams = vec![
            AudioStreamInfo {
                channels: 8,
                codec_name: "truehd".to_string(),
                profile: Some("Dolby TrueHD + Dolby Atmos".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 8,
                codec_name: "dts".to_string(),
                profile: Some("DTS:X".to_string()),
                index: 1,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![8, 8];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 7.1 surround, Opus, 384 kb/s\n                     Track 2: 7.1 surround, Opus, 384 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_multiple_non_spatial_audio_tracks() {
        let streams = vec![
            AudioStreamInfo {
                channels: 6,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 2,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 1,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![6, 2];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: 5.1 surround, Opus, 256 kb/s\n                     Track 2: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_fallback_without_stream_info() {
        let audio_channels = vec![8, 2];

        let result = generate_audio_results_description(&audio_channels, None);

        let expected = "Track 1: 7.1 surround, Opus, 384 kb/s\n                     Track 2: Stereo, Opus, 128 kb/s";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_no_audio_tracks() {
        let audio_channels = vec![];

        let result = generate_audio_results_description(&audio_channels, Some(&[]));

        assert_eq!(result, "No audio");
    }

    #[test]
    fn test_uncommon_channel_configurations() {
        let streams = vec![
            AudioStreamInfo {
                channels: 1,
                codec_name: "aac".to_string(),
                profile: Some("LC".to_string()),
                index: 0,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 4,
                codec_name: "ac3".to_string(),
                profile: Some("Dolby Digital".to_string()),
                index: 1,
                is_spatial: false,
            },
            AudioStreamInfo {
                channels: 10,
                codec_name: "dtshd".to_string(),
                profile: Some("DTS-HD Master Audio".to_string()),
                index: 2,
                is_spatial: false,
            },
        ];
        let audio_channels = vec![1, 4, 10];

        let result = generate_audio_results_description(&audio_channels, Some(&streams));

        let expected = "Track 1: Mono, Opus, 64 kb/s\n                     Track 2: 4 channels, Opus, 192 kb/s\n                     Track 3: 10 channels, Opus, 480 kb/s";
        assert_eq!(result, expected);
    }
}
