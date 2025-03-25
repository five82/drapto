//! Configuration utility functions
//!
//! This module provides helper functions for working with
//! environment variables and configuration values.

use std::path::PathBuf;

/// Get a string value from an environment variable or use the default
pub fn get_env_string(key: &str, default: String) -> String {
    std::env::var(key).unwrap_or(default)
}

/// Get a path value from an environment variable or use the default
pub fn get_env_path(key: &str, default: PathBuf) -> PathBuf {
    std::env::var(key).map(PathBuf::from).unwrap_or(default)
}

/// Get a boolean value from an environment variable or use the default
pub fn get_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key) {
        Ok(val) => val.to_lowercase() == "true" || val == "1",
        Err(_) => default,
    }
}

/// Get a f32 value from an environment variable or use the default
pub fn get_env_f32(key: &str, default: f32) -> f32 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a u8 value from an environment variable or use the default
pub fn get_env_u8(key: &str, default: u8) -> u8 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a u32 value from an environment variable or use the default
pub fn get_env_u32(key: &str, default: u32) -> u32 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a usize value from an environment variable or use the default
pub fn get_env_usize(key: &str, default: usize) -> usize {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a i32 value from an environment variable or use the default
pub fn get_env_i32(key: &str, default: i32) -> i32 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a f64 value from an environment variable or use the default
pub fn get_env_f64(key: &str, default: f64) -> f64 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Get a i64 value from an environment variable or use the default
pub fn get_env_i64(key: &str, default: i64) -> i64 {
    match std::env::var(key) {
        Ok(val) => val.parse().unwrap_or(default),
        Err(_) => default,
    }
}

/// Parse a comma-separated list of u32 values from an environment variable
pub fn get_env_sample_rates(key: &str, default: Vec<u32>) -> Vec<u32> {
    match std::env::var(key) {
        Ok(val) => {
            // Parse comma-separated list of sample rates
            val.split(',')
               .filter_map(|s| s.trim().parse::<u32>().ok())
               .collect::<Vec<u32>>()
        },
        Err(_) => default,
    }
}