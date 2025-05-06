// drapto-cli/src/logging.rs
//
// Contains helper functions related to logging, like timestamp generation.
// The actual logging implementation uses the `log` crate and `env_logger`.

// --- Helper Functions (Timestamp) ---

/// Returns the current local timestamp formatted as "YYYYMMDD_HHMMSS".
pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// --- Removed `create_log_callback` function and related code ---
// The custom callback system has been replaced by standard `log` macros
// and `env_logger` initialization in `main.rs`.