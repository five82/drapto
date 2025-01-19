# Refactoring Plan for State Management and Process Control

## Overall Strategy
- Each phase contains small, atomic changes
- Every change must be independently testable
- Each step must maintain working functionality
- Clear verification points between changes
- Easy rollback points if issues found
- Documentation updated with each change
- Type safety enforced in new Python code

## Phase 1: Foundation (COMPLETED)
1. ✅ Remove PTY handling
   - ✅ Replace with direct subprocess management
   - ✅ Maintain color output via environment variables
   - ✅ Test with both encoding paths
   Verification: All process output visible, both encoding paths working

## Phase 2: Configuration Foundation
1. Basic Configuration System
   - Create central config class with type hints
   - Move environment variables to config
   - Add config validation
   - Add default handling
   - Add configuration documentation
   - Update/create unit tests for config
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
   - Set up new test directory structure
   - Create test utilities and fixtures
   Verification: Package importable, no functionality moved yet

2. Move Current Python Code
   - Move existing modules to new structure
   - Update imports
   - No functional changes
   - Move and update existing unit tests
   - Update test imports and fixtures
   Verification: All existing Python code and tests working in new locations

## Phase 4: Core Python Migration
1. Basic Process Management
   - Create Python process management class
   - Add basic subprocess handling
   - Add type hints and docstrings
   - Keep bash scripts as-is temporarily
   - Update functionality doc process section
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Python can launch and manage processes

2. Analysis Pipeline Migration
   - Port input file analysis
   - Port stream analysis
   - Port path determination logic
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Python analysis matches bash output exactly

3. First Script Migration
   - Port smallest bash script to Python
   - Use new process management
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Python version produces identical results to bash version

4. Encoding Strategy Migration
   - Create Python encoding strategy class
   - Port strategy logic from bash
   - Port resolution-dependent CRF handling
   - Port VMAF-based quality control
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Python version matches bash behavior exactly

5. Standard Encoding Path
   - Port standard encoding path
   - Port quality validation
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Standard encoding identical to bash version

6. Chunked Encoding Path
   - Port chunked encoding path
   - Port VMAF handling
   - Port segment management
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Chunked encoding identical to bash version

7. Output Management
   - Port output assembly logic
   - Port cleanup processes
   - Port error recovery
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Output handling identical to bash version

8. Core Encode Migration
   - Port encode.sh core logic to Python
   - Integrate all migrated components
   - Keep both versions temporarily
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Python version handles all test cases same as bash

## Phase 5: State Management
1. Basic State Tracking
   - Create in-memory state manager with type hints
   - Add basic state validation
   - Migrate first state user
   - Document state management API
   - Update functionality doc state section
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: State tracking working for first component

2. Segment Management
   - Add segment tracking to state manager
   - Migrate segment handling code
   - Verify atomic updates
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Segment handling working through state manager

3. Progress Tracking
   - Add progress tracking to state manager
   - Migrate progress handling code
   - Add state boundaries
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Progress tracking working through state manager

## Phase 6: Process Control
1. Process Hierarchy
   - Implement process hierarchy tracking
   - Add parent-child process management
   - Add signal handling
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Process relationships properly managed

2. Resource Management
   - Add resource tracking
   - Implement automatic cleanup
   - Add cleanup verification
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Resources properly tracked and cleaned up

## Phase 7: Communication
1. Event System
   - Implement basic event system
   - Add first event producer
   - Add first event consumer
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Events flowing between components

2. Status Updates
   - Add status update events
   - Implement progress streaming
   - Add process output handling
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: Status updates working through event system

## Phase 8: Cleanup
1. Remove Bash Scripts
   - Remove each verified script
   - Update documentation
   - Clean up tests
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: System working with no bash scripts

2. Final Verification
   - Full system testing
   - Performance validation
   - Resource cleanup verification
   - Create/update unit tests before implementation
   - Migrate relevant existing tests
   - Update test fixtures and mocks
   - Verify test coverage
   Verification: All functionality working, no regressions

## Documentation Requirements

### Per-Change Documentation
Each atomic change requires:
- Type hints for new Python code
- Docstrings for classes and functions
- API documentation updates
- Functionality doc updates for affected areas
- README updates if public interfaces change

### Phase Documentation
After each phase:
- Architecture documentation updates
- Full API documentation review
- Functionality doc alignment check
- Development guide updates
- Migration guide updates if needed

## Testing Requirements

### Unit Testing Strategy
1. Test Migration
   - Move tests to new structure progressively
   - Update test dependencies and imports
   - Maintain existing test coverage
   - Add new tests for new functionality

2. Test Infrastructure
   - Update test fixtures for new architecture
   - Create mocks for new interfaces
   - Add type checking to test code
   - Maintain test utilities

3. Coverage Requirements
   - Maintain or improve current coverage
   - Add coverage for new components
   - Verify edge cases
   - Test error conditions

### Per-Change Testing
Each atomic change requires:
- Unit tests written/updated before implementation
- Test fixtures and mocks updated
- Coverage requirements met
- Type checking in test code

### Integration Testing
After each phase:
- Full end-to-end testing
- Performance comparison
- Resource cleanup verification
- Error recovery validation

- Type hint validation
- Documentation tests
- API documentation coverage
- Functionality doc alignment tests

## Success Criteria

### Per-Change Success
- Functionality identical to previous state
- All tests passing
- No resource leaks
- Clear rollback point established
- Documentation updated
- Type hints validated
- Documentation updated and verified
- Functionality doc aligned with changes
- Unit tests updated and passing
- Test coverage maintained or improved
- Test infrastructure working

### Phase Success
- All changes in phase verified
- Integration tests passing
- Performance maintained
- No regressions
- Clean state achieved
- Complete documentation coverage
- Type safety verified
- Functionality doc fully aligned
- All unit tests migrated and passing
- Test fixtures and utilities working
- Coverage requirements met

### Final Success
- Complete Python migration
- Single source of truth for state
- Clear process hierarchies
- Direct communication pathways
- Proper resource management
- All tests passing
- Documentation complete
- Complete type safety across codebase
- Comprehensive documentation
- Functionality doc reflects final state
- All unit tests migrated and passing
- Test fixtures and utilities working
- Coverage requirements met

## Rollback Strategy
1. Each atomic change has a git branch
2. Each change has clear verification points
3. Each change can be independently reverted
4. Phase branches for larger rollbacks
5. Main only updated after phase verification 

## Documentation Strategy
1. Keep documentation in sync with code
2. Update functionality doc progressively
3. Maintain API documentation coverage
4. Review documentation at phase boundaries
5. Verify documentation accuracy in tests 