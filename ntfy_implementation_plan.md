# Plan for Implementing ntfy Notifications in Drapto

This document outlines the plan to integrate ntfy notifications into the `drapto` application for signaling video encoding events.

## 1. Configuration Strategy

*   **Command-Line Argument:** Add an optional argument `--ntfy <topic_url>` to the `encode` subcommand.
*   **Environment Variable:** Allow configuration via `DRAPTO_NTFY_TOPIC`.
*   **Precedence:** The command-line argument will override the environment variable if both are set.

## 2. Implementation Steps

1.  **Add Dependency:**
    *   Add the `reqwest` crate (with `json` and potentially `blocking` or `rustls-tls` features) to `drapto-core/Cargo.toml`.

2.  **Create Notification Module:**
    *   Create `drapto-core/src/notifications.rs`.
    *   Define `send_ntfy(topic: &str, message: &str, title: Option<&str>, priority: Option<u8>, tags: Option<&str>) -> Result<(), CoreError>` to send HTTP POST requests to the ntfy topic URL.
    *   Declare the module in `drapto-core/src/lib.rs`.

3.  **Update Core Configuration:**
    *   Add `ntfy_topic: Option<String>` to `CoreConfig` in `drapto-core/src/config.rs`.

4.  **Update CLI Arguments:**
    *   In `drapto-cli/src/cli.rs`, add `--ntfy` to `EncodeArgs` using `clap`, configuring it to read from the `DRAPTO_NTFY_TOPIC` environment variable (`#[arg(long, env = "DRAPTO_NTFY_TOPIC")]`).

5.  **Update CLI Command Handler:**
    *   In `drapto-cli/src/commands/encode.rs`, populate `CoreConfig.ntfy_topic` from the parsed `args.ntfy`.

6.  **Integrate Notification Calls:**
    *   In `drapto-core/src/processing/video.rs` (`process_videos` function):
        *   Import `send_ntfy`.
        *   Check if `config.ntfy_topic` is `Some` at the start of the file processing loop.
        *   If `Some(topic)`:
            *   **Start:** Call `send_ntfy` after the "Processing:" log message (title: filename, priority: 3).
            *   **Success:** Call `send_ntfy` within the `if status.success()` block (title: filename, priority: 4).
            *   **Error:** Call `send_ntfy` within the `else` block for HandBrakeCLI failure (title: filename, priority: 5).
        *   Log errors from `send_ntfy` but continue processing.

## 3. Flow Diagram

```mermaid
graph TD
    A[User runs drapto-cli encode --ntfy <topic_url>] --> B(drapto-cli: Parse Args);
    B -- Reads DRAPTO_NTFY_TOPIC if needed --> C[EncodeArgs has ntfy_topic];
    C --> D(drapto-cli: Create CoreConfig);
    D -- CoreConfig.ntfy_topic set --> E(drapto-core: process_videos);
    E --> F{Loop per file};
    F -- Start --> G[Check config.ntfy_topic];
    G -- If Some(topic) --> H(notifications::send_ntfy - Start);
    F -- Process --> I(Run HandBrakeCLI);
    I -- Success --> J[Check config.ntfy_topic];
    J -- If Some(topic) --> K(notifications::send_ntfy - Success);
    I -- Error --> L[Check config.ntfy_topic];
    L -- If Some(topic) --> M(notifications::send_ntfy - Error);
    H --> N[HTTP POST];
    K --> N;
    M --> N;

    subgraph "drapto-cli"
        B; C; D;
    end

    subgraph "drapto-core"
        E; F; I;
        subgraph "notifications"
            style notifications fill:#eee,stroke:#333
            G; H; J; K; L; M; N;
        end
    end

    style N fill:#f9f,stroke:#333,stroke-width:2px