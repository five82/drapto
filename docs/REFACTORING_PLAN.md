# Refactoring Plan for State Management and Process Control

## Phase 1: Stabilize Current Process Management
1. ✅ Remove PTY handling (COMPLETED)
   - Replace with direct subprocess management
   - Maintain color output via environment variables
   - Test with both encoding paths

2. Directory Structure Migration
   - Create new directory structure as outlined in TARGET_STRUCTURE.md
   - Move existing code to new locations
   - Update imports and references
   - Add placeholder files for planned modules
   - Test structure with existing functionality

3. Consolidate Environment Variables
   - Create central config management in Python
   - Move hardcoded paths to config
   - Validate paths before processing
   - Test path handling

## Phase 2: Simplify State Management 
1. Create StateManager class
   - Track file processing state
   - Handle segment information
   - Manage encoding progress
   - Test state transitions
   - Implement in new `core/state.py`

2. Consolidate Temporary Directories
   - Single temp root with clear structure
   - Consistent cleanup points
   - Maintain separate dirs for parallel processing
   - Test cleanup reliability
   - Follow new temp directory structure from TARGET_STRUCTURE.md

## Phase 3: Improve Process Control
1. Add Process Manager class
   - Handle subprocess lifecycle
   - Manage cleanup between files
   - Track child processes
   - Test process isolation
   - Implement in new `core/process.py`

2. Implement Error Boundaries
   - Clear error states
   - Proper cleanup on failure
   - Consistent error reporting
   - Test error recovery
   - Use new logging utilities

## Phase 4: Streamline Communication
1. Standardize Data Exchange
   - JSON schema for state files
   - Structured logging format
   - Progress reporting interface
   - Test data consistency
   - Implement in new `utils/` modules

2. Improve Debug Support
   - Add logging levels
   - Structured debug output
   - State inspection tools
   - Test debugging capabilities
   - Use new logging module

## Phase 5: Python Migration
1. Migrate Core Functionality
   - Port encode.sh to Python
   - Implement encoding strategies in Python
   - Add ffmpeg-python integration
   - Test Python implementations

2. Migrate Helper Functions
   - Port utility scripts to Python modules
   - Move hardware detection to Python
   - Move audio/subtitle handling to Python
   - Test Python implementations

3. Deprecate Bash Scripts
   - Mark scripts as deprecated
   - Add deprecation warnings
   - Document migration path
   - Maintain compatibility layer

4. Remove Bash Scripts
   - Remove all bash scripts
   - Clean up legacy code
   - Update documentation
   - Final testing without bash scripts

## Testing Strategy

Each phase of the refactoring will be accompanied by appropriate tests to ensure stability and prevent regressions. The project uses pytest as the testing framework with the following structure:

- Unit tests in `tests/unit/`
- Integration tests in `tests/integration/`
- Common fixtures in `tests/conftest.py`

### Phase 1.1 Testing Plan (COMPLETED) ✅
- Direct subprocess management without PTY
- Color output preservation
- Process cleanup
- Environment variable setup
- Error handling and logging

### Phase 1.2 Testing Plan
1. Directory Structure Tests:
   - Package import tests
   - Module accessibility tests
   - Path resolution tests
   - Placeholder module tests

2. Update existing tests:
   - Move to new test directory structure
   - Update import paths
   - Add new test categories
   - Maintain coverage

3. Integration tests for:
   - End-to-end encoding with new structure
   - Module interactions
   - Configuration loading

4. Success Criteria:
   - All tests pass in new structure
   - Coverage maintained or improved
   - No regressions in existing functionality
   - Clear test organization

### Phase 5 Testing Plan
1. Python Implementation Tests:
   - Full encoding pipeline tests
   - Strategy implementation tests
   - FFmpeg integration tests
   - Performance comparison tests

2. Migration Tests:
   - Compatibility layer tests
   - Deprecation warning tests
   - Legacy support tests
   - Clean removal verification

3. Success Criteria:
   - All functionality preserved
   - No performance regressions
   - Clean Python-only operation
   - Complete documentation

## Testing Strategy for Each Phase
1. Verify both encoding paths still work
   - Dolby Vision detection
   - Chunked encoding with ab-av1
   - GNU parallel processing

2. Maintain existing functionality
   - Input/output handling
   - File processing order
   - Encoding quality
   - Performance characteristics

3. Regression testing
   - Process multiple files
   - Handle errors gracefully
   - Cleanup resources properly
   - Maintain parallel processing

## Success Criteria
1. No changes to core functionality
2. Improved stability and error handling
3. Cleaner state management
4. Better process control
5. Easier debugging
6. Maintained performance

## Rollback Plan
1. Git branches for each phase
2. Validation steps between changes
3. Clear success criteria for each step
4. Easy rollback points if issues found 