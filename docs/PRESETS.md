# Drapto Presets

The `--drapto-preset` flag lets you apply a curated set of encoding values in one shot. Each preset is defined in `drapto-core/src/config/mod.rs` as a `DraptoPresetValues` constant, so editing the preset defaults is just a matter of changing literal numbers in that file.

## Baseline Defaults (no preset)

When you omit `--drapto-preset`, Drapto uses the global defaults embedded in `CoreConfig::default()`:

| Setting | Value | Notes |
|---------|-------|-------|
| `quality_sd / hd / uhd` | `24 / 26 / 28` | CRF by resolution |
| `svt_av1_preset` | `6` | Balanced speed/quality |
| `svt_av1_tune` | `0` | Matches upstream SVT defaults |
| `svt_av1_ac_bias` | `0.30` | Helps preserve detail without massive bitrate growth |
| `svt_av1_enable_variance_boost` | `true` | Adaptive quantization enabled |
| `svt_av1_variance_boost_strength` | `1` | Moderate strength |
| `svt_av1_variance_octile` | `6` | Bias toward darker/brighter regions |

## Built-in Presets

| Preset | CRF (SD/HD/UHD) | SVT Preset | Tune | AC Bias | Var Boost | Boost Strength | Octile | Intent |
|--------|-----------------|------------|------|---------|-----------|----------------|--------|--------|
| `grain` | `22 / 24 / 26` | `5` | `0` | `0.50` | `true` | `2` | `5` | Preserve texture and film grain even at the cost of extra bitrate/time. |
| `clean` | `26 / 28 / 30` | `6` | `0` | `0.20` | `false` | `0` | `0` | Target already clean/animated content; prioritizes speed/size. |
| `quick` | `32 / 35 / 36` | `8` | `0` | `0.00` | `false` | `0` | `0` | Fast, non-archival encodes. |

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
};
```

4. Run `cargo test` (or `cargo build --release`) to ensure everything still compiles.

Remember: explicit CLI flags always win. If you run `--drapto-preset grain --quality-hd 28`, the HD CRF will be forced to 28 regardless of what the preset specifies.
