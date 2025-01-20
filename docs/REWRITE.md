# drapto Rewrite Plan

## Target Structure

```
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

## Core Components

### 1. Encoder Interface
```python
from abc import ABC, abstractmethod
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional, Dict, Any

@dataclass
class EncodingOptions:
    """Encoding options that apply to all encoders"""
    preset: int = 6
    pix_fmt: str = "yuv420p10le"
    crop_filter: Optional[str] = None
    hardware_accel: bool = True
    crf: Dict[str, int] = field(default_factory=lambda: {
        "sd": 25,   # ≤720p
        "hd": 25,   # ≤1080p
        "uhd": 29   # >1080p
    })
    vmaf_target: int = 93  # Only used by ChunkedEncoder

class Encoder(ABC):
    """Base encoder interface"""
    @abstractmethod
    def encode(
        self,
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        """Encode video file."""
        pass

    @abstractmethod
    def can_handle(self, input_file: Path) -> bool:
        """Check if encoder can handle input.
        
        StandardEncoder: Required for Dolby Vision content
        ChunkedEncoder: Cannot handle Dolby Vision content
        """
        pass
```

### 2. Standard Encoder
```python
class StandardEncoder(Encoder):
    """Direct FFmpeg encoding using SVT-AV1.
    
    Features:
    - Direct FFmpeg encoding for Dolby Vision content
    - SVT-AV1 configuration
    - Hardware acceleration
    - HDR/DV metadata preservation
    - Progress tracking
    
    This encoder is required for Dolby Vision content as it preserves
    DV metadata through direct FFmpeg encoding without chunking.
    """
    def __init__(
        self,
        ffmpeg: FFmpeg,
        config: DraptoConfig,
        status: StatusStream,
        hdr_handler: HDRHandler
    ):
        self.ffmpeg = ffmpeg
        self.config = config
        self.status = status
        self.hdr_handler = hdr_handler
        
    def encode(
        self,
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        """Execute standard encoding process."""
        pass

    def can_handle(self, input_file: Path) -> bool:
        """Check if encoder can handle input.
        
        Standard encoder is required for Dolby Vision content and
        can handle any input that FFmpeg supports.
        """
        pass
```

### 3. Chunked Encoder
```python
class ChunkedEncoder(Encoder):
    """VMAF-based chunked encoding using ab-av1.
    
    Features:
    - Segment-based encoding with ab-av1
    - VMAF quality analysis for optimal bitrate
    - Quality-targeted encoding
    - Parallel processing
    
    This encoder cannot be used with Dolby Vision content as
    chunked encoding would break DV metadata preservation.
    """
    def __init__(
        self,
        ffmpeg: FFmpeg,
        config: DraptoConfig,
        status: StatusStream,
        temp: TempManager,
        hdr_handler: HDRHandler
    ):
        self.ffmpeg = ffmpeg
        self.config = config
        self.status = status
        self.temp = temp
        self.hdr_handler = hdr_handler
        
    def encode(
        self,
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        """Execute chunked encoding process."""
        pass

    def can_handle(self, input_file: Path) -> bool:
        """Check if encoder can handle input.
        
        Chunked encoder requires seekable input for segmentation
        and cannot handle Dolby Vision content.
        """
        pass
```

### 4. Media Interfaces
```python
@dataclass
class HDRMetadata:
    """HDR metadata container"""
    format: str  # HDR10, HDR10+, DV
    color_primaries: str
    transfer_characteristics: str
    matrix_coefficients: str
    max_cll: Optional[int]
    max_fall: Optional[int]
    dv_profile: Optional[int]
    dv_bl_signal_compatibility_id: Optional[int]

class HDRProcessor(ABC):
    """HDR processing interface"""
    @abstractmethod
    def process_hdr(
        self,
        input_file: Path,
        metadata: HDRMetadata,
        options: EncodingOptions
    ) -> Dict[str, str]:
        """Process HDR content and return FFmpeg parameters"""
        pass
```

## Migration Plan

### Phase 0: Repository Preparation [Status: Complete]
1. Clean repository: ✅
   ```bash
   # First move files we want to keep to a temporary location
   mkdir -p tmp_preserve
   cp -r docs/* tmp_preserve/
   cp pyproject.toml tmp_preserve/
   cp -r tests/fixtures tmp_preserve/fixtures
   cp .gitignore tmp_preserve/
   cp README.md tmp_preserve/
   
   # Delete all code
   git rm -rf src/
   git rm -rf tests/
   git rm -rf scripts/
   
   # Restore preserved files to their new locations
   mkdir -p docs tests
   mv tmp_preserve/* .
   mv fixtures tests/
   rm -rf tmp_preserve
   ```

2. Create directory structure: ✅
   ```bash
   # Create package directories
   mkdir -p src/drapto/{core,encoders,media,processing,state,system,utils}
   touch src/drapto/cli.py
   touch src/drapto/{core,encoders,media,processing,state,system,utils}/__init__.py
   mkdir -p tests/{unit,integration}/drapto/{core,encoders,media,processing,state,system,utils}
   ```

### Phase 1: Core Infrastructure
1. Create core interfaces [Status: Complete]:
   - `src/drapto/core/encoder.py`: Base encoder interface and encoding options
   - `src/drapto/core/media.py`: Media file handling interface
   ```python
   @dataclass
   class HDRMetadata:
       """HDR metadata container"""
       format: str  # HDR10, HDR10+, DV
       color_primaries: str
       transfer_characteristics: str
       matrix_coefficients: str
       max_cll: Optional[int]
       max_fall: Optional[int]
       dv_profile: Optional[int]
       dv_bl_signal_compatibility_id: Optional[int]

   class HDRProcessor(ABC):
       """HDR processing interface"""
       @abstractmethod
       def process_hdr(
           self,
           input_file: Path,
           metadata: HDRMetadata,
           options: EncodingOptions
       ) -> Dict[str, str]:
           """Process HDR content and return FFmpeg parameters"""
           pass
   ```
   - `src/drapto/core/config.py`: Configuration management and validation
   - `src/drapto/core/exceptions.py`: Custom exceptions hierarchy
   - `src/drapto/core/events.py`: Event system interface
   - `src/drapto/core/status.py`: Status streaming interface
   - `src/drapto/core/temp.py`: Temporary file management interface

2. Implement system wrappers:
   - `src/drapto/system/ffmpeg.py`: FFmpeg wrapper
   ```python
   class FFmpeg:
       """FFmpeg process wrapper.
       
       Features:
       - Process management and lifecycle
       - Command building and validation
       - Progress parsing and monitoring
       - Error handling and recovery
       """
       def __init__(self, binary_path: Optional[str] = None):
           self.binary = binary_path or "ffmpeg"
           
       def probe(self, input_file: Path) -> Dict[str, Any]:
           """Get media file information."""
           pass
           
       def encode(
           self,
           input_file: Path,
           output_file: Path,
           options: Dict[str, Any]
       ) -> bool:
           """Execute FFmpeg encoding process."""
           pass
   ```
   
   - `src/drapto/system/mediainfo.py`: MediaInfo wrapper
   ```python
   class MediaInfo:
       """MediaInfo process wrapper.
       
       Features:
       - Process management and lifecycle
       - Stream information extraction
       - Format and codec detection
       - HDR metadata parsing
       """
       def analyze(self, input_file: Path) -> Dict[str, Any]:
           """Get detailed media information."""
           pass
   ```

3. Add utility functions:
   - `src/drapto/utils/logging.py`: Logging configuration
   ```python
   def setup_logging(
       log_level: str,
       log_file: Optional[Path] = None
   ) -> None:
       """Configure logging system."""
       pass
   ```
   
   - `src/drapto/utils/paths.py`: Path management
   ```python
   def ensure_dir(path: Path) -> None:
       """Ensure directory exists."""
       pass
       
   def safe_path(path: Path) -> Path:
       """Get safe path for file operations."""
       pass
   ```

### Phase 2: Encoder Implementations
1. Implement standard encoder:
   - `src/drapto/encoders/standard.py`: Standard FFmpeg-based encoder
   ```python
   class StandardEncoder(Encoder):
       """Direct FFmpeg encoding using SVT-AV1.
       
       Features:
       - Direct FFmpeg encoding for Dolby Vision content
       - SVT-AV1 configuration
       - Hardware acceleration
       - HDR/DV metadata preservation
       - Progress tracking
       
       This encoder is required for Dolby Vision content as it preserves
       DV metadata through direct FFmpeg encoding without chunking.
       """
       def __init__(
           self,
           ffmpeg: FFmpeg,
           config: DraptoConfig,
           status: StatusStream,
           hdr_handler: HDRHandler
       ):
           self.ffmpeg = ffmpeg
           self.config = config
           self.status = status
           self.hdr_handler = hdr_handler
           
       def encode(
           self,
           input_file: Path,
           output_file: Path,
           options: EncodingOptions
       ) -> bool:
           """Execute standard encoding process."""
           pass

       def can_handle(self, input_file: Path) -> bool:
           """Check if encoder can handle input.
           
           Standard encoder is required for Dolby Vision content and
           can handle any input that FFmpeg supports.
           """
           pass
   ```
   
   - `src/drapto/encoders/chunked.py`: VMAF-based chunked encoder
   ```python
   class ChunkedEncoder(Encoder):
       """VMAF-based chunked encoding using ab-av1.
       
       Features:
       - Segment-based encoding with ab-av1
       - VMAF quality analysis for optimal bitrate
       - Quality-targeted encoding
       - Parallel processing
       
       This encoder cannot be used with Dolby Vision content as
       chunked encoding would break DV metadata preservation.
       """
       def __init__(
           self,
           ffmpeg: FFmpeg,
           config: DraptoConfig,
           status: StatusStream,
           temp: TempManager,
           hdr_handler: HDRHandler
       ):
           self.ffmpeg = ffmpeg
           self.config = config
           self.status = status
           self.temp = temp
           self.hdr_handler = hdr_handler
           
       def encode(
           self,
           input_file: Path,
           output_file: Path,
           options: EncodingOptions
       ) -> bool:
           """Execute chunked encoding process."""
           pass

       def can_handle(self, input_file: Path) -> bool:
           """Check if encoder can handle input.
           
           Chunked encoder requires seekable input for segmentation
           and cannot handle Dolby Vision content.
           """
           pass
   ```

2. Add encoder selection logic:
   ```python
   class EncoderSelector:
       """Selects appropriate encoder based on content analysis"""
       
       def __init__(
           self,
           standard: StandardEncoder,
           chunked: ChunkedEncoder,
           analyzer: MediaAnalyzer
       ):
           self.standard = standard
           self.chunked = chunked
           self.analyzer = analyzer
           
       async def select_encoder(self, input_file: Path) -> Encoder:
           """Select encoder based on content analysis.
           
           Returns StandardEncoder for Dolby Vision content to preserve metadata.
           Returns ChunkedEncoder for non-DV content for quality optimization.
           """
           metadata = await self.analyzer.analyze(input_file)
           
           if metadata.has_dolby_vision:
               # DV content requires standard encoder to preserve metadata
               return self.standard
               
           # Default to chunked encoder for quality optimization
           return self.chunked
   ```

### Phase 3: Processing Pipeline
1. Implement processing pipeline:
   - `src/drapto/processing/pipeline.py`: Pipeline orchestration
   ```python
   class ProcessingPipeline:
       """Orchestrates the encoding pipeline.
       
       Features:
       - Pipeline configuration
       - Stage management
       - Segmentation control
       - Resource tracking
       - Progress monitoring
       """
       def __init__(
           self,
           config: DraptoConfig,
           status: StatusStream,
           events: EventEmitter
       ):
           self.config = config
           self.status = status
           self.events = events
           
       def process(
           self,
           input_file: Path,
           output_file: Path,
           options: EncodingOptions
       ) -> bool:
           """Execute processing pipeline."""
           pass
   ```
   
   - `src/drapto/processing/worker.py`: Worker management
   ```python
   class WorkerPool:
       """Manages worker processes.
       
       Features:
       - Process pool management
       - Resource allocation
       - Load balancing
       - Error handling
       """
       def __init__(
           self,
           config: DraptoConfig,
           status: StatusStream,
           events: EventEmitter
       ):
           self.config = config
           self.status = status
           self.events = events
           self.workers: List[Worker] = []
           
       def start(self) -> None:
           """Start worker pool."""
           pass
           
       def stop(self) -> None:
           """Stop worker pool."""
           pass
   ```
   
   - `src/drapto/processing/queue.py`: Job queue management
   ```python
   class JobQueue:
       """Manages encoding job queue.
       
       Features:
       - Job scheduling
       - Priority management
       - Resource allocation
       - State tracking
       """
       def __init__(
           self,
           config: DraptoConfig,
           status: StatusStream,
           events: EventEmitter
       ):
           self.config = config
           self.status = status
           self.events = events
           
       def add_job(
           self,
           input_file: Path,
           output_file: Path,
           options: EncodingOptions
       ) -> str:
           """Add job to queue."""
           pass
           
       def get_next_job(self) -> Optional[Dict[str, Any]]:
           """Get next job from queue."""
           pass
   ```

### Phase 4: State Management
1. Implement state management:
   - `src/drapto/state/manager.py`: State management implementation
   ```python
   class StateManager:
       """Centralized state management.
       
       Features:
       - Thread-safe state updates
       - Event emission and handling
       - State persistence and recovery
       - Resource metrics tracking
       """
       def __init__(
           self,
           config: DraptoConfig,
           events: EventEmitter,
           temp: TempManager
       ):
           self.config = config
           self.events = events
           self.temp = temp
           self._state: Dict[str, EncodingState] = {}
           self._lock = RLock()
           
       def create_job(
           self,
           input_file: Path,
           output_file: Path,
           options: EncodingOptions
       ) -> str:
           """Create new encoding job."""
           pass
           
       def update_job(self, job_id: str, **updates) -> None:
           """Update job state."""
           pass
           
       def get_job(self, job_id: str) -> EncodingState:
           """Get job state."""
           pass
           
       def save_state(self, job_id: str) -> None:
           """Persist job state to disk."""
           pass
           
       def load_state(self, job_id: str) -> None:
           """Load job state from disk."""
           pass
   ```
   
   - `src/drapto/state/progress.py`: Progress tracking implementation
   ```python
   class ProgressTracker:
       """Tracks encoding progress.
       
       Features:
       - Progress calculation and updates
       - ETA estimation and metrics
       - Resource monitoring and limits
       - Performance statistics
       """
       def __init__(
           self,
           state: StateManager,
           events: EventEmitter
       ):
           self.state = state
           self.events = events
           
       def update_progress(
           self,
           job_id: str,
           progress: float,
           stage: ProcessingStage
       ) -> None:
           """Update job progress."""
           pass
           
       def update_resources(
           self,
           job_id: str,
           cpu: float,
           memory: int,
           gpu: Optional[float] = None
       ) -> None:
           """Update resource usage."""
           pass
   ```

### Phase 5: Media Processing
1. Implement media handlers:
   - `src/drapto/media/analysis.py`: Media analysis
   ```python
   class MediaAnalyzer:
       """Analyzes media files.
       
       Features:
       - Stream detection and analysis
       - Format and codec detection
       - HDR/DV metadata detection
       - Quality assessment
       """
       def __init__(
           self,
           mediainfo: MediaInfo,
           ffmpeg: FFmpeg
       ):
           self.mediainfo = mediainfo
           self.ffmpeg = ffmpeg
           
       def analyze(self, input_file: Path) -> MediaFile:
           """Analyze media file."""
           pass
           
       def detect_hdr(self, input_file: Path) -> Optional[HDRMetadata]:
           """Detect and extract HDR metadata."""
           pass
   ```
   
   - `src/drapto/media/hdr.py`: HDR processing
   ```python
   class HDRHandler:
       """Handles HDR/DV processing.
       
       Features:
       - HDR10/HDR10+ handling
       - Dolby Vision profile handling
       - Color space conversion
       - Tone mapping when needed
       """
       def __init__(self, ffmpeg: FFmpeg):
           self.ffmpeg = ffmpeg
           
       def get_encoding_params(
           self,
           metadata: HDRMetadata,
           options: EncodingOptions
       ) -> Dict[str, str]:
           """Get FFmpeg parameters for HDR encoding."""
           pass
           
       def validate_compatibility(
           self,
           metadata: HDRMetadata,
           options: EncodingOptions
       ) -> bool:
           """Check if HDR format is compatible with encoding options."""
           pass
   ```
   
   - `src/drapto/media/audio.py`: Audio processing
   ```python
   class AudioProcessor:
       """Processes audio streams.
       
       Features:
       - Track selection and mapping
       - Channel layout handling
       - Quality control and metrics
       - Stream optimization
       """
       def process_stream(
           self,
           stream: AudioStreamInfo,
           options: EncodingOptions
       ) -> None:
           """Process audio stream."""
           pass
   ```
   
   - `src/drapto/media/subtitle.py`: Subtitle handling
   ```python
   class SubtitleHandler:
       """Handles subtitle streams.
       
       Features:
       - Track selection and mapping
       - Format preservation
       - Language detection
       - Stream optimization
       """
       def process_stream(
           self,
           stream: StreamInfo,
           options: EncodingOptions
       ) -> None:
           """Process subtitle stream."""
           pass
   ```
   
   - `src/drapto/media/muxer.py`: Stream muxing
   ```python
   class StreamMuxer:
       """Handles stream muxing.
       
       Features:
       - Track ordering and mapping
       - Container optimization
       - Metadata preservation
       - Quality validation
       """
       def mux_streams(
           self,
           input_file: Path,
           output_file: Path,
           streams: List[StreamInfo]
       ) -> bool:
           """Mux streams into output container."""
           pass
   ```

### Phase 6: Testing & Documentation
1. Add test infrastructure:
   - `tests/conftest.py`: Common test fixtures
   ```python
   @pytest.fixture
   def config() -> DraptoConfig:
       """Test configuration."""
       return DraptoConfig(
           temp_dir=Path("/tmp/drapto_test"),
           parallel_jobs=2,
           log_level="DEBUG"
       )
   
   @pytest.fixture
   def ffmpeg() -> FFmpeg:
       """FFmpeg wrapper fixture."""
       return MockFFmpeg()
           
   @pytest.fixture
   def mediainfo() -> MediaInfo:
       """MediaInfo wrapper fixture."""
       return MockMediaInfo()
   ```

2. Add unit tests:
   - Core functionality tests
   - Interface contract tests
   - Configuration validation
   - Error handling

3. Add integration tests:
   - End-to-end encoding flows
   - Pipeline orchestration
   - Resource management
   - State persistence

4. Add performance tests:
   - Encoding speed benchmarks
   - Memory usage profiling
   - Resource utilization

5. Add property tests:
   - Configuration validation
   - State transitions
   - Resource bounds
   - Error propagation

### Phase 7: CLI Implementation
1. Add CLI interface:
   - `src/drapto/cli.py`: Command line interface
   ```python
   @click.group()
   @click.option(
       "--config",
       type=click.Path(exists=True),
       help="Path to config file"
   )
   @click.option(
       "--log-level",
       type=click.Choice(["DEBUG", "INFO", "WARNING", "ERROR"]),
       default="INFO",
       help="Logging level"
   )
   def cli(config: Optional[str], log_level: str):
       """drapto video encoder."""
       setup_logging(log_level)
       
   @cli.command()
   @click.argument(
       "input_file",
       type=click.Path(exists=True)
   )
   @click.argument(
       "output_file",
       type=click.Path()
   )
   def encode(
       input_file: str,
       output_file: str
   ):
       """Encode video file."""
       pipeline = ProcessingPipeline(
           config=load_config(),
           status=ConsoleStatusStream(),
           events=EventEmitter()
       )
       
       result = pipeline.process(
           Path(input_file),
           Path(output_file),
           EncodingOptions()
       )
       
       sys.exit(0 if result else 1)
   ```

2. Add CLI tests:
   - Command validation
   - Error handling
   - Integration flows
   - Configuration loading

## Key Improvements

### 1. Type Safety
- Full type hints throughout
- Dataclass-based configurations
- Proper error types
- Interface contracts

### 2. Error Handling
```python
class EncodingError(Exception):
    """Base class for encoding errors"""
    pass

class ValidationError(EncodingError):
    """Input/output validation errors"""
    pass
```

## Testing Strategy

1. **Unit Tests**
   - Core encoder logic
   - State management
   - Process handling
   - Path management

2. **Integration Tests**
   - Full encoding pipeline
   - State persistence
   - Error recovery
   - CLI functionality

3. **Performance Tests**
   - Parallel processing
   - Memory usage
   - Disk I/O
   - CPU utilization

## Configuration

```python
@dataclass
class DraptoConfig:
    """Global configuration"""
    temp_dir: Path
    parallel_jobs: int
    log_level: str
    hardware_accel: bool
    
    @classmethod
    def from_file(cls, path: Path) -> "DraptoConfig":
        pass
    
    def save(self, path: Path) -> None:
        pass
```

## CLI Design

```python
@click.group()
def cli():
    """drapto video encoder"""
    pass

@cli.command()
@click.argument("input_path")
@click.argument("output_path")
@click.option("--chunked/--standard", default=True)
@click.option("--target-vmaf", default=93)
def encode(
    input_path: str,
    output_path: str,
    chunked: bool,
    target_vmaf: float
):
    """Encode video files"""
    pass
```

## Documentation Synchronization

When implementing changes according to this rewrite plan, maintain strict synchronization with `FUNCTIONALITY.md`:

1. **Documentation First**
   - Update `FUNCTIONALITY.md` to reflect any requiredplanned changes before implementation
   - Use the documentation as a design spec for implementation
   - Ensure all architectural decisions are documented

2. **Implementation Alignment**
   - Keep class and method names consistent between docs and code
   - Maintain the same structure and organization
   - Preserve exact parameter names and types
   - Document all events and state changes

3. **Validation Process**
   - Verify documentation matches implementation after changes
   - Update both files in the same commit
   - Use documentation review as part of code review
   - Test examples from documentation

4. **Key Areas to Sync**
   - Class hierarchies and relationships
   - Event types and payloads
   - State management structures
   - Configuration schemas
   - Error handling patterns
   - Process management flows

This synchronization ensures:
- Clear implementation guidance
- Accurate documentation
- Consistent architecture
- Reliable reference material
- Easier maintenance