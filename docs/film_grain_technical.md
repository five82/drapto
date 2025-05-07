# Film Grain Analysis, Denoising, and Synthesis: Technical Documentation

## Overview

Drapto implements a sophisticated film grain management system that optimizes video compression efficiency while maintaining perceptual quality. This document provides a detailed technical explanation of the film grain analysis, denoising, and synthesis processes used in the Drapto system.

The core principle behind Drapto's approach is that film grain and noise consume a disproportionate amount of bitrate during video encoding due to their high-entropy, random nature. By selectively reducing natural grain before encoding and then adding back controlled synthetic grain, Drapto achieves significantly better compression efficiency without sacrificing perceptual quality.

## System Architecture

The film grain management system consists of three main components:

1. **Film Grain Analysis**: Detects and quantifies the amount of grain/noise in the source video
2. **Adaptive Denoising**: Applies optimal denoising parameters based on the analysis results
3. **Film Grain Synthesis**: Adds back controlled synthetic grain during encoding

This approach creates a balanced solution that preserves the visual character of content while dramatically improving compression efficiency.

## 1. Film Grain Analysis

### Multi-Phase Approach

The grain analysis module implements a multi-phase approach to determine the optimal denoising parameters:

1. **Sample Extraction**: Multiple short samples are extracted from different parts of the video
2. **Initial Testing (Phase 1)**: Each sample is encoded with various denoising levels
3. **Knee Point Analysis (Phase 2)**: File size reductions are analyzed to find the optimal point
4. **Adaptive Refinement (Phase 3)**: Additional tests are performed around the initial estimates
5. **Result Aggregation (Phase 4)**: The final optimal denoising level is determined

### Sample Extraction Logic

- The number of samples is dynamically calculated based on video duration
- Samples are distributed evenly throughout the video to capture different scenes
- Each sample is typically 10 seconds long (configurable via `film_grain_sample_duration`)
- A minimum of 3 and maximum of 9 samples are used (odd number for reliable median calculation)

```rust
// Sample count calculation
let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
let mut num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
if num_samples % 2 == 0 {
    num_samples = (num_samples + 1).min(MAX_SAMPLES);
}
```

### Sample Analysis Output

During analysis, the system outputs the results of each sample test. Here's an example of the output format:

```
Phase 1: Testing initial grain levels...
  Processing sample 1/3 (Start: 35.26s, Duration: 10s)...
      -> VeryClean  size: 26.25 MB
      -> VeryLight  size: 20.05 MB
      -> Light      size: 17.73 MB
      -> Visible    size: 14.64 MB
      -> Medium     size: 12.40 MB
```

Note: The output logs have been updated to consistently use "VeryClean" instead of "None" when referring to videos with no grain or when no denoising is applied.

### Grain Level Classification

The system classifies grain into five distinct levels:

| Grain Level | Description | Denoising Approach |
|-------------|-------------|-------------------|
| VeryClean   | Very little to no grain | No denoising applied |
| VeryLight   | Barely noticeable grain | Very light denoising |
| Light       | Light grain | Light denoising |
| Visible     | Noticeable grain with spatial patterns | Spatially-focused denoising |
| Medium      | Medium grain with temporal fluctuations | Temporally-focused denoising |

### Knee Point Detection Algorithm

The knee point detection algorithm is a key innovation that finds the optimal balance between compression efficiency and visual quality preservation:

1. Each sample is encoded with different denoising levels (always including "VeryClean" as baseline)
2. File sizes are compared to calculate size reduction percentages
3. An efficiency metric is calculated for each level: `(size_reduction / sqrt(grain_level_value))`
4. The knee point is identified as the level where additional denoising provides diminishing returns
5. The threshold for diminishing returns is configurable via `film_grain_knee_threshold` (default: 0.8)

```rust
// Efficiency calculation (simplified)
let efficiency = size_reduction / (grain_level_value.sqrt());
```

### Adaptive Refinement

To improve accuracy, the system performs adaptive refinement around the initial estimates:

1. Statistical analysis of initial estimates (median, standard deviation)
2. Testing of intermediate grain levels not covered in the initial phase
3. Adaptive range width based on the variance of initial estimates
4. Type-safe parameter selection using predefined values for each level

## 2. Denoising Implementation

### HQDN3D Filter

Drapto uses FFmpeg's high-quality 3D denoiser (hqdn3d) filter for grain reduction. This filter provides excellent temporal and spatial noise reduction while preserving detail.

