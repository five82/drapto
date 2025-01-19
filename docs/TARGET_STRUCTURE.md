# Target Directory and File Structure

## Overview
This document outlines the target directory and file structure for drapto after the refactoring process. The goal is to have a cleaner, more maintainable structure that better separates concerns and makes the codebase easier to understand and modify.

## Project Root Structure
```
drapto/
├── src/
│   └── drapto/
│       ├── __init__.py
│       ├── cli.py               # Command-line interface
│       ├── core/
│       │   ├── __init__.py
│       │   ├── encoder.py       # Main encoder class
│       │   ├── process.py       # Process management
│       │   └── state.py         # State management
│       ├── config/
│       │   ├── __init__.py
│       │   ├── settings.py      # Configuration management
│       │   └── defaults.py      # Default configuration values
│       ├── encoding/
│       │   ├── __init__.py
│       │   ├── strategies/      # Different encoding strategies
│       │   │   ├── __init__.py
│       │   │   ├── base.py      # Base strategy class
│       │   │   ├── standard.py  # Standard encoding
│       │   │   ├── chunked.py   # Chunked encoding
│       │   │   └── parallel.py  # Parallel processing
│       │   ├── video.py         # Video encoding logic
│       │   ├── audio.py         # Audio encoding logic
│       │   └── subtitles.py     # Subtitle handling
│       └── utils/
│           ├── __init__.py
│           ├── ffmpeg.py        # FFmpeg integration
│           ├── logging.py       # Logging utilities
│           └── paths.py         # Path handling utilities
├── tests/
│   ├── unit/                    # Unit tests
│   │   ├── test_core/
│   │   ├── test_encoding/
│   │   └── test_utils/
│   ├── integration/             # Integration tests
│   │   ├── test_strategies/
│   │   └── test_end_to_end/
│   └── conftest.py             # Test fixtures and configuration
├── docs/
│   ├── ARCHITECTURE.md         # Architecture documentation
│   ├── REFACTORING_PLAN.md     # Refactoring plan
│   └── API.md                  # API documentation
└── examples/                    # Example scripts and configurations
    ├── basic_usage.py
    └── advanced_config.py
```

## Key Changes from Current Structure

1. Core Components
   - Move from bash scripts to Python-based encoding logic
   - Separate process management into dedicated module
   - Centralize state management

2. Configuration Management
   - Dedicated config module for all settings
   - Clear separation of default and user configurations
   - Environment variable handling in one place

3. Encoding Strategies
   - Clear separation of different encoding approaches
   - Common base class for all strategies
   - Modular approach to adding new strategies

4. Utilities
   - Dedicated modules for common functionality
   - Better organization of helper functions
   - Improved logging and debugging support

## Temporary Directory Structure
During encoding operations, the following temporary directory structure will be used:
```
$TEMP_DIR/
├── state/                 # State files
│   └── encoding_state.json
├── segments/             # Video segments for chunked encoding
├── encoded_segments/     # Encoded video segments
├── working/             # Working directory for current operations
└── logs/                # Log files
    ├── encoder.log
    ├── ffmpeg.log
    └── debug.log
```

## Configuration Files
User configuration will be handled through:
1. Command-line arguments
2. Environment variables
3. Optional configuration file (YAML/JSON)
4. Default settings

## Migration Strategy
The transition from the current structure to the target structure will be gradual:
1. First, maintain compatibility with existing scripts
2. Gradually move functionality to Python modules
3. Deprecate bash scripts as Python replacements are completed
4. Finally remove bash scripts entirely

## Future Considerations
- Plugin system for custom encoding strategies
- API for integration with other tools
- Web interface capabilities
- Remote encoding support 