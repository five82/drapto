# drapto Rewrite Plan

## Target Structure

```
src/drapto/
├── core/
│   ├── __init__.py
│   ├── encoder.py          # Core encoder interface and base classes
│   ├── config.py           # Configuration management
│   └── exceptions.py       # Custom exceptions
├── encoders/
│   ├── __init__.py
│   ├── standard.py         # Standard FFmpeg-based encoder
│   └── chunked.py          # Chunked VMAF-based encoder
├── media/
│   ├── __init__.py
│   ├── video.py           # Video stream analysis
│   ├── audio.py           # Audio stream handling
│   └── subtitle.py        # Subtitle processing
├── processing/
│   ├── __init__.py
│   ├── pipeline.py        # Processing pipeline orchestration
│   ├── analysis.py        # Media analysis (crop, DV, etc.)
│   └── validation.py      # Input/output validation
├── state/
│   ├── __init__.py
│   ├── job.py            # Job state management
│   ├── progress.py       # Progress tracking
│   └── storage.py        # State persistence
├── system/
│   ├── __init__.py
│   ├── ffmpeg.py         # FFmpeg wrapper
│   ├── mediainfo.py      # MediaInfo wrapper
│   └── process.py        # Process management
├── utils/
│   ├── __init__.py
│   ├── paths.py          # Path management
│   ├── logging.py        # Logging configuration
│   └── temp.py          # Temporary file management
└── cli.py               # Command line interface

tests/
├── unit/               # Unit tests matching src structure
├── integration/        # Integration tests
└── fixtures/          # Test data and fixtures
```

## Core Components

### 1. Encoder Interface
```python
from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path
from typing import Optional, Dict, Any

@dataclass
class EncodingOptions:
    """Encoding options that apply to all encoders"""
    preset: int = 6
    pix_fmt: str = "yuv420p10le"
    crop_filter: Optional[str] = None
    hardware_accel: bool = True

class Encoder(ABC):
    """Base encoder interface"""
    @abstractmethod
    def encode(
        self, 
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        """Execute encoding process"""
        pass

    @abstractmethod
    def can_handle(self, input_file: Path) -> bool:
        """Check if encoder can handle input"""
        pass
```

### 2. Standard Encoder
```python
class StandardEncoder(Encoder):
    """Direct FFmpeg encoding using SVT-AV1"""
    def __init__(self, ffmpeg: FFmpeg):
        self.ffmpeg = ffmpeg
        
    def encode(
        self,
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        # Implement existing standard encoding path
        # Uses direct FFmpeg with SVT-AV1
        pass
```

### 3. Chunked Encoder
```python
class ChunkedEncoder(Encoder):
    """VMAF-based chunked encoding using ab-av1"""
    def __init__(
        self,
        ffmpeg: FFmpeg,
        temp_dir: Path,
        target_vmaf: float = 93
    ):
        self.ffmpeg = ffmpeg
        self.temp_dir = temp_dir
        self.target_vmaf = target_vmaf
        
    def encode(
        self,
        input_file: Path,
        output_file: Path,
        options: EncodingOptions
    ) -> bool:
        # Implement existing chunked encoding path
        # Preserves all VMAF-based logic
        pass
```

## Migration Plan

### Phase 0: Repository Preparation
1. Clean repository:
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

2. Create directory structure:
   ```bash
   # Create package directories
   mkdir -p src/drapto/{core,encoders,media,processing,state,system,utils}
   touch src/drapto/cli.py
   touch src/drapto/{core,encoders,media,processing,state,system,utils}/__init__.py
   mkdir -p tests/{unit,integration}/drapto/{core,encoders,media,processing,state,system,utils}
   ```

