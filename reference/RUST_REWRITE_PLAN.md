# Plan for Rewriting Drapto in Rust

## Phase 1: Project Setup and Core Infrastructure
1. Set up Rust project with Cargo
2. Establish error handling patterns
3. Implement logging framework
4. Create FFmpeg/FFprobe wrappers

## Phase 2: Core Modules
1. Implement media inspection (FFprobe wrapper)
2. Build command execution system
3. Create config management module with layered configuration:
   - Default values defined in code
   - TOML configuration file support
   - Environment variable overrides 
   - Command-line argument precedence
4. Develop validation framework

## Phase 3: Processing Pipelines
1. Implement video detection (Dolby Vision, HDR, crop detection)
2. Build scene detection module
3. Develop video segmentation system
4. Create memory-aware scheduler for parallel encoding. parallel chunk encoding tasks can use a significant amount of memory per task depending on the video being encoded. So we need to properly manage tasks so we do not exhaust physical memory.

## Phase 4: Encoding Implementation
1. Implement ab-av1 video encoding modules
2. Build ffmpeg audio encoding functionality
3. Develop segment merger/concatenation
4. Create muxing system

## Phase 5: CLI and Integration
1. Implement command-line interface
2. Set up pipeline orchestration
3. Add validation and quality checks. Only add quality checks that are already in the python code.
4. Create summary reporting

## Key Rust Dependencies
- ffmpeg-the-third (latest version - supports ffmpeg 7.1): FFmpeg bindings
- clap: Command line argument parsing
- log + env_logger: Logging
- rayon: Parallel computing
- serde: Serialization/deserialization
- toml: Configuration file parsing
- dirs: Cross-platform directory paths
- tokio: Async runtime for I/O operations
- ctor: For resource cleanup
- anyhow/thiserror: Error handling

## Principles
1. Follow Rust idioms over direct Python translation
2. Use strong typing and Results for error handling
3. Leverage Rust's ownership model for memory safety
4. Use traits to define interfaces between components
5. Implement concurrency with Rust's safety guarantees
6. Focus on minimalism and performance
7. Avoid scope creap and bloat. Only implement drapto application functionality that is already in the Python implementation
8. Look at the old python code for context.
9. Use proper rust file and directory structure according to Rust best practices.
