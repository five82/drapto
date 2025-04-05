# Plan: Implement New Film Grain Sampling Logic

This document outlines the plan to modify the film grain sampling logic in the `drapto-core` library.

**Goal:** Change the sampling strategy from a fixed number of samples at fixed positions to a dynamic number of samples based on video duration, with randomized start times within a specific range.

**Requirements:**

1.  **Number of Samples:**
    *   1 sample per 10 minutes of video length.
    *   Minimum of 3 samples.
    *   Maximum of 9 samples.
    *   Always an odd number (round up if even, respecting the max).
2.  **Sample Positions:**
    *   Randomized start times.
    *   Do not take samples from the first 15% or the last 15% of the video.
3.  **Sample Duration:** Remain at 10 seconds.

**Implementation Steps:**

1.  **Add Dependency:**
    *   Modify `drapto-core/Cargo.toml` to include the `rand` crate:
        ```toml
        [dependencies]
        # ... other dependencies
        rand = "0.8"
        ```

2.  **Modify `determine_optimal_grain` in `drapto-core/src/processing/film_grain/mod.rs`:**
    *   **Import:** Add `use rand::{thread_rng, Rng};`.
    *   **Calculate Sample Count:**
        *   Remove the line reading `sample_count` from config/default.
        *   After getting `total_duration_secs`, calculate `num_samples`:
            *   Base: `let base_samples = (total_duration_secs / 600.0).ceil() as usize;`
            *   Min/Max: `let mut num_samples = base_samples.max(3).min(9);`
            *   Odd: `if num_samples % 2 == 0 { num_samples = (num_samples + 1).min(9); }`
            *   Log the calculated number.
    *   **Calculate Sample Positions:**
        *   Replace the existing interval calculation loop.
        *   Define boundaries: `start_boundary = total_duration_secs * 0.15`, `end_boundary = total_duration_secs * 0.85`.
        *   Check feasibility: Ensure enough usable duration (`end_boundary - start_boundary`) for `num_samples` * `sample_duration` within the 15%-85% window. Log warning/fallback if not feasible.
        *   Generate random start times using `rng.gen_range(start_boundary..=(end_boundary - sample_duration as f64))`.
        *   Store times in `sample_start_times`.
        *   Log the generated times.
    *   **Update Variable Usage:** Ensure subsequent code uses the calculated `num_samples` and `sample_start_times`.

**Workflow Diagram:**

```mermaid
graph TD
    A[Start determine_optimal_grain] --> B{Get Video Duration};
    B --> C{Calculate Base Sample Count (1 per 10 min)};
    C --> D{Apply Min (3) & Max (9) Constraints};
    D --> E{Ensure Odd Number (Round Up)};
    E --> F[Final Sample Count `num_samples`];
    F --> G{Calculate Valid Sampling Window (15% - 85%)};
    G --> H{Check if Window is Sufficient for `num_samples`};
    H -- Yes --> I{Generate `num_samples` Random Start Times within Window};
    H -- No --> J[Log Warning/Error/Fallback];
    I --> K[Proceed with Phase 1 Testing using Random Samples];
    K --> L[Continue with Phases 2-4 (Analysis based on results)];
    L --> M[Return Final Grain Value];
    J --> M;

    style J fill:#f9f,stroke:#333,stroke-width:2px