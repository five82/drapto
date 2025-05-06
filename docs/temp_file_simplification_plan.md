# Temporary File Handling Simplification Plan

This document outlines a plan to simplify the temporary file handling logic within the `drapto-core` grain analysis feature.

## Problem

The current temporary file handling in `analyze_grain` and `extract_sample` uses a mix of `tempfile::TempDir` for automatic directory cleanup and `tempfile::TempPath` (via `into_temp_path()`) for manually managed temporary files. This leads to slightly more complex logic, including manual file deletion steps.

## Current Approach Recap

1.  `analyze_grain` creates a main temporary directory (`TempDir`) which *will* auto-cleanup everything inside it when it goes out of scope.
2.  `extract_sample` creates a temporary file *within* that directory but uses `into_temp_path()` which prevents the *file itself* from being auto-deleted immediately, returning its path.
3.  `analyze_grain` then manually deletes the raw sample file created by `extract_sample` after it's used.
4.  Encoded samples are created directly within the main temporary directory path.

## Proposed Simplification Plan

The core idea is to rely *entirely* on the main `TempDir` created in `analyze_grain` for all cleanup.

1.  **Refactor `extract_sample` (`ffmpeg_executor.rs`):**
    *   Stop using `tempfile::tempfile_in()` and `into_temp_path()`.
    *   Generate a unique filename for the raw sample (e.g., using the `rand` crate or adding the `uuid` crate).
    *   Construct a normal `PathBuf` by joining the `output_dir` (which is the path of the `TempDir` from `analyze_grain`) and the unique filename.
    *   Tell `ffmpeg` to create the sample at this specific `PathBuf`.
    *   Perform the existence check previously added.
    *   Return the `PathBuf`. The file created at this path will now be automatically cleaned up when the `TempDir` in `analyze_grain` is dropped.
2.  **Refactor `analyze_grain` (`grain_analysis.rs`):**
    *   **Remove** the manual `fs::remove_file(&raw_sample_path)` call. This is no longer needed as the `TempDir`'s cleanup will handle it.
    *   Keep the rest of the logic the same (creating the `TempDir`, calling the refactored `extract_sample`, creating encoded samples within the `TempDir`).

## Benefits

*   **Centralized Cleanup:** All temporary files (raw samples, encoded samples) are managed by the single `TempDir` created in `analyze_grain`.
*   **Simplified Logic:** Removes the need for `into_temp_path()` and manual file deletion, making the code easier to follow.
*   **Reduced Complexity:** Less mixing of different temporary file management strategies.

## Diagrammatic Flow (Simplified)

```mermaid
sequenceDiagram
    participant AG as analyze_grain
    participant ES as extract_sample
    participant FF as ffmpeg
    participant TD as TempDir (Auto Cleanup)

    AG->>TD: Create TempDir (e.g., /tmp/analysis_xyz)
    AG->>ES: Call extract_sample(input, ..., /tmp/analysis_xyz)
    ES->>ES: Generate unique name (e.g., raw_abc.mkv)
    ES->>FF: Run ffmpeg -i input ... -c copy /tmp/analysis_xyz/raw_abc.mkv
    FF-->>ES: Exit status 0
    ES->>ES: Check /tmp/analysis_xyz/raw_abc.mkv exists
    ES-->>AG: Return PathBuf(/tmp/analysis_xyz/raw_abc.mkv)
    Note over AG: Loop through denoise levels
    AG->>FF: Run ffmpeg -i /tmp/analysis_xyz/raw_abc.mkv ... /tmp/analysis_xyz/encoded_1.mkv
    FF-->>AG: Exit status 0
    AG->>AG: Get size of /tmp/analysis_xyz/encoded_1.mkv
    Note over AG: (Repeat for other levels)
    Note over AG: End of analyze_grain function
    AG->>TD: TempDir goes out of scope
    TD->>TD: Delete /tmp/analysis_xyz and all its contents