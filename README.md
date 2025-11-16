# drapto

drapto is a small Rust workspace that wraps FFmpeg/SVT-AV1 with the set of defaults I use when turning Blu-ray remuxes into Plex-friendly AV1 encodes. It is intentionally opinionated: the CLI focuses on a handful of switches so you can queue a directory of movies, walk away, and trust that the encode parameters will be sensible without hand-tuning every title.

## Why this exists

I got tired of re-deriving the same FFmpeg incantations for every rip. The workspace keeps the boring parts on autopilot:

* **Keep Plex happy** – Always output 10-bit AV1 + Opus (unless the stream is spatial audio) so the files land in a format the server can direct-play.
* **Catch surprises early** – Every encode runs through MediaInfo validation before being marked complete; failures kick back to the queue automatically.
* **Set-and-forget defaults** – Resolution-aware CRFs, per-channel Opus bitrates, and the "do I denoise this?" decision tree all come from the same baseline that I use on my own UHD/1080p sources.
* **Stay transparent** – All of the heuristics live in `drapto-core`, so you can read exactly why a title was cropped, denoised, or marked HDR in the log output.

## Features

* **Analysis before encode** – Optional black-bar detection, HDR metadata detection, noise probes that influence filtering, and MediaInfo-based validation after each run.
* **Optimized encoding defaults** – libsvtav1 video, Opus for non-spatial audio streams, automatic bitrate selection by channel count, and per-resolution CRF defaults that track the quality I prefer for 1080p/4K sources.
* **Adaptive denoising (experimental)** – External HQDN3D plus film-grain synthesis that kicks in only when noise probes decide the content benefits from it.
* **Hardware-aware decoding** – VideoToolbox/VAAPI detection so decoding can be offloaded when the platform supports it.
* **Daemon or foreground workflows** – Run it as a queueing daemon, drop into an interactive foreground session with progress bars, and opt into ntfy notifications.

## Installation

1. **Prerequisites**: Install ffmpeg (with libsvtav1 and libopus support) and mediainfo
   ```bash
   # Ubuntu/Debian
   sudo apt install ffmpeg mediainfo

   # macOS
   brew install ffmpeg media-info
   ```

2. **Install Drapto**:
   ```bash
   cargo install --git https://github.com/five82/drapto
   ```

## Usage

```bash
# Basic usage (runs in background by default)
drapto encode -i input.mkv -o output/

# Encode directory (processes all video files)
drapto encode -i /videos/ -o /encoded/

# Foreground mode (with progress bars and terminal output)
drapto encode --foreground -i input.mkv -o output/

# Custom settings
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 4

# With notifications
drapto encode -i input.mkv -o output/ --ntfy https://ntfy.sh/your_topic

# Verbose output
drapto encode -v -i input.mkv -o output/
```

## Key Options

**Required:**
* `-i, --input <PATH>`: Input file or directory containing .mkv files
* `-o, --output <DIR>`: Output directory (or filename if single file)

**Common Options:**
* `--foreground`: Run in foreground instead of daemon mode
* `-v, --verbose`: Enable verbose output with detailed information
* `--no-color`: Disable colored output
* `-l, --log-dir <DIR>`: Directory for log files (defaults to OUTPUT_DIR/logs)
* `--preset <0-13>`: SVT-AV1 encoder speed/quality (default: 4, lower = slower/better)
* `--quality-sd/hd/uhd <CRF>`: Override quality settings (defaults: SD=23, HD=25, UHD=27)
* `--no-denoise`: Disable denoising and film grain synthesis (denoise is enabled by default)
* `--responsive`: Reserve a few CPU threads so other applications stay responsive (disabled by default)
* `--disable-autocrop`: Disable black bar cropping (auto-crop is enabled by default)
* `--ntfy <URL>`: Send notifications to ntfy.sh

## Advanced Features

### Adaptive Denoising (experimental)

The denoiser is intentionally external to SVT-AV1 so its behavior is predictable and easy to reason about. Drapto samples a few clips with FFmpeg's `bitplanenoise`, averages the values per plane, and feeds that into a small decision tree (`drapto-core/src/noise.rs`) that picks an HQDN3D matrix and a matching film-grain value.

What you should know before enabling it:

1. **It costs time**: Each probe is a short FFmpeg run and HQDN3D itself is a 10-bit filter. Expect ~15‑25 % slower encodes on UHD sources. Pass `--no-denoise` if throughput matters more than squeezing a few extra percent of bitrate. (The CLI flag simply flips `Config::enable_denoise` to false, skipping the probes and filters entirely.)
2. **It is conservative by design**: The four presets range from "barely touch it" to "moderate". HDR titles automatically use lighter matrices so highlights keep their texture.
3. **Film grain is tied to the denoiser**: The amount of synthetic grain sent to SVT-AV1 is derived from the spatial luma term of HQDN3D. If denoise is off, no grain synthesis is added either.

