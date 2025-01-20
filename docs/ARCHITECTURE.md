# drapto Architecture Documentation

This document provides a detailed overview of drapto's architecture, including its directory structure, package organization, and module responsibilities.

## Target Structure

The codebase is being reorganized into the following structure:

```python
src/drapto/
├── core/                  # Core interfaces and base abstractions
│   ├── __init__.py
│   ├── encoder.py        # Encoder interface and base classes
│   ├── media.py          # Media handling interfaces
│   ├── config.py         # Configuration management
│   ├── exceptions.py     # Custom exceptions
│   ├── events.py         # Event system interface
│   ├── status.py         # Status tracking interface
│   └── temp.py           # Temporary file management interface
├── encoders/             # Encoder implementations
│   ├── __init__.py
│   ├── standard.py       # Standard FFmpeg-based encoder
│   └── chunked.py        # Chunked VMAF-based encoder
├── media/                # Media handling implementations
│   ├── __init__.py
│   ├── analysis.py       # Media analysis and detection
│   ├── hdr.py           # HDR/Dolby Vision processing
│   ├── audio.py          # Audio stream handling
│   ├── subtitle.py       # Subtitle handling
│   └── muxer.py          # Stream muxing
├── processing/           # Processing pipeline implementations
│   ├── __init__.py
│   ├── pipeline.py       # Pipeline orchestration
│   ├── worker.py         # Worker process management
│   └── queue.py          # Job queue management
├── state/                # State management implementations
│   ├── __init__.py
│   ├── manager.py        # State management implementation
│   └── progress.py       # Progress tracking implementation
├── system/               # System integration
│   ├── __init__.py
│   ├── ffmpeg.py         # FFmpeg wrapper
│   └── mediainfo.py      # MediaInfo wrapper
├── utils/                # Utility functions and helpers
│   ├── __init__.py
│   ├── paths.py          # Path management
│   └── logging.py        # Logging configuration
└── cli.py               # Command line interface

tests/
├── unit/                # Unit tests matching src structure
├── integration/         # Integration tests
└── fixtures/           # Test data and fixtures
```

## Directory Structure and Organization

### Package Structure

drapto is organized as a Python package with modular components:

```python
src/drapto/
├── core/              # Core infrastructure
│   ├── encoder.py     # Base encoder interface
│   ├── config.py      # Configuration management and validation
│   ├── events.py      # Event system
│   ├── status.py      # Status streaming
│   ├── exceptions.py  # Error handling and exceptions
│   └── temp.py        # Temporary file management
│
├── encoders/          # Encoding implementations
│   ├── standard.py    # Standard encoding path
│   └── chunked.py     # VMAF-based chunked encoding
│
├── media/             # Media handling
│   ├── analysis.py    # Video/audio analysis and metadata
│   ├── audio.py       # Audio processing
│   ├── subtitle.py    # Subtitle handling
│   └── muxer.py       # Stream muxing
│
├── processing/        # Processing logic
│   ├── pipeline.py    # Pipeline orchestration and segmentation
│   ├── worker.py      # Worker management
│   └── queue.py       # Job queue
│
├── state/            # State management
│   ├── manager.py    # State coordination
│   └── progress.py   # Progress tracking and metrics
│
├── system/           # System integration
│   ├── ffmpeg.py     # FFmpeg wrapper
│   └── mediainfo.py  # MediaInfo wrapper
│
└── utils/            # Utilities
    ├── logging.py    # Logging setup
    ├── paths.py      # Path handling
    └── validation.py # Input validation and terminal output
```

### Module Responsibilities

#### Core (`core/`)
- **Configuration Management**: Configuration loading, validation, and schema
- **Event System**: Event-driven communication between components
- **Error Handling**: Centralized exception hierarchy and error handling
- **Base Interfaces**: Core interfaces for encoders and media handling
- **Temporary Files**: Managed temporary file and directory lifecycle

#### Encoders (`encoders/`)
- **Standard Encoder**: Direct FFmpeg-based encoding with CRF control
- **Chunked Encoder**: VMAF-based encoding with segmentation
- Hardware acceleration and HDR handling integrated into each encoder

