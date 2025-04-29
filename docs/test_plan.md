# Drapto Test Coverage Improvement Plan

This document outlines the plan to improve test coverage for the Drapto project.

## 1. Document the Testing Strategy [✅ COMPLETED]

*   **Action:** Create a new file `docs/testing_strategy.md`. [✅ COMPLETED]
*   **Content:** This document should outline: [✅ COMPLETED]
    *   The different types of tests used in the project (Unit, Integration, CLI).
    *   The scope and purpose of each test type.
    *   The strategy for handling external dependencies like `ffmpeg` and `ntfy` in tests (e.g., mocking, using real binaries with specific test data, etc.).
    *   How code coverage is measured (or will be measured).

## 2. Enhance `drapto-cli` Tests

*   **`cli_integration.rs`:**
    *   **Implement Mocking:** Introduce mocking for `ffmpeg`/`ffprobe` calls. [✅ COMPLETED - via core mocks and feature flags]
    *   **Expand `encode` Command Tests:** Using the mocks, add tests covering scenarios identified in the existing TODOs:
        *   Various valid and invalid `--preset` options. [✅ Partially Completed - Valid & Invalid tested]
        *   Using a `--config` file to provide settings. [❌ Invalid - CLI doesn't support --config flag]
        *   Handling non-existent `--input` paths gracefully. [✅ COMPLETED]
        *   Handling invalid arguments (e.g., incorrect quality values, malformed paths). [✅ Partially Completed - Preset & Quality tested]
        *   Verifying that the correct `ffmpeg` command arguments are generated based on the CLI inputs. [✅ Partially Completed - Verified in core success test]
*   **`ntfy_integration.rs`:**
    *   **Test Success Cases:** Add tests to verify that `ntfy` notifications are sent correctly for *successful* operations (start, completion, errors during processing). [TODO - Requires core tests first]
    *   **Mock `ntfy`:** This will likely require mocking the HTTP client used to send notifications or setting up a mock HTTP server (like `mockito` or `wiremock-rs`) to capture and verify outgoing requests to the `ntfy` URL. [✅ COMPLETED - via core mocks]

## 3. Enhance `drapto-core` Tests

*   **`core_tests.rs` (and potentially new test files):**
    *   **Test Processing Logic:** Add integration tests for the core media processing functions within `drapto-core/src/processing/`. This will heavily rely on the mocking implemented for `ffmpeg`/`ffprobe`. Tests should cover: [⏳ In Progress]
        *   Video processing logic (`video.rs`). [✅ Partially Completed - Success/Fail/SpawnError/NtfyFail tests added]
        *   Audio processing logic (`audio.rs`). [✅ COMPLETED - Unit test for calc, Mock test for log]
        *   Detection logic (`detection.rs`). [✅ COMPLETED - SDR and HDR paths tested w/ mock]
    *   **Test External Wrappers:** Add specific tests for the `ffmpeg` wrapper (`external/ffmpeg.rs`) to ensure it generates correct command-line arguments based on different parameters. Mock the actual command execution (`std::process::Command`). [✅ Partially Completed - Added tests verifying args in `ffmpeg_wrapper_tests.rs`]
    *   **Test Configuration:** Add tests for loading, parsing, and merging configuration from files (`config.rs` in `drapto-core`). [TODO]
    *   **Test Error Handling:** Review `error.rs` and ensure different `CoreError` variants are triggered and handled correctly in relevant test cases (beyond the file discovery tests). [✅ Partially Completed - Tested IoError, CommandFailed, CommandStart]
    *   **Test Private Utilities (Unit Tests):** For utility functions currently marked as private but critical (like `calculate_audio_bitrate`), add unit tests within their respective modules (`src/lib.rs`, `src/utils.rs`, etc.) using inline `#[cfg(test)] mod tests { ... }` blocks. [✅ Partially Completed - Existing unit tests seem okay]
*   **Test Organization:** Integration tests in `tests/` directory reorganized into logical files (`utils_tests.rs`, `discovery_tests.rs`, `detect_crop_tests.rs`, `process_videos_success_tests.rs`, `process_videos_ffmpeg_fail_tests.rs`, `process_videos_ntfy_fail_tests.rs`, `ffmpeg_wrapper_tests.rs`). [✅ COMPLETED]
*   **Code Organization:** Refactored `drapto-core/src/external` and `drapto-core/src/processing/detection` into logical submodules. [✅ COMPLETED]

## 4. Implement Code Coverage Reporting [TODO]

*   **Action:** Integrate a code coverage tool like `cargo-tarpaulin` or `grcov`. [TODO]
*   **Integration:** Add steps to `build.sh` or a CI workflow to run tests with coverage enabled and potentially report the results or fail the build if coverage drops below a certain threshold. [TODO]

## Plan Visualization (Gantt Chart)

```mermaid
gantt
    dateFormat  YYYY-MM-DD
    title Drapto Test Coverage Improvement Plan

    section Documentation
    Create Testing Strategy Doc :doc_strat, 2025-04-29, 1d

    section drapto-cli Tests
    Implement ffmpeg/ntfy Mocking :cli_mock, after doc_strat, 3d
    Add Encode Arg/Logic Tests :cli_args, after cli_mock, 3d
    Add Ntfy Success/Error Tests :cli_ntfy, after cli_mock, 2d
    Add CLI Config Loading Tests :cli_config, after cli_args, 1d

    section drapto-core Tests
    Add Processing Logic Tests (Video, Audio, Detect) :core_proc, after cli_mock, 4d
    Add FFmpeg Wrapper Tests :core_ffmpeg, after core_proc, 2d
    Add Core Config Tests :core_config, after cli_config, 1d
    Review/Add Error Handling Tests :core_error, after core_proc, 1d
    Add Unit Tests for Private Funcs (Optional) :core_unit, after core_proc, 1d

    section Tooling
    Integrate Code Coverage Tool :cov_tool, after core_ffmpeg, 1d