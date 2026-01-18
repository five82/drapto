//! Scene change detection helper for drapto.
//!
//! Uses av-scenechange with FFmpeg backend to detect scene boundaries.

use anyhow::{Context, Result};
use av_scenechange::{
    decoder::Decoder,
    detect_scene_changes,
    ffmpeg::FfmpegDecoder,
    DetectionOptions, SceneDetectionSpeed,
};
use clap::Parser;
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

    if args.progress {
        eprintln!(
            "Detecting scene changes in {:?}",
            args.input
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

    // Write output file
    let file = File::create(&args.output)
        .with_context(|| format!("Failed to create output file {:?}", args.output))?;
    let mut writer = BufWriter::new(file);

    for frame in &scene_starts {
        writeln!(writer, "{}", frame)?;
    }

    writer.flush()?;

    if args.progress {
        eprintln!(
            "Wrote {} scene boundaries to {:?}",
            scene_starts.len(),
            args.output
        );
    }

    Ok(())
}
