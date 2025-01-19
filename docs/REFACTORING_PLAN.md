# Refactoring Plan for State Management and Process Control

## Overall Strategy
- Each phase contains small, atomic changes
- Every change must be independently testable
- Each step must maintain working functionality
- Clear verification points between changes
- Easy rollback points if issues found

## Phase 1: Foundation (COMPLETED)
1. ✅ Remove PTY handling
   - ✅ Replace with direct subprocess management
   - ✅ Maintain color output via environment variables
   - ✅ Test with both encoding paths
   Verification: All process output visible, both encoding paths working

## Phase 2: Configuration Foundation
1. Basic Configuration System
   - Create central config class
   - Move environment variables to config
   - Add config validation
   - Add default handling
   Verification: All env vars accessed through config, existing functionality unchanged

2. Path Configuration
   - Move hardcoded paths to config
   - Add path validation
   - Update path handling code
   Verification: All paths working through config system

## Phase 3: Directory Structure
1. Create New Structure
   - Set up new Python package structure
   - Create placeholder modules
   - Add __init__.py files
   Verification: Package importable, no functionality moved yet

2. Move Current Python Code
   - Move existing modules to new structure
   - Update imports
   - No functional changes
   Verification: All existing Python code working in new locations

## Phase 4: Core Python Migration
1. Basic Process Management
   - Create Python process management class
   - Add basic subprocess handling
   - Keep bash scripts as-is temporarily
   Verification: Python can launch and manage processes

2. Analysis Pipeline Migration
   - Port input file analysis
   - Port stream analysis
   - Port path determination logic
   - Keep both versions temporarily
   Verification: Python analysis matches bash output exactly

3. First Script Migration
   - Port smallest bash script to Python
   - Use new process management
   - Keep both versions temporarily
   Verification: Python version produces identical results to bash version

4. Encoding Strategy Migration
   - Create Python encoding strategy class
   - Port strategy logic from bash
   - Port resolution-dependent CRF handling
   - Port VMAF-based quality control
   - Keep both versions temporarily
   Verification: Python version matches bash behavior exactly

5. Standard Encoding Path
   - Port standard encoding path
   - Port quality validation
   - Keep both versions temporarily
   Verification: Standard encoding identical to bash version

6. Chunked Encoding Path
   - Port chunked encoding path
   - Port VMAF handling
   - Port segment management
   - Keep both versions temporarily
   Verification: Chunked encoding identical to bash version

7. Output Management
   - Port output assembly logic
   - Port cleanup processes
   - Port error recovery
   - Keep both versions temporarily
   Verification: Output handling identical to bash version

8. Core Encode Migration
   - Port encode.sh core logic to Python
   - Integrate all migrated components
   - Keep both versions temporarily
   Verification: Python version handles all test cases same as bash

## Phase 5: State Management
1. Basic State Tracking
   - Create in-memory state manager
   - Add basic state validation
   - Migrate first state user
   Verification: State tracking working for first component

2. Segment Management
   - Add segment tracking to state manager
   - Migrate segment handling code
   - Verify atomic updates
   Verification: Segment handling working through state manager

3. Progress Tracking
   - Add progress tracking to state manager
   - Migrate progress handling code
   - Add state boundaries
   Verification: Progress tracking working through state manager

## Phase 6: Process Control
1. Process Hierarchy
   - Implement process hierarchy tracking
   - Add parent-child process management
   - Add signal handling
   Verification: Process relationships properly managed

2. Resource Management
   - Add resource tracking
   - Implement automatic cleanup
   - Add cleanup verification
   Verification: Resources properly tracked and cleaned up

## Phase 7: Communication
1. Event System
   - Implement basic event system
   - Add first event producer
   - Add first event consumer
   Verification: Events flowing between components

2. Status Updates
   - Add status update events
   - Implement progress streaming
   - Add process output handling
   Verification: Status updates working through event system

## Phase 8: Cleanup
1. Remove Bash Scripts
   - Remove each verified script
   - Update documentation
   - Clean up tests
   Verification: System working with no bash scripts

2. Final Verification
   - Full system testing
   - Performance validation
   - Resource cleanup verification
   Verification: All functionality working, no regressions

## Testing Requirements

### Per-Change Testing
Each atomic change requires:
- Unit tests for new code
- Integration tests with existing code
- Verification of unchanged behavior
- Resource usage validation
- Error handling verification

### Integration Testing
After each phase:
- Full end-to-end testing
- Performance comparison
- Resource cleanup verification
- Error recovery validation

## Success Criteria

### Per-Change Success
- Functionality identical to previous state
- All tests passing
- No resource leaks
- Clear rollback point established
- Documentation updated

### Phase Success
- All changes in phase verified
- Integration tests passing
- Performance maintained
- No regressions
- Clean state achieved

### Final Success
- Complete Python migration
- Single source of truth for state
- Clear process hierarchies
- Direct communication pathways
- Proper resource management
- All tests passing
- Documentation complete

## Rollback Strategy
1. Each atomic change has a git branch
2. Each change has clear verification points
3. Each change can be independently reverted
4. Phase branches for larger rollbacks
5. Main only updated after phase verification 