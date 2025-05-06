# Plan: Replace ffprobe bitplanenoise with FFmpeg Sample Encoding for Grain Detection

**Date:** 2025-05-04

**Goal:** Detect the level of existing grain in a video by encoding samples with FFmpeg using the *same parameters as the final encode* (SVT-AV1, quality, preset, crop, etc., but *without* denoising) and comparing output file sizes. Use this detected level to select `hqdn3d` denoising parameters for the final encode.

**Core Idea:** Encode one or more video samples using the exact settings intended for the final encode (minus `hqdn3d`). Compare the resulting file size(s)/bitrate against predefined thresholds to classify the original grain level. The hypothesis is that videos with more inherent grain will compress less efficiently, resulting in larger file sizes when encoded with consistent, high-quality settings (and no denoising).

**Implementation Steps:**

1.  **Configuration & Constants:**
    *   Leverage constants and logic defined in `docs/reference/mod.rs` for determining sample count, duration, and selection strategy (e.g., randomized sampling between 15%-85% points, dynamic sample count based on duration).
    *   Add configuration for `grain_sample_size_thresholds` (placeholder values initially) to `drapto-core/src/config.rs`. This will map average sample size/bitrate ranges to `GrainLevel` enums (VeryClean, Light, Visible, Heavy).

2.  **Sampling Functionality:**
    *   Implement an `extract_sample(input: &Path, start_time: f64, duration: u32, output_dir: &Path) -> CoreResult<PathBuf>` function (likely within `drapto-core/src/external/ffmpeg_executor.rs` or a new `drapto-core/src/processing/sampling.rs` module).
    *   This function will use the existing `FfmpegExecutor` trait to execute an FFmpeg command to cut the specified segment from the input video into a temporary file.

3.  **Sample Encoding & Measurement Functionality:**
    *   Implement `encode_sample_for_grain_test(sample_path: &Path, final_encode_args: &[String], output_dir: &Path) -> CoreResult<u64>`.
    *   This function will:
        *   Accept the *final* FFmpeg arguments determined for the main encode (derived from `CoreConfig`, video properties like resolution, and crop detection results).
        *   **Crucially:** Filter out any `-vf` arguments containing `hqdn3d` from the provided `final_encode_args` before execution.
        *   Use the `FfmpegExecutor` trait to run the FFmpeg encode command with the filtered arguments.
        *   Retrieve and return the output file size (`u64`) of the encoded sample.
        *   Ensure temporary encoded files are placed in a managed temporary directory (e.g., using `tempfile` crate within a dedicated subdirectory like `grain_samples_tmp`).

4.  **New Analysis Logic (`drapto-core/src/processing/detection/grain_analysis.rs`):**
    *   Retain the `GrainLevel` enum (`VeryClean`, `Light`, `Visible`, `Heavy`).
    *   Retain the `determine_hqdn3d_params(level: GrainLevel) -> String` function.
    *   Modify `GrainAnalysisResult` struct if necessary to store relevant information (e.g., average sample bitrate, number of samples tested).
    *   Rewrite the main `analyze_grain` function:
        *   Update its signature to accept necessary context: input path, `CoreConfig`, an `FfmpegExecutor` instance, and the determined `final_encode_args` (Vec<String>).
        *   Get the video duration using `ffprobe` (potentially refactoring the duration logic from `docs/reference/sampling.rs` into `external/ffprobe_executor.rs`).
        *   Determine the number and start times of samples based on the logic adapted from `docs/reference/mod.rs`.
        *   Loop through the required number of samples:
            *   Call `extract_sample` to get a temporary raw sample file.
            *   Call `encode_sample_for_grain_test`, passing the *filtered* `final_encode_args`.
            *   Record the resulting file size.
            *   Ensure cleanup of temporary raw and encoded sample files (potentially using `tempfile` RAII).
        *   Calculate the average file size across all tested samples. Convert this to an average bitrate (bytes per second) using the sample duration.
        *   Compare the calculated average bitrate against the configured `grain_sample_size_thresholds`.
        *   Determine the corresponding `GrainLevel`.
        *   Return `Ok(Some(GrainAnalysisResult { ... }))` containing the detected level and potentially the calculated bitrate.
        *   Implement robust error handling for failures during sampling or encoding (e.g., return `Ok(None)` or a specific `CoreError` variant).

5.  **Integration (`drapto-core/src/processing/video.rs`):**
    *   Modify the main video processing flow:
        *   First, determine the *complete* set of final encode parameters based on config, video properties, and crop detection results (as done currently). Store these, e.g., `final_encode_args: Vec<String>`.
        *   *Before* the main encode, call the *new* `analyze_grain` function, passing the `final_encode_args`.
        *   Retrieve the `GrainAnalysisResult`.
        *   If grain analysis was successful and returned a `GrainLevel` other than `VeryClean`:
            *   Generate the appropriate `hqdn3d` filter string using `determine_hqdn3d_params`.
            *   Inject this `hqdn3d` filter string into the `final_encode_args` (e.g., appending to existing `-vf` or creating a new one).
        *   Proceed with the main video encode using the potentially modified `final_encode_args`.

6.  **Testing:**
    *   Update unit tests in `grain_analysis.rs` to mock the `FfmpegExecutor` trait for both the `extract_sample` and `encode_sample_for_grain_test` steps. Test different simulated file sizes to verify correct `GrainLevel` determination.
    *   Update or add integration tests (`drapto-core/tests/`) to cover the end-to-end flow, potentially using small test video files with known grain characteristics (though exact size thresholds will require tuning).

7.  **Documentation:**
    *   Update comments within the modified Rust files (`grain_analysis.rs`, `video.rs`, `config.rs`, etc.).
    *   Update any relevant external documentation (e.g., README, design documents) if necessary.

**Diagram:**

```mermaid
graph TD
    A[Input Video Path + Config] --> B(Get Video Duration);
    B --> C{Determine Sample Points (using reference/mod.rs logic)};
    A --> D(Determine Final Encode Params - SVT-AV1, Quality, Preset, Crop);
    D --> D_Filter(Filter out hqdn3d from Final Params);
    C --> E[Loop N Times (based on reference/mod.rs)];
    E --> F(Extract Sample using FFmpeg);
    F --> G{Encode Sample using FFmpeg (with Filtered Final Params)};
    G --> H(Get Encoded Sample File Size);
    H --> I(Cleanup Temp Files);
    I --> E;
    E -- All Samples Done --> J(Calculate Avg File Size/Bitrate);
    J --> K{Compare Avg Size vs Thresholds};
    K -- Very Clean --> L[GrainLevel::VeryClean];
    K -- Light --> M[GrainLevel::Light];
    K -- Visible --> N[GrainLevel::Visible];
    K -- Heavy --> O[GrainLevel::Heavy];
    L --> P(GrainAnalysisResult);
    M --> P;
    N --> P;
    O --> P;
    P --> Q{Use GrainLevel to select hqdn3d};
    Q & D --> R(Final Encode with Correct hqdn3d);

    subgraph Grain Analysis
        B; C; E; F; G; H; I; J; K; L; M; N; O; P; Q; D_Filter;
    end

    subgraph Main Processing
        A; D; R;
    end
```

**Open Questions/Tuning:**

*   The exact values for `grain_sample_size_thresholds` will need empirical tuning based on testing with various video sources and the chosen final encode settings. Initial placeholders will be used.
*   The specific implementation details of adapting the sampling logic from `docs/reference/mod.rs` (which uses HandBrake concepts) to an FFmpeg context will need careful consideration during implementation.