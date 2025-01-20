# drapto Media Handling Documentation

This document provides a detailed overview of how drapto handles various media aspects, including audio processing, Dolby Vision detection, crop detection, muxing process, and codec usage.

## Audio Processing

drapto implements a modern event-driven audio processing system with comprehensive error handling and state management:

```python
class AudioProcessor:
    """Modern event-driven audio processing with state management"""
    
    # Channel layout and bitrate mapping (preserved exactly)
    CHANNEL_CONFIG = {
        1: {"bitrate": 64000,   "layout": "mono"},    # Mono
        2: {"bitrate": 128000,  "layout": "stereo"},  # Stereo
        6: {"bitrate": 256000,  "layout": "5.1"},     # 5.1
        8: {"bitrate": 384000,  "layout": "7.1"}      # 7.1
    }
    
    def __init__(self, config: AudioConfig):
        self.config = config
```

1. **Configuration**
   - Type-safe audio parameters
   - Opus codec configuration
   - FFmpeg argument generation
   - Channel-specific settings

2. **State Management**
   - Centralized audio processing state
   - Track-level progress tracking
   - Error state preservation
   - Atomic state updates

3. **Error Handling**
   - Specialized audio error types
   - Codec-specific error handling
   - Retry mechanisms with backoff
   - Error context preservation

4. **Validation**
   - Track configuration validation
   - Channel layout verification
   - Bitrate confirmation
   - Output quality checks

The system ensures reliable audio processing with:
- Proper state tracking
- Comprehensive error handling
- Event-based progress updates
- Clean error recovery
- Type-safe interfaces

## Dolby Vision Detection

drapto implements a modern event-driven Dolby Vision detection system with comprehensive error handling and state management:

1. **Detection Process**
   - Uses MediaInfo for metadata extraction
   - Validates Dolby Vision presence
   - Extracts profile information
   - Verifies layer compatibility

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

## Crop Detection

drapto implements a modern event-driven crop detection system with HDR awareness:

1. **HDR Content**
   - Detects various HDR formats:
     - SMPTE 2084 (PQ)
     - ARIB STD-B67 (HLG)
     - BT.2020
   - Adjusts crop detection thresholds:
     - Standard content: 24
     - HDR content: Dynamic 128-256 based on black levels
   - Black level analysis for optimal crop detection

### Integration
- Encoders use HDRHandler for consistent processing
- Pipeline ensures HDR metadata flows correctly
- Proper handling of different HDR formats:
  - HDR10: Direct passthrough
  - HDR10+: Dynamic metadata preservation
  - Dolby Vision: Profile-aware processing

### Configuration
```python
@dataclass
class HDRConfig:
    """HDR processing configuration"""
    preserve_hdr: bool = True
    allowed_formats: List[str] = ["HDR10", "HDR10+", "DV"]
    dv_profile: Optional[int] = None
    tone_map_hdr: bool = False
```

## Muxing Process

drapto implements a modern event-driven muxing system with comprehensive state management:

1. **Track Validation**
   - Comprehensive track validation
   - Metadata verification
   - Integrity checking

2. **State Management**
   - Centralized muxing state
   - Track-level progress tracking
   - Error state preservation

3. **Error Handling**
   - Specialized muxing error types
   - Track-specific error handling
   - Retry mechanisms with backoff

The system ensures reliable track muxing with:
- Proper state tracking
- Comprehensive error handling
- Event-based progress updates
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
```

1. **Configuration**
   - Type-safe codec parameters
   - Format-specific settings
   - Hardware acceleration options
   - Quality presets

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