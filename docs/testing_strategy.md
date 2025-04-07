# Pragmatic Testing Strategy for Drapto

## Introduction

This document outlines a practical and focused testing strategy for the Drapto project. As a hobby project maintained by a single developer, the goal is to maximize the value of testing efforts by concentrating on critical areas, ensuring core functionality works reliably without introducing excessive complexity or maintenance overhead.

## Guiding Principles

*   **Simplicity:** Prefer straightforward tests that are easy to write and maintain. Avoid complex mocking frameworks or setup unless strictly necessary.
*   **Focus on Critical Paths:** Prioritize testing the main workflows, complex logic (like film grain analysis), and interactions with external tools (HandBrakeCLI).
*   **Prioritize Integration:** Ensure the CLI works as expected from end-to-end (using mocked external dependencies) as this provides high confidence in the overall tool.
*   **Maintainability:** Tests should be robust to minor refactoring and easy to update as the codebase evolves.

## Recommended Actions (Prioritized)

1.  **CLI Integration Tests:**
    *   **Goal:** Verify that the `drapto-cli` correctly parses arguments, interacts with `drapto-core`, handles configuration, and manages basic error scenarios.
    *   **Approach:** Create integration tests (e.g., in `drapto-cli/tests/`) that invoke the CLI binary. Mock the `HandBrakeCLI` interaction (e.g., by replacing the command execution logic with a test double or using a feature flag) to simulate success, failure, and different output scenarios without actually running encodes. Test key commands like `encode`.
    *   **Example:** A test that runs `drapto encode --input fake.mkv --output fake.mp4 --preset Fast` and verifies that the correct (mocked) HandBrake command would have been generated.

2.  **Targeted Unit Tests for Core Logic:**
    *   **Goal:** Test specific functions within `drapto-core` and `drapto-cli` that contain complex logic or are critical to the tool's operation.
    *   **Approach:** Use inline `#[cfg(test)]` modules. Focus on:
        *   `drapto-core/src/processing/film_grain/`: Test the analysis, sampling, and type logic with various inputs.
        *   `drapto-core/src/config.rs`: Test configuration loading and validation.
        *   `drapto-cli/src/cli.rs`: Test argument parsing logic, especially edge cases or complex interactions.
        *   `drapto-core/src/error.rs`: Ensure custom error types behave as expected.
        *   `drapto-core/src/notifications.rs`: Test notification formatting and sending logic (potentially mocking the actual sending mechanism).
    *   **Example:** A unit test for the film grain analysis function that provides known sample file sizes and verifies the correct grain level is chosen.

3.  **Basic Continuous Integration (CI):**
    *   **Goal:** Automatically run tests on every commit or pull request to catch regressions early.
    *   **Approach:** Set up a simple CI workflow using GitHub Actions (or a similar service). The workflow should check out the code, build the project, and run `cargo test --all`.
    *   **Benefit:** Provides immediate feedback and ensures tests are consistently executed.

## Future Considerations (Optional)

*   **Code Coverage:** Tools like `cargo-tarpaulin` can be used *later* to identify untested code paths if specific areas prove problematic or if a more comprehensive approach is desired.
*   **Advanced Mocking:** If interactions become significantly more complex, libraries like `mockall` could be introduced, but start simple.
*   **End-to-End Tests with Real HandBrake:** For critical validation, occasional manual or scripted tests involving actual short HandBrake encodes could be performed, but these should not be part of the automated CI due to their runtime cost.

## Conclusion

This focused testing strategy aims to provide a good balance between test coverage and development effort for Drapto. By prioritizing integration tests for the CLI and unit tests for critical logic, combined with basic CI, we can significantly improve the reliability and maintainability of the tool without overengineering the testing setup.