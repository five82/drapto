# Implementation Plan: Adding Adaptive Refinement and Knee Point Detection to Grain Analysis

## 1. Introduction and Goals

This document outlines a plan to enhance the grain analysis system in `drapto-core` by incorporating two key techniques from the reference implementation:

1. **Knee Point Detection Algorithm**: A more sophisticated method for determining optimal grain levels that avoids over-denoising.
2. **Adaptive Refinement**: A technique to dynamically adjust and test additional denoise parameters based on initial results.

The goal is to make the grain detection more accurate and versatile while maintaining the strengths of our current implementation.

## 2. Conceptual Approach

Our current implementation uses predefined hqdn3d parameter sets and a simple diminishing returns analysis. The enhanced approach will:

1. Start with the same predefined parameter sets for initial testing
2. Use the knee point algorithm to analyze initial results
3. Add a refinement phase that tests interpolated parameters between our predefined settings
4. Apply the knee point algorithm again to the combined results
5. Determine the final grain level while preserving our categorical approach

## 3. Implementation Details

### 3.1 Adding Knee Point Detection

#### A. Create a new function in `grain_analysis.rs`:

```rust
/// Calculates the optimal grain level for a single sample based on a knee point analysis.
/// Returns the GrainLevel that provides the best compression efficiency.
fn analyze_sample_with_knee_point(
    results: &HashMap<Option<GrainLevel>, u64>, 
    knee_threshold: f64,
    log_callback: &mut impl Fn(&str)
) -> GrainLevel {
    // Implementation based on reference code's calculate_knee_point_grain
    
    // Find baseline size (no denoise)
    let baseline_size = match results.get(&None) {
        Some(&size) => size,
        None => {
            log_callback("Baseline encode size missing from results. Cannot analyze with knee point.");
            return GrainLevel::VeryClean;
        }
    };

    // Map GrainLevel to a numerical scale for calculations
    // VeryLight -> 1, Light -> 2, Visible -> 3, Heavy -> 4
    let to_numeric = |level: Option<GrainLevel>| -> f64 {
        match level {
            None => 0.0,
            Some(GrainLevel::VeryClean) => 0.0,
            Some(GrainLevel::VeryLight) => 1.0,
            Some(GrainLevel::Light) => 2.0,
            Some(GrainLevel::Visible) => 3.0,
            Some(GrainLevel::Heavy) => 4.0,
        }
    };

    // Collect efficiencies using adjusted metric: reduction divided by sqrt(grain_value)
    let mut efficiencies: Vec<(Option<GrainLevel>, f64)> = Vec::new();
    
    for (&level, &size) in results.iter().filter(|(&k, &v)| v > 0 && k.is_some()) {
        let grain_numeric = to_numeric(level);
        if grain_numeric <= 0.0 {
            continue; // Skip baseline or VeryClean
        }
        
        let reduction = baseline_size.saturating_sub(size) as f64;
        if reduction <= 0.0 {
            continue; // Only consider tests with positive size reduction
        }
        
        // Use square-root scale to reduce bias against higher levels
        let adjusted_denom = grain_numeric.sqrt();
        let efficiency = reduction / adjusted_denom;
        
        if efficiency > 0.0 {
            efficiencies.push((level, efficiency));
        }
    }

    if efficiencies.is_empty() {
        log_callback("No positive efficiency improvements found with knee point analysis.");
        return GrainLevel::VeryClean;
    }

    // Find the maximum efficiency
    let (max_level, max_efficiency) = efficiencies.iter()
        .fold((None, 0.0), |acc, &(level, eff)| {
            if eff > acc.1 && eff.is_finite() {
                (level, eff)
            } else {
                acc
            }
        });

    if max_efficiency <= 0.0 {
        log_callback(&format!(
            "Max efficiency is not positive (Max: {:.2}). Using VeryClean.",
            max_efficiency
        ));
        return GrainLevel::VeryClean;
    }

    // Find all candidates meeting the threshold
    let threshold_efficiency = knee_threshold * max_efficiency;
    let mut candidates: Vec<(Option<GrainLevel>, f64)> = efficiencies
        .into_iter()
        .filter(|&(_, eff)| eff.is_finite() && eff >= threshold_efficiency)
        .collect();

    // Sort candidates by lowest grain level first (VeryLight before Light, etc.)
    candidates.sort_by(|a, b| {
        let a_val = to_numeric(a.0);
        let b_val = to_numeric(b.0);
        a_val.partial_cmp(&b_val).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Choose the lowest level that meets the threshold
    if let Some(&(Some(level), _)) = candidates.first() {
        log_callback(&format!(
            "Knee point analysis: Max efficiency {:.2} at level {:?}. Threshold {:.1}%. Choosing: {:?}",
            max_efficiency,
            max_level,
            knee_threshold * 100.0,
            level
        ));
        level
    } else {
        log_callback(&format!(
            "No suitable candidates found in knee point analysis. Using VeryClean."
        ));
        GrainLevel::VeryClean
    }
}
```