### Phase 1: Core Infrastructure
1. Create core interfaces:
   - `src/drapto/core/encoder.py`: Base encoder interface
   - `src/drapto/core/media.py`: Media file handling interface
   - `src/drapto/core/config.py`: Configuration management
   - `src/drapto/core/errors.py`: Custom exceptions
   - `src/drapto/core/temp.py`: Temporary file/directory management
   ```python
   @dataclass
   class ConfigSchema:
       """Configuration schema with validation"""
       field_name: str
       field_type: type
       required: bool
       validator: Callable[[Any], bool]
       default: Any = None
   
   class ConfigValidator:
       """Validate configuration against schema"""
       def validate(self, config: Any) -> list[str]: ...
       def apply_defaults(self, config: Any) -> Any: ...
   
   @dataclass
   class DraptoConfig:
       """Enhanced configuration with validation"""
       temp_dir: Path
       parallel_jobs: int
       log_level: str
       hardware_accel: bool
       
       _schema: ClassVar[dict[str, ConfigSchema]] = {
           'temp_dir': ConfigSchema(
               'temp_dir', Path, True,
               lambda p: p.parent.exists(),
               Path('/tmp/drapto')
           ),
           'parallel_jobs': ConfigSchema(
               'parallel_jobs', int, True,
               lambda n: 1 <= n <= cpu_count(),
               cpu_count() // 2
           )
       }
   ```
   - `src/drapto/core/events.py`: Event system
   ```python
   class EventEmitter:
       """Base event system for component communication"""
       def emit(self, event: str, data: Any) -> None: ...
       def on(self, event: str, callback: Callable[[Any], None]) -> None: ...
       def off(self, event: str, callback: Callable[[Any], None]) -> None: ...
   ```
   - `src/drapto/core/status.py`: Status streaming
   ```python
   class StatusStream:
       """Real-time status and progress updates"""
       def update_progress(self, percent: float, message: str) -> None: ...
       def update_stage(self, stage: str, details: dict) -> None: ...
       def error(self, error: str, details: dict) -> None: ...
   ```
   - `src/drapto/core/errors.py`: Custom exceptions
   - `src/drapto/core/temp.py`: Temporary file/directory management
   ```python
   class TempManager:
       """Manages temporary files and directories
       
       Directory structure:
       TEMP_DIR/
       ├── logs/           # Processing logs
       ├── encode_data/    # Encoding state and metadata
       ├── segments/       # Video segments for chunked encoding
       ├── encoded/        # Encoded segments
       └── working/        # Temporary processing files
       """
   ```

2. Implement system wrappers:
   - `src/drapto/system/ffmpeg.py`: FFmpeg wrapper
   - `src/drapto/system/mediainfo.py`: MediaInfo wrapper
   - `src/drapto/system/process.py`: Process management
   - `src/drapto/system/signals.py`: Signal handling
   - `src/drapto/system/abav1.py`: ab-av1 wrapper
   - `src/drapto/system/validation.py`: Input/output validation

3. Add utility functions:
   - `src/drapto/utils/logging.py`: Logging setup
   - `src/drapto/utils/paths.py`: Path handling
   - `src/drapto/utils/validation.py`: Input validation
   - `src/drapto/utils/tracking.py`: File tracking and statistics
   ```python
   class FileTracker:
       """Tracks processed files and statistics
       
       Manages:
       - encoded_files.txt: List of processed files
       - encoding_times.txt: Processing duration data
       - input_sizes.txt: Original file sizes
       - output_sizes.txt: Encoded file sizes
       """
   ```
   - `src/drapto/utils/terminal.py`: Terminal capabilities
   ```python
   class TerminalCapabilities:
       """Detect and manage terminal capabilities"""
       def __init__(self):
           self.colors = self._detect_colors()
           self.width = self._detect_width()
           self.interactive = self._is_interactive()
   
   class OutputFormatter:
       """Platform-independent output formatting"""
       def __init__(self, capabilities: TerminalCapabilities):
           self.caps = capabilities
           
       def format(self, text: str, style: dict) -> str:
           """Format text based on terminal capabilities"""
           
       def progress_bar(self, percent: float, width: int) -> str:
           """Create progress bar based on terminal width"""
   ```

### Phase 2: Basic Encoding
1. Implement standard encoder:
   - `src/drapto/encoders/standard.py`: Basic encoding implementation
   - `src/drapto/encoders/options.py`: Encoding options dataclass
   - `src/drapto/encoders/hardware.py`: Hardware acceleration support
   - `src/drapto/encoders/dolby.py`: Dolby Vision handling
   ```python
   @dataclass
   class EncodingOptions:
       """Encoding configuration
       
       Resolution-based CRF values:
       - SD  (≤720p):  CRF 25
       - HD  (≤1080p): CRF 25
       - UHD (>1080p): CRF 29
       """
       preset: int = 6
       film_grain: int = 0
       film_grain_denoise: int = 0
       tune: int = 0
       keyint: int = 240  # 10s at 24fps
   ```

