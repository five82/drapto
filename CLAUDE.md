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