#### B. Add constants for knee point configuration:

```rust
// Add to const section of grain_analysis.rs
const KNEE_THRESHOLD: f64 = 0.8; // 80% efficiency threshold (from reference code)
```

### 3.2 Adding Adaptive Refinement Phase

#### A. Create an interpolation function for hqdn3d parameters:

```rust
/// Interpolates hqdn3d parameters between two predefined GrainLevels.
/// Returns a new param string.
fn interpolate_hqdn3d_params(lower_level: GrainLevel, upper_level: GrainLevel, factor: f32) -> String {
    // Get param strings for the two levels
    let lower_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == lower_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0");
    
    let upper_params = HQDN3D_PARAMS.iter()
        .find(|(level, _)| *level == upper_level)
        .map(|(_, params)| *params)
        .unwrap_or("hqdn3d=0:0:0:0");
    
    // Parse the parameters into components
    let l_components = parse_hqdn3d_params(lower_params);
    let u_components = parse_hqdn3d_params(upper_params);
    
    // Interpolate each component
    let new_luma_spatial = l_components.0 + (u_components.0 - l_components.0) * factor;
    let new_chroma_spatial = l_components.1 + (u_components.1 - l_components.1) * factor;
    let new_luma_tmp = l_components.2 + (u_components.2 - l_components.2) * factor;
    let new_chroma_tmp = l_components.3 + (u_components.3 - l_components.3) * factor;
    
    format!("hqdn3d={:.1}:{:.1}:{:.0}:{:.0}", 
        new_luma_spatial, new_chroma_spatial, new_luma_tmp, new_chroma_tmp)
}

/// Helper to parse hqdn3d parameter string into components.
/// Returns (luma_spatial, chroma_spatial, luma_tmp, chroma_tmp)
fn parse_hqdn3d_params(params: &str) -> (f32, f32, f32, f32) {
    let parts: Vec<&str> = params.trim_start_matches("hqdn3d=").split(':').collect();
    if parts.len() == 4 {
        (
            parts[0].parse().unwrap_or(0.0),
            parts[1].parse().unwrap_or(0.0),
            parts[2].parse().unwrap_or(0.0),
            parts[3].parse().unwrap_or(0.0),
        )
    } else {
        (0.0, 0.0, 0.0, 0.0) // Default values if parsing fails
    }
}
```

#### B. Add function to calculate standard deviation and adaptive refinement range:

