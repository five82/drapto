# Plan: Integrate `ntfy` v0.7.0 (Blocking Mode)

This document outlines the plan to replace the custom `reqwest`-based ntfy notification implementation in `drapto-core` with the official `ntfy` crate (version 0.7.0), utilizing its blocking feature to maintain compatibility with the existing synchronous codebase.

## Goals

*   Replace the manual HTTP request logic with the `ntfy` crate.
*   Use the `blocking` feature of the `ntfy` crate to avoid introducing an async runtime (`tokio`) at this stage.
*   Ensure the existing `Notifier` trait interface remains unchanged.
*   Minimize changes outside the `drapto-core/src/notifications.rs` module.
*   Verify existing tests and update them as necessary.

## Steps

1.  **Modify Dependencies (`drapto-core/Cargo.toml`):**
    *   Add the `ntfy` crate dependency, explicitly enabling the `blocking` feature and disabling default features:
        ```toml
        ntfy = { version = "0.7.0", default-features = false, features = ["blocking"] }
        ```
    *   Remove the direct `reqwest` dependency (`reqwest = { version = "0.12", ... }`). The `url` crate might also be needed if not already present transitively.
2.  **Update `drapto-core/src/notifications.rs`:**
    *   **Imports:** Add necessary imports:
        ```rust
        use ntfy::blocking::Dispatcher; // Use blocking dispatcher
        use ntfy::payload::{Payload, Priority};
        use ntfy::error::Error as NtfyError;
        // Potentially: use ntfy::Auth;
        use url::Url; // For parsing the topic URL
        use crate::error::{CoreError, CoreResult}; // Keep existing error types
        ```
    *   **`NtfyNotifier` Struct:** Simplify the struct. Since the dispatcher will likely be created dynamically within the `send` method, the struct might become empty:
        ```rust
        pub struct NtfyNotifier;
        ```
    *   **`NtfyNotifier::new()`:** Update the constructor to reflect the simplified struct:
        ```rust
        impl NtfyNotifier {
            pub fn new() -> CoreResult<Self> {
                Ok(Self) // Simple instantiation
            }
        }
        ```
    *   **`NtfyNotifier::send()`:** Rewrite the core logic:
        *   Parse the input `topic_url: &str` into a `url::Url`. Handle parsing errors, returning `CoreError::NotificationError`.
        *   Extract the base URL (scheme + host + port) and the topic (path) from the parsed URL.
        *   Create an `ntfy::blocking::Dispatcher` dynamically using `Dispatcher::builder(base_url).build()`. Map the potential `NtfyError` to `CoreError::NotificationError`.
        *   Create an `ntfy::Payload` using the extracted topic.
        *   Set the payload's message (`payload.message(message)`).
        *   Set the title if present (`if let Some(t) = title { payload.title(t); }`).
        *   Map the `Option<u8>` priority to `Option<ntfy::Priority>` and set it (`if let Some(p) = priority { /* map p to ntfy::Priority */ payload.priority(mapped_p); }`).
        *   Handle tags: Combine input `tags` with the default "drapto" tag and set them (`payload.tags(final_tags_vec)`).
        *   Call `dispatcher.send(&payload)`.
        *   Map any resulting `NtfyError` to `CoreError::NotificationError`.
    *   **Remove Deprecated Function:** Delete the old `send_ntfy` function.
    *   **Update Tests:** Modify the unit tests within `notifications.rs` to align with the new implementation and error mapping.
    *   **Update Mocking (`mocks` module):** Verify the `MockNotifier` still functions correctly with the unchanged `Notifier` trait. Ensure tag handling logic is consistent.
3.  **Verify `drapto-cli/src/main.rs`:** Confirm no code changes are needed, as it relies on the `Notifier` trait and the `NtfyNotifier::new()` signature remains compatible.
4.  **Review Integration Tests:** Examine `drapto-cli/tests/ntfy_integration.rs` and `drapto-core/tests/process_videos_ntfy_fail_tests.rs`. Update assertions if necessary to match potential changes in error messages originating from `ureq` (via `ntfy`) or the `CoreError` mapping.

## Diagram

```mermaid
graph TD
    A[Start: Integrate ntfy v0.7.0 Blocking] --> B(Modify drapto-core/Cargo.toml);
    B --> C[Add ntfy = {..., features = ["blocking"]}];
    B --> D[Remove reqwest dependency];
    C & D --> E(Update drapto-core/src/notifications.rs);
    subgraph "notifications.rs Changes"
        direction LR
        E --> F[Update Imports];
        E --> G[Simplify NtfyNotifier struct];
        E --> H[Simplify NtfyNotifier::new()];
        E --> I[Rewrite NtfyNotifier::send() logic];
        E --> J[Remove deprecated send_ntfy function];
        E --> K[Update Unit Tests];
        E --> L[Verify MockNotifier];
    end
    subgraph "NtfyNotifier::send() Rewrite"
        direction TB
        I --> I1[Parse topic_url (url::Url)];
        I1 --> I2[Extract base_url & topic];
        I2 --> I3[Build ntfy::blocking::Dispatcher];
        I3 --> I4[Build ntfy::Payload];
        I4 --> I5[Set message, title, priority, tags];
        I5 --> I6[dispatcher.send(&payload)];
        I6 --> I7[Map ntfy::Error to CoreError];
    end
    E --> M(Verify drapto-cli/src/main.rs - No changes expected);
    K & L & M --> N(Review/Update Integration Tests);
    N --> O(End: ntfy v0.7.0 integrated);