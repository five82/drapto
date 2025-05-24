// ============================================================================
// drapto-cli/src/commands/mod.rs
// ============================================================================
//
// COMMAND MODULES: Submodule Declarations for CLI Commands
//
// This file declares the submodules for different CLI commands in the Drapto
// application. Each submodule contains the implementation of a specific command.
//
// CURRENT COMMANDS:
// - encode: Converts video files to AV1 format
//
// FUTURE COMMANDS:
// - analyze: Analyze video files without encoding
// - config: Manage application configuration
//
// AI-ASSISTANT-INFO: Command module declarations, entry point for command implementations

/// Module containing the implementation of the `encode` command.
/// This command converts video files to AV1 format with configurable settings.
pub mod encode;

// Future command modules will be added here as the application evolves:
// pub mod analyze;
// pub mod config;
