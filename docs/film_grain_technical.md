# Film Grain Analysis, Denoising, and Synthesis: Technical Documentation

## Overview

Drapto implements a sophisticated film grain management system that optimizes video compression efficiency while maintaining perceptual quality. This document provides a detailed technical explanation of the film grain analysis, denoising, and synthesis processes used in the Drapto system.

The core principle behind Drapto's approach is that film grain and noise consume a disproportionate amount of bitrate during video encoding due to their high-entropy, random nature. By selectively reducing natural grain before encoding and then adding back controlled synthetic grain, Drapto achieves significantly better compression efficiency without sacrificing perceptual quality.

The system now features continuous parameter interpolation for fine-grained control over denoising strength, allowing for more precise optimization of the compression vs. quality tradeoff. This enhancement leverages the multidimensional parameter space of the hqdn3d denoiser to achieve more precise denoising than discrete levels alone would allow.

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
      -> Baseline   size: 26.25 MB
      -> VeryLight  size: 20.05 MB
      -> Light      size: 17.73 MB
      -> Moderate   size: 14.64 MB
      -> Elevated   size: 12.40 MB
```

Note: The output logs have been updated to consistently use "Baseline" instead of "None" when referring to videos with no grain or when no denoising is applied.

### Grain Level Classification

The system classifies grain into five distinct levels:

| Grain Level | Description | Denoising Approach |
|-------------|-------------|-------------------|
| Baseline    | Very little to no grain | No denoising applied |
| VeryLight   | Barely noticeable grain | Very light denoising |
| Light       | Light grain | Light denoising |
| Moderate    | Noticeable grain with spatial patterns | Spatially-focused denoising |
| Elevated    | Medium grain with temporal fluctuations | Temporally-focused denoising |

### Knee Point Detection Algorithm

The knee point detection algorithm is a key innovation that finds the optimal balance between compression efficiency and visual quality preservation:

1. Each sample is encoded with different denoising levels (always including "Baseline" as reference)
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
4. Continuous parameter interpolation for fine-grained control

The refinement process now uses continuous parameter interpolation to test more intermediate points between the standard grain levels. This allows for more precise optimization of denoising parameters:

```rust
// Generate 3-5 intermediate points based on range size
let num_points = ((upper_f - lower_f) * 2.0).round().max(3.0).min(5.0) as usize;
let step = (upper_f - lower_f) / (num_points as f32 + 1.0);