I consider the feature useful for catalog titles with visible grain, but experimental enough that it is disabled by default on my own queue. Keep it if you like the look; skip it if you just want speed.

| Noise band (avg bitplane) | SDR HQDN3D matrix | HDR HQDN3D matrix | Typical grain level |
| --- | --- | --- | --- |
| `< 0.6` (clean) | `1:0.8:2:2` | `0.5:0.4:1.5:1.5` | 4–6 |
| `0.6 – <0.7` (low) | `2:1.5:3:2.5` | `1:0.8:2.5:2` | 6–9 |
| `0.7 – <0.8` (moderate) | `3:2.5:4:3.5` | `2:1.5:3.5:3` | 9–13 |
| `≥ 0.8` (noticeable) | `4:3.5:5:4.5` | `3:2.5:4.5:4` | 13–16 |

Those numbers come straight from [`drapto-core/src/processing/noise_analysis.rs`](drapto-core/src/processing/noise_analysis.rs). When the log prints something like `Noise analysis: avg=0.73`, you can map it to the table to see exactly why HQDN3D/film grain were applied. The guardrail is still manual: I usually leave `--no-denoise` on for animation and clean CGI, then re-run with denoise enabled only on titles where the probe average lands above ~0.65.

### HDR Support

Automatically detects and preserves HDR content using MediaInfo for comprehensive color space analysis:
- Detects HDR based on color primaries (BT.2020, BT.2100)
- Recognizes HDR transfer characteristics (PQ, HLG)
- Adapts processing parameters for HDR content

### Post-Encode Validation

Includes comprehensive validation to ensure encoding success:

* **Video Codec**: Verifies AV1 encoding and 10-bit depth
* **Audio Codec**: Validates audio processing based on stream type:
  * Spatial audio streams: Confirms preservation of original codecs (TrueHD/DTS)
  * Non-spatial streams: Confirms Opus transcoding
  * Mixed content: Validates each stream according to its processing method
* **Dimensions**: Validates crop detection and output dimensions
* **Duration**: Ensures encoded duration matches input
* **HDR/Color Space**: Uses MediaInfo to verify HDR content preservation and color space accuracy
* **Failure Reporting**: Logs and notifies about validation issues

### Hardware Acceleration

* VideoToolbox and VAAPI hardware decoding support (automatically enabled when available)
* Improves performance by hardware-accelerating video decoding

### Multi-Stream Audio

* Automatic detection of all audio streams with intelligent spatial audio preservation
* **Spatial Audio Support**: Automatically detects and preserves spatial audio formats:
  * **Dolby Atmos**: TrueHD + Atmos and E-AC-3 + JOC (Joint Object Coding)
  * **DTS:X**: DTS with DTS:X profile variations
  * Spatial audio streams are copied unchanged to preserve object-based metadata
* **Mixed Audio Processing**: Handles files with both spatial and non-spatial audio streams
  * Spatial streams: Copied to preserve quality and metadata
  * Non-spatial streams: Transcoded to Opus with optimized bitrates
* Channel-based bitrate allocation for Opus transcoding:
  * Mono: 64 kbps
  * Stereo: 128 kbps
  * 5.1: 256 kbps
  * 7.1: 384 kbps
  * Custom: 48 kbps per channel

### Progress Reporting

In foreground mode:
* Real-time encoding progress with ETA
* Current encoding speed (fps)
* Bitrate monitoring
* File size reduction calculations

### Environment Variables

* `DRAPTO_NTFY_TOPIC`: Default ntfy.sh topic URL
* `NO_COLOR`: Disable colored output
* `RUST_LOG`: Control logging level (e.g., `debug`, `trace`)

### Notifications

Push notifications via ntfy.sh include:
* Encode start notification
* Encode completion with file sizes and duration
* Error notifications
* Size reduction percentage

## Building from Source

```bash
git clone https://github.com/five82/drapto.git
cd drapto
cargo build --release
./target/release/drapto --help
```

## Architecture

Drapto is built as a Rust workspace with two main components:

* **drapto-cli**: Command-line interface
  * Argument parsing and validation
  * Daemon/interactive mode handling
  * Progress reporting and user feedback
  * Terminal color support

* **drapto-core**: Core video processing library
  * Video analysis (crop detection, video properties)
  * FFmpeg/FFprobe and MediaInfo integration
  * Audio stream processing and validation
  * Post-encode validation system
  * Notification services (ntfy.sh)
  * Hardware acceleration detection
  * Event-based progress reporting
  * System information collection
  * Temporary file management

## Debugging

Enable detailed logging for troubleshooting:

```bash
# Debug level
RUST_LOG=debug drapto encode -i input.mkv -o output/

# Trace level (very detailed)
RUST_LOG=trace drapto encode --interactive -i input.mkv -o output/
```

For all available options and examples, use `drapto encode --help`.
