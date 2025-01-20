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
20. [State Management](#state-management)
21. [Error Handling](#error-handling)
22. [Configuration](#configuration)
23. [Testing](#testing)

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

drapto uses MediaInfo for Dolby Vision detection, wrapped in a modern event-driven system:

```python
class DolbyVisionDetector:
    """Modern wrapper around MediaInfo for Dolby Vision detection"""
    
    def __init__(self, config: DolbyConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        self.error_handler = DolbyErrorHandler()
        self.validator = DolbyValidator()
        self.mediainfo = MediaInfoWrapper()  # Wrapper for mediainfo CLI
        
    async def detect(self, input_file: Path) -> Result[DolbyMetadata, DolbyError]:
        """Detect Dolby Vision using MediaInfo with state management"""
        try:
            # Initialize detection state
            state = await self.state_manager.create_dolby_state(input_file)
            
            # Emit detection start
            await self.event_bus.emit(DolbyEvents.DETECTION_START, {
                "file": str(input_file),
                "timestamp": datetime.now()
            })
            
            # Run MediaInfo detection
            mediainfo_data = await self.mediainfo.get_metadata(input_file)
            if mediainfo_data.is_err():
                return Err(mediainfo_data.unwrap_err())
                
            # Parse MediaInfo output for Dolby Vision data
            metadata = DolbyMetadata(mediainfo_data.unwrap())
            
            # Validate Dolby Vision metadata
            validation = await self.validator.validate_metadata(metadata)
            if validation.is_err():
                return Err(validation.unwrap_err())
                
            # Update state with results
            state.metadata = metadata
            await self.state_manager.update(state)
            
            # Emit detection complete
            await self.event_bus.emit(DolbyEvents.DETECTION_COMPLETE, {
                "file": str(input_file),
                "has_dolby": metadata.has_dolby_vision,
                "profile": metadata.profile
            })
            
            return Ok(metadata)
            
        except Exception as e:
            error = self.error_handler.handle_error(e, state)
            await self.event_bus.emit(DolbyEvents.DETECTION_ERROR, {
                "file": str(input_file),
                "error": str(error)
            })
            return Err(error)

class MediaInfoWrapper:
    """Modern wrapper for mediainfo CLI"""
    
    async def get_metadata(self, input_file: Path) -> Result[Dict[str, Any], MediaInfoError]:
        """Get metadata using mediainfo CLI with JSON output"""
        try:
            # Run mediainfo with JSON output format for reliable parsing
            process = await asyncio.create_subprocess_exec(
                "mediainfo",
                "--Output=JSON",
                str(input_file),
                stdout=asyncio.subprocess.PIPE,
                stderr=asyncio.subprocess.PIPE
            )
            stdout, stderr = await process.communicate()
            
            if process.returncode != 0:
                return Err(MediaInfoError(f"MediaInfo failed: {stderr.decode()}"))
                
            # Parse JSON output
            return Ok(json.loads(stdout))
            
        except Exception as e:
            return Err(MediaInfoError(f"MediaInfo error: {e}"))

class DolbyMetadata:
    """Dolby Vision metadata from MediaInfo"""
    
    def __init__(self, mediainfo_data: Dict[str, Any]):
        self.has_dolby_vision: bool = False
        self.profile: Optional[int] = None
        self.level: Optional[int] = None
        self.rpu_present: bool = False
        self.bl_present: bool = False
        self.el_present: bool = False
        self.parse_metadata(mediainfo_data)
        
    def parse_metadata(self, data: Dict[str, Any]) -> None:
        """Parse mediainfo JSON output for Dolby Vision"""
        if dv_data := data.get("DolbyVision"):
            self.has_dolby_vision = True
            self.profile = dv_data.get("dv_profile")
            self.level = dv_data.get("dv_level")
            self.rpu_present = dv_data.get("rpu_present", False)
            self.bl_present = dv_data.get("bl_present", False)
            self.el_present = dv_data.get("el_present", False)

class DolbyValidator:
    """Dolby Vision validation system"""
    
    async def validate_metadata(self, metadata: DolbyMetadata) -> Result[None, ValidationError]:
        """Validate Dolby Vision metadata"""
        # Verify profile is supported
        if metadata.has_dolby_vision and metadata.profile not in {5, 7, 8}:
            return Err(ValidationError(f"Unsupported Dolby Vision profile: {metadata.profile}"))
            
        # Verify required layers
        if metadata.has_dolby_vision:
            if not metadata.bl_present:
                return Err(ValidationError("Missing base layer"))
            if metadata.profile in {7, 8} and not metadata.el_present:
                return Err(ValidationError("Missing enhancement layer"))
            if not metadata.rpu_present:
                return Err(ValidationError("Missing RPU data"))
                
        return Ok(None)

class DolbyErrorHandler:
    """Dolby Vision error handling"""
    
    def __init__(self):
        self.retry_manager = RetryManager()
        
    def handle_error(self, error: Exception, state: DolbyState) -> DolbyError:
        """Handle detection errors"""
        if isinstance(error, MediaInfoError):
            return self._handle_mediainfo_error(error)
        if isinstance(error, ValidationError):
            return self._handle_validation_error(error)
        return DolbyError(str(error))
        
    def _handle_mediainfo_error(self, error: MediaInfoError) -> DolbyError:
        """Handle MediaInfo-specific errors"""
        if "no such file" in str(error).lower():
            return DolbyError("Input file not found")
        if "invalid data" in str(error).lower():
            return DolbyError("Invalid video data")
        return DolbyError(f"MediaInfo error: {error}")

class DolbyEvents(Enum):
    """Dolby Vision detection events"""
    DETECTION_START = "dolby_detection_start"
    DETECTION_PROGRESS = "dolby_detection_progress"
    DETECTION_COMPLETE = "dolby_detection_complete"
    DETECTION_ERROR = "dolby_detection_error"
    VALIDATION_START = "dolby_validation_start"
    VALIDATION_COMPLETE = "dolby_validation_complete"
    VALIDATION_ERROR = "dolby_validation_error"

class DolbyConfig:
    """Dolby Vision configuration"""
    
    def __init__(self):
        self.supported_profiles = {5, 7, 8}  # MEL, BL+EL, BL+EL with dynamic reshaping
        self.validation_timeout = 30.0  # seconds
        self.max_retries = 3
        self.retry_delay = 1.0
```

This implementation preserves MediaInfo's reliable Dolby Vision detection while adding modern improvements:

1. **MediaInfo Integration**
   - Uses MediaInfo's JSON output for reliable parsing
   - Proper async execution of MediaInfo CLI
   - Structured error handling for MediaInfo failures
   - Type-safe metadata parsing

2. **Enhanced Detection**
   - Extracts detailed Dolby Vision information:
     * Profile number
     * Level information
     * RPU presence
     * Base/Enhancement layer presence
   - Validates supported profiles (5, 7, 8)
   - Verifies required layer presence

3. **Error Handling**
   - Specialized MediaInfo error types
   - Proper process execution handling
   - JSON parsing error recovery
   - Invalid metadata handling

4. **State Management**
   - Tracks detection progress
   - Preserves metadata state
   - Enables recovery on failures
   - Maintains validation history

The system provides reliable Dolby Vision detection through MediaInfo while adding modern features for robustness and maintainability.

## Encoding Paths

The encoding system uses a modern Python-based architecture with comprehensive wrappers, retry strategies, and quality control:

### FFmpeg Integration

```python
class FFmpegWrapper:
    """FFmpeg wrapper maintaining exact parameters"""
    
    def __init__(self, config: FFmpegConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        
    async def encode(self, input_file: Path, output_file: Path) -> None:
        """Execute FFmpeg encoding with state tracking"""
        params = self._build_params(input_file, output_file)
        async with FFmpegProcess(params) as process:
            while True:
                progress = await process.get_progress()
                if progress.complete:
                    break
                self.state_manager.update_progress(progress)
                self.event_bus.emit(ProcessingEvents.PROGRESS_UPDATE, progress)

class AbAv1Wrapper:
    """ab-av1 wrapper maintaining exact parameters"""
    
    def __init__(self, config: AbAv1Config):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        
    async def encode_segment(self, segment: VideoSegment) -> None:
        """Encode single video segment with quality targeting"""
        params = self._build_params(segment)
        async with AbAv1Process(params) as process:
            while True:
                progress = await process.get_progress()
                if progress.complete:
                    break
                self.state_manager.update_progress(progress)
                self.event_bus.emit(ProcessingEvents.PROGRESS_UPDATE, progress)
```

This modern implementation preserves:
- Exact FFmpeg parameters (CRF values, preset, pixel format, SVT-AV1 params)
- Exact ab-av1 parameters (VMAF target, samples, duration, keyint)
- Separate configuration for each encoding path
- Clean process management without interfering with encoding parameters

### Encoding Parameters

Parameters are managed through type-safe configuration with separate FFmpeg and ab-av1 paths:

```python
@dataclass
class FFmpegConfig:
    """FFmpeg encoding configuration"""
    
    # Core parameters (preserved exactly)
    preset: int = 6
    crf: CRFConfig = CRFConfig(
        sd=25,   # ≤720p
        hd=25,   # ≤1080p
        uhd=29   # >1080p
    )
    pixel_format: str = "yuv420p10le"
    svtav1_params: str = "tune=0:film-grain=0:film-grain-denoise=0"
    
    @property
    def ffmpeg_args(self) -> List[str]:
        """Generates FFmpeg arguments preserving exact parameters"""
        return [
            "ffmpeg",
            "-hide_banner",
            "-loglevel", "warning",
            *self._get_hwaccel_opts(),  # Hardware decode if available
            "-i", str(self.input_file),
            "-map", "0:v:0",            # Select first video stream
            "-c:v", "libsvtav1",        # SVT-AV1 codec
            "-preset", str(self.preset),
            "-crf", str(self.crf.get_for_resolution(self.resolution)),
            "-pix_fmt", self.pixel_format,
            "-svtav1-params", self.svtav1_params,
            "-y",
            str(self.output_file)
        ]

@dataclass
class AbAv1Config:
    """ab-av1 encoding configuration"""
    
    # Core parameters (preserved exactly)
    preset: int = 6
    vmaf_target: int = 93
    samples: int = 3
    sample_duration: int = 1
    keyint: str = "10s"
    pixel_format: str = "yuv420p10le"
    svtav1_params: str = "tune=0:film-grain=0:film-grain-denoise=0"
    vmaf_params: str = "n_subsample=8:pool=harmonic_mean"
    
    @property
    def abav1_args(self) -> List[str]:
        """Generates ab-av1 arguments preserving exact parameters"""
        return [
            "ab-av1",
            "--input", str(self.input_file),
            "--output", str(self.output_file),
            "--encoder", "svtav1",
            "--preset", str(self.preset),
            "--vmaf-target", str(self.vmaf_target),
            "--samples", str(self.samples),
            "--sample-duration", f"{self.sample_duration}s",
            "--keyint", self.keyint,
            "--pix-fmt", self.pixel_format,
            "--svtav1-params", self.svtav1_params,
            "--vmaf", self.vmaf_params,
            *self._get_vfilter_args(),  # Crop filter if enabled
            "--quiet"
        ]

@dataclass
class EncodingConfig:
    """Top-level encoding configuration"""
    ffmpeg: FFmpegConfig
    abav1: AbAv1Config
    
    # System resource management (optional, for process management)
    process_limits: Optional[ResourceLimits] = None
```

### Retry and Recovery Strategy

Comprehensive retry handling with state preservation:

```python
class EncodingRetryStrategy:
    """Manages encoding retries with state preservation"""
    
    def __init__(self, max_retries: int = 3):
        self.max_retries = max_retries
        self.state_manager = StateManager()
        
    async def execute_with_retry(
        self, 
        segment: VideoSegment,
        encoder: FFmpegWrapper
    ) -> Result[EncodedSegment, FatalEncodingError]:
        """Executes encoding with retry logic"""
        
        for attempt in range(self.max_retries):
            # Load previous state if exists
            state = await self.state_manager.load_segment_state(segment.id)
            
            # Attempt encoding
            result = await encoder.encode_segment(segment)
            
            if result.is_ok():
                return result
                
            # Handle specific error types
            error = result.unwrap_err()
            if isinstance(error, ResourceExhaustedError):
                await self._handle_resource_error(error)
                continue
                
            if isinstance(error, QualityError):
                await self._adjust_quality_parameters(error)
                continue
                
            if error.is_fatal():
                return Err(FatalEncodingError(error))
                
        return Err(MaxRetriesExceededError())
        
    async def _handle_resource_error(self, error: ResourceExhaustedError):
        """Adjusts resource allocation based on error"""
        await self.state_manager.reduce_resource_limits()
        
    async def _adjust_quality_parameters(self, error: QualityError):
        """Adjusts encoding parameters for quality issues"""
        await self.state_manager.adjust_quality_parameters()
```

### Quality Control System

Quality control is path-specific:

```python
class FFmpegQualityControl:
    """FFmpeg path quality control (CRF-based)"""
    
    def __init__(self, config: FFmpegConfig):
        self.config = config
        self.metrics_collector = MetricsCollector()
        
    async def validate_output(self, output: Path) -> Result[QualityMetrics, QualityError]:
        """Validates encoded output quality"""
        
        # Basic output validation
        if not output.exists():
            return Err(QualityError("Output file missing"))
            
        # Collect format metrics
        metrics = await self.metrics_collector.collect_format(output)
        
        # Verify resolution-appropriate CRF was used
        crf_used = metrics.get_video_param("crf")
        resolution = metrics.get_resolution()
        expected_crf = self.config.crf.get_for_resolution(resolution)
        
        if crf_used != expected_crf:
            return Err(QualityError(f"Incorrect CRF value: {crf_used} != {expected_crf}"))
            
        # Verify SVT-AV1 parameters
        svtav1_params = metrics.get_video_param("svtav1-params")
        if svtav1_params != self.config.svtav1_params:
            return Err(QualityError("Incorrect SVT-AV1 parameters"))
            
        return Ok(metrics)

class AbAv1QualityControl:
    """ab-av1 path quality control (VMAF-based)"""
    
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
    
    def __init__(self, config: AbAv1Config):
        self.config = config
        self.vmaf_analyzer = VMAFAnalyzer()
        self.metrics_collector = MetricsCollector()
        self.current_strategy_index = 0
        
    async def validate_segment(self, segment: EncodedSegment) -> Result[QualityMetrics, QualityError]:
        """Validates encoded segment quality with retry strategies"""
        
        for strategy_index in range(len(self.VMAF_STRATEGIES)):
            self.current_strategy_index = strategy_index
            strategy = self.VMAF_STRATEGIES[strategy_index]
            
            # Configure VMAF analysis
            vmaf_scores = await self.vmaf_analyzer.analyze(
                original=segment.source_path,
                encoded=segment.output_path,
                vmaf_params=strategy["vmaf_params"],
                samples=strategy["samples"],
                sample_duration=strategy["duration"]
            )
            
            # Check if current strategy's target is met
            if vmaf_scores.mean >= strategy["target"]:
                return Ok(QualityMetrics(
                    vmaf_scores=vmaf_scores,
                    format_metrics=await self.metrics_collector.collect(segment.output_path),
                    strategy_used=strategy
                ))
            
            # Try next strategy if available
            if strategy_index < len(self.VMAF_STRATEGIES) - 1:
                continue
                
            # All strategies exhausted
            return Err(QualityError(
                f"Failed to meet VMAF target after all strategies. Best score: {vmaf_scores.mean}"
            ))
    
    def get_current_strategy(self) -> Dict[str, Any]:
        """Get current VMAF strategy parameters"""
        return self.VMAF_STRATEGIES[self.current_strategy_index]
    
    def has_next_strategy(self) -> bool:
        """Check if there are more strategies to try"""
        return self.current_strategy_index < len(self.VMAF_STRATEGIES) - 1

class AbAv1RetryStrategy:
    """Segment retry strategy for ab-av1 path"""
    
    def __init__(self, quality_control: AbAv1QualityControl):
        self.quality_control = quality_control
        self.state_manager = StateManager()
        
    async def encode_with_retry(self, segment: VideoSegment, encoder: AbAv1Wrapper) -> Result[EncodedSegment, EncodingError]:
        """Execute encoding with VMAF-based retry strategy"""
        
        while True:
            # Get current strategy
            strategy = self.quality_control.get_current_strategy()
            
            # Attempt encoding with current strategy
            result = await encoder.encode_segment(segment, strategy)
            if result.is_err():
                return result
                
            # Validate quality
            encoded_segment = result.unwrap()
            quality_result = await self.quality_control.validate_segment(encoded_segment)
            
            if quality_result.is_ok():
                return Ok(encoded_segment)
                
            # Try next strategy if available
            if self.quality_control.has_next_strategy():
                continue
                
            # All strategies exhausted
            return Err(QualityError("Failed to meet quality targets with all strategies"))

@dataclass
class CRFConfig:
    """Resolution-dependent CRF values"""
    sd: int = 25   # ≤720p
    hd: int = 25   # ≤1080p
    uhd: int = 29  # >1080p
    
    def get_for_resolution(self, resolution: Resolution) -> int:
        """Get appropriate CRF for resolution"""
        return {
            Resolution.SD: self.sd,   # ≤720p
            Resolution.HD: self.hd,   # ≤1080p
            Resolution.UHD: self.uhd  # >1080p
        }[resolution]
```

This implementation maintains:
- **FFmpeg Path**:
  - Resolution-dependent CRF values (25 for SD/HD, 29 for UHD)
  - SVT-AV1 parameter validation
  - Format and codec parameter verification
  - No VMAF checks

- **ab-av1 Path**:
  - VMAF target validation (93)
  - Sample-based quality analysis
  - Preset verification
  - Parameter validation

### Progress Monitoring

Real-time encoding progress tracking:

```python
class EncodingProgress:
    """Tracks encoding progress with detailed metrics"""
    
    def __init__(self):
        self.event_bus = EventBus()
        self.metrics_store = MetricsStore()
        
    async def track_progress(self, process: FFmpegProcess):
        """Tracks encoding progress and emits events"""
        
        async for progress in process.stream_progress():
            # Update progress metrics
            await self.metrics_store.update(progress)
            
            # Emit progress event
            await self.event_bus.emit(
                ProgressEvent(
                    timestamp=datetime.now(),
                    frames_encoded=progress.frames,
                    estimated_remaining=progress.eta,
                    current_speed=progress.speed,
                    quality_metrics=progress.quality
                )
            )
            
            # Check resource usage
            if await self._should_adjust_resources(progress):
                await self._optimize_resource_usage()
```

This modern implementation provides:
- Full async/await support for efficient processing
- Type-safe configuration management
- Comprehensive error handling and recovery
- Real-time quality monitoring
- Resource-aware processing
- Event-driven progress updates
- State preservation and recovery
- HDR content validation
- Audio synchronization checks

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

drapto implements modern parallel processing using Python's async/await with comprehensive resource management and state coordination:

```python
class ParallelProcessor:
    """Modern parallel processing with resource management"""
    
    def __init__(self, config: ProcessingConfig):
        self.config = config
        self.state_manager = StateManager()
        self.resource_monitor = ResourceMonitor()
        self.job_scheduler = JobScheduler(config.max_parallel_jobs)
        self.event_bus = EventBus()
        
    async def process_segments(self, segments: List[VideoSegment]) -> Result[List[EncodedSegment], ProcessingError]:
        """Process video segments in parallel with resource management"""
        try:
            # Initialize processing state
            state = await self.state_manager.create_parallel_state(segments)
            
            # Create segment processing tasks
            tasks = [
                self._process_segment(segment, state)
                for segment in segments
            ]
            
            # Process with controlled concurrency
            results = await self.job_scheduler.gather(tasks)
            
            # Validate all results
            if any(result.is_err() for result in results):
                return Err(self._collect_errors(results))
                
            return Ok([result.unwrap() for result in results])
            
        except Exception as e:
            return Err(ProcessingError(f"Parallel processing failed: {e}"))

    async def _process_segment(self, segment: VideoSegment, state: ParallelState) -> Result[EncodedSegment, SegmentError]:
        """Process single segment with resource monitoring"""
        try:
            # Acquire processing slot
            async with self.job_scheduler.acquire_slot() as slot:
                # Monitor resources
                async with self.resource_monitor.watch(slot):
                    # Update state
                    state.start_segment(segment)
                    await self.state_manager.update(state)
                    
                    # Process segment
                    result = await self._encode_segment(segment, slot)
                    
                    # Update state
                    state.complete_segment(segment)
                    await self.state_manager.update(state)
                    
                    return result
                    
        except Exception as e:
            state.fail_segment(segment, str(e))
            await self.state_manager.update(state)
            return Err(SegmentError(f"Segment {segment.id} failed: {e}"))

class JobScheduler:
    """Parallel job scheduling and resource management"""
    
    def __init__(self, max_concurrent: int):
        self.max_concurrent = max_concurrent
        self.semaphore = asyncio.Semaphore(max_concurrent)
        self.active_jobs: Dict[str, JobInfo] = {}
        
    async def gather(self, tasks: List[Coroutine]) -> List[Result]:
        """Execute tasks with controlled concurrency"""
        async with AsyncExitStack() as stack:
            # Setup resource monitoring
            monitor = await stack.enter_async_context(ResourceMonitor())
            
            # Process tasks
            return await asyncio.gather(
                *(self._managed_task(task, monitor) for task in tasks)
            )
    
    async def _managed_task(self, task: Coroutine, monitor: ResourceMonitor) -> Result:
        """Execute single task with resource management"""
        async with self.semaphore:
            job_id = str(uuid.uuid4())
            job_info = JobInfo(
                id=job_id,
                start_time=datetime.now(),
                resources=ResourceUsage()
            )
            self.active_jobs[job_id] = job_info
            
            try:
                # Monitor resources
                async with monitor.watch_job(job_id):
                    return await task
                    
            finally:
                del self.active_jobs[job_id]

class ParallelState:
    """Parallel processing state tracking"""
    
    def __init__(self, segments: List[VideoSegment]):
        self.total_segments = len(segments)
        self.pending = set(s.id for s in segments)
        self.active: Dict[str, SegmentProgress] = {}
        self.completed: Set[str] = set()
        self.failed: Dict[str, str] = {}  # segment_id -> error
        
    def start_segment(self, segment: VideoSegment) -> None:
        """Track segment start"""
        self.pending.remove(segment.id)
        self.active[segment.id] = SegmentProgress(
            segment=segment,
            start_time=datetime.now()
        )
        
    def complete_segment(self, segment: VideoSegment) -> None:
        """Track segment completion"""
        del self.active[segment.id]
        self.completed.add(segment.id)
        
    def fail_segment(self, segment: VideoSegment, error: str) -> None:
        """Track segment failure"""
        del self.active[segment.id]
        self.failed[segment.id] = error
        
    @property
    def progress(self) -> float:
        """Calculate overall progress"""
        return len(self.completed) / self.total_segments

class ResourceMonitor:
    """Resource monitoring and management"""
    
    def __init__(self):
        self.limits = SystemLimits()
        self.usage = SystemUsage()
        self.history = ResourceHistory()
        
    async def watch_job(self, job_id: str) -> AsyncContextManager:
        """Monitor job resource usage"""
        return ResourceWatcher(self, job_id)
        
    async def update_usage(self, job_id: str, usage: ResourceUsage) -> None:
        """Update job resource usage"""
        self.usage.update_job(job_id, usage)
        await self._check_limits()
        
    async def _check_limits(self) -> None:
        """Verify resource usage within limits"""
        if self.usage.memory > self.limits.max_memory:
            raise ResourceExhaustedError("Memory limit exceeded")
        if self.usage.cpu > self.limits.max_cpu:
            raise ResourceExhaustedError("CPU limit exceeded")

class SystemLimits:
    """System resource limits"""
    
    def __init__(self):
        self.max_memory = psutil.virtual_memory().total * 0.8  # 80% of total RAM
        self.max_cpu = psutil.cpu_count() * 100  # 100% per CPU
        self.max_disk_io = 100 * 1024 * 1024  # 100MB/s
        
    def adjust_for_parallel(self, jobs: int) -> None:
        """Adjust limits for parallel processing"""
        self.max_memory = self.max_memory / jobs
        self.max_cpu = self.max_cpu / jobs
        self.max_disk_io = self.max_disk_io / jobs

class ResourceHistory:
    """Resource usage history tracking"""
    
    def __init__(self):
        self.samples = deque(maxlen=1000)  # Last 1000 samples
        self.peak_memory = 0
        self.peak_cpu = 0
        self.peak_disk_io = 0
        
    def add_sample(self, usage: SystemUsage) -> None:
        """Record resource usage sample"""
        self.samples.append(usage)
        self.peak_memory = max(self.peak_memory, usage.memory)
        self.peak_cpu = max(self.peak_cpu, usage.cpu)
        self.peak_disk_io = max(self.peak_disk_io, usage.disk_io)
```

This modern implementation provides:

1. **Async/Await Processing**
   - Controlled parallel execution
   - Resource-aware scheduling
   - Clean task management
   - Proper error handling

2. **Resource Management**
   - Memory monitoring
   - CPU usage tracking
   - Disk I/O control
   - Per-job resource limits

3. **State Coordination**
   - Centralized state tracking
   - Progress monitoring
   - Failure tracking
   - Resource history

4. **Job Scheduling**
   - Controlled concurrency
   - Resource-based scheduling
   - Job lifecycle management
   - Clean resource cleanup

The system ensures efficient parallel processing while preventing resource exhaustion and maintaining system stability.

## Muxing Process

drapto implements a modern event-driven muxing system with comprehensive state management:

```python
class MuxingManager:
    """Modern event-driven muxing system"""
    
    def __init__(self, config: MuxConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        self.error_handler = MuxErrorHandler()
        
    async def mux_tracks(self, tracks: List[MediaTrack]) -> Result[MuxedOutput, MuxError]:
        """Mux tracks with state management"""
        try:
            # Initialize muxing state
            state = await self.state_manager.create_mux_state(tracks)
            
            # Process tracks in sequence
            for track in tracks:
                result = await self._process_track(track, state)
                if result.is_err():
                    return Err(result.unwrap_err())
                    
            # Perform final muxing
            return await self._finalize_mux(state)
            
        except Exception as e:
            return Err(self.error_handler.handle_error(e, state))

    async def _process_track(self, track: MediaTrack, state: MuxState) -> Result[ProcessedTrack, MuxError]:
        """Process single track with state tracking"""
        try:
            # Update state
            state.current_track = track
            await self.state_manager.update(state)
            
            # Emit track processing start
            await self.event_bus.emit(MuxEvents.TRACK_PROCESSING_START, {
                "track_id": track.id,
                "track_type": track.type,
                "metadata": track.metadata
            })
            
            # Process track
            processed = await self._prepare_track(track)
            
            # Update state with processed track
            state.add_processed_track(processed)
            await self.state_manager.update(state)
            
            # Emit completion
            await self.event_bus.emit(MuxEvents.TRACK_PROCESSING_COMPLETE, {
                "track_id": track.id,
                "output_path": processed.path
            })
            
            return Ok(processed)
            
        except Exception as e:
            error = self.error_handler.handle_error(e, state)
            await self.event_bus.emit(MuxEvents.TRACK_PROCESSING_ERROR, {
                "track_id": track.id,
                "error": error
            })
            return Err(error)

class MuxState:
    """Muxing process state"""
    
    def __init__(self, tracks: List[MediaTrack]):
        self.tracks = tracks
        self.current_track: Optional[MediaTrack] = None
        self.processed_tracks: Dict[str, ProcessedTrack] = {}
        self.errors: List[MuxError] = []
        self.stage = MuxStage.INITIALIZING
        
    def add_processed_track(self, track: ProcessedTrack) -> None:
        """Add processed track to state"""
        self.processed_tracks[track.id] = track
        
    @property
    def progress(self) -> float:
        """Calculate overall progress"""
        return len(self.processed_tracks) / len(self.tracks)

class MuxErrorHandler:
    """Muxing-specific error handling"""
    
    def __init__(self):
        self.retry_manager = RetryManager()
        
    def handle_error(self, error: Exception, state: MuxState) -> MuxError:
        """Handle muxing error"""
        if isinstance(error, FFmpegError):
            return self._handle_ffmpeg_error(error, state)
        if isinstance(error, TrackError):
            return self._handle_track_error(error, state)
        return MuxError(str(error))

class MuxConfig:
    """Muxing configuration"""
    
    def __init__(self):
        self.output_format = "matroska"
        self.max_retries = 3
        self.retry_delay = 1.0
        
    @property
    def ffmpeg_args(self) -> List[str]:
        """Generate FFmpeg muxing arguments"""
        return [
            "-f", self.output_format,
            "-max_interleave_delta", "0",
            "-map_metadata", "0",
            "-map_chapters", "0"
        ]

class MuxEvents(Enum):
    """Muxing process events"""
    TRACK_DETECTED = "track_detected"
    TRACK_PROCESSING_START = "track_processing_start"
    TRACK_PROCESSING_PROGRESS = "track_processing_progress"
    TRACK_PROCESSING_COMPLETE = "track_processing_complete"
    TRACK_PROCESSING_ERROR = "track_processing_error"
    MUXING_START = "muxing_start"
    MUXING_PROGRESS = "muxing_progress"
    MUXING_COMPLETE = "muxing_complete"
    MUXING_ERROR = "muxing_error"

class TrackValidator:
    """Track validation system"""
    
    async def validate_track(self, track: ProcessedTrack) -> Result[None, ValidationError]:
        """Validate processed track"""
        # Verify track exists
        if not track.path.exists():
            return Err(ValidationError("Track file missing"))
            
        # Verify track integrity
        integrity = await self._check_integrity(track)
        if not integrity.is_ok():
            return Err(ValidationError(f"Track integrity check failed: {integrity.unwrap_err()}"))
            
        # Verify track metadata
        metadata = await self._check_metadata(track)
        if not metadata.is_ok():
            return Err(ValidationError(f"Track metadata invalid: {metadata.unwrap_err()}"))
            
        return Ok(None)

class MuxingPipeline:
    """Track muxing pipeline"""
    
    def __init__(self, config: MuxConfig):
        self.config = config
        self.mux_manager = MuxingManager(config)
        self.validator = TrackValidator()
        self.temp_manager = TempManager()
        
    async def mux_file(self, input_file: Path) -> Result[Path, MuxError]:
        """Process complete muxing pipeline"""
        try:
            # Extract tracks
            tracks = await self._extract_tracks(input_file)
            
            # Process and validate tracks
            processed = await self.mux_manager.mux_tracks(tracks)
            if processed.is_err():
                return Err(processed.unwrap_err())
                
            # Validate output
            validation = await self.validator.validate_track(processed.unwrap())
            if validation.is_err():
                return Err(MuxError(f"Output validation failed: {validation.unwrap_err()}"))
                
            return Ok(processed.unwrap().path)
            
        finally:
            # Cleanup temporary files
            await self.temp_manager.cleanup()

class RetryManager:
    """Muxing retry management"""
    
    def __init__(self, max_retries: int = 3, delay: float = 1.0):
        self.max_retries = max_retries
        self.delay = delay
        self.attempts: Dict[str, int] = {}
        
    async def execute_with_retry(
        self,
        track_id: str,
        operation: Callable[[], Awaitable[Result[T, MuxError]]]
    ) -> Result[T, MuxError]:
        """Execute operation with retry logic"""
        
        attempts = self.attempts.get(track_id, 0)
        while attempts < self.max_retries:
            result = await operation()
            if result.is_ok():
                return result
                
            attempts += 1
            self.attempts[track_id] = attempts
            
            if attempts < self.max_retries:
                await asyncio.sleep(self.delay * attempts)
                continue
                
            return result
```

This modern implementation provides:

1. **Event-Driven Architecture**
   - Real-time event emission for track processing
   - Progress tracking through events
   - State updates via event bus

2. **State Management**
   - Centralized muxing state
   - Track-level progress tracking
   - Error state preservation

3. **Error Handling**
   - Specialized muxing error types
   - Track-specific error handling
   - Retry mechanisms with backoff

4. **Track Validation**
   - Comprehensive track validation
   - Metadata verification
   - Integrity checking

The system ensures reliable track muxing with:
- Proper state tracking
- Comprehensive error handling
- Event-based progress updates
- Clean error recovery

## Audio Processing

drapto implements a modern event-driven audio processing system:

```python
class AudioProcessor:
    """Modern event-driven audio processing"""
    
    # Channel layout and bitrate mapping (preserved exactly)
    CHANNEL_CONFIG = {
        1: {"bitrate": 64000,   "layout": "mono"},    # Mono
        2: {"bitrate": 128000,  "layout": "stereo"},  # Stereo
        6: {"bitrate": 256000,  "layout": "5.1"},     # 5.1
        8: {"bitrate": 384000,  "layout": "7.1"}      # 7.1
    }
    
    def __init__(self, config: AudioConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        self.error_handler = AudioErrorHandler()
        
    async def _process_track(self, track: AudioTrack, state: AudioState) -> Result[ProcessedTrack, AudioError]:
        """Process single audio track with progress tracking"""
        try:
            # Update state
            state.current_track = track
            await self.state_manager.update(state)
            
            # Get channel-specific configuration
            self.config.current_channels = track.channels
            channel_config = self.config.get_channel_config(track.channels)
            
            # Log channel configuration
            await self.event_bus.emit(AudioEvents.TRACK_CONFIG_SELECTED, {
                "track_id": track.id,
                "channels": track.channels,
                "bitrate": channel_config["bitrate"],
                "layout": channel_config["layout"]
            })
            
            # Process track with correct parameters
            processed = await self._encode_track(track)
            
            # Validate output matches configuration
            if not await self._validate_track_config(processed, channel_config):
                return Err(AudioError("Track configuration validation failed"))
            
            return Ok(processed)
            
        except Exception as e:
            error = self.error_handler.handle_error(e, state)
            await self.event_bus.emit(AudioEvents.TRACK_PROCESSING_ERROR, {
                "track_id": track.id,
                "error": error
            })
            return Err(error)

    async def _validate_track_config(self, track: ProcessedTrack, config: Dict[str, Any]) -> bool:
        """Validate track matches channel configuration"""
        metrics = await self.metrics_collector.collect_audio(track.path)
        
        return (
            metrics.channels == self.config.current_channels and
            metrics.bitrate == config["bitrate"] and
            metrics.channel_layout == config["layout"]
        )

class AudioState:
    """Audio processing state"""
    
    def __init__(self, input_file: Path):
        self.input_file = input_file
        self.tracks: List[AudioTrack] = []
        self.current_track: Optional[AudioTrack] = None
        self.processed_tracks: List[ProcessedTrack] = []
        self.errors: List[AudioError] = []
        self.stage = AudioStage.INITIALIZING
        
    def track_progress(self) -> float:
        """Calculate overall progress"""
        if not self.tracks:
            return 0.0
        return len(self.processed_tracks) / len(self.tracks)

class AudioErrorHandler:
    """Audio-specific error handling"""
    
    def __init__(self):
        self.retry_manager = RetryManager()
        
    def handle_error(self, error: Exception, state: AudioState) -> AudioError:
        """Handle audio processing error"""
        if isinstance(error, FFmpegError):
            return self._handle_ffmpeg_error(error, state)
        if isinstance(error, OpusError):
            return self._handle_opus_error(error, state)
        return AudioError(str(error))
        
    async def _handle_ffmpeg_error(self, error: FFmpegError, state: AudioState) -> AudioError:
        """Handle FFmpeg-specific errors"""
        if error.is_decoder_error():
            return AudioDecoderError(error.message)
        if error.is_encoder_error():
            return AudioEncoderError(error.message)
        return AudioError(error.message)

class AudioConfig:
    """Audio processing configuration"""
    
    # Channel layout and bitrate mapping (preserved exactly)
    CHANNEL_CONFIG = {
        1: {"bitrate": 64000,   "layout": "mono"},    # Mono
        2: {"bitrate": 128000,  "layout": "stereo"},  # Stereo
        6: {"bitrate": 256000,  "layout": "5.1"},     # 5.1
        8: {"bitrate": 384000,  "layout": "7.1"}      # 7.1
    }
    
    def __init__(self):
        self.opus_params = OpusParams(
            vbr=True,           # Variable bitrate
            mapping_family=0    # Auto-select channel mapping
        )
        self.max_retries = 3
        self.retry_delay = 1.0
        
    def get_channel_config(self, num_channels: int) -> Dict[str, Any]:
        """Get channel-specific configuration"""
        # Default to stereo if unsupported channel count
        return self.CHANNEL_CONFIG.get(num_channels, self.CHANNEL_CONFIG[2])
        
    @property
    def ffmpeg_args(self) -> List[str]:
        """Generate FFmpeg arguments for audio processing"""
        config = self.get_channel_config(self.current_channels)
        return [
            # Audio codec and bitrate
            "-c:a", "libopus",
            "-b:a", f"{config['bitrate']}",  # Channel-based bitrate
            
            # Channel configuration
            "-ac", str(self.current_channels),
            "-channel_layout", config["layout"],  # Channel layout
            
            # Opus parameters
            "-vbr", "on" if self.opus_params.vbr else "off",
            "-mapping_family", str(self.opus_params.mapping_family),
            
            # Workaround for libopus bug: force valid channel layouts
            "-af", "aformat=channel_layouts=7.1|5.1|stereo|mono"
        ]

class AudioEvents(Enum):
    """Audio processing events"""
    TRACK_DETECTED = "track_detected"
    TRACK_PROCESSING_START = "track_processing_start"
    TRACK_PROCESSING_PROGRESS = "track_processing_progress"
    TRACK_PROCESSING_COMPLETE = "track_processing_complete"
    TRACK_PROCESSING_ERROR = "track_processing_error"
    ALL_TRACKS_COMPLETE = "all_tracks_complete"
    TRACK_CONFIG_SELECTED = "track_config_selected"

class RetryManager:
    """Audio processing retry management"""
    
    def __init__(self, max_retries: int = 3, delay: float = 1.0):
        self.max_retries = max_retries
        self.delay = delay
        self.attempts: Dict[str, int] = {}
        
    async def execute_with_retry(
        self,
        track_id: str,
        operation: Callable[[], Awaitable[Result[T, AudioError]]]
    ) -> Result[T, AudioError]:
        """Execute operation with retry logic"""
        
        attempts = self.attempts.get(track_id, 0)
        while attempts < self.max_retries:
            result = await operation()
            if result.is_ok():
                return result
                
            attempts += 1
            self.attempts[track_id] = attempts
            
            if attempts < self.max_retries:
                await asyncio.sleep(self.delay * attempts)
                continue
                
            return result
```

This modern implementation provides:

1. **Event-Driven Architecture**
   - Real-time event emission for track processing
   - Progress tracking through events
   - State updates via event bus

2. **State Management**
   - Centralized audio processing state
   - Track-level progress tracking
   - Error state preservation

3. **Error Handling**
   - Specialized audio error types
   - Codec-specific error handling
   - Retry mechanisms with backoff

4. **Configuration**
   - Type-safe audio parameters
   - Opus codec configuration
   - FFmpeg argument generation

The system ensures reliable audio processing with:
- Proper state tracking
- Comprehensive error handling
- Event-based progress updates
- Clean error recovery

## Crop Detection

drapto implements a modern event-driven crop detection system with HDR awareness:

```python
class CropDetector:
    """Modern crop detection system with HDR awareness"""
    
    def __init__(self, config: CropConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        self.error_handler = CropErrorHandler()
        self.validator = CropValidator()
        
    async def detect(self, input_file: Path) -> Result[CropMetadata, CropError]:
        """Detect crop values with state management"""
        try:
            # Initialize detection state
            state = await self.state_manager.create_crop_state(input_file)
            
            # Emit detection start
            await self.event_bus.emit(CropEvents.DETECTION_START, {
                "file": str(input_file),
                "timestamp": datetime.now()
            })
            
            # Analyze black borders with HDR awareness
            crop_data = await self._analyze_borders(input_file, state)
            if crop_data.is_err():
                return Err(crop_data.unwrap_err())
                
            # Validate crop values
            validation = await self.validator.validate_crop(crop_data.unwrap())
            if validation.is_err():
                return Err(validation.unwrap_err())
                
            # Update state with results
            state.metadata = crop_data.unwrap()
            await self.state_manager.update(state)
            
            # Emit detection complete
            await self.event_bus.emit(CropEvents.DETECTION_COMPLETE, {
                "file": str(input_file),
                "crop": state.metadata.to_dict()
            })
            
            return Ok(state.metadata)
            
        except Exception as e:
            error = self.error_handler.handle_error(e, state)
            await self.event_bus.emit(CropEvents.DETECTION_ERROR, {
                "file": str(input_file),
                "error": str(error)
            })
            return Err(error)

    async def _analyze_borders(self, input_file: Path, state: CropState) -> Result[CropMetadata, CropError]:
        """Analyze video borders with HDR awareness"""
        try:
            # Detect content type and HDR characteristics
            content_info = await self._detect_content_info(input_file)
            
            # Set appropriate threshold based on content type
            threshold = self._get_threshold(content_info)
            
            # Emit threshold selection
            await self.event_bus.emit(CropEvents.THRESHOLD_SELECTED, {
                "file": str(input_file),
                "content_type": content_info.type,
                "is_hdr": content_info.is_hdr,
                "threshold": threshold
            })
            
            # Sample frames for analysis
            samples = await self._sample_frames(input_file, state)
            if samples.is_err():
                return Err(samples.unwrap_err())
                
            # Analyze black borders with content-aware threshold
            crop_values = await self._detect_black_borders(
                samples.unwrap(),
                threshold,
                state
            )
            
            return Ok(CropMetadata(
                content_type=content_info.type,
                is_hdr=content_info.is_hdr,
                threshold=threshold,
                values=crop_values,
                confidence=self._calculate_confidence(crop_values)
            ))
            
        except Exception as e:
            return Err(CropError(f"Border analysis failed: {e}"))
            
    async def _detect_content_info(self, input_file: Path) -> ContentInfo:
        """Detect content type and HDR characteristics"""
        try:
            # Get video stream info
            info = await self._get_stream_info(input_file)
            
            # Check for HDR indicators
            is_hdr = any([
                info.get("color_transfer") in {"smpte2084", "arib-std-b67"},  # PQ or HLG
                info.get("color_primaries") == "bt2020",  # BT.2020 color space
                info.get("color_space") == "bt2020nc"     # BT.2020 non-constant
            ])
            
            # Determine content type
            content_type = ContentType.HDR if is_hdr else ContentType.SDR
            
            return ContentInfo(
                type=content_type,
                is_hdr=is_hdr,
                color_transfer=info.get("color_transfer"),
                color_primaries=info.get("color_primaries"),
                color_space=info.get("color_space")
            )
            
        except Exception as e:
            raise CropError(f"Content detection failed: {e}")
            
    def _get_threshold(self, content_info: ContentInfo) -> int:
        """Get appropriate threshold based on content type"""
        if not content_info.is_hdr:
            return self.config.base_threshold  # Standard threshold for SDR
            
        # For HDR content, use dynamic threshold based on configuration
        return max(
            self.config.hdr_threshold_min,
            min(
                self.config.hdr_threshold_max,
                self.config.base_threshold * 6  # 6x multiplier for HDR
            )
        )

class ContentInfo:
    """Content type and HDR information"""
    
    def __init__(self, type: ContentType, is_hdr: bool,
                 color_transfer: Optional[str] = None,
                 color_primaries: Optional[str] = None,
                 color_space: Optional[str] = None):
        self.type = type
        self.is_hdr = is_hdr
        self.color_transfer = color_transfer
        self.color_primaries = color_primaries
        self.color_space = color_space

class ContentType(Enum):
    """Video content types"""
    SDR = "sdr"
    HDR = "hdr"

class CropConfig:
    """Crop detection configuration"""
    
    def __init__(self):
        # Detection thresholds
        self.base_threshold = 24        # Base threshold for SDR content
        self.hdr_threshold_min = 128    # Minimum HDR threshold
        self.hdr_threshold_max = 256    # Maximum HDR threshold
        
        # HDR detection
        self.hdr_multiplier = 6         # Threshold multiplier for HDR
        self.hdr_black_analyze = True   # Analyze HDR black levels
        
        # Sampling configuration
        self.samples_per_hour = 60
        self.min_samples = 10
        self.max_samples = 100
        
        # Validation thresholds
        self.min_confidence = 0.8
        self.aspect_ratio_tolerance = 0.1
        
        # Error handling
        self.max_retries = 3
        self.retry_delay = 1.0

class CropEvents(Enum):
    """Crop detection events"""
    DETECTION_START = "crop_detection_start"
    DETECTION_PROGRESS = "crop_detection_progress"
    DETECTION_COMPLETE = "crop_detection_complete"
    DETECTION_ERROR = "crop_detection_error"
    THRESHOLD_SELECTED = "threshold_selected"
    VALIDATION_START = "crop_validation_start"
    VALIDATION_COMPLETE = "crop_validation_complete"
    VALIDATION_ERROR = "crop_validation_error"
```

This modern implementation provides:

1. **HDR-Aware Detection**
   - Dynamic threshold adjustment for HDR
   - Content type detection
   - Color space analysis
   - Black level analysis

2. **Content-Based Thresholds**
   - SDR: Base threshold of 24
   - HDR: Dynamic range 128-256
   - Configurable multiplier
   - Black level analysis

3. **HDR Detection**
   - Color transfer characteristics
   - Color primaries
   - Color space information
   - HDR format detection

4. **Validation System**
   - Content-aware validation
   - HDR-specific checks
   - Confidence thresholds
   - Aspect ratio preservation

The system ensures reliable crop detection with:
- Proper HDR content handling
- Dynamic threshold adjustment
- Comprehensive validation
- Clean error recovery

## Codec Usage

drapto implements modern Python wrappers around its core codecs with comprehensive error handling and state management:

```python
class CodecManager:
    """Modern codec management system"""
    
    def __init__(self, config: CodecConfig):
        self.config = config
        self.state_manager = StateManager()
        self.event_bus = EventBus()
        self.error_handler = CodecErrorHandler()
        self.validator = CodecValidator()
        
        # Initialize codec wrappers
        self.svtav1 = SVTAV1Wrapper(config.video)
        self.opus = OpusWrapper(config.audio)

class SVTAV1Wrapper:
    """Modern SVT-AV1 codec wrapper"""
    
    def __init__(self, config: VideoConfig):
        self.config = config
        self.state = CodecState()
        self.error_handler = SVTAV1ErrorHandler()
        
    async def encode_frame(self, frame: VideoFrame) -> Result[EncodedFrame, CodecError]:
        """Encode single frame with error handling"""
        try:
            # Update state
            self.state.frame_number += 1
            self.state.last_frame = frame
            
            # Validate frame
            if not self._validate_frame(frame):
                return Err(CodecError("Invalid frame format"))
            
            # Encode with parameters
            params = self._get_encoding_params()
            encoded = await self._encode_with_params(frame, params)
            
            # Update statistics
            self.state.update_stats(encoded)
            
            return Ok(encoded)
            
        except Exception as e:
            return Err(self.error_handler.handle_error(e, self.state))
            
    def _validate_frame(self, frame: VideoFrame) -> bool:
        """Validate frame format"""
        return (
            frame.format == "yuv420p10le" and
            frame.width % 2 == 0 and
            frame.height % 2 == 0
        )
        
    def _get_encoding_params(self) -> Dict[str, Any]:
        """Get current encoding parameters"""
        return {
            "preset": self.config.preset,
            "crf": self.config.get_crf_for_resolution(self.state.resolution),
            "film-grain": 0,  # Disabled for better compression
            "film-grain-denoise": 0,
            "tune": 0  # Visual quality tuning
        }

class OpusWrapper:
    """Modern Opus codec wrapper"""
    
    def __init__(self, config: AudioConfig):
        self.config = config
        self.state = CodecState()
        self.error_handler = OpusErrorHandler()
        
    async def encode_frame(self, frame: AudioFrame) -> Result[EncodedFrame, CodecError]:
        """Encode single frame with error handling"""
        try:
            # Update state
            self.state.frame_number += 1
            self.state.last_frame = frame
            
            # Validate frame
            if not self._validate_frame(frame):
                return Err(CodecError("Invalid frame format"))
            
            # Get channel configuration
            channel_config = self.config.get_channel_config(frame.channels)
            
            # Encode with parameters
            params = self._get_encoding_params(channel_config)
            encoded = await self._encode_with_params(frame, params)
            
            # Update statistics
            self.state.update_stats(encoded)
            
            return Ok(encoded)
            
        except Exception as e:
            return Err(self.error_handler.handle_error(e, self.state))
            
    def _validate_frame(self, frame: AudioFrame) -> bool:
        """Validate frame format"""
        return (
            frame.channels in {1, 2, 6, 8} and  # Supported channel counts
            frame.sample_rate in {44100, 48000} and  # Supported rates
            frame.format == "float32"  # Required format
        )
        
    def _get_encoding_params(self, channel_config: Dict[str, Any]) -> Dict[str, Any]:
        """Get current encoding parameters"""
        return {
            "bitrate": channel_config["bitrate"],
            "vbr": True,  # Variable bitrate
            "mapping_family": 0,  # Auto channel mapping
            "application": "audio"  # High quality audio mode
        }

class CodecState:
    """Codec state tracking"""
    
    def __init__(self):
        self.frame_number: int = 0
        self.last_frame: Optional[Union[VideoFrame, AudioFrame]] = None
        self.resolution: Optional[Resolution] = None
        self.stats = EncodingStats()
        self.errors: List[CodecError] = []
        
    def update_stats(self, frame: EncodedFrame) -> None:
        """Update encoding statistics"""
        self.stats.frames_encoded += 1
        self.stats.bytes_encoded += len(frame.data)
        self.stats.encoding_time += frame.encoding_time
        
        if isinstance(frame, EncodedVideoFrame):
            self.stats.update_video_stats(frame)
        elif isinstance(frame, EncodedAudioFrame):
            self.stats.update_audio_stats(frame)

class CodecValidator:
    """Codec validation system"""
    
    async def validate_video_params(self, params: Dict[str, Any]) -> Result[None, ValidationError]:
        """Validate video encoding parameters"""
        # Verify SVT-AV1 parameters
        if not self._verify_svtav1_params(params):
            return Err(ValidationError("Invalid SVT-AV1 parameters"))
            
        # Verify resolution constraints
        if not self._verify_resolution(params):
            return Err(ValidationError("Invalid resolution"))
            
        # Verify bit depth
        if params.get("bit_depth", 10) != 10:
            return Err(ValidationError("Only 10-bit depth supported"))
            
        return Ok(None)
        
    async def validate_audio_params(self, params: Dict[str, Any]) -> Result[None, ValidationError]:
        """Validate audio encoding parameters"""
        # Verify Opus parameters
        if not self._verify_opus_params(params):
            return Err(ValidationError("Invalid Opus parameters"))
            
        # Verify channel configuration
        if not self._verify_channel_config(params):
            return Err(ValidationError("Invalid channel configuration"))
            
        # Verify bitrate constraints
        if not self._verify_bitrate(params):
            return Err(ValidationError("Invalid bitrate"))
            
        return Ok(None)

class CodecErrorHandler:
    """Codec error handling"""
    
    def __init__(self):
        self.retry_manager = RetryManager()
        
    def handle_error(self, error: Exception, state: CodecState) -> CodecError:
        """Handle codec-specific errors"""
        if isinstance(error, SVTAV1Error):
            return self._handle_svtav1_error(error, state)
        if isinstance(error, OpusError):
            return self._handle_opus_error(error, state)
        return CodecError(str(error))
        
    def _handle_svtav1_error(self, error: SVTAV1Error, state: CodecState) -> CodecError:
        """Handle SVT-AV1 specific errors"""
        if "memory allocation" in str(error):
            return ResourceError("SVT-AV1 memory allocation failed")
        if "unsupported resolution" in str(error):
            return FormatError("Unsupported resolution for SVT-AV1")
        return CodecError(f"SVT-AV1 error: {error}")
        
    def _handle_opus_error(self, error: OpusError, state: CodecState) -> CodecError:
        """Handle Opus specific errors"""
        if "invalid channel layout" in str(error):
            return FormatError("Invalid channel layout for Opus")
        if "bitrate out of range" in str(error):
            return ConfigError("Invalid bitrate for Opus")
        return CodecError(f"Opus error: {error}")

class CodecEvents(Enum):
    """Codec events"""
    FRAME_START = "frame_encoding_start"
    FRAME_COMPLETE = "frame_encoding_complete"
    FRAME_ERROR = "frame_encoding_error"
    PARAMS_UPDATED = "encoding_params_updated"
    STATS_UPDATED = "encoding_stats_updated"
    ERROR_OCCURRED = "codec_error_occurred"

class CodecConfig:
    """Codec configuration"""
    
    def __init__(self):
        self.video = VideoConfig(
            preset=6,
            crf=CRFConfig(sd=25, hd=25, uhd=29),
            pixel_format="yuv420p10le",
            svtav1_params="tune=0:film-grain=0:film-grain-denoise=0"
        )
        
        self.audio = AudioConfig(
            channel_config={
                1: {"bitrate": 64000,   "layout": "mono"},
                2: {"bitrate": 128000,  "layout": "stereo"},
                6: {"bitrate": 256000,  "layout": "5.1"},
                8: {"bitrate": 384000,  "layout": "7.1"}
            },
            opus_params=OpusParams(vbr=True, mapping_family=0)
        )
```

This modern implementation provides:

1. **Python Wrappers**
   - Type-safe codec interfaces
   - Async frame processing
   - Clean parameter management
   - Stateful encoding

2. **Error Handling**
   - Codec-specific error types
   - Detailed error context
   - Retry mechanisms
   - Resource error handling

3. **State Management**
   - Frame-level state tracking
   - Encoding statistics
   - Parameter management
   - Progress monitoring

4. **Validation**
   - Parameter validation
   - Format verification
   - Codec constraints
   - Quality checks

The system ensures reliable codec operation with:
- Clean Python interfaces
- Comprehensive error handling
- Proper state tracking
- Parameter validation

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

4. **User Expectations**
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

5. **Cleanup Commands**
   ```bash
   # Clean temporary files only
   rm -rf temp/working/* temp/segments/* temp/encoded/*

   # Preserve logs and tracking data
   rm -rf temp/working temp/segments temp/encoded

   # Full cleanup (including logs)
   rm -rf temp/*
   ```

6. **Storage Management**
   - Regular cleanup of old log files
   - Segment file management
   - Working directory maintenance
   - State file preservation
   - Resource monitoring

7. **Safety Measures**
   - Atomic file operations
   - State tracking during cleanup
   - Error logging
   - Recovery state preservation

## State Management

drapto implements a centralized state management system with event-driven updates and robust persistence:

1. **Core State System**
   ```python
   class StateManager:
       """Centralized state management"""
       def __init__(self):
           self.state = GlobalState()
           self.event_bus = EventBus()
           self.persistence = StatePersistence()
           self.recovery = StateRecovery()
           
       async def update(self, event: StateEvent) -> None:
           """Process state update event"""
           async with self.state.lock:
               self.state.apply_event(event)
               await self.persistence.save_checkpoint()
               self.event_bus.emit(StateEvents.STATE_UPDATED, self.state)

   class GlobalState:
       """Global application state"""
       def __init__(self):
           self.lock = asyncio.Lock()
           self.encoding: Optional[EncodingState] = None
           self.resources: ResourceState = ResourceState()
           self.processes: Dict[str, ProcessState] = {}
           self.errors: List[ErrorState] = []
           
       def apply_event(self, event: StateEvent) -> None:
           """Apply state mutation event"""
           if isinstance(event, EncodingEvent):
               self._update_encoding(event)
           elif isinstance(event, ResourceEvent):
               self._update_resources(event)
           elif isinstance(event, ProcessEvent):
               self._update_process(event)
   ```

2. **Event System**
   ```python
   class EventBus:
       """Event distribution system"""
       def __init__(self):
           self.subscribers: Dict[StateEvents, List[Callable]] = defaultdict(list)
           self.history = deque(maxlen=1000)  # Last 1000 events
           
       def subscribe(self, event_type: StateEvents, handler: Callable) -> None:
           """Register event handler"""
           self.subscribers[event_type].append(handler)
           
       def emit(self, event_type: StateEvents, data: Any) -> None:
           """Emit state event"""
           event = StateEvent(event_type, data, datetime.now())
           self.history.append(event)
           for handler in self.subscribers[event_type]:
               asyncio.create_task(handler(event))

   class StateEvents(Enum):
       """Core state events"""
       STATE_UPDATED = "state_updated"
       ENCODING_STARTED = "encoding_started"
       ENCODING_PROGRESS = "encoding_progress"
       ENCODING_COMPLETE = "encoding_complete"
       RESOURCE_UPDATED = "resource_updated"
       PROCESS_STARTED = "process_started"
       PROCESS_ENDED = "process_ended"
       ERROR_OCCURRED = "error_occurred"
       RECOVERY_STARTED = "recovery_started"
       RECOVERY_COMPLETE = "recovery_complete"
   ```

3. **Persistence Strategy**
   ```python
   class StatePersistence:
       """State persistence management"""
       def __init__(self, state_dir: Path):
           self.state_dir = state_dir
           self.state_file = state_dir / "state.json"
           self.checkpoint_dir = state_dir / "checkpoints"
           self.max_checkpoints = 5
           
       async def save_checkpoint(self, state: GlobalState) -> None:
           """Save state checkpoint"""
           checkpoint = StateCheckpoint(
               state=state,
               timestamp=datetime.now(),
               version=STATE_VERSION
           )
           
           # Atomic state save
           async with aiofiles.open(self.state_file.with_suffix('.tmp'), 'w') as f:
               await f.write(checkpoint.to_json())
           self.state_file.with_suffix('.tmp').rename(self.state_file)
           
           # Maintain checkpoint history
           await self._maintain_checkpoints(checkpoint)
           
       async def load_latest(self) -> Optional[GlobalState]:
           """Load most recent valid state"""
           try:
               async with aiofiles.open(self.state_file, 'r') as f:
                   data = await f.read()
               return GlobalState.from_json(data)
           except FileNotFoundError:
               return None
   ```

4. **Recovery Procedures**
   ```python
   class StateRecovery:
       """State recovery management"""
       def __init__(self, state_manager: StateManager):
           self.state_manager = state_manager
           self.recovery_strategies = self._load_strategies()
           
       async def recover(self) -> None:
           """Execute state recovery"""
           # Notify recovery start
           self.state_manager.event_bus.emit(
               StateEvents.RECOVERY_STARTED,
               {"timestamp": datetime.now()}
           )
           
           try:
               # Load latest valid state
               state = await self.state_manager.persistence.load_latest()
               if not state:
                   return
                   
               # Validate state consistency
               if not await self._validate_state(state):
                   state = await self._find_last_valid_checkpoint()
                   
               # Apply recovery strategies
               for strategy in self.recovery_strategies:
                   await strategy.apply(state)
                   
               # Restore state
               await self.state_manager.restore(state)
               
           finally:
               # Notify recovery completion
               self.state_manager.event_bus.emit(
                   StateEvents.RECOVERY_COMPLETE,
                   {"timestamp": datetime.now()}
               )

   class RecoveryStrategy:
       """Base recovery strategy"""
       async def apply(self, state: GlobalState) -> None:
           """Apply recovery actions"""
           raise NotImplementedError()

   class EncodingRecovery(RecoveryStrategy):
       """Encoding state recovery"""
       async def apply(self, state: GlobalState) -> None:
           if not state.encoding:
               return
               
           # Verify segment files
           for segment in state.encoding.segments:
               if not segment.path.exists():
                   state.encoding.segments.remove(segment)
                   
           # Adjust progress
           state.encoding.progress = len(state.encoding.completed_segments) / state.encoding.total_segments
   ```

5. **State Validation**
   ```python
   class StateValidator:
       """State validation system"""
       def __init__(self):
           self.validators: List[Validator] = [
               FileSystemValidator(),
               ProcessValidator(),
               ResourceValidator()
           ]
           
       async def validate(self, state: GlobalState) -> ValidationResult:
           """Validate state consistency"""
           results = []
           for validator in self.validators:
               result = await validator.validate(state)
               results.append(result)
               if result.is_critical_failure():
                   return ValidationResult(False, results)
           return ValidationResult(True, results)
   ```

6. **Error Reporting**
   ```python
   class ErrorReporter:
       """Error reporting system"""
       def __init__(self, error_manager: ErrorManager):
           self.error_manager = error_manager
           self.subscribers: List[ErrorSubscriber] = []
           
       async def report_error(self, context: ErrorContext) -> None:
           """Report error to all subscribers"""
           report = self._generate_report(context)
           for subscriber in self.subscribers:
               await subscriber.notify(report)
               
       def _generate_report(self, context: ErrorContext) -> ErrorReport:
           """Generate detailed error report"""
           return ErrorReport(
               context=context,
               history=self._get_relevant_history(context),
               patterns=self._get_pattern_analysis(context),
               system_state=self._get_system_state()
           )
   ```

This error handling system provides:
- Rich error context with full system state
- Flexible retry policies with backoff
- Multiple recovery strategies
- Comprehensive validation
- Pattern analysis
- Historical tracking
- Detailed reporting

The system ensures errors are handled systematically while maintaining system stability and providing insights for prevention.

## Configuration

drapto implements a schema-based configuration system with environment variable mapping and validation:

1. **Schema Definition**
   ```python
   class ConfigSchema:
       """Configuration schema with validation"""
       class Encoding(BaseModel):
           preset: int = Field(6, ge=0, le=13)
           crf: Dict[str, int] = Field({
               "sd": 25,
               "hd": 25,
               "uhd": 29
           })
           vmaf_target: int = Field(93, ge=70, le=99)
           film_grain: bool = Field(False)
           
       class Processing(BaseModel):
           chunk_size: int = Field(15, ge=1, le=300)
           parallel_jobs: int = Field(4, ge=1, le=32)
           temp_dir: Path = Field("/tmp/drapto")
           
       class Resources(BaseModel):
           max_memory: int = Field(8 * 1024 * 1024 * 1024)  # 8GB
           max_cpu: float = Field(95.0, ge=0, le=100)
           max_disk_io: int = Field(100 * 1024 * 1024)  # 100MB/s
           
       encoding: Encoding
       processing: Processing
       resources: Resources
   ```

2. **Environment Variable Mapping**
   ```python
   class EnvMapping:
       """Environment variable configuration mapping"""
       def __init__(self):
           self.mappings = {
               # Encoding settings
               "DRAPTO_PRESET": ("encoding.preset", int),
               "DRAPTO_CRF_SD": ("encoding.crf.sd", int),
               "DRAPTO_CRF_HD": ("encoding.crf.hd", int),
               "DRAPTO_CRF_UHD": ("encoding.crf.uhd", int),
               "DRAPTO_VMAF_TARGET": ("encoding.vmaf_target", int),
               "DRAPTO_FILM_GRAIN": ("encoding.film_grain", bool),
               
               # Processing settings
               "DRAPTO_CHUNK_SIZE": ("processing.chunk_size", int),
               "DRAPTO_PARALLEL_JOBS": ("processing.parallel_jobs", int),
               "DRAPTO_TEMP_DIR": ("processing.temp_dir", Path),
               
               # Resource limits
               "DRAPTO_MAX_MEMORY": ("resources.max_memory", int),
               "DRAPTO_MAX_CPU": ("resources.max_cpu", float),
               "DRAPTO_MAX_DISK_IO": ("resources.max_disk_io", int)
           }
           
       def load_from_env(self) -> Dict[str, Any]:
           """Load configuration from environment"""
           config = {}
           for env_var, (path, type_) in self.mappings.items():
               if env_var in os.environ:
                   self._set_nested(config, path, type_(os.environ[env_var]))
           return config
   ```

3. **Validation Rules**
   ```python
   class ConfigValidator:
       """Configuration validation system"""
       def __init__(self, schema: Type[ConfigSchema]):
           self.schema = schema
           self.validators = {
               "encoding": self._validate_encoding,
               "processing": self._validate_processing,
               "resources": self._validate_resources
           }
           
       async def validate(self, config: Dict[str, Any]) -> None:
           """Validate configuration against schema"""
           try:
               # Basic schema validation
               validated = self.schema(**config)
               
               # Custom validation rules
               for section, validator in self.validators.items():
                   await validator(getattr(validated, section))
                   
           except ValidationError as e:
               raise ConfigError(f"Configuration validation failed: {e}")
               
       async def _validate_encoding(self, config: ConfigSchema.Encoding) -> None:
           """Validate encoding configuration"""
           # Verify CRF values are appropriate for quality targets
           if config.vmaf_target > 95 and any(crf > 20 for crf in config.crf.values()):
               raise ValidationError("CRF values too high for VMAF target")
               
           # Check preset compatibility
           if config.film_grain and config.preset > 8:
               raise ValidationError("Film grain synthesis requires preset ≤ 8")
   ```

4. **Version Migration**
   ```python
   class ConfigMigration:
       """Configuration version migration"""
       def __init__(self):
           self.migrations = {
               1: self._migrate_v1_to_v2,
               2: self._migrate_v2_to_v3
           }
           
       async def migrate(self, config: Dict[str, Any], from_version: int) -> Dict[str, Any]:
           """Migrate configuration to latest version"""
           current_version = from_version
           current_config = config.copy()
           
           while current_version < LATEST_CONFIG_VERSION:
               migration = self.migrations[current_version]
               current_config = await migration(current_config)
               current_version += 1
               
           return current_config
           
       async def _migrate_v1_to_v2(self, config: Dict[str, Any]) -> Dict[str, Any]:
           """Migrate from v1 to v2 format"""
           # Convert old CRF format
           if "crf" in config:
               config["encoding"] = {
                   "crf": {
                       "sd": config.pop("crf"),
                       "hd": config.pop("crf"),
                       "uhd": config.pop("crf_uhd", config["crf"] + 4)
                   }
               }
           return config
   ```

5. **Configuration Loading**
   ```python
   class ConfigLoader:
       """Configuration loading system"""
       def __init__(self):
           self.schema = ConfigSchema
           self.validator = ConfigValidator(self.schema)
           self.env_mapping = EnvMapping()
           self.migration = ConfigMigration()
           
       async def load(self, config_file: Path) -> ConfigSchema:
           """Load and validate configuration"""
           # Load base configuration
           config = await self._load_file(config_file)
           
           # Load environment overrides
           env_config = self.env_mapping.load_from_env()
           config = self._merge_configs(config, env_config)
           
           # Migrate if needed
           if config.get("version", 1) < LATEST_CONFIG_VERSION:
               config = await self.migration.migrate(
                   config,
                   config.get("version", 1)
               )
           
           # Validate final configuration
           await self.validator.validate(config)
           
           return self.schema(**config)
   ```

6. **Default Configuration**
   ```python
   DEFAULT_CONFIG = {
       "version": 3,
       "encoding": {
           "preset": 6,
           "crf": {
               "sd": 25,
               "hd": 25,
               "uhd": 29
           },
           "vmaf_target": 93,
           "film_grain": False
       },
       "processing": {
           "chunk_size": 15,
           "parallel_jobs": 4,
           "temp_dir": "/tmp/drapto"
       },
       "resources": {
           "max_memory": 8 * 1024 * 1024 * 1024,  # 8GB
           "max_cpu": 95.0,
           "max_disk_io": 100 * 1024 * 1024  # 100MB/s
       }
   }
   ```

This configuration system provides:
- Schema-based configuration validation
- Environment variable overrides
- Version migration support
- Nested configuration structure
- Type safety
- Default values
- Custom validation rules

The system ensures configuration consistency while providing flexibility through environment variables and maintaining backward compatibility through migrations.

## Testing

drapto implements a comprehensive testing infrastructure with mocking, performance testing, and test data management:

1. **Test Infrastructure**
   ```python
   class TestInfrastructure:
       """Test infrastructure management"""
       def __init__(self):
           self.test_root = Path("tests")
           self.fixtures = self.test_root / "fixtures"
           self.mocks = self.test_root / "mocks"
           self.performance = self.test_root / "performance"
           self.results = self.test_root / "results"
           
       async def setup(self) -> None:
           """Initialize test environment"""
           # Create directory structure
           for path in [self.fixtures, self.mocks, self.performance, self.results]:
               path.mkdir(parents=True, exist_ok=True)
               
           # Setup test data
           await self._setup_test_data()
           
           # Initialize mocks
           await self._setup_mocks()
           
           # Configure performance monitoring
           await self._setup_performance_monitoring()

   class TestRunner:
       """Test execution management"""
       def __init__(self, infrastructure: TestInfrastructure):
           self.infrastructure = infrastructure
           self.collectors = [
               UnitTestCollector(),
               IntegrationTestCollector(),
               PerformanceTestCollector()
           ]
           
       async def run_suite(self, suite_name: str) -> TestResults:
           """Run test suite with full instrumentation"""
           suite = await self._load_suite(suite_name)
           results = TestResults()
           
           for test in suite.tests:
               # Setup test environment
               async with TestEnvironment(test) as env:
                   # Run test with monitoring
                   result = await self._run_test(test, env)
                   results.add_result(result)
                   
           return results
   ```

2. **Mocking Strategies**
   ```python
   class MockRegistry:
       """Mock management system"""
       def __init__(self):
           self.mocks: Dict[str, BaseMock] = {}
           self.patches: List[MockPatch] = []
           
       async def register_mock(self, target: str, mock: BaseMock) -> None:
           """Register mock implementation"""
           self.mocks[target] = mock
           
       async def apply_mocks(self) -> AsyncContextManager[None]:
           """Apply all registered mocks"""
           return AsyncMockContext(self._apply_all_mocks())

   class ProcessMock(BaseMock):
       """Process execution mocking"""
       def __init__(self):
           self.commands: List[str] = []
           self.responses: Dict[str, MockResponse] = {}
           
       async def execute(self, cmd: List[str]) -> MockResponse:
           """Mock process execution"""
           self.commands.append(" ".join(cmd))
           return self.responses.get(
               self._match_command(cmd),
               MockResponse(returncode=0, stdout="", stderr="")
           )

   class FFmpegMock(ProcessMock):
       """FFmpeg-specific mocking"""
       def __init__(self):
           super().__init__()
           self.default_responses = {
               "version": MockResponse(
                   returncode=0,
                   stdout="ffmpeg version 4.4",
                   stderr=""
               ),
               "probe": MockResponse(
                   returncode=0,
                   stdout=json.dumps(SAMPLE_PROBE_DATA),
                   stderr=""
               )
           }
   ```

3. **Performance Testing**
   ```python
   class PerformanceTest:
       """Performance test base"""
       def __init__(self):
           self.metrics = PerformanceMetrics()
           self.thresholds = PerformanceThresholds()
           
       async def run_benchmark(self, scenario: str) -> BenchmarkResult:
           """Run performance benchmark"""
           results = []
           for _ in range(self.iterations):
               # Setup clean environment
               async with BenchmarkEnvironment() as env:
                   # Run scenario with metrics
                   result = await self._run_scenario(scenario, env)
                   results.append(result)
                   
           return BenchmarkResult(results)

   class EncodingPerformanceTest(PerformanceTest):
       """Encoding performance testing"""
       async def test_encoding_speed(self) -> None:
           """Test encoding performance"""
           result = await self.run_benchmark("standard_encode")
           
           # Verify encoding speed
           assert result.fps_avg >= self.thresholds.min_fps
           assert result.memory_max <= self.thresholds.max_memory
           assert result.cpu_avg <= self.thresholds.max_cpu

   class ResourceMonitoringTest(PerformanceTest):
       """Resource usage testing"""
       async def test_resource_limits(self) -> None:
           """Test resource monitoring"""
           async with ResourceMonitor() as monitor:
               result = await self.run_benchmark("parallel_encode")
               
           # Verify resource constraints
           assert monitor.peak_memory <= self.limits.max_memory
           assert monitor.peak_cpu <= self.limits.max_cpu
           assert monitor.peak_disk_io <= self.limits.max_disk_io
   ```

4. **Test Data Management**
   ```python
   class TestDataManager:
       """Test data lifecycle management"""
       def __init__(self, root: Path):
           self.root = root
           self.cache = TestDataCache()
           self.generator = TestDataGenerator()
           
       async def get_test_file(self, profile: str) -> Path:
           """Get or generate test file"""
           if cached := await self.cache.get(profile):
               return cached
               
           generated = await self.generator.create(profile)
           await self.cache.store(profile, generated)
           return generated

   class TestDataGenerator:
       """Test data generation"""
       async def create(self, profile: str) -> Path:
           """Generate test video file"""
           params = TEST_PROFILES[profile]
           
           # Generate synthetic video
           video = await self._generate_video(
               duration=params.duration,
               resolution=params.resolution,
               framerate=params.framerate
           )
           
           # Add test patterns
           await self._add_test_patterns(video, params.patterns)
           
           # Add audio tracks
           await self._add_audio_tracks(video, params.audio)
           
           return video

   class TestDataCache:
       """Test data caching"""
       def __init__(self):
           self.cache_dir = Path("tests/cache")
           self.manifest = self.cache_dir / "manifest.json"
           self.max_size = 50 * 1024 * 1024 * 1024  # 50GB
           
       async def get(self, profile: str) -> Optional[Path]:
           """Get cached test file"""
           manifest = await self._load_manifest()
           if entry := manifest.get(profile):
               path = self.cache_dir / entry["filename"]
               if await self._validate_file(path, entry["hash"]):
                   return path
           return None
   ```

5. **Test Profiles**
   ```python
   TEST_PROFILES = {
       "sd_short": {
           "duration": 30,
           "resolution": (720, 480),
           "framerate": 24,
           "patterns": ["color_bars", "motion"],
           "audio": ["stereo"]
       },
       "hd_medium": {
           "duration": 300,
           "resolution": (1920, 1080),
           "framerate": 30,
           "patterns": ["color_bars", "motion", "text"],
           "audio": ["stereo", "5.1"]
       },
       "uhd_long": {
           "duration": 3600,
           "resolution": (3840, 2160),
           "framerate": 60,
           "patterns": ["color_bars", "motion", "text", "hdr"],
           "audio": ["stereo", "5.1", "7.1"]
       }
   }
   ```

6. **Integration Testing**
   ```python
   class IntegrationTest:
       """Integration test base class"""
       async def setup_test_env(self) -> None:
           """Setup integration test environment"""
           # Create isolated environment
           self.temp_dir = TemporaryDirectory()
           self.config = TestConfig(self.temp_dir.name)
           
           # Initialize components
           self.state_manager = StateManager()
           self.process_manager = ProcessManager()
           self.event_bus = EventBus()
           
           # Setup test data
           self.test_file = await self.data_manager.get_test_file("hd_medium")

   class EncodingIntegrationTest(IntegrationTest):
       """End-to-end encoding tests"""
       async def test_full_encode(self) -> None:
           """Test complete encoding process"""
           # Initialize pipeline
           pipeline = ProcessingPipeline(self.config)
           
           # Process test file
           result = await pipeline.process_file(self.test_file)
           
           # Verify output
           assert result.success
           assert await self._verify_output(result.output_file)
           assert await self._verify_metadata(result.output_file)
   ```

This testing infrastructure provides:
- Comprehensive test suite organization
- Flexible mocking system
- Performance benchmarking
- Test data generation and caching
- Integration testing support
- Resource usage validation
- Automated verification

The system ensures reliable testing while providing tools for performance optimization and regression prevention.