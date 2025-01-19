# Target Directory and File Structure

## Overview
This document outlines the target directory and file structure for drapto after the refactoring process. The goal is to have a cleaner, more maintainable structure that better separates concerns and makes the codebase easier to understand and modify.

## Project Root Structure
```
drapto/
├── src/
│   └── drapto/
│       ├── __init__.py
│       ├── cli.py                    # Command-line interface
│       ├── core/
│       │   ├── __init__.py
│       │   ├── encoder.py            # Main encoder class
│       │   ├── process/              # Process management
│       │   │   ├── __init__.py
│       │   │   ├── manager.py        # Process lifecycle management
│       │   │   ├── hierarchy.py      # Process hierarchy tracking
│       │   │   ├── resources.py      # Resource tracking and cleanup
│       │   │   └── signals.py        # Signal handling
│       │   ├── state/               # State management
│       │   │   ├── __init__.py
│       │   │   ├── manager.py       # In-memory state manager
│       │   │   ├── boundaries.py    # State boundaries
│       │   │   └── validation.py    # State validation
│       │   └── events/              # Event system
│       │       ├── __init__.py
│       │       ├── dispatcher.py    # Event dispatching
│       │       ├── handlers.py      # Event handlers
│       │       └── streams.py       # Progress streaming
│       ├── config/
│       │   ├── __init__.py
│       │   ├── settings.py          # Configuration management
│       │   ├── validation.py        # Config validation
│       │   ├── types.py            # Config type definitions
│       │   └── defaults.py         # Default configuration values
│       ├── encoding/
│       │   ├── __init__.py
│       │   ├── strategies/         # Strategy coordination
│       │   │   ├── __init__.py
│       │   │   ├── base.py        # Base strategy interface
│       │   │   ├── factory.py     # Strategy selection/creation
│       │   │   ├── standard.py    # Standard strategy implementation
│       │   │   └── chunked.py     # Chunked strategy implementation
│       │   ├── base/              # Common encoding components
│       │   │   ├── __init__.py
│       │   │   ├── analysis.py    # Input analysis
│       │   │   ├── hardware.py    # Hardware acceleration
│       │   │   └── validation.py  # Common validation
│       │   ├── standard/          # Standard encoding path
│       │   │   ├── __init__.py
│       │   │   ├── encoder.py     # Direct FFmpeg encoding
│       │   │   ├── quality.py     # CRF-based quality control
│       │   │   └── validation.py  # Standard path validation
│       │   ├── chunked/           # Chunked encoding path
│       │   │   ├── __init__.py
│       │   │   ├── encoder.py     # ab-av1 based encoding
│       │   │   ├── segments.py    # Segment management
│       │   │   ├── vmaf.py       # VMAF-based quality control
│       │   │   └── parallel.py    # Parallel processing
│       │   ├── quality/           # Quality control components
│       │   │   ├── __init__.py
│       │   │   ├── base.py       # Common quality interfaces
│       │   │   ├── crf.py        # CRF-based quality control
│       │   │   └── vmaf/         # VMAF-based quality control
│       │   │       ├── __init__.py
│       │   │       ├── targeting.py  # VMAF target management
│       │   │       ├── sampling.py   # Sample selection
│       │   │       └── validation.py # VMAF validation
│       │   ├── audio.py          # Audio encoding
│       │   └── subtitles.py      # Subtitle handling
│       ├── types/                 # Type definitions
│       │   ├── __init__.py
│       │   ├── process.py         # Process types
│       │   ├── state.py          # State types
│       │   └── events.py         # Event types
│       ├── monitoring/           # Monitoring components
│       │   ├── __init__.py
│       │   ├── metrics.py        # Performance metrics
│       │   ├── resources.py      # Resource monitoring
│       │   └── debug.py         # Debug output
│       └── utils/
│           ├── __init__.py
│           ├── ffmpeg.py         # FFmpeg integration
│           ├── logging.py        # Logging utilities
│           ├── paths.py          # Path handling utilities
│           └── formatting/       # Output formatting
│               ├── __init__.py
│               ├── colors.py     # Color support and detection
│               ├── terminal.py   # Terminal capability detection
│               └── styles.py     # Output styling functions
├── tests/
│   ├── unit/                    # Unit tests
│   │   ├── test_core/
│   │   │   ├── test_process/
│   │   │   ├── test_state/
│   │   │   └── test_events/
│   │   ├── test_encoding/
│   │   └── test_utils/
│   ├── integration/             # Integration tests
│   │   ├── test_strategies/
│   │   └── test_end_to_end/
│   ├── fixtures/               # Test fixtures
│   │   ├── __init__.py
│   │   ├── process.py
│   │   ├── state.py
│   │   └── events.py
│   ├── utilities/              # Test utilities
│   │   ├── __init__.py
│   │   ├── mocks.py
│   │   └── helpers.py
│   ├── coverage/              # Coverage reports
│   ├── type_checking/         # Type validation tests
│   └── conftest.py           # Test configuration
├── docs/
│   ├── api/                  # API documentation
│   │   ├── current/
│   │   └── migrations/
│   ├── architecture/
│   │   ├── OVERVIEW.md
│   │   ├── PROCESS.md
│   │   ├── STATE.md
│   │   └── EVENTS.md
│   ├── testing/
│   │   ├── OVERVIEW.md
│   │   ├── FIXTURES.md
│   │   └── COVERAGE.md
│   └── development/
│       ├── CONTRIBUTING.md
│       ├── STYLE.md
│       └── WORKFLOW.md
└── examples/                 # Example scripts and configurations
    ├── basic_usage.py
    └── advanced_config.py
```

