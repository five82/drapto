# Film Grain Analysis, Denoising, and Synthesis: Technical Documentation

## Overview

Drapto implements a sophisticated film grain management system that optimizes video compression efficiency while maintaining perceptual quality. This document provides a detailed technical explanation of the film grain analysis, denoising, and synthesis processes used in the Drapto system.

The core principle behind Drapto's approach is that film grain and noise consume a disproportionate amount of bitrate during video encoding due to their high-entropy, random nature. By selectively reducing natural grain before encoding and then adding back controlled synthetic grain, Drapto achieves significantly better compression efficiency without sacrificing perceptual quality.

## Purpose and Goals

### Primary Goal: File Size Reduction
The primary purpose of Drapto's grain management system is to **reduce file size** for home and mobile viewing while maintaining acceptable visual quality. This is achieved through intelligent denoising that removes high-entropy grain before encoding.

### Key Principles

1. **Practical Optimization**: The system is designed for real-world use cases where storage and bandwidth are considerations, not for archival or professional mastering.

2. **Conservative Defaults**: When the optimal denoising level is unclear, the system defaults to VeryLight denoising, which provides compression benefits (typically 5-10% file size reduction) with virtually no risk of visible quality loss.

3. **Grain Fidelity is Secondary**: While the system does add synthetic grain to maintain visual texture, exact reproduction of the original grain pattern is not the goal. The focus is on achieving a pleasing result that looks good on typical viewing devices.

4. **Always Some Benefit**: Based on video encoding best practices, even the lightest denoising almost always provides file size benefits by removing encoding artifacts and high-frequency noise that doesn't contribute to perceived quality.

### Target Use Cases

- Home media servers (Plex, Jellyfin, etc.)
- Mobile device storage optimization
- Streaming over limited bandwidth
- Personal video libraries where storage efficiency matters

The system now features continuous parameter interpolation for fine-grained control over denoising strength, allowing for more precise optimization of the compression vs. quality tradeoff.

## System Architecture

The film grain management system consists of three main components:

1. **Film Grain Analysis**: Detects and quantifies the amount of grain/noise in the source video
2. **Adaptive Denoising**: Applies optimal denoising parameters based on the analysis results
3. **Film Grain Synthesis**: Adds back controlled synthetic grain during encoding

This approach creates a balanced solution that preserves the visual character of content while dramatically improving compression efficiency.

## 1. Film Grain Analysis

### Streamlined Analysis Approach

The grain analysis module implements a streamlined approach to determine the optimal denoising parameters:

1. **Sample Extraction**: Multiple short samples are extracted from different parts of the video
2. **Comprehensive Testing**: Each sample is encoded with all six denoising levels plus baseline
3. **Quality Measurement**: XPSNR (eXtended Peak Signal-to-Noise Ratio) is calculated against the raw sample for all levels including baseline
4. **Knee Point Analysis**: File size reductions and quality metrics are analyzed using diminishing returns detection with delta-based quality factors
5. **Result Aggregation**: The final optimal denoising level is determined using the median

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
      -> VeryLight  size: 20.05 MB, XPSNR: 42.3 dB
      -> Light      size: 17.73 MB, XPSNR: 40.8 dB
      -> LightModerate size: 15.91 MB, XPSNR: 39.2 dB
      -> Moderate   size: 14.64 MB, XPSNR: 37.5 dB
      -> Elevated   size: 12.40 MB, XPSNR: 35.1 dB
