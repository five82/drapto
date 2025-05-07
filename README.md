# Drapto

Advanced ffmpeg video encoding wrapper with intelligent optimization.

Drapto is a command-line tool that automates video encoding tasks using ffmpeg (with libsvtav1 and libopus). It simplifies the encoding process by providing intelligent analysis, adaptive optimization, and sensible defaults to produce high-quality, efficient video encodes.

## Features

* **Intelligent Video Analysis**
  * Automatic black bar detection and cropping
  * Advanced film grain/noise analysis with adaptive denoising
  * HDR-aware processing with black level analysis
  * Multi-sample analysis for consistent results

* **Optimized Encoding**
  * High-quality AV1 video encoding using libsvtav1
  * Opus audio encoding with bitrate optimization
  * Resolution-based quality settings
  * Configurable encoding presets

* **Convenient Workflow**
  * Daemon mode for background processing
  * Interactive mode for real-time feedback
  * Detailed logging and progress reporting
  * Push notifications via ntfy.sh

## Architecture

Drapto is built with a modular architecture:

* **drapto-cli**: Command-line interface and user interaction
* **drapto-core**: Core video processing and analysis library

## Installation

1. **Install ffmpeg & ffprobe:** Ensure you have `ffmpeg` (built with `--enable-libsvtav1` and `--enable-libopus`) and `ffprobe` installed and available in your system's PATH.
   ```bash
   # Ubuntu/Debian
   sudo apt install ffmpeg

   # macOS with Homebrew
   brew install ffmpeg
   ```

   Alternatively, download from the [official FFmpeg website](https://ffmpeg.org/download.html).

2. **Install Rust:** If you don't have Rust installed, follow the instructions at [rustup.rs](https://rustup.rs/).

3. **Install Drapto:** Install directly from the Git repository using `cargo install`.
   ```bash
   cargo install --git https://github.com/five82/drapto
   ```

   This command clones the repository, builds the `drapto` binary, and installs it to `~/.cargo/bin/`.

   **Important:** Ensure `~/.cargo/bin` is included in your system's PATH environment variable so you can run `drapto` from anywhere.

## Usage

Basic usage involves specifying an input file/directory and an output directory. By default, Drapto runs in **daemon mode**, meaning it will start the encoding process in the background and detach from the terminal, allowing you to log out while it continues running.

```bash
# Encode a single file in the background (default daemon mode)
drapto encode -i /path/to/input/video.mkv -o /path/to/output/

# Encode all videos in a directory in the background
drapto encode -i /path/to/input_directory/ -o /path/to/output_directory/

# Encode a single file interactively (in the foreground)
drapto encode --interactive -i /path/to/input/video.mkv -o /path/to/output/

# Encode with custom quality settings
drapto encode -i input.mkv -o output/ --quality-hd 24 --preset 6

# Encode without denoising
drapto encode -i input.mkv -o output/ --no-denoise

# Encode with custom grain analysis settings
drapto encode -i input.mkv -o output/ --grain-knee-threshold 0.7 --grain-max-level Visible

# Encode and send notifications to an ntfy.sh topic
drapto encode -i video.mkv -o output/ --ntfy https://ntfy.sh/your_topic
```

### Logging

When running in daemon mode, log files are created in the specified log directory (or `output_dir/logs` by default). A PID file (`drapto.pid`) is also created in the log directory to track the running process.

To run Drapto in the foreground with real-time logging, use the `--interactive` flag.

### Notifications

Drapto can send notifications about encoding progress (start, success, error) to an [ntfy.sh](https://ntfy.sh/) topic URL. The notification message will include the hostname where the encode job is running.

* Use the `--ntfy <topic_url>` argument to specify the topic URL.
* Alternatively, set the `DRAPTO_NTFY_TOPIC` environment variable.
* If both are set, the command-line argument takes precedence.

## Command-Line Options

### Global Options

* `--interactive`: Run in the foreground instead of the background (daemon mode).
* `--help`: Display help information.
* `--version`: Display version information.

### Encode Command Options

* `-i, --input <INPUT_PATH>`: Input file or directory containing video files (required).
* `-o, --output <OUTPUT_DIR>`: Directory where encoded files will be saved (required).
* `-l, --log-dir <LOG_DIR>`: Directory for log files (defaults to OUTPUT_DIR/logs).
* `--disable-autocrop`: Disable automatic black bar detection and cropping.
* `--no-denoise`: Disable video denoising (hqdn3d filter).
* `--preset <0-13>`: Override the SVT-AV1 encoder preset (default: 6, lower is slower but better quality).
* `--quality-sd <CRF>`: Override CRF quality for SD videos (default: 25, <1920 width).
* `--quality-hd <CRF>`: Override CRF quality for HD videos (default: 27, ≥1920 width).
* `--quality-uhd <CRF>`: Override CRF quality for UHD videos (default: 27, ≥3840 width).
* `--ntfy <TOPIC_URL>`: ntfy.sh topic URL for sending notifications.

#### Grain Analysis Options

* `--grain-sample-duration <SECONDS>`: Sample duration for grain analysis in seconds (default: 10).
* `--grain-knee-threshold <THRESHOLD>`: Knee point threshold (0.1-1.0) for determining optimal grain level (default: 0.8).
* `--grain-max-level <LEVEL>`: Maximum allowed grain level (VeryClean, VeryLight, Light, Visible, Medium) (default: Medium).
* `--grain-fallback-level <LEVEL>`: Fallback grain level if analysis fails (default: VeryClean).

## Advanced Features

### Intelligent Grain Detection and Denoising

Drapto includes a sophisticated film grain analysis system that optimizes denoising parameters for each video using the high-quality hqdn3d filter. The primary goal is to achieve significant bitrate reduction while maintaining visual quality, not to remove all grain or create an artificially smooth appearance.

Film grain and noise can consume a substantial portion of the bitrate in video encoding. By selectively reducing grain before encoding and then adding back controlled synthetic grain, Drapto achieves much better compression efficiency without sacrificing perceptual quality.

The system includes:

1. **Multi-Sample Analysis**: Extracts multiple short samples from different parts of the video to ensure consistent results.
2. **Baseline Comparison**: Always uses "VeryClean" (no grain) as the baseline for accurate comparison and analysis.
3. **Knee Point Detection**: Uses an advanced algorithm to find the optimal denoising strength that balances file size reduction and visual quality.
4. **Adaptive Refinement**: Dynamically adjusts and tests additional denoise parameters based on initial results.
5. **Categorical Classification**: Classifies videos into grain levels (VeryClean, VeryLight, Light, Visible, Medium) and applies appropriate hqdn3d parameters.
6. **Configurable Constraints**: Allows setting maximum grain levels and fallback options for fine-tuned control.
7. **High-Quality Denoising**: Uses FFmpeg's hqdn3d (high-quality 3D denoiser) filter with optimized parameters for each grain level. Conservative denoising settings are used to avoid excessive blurring while still improving compression.

This system ensures that videos with different grain characteristics are processed optimally:
- Videos with minimal grain receive minimal or no denoising to preserve detail
- Videos with medium grain receive moderate denoising to improve compression efficiency
- The process automatically finds the "sweet spot" where additional denoising provides diminishing returns
- Configuration options allow fine-tuning the analysis for different content types

### Film Grain Synthesis

Drapto not only detects and removes film grain when appropriate, but also intelligently applies film grain synthesis during encoding. This two-step approach is key to achieving significant bitrate savings:

1. **Adaptive Film Grain**: The detected grain level is mapped to appropriate SVT-AV1 film grain synthesis parameters
2. **Perceptual Quality**: Synthetic grain is added to maintain the visual character of the content while improving compression
3. **Balanced Approach**: The system applies:
   - No synthetic grain for very clean content
   - Light synthetic grain (level 4-8) for content with light natural grain
   - Medium synthetic grain (level 8-12) for content with moderate natural grain
   - Stronger synthetic grain (level 12-16) for content with medium natural grain

This approach provides the best of both worlds:
- Removes random, high-entropy natural grain that consumes excessive bitrate during encoding
- Adds back controlled synthetic grain that preserves the intended visual aesthetic but requires far fewer bits
- Results in significantly smaller files (often 20-40% smaller) while maintaining perceptual quality
- Preserves the original artistic intent of the content without creating an artificially smooth appearance

To disable both grain detection/denoising and film grain synthesis entirely, use the `--no-denoise` flag.

### HDR-Aware Processing

Drapto automatically detects HDR content and adjusts processing parameters accordingly:

1. **Black Level Analysis**: Performs specialized black level detection for HDR content
2. **Color Space Preservation**: Maintains HDR color information throughout the encoding process
3. **Adaptive Crop Thresholds**: Uses different crop detection thresholds for HDR content

## Development

Drapto is built with Rust and follows a modular architecture with two main components:

### Project Structure

```
drapto/
├── Cargo.toml           # Workspace configuration
├── drapto-cli/          # Command-line interface
│   ├── Cargo.toml       # CLI dependencies
│   └── src/             # CLI source code
└── drapto-core/         # Core library
    ├── Cargo.toml       # Core dependencies
    └── src/             # Core source code
```

### Building from Source

1. Clone the repository:
   ```bash
   git clone https://github.com/five82/drapto.git
   cd drapto
   ```

2. Build the project:
   ```bash
   cargo build --release
   ```

3. Run the binary:
   ```bash
   ./target/release/drapto --help
   ```