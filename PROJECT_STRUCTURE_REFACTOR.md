# Drapto Project Structure Refactoring Plan

## Current Issues

The current project structure of drapto-rs has several limitations:

1. **Monolithic Architecture**: The codebase doesn't fully separate the core logic from the CLI interface.
2. **Unclear API Boundaries**: Public interfaces aren't well-defined between modules.
3. **Limited Reusability**: The current structure makes it difficult to reuse components in other projects.
4. **Testing Challenges**: Tightly coupled components make unit testing more difficult.

## Proposed Structure: Workspace with Multiple Crates

### 1. Create a Cargo Workspace

Restructure the project as a Cargo workspace with the following crates:

- `drapto-core`: Core library containing all reusable functionality
- `drapto-cli`: Command-line interface application
- `drapto` (optional): Meta-package that re-exports core functionality

### 2. drapto-core Crate

This will be the foundation library containing all the core functionality:

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

### 3. drapto-cli Crate

This will contain only the CLI application:

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

## Implementation Steps

1. **Create Workspace Structure**:
   - Create a top-level `Cargo.toml` with workspace members
   - Create directories for each crate

2. **Move Core Logic to drapto-core**:
   - Refactor the existing code to fit the new structure
   - Clean up public APIs and module boundaries
   - Make sure each module has a well-defined responsibility

3. **Develop CLI Interface**:
   - Create a clean CLI interface that depends on drapto-core
   - Ensure all CLI-specific logic is in drapto-cli

4. **Update Build and Test Infrastructure**:
   - Adjust CI/CD workflows for the new structure
   - Update test cases for the refactored code

5. **Documentation Updates**:
   - Update README and documentation to reflect the new structure
   - Document the new API boundaries and usage patterns

## Benefits

1. **Improved Maintainability**: Clear separation of concerns makes the codebase easier to maintain
2. **Better Reusability**: Core logic can be used in other projects without the CLI component
3. **Enhanced Testability**: Clearer boundaries make unit testing more straightforward
4. **API Stability**: Better defined interfaces between components
5. **Extensibility**: Easier to add new features or alternative interfaces (GUI, web API, etc.)

## Compatibility Considerations

The refactoring should be implemented in a way that:
- Maintains backward compatibility for existing users
- Doesn't change the functionality of the application
- Allows for future expansion

## Timeline

This refactoring should be completed before implementing Phase 4 (Encoding Implementation) of the main RUST_REWRITE_PLAN.md, as it will provide a better foundation for those features.