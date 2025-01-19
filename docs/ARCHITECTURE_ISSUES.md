# Current Architecture Issues

## 1. Complex State Management Across Languages
- Python manages high-level file processing and environment
- Bash scripts handle encoding strategy and process control 
- Python-generated JSON files for state tracking
- Multiple layers of process management (pty, subprocess, bash scripts)

## 2. Fragile Communication Channels
- Environment variables for path communication
- JSON files for state persistence
- Temporary directories for data exchange
- Process output parsing for status updates

## 3. Multiple Sources of Truth
- Python tracking processed files
- Bash scripts managing encoding state
- JSON files storing segment information
- Multiple temporary directories with state data

## 4. Error-Prone Process Management
- Complex pseudo-terminal handling
- Multiple cleanup points
- Nested process hierarchies
- Manual process cleanup between files 