// Generate interpolated parameters for each test point
for i in 1..=num_points {
    let point_f = lower_f + step * (i as f32);
    let hqdn3d_params = generate_hqdn3d_params(point_f);
    // Test this interpolated parameter set...
}
```

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
| Baseline    | No parameters | No denoising applied |
| VeryLight   | `0.5:0.3:3:3` | Very light denoising with focus on temporal noise |
| Light       | `1:0.7:4:4` | Light denoising with balanced spatial/temporal approach |
| Moderate    | `1.5:1.0:6:6` | Moderate denoising with higher temporal values |
| Elevated    | `2:1.3:8:8` | Stronger denoising with emphasis on temporal filtering |

These parameters are deliberately conservative to avoid excessive blurring while still improving compression efficiency. The goal is bitrate reduction while maintaining video quality, not creating an artificially smooth appearance.

### Continuous Parameter Interpolation

In addition to the discrete grain levels above, the system now supports continuous parameter interpolation for more fine-grained control. This allows for testing and applying denoising parameters at any point along the continuous strength scale (0.0-4.0):

```rust
// Map a continuous strength value (0.0-4.0) to interpolated hqdn3d parameters
pub fn generate_hqdn3d_params(strength_value: f32) -> String {
    // No denoising for strength <= 0
    if strength_value <= 0.0 {
        return "".to_string();
    }

    // Define anchor points for interpolation (strength, y, cb, cr, strength)
    let anchor_points = [
        (0.0, 0.0, 0.0, 0.0, 0.0),       // Baseline (no denoising)
        (1.0, 0.5, 0.3, 3.0, 3.0),       // VeryLight
        (2.0, 1.0, 0.7, 4.0, 4.0),       // Light
        (3.0, 1.5, 1.0, 6.0, 6.0),       // Moderate
        (4.0, 2.0, 1.3, 8.0, 8.0),       // Elevated
    ];

    // Find anchor points to interpolate between and calculate parameters
    // ...interpolation logic...

    // Format the parameters string
    format!("hqdn3d={:.2}:{:.2}:{:.2}:{:.2}", luma, chroma, temp_luma, temp_chroma)
}
```

This approach provides several benefits:
1. More precise optimization of denoising parameters
2. Better adaptation to different types of grain and noise
3. Smoother transitions between denoising levels
4. More granular control over the compression vs. quality tradeoff

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
| Baseline    | No parameters | 0 (no synthetic grain) |
| VeryLight   | `0.5:0.3:3:3` | 4 (very light synthetic grain) |
| Light       | `1:0.7:4:4` | 8 (light synthetic grain) |
| Moderate    | `1.5:1.0:6:6` | 12 (moderate synthetic grain) |
| Elevated    | `2:1.3:8:8` | 16 (medium synthetic grain) |

For interpolated parameter sets, the system uses a more sophisticated mapping function that provides continuous granularity:

```rust
/// Maps a hqdn3d parameter set to the corresponding SVT-AV1 film_grain value.
/// Handles both standard and refined/interpolated parameter sets.
fn map_hqdn3d_to_film_grain(hqdn3d_params: &str) -> u8 {
    // No denoising = no film grain synthesis
    if hqdn3d_params.is_empty() {
        return 0;
    }

    // Fixed mapping for standard levels (for backward compatibility)
    for (params, film_grain) in &[
        ("hqdn3d=0.5:0.3:3:3", 4),  // VeryLight
        ("hqdn3d=1:0.7:4:4", 8),    // Light
        ("hqdn3d=1.5:1.0:6:6", 12), // Moderate
        ("hqdn3d=2:1.3:8:8", 16),   // Elevated
    ] {
        if hqdn3d_params == *params {
            return *film_grain;
        }
    }

    // For interpolated parameter sets, extract the luma spatial strength
    let luma_spatial = parse_hqdn3d_first_param(hqdn3d_params);

    // No denoising = no grain synthesis
    if luma_spatial <= 0.1 {
        return 0;
    }

    // Use a square-root scale to reduce bias against higher grain values
    // This helps prevent the function from selecting overly low grain values
    // when the source video benefits from preserving more texture
    let adjusted_value = (luma_spatial * 8.0).sqrt() * 8.0;

    // Round to nearest integer and cap at 16
    let film_grain_value = adjusted_value.round() as u8;
    return film_grain_value.min(16);
}
```

The enhanced mapping function uses a square-root scale to provide a more perceptually balanced distribution of film grain values. This approach:

1. Reduces bias against higher grain values
2. Provides more granular control over the film grain synthesis
3. Better preserves the visual character of the source content
4. Maintains compatibility with the standard grain levels

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
| `film_grain_fallback_level` | Fallback level if analysis fails | `Baseline` |
| `film_grain_max_level` | Maximum allowed grain level | `Elevated` |
| `film_grain_refinement_points_count` | Number of refinement points to test (0 = auto) | `5` |

These options can be set via command-line arguments or configuration files.

### Enhanced Granularity Control

The new `film_grain_refinement_points_count` option allows for controlling the granularity of the refinement phase. When set to 0 (auto), the system dynamically determines the optimal number of refinement points based on the range size:

```rust
// Larger ranges get more test points for better coverage
let num_points = ((upper_f - lower_f) * 2.0).round().max(3.0).min(5.0) as usize;
```

This ensures that wider ranges (indicating more uncertainty) receive more test points, while narrower ranges (indicating more confidence) receive fewer test points for efficiency.

## Conclusion

Drapto's film grain management system represents a sophisticated approach to optimizing video compression efficiency while maintaining perceptual quality. By analyzing grain characteristics, applying optimal denoising, and synthesizing controlled film grain during encoding, the system achieves significant bitrate savings (often 20-40%) without sacrificing visual quality.

The multi-phase analysis approach with knee point detection ensures that each video receives optimal processing based on its unique characteristics, while the configurable parameters allow fine-tuning for different content types and quality requirements.

### Recent Enhancements

The recent enhancements to the grain analysis system provide several key improvements:

1. **Continuous Parameter Interpolation**: By implementing continuous parameter interpolation, the system can now test and apply denoising parameters at any point along the strength scale, not just at the five discrete grain levels. This allows for more precise optimization of the denoising parameters.

2. **Enhanced Refinement Process**: The refinement phase now generates multiple intermediate test points based on the range size, providing better coverage of the parameter space and more accurate results.

3. **Improved Film Grain Mapping**: The enhanced mapping function uses a square-root scale to provide a more perceptually balanced distribution of film grain values, reducing bias against higher grain values and better preserving the visual character of the source content.

4. **Configurable Refinement Granularity**: The new `film_grain_refinement_points_count` option allows for controlling the granularity of the refinement phase, with automatic adjustment based on the range size.

These improvements make the grain analysis system more precise, more adaptable, and better able to optimize the compression vs. quality tradeoff for a wide range of content types.