2. Add media analysis:
   - `src/drapto/media/analysis.py`: Video/audio analysis
   - `src/drapto/media/metadata.py`: Media metadata handling
   - `src/drapto/media/dolby.py`: Dolby Vision detection
   ```python
   class DolbyVisionDetector:
       """Detects and validates Dolby Vision content
       
       Uses mediainfo to:
       1. Check for DV metadata
       2. Validate DV profile
       3. Extract HDR metadata
       """
   ```

### Phase 3: Chunked Encoding
1. Implement chunked encoder:
   - `src/drapto/encoders/chunked.py`: VMAF-based chunked encoding
   - `src/drapto/processing/segmentation.py`: Video segmentation
   - `src/drapto/processing/vmaf.py`: VMAF calculation and analysis

2. Add parallel processing:
   - `src/drapto/processing/worker.py`: Worker process management
   - `src/drapto/processing/queue.py`: Job queue handling

### Phase 4: State Management
1. Implement centralized state:
   - `src/drapto/state/types.py`: State data structures
   ```python
   @dataclass
   class EncodingState:
       """Complete encoding state"""
       # Core state
       job_id: str
       status: EncodingStatus
       stage: EncodingStage
       error: Optional[str] = None
       
       # Progress state
       progress: float = 0.0
       current_segment: Optional[int] = None
       total_segments: Optional[int] = None
       eta_seconds: Optional[float] = None
       
       # File state
       input_file: Path
       output_file: Path
       temp_files: list[Path] = field(default_factory=list)
       
       # Resource state
       processes: list[int] = field(default_factory=list)  # PIDs
       memory_usage: Optional[int] = None
       cpu_usage: Optional[float] = None
       
       # Quality metrics
       vmaf_scores: list[float] = field(default_factory=list)
       average_vmaf: Optional[float] = None
   ```

   - `src/drapto/state/manager.py`: State management
   ```python
   class StateManager:
       """Centralized state management
       
       Features:
       1. Single source of truth
       2. Thread-safe state updates
       3. Event emission on state changes
       4. Automatic persistence
       5. Recovery from crashes
       """
       def __init__(self, event_emitter: EventEmitter):
           self._state: dict[str, EncodingState] = {}
           self._lock = RLock()
           self._events = event_emitter
       
       def create_job(self, input_file: Path, output_file: Path) -> str:
           """Create new encoding job"""
           with self._lock:
               job_id = str(uuid4())
               state = EncodingState(
                   job_id=job_id,
                   status=EncodingStatus.CREATED,
                   stage=EncodingStage.INIT,
                   input_file=input_file,
                   output_file=output_file
               )
               self._state[job_id] = state
               self._events.emit("job_created", job_id)
               return job_id
       
       def update_job(self, job_id: str, **updates) -> None:
           """Update job state"""
           with self._lock:
               if job_id not in self._state:
                   raise StateError(f"No such job: {job_id}")
               
               old_state = self._state[job_id]
               new_state = replace(old_state, **updates)
               self._state[job_id] = new_state
               
               # Emit specific events based on what changed
               if old_state.status != new_state.status:
                   self._events.emit("status_changed", {
                       "job_id": job_id,
                       "old": old_state.status,
                       "new": new_state.status
                   })
               if old_state.progress != new_state.progress:
                   self._events.emit("progress_updated", {
                       "job_id": job_id,
                       "progress": new_state.progress
                   })
       
       def get_job(self, job_id: str) -> EncodingState:
           """Get job state"""
           with self._lock:
               if job_id not in self._state:
                   raise StateError(f"No such job: {job_id}")
               return self._state[job_id]
       
       def persist(self) -> None:
           """Persist state to disk for recovery"""
           with self._lock:
               state_file = Path("/tmp/drapto/state.json")
               with state_file.open("w") as f:
                   json.dump(
                       {id: asdict(state) for id, state in self._state.items()},
                       f
                   )
       
       @classmethod
       def recover(cls, event_emitter: EventEmitter) -> "StateManager":
           """Recover state from disk"""
           manager = cls(event_emitter)
           try:
               state_file = Path("/tmp/drapto/state.json")
               if state_file.exists():
                   with state_file.open() as f:
                       data = json.load(f)
                       manager._state = {
                           id: EncodingState(**state)
                           for id, state in data.items()
                       }
           except Exception as e:
               logger.error(f"Failed to recover state: {e}")
           return manager
   ```

   - `src/drapto/state/errors.py`: State-related errors
   ```python
   class StateError(Exception):
       """Base class for state-related errors"""
       pass

   class StateNotFoundError(StateError):
       """Job state not found"""
       pass

   class StateUpdateError(StateError):
       """Failed to update state"""
       pass
   ```