## Temporary Directory Structure
During encoding operations, the following temporary directory structure will be used:
```
$TEMP_DIR/
├── working/              # Single working directory for all operations
│   ├── segments/        # Video segments for chunked encoding
│   ├── encoded/         # Encoded segments
│   └── resources/       # Resource tracking
├── logs/                # Log files
│   ├── process/         # Process-specific logs
│   ├── events/          # Event logs
│   └── debug/           # Debug output
└── monitoring/          # Monitoring data
    ├── metrics/         # Performance metrics
    └── resources/       # Resource usage data
```

## Core Components

1. **Process Management**
   - Process lifecycle tracking
   - Parent-child process relationships
   - Resource tracking and cleanup
   - Signal handling and propagation

2. **State Management**
   - In-memory state tracking
   - Atomic state updates
   - State boundaries and validation
   - Recovery mechanisms

3. **Event System**
   - Event dispatching
   - Status updates
   - Progress streaming
   - Error propagation

4. **Configuration**
   - Type-safe settings
   - Validation schemas
   - Default configurations
   - User overrides

5. **Monitoring**
   - Performance metrics
   - Resource tracking
   - Debug output
   - Event logging

## Testing Structure

1. **Unit Tests**
   - Component-level testing
   - State validation
   - Event handling
   - Process management

2. **Integration Tests**
   - End-to-end workflows
   - Strategy testing
   - Performance validation
   - Resource management

3. **Test Infrastructure**
   - Shared fixtures
   - Mock objects
   - Helper utilities
   - Coverage tracking

4. **Type Checking**
   - Static type validation
   - Runtime type checking
   - Interface verification
   - Type documentation

## Documentation Organization

1. **API Documentation**
   - Current version
   - Migration guides
   - Version history
   - Breaking changes

2. **Architecture Documentation**
   - System overview
   - Component interactions
   - State management
   - Process control

3. **Testing Documentation**
   - Testing strategy
   - Fixture usage
   - Coverage requirements
   - Type checking

4. **Development Guides**
   - Contributing guidelines
   - Code style
   - Workflow procedures
   - Review process 