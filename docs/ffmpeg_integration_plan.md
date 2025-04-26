# Plan: Implement FFmpeg/FFprobe with `ffmpeg-sidecar` v2.0.5

This document outlines the plan to integrate the `ffmpeg-sidecar` crate (version 2.0.5) into the `drapto` project to handle the execution of `ffmpeg` and `ffprobe` commands.

## Background

Initial investigation revealed that while logic exists in `drapto-core/src/external/ffmpeg.rs` to *build* FFmpeg command arguments (`build_ffmpeg_args`), this function is unused outside its own tests. Furthermore, no code was found that actually executes `ffmpeg` or `ffprobe` using `std::process::Command` or similar within the `src` directories.

Therefore, the task is to **introduce** `ffmpeg-sidecar` as the mechanism for executing these external tools.

## Approved Plan Details

1.  **Add Dependency:**
    *   Add `ffmpeg-sidecar = "2.0.5"` to the `[dependencies]` section of `drapto-core/Cargo.toml`.
    *   The optional `download_ffmpeg` feature will **not** be enabled.

2.  **Implement FFprobe Logic:**
    *   **File(s):** Primarily `drapto-core/src/processing/detection.rs`, potentially involving `drapto-core/src/external/ffmpeg.rs` or a new `ffprobe.rs` module.
    *   **Action:**
        *   Create functions using `ffmpeg_sidecar::FfprobeCommand` to execute `ffprobe`.
        *   Parse the output (likely JSON) to extract necessary media information (streams, resolution, duration, etc.).
        *   Integrate these calls into the media detection workflow.
    *   **Testing:** Update or replace the `test-mock-ffprobe` feature and associated tests to mock `ffmpeg_sidecar::FfprobeCommand` calls.

3.  **Implement FFmpeg Execution Logic:**
    *   **File(s):** A new function within `drapto-core/src/external/ffmpeg.rs` or `drapto-core/src/processing/video.rs`.
    *   **Trigger:** Called from `drapto-cli/src/commands/encode.rs`.
    *   **Action:**
        *   Use `ffmpeg_sidecar::FfmpegCommand`'s builder pattern to construct and run the `ffmpeg` command.
        *   Map `drapto` configuration parameters to `ffmpeg-sidecar` methods.
        *   The existing `FfmpegCommandArgs` struct and `build_ffmpeg_args` function will be **replaced** entirely by `ffmpeg-sidecar`'s builder methods.
        *   Implement progress handling using `ffmpeg-sidecar`'s features.
        *   Map errors from `ffmpeg-sidecar` to `drapto_core::error::CoreError`.

4.  **Proposed Interaction Flow:**

    ```mermaid
    sequenceDiagram
        participant CLI as drapto-cli (encode.rs)
        participant Core as drapto-core
        participant Detection as Core (detection.rs)
        participant Ffprobe as ffmpeg_sidecar::FfprobeCommand
        participant Encoding as Core (ffmpeg.rs/video.rs)
        participant Ffmpeg as ffmpeg_sidecar::FfmpegCommand

        CLI->>Core: Start Encode Process(input_path, config)
        Core->>Detection: Detect Media Properties(input_path)
        Detection->>Ffprobe: Run ffprobe(input_path)
        Ffprobe-->>Detection: Return Media Info (streams, crop, etc.)
        Detection-->>Core: Return Processed Media Info
        Core->>Encoding: Execute FFmpeg Encode(media_info, config)
        Encoding->>Ffmpeg: Build and Run ffmpeg command
        Note over Ffmpeg: Handles execution, progress, errors
        Ffmpeg-->>Encoding: Return Result/Error
        Encoding-->>Core: Return Result/Error
        Core-->>CLI: Return Final Result/Error