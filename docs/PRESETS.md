# Drapto Presets

The `--drapto-preset` flag lets you apply a curated set of encoding values in one shot. Each preset is defined in `drapto-core/src/config/mod.rs` as a `DraptoPresetValues` constant, so editing the preset defaults is just a matter of changing literal numbers in that file.

## Baseline Defaults (no preset)

When you omit `--drapto-preset`, Drapto uses the global defaults embedded in `CoreConfig::default()`:

| Setting | Value | Notes |
|---------|-------|-------|
| `quality_sd / hd / uhd` | `25 / 27 / 29` | CRF by resolution |
| `svt_av1_preset` | `6` | Balanced speed/quality |
| `svt_av1_tune` | `0` | Matches upstream SVT defaults |
| `svt_av1_ac_bias` | `0.10` | Helps preserve detail without massive bitrate growth |
| `svt_av1_enable_variance_boost` | `false` | Adaptive quantization disabled by default |
| `svt_av1_variance_boost_strength` | `0` | N/A when variance boost is disabled |
| `svt_av1_variance_octile` | `0` | N/A when variance boost is disabled |
| `video_denoise_filter` | _(none)_ | No denoising filter unless a preset enables it |
| `svt_av1_film_grain` | _(none)_ | Film-grain synthesis disabled by default |
| `svt_av1_film_grain_denoise` | _(none)_ | N/A unless `svt_av1_film_grain` is set |

## Built-in Presets

| Preset | CRF (SD/HD/UHD) | SVT Preset | Tune | AC Bias | Var Boost | Boost Strength | Octile | Denoise (`-vf`) | Film Grain | Grain Denoise | Intent |
|--------|-----------------|------------|------|---------|-----------|----------------|--------|------------------|-----------|--------------|--------|
| `grain` | `25 / 27 / 29` | `6` | `0` | `0.10` | `true` | `1` | `5` | `hqdn3d=3:3:6:6` | `8` | `0` | Film-sourced or noisy captures: apply moderate denoising, then synthesize film grain for a balanced bitrate + "film look". |
| `clean` | `27 / 29 / 31` | `6` | `0` | `0.05` | `false` | `0` | `0` | _(none)_ | _(none)_ | _(none)_ | Target already clean/animated content; prioritizes speed/size. |
| `quick` | `32 / 35 / 36` | `8` | `0` | `0.00` | `false` | `0` | `0` | _(none)_ | _(none)_ | _(none)_ | Fast, non-archival encodes. |

Pass `--drapto-preset grain`, `clean`, or `quick` to apply one of these bundles before any per-flag overrides.

## Customizing Presets

1. Open `drapto-core/src/config/mod.rs`.
2. Locate the `DRAPTO_PRESET_GRAIN_VALUES` / `DRAPTO_PRESET_CLEAN_VALUES` / `DRAPTO_PRESET_QUICK_VALUES` constants.
3. Replace the literal values with the numbers you want. Example:

```rust
pub const DRAPTO_PRESET_GRAIN_VALUES: DraptoPresetValues = DraptoPresetValues {
    quality_sd: 21,
    quality_hd: 23,
    quality_uhd: 25,
    svt_av1_preset: 4,
    svt_av1_tune: 2,
    svt_av1_ac_bias: 0.45,
    svt_av1_enable_variance_boost: true,
    svt_av1_variance_boost_strength: 3,
    svt_av1_variance_octile: 4,
    video_denoise_filter: None,
    svt_av1_film_grain: None,
    svt_av1_film_grain_denoise: None,
};
```

4. Run `cargo test` (or `cargo build --release`) to ensure everything still compiles.

Remember: explicit CLI flags always win. If you run `--drapto-preset grain --quality-hd 28`, the HD CRF will be forced to 28 regardless of what the preset specifies.