```rust
/// Calculate standard deviation of grain level estimates.
/// Returns the standard deviation as f64.
fn calculate_std_dev(levels: &[GrainLevel]) -> f64 {
    if levels.len() <= 1 {
        return 0.0;
    }
    
    // Convert GrainLevel to numeric values
    let numeric_values: Vec<f64> = levels.iter().map(|level| match level {
        GrainLevel::VeryClean => 0.0,
        GrainLevel::VeryLight => 1.0,
        GrainLevel::Light => 2.0,
        GrainLevel::Visible => 3.0,
        GrainLevel::Heavy => 4.0,
    }).collect();
    
    // Calculate mean
    let mean: f64 = numeric_values.iter().sum::<f64>() / numeric_values.len() as f64;
    
    // Calculate variance
    let variance: f64 = numeric_values.iter()
        .map(|val| (val - mean).powi(2))
        .sum::<f64>() / numeric_values.len() as f64;
    
    // Return standard deviation
    variance.sqrt()
}

/// Determine the refinement range based on initial estimates.
/// Returns a tuple with (lower bound GrainLevel, upper bound GrainLevel).
fn calculate_refinement_range(initial_estimates: &[GrainLevel]) -> (GrainLevel, GrainLevel) {
    if initial_estimates.is_empty() {
        return (GrainLevel::VeryClean, GrainLevel::VeryLight);
    }
    
    // Create a copy and sort for median calculation
    let mut sorted_estimates = initial_estimates.to_vec();
    sorted_estimates.sort();
    
    // Get median
    let median_idx = sorted_estimates.len() / 2;
    let median = sorted_estimates[median_idx];
    
    // Calculate std dev and use it to determine range width
    let std_dev = calculate_std_dev(initial_estimates);
    const ADAPTIVE_FACTOR: f64 = 1.5; // From reference code
    let level_delta = (std_dev * ADAPTIVE_FACTOR).round() as usize;
    let level_delta = level_delta.max(1); // At least 1 level
    
    // Convert median GrainLevel to index
    let median_index = match median {
        GrainLevel::VeryClean => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Visible => 3,
        GrainLevel::Heavy => 4,
    };
    
    // Calculate lower and upper bounds, clamping to valid indices
    let lower_idx = median_index.saturating_sub(level_delta);
    let upper_idx = (median_index + level_delta).min(4); // 4 is max index (Heavy)
    
    // Convert indices back to GrainLevels
    let lower_level = match lower_idx {
        0 => GrainLevel::VeryClean,
        1 => GrainLevel::VeryLight,
        2 => GrainLevel::Light,
        3 => GrainLevel::Visible,
        _ => GrainLevel::Heavy,
    };
    
    let upper_level = match upper_idx {
        0 => GrainLevel::VeryClean,
        1 => GrainLevel::VeryLight,
        2 => GrainLevel::Light,
        3 => GrainLevel::Visible,
        _ => GrainLevel::Heavy,
    };
    
    (lower_level, upper_level)
}

/// Generate refinement parameters between lower and upper bound levels.
/// Returns a vector of (GrainLevel, hqdn3d_params) pairs.
fn generate_refinement_params(
    lower_level: GrainLevel, 
    upper_level: GrainLevel,
    initial_params: &[(Option<GrainLevel>, Option<&str>)]
) -> Vec<(Option<GrainLevel>, String)> {
    let mut refined_params = Vec::new();
    
    // If bounds are the same or too close, nothing to refine
    if lower_level == upper_level || 
       (lower_level == GrainLevel::VeryClean && upper_level == GrainLevel::VeryLight) ||
       (lower_level == GrainLevel::VeryLight && upper_level == GrainLevel::Light) ||
       (lower_level == GrainLevel::Light && upper_level == GrainLevel::Visible) ||
       (lower_level == GrainLevel::Visible && upper_level == GrainLevel::Heavy) {
        return refined_params;
    }
    
    // Convert levels to numeric for calculation
    let lower_idx = match lower_level {
        GrainLevel::VeryClean => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Visible => 3,
        GrainLevel::Heavy => 4,
    };
    
    let upper_idx = match upper_level {
        GrainLevel::VeryClean => 0,
        GrainLevel::VeryLight => 1,
        GrainLevel::Light => 2,
        GrainLevel::Visible => 3,
        GrainLevel::Heavy => 4,
    };
    
    // Skip if bounds are adjacent or invalid
    if upper_idx <= lower_idx + 1 {
        return refined_params;
    }
    
    // Generate two intermediate points between lower and upper
    // Use 1/3 and 2/3 positions for a nice distribution
    let range = upper_idx - lower_idx;
    let num_points = range - 1; // Generate all intermediate points
    
    for i in 1..=num_points {
        let factor = i as f32 / (range as f32);
        
        // Map the interpolation factor to a GrainLevel for labeling
        let interpolated_level_idx = lower_idx + i;
        let interpolated_level = match interpolated_level_idx {
            1 => GrainLevel::VeryLight,
            2 => GrainLevel::Light,
            3 => GrainLevel::Visible,
            4 => GrainLevel::Heavy,
            _ => continue, // Skip invalid indices
        };
        
        // Skip if this level was already tested in the initial phase
        if initial_params.iter().any(|(level, _)| level == &Some(interpolated_level)) {
            continue;
        }
        
        // Interpolate parameters
        let params = interpolate_hqdn3d_params(lower_level, upper_level, factor);
        refined_params.push((Some(interpolated_level), params));
    }
    
    refined_params
}
```

