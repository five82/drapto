# Drapto Testing Strategy

This document outlines the testing strategy for the Drapto project.

## Test Types

Drapto utilizes several types of tests to ensure code quality, correctness, and robustness:

1.  **Unit Tests:**
    *   **Location:** Defined within `src/` modules using `#[cfg(test)] mod tests { ... }`.
    *   **Scope:** Test individual functions, methods, or small units of code in isolation.
    *   **Purpose:** Verify the correctness of specific algorithms, logic branches, and edge cases within a single module. Dependencies are typically mocked or stubbed.

2.  **Integration Tests:**
    *   **Location:** Reside in the `tests/` directory at the crate root (`drapto-core/tests/`, `drapto-cli/tests/`).
    *   **Scope:** Test the interaction between different modules within a crate or the public API of a crate.
    *   **Purpose:** Ensure that different parts of the crate work together as expected. May involve interacting with the filesystem (using `tempfile`) but external processes are generally mocked.

3.  **CLI Tests:**
    *   **Location:** `drapto-cli/tests/cli_integration.rs` (and potentially others).
    *   **Scope:** Test the `drapto` command-line executable as a black box.
    *   **Purpose:** Verify command-line argument parsing, overall application flow, exit codes, and interactions with external dependencies (which will be mocked). Uses `assert_cmd`.

## Handling External Dependencies

Testing code that interacts with external processes (`ffmpeg`, `ffprobe`) or network services (`ntfy`) requires a specific approach:

*   **Mocking:** For most unit and integration tests, external dependencies will be mocked.
    *   **Processes (`ffmpeg`, `ffprobe`):** This might involve mocking `std::process::Command` execution or creating wrapper functions/traits around process calls that can be replaced with test doubles.
    *   **Network (`ntfy`):** Mock HTTP clients or use mock HTTP servers (e.g., `mockito`, `wiremock-rs`) to simulate `ntfy` server responses and verify requests.
*   **Real Binaries (Limited Use):** Some end-to-end CLI tests *might* eventually use real binaries with carefully crafted small/dummy media files, but the primary strategy relies on mocking for deterministic and faster tests.