```

Note: The output logs consistently use "Baseline" when referring to videos with no grain or when no denoising is applied.

### Grain Level Classification

The system classifies grain into six distinct levels:

| Grain Level | Description | Denoising Approach |
|-------------|-------------|-------------------|
| Baseline    | Very little to no grain | No denoising applied |
| VeryLight   | Barely noticeable grain | Very light denoising |
| Light       | Light grain | Light denoising |
| LightModerate | Light to moderate grain | Balanced denoising |
| Moderate    | Noticeable grain with spatial patterns | Spatially-focused denoising |
| Elevated    | Medium grain with temporal fluctuations | Temporally-focused denoising |

### Knee Point Detection Algorithm with Quality Awareness

The knee point detection algorithm finds the optimal balance between compression efficiency and visual quality preservation using a diminishing returns approach with XPSNR quality metrics:

1. **Reference Points**: Each sample is encoded with all denoising levels including baseline. Baseline metrics are displayed for transparency, while VeryLight serves as the reference for comparisons
2. **Quality-Adjusted Efficiency Calculation**: An efficiency metric is calculated for levels beyond VeryLight: `(size_reduction_from_verylight * quality_factor / sqrt(grain_level_value))`
3. **Delta-Based Quality Factor**: Based on XPSNR loss from VeryLight (our minimum denoise level):
   - Less than 0.5 dB loss: factor = 1.0 (virtually no perceptible difference)
   - 0.5-1 dB loss: factor = 0.95-1.0 (minimal quality loss)
   - 1-2 dB loss: factor = 0.85-0.95 (slight quality loss)
   - 2-3 dB loss: factor = 0.7-0.85 (moderate quality loss)
   - 3-5 dB loss: factor = 0.5-0.7 (significant quality loss)
   - More than 5 dB loss: factor < 0.5 (severe quality loss)
4. **Diminishing Returns Detection**: The algorithm calculates improvement rates between consecutive levels and stops when the rate drops below 25%
5. **Continuous Improvement Handling**: When efficiency continuously improves without diminishing returns, the algorithm selects Light as a balanced compromise
6. **Conservative Selection**: The algorithm prioritizes quality preservation while ensuring meaningful compression benefits

```rust
// Quality-adjusted efficiency calculation with delta-based quality factor
let xpsnr_delta = verylight_xpsnr - xpsnr;  // Quality loss from VeryLight in dB
let quality_factor = calculate_quality_factor_from_delta(xpsnr_delta);
let efficiency = (size_reduction_from_verylight * quality_factor) / (grain_level_value.sqrt());
```

#### Fallback Behavior

The algorithm handles different patterns intelligently:

1. **Diminishing Returns Found**: Selects the level before efficiency improvement drops below 25%
2. **Continuous Improvement**: When efficiency keeps increasing, selects Light as a balanced choice
3. **No Clear Pattern**: Defaults to VeryLight as the ultimate safe fallback

Example messages:
- `Sample 1: Baseline reference - 26.3 MB, XPSNR: 45.2 dB` - Shows baseline metrics for transparency
- `Sample 1: Selected Light (3.5% additional reduction, XPSNR: 42.8 dB, -0.8 dB from VeryLight)` - When diminishing returns detected
- `Sample 1: Selected Light (4.2% additional reduction, XPSNR: 42.5 dB, -1.1 dB from VeryLight) - continuous improvement` - When efficiency keeps improving
- `Sample 1: Using VeryLight (no better alternatives found)` - When no levels improve beyond VeryLight

This approach ensures that:
- Some denoising is always applied (following encoding best practices)
- The system errs on the side of quality preservation
- Users understand what decision was made and why

### Maximum Level Constraint

After the analysis determines an optimal grain level, a final safety check is applied:

```rust
if final_level > max_level {
    final_level = max_level;
}
```

This ensures that denoising never exceeds the user-configured maximum, even if the analysis suggests stronger denoising would be beneficial. The default `max_level` is Elevated, which provides a good balance for most content.

### Early Exit Optimization

To improve efficiency, the system implements early exit when consistent results are detected:

1. **Consistency Check**: After analyzing at least 3 samples, checks if all results agree
2. **Adjacent Level Detection**: Exits early if all samples fall within adjacent grain levels
3. **Time Savings**: Can reduce analysis time by 20-40% for consistent content
4. **Quality Maintained**: Early exit only occurs when additional samples wouldn't change the result

This optimization ensures faster processing without sacrificing accuracy, particularly beneficial when encoding multiple similar files.

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
| VeryLight   | `0.5:0.4:3:3` | Very light denoising with slight chroma enhancement |
| Light       | `0.9:0.7:4:4` | Light denoising with balanced spatial/temporal approach |
| LightModerate | `1.2:0.85:5:5` | Balanced denoising bridging light and moderate levels |
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
        (1.0, 0.5, 0.4, 3.0, 3.0),       // VeryLight
        (2.0, 0.9, 0.7, 4.0, 4.0),       // Light
        (2.5, 1.2, 0.85, 5.0, 5.0),      // LightModerate
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

Denoising levels are mapped to corresponding SVT-AV1 film grain synthesis values with even spacing:

| Grain Level | HQDN3D Parameters | Film Grain Value |
|-------------|-------------------|------------------|
| Baseline    | No parameters | 0 (no synthetic grain) |
| VeryLight   | `0.5:0.4:3:3` | 4 (very light synthetic grain) |
| Light       | `0.9:0.7:4:4` | 7 (light synthetic grain) |
| LightModerate | `1.2:0.85:5:5` | 10 (light to moderate synthetic grain) |
| Moderate    | `1.5:1.0:6:6` | 13 (moderate synthetic grain) |
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

    // Fixed mapping for standard levels (for optimization)
    for (params, film_grain) in &[
        ("hqdn3d=0.5:0.4:3:3", 4),       // VeryLight
        ("hqdn3d=0.9:0.7:4:4", 7),       // Light
        ("hqdn3d=1.2:0.85:5:5", 10),     // LightModerate
        ("hqdn3d=1.5:1.0:6:6", 13),      // Moderate
        ("hqdn3d=2:1.3:8:8", 16),        // Elevated
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
| `film_grain_max_level` | Maximum allowed grain level | `Elevated` |
| `film_grain_refinement_points_count` | Number of refinement points to test | `5` |

These options can be set via command-line arguments or configuration files.

### Enhanced Granularity Control

The new `film_grain_refinement_points_count` option allows for controlling the granularity of the refinement phase. When set to 0 (auto), the system dynamically determines the optimal number of refinement points based on the range size:

```rust
// Larger ranges get more test points for better coverage
let num_points = ((upper_f - lower_f) * 2.0).round().max(3.0).min(5.0) as usize;
```

This ensures that wider ranges (indicating more uncertainty) receive more test points, while narrower ranges (indicating more confidence) receive fewer test points for efficiency.

## Conclusion

Drapto's film grain management system provides a practical solution for reducing video file sizes while maintaining quality suitable for home and mobile viewing. The system prioritizes:

1. **Reliable File Size Reduction**: Typically achieving 20-40% smaller files through intelligent grain removal
2. **Safe Defaults**: Using VeryLight denoising when uncertain ensures compression benefits without quality risks
3. **User Control**: Configuration options like `max_level` provide guardrails against over-processing
4. **Transparency**: Clear messaging helps users understand what decisions are being made

The multi-phase analysis approach ensures consistent results across diverse content, while the conservative fallback behavior means the system will always provide some benefit rather than doing nothing when analysis is inconclusive.

### Recent Enhancements

The recent enhancements to the grain analysis system provide several key improvements:

1. **Additional Grain Level**: Added LightModerate level between Light and Moderate for better granularity in the common denoising range, improving the system's ability to find optimal settings.

2. **Refined HQDN3D Parameters**: Updated parameter progression for more even spacing:
   - VeryLight: Increased chroma denoising (0.3→0.4) for better color noise reduction
   - Light: Reduced luma denoising (1.0→0.9) for smoother progression
   - LightModerate: New balanced level at 1.2:0.85:5:5

3. **Even Film Grain Spacing**: Film grain synthesis values now use consistent 3-point gaps (4,7,10,13,16) for more predictable quality progression.

4. **Improved Knee Point Algorithm**: Enhanced with:
   - Diminishing returns detection (25% improvement threshold)
   - Continuous improvement pattern recognition
   - Intelligent selection of Light when efficiency keeps increasing
   - Clear reporting of selection reasoning

5. **Early Exit Optimization**: Added intelligent early exit when samples show consistent results, reducing analysis time by 20-40% without sacrificing accuracy.

6. **Simplified Architecture**: Removed the adaptive refinement phase, reducing complexity while maintaining accuracy through better initial level coverage.

7. **XPSNR Quality Metrics**: Integrated XPSNR calculations to measure quality for all encoding levels. Baseline metrics are displayed for transparency, while quality factors are calculated using delta from VeryLight XPSNR. This approach aligns with the system's design where VeryLight is the minimum denoising level, focusing decisions on whether additional denoising beyond VeryLight provides worthwhile benefits. The system adapts to any CRF value by using relative measurements rather than absolute quality thresholds.

These improvements make the grain analysis system faster, more predictable, and better aligned with the goal of practical file size reduction for home viewing.