#### Media (`media/`)
- **Analysis**: Video/audio stream analysis and metadata extraction
- **Audio Processing**: Audio track management and encoding
- **Subtitle Handling**: Subtitle track preservation
- **Stream Muxing**: Final container assembly

#### Processing (`processing/`)
- **Pipeline**: Video processing orchestration and segmentation
- **Worker Management**: Parallel processing control
- **Queue Management**: Job scheduling and coordination

#### State (`state/`)
- **State Management**: Centralized state tracking and persistence
- **Progress Tracking**: Real-time progress updates and resource metrics

#### System (`system/`)
- **External Tools**: FFmpeg and MediaInfo integration
- Process management integrated into each wrapper

#### Utilities (`utils/`)
- **Logging**: Structured logging configuration
- **Path Management**: File and directory path handling
- **Validation**: Input validation and terminal output formatting

### Runtime Directory Structure

During operation, drapto maintains the following directory structure:

```
/tmp/drapto/
├── logs/           # Application logs
│   ├── app.log     # Main application log
│   └── debug.log   # Debug information
│
├── state/          # State persistence
│   ├── jobs/       # Per-job state files
│   └── metrics/    # Resource metrics data
│
├── segments/       # Video segments
│   ├── raw/       # Original segments
│   └── encoded/   # Encoded segments
│
├── encoded/        # Encoded outputs
│   └── temp/      # Temporary encoded files
│
└── working/        # Temporary processing
    ├── vmaf/      # VMAF analysis files
    └── mux/       # Muxing workspace
```

#### Directory Purposes