2. Add state consumers:
   - `src/drapto/state/progress.py`: Progress tracking
   ```python
   class ProgressTracker:
       """Tracks encoding progress"""
       def __init__(self, state_manager: StateManager):
           self.state = state_manager
           
       def on_segment_complete(self, job_id: str, segment: int) -> None:
           """Update progress when segment completes"""
           state = self.state.get_job(job_id)
           if state.total_segments:
               progress = (segment + 1) / state.total_segments
               self.state.update_job(job_id, progress=progress)
   ```

   - `src/drapto/state/metrics.py`: Resource monitoring
   ```python
   class ResourceMonitor:
       """Monitors system resource usage"""
       def __init__(self, state_manager: StateManager):
           self.state = state_manager
           
       def update_metrics(self, job_id: str) -> None:
           """Update resource usage metrics"""
           state = self.state.get_job(job_id)
           memory, cpu = self._get_resource_usage(state.processes)
           self.state.update_job(
               job_id,
               memory_usage=memory,
               cpu_usage=cpu
           )
   ```

3. Add state producers:
   - `src/drapto/state/events.py`: Event definitions
   ```python
   class StateEvents:
       """State-related event definitions"""
       JOB_CREATED = "job_created"
       JOB_STARTED = "job_started"
       JOB_COMPLETED = "job_completed"
       JOB_FAILED = "job_failed"
       STATUS_CHANGED = "status_changed"
       PROGRESS_UPDATED = "progress_updated"
       SEGMENT_COMPLETE = "segment_complete"
       RESOURCE_UPDATED = "resource_updated"
   ```

4. Add error recovery:
   - `src/drapto/core/retry.py`: Retry handling
   ```python
   @dataclass
   class RetryPolicy:
       """Retry policy configuration"""
       max_attempts: int
       backoff_factor: float
       max_backoff: float
       retryable_errors: set[type[Exception]]

   class RetryableOperation:
       """Wrapper for operations that can be retried"""
       def __init__(self, policy: RetryPolicy):
           self.policy = policy
           self._circuit = CircuitBreaker()
       
       async def run(self, operation: Callable, *args, **kwargs):
           """Run operation with retry policy"""
           if not self._circuit.allow_request():
               raise CircuitBreakerOpen()
               
           attempt = 0
           last_error = None
           
           while attempt < self.policy.max_attempts:
               try:
                   result = await operation(*args, **kwargs)
                   self._circuit.record_success()
                   return result
               except Exception as e:
                   if not any(isinstance(e, err) for err in self.policy.retryable_errors):
                       raise
                       
                   last_error = e
                   attempt += 1
                   self._circuit.record_failure()
                   
                   if attempt < self.policy.max_attempts:
                       delay = min(
                           self.policy.backoff_factor * (2 ** attempt),
                           self.policy.max_backoff
                       )
                       await asyncio.sleep(delay)
           
           raise MaxRetriesExceeded(
               f"Operation failed after {attempt} attempts",
               last_error=last_error
           )
   ```

   - `src/drapto/core/errors.py`: Enhanced error handling
   ```python
   @dataclass
   class ErrorContext:
       """Rich error context for debugging"""
       error: Exception
       operation: str
       inputs: dict
       state: Optional[dict] = None
       timestamp: datetime = field(default_factory=datetime.now)
       stack_trace: str = field(default_factory=lambda: traceback.format_exc())

   class EncodingError(Exception):
       """Base class for encoding errors with context"""
       def __init__(self, message: str, context: ErrorContext):
           super().__init__(message)
           self.context = context
           
       def to_dict(self) -> dict:
           """Convert error to structured format"""
           return {
               "error": str(self),
               "type": self.__class__.__name__,
               "operation": self.context.operation,
               "inputs": self.context.inputs,
               "state": self.context.state,
               "timestamp": self.context.timestamp.isoformat(),
               "stack_trace": self.context.stack_trace
           }
   ```

