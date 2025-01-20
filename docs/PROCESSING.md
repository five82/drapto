# drapto Processing Documentation

This document provides a detailed overview of how drapto processes and encodes videos, including the input processing flow, encoding paths, parallel processing, and pipeline details.

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
   ```

## Encoding Paths

drapto uses two distinct encoding paths based on input content:

1. **Direct FFmpeg Encoding (Standard Path)**
   - Used for Dolby Vision content to preserve DV metadata
   - Direct FFmpeg encoding without chunking
   - SVT-AV1 encoder with HDR/DV metadata preservation
   - Hardware acceleration support
   - Fixed CRF-based quality control
   - CRF values:
     * SD (≤720p): 25
     * HD (≤1080p): 25
     * UHD (>1080p): 29
   - Required for Dolby Vision content to preserve metadata

2. **Chunked ab-av1 Encoding (Quality-Optimized Path)**
   - Used for non-DV content where quality optimization is desired
   - Segment-based encoding with ab-av1
   - VMAF-based quality analysis for optimal bitrate
   - Parallel processing support
   - VMAF target: 93
   - Cannot be used with Dolby Vision content (chunking breaks DV metadata)
   - Default path for non-DV content

The encoding system uses a modern Python-based architecture with comprehensive wrappers, retry strategies, and quality control:

### Strategy Architecture
   ```
   encode_strategies/
   ├── strategy_base.sh      # Base strategy interface
   ├── chunked_encoding.sh   # Chunked encoding implementation
   ├── dolby_vision.sh       # Dolby Vision handling
   └── json_helper.py        # Strategy configuration
   ```

### Base Strategy Interface
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
   ```

### Strategy Components
   - Segment Tracking
     * Individual segment status tracking
     * Retry strategy management
     * Progress monitoring
     * Error tracking
   
   - Progress Monitoring
     * Overall progress tracking
     * Segment completion status
     * Failure tracking
     * Performance metrics
   
   - Error Handling
     * Detailed error tracking per segment
     * Strategy attempt history
     * Failure cause identification
     * Recovery state preservation
   
   - Performance Metrics
     * Processing time tracking
     * Resource utilization
     * Compression statistics
     * Quality measurements

## Parallel Processing

drapto implements modern parallel processing using Python's async/await with comprehensive resource management and state coordination:

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

## Pipeline Details

The processing pipeline is implemented through several key components:

1. **Pipeline Orchestration** (`pipeline.py`)
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

2. **Worker Management** (`worker.py`)
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

3. **Job Queue Management** (`queue.py`)
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