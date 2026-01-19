# Target Quality Encoding

Target Quality (TQ) encoding uses GPU-accelerated SSIMULACRA2 to iteratively find the optimal CRF for each chunk, ensuring consistent perceptual quality across your entire encode rather than consistent bitrate.

## Quick Start

```bash
# Basic usage - target SSIMULACRA2 score of 75-80
drapto encode -i input.mkv -o output/ -t 75-80

# With multiple encoder workers
drapto encode -i input.mkv -o output/ -t 75-80 --workers 8
```

## Requirements

- **NVIDIA GPU** with CUDA support (tested on RTX 30/40/50 series)
- **CUDA 13.x** with compatible driver (580+)
- **libvship** installed at `/usr/local/lib/libvship.so`
- **FFMS2** for frame-accurate video indexing

## How It Works

### Traditional CRF Encoding
With fixed CRF, simple scenes get more quality than needed while complex scenes may not get enough. A dark, static dialogue scene encoded at CRF 25 might achieve SSIMULACRA2 85, while an action scene at the same CRF might only reach 65.

### Target Quality Encoding
TQ encoding flips this around: you specify the quality level you want, and the encoder finds the CRF that achieves it for each chunk independently.

1. **Initial probes**: Binary search tests CRF values at the bounds and midpoint
2. **Interpolation**: Spline interpolation predicts the CRF likely to hit the target
3. **Refinement**: Additional probes refine the prediction until convergence
4. **Convergence**: When the score falls within your target range, that CRF is used

This means simple scenes get higher CRF (smaller files) while complex scenes get lower CRF (more bits), resulting in consistent visual quality throughout.

## SSIMULACRA2 Score Reference

SSIMULACRA2 is currently the most accurate perceptual quality metric, correlating better with human vision than VMAF, SSIM, or PSNR. Scores range from -∞ to 100:

| Score | Quality Level | Description |
|-------|---------------|-------------|
| 90+ | Visually lossless | Not noticeable in flicker test at 1:1 from normal viewing distance |
| 85 | Excellent | Virtually impossible to distinguish in flip test |
| 80 | Very high | Not noticeable by average observer in side-by-side at 1:1 |
| 70 | High | Artifacts perceptible but not annoying |
| 50 | Medium | Slightly annoying artifacts |
| 30 | Low | Obvious and annoying artifacts |

**Key insight for video**: The "visually lossless" threshold for video is approximately **80** rather than 90, because individual frame artifacts are harder to notice during playback than in still images.

## Recommended Settings

### For Jellyfin/Plex Streaming at Normal Viewing Distance

