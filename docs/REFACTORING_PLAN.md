# Refactoring Plan for State Management and Process Control

## Phase 1: Stabilize Current Process Management
1. Remove PTY handling
   - Replace with direct subprocess management
   - Maintain color output via environment variables
   - Test with both encoding paths

2. Consolidate Environment Variables
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

2. Consolidate Temporary Directories
   - Single temp root with clear structure
   - Consistent cleanup points
   - Maintain separate dirs for parallel processing
   - Test cleanup reliability

## Phase 3: Improve Process Control
1. Add Process Manager class
   - Handle subprocess lifecycle
   - Manage cleanup between files
   - Track child processes
   - Test process isolation

2. Implement Error Boundaries
   - Clear error states
   - Proper cleanup on failure
   - Consistent error reporting
   - Test error recovery

## Phase 4: Streamline Communication
1. Standardize Data Exchange
   - JSON schema for state files
   - Structured logging format
   - Progress reporting interface
   - Test data consistency

2. Improve Debug Support
   - Add logging levels
   - Structured debug output
   - State inspection tools
   - Test debugging capabilities

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