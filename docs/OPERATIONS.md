# drapto Operations Documentation

This document provides a detailed overview of drapto's operational aspects, including validation, error recovery, progress tracking, process management, and error handling.

## Validation Process

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

2. **State Management**
   - Centralized validation state
   - Track-level progress tracking
   - Error state preservation
   - Atomic state updates

3. **Error Handling**
   - Specialized validation error types
   - Track-specific error handling
   - Retry mechanisms with backoff
   - Error context preservation

4. **Track Validation**
   - Comprehensive track validation
   - Metadata verification
   - Integrity checking
   - Quality validation

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

1. **Progress Tracking**
   - Overall progress percentage
   - Stage-specific progress
   - Time estimates
   - Resource utilization

2. **Logging System**
   - Structured log format
   - Level-based logging
   - Component-specific logs
   - Error tracking

3. **Metrics Collection**
   - Performance metrics
   - Resource usage
   - Quality measurements
   - Error statistics

4. **User Feedback**
   - Real-time progress updates
   - Stage transitions
   - Error notifications
   - Completion status

## Process Management

drapto implements sophisticated process management with resource awareness and error handling:

1. **Resource Management**
   - CPU utilization monitoring
   - Memory usage tracking
   - Disk I/O management
   - Network bandwidth control

2. **Process Control**
   - Graceful startup/shutdown
   - Signal handling
   - Resource cleanup
   - State preservation

3. **Worker Management**
   - Process pool control
   - Load balancing
   - Resource allocation
   - Error handling

4. **Pipeline Coordination**
   - Stage synchronization
   - Resource scheduling
   - Error propagation
   - State coordination

## Error Handling

drapto implements comprehensive error handling with recovery mechanisms and state preservation:

1. **Error Types**
   ```python
   class EncodingError(Exception):
       """Base class for encoding errors"""
       pass

   class ValidationError(EncodingError):
       """Input/output validation errors"""
       pass

   class HardwareError(EncodingError):
       """Hardware-related errors"""
       pass

   class ResourceError(EncodingError):
       """Resource allocation errors"""
       pass
   ```

2. **Error Recovery**
   - Stage-specific recovery
   - Resource cleanup
   - State preservation
   - Retry mechanisms

3. **Error Reporting**
   - Structured error logs
   - User notifications
   - Debug information
   - Stack traces

4. **Error Prevention**
   - Input validation
   - Resource checks
   - State validation
   - Format verification

The system ensures errors are handled systematically while maintaining system stability and providing insights for prevention. 