These recommendations assume viewing from a typical living room distance (8-12 feet from a 55-65" TV), which is approximately 2-3x screen height.

#### DVD Content (480p/576p)
```bash
drapto encode -i dvd_rip.mkv -o output/ -t 68-72
```
- **Target**: 68-72
- **Rationale**: DVD's limited resolution means artifacts are less visible. Lower targets save significant space without perceptible quality loss at normal viewing distances.

#### Blu-ray Content (1080p)
```bash
drapto encode -i bluray_rip.mkv -o output/ -t 75-80
```
- **Target**: 75-80
- **Rationale**: Near-transparent quality at typical viewing distances. For storage-constrained libraries, 70-75 is acceptable but artifacts may be perceptible on close inspection or smaller screens.

#### 4K UHD Blu-ray Content (2160p SDR)
```bash
drapto encode -i uhd_rip.mkv -o output/ -t 75-80
```
- **Target**: 75-80
- **Rationale**: Higher resolution benefits from higher targets. The increased pixel density at 4K makes subtle artifacts more visible on large displays.

#### 4K HDR Content
```bash
drapto encode -i uhd_hdr.mkv -o output/ -t 78-82 --metric-mode p5
```
- **Target**: 78-82
- **Rationale**: HDR's expanded dynamic range reveals compression artifacts more readily, particularly banding in dark scenes and color gradients. Using `--metric-mode p5` ensures the worst frames (often dark HDR scenes) meet quality standards rather than being averaged away.

### Quality Tiers

| Use Case | Target Range | Notes |
|----------|--------------|-------|
| Storage-optimized | 68-72 | Artifacts perceptible on inspection, acceptable for casual viewing |
| Balanced (recommended) | 75-80 | Near-transparent at normal viewing distances |
| High quality | 80-85 | Visually indistinguishable from source for video |
| Reference | 85-90 | Maximum fidelity, diminishing returns on file size |

### Content-Specific Adjustments

**Animated content**: Western 3D animation (Pixar, DreamWorks) can use lower targets (68-72) as it compresses efficiently. **Anime and cel-style animation** should use standard or higher targets (75-80) because flat gradients and sky backgrounds are prone to banding artifacts.

**Film grain / noisy content**: Consider using slightly higher targets (add 2-3 points) as grain is difficult to preserve without sufficient bitrate. Alternatively, rely on AV1's film grain synthesis (enabled by default in drapto).

**Dark/shadow-heavy content**: Dark scenes can reveal banding artifacts. Consider higher targets (+2-3 points) for content with significant low-light footage.

**Fast action / sports**: Complex motion benefits from higher targets to preserve detail during rapid movement.

## CLI Options

```
Target Quality Options:
  -t, --target <RANGE>       Target SSIMULACRA2 quality range (e.g., "70-75")
  --qp <RANGE>               CRF search range. Default: 8-48
  --metric-workers <N>       Number of GPU metric workers. Default: 1
  --metric-mode <MODE>       Metric aggregation mode. Default: mean

Sample-Based Probing Options:
  --sample-duration <N>      Seconds to sample for TQ probing. Default: 3.0
  --sample-min-chunk <N>     Minimum chunk duration to use sampling. Default: 6.0
  --no-tq-sampling           Disable sample-based probing (use full chunks)
```

### --target (required for TQ mode)
The SSIMULACRA2 score range to target. Format: `min-max`

```bash
-t 75-80    # Target scores between 75 and 80
-t 77-78    # Narrow range for precise targeting
```

Narrower ranges require more iterations to converge but provide more consistent quality. A range of 3-5 points is recommended.

### --qp
The CRF search bounds. Default: `8-48`

```bash
--qp 15-40   # Restrict CRF search to 15-40 range
```

Narrowing this range can speed up convergence if you know approximately what CRF range your content needs.

### --metric-workers
Number of parallel GPU metric computation workers. Default: `1`

```bash
--metric-workers 2   # Use 2 GPU workers
```

Multiple workers can improve throughput on high-end GPUs with sufficient VRAM. Start with 1 and increase if GPU utilization is low.

### --metric-mode
How to aggregate per-frame scores into a chunk score. Default: `mean`

- `mean`: Average of all frame scores (recommended for most content)
- `pN`: Mean of the worst N% of frames (e.g., `p5` = worst 5%)

```bash
--metric-mode p5    # Target the worst 5% of frames
```

Using `p5` or `p10` ensures even the worst frames meet quality standards, useful for content with highly variable complexity.

### --sample-duration
Duration in seconds to sample from each chunk for TQ probing. Default: `3.0`

```bash
--sample-duration 5.0   # Sample 5 seconds per chunk
```

Sample-based probing encodes only a portion of each chunk during quality search, then encodes the full chunk at the discovered CRF. This significantly speeds up the quality search process.

### --sample-min-chunk
Minimum chunk duration in seconds required to use sampling. Default: `6.0`

```bash
--sample-min-chunk 10.0   # Only sample chunks longer than 10 seconds
```

For chunks shorter than this threshold, full-chunk probing is used regardless of `--sample-duration`. This ensures representative quality measurements on short scenes.

### --no-tq-sampling
Disable sample-based probing entirely. When set, full chunks are used for all TQ probing iterations.

```bash
--no-tq-sampling   # Always probe full chunks (slower but more accurate)
```

Use this if you notice quality inconsistencies with sample-based probing, particularly for content with high temporal variation within scenes.

## Performance Considerations

### Encoding Speed
TQ encoding is slower than fixed-CRF encoding because:
1. Each chunk requires multiple encode attempts to find the optimal CRF
2. Each attempt requires GPU metric computation

Typical overhead is 2-4x slower than single-pass encoding, depending on:
- How quickly chunks converge to the target
- Target range width (narrower = more iterations)
- Content complexity variation

### GPU Utilization
VSHIP uses the GPU for SSIMULACRA2 computation. During metric calculation:
- GPU handles color space conversion and metric computation
- VRAM usage scales with resolution (~500MB for 1080p, ~2GB for 4K)

### Optimal Worker Configuration
```bash
# Balanced configuration for most systems
drapto encode -i input.mkv -o output/ -t 75-80 --workers 8 --metric-workers 1

# High-end GPU with abundant VRAM
drapto encode -i input.mkv -o output/ -t 75-80 --workers 12 --metric-workers 2
```

## Comparison with Other Tools

| Tool | Metric Support | GPU Acceleration |
|------|----------------|------------------|
| drapto | SSIMULACRA2 | Yes (VSHIP) |
| Av1an | SSIMULACRA2, VMAF | CPU only |
| xav | SSIMULACRA2, CVVDP, Butteraugli | Yes (VSHIP) |
| ab-av1 | VMAF | CPU only |

Drapto's implementation is derived from xav's approach, using VSHIP for GPU-accelerated metrics.

## Troubleshooting

### "Failed to initialize VSHIP device"
- Verify NVIDIA driver version supports your CUDA toolkit (CUDA 13.x requires driver 580+)
- Check `nvidia-smi` shows your GPU
- Ensure libvship.so is the CUDA build, not HIP/AMD

### Slow convergence
- Widen the target range (e.g., 75-81 instead of 77-79)
- Narrow the QP search range if you know typical CRF values
- Use `--metric-mode mean` instead of percentile modes

### Quality varies between chunks
This is expected behavior - TQ targets consistent *perceptual* quality, not consistent bitrate or CRF. Simple chunks will have higher CRF, complex chunks will have lower CRF.

## References

- [SSIMULACRA2 - Codec Wiki](https://wiki.x266.mov/docs/metrics/SSIMULACRA2)
- [Cloudinary SSIMULACRA2](https://github.com/cloudinary/ssimulacra2)
- [Av1an Target Quality](https://rust-av.github.io/Av1an/Features/TargetQuality.html)
- [VSHIP GPU Metrics](https://github.com/Line-fr/Vship)
- [SVT-AV1 Encoding Guide](https://gist.github.com/dvaupel/716598fc9e7c2d436b54ae00f7a34b95)
