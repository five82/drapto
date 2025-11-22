# AGENTS.md

This file provides guidance when working with code in this repository.

Use `python3` for all Python commands.
Do not git commit or git push unless explicitly ask to.

## Project Overview

Drapto is an advanced video encoding tool that uses ffmpeg to optimize and encode videos with intelligent analysis and high-quality compression. The tool automates video encoding tasks using ffmpeg with libsvtav1 (for video) and libopus (for audio), providing features like automatic crop detection and HDR-aware processing.

## Architecture

The project follows a modular Rust workspace architecture with two main components:

1. **drapto-cli**: Command-line interface and user interaction
   - Handles argument parsing, logging, and progress orchestration
   - Provides user-friendly command interface
   - Manages progress reporting and feedback

2. **drapto-core**: Core video processing and analysis library
   - Video analysis (crop detection, HDR awareness)
   - FFmpeg integration and command generation
   - Video encoding orchestration

## Development Commands

### Building

```bash
# Clean and build the project
./build.sh

# Or manually build with cargo
cargo build --release
```

### Running

```bash
# Run a build from the project directory
cargo run -- encode -i /path/to/video.mkv -o /path/to/output/

# Enable debug logging
RUST_LOG=debug cargo run -- encode -i /path/to/video.mkv -o /path/to/output/
```

### Debugging

```bash
# Enable trace-level logging for more detailed output
RUST_LOG=trace cargo run -- encode -i input.mkv -o output/
```

## Key Components

### FFmpeg Integration

The project uses FFmpeg for video processing via:

1. `ffmpeg-sidecar` for command execution
2. `ffprobe` for media file analysis
3. Custom command builders that generate optimized encoding commands

### Progress Reporting

The progress reporting system provides feedback during long-running operations:

1. Terminal-based progress bars for foreground mode
2. Detailed logging for automated consumers

### CLI Output Style

1. Render just four sections in human mode: Hardware → Video → Encoding → Validation → Results; keep each section to a handful of lines.
2. Use plain `println!` plus `indicatif` for the progress bar—no template engine or ASCII art. Colors are fine when they reinforce meaning (cyan headers, magenta stage bullets, green checkmarks, yellow warnings, red errors).
3. Show real progress information only once: rely on the progress bar during encoding, and print a single validation summary after it completes.
4. Keep the JSON stream (`--progress-json`) backward-compatible with the existing objects Spindle consumes (`encoding_progress`, `validation_complete`, `encoding_complete`, `warning`, `error`, `batch_complete`).
5. Prefer natural language sentences (“Encoding finished successfully”) and reserve emphatic formatting for values that matter (reduction %, warnings, output paths).

## Code Style Guidelines

1. Use descriptive variable names and comprehensive documentation
2. Follow Rust's naming conventions (snake_case for variables/functions, CamelCase for types)
3. Organize code into modular components with clear responsibilities
4. Use Rust's type system to enforce invariants where possible
5. Use Result and Option types for proper error handling and state representation

## Project Structure

The core functionality is organized into modules:

- **detection**: Crop detection algorithms
- **external**: FFmpeg and FFprobe integrations
- **processing**: Video and audio processing pipelines
- **config**: Configuration management

When working with the codebase, understand the flow:
1. CLI parses arguments and initializes components
2. Core detection modules analyze input video
3. Processing modules apply transformations
4. External tools execute the actual encoding

## Principles

1. Follow Rust idioms
2. Use strong typing and Results for error handling
3. Leverage Rust's ownership model for memory safety
4. Use traits to define interfaces between components
5. Implement concurrency with Rust's safety guarantees
6. Focus on minimalism and performance
7. Avoid scope creap and bloat.
8. Avoid overengineering solutions.
9. Use proper rust file and directory structure according to Rust best practices.
10. This is a small hobby project maintained by a single develper. The project scope should reflect this.
11. When running drapto with a timeout, do not use a timeout value of less than 120 seconds so the encoding processing steps have a chance to finish.
12. Video encoding takes significant time. When testing drapto, use unit tests to test logic over running actual encodes when possible.