The hqdn3d parameters are in the format `y:cb:cr:strength` where:
- `y`: Luma spatial strength (higher values = more denoising)
- `cb`: Chroma spatial strength
- `cr`: Temporal luma strength
- `strength`: Temporal chroma strength

### Denoising Parameters by Grain Level

Each grain level maps to specific hqdn3d parameters:

| Grain Level | HQDN3D Parameters | Description |
|-------------|-------------------|-------------|
| VeryClean   | No parameters | No denoising applied |
| VeryLight   | `0.5:0.3:3:3` | Very light denoising with focus on temporal noise |
| Light       | `1:0.7:4:4` | Light denoising with balanced spatial/temporal approach |
| Visible     | `1.5:1.0:6:6` | Moderate denoising with higher temporal values |
| Medium      | `2:1.3:8:8` | Stronger denoising with emphasis on temporal filtering |

These parameters are deliberately conservative to avoid excessive blurring while still improving compression efficiency. The goal is bitrate reduction while maintaining video quality, not creating an artificially smooth appearance.

## 3. Film Grain Synthesis

### SVT-AV1 Film Grain Synthesis

After denoising, Drapto applies synthetic film grain during encoding using the SVT-AV1 encoder's film grain synthesis feature. This approach provides several advantages:

1. Synthetic grain is more compression-friendly than natural grain
2. The visual character of the content is preserved
3. The encoder can represent the grain pattern with minimal bits

### Mapping Denoising Levels to Film Grain Values

Denoising levels are mapped to corresponding SVT-AV1 film grain synthesis values:

| Grain Level | HQDN3D Parameters | Film Grain Value |
|-------------|-------------------|------------------|
| VeryClean   | No parameters | 0 (no synthetic grain) |
| VeryLight   | `0.5:0.3:3:3` | 4 (very light synthetic grain) |
| Light       | `1:0.7:4:4` | 8 (light synthetic grain) |
| Visible     | `1.5:1.0:6:6` | 12 (moderate synthetic grain) |
| Medium      | `2:1.3:8:8` | 16 (medium synthetic grain) |

```rust
// Film grain mapping function
fn map_hqdn3d_to_film_grain(hqdn3d_params: &str) -> u8 {
    // For VeryClean videos, no film grain is needed
    if hqdn3d_params == None || hqdn3d_params.is_empty() {
        return 0; // No synthetic grain for VeryClean
    }

    // Map other grain levels to corresponding film grain values
    for (params, film_grain) in &[
        ("hqdn3d=0.5:0.3:3:3", 4),  // VeryLight
        ("hqdn3d=1:0.7:4:4", 8),    // Light
        ("hqdn3d=1.5:1.0:6:6", 12), // Visible
        ("hqdn3d=2:1.3:8:8", 16),   // Medium
    ] {
        if hqdn3d_params == *params {
            return *film_grain;
        }
    }
    // Fallback logic for non-standard parameters...
}
```

### SVT-AV1 Encoder Configuration

The film grain synthesis is applied through the SVT-AV1 encoder parameters:

```
-svtav1-params tune=3:film-grain={value}:film-grain-denoise=0
```

Where:
- `tune=3`: Sets the encoder tuning mode to "Visual Quality"
- `film-grain={value}`: Sets the film grain synthesis level (0-50, with our mapping using 0-16)
- `film-grain-denoise=0`: Disables additional denoising in the encoder (as we've already applied optimal denoising)

## Configuration Options

Drapto provides several configuration options to fine-tune the grain analysis and synthesis process:

| Option | Description | Default |
|--------|-------------|---------|
| `enable_denoise` | Enable/disable the entire grain management system | `true` |
| `film_grain_sample_duration` | Duration in seconds for each sample | `10` |
| `film_grain_knee_threshold` | Threshold for knee point detection (0.0-1.0) | `0.8` |
| `film_grain_fallback_level` | Fallback level if analysis fails | `VeryClean` |
| `film_grain_max_level` | Maximum allowed grain level | `Medium` |

These options can be set via command-line arguments or configuration files.

## Conclusion

Drapto's film grain management system represents a sophisticated approach to optimizing video compression efficiency while maintaining perceptual quality. By analyzing grain characteristics, applying optimal denoising, and synthesizing controlled film grain during encoding, the system achieves significant bitrate savings (often 20-40%) without sacrificing visual quality.

The multi-phase analysis approach with knee point detection ensures that each video receives optimal processing based on its unique characteristics, while the configurable parameters allow fine-tuning for different content types and quality requirements.