### 3.3 Modifying Main Analysis Logic

Update the main `analyze_grain` function to incorporate the knee point and adaptive refinement:

```rust
pub fn analyze_grain<S: FfmpegSpawner, P: FileMetadataProvider>(
    file_path: &Path,
    config: &CoreConfig,
    base_encode_params: &EncodeParams,
    duration_secs: f64,
    spawner: &S,
    metadata_provider: &P,
) -> CoreResult<Option<GrainAnalysisResult>> {
    // Initial setup remains the same...
    
    // --- Phase 1: Test Initial Values ---
    let mut phase1_results: Vec<HashMap<Option<GrainLevel>, u64>> = Vec::with_capacity(num_samples);
    
    // Define test levels including baseline (None) and mapped levels
    let initial_test_levels: Vec<(Option<GrainLevel>, Option<&str>)> = std::iter::once((None, None))
        .chain(HQDN3D_PARAMS.iter().map(|(level, params)| (Some(*level), Some(*params))))
        .collect();
    
    // Execute Phase 1 testing with initial values
    // This code remains mostly unchanged...
    
    // --- Phase 2: Initial Estimation with Knee Point ---
    log::info!("Phase 2: Estimating optimal grain per sample using Knee Point metric...");
    let mut initial_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);
    
    for (i, sample_results) in phase1_results.iter().enumerate() {
        // Use knee point analysis instead of diminishing returns
        let estimate = analyze_sample_with_knee_point(
            sample_results, 
            KNEE_THRESHOLD, 
            &mut |msg| log::info!("{}", msg)
        );
        initial_estimates.push(estimate);
    }
    
    log::info!("Phase 2 Initial estimates per sample: {:?}", initial_estimates);
    
    // --- Phase 3: Adaptive Refinement ---
    let mut phase3_results: Vec<HashMap<Option<GrainLevel>, u64>> = Vec::with_capacity(num_samples);
    for _ in 0..num_samples {
        phase3_results.push(HashMap::new());
    }
    
    // Skip refinement if we don't have enough initial estimates
    if initial_estimates.len() < 3 {
        log::info!("Too few samples for reliable refinement. Using initial estimates only.");
    } else {
        // Calculate refinement range based on initial estimates
        let (lower_bound, upper_bound) = calculate_refinement_range(&initial_estimates);
        log::info!(
            "Phase 3: Adaptive refinement range: {:?} to {:?}",
            lower_bound, upper_bound
        );
        
        // Generate refined parameter sets to test
        let refined_params = generate_refinement_params(
            lower_bound, upper_bound, &initial_test_levels
        );
        
        if refined_params.is_empty() {
            log::info!("No refinement parameters generated. Skipping Phase 3.");
        } else {
            log::info!(
                "Phase 3: Testing {} refined parameter sets: {:?}",
                refined_params.len(),
                refined_params
            );
            
            // Test each refined parameter set on all samples
            for (level_opt, params) in &refined_params {
                log::info!("  Testing refined level {:?}...", level_opt);
                
                for (i, &start_time) in sample_start_times.iter().enumerate() {
                    // Create encode parameters specific to this sample test
                    let mut sample_params = base_encode_params.clone();
                    sample_params.input_path = file_path.to_path_buf();
                    let output_filename = format!(
                        "sample_{}_refined_{:?}.mkv",
                        i + 1,
                        level_opt
                    );
                    sample_params.output_path = temp_dir_path.join(output_filename);
                    sample_params.hqdn3d_params = Some(params.clone());
                    sample_params.duration = sample_duration as f64;
                    
                    // Run encode
                    if let Err(e) = crate::external::ffmpeg::run_ffmpeg_encode(
                        spawner,
                        &sample_params,
                        true, /* disable_audio */
                        true, /* is_grain_analysis_sample */
                        *level_opt, /* grain_level_being_tested */
                    ) {
                        log::error!("Failed to encode refined sample {} with level {:?}: {}", 
                                   i + 1, level_opt, e);
                        continue; // Skip this test but don't fail the whole analysis
                    }
                    
                    // Get size
                    match metadata_provider.get_size(&sample_params.output_path) {
                        Ok(size) => {
                            phase3_results[i].insert(*level_opt, size);
                        },
                        Err(e) => {
                            log::error!("Failed to get size for refined sample {}: {}", i + 1, e);
                        }
                    }
                }
            }
        }
    }
    
    // --- Phase 4: Final Analysis with Combined Results ---
    log::info!("Phase 4: Final analysis with knee point on combined results...");
    let mut final_estimates: Vec<GrainLevel> = Vec::with_capacity(num_samples);
    
    for i in 0..num_samples {
        // Combine results from Phase 1 and Phase 3 for this sample
        let mut combined_results = phase1_results[i].clone();
        combined_results.extend(phase3_results[i].clone());
        
        // Use knee point analysis on combined results
        let final_estimate = analyze_sample_with_knee_point(
            &combined_results, 
            KNEE_THRESHOLD, 
            &mut |msg| log::info!("{}", msg)
        );
        final_estimates.push(final_estimate);
    }
    
    log::info!("Phase 4 Final estimates per sample: {:?}", final_estimates);
    
    // --- Determine Final Result ---
    if final_estimates.is_empty() {
        log::error!("No final estimates generated. Analysis failed.");
        return Ok(None);
    }
    
    // Calculate the median level
    let mut sorted_estimates = final_estimates.clone();
    sorted_estimates.sort();
    let median_idx = sorted_estimates.len() / 2;
    let final_level = sorted_estimates[median_idx];
    
    log::info!(
        "Final detected grain level: {:?}",
        final_level
    );
    
    Ok(Some(GrainAnalysisResult {
        detected_level: final_level,
    }))
    
    // Temporary directory cleanup handled by TempDir drop
}
```

## 4. Code Integration

Key changes to integrate the new features:

1. Replace the current `analyze_sample_results` function with the new `analyze_sample_with_knee_point`
2. Modify `analyze_grain` to include the refinement phase
3. Add the helper functions for interpolation and standard deviation
4. Update constants to include knee point threshold


## 6. Risks and Mitigations

1. **Parameter Generation Issues**:
   - **Risk**: Interpolation might generate invalid hqdn3d parameters
   - **Mitigation**: Validate all generated parameters before testing

## 7. Conclusion

This implementation plan provides a comprehensive approach to enhancing our grain analysis with the best features from the reference code. The combination of knee point detection and adaptive refinement will significantly improve our accuracy while maintaining our practical categorical approach.

After implementation, we should conduct a thorough review of the interpolated hqdn3d parameter sets to ensure they represent meaningful increments in denoising intensity that avoid excessive blurring while still providing effective grain reduction.