### Phase 5: Media Processing
1. Add media handlers:
   - `src/drapto/media/audio.py`: Audio stream handling
   ```python
   class AudioProcessor:
       """Handles audio processing
       
       Steps:
       1. Track discovery and analysis
       2. Channel detection
       3. Bitrate assignment
       4. Per-track processing
       5. Quality control
       6. Track management
       7. Error handling
       """
   ```
   - `src/drapto/media/subtitle.py`: Subtitle processing
   ```python
   class SubtitleProcessor:
       """Handles subtitle processing
       
       Features:
       1. Track extraction
       2. Format preservation
       3. Timing preservation
       4. Direct stream copy
       """
   ```
   - `src/drapto/media/muxer.py`: Stream muxing
   ```python
   class StreamMuxer:
       """Handles final file assembly
       
       Steps:
       1. Video track muxing
       2. Audio track addition
       3. Subtitle inclusion
       4. Chapter preservation
       5. Metadata preservation
       6. Container validation
       """
   ```
   - `src/drapto/media/validator.py`: Media validation
   ```python
   class MediaValidator:
       """Validates media files
       
       Checks:
       1. Stream integrity
       2. Codec validation
       3. Duration verification
       4. Size validation
       5. Container structure
       """
   ```
   - `src/drapto/media/stream.py`: Stream mapping and track management
   ```python
   class StreamManager:
       """Manages media streams
       
       Features:
       1. Stream discovery
       2. Track mapping
       3. Stream selection
       4. Track ordering
       5. Stream validation
       """
   ```

### Phase 6: Testing & Documentation
1. Add test infrastructure:
   - `tests/conftest.py`: Test fixtures and mocks
   ```python
   @pytest.fixture
   def mock_ffmpeg():
       """Mock FFmpeg wrapper with recorded outputs"""
       with MockFFmpeg() as ffmpeg:
           ffmpeg.add_response(
               ["ffmpeg", "-i", "input.mkv"],
               stdout=SAMPLE_MEDIAINFO,
               stderr=""
           )
           yield ffmpeg

   @pytest.fixture
   def mock_state_manager():
       """Mock state manager with recorded events"""
       with MockStateManager() as state:
           yield state
   ```

   - `tests/property/test_encoding.py`: Property-based tests
   ```python
   @given(st.integers(min_value=0, max_value=100))
   def test_vmaf_calculation(mock_ffmpeg, target_vmaf):
       """Property-based test for VMAF calculations"""
       encoder = ChunkedEncoder(mock_ffmpeg, target_vmaf=target_vmaf)
       result = encoder.calculate_vmaf(SAMPLE_VIDEO)
       assert 0 <= result <= 100
   ```

   - `tests/performance/test_encoding.py`: Performance tests
   ```python
   @pytest.mark.performance
   def test_encoding_speed(benchmark):
       """Test encoding performance with thresholds"""
       def encode():
           encoder = StandardEncoder()
           encoder.encode(SAMPLE_VIDEO, OUTPUT_PATH)
       
       result = benchmark(encode)
       assert result.stats.mean < MAX_ENCODE_TIME
       assert result.stats.stddev < ENCODE_TIME_STDDEV
   ```

2. Add test scenarios:
   - Unit tests for each component
   - Integration tests for encoding workflows
   - System tests for CLI functionality
   - Performance tests with thresholds
   - Property-based tests for encoding logic
   - Mocking strategies for external dependencies
   - Test fixtures for common scenarios
   - Test data generation

## Key Improvements

### 1. Type Safety
- Full type hints throughout
- Dataclass-based configurations
- Proper error types
- Interface contracts

### 2. State Management
```python
@dataclass
class JobState:
    """Encoding job state"""
    job_id: str
    status: str
    progress: float
    current_segment: Optional[int]
    error: Optional[str]
    
class StateManager:
    """Manages job state persistence"""
    def save_state(self, state: JobState) -> None:
        pass
    
    def load_state(self, job_id: str) -> JobState:
        pass
```

### 3. Process Management
```python
class ProcessManager:
    """Manages external processes"""
    def run(
        self,
        cmd: list[str],
        timeout: Optional[float] = None,
        capture_output: bool = True
    ) -> ProcessResult:
        pass
    
    def run_with_progress(
        self,
        cmd: list[str],
        progress_callback: Callable[[float], None]
    ) -> ProcessResult:
        pass
```

### 4. Error Handling
```python
class EncodingError(Exception):
    """Base class for encoding errors"""
    pass

class ValidationError(EncodingError):
    """Input/output validation errors"""
    pass

class ProcessError(EncodingError):
    """External process errors"""
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