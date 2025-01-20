# drapto Functionality Documentation

This document provides a detailed overview of how drapto processes and encodes videos, including its detection mechanisms, encoding paths, and configuration options.

drapto is designed to work specifically with MKV video files sourced from DVD, Blu-ray, and 4K UHD Blu-ray sources. The output is strictly standardized:
- Container format is always MKV
- Video is always encoded using SVT-AV1
- Audio is always encoded using Opus

## Table of Contents
1. [Directory Structure and Organization](#directory-structure-and-organization)
2. [Input Video Processing Flow](#input-video-processing-flow)
3. [Dolby Vision Detection](#dolby-vision-detection)
4. [Encoding Paths](#encoding-paths)
5. [Encoding Strategy System](#encoding-strategy-system)
6. [Parallel Processing](#parallel-processing)
7. [Muxing Process](#muxing-process)
8. [Audio Processing](#audio-processing)
9. [Crop Detection](#crop-detection)
10. [Codec Usage](#codec-usage)
11. [Validation and Quality Control](#validation-and-quality-control)
12. [Default Settings](#default-settings)
13. [User Configuration](#user-configuration)
14. [Hardware Acceleration](#hardware-acceleration)
15. [Validation Process](#validation-process)
16. [Error Recovery and Fallback Mechanisms](#error-recovery-and-fallback-mechanisms)
17. [Progress Tracking and Logging](#progress-tracking-and-logging)
18. [Process Management](#process-management)
19. [Temporary File Management](#temporary-file-management)

## Directory Structure and Organization

### Package Structure

drapto is organized as a Python package with modular components:

```python
src/drapto/
├── core/              # Core infrastructure
│   ├── encoder.py     # Base encoder interface
│   ├── config/        # Configuration management
│   ├── events.py      # Event system
│   ├── status.py      # Status streaming
│   ├── errors.py      # Error handling
│   └── temp.py        # Temporary file management
│
├── encoders/          # Encoding implementations
│   ├── standard.py    # Standard encoding path
│   ├── chunked.py     # VMAF-based chunked encoding
│   ├── options.py     # Encoding options/config
│   ├── hardware.py    # Hardware acceleration
│   └── dolby.py       # Dolby Vision handling
│
├── media/             # Media handling
│   ├── analysis.py    # Video/audio analysis
│   ├── metadata.py    # Media metadata
│   ├── audio.py       # Audio processing
│   ├── subtitle.py    # Subtitle handling
│   └── muxer.py       # Stream muxing
│
├── processing/        # Processing logic
│   ├── segmentation.py # Video segmentation
│   ├── vmaf.py        # VMAF calculations
│   ├── worker.py      # Worker management
│   └── queue.py       # Job queue
│
├── state/            # State management
│   ├── manager.py    # State coordination
│   ├── types.py      # State data structures
│   ├── progress.py   # Progress tracking
│   └── metrics.py    # Resource monitoring
│
├── system/           # System integration
│   ├── ffmpeg.py     # FFmpeg wrapper
│   ├── mediainfo.py  # MediaInfo wrapper
│   ├── process.py    # Process management
│   └── signals.py    # Signal handling
│
└── utils/            # Utilities
    ├── logging.py    # Logging setup
    ├── paths.py      # Path handling
    ├── validation.py # Input validation
    └── terminal.py   # Terminal handling
```

### Module Responsibilities

#### Core (`core/`)
- **Configuration Management**: Schema-based configuration with validation
- **Event System**: Event-driven communication between components
- **Error Handling**: Structured error handling with context
- **Base Interfaces**: Core interfaces for encoders and media handling
- **Temporary Files**: Managed temporary file and directory lifecycle

#### Encoders (`encoders/`)
- **Standard Encoder**: Direct FFmpeg-based encoding with CRF control
- **Chunked Encoder**: VMAF-based encoding with segmentation
- **Hardware Support**: GPU acceleration and hardware detection
- **Options Management**: Encoding parameters and validation
- **Dolby Vision**: HDR and Dolby Vision content handling

#### Media (`media/`)
- **Analysis**: Video and audio stream analysis
- **Metadata**: Media information extraction and validation
- **Audio Processing**: Audio track management and encoding
- **Subtitle Handling**: Subtitle track preservation
- **Stream Muxing**: Final container assembly

#### Processing (`processing/`)
- **Segmentation**: Video chunk management
- **VMAF Analysis**: Quality metric calculations
- **Worker Management**: Parallel processing control
- **Queue Management**: Job scheduling and coordination

#### State (`state/`)
- **State Management**: Centralized state tracking
- **Progress Monitoring**: Real-time progress updates
- **Resource Metrics**: System resource tracking
- **State Persistence**: Crash recovery and state restoration

#### System (`system/`)
- **External Tools**: FFmpeg and MediaInfo integration
- **Process Control**: Process lifecycle management
- **Signal Handling**: Clean process termination
- **Resource Management**: System resource allocation

#### Utilities (`utils/`)
- **Logging**: Structured logging configuration
- **Path Management**: File and directory path handling
- **Input Validation**: Data validation utilities
- **Terminal Output**: Progress display and formatting

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

## Input Video Processing Flow

The input processing system uses a modern event-driven architecture with centralized configuration and state management:

1. **Configuration Management**
   ```python
   # Core configuration structure
   config/
   ├── default.py      # Default configuration
   ├── schema.py       # Configuration schema
   └── validator.py    # Schema validation
   ```

   - **Configuration Loading**
     ```python
     {
       "input": {
         "source_dir": str,
         "file_pattern": str,
         "min_size": int,
         "max_size": int
       },
       "processing": {
         "chunk_size": int,
         "parallel_jobs": int,
         "temp_dir": str
       },
       "encoding": {
         "preset": int,
         "crf": {
           "sd": int,
           "hd": int,
           "uhd": int
         }
       }
     }
     ```

2. **Event-Based Status Updates**
   - Core events:
     ```python
     class ProcessingEvents(Enum):
         FILE_DISCOVERED = "file_discovered"
         ANALYSIS_STARTED = "analysis_started"
         ANALYSIS_COMPLETE = "analysis_complete"
         ENCODING_STARTED = "encoding_started"
         SEGMENT_COMPLETE = "segment_complete"
         ENCODING_COMPLETE = "encoding_complete"
         ERROR_OCCURRED = "error_occurred"
     ```
   - Event payload structure:
     ```python
     {
         "event": ProcessingEvents,
         "timestamp": datetime,
         "file": str,
         "data": Dict[str, Any],
         "status": ProcessingStatus
     }
     ```

3. **State Management Flow**
   ```python
   class ProcessingState:
       def __init__(self):
           self.current_file: Optional[str] = None
           self.stage: ProcessingStage = ProcessingStage.INIT
           self.progress: float = 0.0
           self.errors: List[Error] = []
           self.stats: Dict[str, Any] = {}

   class StateManager:
       def update_state(self, event: ProcessingEvent) -> None:
           """Update processing state based on event"""
           
       def get_current_state(self) -> ProcessingState:
           """Get current processing state"""
           
       def save_checkpoint(self) -> None:
           """Save state checkpoint for recovery"""
   ```

4. **Directory Management**
   ```python
   class TempManager:
       def __init__(self, config: Config):
           self.base_dir = config.processing.temp_dir
           self.paths = self._initialize_paths()
           
       def _initialize_paths(self) -> Dict[str, Path]:
           """Initialize directory structure"""
           return {
               "working": self.base_dir / "working",
               "segments": self.base_dir / "segments",
               "encoded": self.base_dir / "encoded",
               "logs": self.base_dir / "logs",
               "state": self.base_dir / "state"
           }
           
       def setup(self) -> None:
           """Create directory structure"""
           for path in self.paths.values():
               path.mkdir(parents=True, exist_ok=True)
               
       def cleanup(self, keep_logs: bool = True) -> None:
           """Clean temporary directories"""
   ```

5. **Processing Pipeline**
   ```python
   class ProcessingPipeline:
       def __init__(self, config: Config):
           self.config = config
           self.state_manager = StateManager()
           self.temp_manager = TempManager(config)
           self.event_bus = EventBus()
           
       async def process_file(self, input_file: Path) -> None:
           """Process single input file"""
           try:
               # Initialize processing
               self.temp_manager.setup()
               self.state_manager.start_file(input_file)
               
               # Analysis phase
               await self._analyze_file(input_file)
               
               # Encoding phase
               await self._encode_file(input_file)
               
               # Finalization
               await self._finalize_output()
               
           except ProcessingError as e:
               self.event_bus.emit(
                   ProcessingEvents.ERROR_OCCURRED,
                   {"error": str(e)}
               )
               raise
               
           finally:
               self.temp_manager.cleanup()
   ```

6. **Error Recovery**
   - State-based recovery system:
     ```python
     class RecoveryManager:
         def can_recover(self, state: ProcessingState) -> bool:
             """Check if processing can be recovered"""
             
         def get_recovery_point(self) -> Optional[CheckPoint]:
             """Get last valid checkpoint"""
             
         async def recover_processing(self, checkpoint: CheckPoint) -> None:
             """Resume processing from checkpoint"""
     ```

7. **Progress Tracking**
   - Event-based progress updates:
     ```python
     class ProgressTracker:
         def __init__(self):
             self.start_time: Optional[datetime] = None
             self.progress: float = 0.0
             self.stats: Dict[str, Any] = {}
             
         def update(self, event: ProcessingEvent) -> None:
             """Update progress based on event"""
             
         def get_eta(self) -> Optional[timedelta]:
             """Calculate estimated time remaining"""
     ```

8. **Resource Management**
   - Managed cleanup through context managers:
     ```python
     class ResourceManager:
         def __init__(self, temp_manager: TempManager):
             self.temp_manager = temp_manager
             
         async def __aenter__(self) -> 'ResourceManager':
             await self.setup_resources()
             return self
             
         async def __aexit__(self, *args) -> None:
             await self.cleanup_resources()
     ```

This modern architecture provides several advantages:
- Centralized configuration management
- Real-time status updates via events
- Robust state tracking and recovery
- Clean temporary file management
- Structured error handling
- Progress monitoring
- Resource cleanup guarantees

The system maintains a clear separation of concerns while providing comprehensive monitoring and control over the processing pipeline.

## Dolby Vision Detection

Dolby Vision detection is performed using the following process:

1. **Detection Method**
   - Uses `mediainfo` to check for Dolby Vision metadata
   - Sets internal flag `IS_DOLBY_VISION=true` when detected

2. **Special Handling**
   - Activates Dolby Vision-specific encoding path
   - Uses specialized SVT-AV1 parameters for DV content
   - Maintains HDR metadata throughout the encoding process

## Encoding Paths

drapto implements two encoding paths through Python-based wrappers and state management:

### Standard Path
1. **FFmpeg Integration**
   ```python
   class FFmpegWrapper:
       """FFmpeg wrapper maintaining exact parameters"""
       def __init__(self, config: Config):
           self.config = config
           self.state_manager = StateManager()
           self.event_bus = EventBus()
           
       async def encode(self, input_file: Path, output_file: Path) -> None:
           """Execute FFmpeg encoding with state tracking"""
           params = self._build_params()
           async with FFmpegProcess(params) as process:
               while True:
                   progress = await process.get_progress()
                   if progress.complete:
                       break
                   self.state_manager.update_progress(progress)
                   self.event_bus.emit(ProcessingEvents.PROGRESS_UPDATE, progress)

   class FFmpegParams:
       """FFmpeg parameter builder with validation"""
       def __init__(self, config: EncodingConfig):
           self.config = config
           self.params: List[str] = []

       def build_command(self) -> List[str]:
           """Build FFmpeg command with exact parameters"""
           return [
               "ffmpeg",
               "-hide_banner",
               "-loglevel", "warning",
               *self._get_hwaccel_opts(),  # Hardware decode if available
               "-i", str(self.input_file),
               "-map", "0:v:0",            # Select first video stream
               "-c:v", "libsvtav1",        # SVT-AV1 codec
               "-preset", str(self.config.preset),  # Default: 6
               "-crf", str(self._get_crf()),       # Based on resolution
               "-pix_fmt", "yuv420p10le",          # 10-bit color
               "-svtav1-params",
               "tune=0:film-grain=0:film-grain-denoise=0",
               "-y",
               str(self.output_file)
           ]

       def _get_crf(self) -> int:
           """Get resolution-appropriate CRF value"""
           return {
               Resolution.SD: 25,    # ≤720p
               Resolution.HD: 25,    # ≤1080p
               Resolution.UHD: 29,   # >1080p
           }[self.resolution]
   ```

2. **Quality Control**
   - Resolution-dependent CRF values maintained:
     ```python
     CRF_VALUES = {
         "SD":  25,  # ≤720p
         "HD":  25,  # ≤1080p
         "UHD": 29,  # >1080p
     }
     ```
   - SVT-AV1 preset 6 by default
   - Film grain synthesis disabled
   - 10-bit depth processing

### Chunked Path (VMAF-based)
1. **ab-av1 Integration**
   ```python
   class AbAv1Wrapper:
       """ab-av1 wrapper maintaining exact parameters"""
       def __init__(self, config: Config):
           self.config = config
           self.state_manager = StateManager()
           self.event_bus = EventBus()
           
       async def encode_segment(self, segment: VideoSegment) -> None:
           """Encode single video segment with quality targeting"""
           for strategy in self.get_vmaf_strategies():
               try:
                   return await self._try_encode_segment(segment, strategy)
               except QualityTargetError:
                   continue
           raise EncodingError("Failed to meet quality target")

       def get_vmaf_strategies(self) -> List[VmafStrategy]:
           """Get ordered list of VMAF strategies with exact parameters"""
           return [
               # Tier 1: Default Strategy
               VmafStrategy(
                   target=93,
                   samples=3,
                   sample_duration=1,
                   vmaf_params="n_subsample=8:pool=harmonic_mean"
               ),
               # Tier 2: More Samples
               VmafStrategy(
                   target=93,
                   samples=6,
                   sample_duration=2,
                   vmaf_params="n_subsample=8:pool=harmonic_mean"
               ),
               # Tier 3: Lower VMAF
               VmafStrategy(
                   target=91,  # Reduced by 2
                   samples=6,
                   sample_duration=2,
                   vmaf_params="n_subsample=8:pool=harmonic_mean"
               )
           ]

   class VmafStrategy:
       """VMAF-based encoding strategy with exact parameters"""
       def build_command(self) -> List[str]:
           """Build ab-av1 command with exact parameters"""
           return [
               "ab-av1",
               "--input", str(self.input_file),
               "--output", str(self.output_file),
               "--encoder", "svtav1",
               "--preset", str(self.config.preset),  # Default: 6
               "--vmaf-target", str(self.target),    # Default: 93
               "--samples", str(self.samples),       # 3 or 6
               "--sample-duration", f"{self.sample_duration}s",
               "--keyint", "10s",
               "--pix-fmt", "yuv420p10le",
               "--svtav1-params",
               "tune=0:film-grain=0:film-grain-denoise=0",
               "--vmaf", self.vmaf_params,
               *self._get_vfilter_args(),  # Crop filter if enabled
               "--quiet"
           ]
   ```

2. **Quality Control**
   - Target VMAF score: 93 (default)
   - Three-tier strategy with exact parameters:
     ```python
     VMAF_STRATEGIES = [
         # Tier 1: Default Strategy
         {
             "target": 93,
             "samples": 3,
             "duration": 1,
             "vmaf_params": "n_subsample=8:pool=harmonic_mean"
         },
         # Tier 2: More Samples
         {
             "target": 93,
             "samples": 6,
             "duration": 2,
             "vmaf_params": "n_subsample=8:pool=harmonic_mean"
         },
         # Tier 3: Lower VMAF
         {
             "target": 91,  # Reduced by 2
             "samples": 6,
             "duration": 2,
             "vmaf_params": "n_subsample=8:pool=harmonic_mean"
         }
     ]
     ```

This modern implementation provides:
- Type-safe FFmpeg and ab-av1 integration
- Comprehensive state tracking and recovery
- Sophisticated quality validation
- Parallel processing with proper resource management
- Clear error handling and retry strategies
- Event-based progress monitoring
- Persistent state checkpoints

The system maintains encoding state throughout the process, enabling robust error recovery and quality assurance at every stage.

## Encoding Strategy System

drapto implements a modular strategy system for encoding, allowing different approaches based on content type:

1. **Strategy Architecture**
   ```
   encode_strategies/
   ├── strategy_base.sh      # Base strategy interface
   ├── chunked_encoding.sh   # Chunked encoding implementation
   ├── dolby_vision.sh       # Dolby Vision handling
   └── json_helper.py        # Strategy configuration
   ```

2. **Base Strategy Interface**
   Every strategy must implement these core functions:
   ```bash
   # Initialize encoding process
   initialize_encoding() {
     # Setup working directories
     # Initialize state tracking
     # Validate input parameters
   }

   # Prepare video for encoding
   prepare_video() {
     # Segment if needed
     # Configure encoding options
     # Setup quality targets
   }

   # Perform encoding
   encode_video() {
     # Execute encoding process
     # Track progress
     # Handle failures
   }

   # Finalize encoding
   finalize_encoding() {
     # Concatenate if needed
     # Cleanup temporary files
     # Validate output
   }

   # Validate strategy compatibility
   can_handle() {
     # Check if strategy can process input
     # Verify requirements are met
   }
   ```

3. **Strategy Selection**
   - Automatic selection based on:
     - Content type (Dolby Vision, HDR, SDR)
     - User preferences (chunked vs. standard)
     - Input file characteristics
     - System capabilities
   - Selection process:
     1. Check for Dolby Vision content
     2. Verify chunked encoding setting
     3. Load appropriate strategy
     4. Validate strategy compatibility

4. **Available Strategies**
   - **Standard Encoding**
     - Direct FFmpeg processing
     - CRF-based quality control
     - No segmentation
     - Suitable for most content

   - **Chunked Encoding**
     - Segments video for parallel processing
     - VMAF-based quality targeting
     - Multi-tier encoding approach
     - Better for longer content

   - **Dolby Vision**
     - Specialized parameters for DV content
     - HDR metadata preservation
     - Quality-focused encoding
     - Strict format compliance

5. **Extending the System**
   To create a new strategy:
   1. Create new script in `encode_strategies/`
   2. Source `strategy_base.sh`
   3. Implement required functions:
      ```bash
      #!/usr/bin/env bash
      source "${SCRIPT_DIR}/encode_strategies/strategy_base.sh"

      initialize_encoding() {
          # Your initialization code
      }

      prepare_video() {
          # Your preparation code
      }

      encode_video() {
          # Your encoding code
      }

      finalize_encoding() {
          # Your finalization code
      }

      can_handle() {
          # Your validation code
      }
      ```
   4. Add strategy loading in main script
   5. Update selection logic if needed

6. **Configuration System**
   The configuration system uses Python-based JSON helpers for robust state management and strategy configuration:

   ```python
   # Core configuration files
   TEMP_DIR/encode_data/
   ├── encoding.json     # Encoding state and retry strategies
   ├── segments.json     # Segment tracking
   └── progress.json     # Overall progress tracking
   ```

   **encoding.json Structure:**
   ```json
   {
     "segments": {},           # Segment processing state
     "created_at": "",        # Creation timestamp
     "updated_at": "",        # Last update timestamp
     "total_attempts": 0,     # Total encoding attempts
     "failed_segments": 0,    # Number of failed segments
     "max_attempts": 3,       # Maximum retry attempts
     "retry_strategies": [    # Available retry strategies
       {
         "name": "default",
         "description": "Default encoding settings",
         "samples": 4,
         "sample_duration": 1
       },
       {
         "name": "more_samples",
         "description": "More samples for better quality estimation",
         "samples": 6,
         "sample_duration": 2
       },
       {
         "name": "lower_vmaf",
         "description": "Lower VMAF target by 2 points",
         "samples": 6,
         "sample_duration": 2,
         "vmaf_reduction": 2
       }
     ]
   }
   ```

   **segments.json Structure:**
   ```json
   {
     "created_at": "",     # Creation timestamp
     "updated_at": "",     # Last update timestamp
     "segments": [         # List of video segments
       {
         "index": 0,
         "path": "segments/0001.mkv",
         "size": 15728640,
         "start_time": 0.0,
         "duration": 15.0,
         "created_at": "2024-01-18T00:00:00Z"
       }
     ]
   }
   ```

   **progress.json Structure:**
   ```json
   {
     "created_at": "",           # Creation timestamp
     "updated_at": "",           # Last update timestamp
     "current_segment": 0,       # Current processing segment
     "segments_completed": 0,    # Completed segments count
     "segments_failed": 0,       # Failed segments count
     "total_segments": 0        # Total number of segments
   }
   ```

   **Configuration Features:**
   1. **State Management**
      - Atomic file operations with locking
      - Retry mechanism for file operations
      - Default data initialization
      - Timestamp management

   2. **Segment Tracking**
      - Individual segment status tracking
      - Retry strategy management
      - Progress monitoring
      - Error tracking

   3. **Progress Monitoring**
      - Overall progress tracking
      - Segment completion status
      - Failure tracking
      - Performance metrics

   4. **Error Handling**
      - Detailed error tracking per segment
      - Strategy attempt history
      - Failure cause identification
      - Recovery state preservation

   5. **Performance Metrics**
      - Processing time tracking
      - Resource utilization
      - Compression statistics
      - Quality measurements

7. **Error Handling**
   - Strategy-specific error recovery
   - Fallback mechanisms
   - Progress preservation
   - State recovery support
   - Detailed error logging

## Parallel Processing

The chunked encoding path utilizes GNU Parallel for efficient parallel processing:

1. **Segmentation**
   - Input video is split into fixed-duration segments (default: 15 seconds)
   - Each segment maintains keyframe alignment
   - Segments are stored in a temporary directory
   - FFmpeg segmentation process:
     ```
     ffmpeg -i input.mkv \
       -c:v copy \
       -an \
       -f segment \
       -segment_time 15 \
       -reset_timestamps 1 \
       segments/%04d.mkv
     ```
   - Uses stream copy (`-c:v copy`) to avoid re-encoding during split
   - Segments are numbered sequentially (0001.mkv, 0002.mkv, etc.)
   - Audio is excluded during segmentation (`-an`) and processed separately

2. **Parallel Encoding**
   - GNU Parallel distributes encoding jobs across CPU cores
   - Each segment is encoded independently
   - Job allocation adapts to available system resources
   - Progress tracking for each segment

3. **Encoding Process**
   ```   Input Video → Segments → Parallel Encoding → Concatenation
   [full video] → [seg1, seg2, ...] → [parallel encode] → [final video]
   ```
4. **Failure Handling**
   - Failed segments are automatically retried
   - Three-tier strategy applied independently to each segment
   - Failed segments don't affect other parallel jobs

5. **Resource Management**
   - Automatic CPU core allocation
   - Memory usage controlled per segment
   - Disk I/O balanced across parallel jobs

6. **Validation**
   - Each segment validated after encoding
   - VMAF scores checked per segment
   - Seamless transitions verified between segments

7. **Concatenation**
   - After all segments are encoded, they are concatenated using FFmpeg
   - Process:
     1. Generate concat file listing all segments in order:
        ```
        file 'encoded_segments/0001.mkv'
        file 'encoded_segments/0002.mkv'
        ...
        ```
     2. FFmpeg concatenation command:
        ```
        ffmpeg -f concat \
          -safe 0 \
          -i concat.txt \
          -c copy \
          output.mkv
        ```
   - Uses direct stream copy for lossless joining
   - Maintains frame accuracy at segment boundaries
   - Verifies segment order using numerical prefixes
   - Validates segment integrity before concatenation

## Muxing Process

drapto employs a sophisticated track-by-track muxing system to ensure proper handling of all streams:

1. **Working Directory Structure**
   ```
   WORKING_DIR/
   ├── video.mkv          # Processed video track
   ├── audio-0.mkv        # First audio track
   ├── audio-1.mkv        # Second audio track (if present)
   ├── audio-N.mkv        # Additional audio tracks
   ├── concat.txt         # Segment list for chunked encoding
   └── temp/              # Temporary processing files
   ```

2. **Track Processing Order**
   1. **Video Track**
      - Processed first and stored as `video.mkv`
      - Uses hardware-accelerated decoding if available
      - Encoded with SVT-AV1 using selected quality settings
      - Validated before muxing

   2. **Audio Tracks**
      - Processed individually in sequence
      - Each track stored as `audio-N.mkv`
      - Channel layout analysis per track
      - Opus encoding with track-specific bitrates
      - Metadata preserved per track

   3. **Subtitle Tracks**
      - Stream copied without re-encoding
      - Format and timing preserved
      - Stored temporarily before final mux

3. **Muxing Command Construction**
   ```bash
   ffmpeg -hide_banner -loglevel warning \
     -i video.mkv \                    # Video input
     -i audio-0.mkv \                  # First audio
     -i audio-1.mkv \                  # Second audio
     -map 0:v:0 \                      # Map video
     -map 1:a:0 \                      # Map first audio
     -map 2:a:0 \                      # Map second audio
     -c copy \                         # Stream copy
     output.mkv
   ```

4. **Track Mapping**
   - Video track mapped from primary input
   - Audio tracks mapped sequentially
   - Stream indexes preserved
   - Track languages maintained
   - Track metadata retained

5. **Quality Control**
   - Pre-mux validation of all tracks
   - Stream presence verification
   - Codec validation
   - Duration checks
   - Size verification

6. **Error Handling**
   - Individual track failure recovery
   - Muxing process monitoring
   - Temporary file cleanup
   - Detailed error logging
   - Progress tracking

7. **Cleanup Process**
   - Temporary tracks removed after successful mux
   - Working directory cleaned
   - Logs preserved
   - Final output validated
   - Resource cleanup

## Audio Processing

drapto implements granular audio track processing with individual track handling and failure recovery:

1. **Track Discovery and Analysis**
   ```bash
   # Get number of audio tracks
   audio_stream_count=$("${FFPROBE}" -v error \
     -select_streams a \
     -show_entries stream=index \
     -of csv=p=0 "${input_file}" | wc -l)
   
   # For each track, analyze characteristics
   IFS=$'\n' read -r -d '' -a audio_channels < <("${FFPROBE}" -v error \
     -select_streams a \
     -show_entries stream=channels \
     -of csv=p=0 "${input_file}" && printf '\0')
   ```

2. **Channel Detection and Bitrate Assignment**
   ```bash
   # Standardize channel layouts and bitrates
   case $num_channels in
       1)  bitrate="64k"; layout="mono" ;;
       2)  bitrate="128k"; layout="stereo" ;;
       6)  bitrate="256k"; layout="5.1" ;;
       8)  bitrate="384k"; layout="7.1" ;;
       *)  print_warning "Unsupported channel count, defaulting to stereo"
           num_channels=2
           bitrate="128k"
           layout="stereo"
           ;;
   esac
   ```

3. **Per-Track Processing**
   ```bash
   # Apply consistent audio encoding settings
   audio_opts+=" -map 0:a:${stream_index}"
   audio_opts+=" -c:a:${stream_index} libopus"
   audio_opts+=" -b:a:${stream_index} ${bitrate}"
   audio_opts+=" -ac:${stream_index} ${num_channels}"
   
   # Apply consistent channel layout filter
   audio_opts+=" -filter:a:${stream_index} aformat=channel_layouts=7.1|5.1|stereo|mono"
   
   # Set consistent opus-specific options
   audio_opts+=" -application:a:${stream_index} audio"
   audio_opts+=" -frame_duration:a:${stream_index} 20"
   audio_opts+=" -vbr:a:${stream_index} on"
   audio_opts+=" -compression_level:a:${stream_index} 10"
   ```

4. **Track Metadata Preservation**
   - Language tags
   - Track titles
   - Delay information
   - Channel layout
   - Stream metadata

5. **Quality Control**
   - Track integrity verification
   - Channel count validation
   - Bitrate confirmation
   - Duration matching
   - Sample rate checking
   - Gap detection
   - Encoding parameter validation

6. **Error Handling**
   - Track-specific error codes
   - Granular error reporting
   - Track processing status tracking
   - Failure cause identification
   - Recovery action logging

7. **Performance Optimization**
   - Sequential track processing
   - Resource allocation per track
   - Progress monitoring
   - Track-specific timing
   - Memory usage control
   - I/O optimization

8. **Recovery Procedures**
   - Track processing resume
   - Partial progress preservation
   - Failed track isolation
   - Alternative processing paths
   - Quality compromise options

9. **Logging and Diagnostics**
   - Per-track log files
   - Processing statistics
   - Error condition details
   - Performance metrics
   - Quality measurements
   - Recovery attempts

## Crop Detection

Crop detection is sophisticated and content-aware:

1. **Detection Parameters**
   - Standard Content: Base threshold of 24
   - HDR Content: Dynamic threshold 128-256
   - Black level analysis for HDR content

2. **HDR Detection**
   Identifies HDR through:
   - Color transfer characteristics
   - Color primaries
   - Color space information

3. **Threshold Adjustment**
   - Analyzes black levels in HDR content
   - Adjusts threshold dynamically (1.5x measured black level)
   - Maintains bounds between 16 and 256

4. **Safety Measures**
   - Maintains original aspect ratio
   - Can be disabled with `DISABLE_CROP=true`
   - Validates crop values before applying

## Codec Usage

drapto employs a strict set of codecs:

1. **Video Codec**
   - SVT-AV1 exclusively
   - No support for other encoders (x264, x265, etc.)
   - Supports hardware acceleration for decoding
   - Maintains 10-bit depth with yuv420p10le

2. **Audio Codec**
   - libopus exclusively
   - No support for other codecs (AAC, MP3, etc.)
   - VBR mode with high compression
   - Standardized channel layouts

3. **Container Format**
   - MKV exclusively for both input and output
   - Input sources: DVD, Blu-ray, and 4K UHD Blu-ray rips
   - Preserves chapter information
   - Maintains track metadata

4. **Processing Tools**
   - FFmpeg: General processing and muxing
   - ab-av1: Chunked encoding path
   - mediainfo: Content analysis

## Validation and Quality Control

drapto implements comprehensive validation and quality control throughout the encoding process:

1. **Output File Validation**
   ```bash
   # Core validation checks
   - File existence and size verification
   - AV1 video stream presence
   - Opus audio stream count
   - Duration comparison (±1 second tolerance)
   - Stream integrity verification
   ```

2. **Size and Performance Metrics**
   - Input vs output size comparison
   - Compression ratio calculation
   - Processing time tracking
   - Resource utilization monitoring
   - Performance statistics:
     ```bash
     # Example metrics
     Input size:  15.7 GiB
     Output size: 4.2 GiB
     Reduction:   73.25%
     Encode time: 02h 15m 30s
     ```

3. **Segment Validation** (Chunked Mode)
   - Minimum segment size check (1KB)
   - Video stream presence in each segment
   - Segment ordering verification
   - Concatenation integrity checks
   - VMAF score validation per segment

4. **Track-Level Validation**
   - Video track codec verification
   - Audio track codec and count validation
   - Stream mapping verification
   - Metadata preservation checks
   - Channel layout validation

5. **Quality Metrics**
   - Resolution-specific CRF validation
   - VMAF score tracking (chunked mode)
   - Audio bitrate verification
   - Frame rate consistency
   - Color space validation

6. **Error Detection and Recovery**
   - Hardware acceleration fallback detection
   - Encoding failure identification
   - Resource exhaustion monitoring
   - Stream corruption detection
   - Process interruption handling

7. **Progress and Results Tracking**
   ```bash
   # Tracking data structure
   TEMP_DIR/encode_data/
   ├── encoded_files.txt     # Successfully processed files
   ├── encoding_times.txt    # Processing duration data
   ├── input_sizes.txt       # Original file sizes
   ├── output_sizes.txt      # Encoded file sizes
   ├── segments.json         # Segment tracking data
   └── encoding.json         # Encoding state information
   ```

8. **Final Validation Summary**
   - Overall compression statistics
   - Processing time analysis
   - Success/failure ratio
   - Resource usage summary
   - Quality metrics report

## Default Settings

Key default settings include:

1. **Video Encoding**
   ```   PRESET=6
   SVT_PARAMS="tune=0:film-grain=0:film-grain-denoise=0"
   PIX_FMT="yuv420p10le"
   ```2. **CRF Values**
   ```   CRF_SD=25
   CRF_HD=25
   CRF_UHD=29
   ```

3. **Audio Encoding**
   - Codec: libopus
   - Compression Level: 10
   - Frame Duration: 20ms

## User Configuration

Users can customize the following settings:

1. **Quality Settings**
   - `PRESET`
   - `CRF_SD`, `CRF_HD`, `CRF_UHD`

2. **Processing Options**
   - `DISABLE_CROP`
   - `ENABLE_CHUNKED_ENCODING`

3. **Directory Settings**
   - Input directory
   - Output directory
   - Log directory

4. **Hardware Acceleration**
   - Automatically detected and configured
   - Can be manually disabled

5. **Audio Settings**
   - Channel layouts
   - Bitrates per channel configuration

## Hardware Acceleration

Hardware acceleration in drapto is specifically focused on video decoding to improve input processing performance:

1. **Platform Support**
   - macOS: VideoToolbox (decoding only)
     ```bash
     # Detection via FFmpeg
     ffmpeg -hide_banner -hwaccels | grep -q videotoolbox
     ```
   - Other platforms: Currently not supported
   - Future platforms can be added by extending the detection logic

2. **Detection Process**
   - Automatic platform detection using `$OSTYPE`
   - FFmpeg capability check for supported accelerators
   - Sets `HW_ACCEL` environment variable:
     - `videotoolbox` for macOS with VideoToolbox support
     - `none` when no acceleration is available
   - Logs detection results for debugging

3. **Implementation Details**
   - Applied only during video decoding phase
   - No hardware acceleration for encoding (always uses software SVT-AV1)
   - FFmpeg options set via `HWACCEL_OPTS` variable
   - Special handling for Dolby Vision content
   - Can be manually disabled via configuration

4. **Fallback Mechanism**
   - Primary attempt: Uses configured hardware acceleration
   - On failure:
     1. Logs failure with warning message
     2. Clears hardware acceleration options
     3. Automatically retries with software decoding
     4. Maintains all other encoding parameters
   - Fallback is transparent to the user
   - Performance impact is logged

5. **Configuration**
   - Hardware acceleration state tracked in `HWACCEL_OPTS`
   - Default: Enabled if available
   - Can be disabled through user configuration
   - Status logged during initialization
   - Applied consistently across all processing modes

6. **Logging and Diagnostics**
   - Hardware support detection logged
   - Acceleration mode changes tracked
   - Fallback events recorded
   - Performance metrics maintained
   - Error conditions documented in logs

## Validation Process

1. **Output Validation**
   - Verifies file existence and size
   - Checks for AV1 video stream presence
   - Validates all audio streams are Opus
   - Compares input/output duration (allows 1 second difference)

2. **Segment Validation**
   - Minimum segment size check (1KB)
   - Verifies video stream in each segment
   - Validates segment ordering
   - Checks segment integrity before concatenation

3. **Credits Detection**
   - Smart credits handling for crop detection:
     - Movies (>1 hour): Skips last 3 minutes
     - Long content (>20 min): Skips last 1 minute
     - Medium content (>5 min): Skips last 30 seconds
   - Prevents false crop detection during credits

4. **HDR Content**
   - Detects various HDR formats:
     - SMPTE 2084 (PQ)
     - ARIB STD-B67 (HLG)
     - BT.2020
   - Adjusts crop detection thresholds:
     - Standard content: 24
     - HDR content: Dynamic 128-256 based on black levels
   - Black level analysis for optimal crop detection

## Error Recovery and Fallback Mechanisms

drapto implements a robust error recovery system, particularly focused on hardware-accelerated decoding failures and encoding issues:

1. **Hardware-Accelerated Decoding Fallback**
   - Automatically detects hardware decoding capabilities
   - Attempts hardware-accelerated decoding first (e.g., VideoToolbox on macOS)
   - On failure, gracefully falls back to software decoding
   - Maintains encoding parameters during fallback (SVT-AV1 software encoding is always used)
   - Logs hardware acceleration failures for diagnostics

2. **Decoding Recovery Process**
   - Primary attempt: Hardware-accelerated decoding
   - Final fallback: Pure software decoding
   - Encoding always uses software SVT-AV1 regardless of decoding method
   - Each stage maintains identical quality settings

3. **Error Reporting**
   - Detailed error logging for hardware decoding failures
   - Progress tracking during fallback attempts
   - Clear user feedback on decoding mode changes
   - Diagnostic information for troubleshooting

4. **Performance Implications**
   - Hardware-accelerated decoding: Faster input processing
   - Software decoding: Reduced performance but maximum compatibility
   - Encoding performance unaffected (always uses software SVT-AV1)
   - Automatic selection of optimal decoding path based on system capabilities

5. **Recovery Triggers**
   - Hardware decoder initialization failures
   - Memory allocation errors
   - Driver compatibility issues
   - Resource exhaustion
   - Codec support limitations

## Progress Tracking and Logging

drapto maintains comprehensive progress tracking and logging through a structured data file system:

1. **Tracking File Structure**
   ```
   TEMP_DIR/encode_data/
   ├── encoded_files.txt     # List of successfully processed files
   ├── encoding_times.txt    # Processing duration data
   ├── input_sizes.txt       # Original file sizes
   ├── output_sizes.txt      # Encoded file sizes
   ├── segments.json         # Segment tracking data
   └── encoding.json         # Encoding state information
   ```

2. **Progress Reporting**
   - Real-time console output:
     ```
     ========================================
              Processing video.mkv
     ========================================
     
     ✓ Input analysis complete
     ✓ Detected 2 audio tracks
     ⚠ Hardware acceleration not available
     
     ----------------------------------------
     File: video.mkv
     Input size:  15.7 GiB
     Output size: 4.2 GiB
     Reduction:   73.25%
     ----------------------------------------
     Encoding time: 02h 15m 30s
     Finished encode at 2024-01-18 14:30:00
     ----------------------------------------
     ```

3. **Log File Structure**
   - `logs/encode_[timestamp].log`: Main encoding log
     ```
     [2024-01-18T14:15:00Z] INFO: Starting encode for video.mkv
     [2024-01-18T14:15:01Z] INFO: Input analysis complete
     [2024-01-18T14:15:01Z] INFO: Detected 2 audio tracks
     [2024-01-18T14:15:02Z] WARN: Hardware acceleration not available
     [2024-01-18T14:15:03Z] INFO: Starting video encode
     [2024-01-18T14:30:00Z] INFO: Encode complete
     ```

4. **State Tracking Files**
   - `progress.json`: Overall progress tracking
     ```json
     {
       "status": "encoding",
       "created_at": "2024-01-18T14:15:00Z",
       "updated_at": "2024-01-18T14:20:00Z",
       "total_progress": 45.5,
       "segments_completed": 10,
       "segments_failed": 0,
       "current_segment": 11
     }
     ```
   - `segments.json`: Individual segment tracking
     ```json
     {
       "created_at": "2024-01-18T14:15:00Z",
       "updated_at": "2024-01-18T14:20:00Z",
       "segments": [
         {
           "index": 0,
           "path": "segments/0001.mkv",
           "size": 15728640,
           "start_time": 0.0,
           "duration": 15.0,
           "created_at": "2024-01-18T14:15:00Z"
         }
       ]
     }
     ```
   - `encoding.json`: Encoding state and retry strategies
     ```json
     {
       "segments": {
         "1": {
           "status": "completed",
           "attempts": 1,
           "last_attempt": "2024-01-18T14:16:00Z",
           "error": null
         }
       },
       "created_at": "2024-01-18T14:15:00Z",
       "updated_at": "2024-01-18T14:16:00Z",
       "total_attempts": 1,
       "failed_segments": 0
     }
     ```

5. **User-Visible Output**
   - Status Indicators:
     * ✓ (green): Success
     * ⚠ (yellow): Warning
     * ✗ (red): Error
   - Progress Updates:
     * File processing status
     * Current operation
     * Time estimates
     * Size and reduction statistics
   - Final Summary:
     ```
     ========================================
              Final Encoding Summary
     ========================================
     
     File: video.mkv
     Input size:  15.7 GiB
     Output size: 4.2 GiB
     Reduction:   73.25%
     Encode time: 02h 15m 30s
     
     ----------------------------------------
     Total files processed: 1
     Total input size:  15.7 GiB
     Total output size: 4.2 GiB
     Total reduction:   73.25%
     Total execution time: 02h 15m 30s
     ```

6. **Performance Metrics**
   - Per-file statistics:
     * Input/output sizes
     * Compression ratio
     * Processing duration
     * FPS during encoding
   - Aggregate metrics:
     * Total files processed
     * Total size reduction
     * Average processing speed
     * Overall execution time

7. **Error Reporting**
   - Detailed error messages in logs
   - User-friendly console output
   - Error state preservation
   - Recovery attempt tracking
   - Failure diagnostics

8. **Debug Information**
   - Command execution logs
   - System resource usage
   - Hardware acceleration status
   - Temporary file operations
   - Process state changes

## Process Management

drapto implements a sophisticated process management system with hierarchical control and resource monitoring:

1. **Process Hierarchy**
   ```python
   class ProcessManager:
       """Manages process lifecycle and hierarchy"""
       def __init__(self):
           self.processes: Dict[str, ManagedProcess] = {}
           self.resource_monitor = ResourceMonitor()
           self.circuit_breaker = CircuitBreaker()
           
       async def spawn_process(self, cmd: List[str], **kwargs) -> ManagedProcess:
           """Spawn new managed process"""
           process = ManagedProcess(cmd, **kwargs)
           self.processes[process.id] = process
           await self._setup_monitoring(process)
           return process

   class ManagedProcess:
       """Individual process with monitoring"""
       def __init__(self, cmd: List[str], **kwargs):
           self.id = str(uuid.uuid4())
           self.cmd = cmd
           self.start_time = datetime.now()
           self.resource_usage = ResourceUsage()
           self.status = ProcessStatus.STARTING
           
       async def __aenter__(self) -> 'ManagedProcess':
           """Setup process context"""
           await self.start()
           return self
           
       async def __aexit__(self, *args) -> None:
           """Cleanup on context exit"""
           await self.terminate(timeout=5.0)
   ```

2. **Signal Handling**
   ```python
   class SignalHandler:
       """Graceful process termination"""
       def __init__(self, process_manager: ProcessManager):
           self.process_manager = process_manager
           self.shutdown_event = asyncio.Event()
           
       def setup(self) -> None:
           """Register signal handlers"""
           for sig in (signal.SIGINT, signal.SIGTERM):
               signal.signal(sig, self._handle_shutdown)
               
       async def _handle_shutdown(self, sig: int, frame: Any) -> None:
           """Handle shutdown signals"""
           logger.info(f"Received signal {sig}, initiating shutdown")
           self.shutdown_event.set()
           await self._graceful_shutdown()
           
       async def _graceful_shutdown(self) -> None:
           """Gracefully terminate all processes"""
           for process in self.process_manager.processes.values():
               await process.terminate(timeout=5.0)
   ```

3. **Resource Tracking**
   ```python
   class ResourceMonitor:
       """System resource monitoring"""
       def __init__(self, limits: ResourceLimits):
           self.limits = limits
           self.measurements = deque(maxlen=100)
           
       async def monitor_process(self, process: ManagedProcess) -> None:
           """Monitor process resource usage"""
           while process.is_running():
               usage = await self._get_resource_usage(process)
               self.measurements.append(usage)
               if self._exceeds_limits(usage):
                   await self._handle_resource_violation(process)
               await asyncio.sleep(1.0)
               
       def _exceeds_limits(self, usage: ResourceUsage) -> bool:
           """Check if usage exceeds limits"""
           return (
               usage.memory > self.limits.max_memory or
               usage.cpu > self.limits.max_cpu or
               usage.disk_io > self.limits.max_disk_io
           )

   class ResourceLimits:
       """Resource limit configuration"""
       def __init__(self):
           self.max_memory = 8 * 1024 * 1024 * 1024  # 8GB
           self.max_cpu = 95.0  # 95% CPU
           self.max_disk_io = 100 * 1024 * 1024  # 100MB/s
           self.max_processes = 4
   ```

4. **Circuit Breaker Patterns**
   ```python
   class CircuitBreaker:
       """Process failure protection"""
       def __init__(self, threshold: int = 3, reset_time: float = 300.0):
           self.failures = 0
           self.threshold = threshold
           self.reset_time = reset_time
           self.state = CircuitState.CLOSED
           self.last_failure = None
           
       async def execute(self, process: ManagedProcess) -> None:
           """Execute with circuit breaker protection"""
           if not self._can_execute():
               raise CircuitOpenError()
               
           try:
               await process.run()
               self._handle_success()
           except ProcessError as e:
               self._handle_failure()
               raise

       def _can_execute(self) -> bool:
           """Check if execution is allowed"""
           if self.state == CircuitState.OPEN:
               if self._should_reset():
                   self.state = CircuitState.HALF_OPEN
               else:
                   return False
           return True
   ```

5. **Process Recovery**
   ```python
   class ProcessRecovery:
       """Process failure recovery"""
       def __init__(self, max_retries: int = 3):
           self.max_retries = max_retries
           self.retry_count = 0
           
       async def run_with_recovery(self, process: ManagedProcess) -> None:
           """Run process with automatic recovery"""
           while self.retry_count < self.max_retries:
               try:
                   await process.run()
                   break
               except ProcessError as e:
                   self.retry_count += 1
                   if self.retry_count >= self.max_retries:
                       raise MaxRetriesExceeded(e)
                   await self._prepare_retry(process)
   ```

6. **Resource Cleanup**
   ```python
   class ResourceCleanup:
       """Process resource cleanup"""
       def __init__(self, process_manager: ProcessManager):
           self.process_manager = process_manager
           
       async def cleanup_process(self, process: ManagedProcess) -> None:
           """Clean up process resources"""
           # Stop monitoring
           await self.process_manager.resource_monitor.stop_monitoring(process)
           
           # Release system resources
           await process.terminate(timeout=5.0)
           
           # Clean up temp files
           await self._cleanup_temp_files(process)
           
           # Update process registry
           del self.process_manager.processes[process.id]
   ```

This modern process management system provides:
- Hierarchical process control
- Graceful signal handling
- Comprehensive resource monitoring
- Circuit breaker protection
- Automatic process recovery
- Resource cleanup guarantees

The system ensures reliable process execution while preventing resource exhaustion and handling failures gracefully.

## Temporary File Management

drapto implements a sophisticated temporary file management system with state tracking and cleanup:

1. **Directory Structure**
   ```bash
   TEMP_DIR/
   ├── logs/              # Processing logs
   ├── encode_data/       # State tracking
   │   ├── encoding.json  # Encoding state
   │   ├── segments.json  # Segment tracking
   │   └── progress.json  # Progress tracking
   ├── segments/          # Video segments
   ├── encoded/           # Encoded segments
   └── working/          # Active processing
       ├── video.mkv     # Current video track
       ├── audio-*.mkv   # Audio tracks
       └── temp/         # Temporary files
   ```

2. **Cleanup Process**
   ```bash
   # Cleanup sequence
   1. Remove temporary encode files (*.temp.*, *.log, *.stats)
   2. Clean subdirectories (segments/, encoded/)
   3. Preserve segments during encoding if needed
   4. Clean data directory while preserving state
   5. Remove working directory if empty
   ```

3. **State Preservation**
   - Cleanup state tracked in JSON:
     ```json
     {
       "job_id": "job123",
       "stage": "encode",
       "error": "error message",
       "started_at": "2024-01-18T00:00:00Z",
       "completed_steps": [
         "update_status",
         "remove_temp_file:test.temp.mkv",
         "remove_dir:encoded"
       ],
       "failed_steps": [],
       "segment_index": 1
     }
     ```

4. **Error Recovery**
   - Cleanup triggered on:
     - Process interruption
     - Encoding failures
     - Resource exhaustion
     - System errors
   - State preserved for:
     - Completed segments
     - Progress tracking
     - Error diagnostics
     - Recovery points

5. **User Expectations**
   - Space Requirements:
     * Source file size × 1.5 for temporary files
     * Additional space for encoded output
     * Minimal space for logs and tracking
   - Cleanup Timing:
     * Automatic cleanup on successful completion
     * Manual cleanup may be needed after failures
     * Logs preserved for debugging
   - Recovery Options:
     * Resume from last successful segment
     * Preserve partial progress
     * Maintain encoding parameters

6. **Cleanup Commands**
   ```bash
   # Clean temporary files only
   rm -rf temp/working/* temp/segments/* temp/encoded/*

   # Preserve logs and tracking data
   rm -rf temp/working temp/segments temp/encoded

   # Full cleanup (including logs)
   rm -rf temp/*
   ```

7. **Storage Management**
   - Regular cleanup of old log files
   - Segment file management
   - Working directory maintenance
   - State file preservation
   - Resource monitoring

8. **Safety Measures**
   - Atomic file operations
   - State tracking during cleanup
   - Error logging
   - Recovery state preservation
   - Resource verification

## Directory Structure

drapto maintains a structured directory structure for efficient file management:

1. **Working Directory**
   - Contains active processing files
   - Subdirectories:
     - `video.mkv`: Current video track
     - `audio-*.mkv`: Audio tracks
     - `temp/`: Temporary files

2. **Segments Directory**
   - Contains video segments
   - Subdirectories:
     - `segments/`: Segment files

3. **Encoded Directory**
   - Contains processed video segments
   - Subdirectories:
     - `encoded/`: Encoded segment files

4. **Logs Directory**
   - Contains processing logs
   - Subdirectories:
     - `logs/`: Log files

5. **State Tracking**
   - Contains state tracking files
   - Subdirectories:
     - `encode_data/`: Encoding state and metadata
     - `segments.json`: Segment tracking data
     - `encoding.json`: Encoding state information
     - `progress.json`: Progress tracking data

6. **Cleanup Process**
   - Removes temporary segment files
   - Cleans up working directory
   - Preserves logs for debugging
   - Updates tracking files with results
   - Verifies output file integrity