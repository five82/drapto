# Drapto Presets

The `--drapto-preset` flag lets you apply a curated set of encoding values in one shot. Each preset is defined in `internal/config/config.go` as a `PresetValues` struct.

## Baseline Defaults (no preset)

When you omit `--drapto-preset`, Drapto uses the global defaults embedded in `NewConfig()`:

| Setting | Value | Notes |
|---------|-------|-------|
| `CRFSD / HD / UHD` | `25 / 27 / 29` | CRF by resolution tier |
| `SVTAV1Preset` | `6` | Balanced speed/quality (0-13 scale, lower = slower/better) |
| `SVTAV1Tune` | `0` | Matches upstream SVT defaults |
| `SVTAV1ACBias` | `0.10` | Helps preserve detail without massive bitrate growth |
| `SVTAV1EnableVarianceBoost` | `false` | Adaptive quantization disabled by default |
| `VideoDenoiseFilter` | _(none)_ | No denoising filter unless explicitly set |
| `SVTAV1FilmGrain` | _(none)_ | Film-grain synthesis disabled by default |

Resolution tiers: SD (<1920 width), HD (1920-3839 width), UHD (≥3840 width)

## Built-in Presets

| Preset | CRF (SD/HD/UHD) | SVT Preset | AC Bias | Intent |
|--------|-----------------|------------|---------|--------|
| `grain` | `25 / 27 / 29` | `6` | `0.10` | Currently matches defaults; placeholder for future film-grain tuning. |
| `clean` | `27 / 29 / 31` | `6` | `0.05` | Target already clean/animated content; prioritizes speed/size. |
| `quick` | `32 / 35 / 36` | `8` | `0.10` | Fast, non-archival encodes. |

Pass `--drapto-preset grain`, `clean`, or `quick` to apply one of these bundles before any per-flag overrides.

## Customizing Presets

1. Open `internal/config/config.go`.
2. Locate the `GetPresetValues()` function and find the `PresetGrain` / `PresetClean` / `PresetQuick` cases.
3. Replace the literal values with the numbers you want. Example:

```go
case PresetGrain:
    return PresetValues{
        QualitySD:                   21,
        QualityHD:                   23,
        QualityUHD:                  25,
        SVTAV1Preset:                4,
        SVTAV1Tune:                  2,
        SVTAV1ACBias:                0.45,
        SVTAV1EnableVarianceBoost:   true,
        SVTAV1VarianceBoostStrength: 3,
        SVTAV1VarianceOctile:        4,
    }
```

4. Run `go test ./...` (or `go build ./...`) to ensure everything still compiles.

Remember: explicit CLI flags always win. If you run `--drapto-preset grain --crf 25,28,29`, the HD CRF will be forced to 28 regardless of what the preset specifies.
