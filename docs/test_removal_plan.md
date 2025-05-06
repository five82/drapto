# Plan to Remove Tests and Mocking Infrastructure

This plan outlines the steps to remove all existing test code, mocking code, related feature flags, and development-only dependencies from the `drapto-core` and `drapto-cli` crates. This is the first step towards implementing a new testing strategy.

**Steps:**

1.  **Delete Test Directories:**
    *   Recursively delete the directory `drapto-cli/tests/`.
    *   Recursively delete the directory `drapto-core/tests/`.
2.  **Delete Mocking Code File:**
    *   Delete the file `drapto-core/src/external/mocks.rs`.
3.  **Modify `drapto-core/src/external/mod.rs`:**
    *   Remove the line `pub mod mocks;` (originally line 19).
    *   Remove the `#[cfg(not(feature = "test-mocks"))]` attributes (originally on lines 6, 9, 12, and 32).
4.  **Modify `drapto-core/Cargo.toml`:**
    *   Remove the entire `[features]` section (originally lines 22-24).
    *   Remove the entire `[dev-dependencies]` section (originally lines 26-28).
5.  **Modify `drapto-cli/Cargo.toml`:**
    *   Remove the entire `[features]` section (originally lines 19-23).
    *   Remove the entire `[dev-dependencies]` section (originally lines 24-31).