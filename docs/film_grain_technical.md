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

## System Architecture

The film grain management system consists of three main components:

1. **Film Grain Analysis**: Detects and quantifies the amount of grain/noise in the source video
2. **Adaptive Denoising**: Applies optimal denoising parameters based on the analysis results
3. **Film Grain Synthesis**: Adds back controlled synthetic grain during encoding

This approach creates a balanced solution that preserves the visual character of content while dramatically improving compression efficiency.

## 1. Film Grain Analysis

### Optimized Analysis Approach

The grain analysis module implements an efficient approach to determine the optimal denoising parameters:

1. **Sample Extraction**: Multiple short samples are extracted from different parts of the video
2. **Binary Search Testing**: Uses binary search to efficiently find the optimal denoising level
3. **Quality Measurement**: XPSNR (eXtended Peak Signal-to-Noise Ratio) is calculated against baseline
4. **Progressive Elimination**: Reduces search space for subsequent samples based on previous results
5. **Conservative Selection**: Final level is the minimum (most conservative) across all samples

### Sample Extraction Logic

- The number of samples is dynamically calculated based on video duration
- Samples are randomly distributed between 15% and 85% of video duration
- Each sample is typically 10 seconds long (configurable via `film_grain_sample_duration`)
- A minimum of 3 and maximum of 7 samples are used

```rust
// Sample count calculation
let base_samples = (duration_secs / SECS_PER_SAMPLE_TARGET).ceil() as usize;
let num_samples = base_samples.clamp(MIN_SAMPLES, MAX_SAMPLES);
```

### Binary Search Optimization

Instead of testing all denoising levels, the system uses binary search:

1. **Always test baseline first** - Establishes the reference XPSNR
2. **Start with middle level** - Tests the middle of available grain levels
3. **Binary search based on XPSNR threshold** - If within 1.2 dB, try more aggressive; otherwise try less aggressive
4. **Typically reduces tests from 6 to 3** - Significantly faster analysis

### Progressive Elimination

After each sample is analyzed:
- If a level lower than the current maximum is selected, update the maximum
- Subsequent samples only test levels up to this maximum
- If VeryLight is selected, analysis stops immediately (no lower levels exist)

Example progression:
- Sample 1: Tests all levels, selects Light → max_level = Light
- Sample 2: Only tests VeryLight and Light
- Sample 3: If VeryLight selected → stop analysis

### Sample Analysis Output

During analysis, the system outputs the results in a streamlined format:

```
Sample 1/6 at 1394.5s:
  Baseline:    10.7 MB, XPSNR: 24.1 dB
Sample 1: Baseline reference - 10.7 MB, XPSNR: 24.1 dB
  Light-Mod:   9.2 MB, XPSNR: 24.2 dB
  Elevated:    8.5 MB, XPSNR: 24.4 dB
Sample 1: Selected Elevated (20.7% size reduction, XPSNR: +0.3 dB improvement from baseline)
```

Note: When XPSNR improves with denoising, it indicates noisy/grainy content (possibly dark scenes).

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

### XPSNR Threshold-Based Selection

The selection algorithm is straightforward:

1. **Calculate XPSNR drop from baseline** for each tested level
2. **Select the most aggressive level** where XPSNR drop ≤ 1.2 dB
3. **Conservative final selection** - Use minimum level across all samples

This ensures quality is maintained across the entire video, not just at sample points.

### Final Level Selection Transparency

The system clearly shows why the final level was selected:

```
✓ Grain analysis complete
  Light selected - most conservative of: Elevated, Elevated, Elevated, Elevated, Light, Light
  Detected grain: Light - applying appropriate denoising
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
| VeryLight   | `0.5:0.4:2:2` | Very light denoising with minimal temporal filtering |
| Light       | `0.7:0.55:3:3` | Light denoising with balanced spatial/temporal approach |
| LightModerate | `1.0:0.8:5:5` | Balanced denoising bridging light and moderate levels |
| Moderate    | `1.4:1.05:6:6` | Moderate denoising with higher temporal values |
| Elevated    | `2.5:1.8:10:10` | Stronger denoising with emphasis on temporal filtering |

These parameters are deliberately conservative to avoid excessive blurring while still improving compression efficiency.

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
| VeryLight   | `0.5:0.4:2:2` | 4 (very light synthetic grain) |
| Light       | `0.7:0.55:3:3` | 7 (light synthetic grain) |
| LightModerate | `1.0:0.8:5:5` | 10 (light to moderate synthetic grain) |
| Moderate    | `1.4:1.05:6:6` | 13 (moderate synthetic grain) |
| Elevated    | `2.5:1.8:10:10` | 16 (medium synthetic grain) |

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

Drapto provides several configuration options to fine-tune the grain analysis process:

| Option | Description | Default |
|--------|-------------|---------|
| `enable_denoise` | Enable/disable the entire grain management system | `true` |
| `film_grain_sample_duration` | Duration in seconds for each sample | `10` |
| `film_grain_max_level` | Maximum allowed grain level | `Elevated` |

The XPSNR threshold is fixed at 1.2 dB.

## Performance Optimizations

The current implementation includes several optimizations that significantly reduce analysis time:

1. **Binary Search**: Reduces encode tests from ~36 to ~12-15 (60-70% reduction)
2. **Progressive Elimination**: Further reduces tests as analysis progresses
3. **VeryLight Early Exit**: Stops all analysis if minimum level is reached
4. **Efficient Sample Count**: Uses 3-7 samples based on duration, not fixed count

Example efficiency gain:
- Old approach: 6 levels × 6 samples = 36 encodes
- New approach: ~3 levels × progressively fewer = ~12-15 encodes

## Conclusion

Drapto's film grain management system provides a practical solution for reducing video file sizes while maintaining quality suitable for home and mobile viewing. The system prioritizes:

1. **Reliable File Size Reduction**: Typically achieving 20-40% smaller files through intelligent grain removal
2. **Safe Defaults**: Using VeryLight denoising when uncertain ensures compression benefits without quality risks
3. **Performance**: Binary search and progressive elimination dramatically reduce analysis time
4. **Transparency**: Clear messaging helps users understand what decisions are being made

The simplified XPSNR threshold approach ensures consistent results across diverse content, while the conservative final selection means the system will always preserve quality across the entire video.