- **logs/**: Contains application logs and debug information
- **state/**: Persisted state for crash recovery
- **segments/**: Video chunks for parallel processing
- **encoded/**: Completed encoded files
- **working/**: Temporary processing workspace

All directories are managed by the `TempManager` which ensures proper cleanup on completion or failure. 

## Core Specifications and Requirements

### Input/Output Requirements

#### Input Format Requirements
- **Container**: MKV (Matroska) files only
- **Source Types**:
  - DVD sources (480i/p, 576i/p)
  - Blu-ray sources (720p, 1080i/p)
  - 4K UHD Blu-ray sources (2160p)
- **Video Streams**:
  - H.264/AVC
  - H.265/HEVC
  - VC-1
  - MPEG-2
  - ProRes
  - Supports HDR10, HDR10+, and Dolby Vision
- **Audio Streams**:
  - All common formats (AC3, DTS, TrueHD, etc.)
  - Multiple audio tracks supported
  - Original language tracks preserved
- **Subtitle Streams**:
  - PGS, SUP, SRT formats
  - Multiple subtitle tracks supported

#### Input Validation
- **Stream Validation**:
  - Video stream must be present and valid
  - At least one audio stream required
  - Subtitle streams optional but must be valid if present
- **Container Integrity**:
  - Valid MKV structure
  - Complete header information
  - No corruption in stream mappings
- **Metadata Requirements**:
  - Valid frame rate information
  - Valid resolution data
  - Color space information for HDR content

#### Output Format
- **Container**: MKV (strictly enforced)
- **Video Codec**: SVT-AV1
- **Audio Codec**: Opus
- **Subtitle Format**: Original formats preserved

### Codec Usage and Encoding Paths

drapto implements two distinct encoding paths, each optimized for different use cases:

#### SVT-AV1 Chunked Encoding Path
- **Primary encoding path for standard content**
- **Codec**: SVT-AV1 (CPU-only implementation)
- **Quality Control**: Target VMAF-based encoding
- **Features**:
  - Parallel chunk processing
  - Dynamic quality adjustments based on content complexity
  - CPU-only encoding (no hardware acceleration)
  - Configurable preset levels for speed/quality tradeoff
  - Automatic segment size determination

#### FFmpeg AV1 Dolby Vision Path
- **Specialized path for Dolby Vision content**
- **Codec**: FFmpeg's AV1 implementation
- **Quality Control**: CRF-based with resolution-specific values
  - SD content: Specified CRF value
  - HD content: Specified CRF value
  - UHD content: Specified CRF value
- **Features**:
  - Dolby Vision metadata preservation
  - Hardware acceleration support (when available)
  - Direct FFmpeg pipeline for metadata handling
  - Resolution-aware quality settings

### Processing Pipeline Architecture

The processing pipeline is implemented through several coordinated components:

#### Chunked Encoding Components
- **Analyzer**: Stream analysis and segmentation planning
  - Interfaces with MediaInfo for stream inspection
  - Determines optimal segment boundaries
  - See `PROCESSING.md` for detailed segmentation strategy
  
- **VMAF Controller**: Quality analysis and target management
  - Manages VMAF analysis workers
  - Coordinates quality measurements
  - Detailed VMAF analysis process in `PROCESSING.md`

- **Segment Manager**: Parallel processing coordination
  - Manages segment queue and state
  - Handles worker assignment
  - Coordinates segment reassembly
  
- **Worker Pool**: Processing execution
  - Manages parallel encoding processes
  - Handles resource allocation
  - Provides progress tracking

For detailed processing algorithms and implementation specifics, refer to `PROCESSING.md`.

### HDR Processing Architecture

The HDR processing system is composed of specialized components for handling different HDR formats:

#### HDR Components
- **Format Detector**: Identifies HDR format and metadata
  - HDR10/HDR10+ detection
  - Dolby Vision profile detection
  - Color space analysis
  
- **Metadata Manager**: HDR metadata handling
  - Metadata extraction and validation
  - Profile-specific metadata preservation
  - Cross-format metadata mapping

- **Processing Router**: Format-specific processing path selection
  - Dolby Vision → FFmpeg AV1 path
  - HDR10/HDR10+ → SVT-AV1 path
  - SDR fallback determination

See `PROCESSING.md` for detailed HDR processing algorithms, tone mapping approaches, and profile-specific handling.

### Processing Features

#### Video Analysis
- **Dolby Vision Detection**: Automatic detection and preservation of Dolby Vision metadata
- **Crop Detection**: Intelligent detection of video crop parameters
- **Quality Control**: Automated validation of input and output streams
  - Resolution validation
  - Frame rate verification
  - HDR metadata preservation
  - Audio sync verification

#### Encoding Strategy System
- **Quality-Based Decision Making**:
  - Resolution-specific CRF controls
  - Dynamic quality adjustments based on content complexity
  - VMAF-guided encoding decisions
- **Encoding Paths**:
  - Standard path: Direct FFmpeg-based encoding with CRF control
  - Chunked path: VMAF-based encoding with intelligent segmentation
  - Hardware-accelerated paths when available

### Configuration Management

#### Core Configuration Schema
```python
{
  "input": {
    "source_dir": str,      # Source directory for input files
    "file_pattern": str     # File matching pattern
  },
  "processing": {
    "chunk_size": int,      # Size of processing segments
    "parallel_jobs": int,   # Number of parallel encoding jobs
    "temp_dir": str         # Temporary processing directory
  },
  "encoding": {
    "preset": int,          # SVT-AV1 preset level
    "target_vmaf": float,   # Target VMAF score for chunked encoding
    "crf": {
      "sd": int,           # CRF value for SD content (Dolby Vision path)
      "hd": int,           # CRF value for HD content (Dolby Vision path)
      "uhd": int           # CRF value for UHD content (Dolby Vision path)
    }
  },
  "audio": {
    "bitrate": int,         # Target bitrate in kbps for Opus encoding
    "channels": int         # Number of audio channels to preserve
  },
  "hdr": {
    "preserve_metadata": bool,  # Whether to preserve HDR metadata
    "fallback_mode": str       # HDR fallback mode if unsupported
  }
}
```

### CLI Architecture

The CLI system is built on a layered architecture that separates concerns:

#### CLI Components
- **Command Parser**: Entry point for CLI commands
  - Argument parsing and validation
  - Configuration file handling
  - Environment variable integration

- **Operation Controller**: Manages core system interaction
  - Translates CLI commands to core operations
  - Handles operation lifecycle
  - Manages user feedback and progress display

- **Status Manager**: Real-time status handling
  - Progress bar management
  - ETA calculations
  - Resource usage display
  - Error reporting

The CLI layer communicates with the core system through the event system, maintaining a clean separation between interface and processing logic.

### Event System

#### Core Events
```python
ProcessingEvents = {
    'FILE_DISCOVERED',     # New file found for processing
    'ANALYSIS_STARTED',    # Beginning file analysis
    'ANALYSIS_COMPLETE',   # Analysis finished
    'ENCODING_STARTED',    # Starting encoding process
    'SEGMENT_COMPLETE',    # Individual segment finished
    'ENCODING_COMPLETE',   # Full encoding complete
    'ERROR_OCCURRED'       # Error handling event
}
```

#### Event Payload Structure
```python
{
    "event": ProcessingEvents,
    "timestamp": datetime,
    "file": str,
    "data": Dict[str, Any],
    "status": ProcessingStatus
}
```

### Input Processing Flow

The processing system follows this sequence:
1. File Discovery and Validation
2. Stream Analysis and Metadata Extraction
3. Encoding Strategy Selection
4. Segmentation (if using chunked encoding)
5. Parallel Processing
6. Quality Verification
7. Final Muxing

### State Management Architecture

The state management system maintains processing state through a hierarchical structure:

#### State Components
- **Job State**: Per-job processing information
  ```python
  {
    "job_id": str,           # Unique job identifier
    "input_file": str,       # Source file path
    "stage": ProcessingStage, # Current processing stage
    "segments": List[Dict],   # Segment processing states
    "metadata": Dict,         # Job-specific metadata
    "resources": Dict         # Resource allocation state
  }
  ```

- **Segment State**: Individual segment tracking
  ```python
  {
    "segment_id": str,       # Segment identifier
    "start_time": float,     # Segment start timestamp
    "duration": float,       # Segment duration
    "status": SegmentStatus, # Current segment status
    "vmaf_score": float,     # Segment quality score
    "attempts": int          # Processing attempt count
  }
  ```

- **Resource State**: System resource tracking
  ```python
  {
    "workers": Dict[str, WorkerState], # Worker process states
    "memory_usage": float,             # Current memory usage
    "cpu_usage": float,                # Current CPU usage
    "disk_usage": Dict                 # Temporary storage usage
  }
  ```

State persistence is managed by the StateManager component, which handles:
- State file serialization/deserialization
- Atomic state updates
- Checkpoint creation/restoration
- State cleanup on completion

### Error Recovery and Fallback Mechanisms

#### Recovery Strategies
- **State Persistence**: Continuous state tracking for crash recovery
- **Checkpoint System**: Regular state snapshots during processing
- **Fallback Mechanisms**:
  - Automatic retry for transient failures
  - Graceful degradation for hardware acceleration
  - Alternative encoding path selection
  - Segment-based recovery for partial failures

### Logging Architecture

The logging system is designed for encode troubleshooting with distinct logging levels:

#### Logging Levels
- **ERROR**: Critical failures and unrecoverable errors
  - Encoding failures
  - File system errors
  - Invalid input conditions
  - State corruption

- **WARNING**: Recoverable issues and potential problems
  - Segment retries
  - Quality target misses
  - Resource constraints
  - Fallback to alternative paths

- **INFO**: Normal operation progress
  - Stage transitions
  - Segment completion
  - Quality measurements
  - Resource utilization

- **DEBUG**: Detailed troubleshooting information
  - Command execution details
  - FFmpeg parameters (Dolby Vision path)
  - ab-av1/SVT-AV1 parameters (Chunked encoding path)
  - VMAF calculations and target adjustments
  - Worker state changes
  - Segment boundaries and chunk decisions

Logs are written to `app.log` and `debug.log` in the temporary directory and are cleared on successful completion.

### Testing Strategy

#### Test Categories
- **Unit Tests**: Individual component verification
  - Core processing functions
  - Encoder implementations
  - Configuration validation
- **Integration Tests**: Component interaction validation
  - Full processing pipeline
  - Event system integration
  - State management
- **Performance Tests**: System optimization validation
  - Encoding speed benchmarks
  - Memory usage profiling
  - Parallel processing efficiency
- **Property Tests**: System invariant verification
  - Output format compliance
  - Quality thresholds
  - Resource management

#### Quality Assurance
- Minimum 95% test coverage requirement
- Automated CI/CD pipeline integration
- Performance regression testing
- Cross-platform compatibility verification 