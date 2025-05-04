# HQDN3D Filter Implementation Plan

This document outlines the plan for integrating the `hqdn3d` ffmpeg filter into the `drapto` project for optional light video denoising.

## Goal

Implement light denoising using the `hqdn3d` filter to potentially reduce bitrate caused by film grain. Denoising should be enabled by default and disable-able via a command-line flag.

## Affected Files

*   `drapto-core/src/config.rs`
*   `drapto-cli/src/cli.rs`
*   `drapto-cli/src/config.rs` (or wherever CLI args are mapped to CoreConfig)
*   `drapto-core/src/external/ffmpeg.rs`
*   `drapto-core/src/processing/video.rs`

## Plan Phases

### Phase 1: Configuration

1.  **Modify `drapto-core/src/config.rs`:**
    *   Add `pub enable_denoise: bool` to `CoreConfig`.
    *   Ensure it defaults to `true`.
2.  **Modify `drapto-cli/src/cli.rs`:**
    *   Add a `--no-denoise` flag using `clap` (`ArgAction::SetFalse`).
3.  **Modify `drapto-cli/src/config.rs` (or equivalent):**
    *   Map the `--no-denoise` flag to `core_config.enable_denoise` (setting it to `false` when the flag is present).

### Phase 2: Parameter Propagation

1.  **Modify `drapto-core/src/external/ffmpeg.rs`:**
    *   Add `pub enable_denoise: bool` to the `EncodeParams` struct.
2.  **Modify `drapto-core/src/processing/video.rs`:**
    *   In `process_videos`, pass `config.enable_denoise` when creating `EncodeParams`.

### Phase 3: Filter Implementation

1.  **Modify `drapto-core/src/external/ffmpeg.rs` (`run_ffmpeg_encode`):**
    *   Retrieve `enable_denoise` and `crop_filter` from `params`.
    *   Use a `match` statement on `(enable_denoise, crop_filter.is_some())`:
        *   **Both True:** Use `-filter_complex` chaining `crop` and `hqdn3d` (e.g., `[0:v:0]crop=...,hqdn3d[vout]`). Map `[vout]`.
        *   **Denoise True, Crop False:** Use `-vf hqdn3d`. Map `0:v:0?`.
        *   **Denoise False, Crop True:** Use `-filter_complex` with only `crop` (existing logic). Map `[vout]`.
        *   **Both False:** Map `0:v:0?` directly (existing logic).

## Visual Plan (Mermaid)

```mermaid
graph TD
    subgraph Config Changes
        direction LR
        CfgCore[drapto-core/config.rs: Add enable_denoise to CoreConfig]
        CfgCli[drapto-cli/cli.rs: Add --no-denoise flag]
        CfgMap[drapto-cli/config.rs: Map CLI flag to CoreConfig]
    end

    subgraph Parameter Propagation
        direction LR
        ParamStruct[drapto-core/ffmpeg.rs: Add enable_denoise to EncodeParams]
        ParamPass[drapto-core/video.rs: Pass config.enable_denoise to EncodeParams]
    end

    subgraph Filter Logic
        direction TB
        FilterStart[drapto-core/ffmpeg.rs: run_ffmpeg_encode] --> CheckFilters{Denoise AND Crop?};
        CheckFilters -- Both True --> FC_Both[Build filter_complex: crop,hqdn3d];
        CheckFilters -- Denoise True, Crop False --> VF_Denoise[Build vf: hqdn3d];
        CheckFilters -- Denoise False, Crop True --> FC_Crop[Build filter_complex: crop];
        CheckFilters -- Both False --> MapDirect[Map video directly: 0:v:0?];
        FC_Both --> AddFCArgs[Add filter_complex/map args];
        VF_Denoise --> AddVFArgs[Add vf/map args];
        FC_Crop --> AddFCArgs;
        MapDirect --> AddMapArgs[Add map arg];
        AddFCArgs --> ContinueBuild[Continue Command Build];
        AddVFArgs --> ContinueBuild;
        AddMapArgs --> ContinueBuild;
    end

    CfgCore --> CfgMap;
    CfgCli --> CfgMap;
    CfgMap --> ParamPass;
    ParamStruct --> ParamPass;
    ParamPass --> FilterStart;