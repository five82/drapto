# drapto Configuration Documentation

This document provides a detailed overview of drapto's configuration system, including user settings, defaults, hardware acceleration, state management, and temporary file handling.

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

## Default Settings

The system uses standardized defaults based on content type:

1. **Direct FFmpeg Encoding (Standard Path)**
   - Used for Dolby Vision content
   - CRF values:
     * SD (≤720p): 25
     * HD (≤1080p): 25
     * UHD (>1080p): 29
   - Hardware acceleration: Enabled for decoding
   - Audio: Opus with standard channel layouts

2. **Chunked ab-av1 Encoding (Quality-Optimized Path)**
   - Used for non-DV content
   - VMAF target: 93
   - Segment size: 10 seconds
   - Parallel jobs: CPU core count - 1
   - Audio: Same as standard path

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

2. **Hardware-Accelerated Decoding Fallback**
   - Automatically detects hardware decoding capabilities
   - Attempts hardware-accelerated decoding first (e.g., VideoToolbox on macOS)
   - On failure, gracefully falls back to software decoding
   - Maintains encoding parameters during fallback (SVT-AV1 software encoding is always used)
   - Logs hardware acceleration failures for diagnostics

3. **Decoding Recovery Process**
   - Primary attempt: Hardware-accelerated decoding
   - Final fallback: Pure software decoding
   - Encoding always uses software SVT-AV1 regardless of decoding method
   - Each stage maintains identical quality settings

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

## State Management

drapto implements a schema-based state management system with comprehensive error handling:

1. **State Architecture**
   ```python
   @dataclass
   class ProcessingState:
       """Global processing state"""
       current_file: Optional[str]
       stage: ProcessingStage
       progress: float
       errors: List[Error]
       stats: Dict[str, Any]
   ```

2. **State Components**
   - Centralized state tracking
   - Progress monitoring
   - Failure tracking
   - Resource history

3. **State Persistence**
   - Atomic state updates
   - Checkpoint creation
   - Recovery mechanisms
   - State validation

4. **Error State**
   - Error classification
   - Recovery tracking
   - Resource state
   - Validation state

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
   - Atomic file operations
   - Checkpoint creation
   - Recovery mechanisms
   - Cleanup validation

4. **Error Recovery**
   - Partial cleanup on failure
   - State preservation
   - Resource cleanup
   - Validation checks 