//! Scene change detection helper for drapto.
//!
//! Uses av-scenechange with FFmpeg backend to detect scene boundaries.
//! Long scenes are split at regular intervals.

use anyhow::{Context, Result};
use av_scenechange::{
    decoder::Decoder,
    detect_scene_changes,
    ffmpeg::FfmpegDecoder,
    DetectionOptions, SceneDetectionSpeed,
};
use clap::Parser;
use std::cmp::min;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "drapto-scd")]
#[command(about = "Scene change detection for drapto chunked encoding")]
struct Args {
    /// Input video file
    #[arg(short, long)]
    input: PathBuf,

    /// Output scene file (one frame number per line)
    #[arg(short, long)]
    output: PathBuf,

    /// FPS numerator
    #[arg(long)]
    fps_num: u32,

    /// FPS denominator
    #[arg(long)]
    fps_den: u32,

    /// Total number of frames in the video
    #[arg(long)]
    total_frames: usize,

    /// Show progress output
    #[arg(long, default_value_t = false)]
    progress: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Calculate effective FPS for max scene length calculation
    let fps = args.fps_num as f64 / args.fps_den as f64;

    // Max scene length: 30 seconds or 1000 frames, whichever is smaller
    let max_scene_frames = min((fps * 30.0).ceil() as usize, 1000);

    if args.progress {
        eprintln!(
            "Detecting scene changes in {:?} (max {} frames/scene)",
            args.input, max_scene_frames
        );
    }

    // Create FFmpeg decoder for scene detection
    let ffmpeg_dec = FfmpegDecoder::new(&args.input)
        .context("Failed to create FFmpeg decoder")?;
    let mut decoder: Decoder<std::io::Empty> = Decoder::Ffmpeg(ffmpeg_dec);

    // Configure scene detection
    let opts = DetectionOptions {
        analysis_speed: SceneDetectionSpeed::Standard,
        detect_flashes: true,
        lookahead_distance: 5,
        ..Default::default()
    };

    // Progress callback - use args.total_frames since callback's total is unreliable
    let known_total = args.total_frames;
    let progress_fn = |current: usize, _total: usize| {
        if known_total > 0 && current % 100 == 0 {
            let pct = (current as f64 / known_total as f64) * 100.0;
            // Clamp to 100% in case of frame count mismatch
            let pct = if pct > 100.0 { 100.0 } else { pct };
            eprint!("\rAnalyzing: {:.1}%", pct);
        }
    };

    let progress_callback: Option<&dyn Fn(usize, usize)> = if args.progress {
        Some(&progress_fn)
    } else {
        None
    };

    // Run scene detection with appropriate pixel type based on bit depth
    let bit_depth = decoder.get_video_details()
        .context("Failed to get video details")?
        .bit_depth;

    let results: av_scenechange::DetectionResults = if bit_depth > 8 {
        detect_scene_changes::<std::io::Empty, u16>(&mut decoder, opts, None, progress_callback)
            .context("Scene detection failed")?
    } else {
        detect_scene_changes::<std::io::Empty, u8>(&mut decoder, opts, None, progress_callback)
            .context("Scene detection failed")?
    };

    if args.progress {
        eprintln!(
            "\rScene detection complete, found {} scenes",
            results.scene_changes.len()
        );
    }

    // Extract scene boundaries
    let mut scene_starts: Vec<usize> = results.scene_changes;

    // Ensure we always have frame 0 as first scene start
    if scene_starts.is_empty() || scene_starts[0] != 0 {
        scene_starts.insert(0, 0);
    }

    // Use total_frames from args (more reliable than frame_count for some formats)
    let total_frames = if args.total_frames > 0 {
        args.total_frames
    } else {
        results.frame_count
    };

    // Split long scenes at regular intervals
    let final_scenes = split_long_scenes(&scene_starts, total_frames, max_scene_frames);

    // Write output file
    let file = File::create(&args.output)
        .with_context(|| format!("Failed to create output file {:?}", args.output))?;
    let mut writer = BufWriter::new(file);

    for frame in &final_scenes {
        writeln!(writer, "{}", frame)?;
    }

    writer.flush()?;

    if args.progress {
        eprintln!(
            "Wrote {} scene boundaries to {:?}",
            final_scenes.len(),
            args.output
        );
    }

    Ok(())
}

/// Split long scenes into smaller chunks at regular intervals.
///
/// When a scene is longer than max_frames, we split it evenly to create
/// chunks that are as close to equal length as possible while staying
/// under the max_frames limit.
fn split_long_scenes(
    scene_starts: &[usize],
    total_frames: usize,
    max_frames: usize,
) -> Vec<usize> {
    let mut result = Vec::new();

    // Build scene ranges
    for i in 0..scene_starts.len() {
        let start = scene_starts[i];
        let end = if i + 1 < scene_starts.len() {
            scene_starts[i + 1]
        } else {
            total_frames
        };

        result.push(start);

        let scene_len = end.saturating_sub(start);
        if scene_len > max_frames {
            // Calculate how many chunks we need
            let num_chunks = (scene_len + max_frames - 1) / max_frames;
            let chunk_size = scene_len / num_chunks;

            // Add intermediate split points
            for j in 1..num_chunks {
                let split = start + j * chunk_size;
                if split < end {
                    result.push(split);
                }
            }
        }
    }

    // Sort and deduplicate
    result.sort();
    result.dedup();
    result
}
