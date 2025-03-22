We are implementing a rewrite of the Python drapto in rust
Rewrite plan is RUST_REWRITE_PLAN.md

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

## Key Rust Dependencies
- ffmpeg-the-third (latest version - supports ffmpeg 7.1): FFmpeg bindings
- clap: Command line argument parsing
- log + env_logger: Logging
- rayon: Parallel computing
- serde: Serialization/deserialization
- tokio: Async runtime for I/O operations
- ctor: For resource cleanup
- anyhow/thiserror: Error handling

## Benefits

1. **Improved Maintainability**: Clear separation of concerns makes the codebase easier to maintain
2. **Better Reusability**: Core logic can be used in other projects without the CLI component
3. **Enhanced Testability**: Clearer boundaries make unit testing more straightforward
4. **API Stability**: Better defined interfaces between components
5. **Extensibility**: Easier to add new features or alternative interfaces (GUI, web API, etc.)

### drapto-core Crate

This is the foundation library containing all the core functionality:

```
drapto-core/
├── Cargo.toml
└── src/
    ├── lib.rs          # Main library entry point with public API
    ├── error.rs        # Error types and handling
    ├── media/          # Media processing and information
    │   ├── mod.rs
    │   ├── info.rs     # Media information structures
    │   └── probe.rs    # FFprobe wrapper
    ├── encoding/       # Encoding logic
    │   ├── mod.rs
    │   ├── video.rs    # Video encoding
    │   ├── audio.rs    # Audio encoding
    │   └── pipeline.rs # Encoding pipeline
    ├── detection/      # Detection algorithms
    │   ├── mod.rs
    │   ├── scene.rs    # Scene detection
    │   └── format.rs   # Format detection (HDR, DV, etc.)
    ├── validation/     # Media validation
    │   ├── mod.rs
    │   ├── video.rs
    │   ├── audio.rs
    │   ├── sync.rs     # A/V sync validation
    │   └── report.rs   # Validation reporting
    ├── util/           # Utility functions
    │   ├── mod.rs
    │   ├── command.rs  # Command execution
    │   └── logging.rs  # Logging utilities
    └── config.rs       # Configuration structures
```

### drapto-cli Crate

This contains only the CLI application:

```
drapto-cli/
├── Cargo.toml
└── src/
    ├── main.rs         # CLI entry point
    ├── commands/       # CLI commands
    │   ├── mod.rs
    │   ├── encode.rs
    │   ├── validate.rs
    │   └── info.rs
    ├── output.rs       # Pretty-printing and output formatting
    └── args.rs         # Command-line argument parsing
```
