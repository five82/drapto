//! Simple terminal output functions for drapto-core.
//!
//! This module provides basic terminal formatting functions that maintain
//! the hierarchical output structure without complex dependencies.

use log::info;
use indicatif::{ProgressBar, ProgressStyle, ProgressDrawTarget};
use std::sync::Mutex;
use std::io::IsTerminal;
use owo_colors::OwoColorize;
use console::style;

struct ProgressState {
    progress_bar: Option<ProgressBar>,
}

impl ProgressState {
    const fn new() -> Self {
        Self {
            progress_bar: None,
        }
    }
}

static PROGRESS_STATE: Mutex<ProgressState> = Mutex::new(ProgressState::new());

/// Check if color should be used (respects NO_COLOR environment variable)
fn should_use_color() -> bool {
    std::env::var("NO_COLOR").is_err()
}

/// Print a section header (Level 1 - Main sections with cyan color)
pub fn print_section(title: &str) {
    info!("");
    if should_use_color() {
        info!("===== {} =====", title.to_uppercase().cyan().bold());
    } else {
        info!("===== {} =====", title.to_uppercase());
    }
    info!("");
}

/// Print a processing step (Level 2 - Subsections with 2 spaces indentation and bold)
pub fn print_processing(message: &str) {
    info!("");
    if should_use_color() {
        info!("  {} {}", "»", style(message).bold());
    } else {
        info!("  » {}", message);
    }
}

/// Print a status line (Level 4 - Primary info with 6 spaces indentation)
pub fn print_status(label: &str, value: &str, highlight: bool) {
    let label_width = 15;
    let padding = if label.len() < label_width {
        label_width - label.len()
    } else {
        1
    };
    
    if should_use_color() && highlight {
        info!(
            "      {}:{} {}",
            label,
            " ".repeat(padding),
            style(value).bold()
        );
    } else {
        info!(
            "      {}:{} {}",
            label,
            " ".repeat(padding),
            value
        );
    }
}

/// Print a success message (Level 2 - Success with 2 spaces indentation and green color)
pub fn print_success(message: &str) {
    info!("");
    if should_use_color() {
        info!("  ✓ {}", message.green());
    } else {
        info!("  ✓ {}", message);
    }
}

/// Print a sub-item (Level 3 - Operations with 4 spaces indentation)
pub fn print_sub_item(message: &str) {
    info!("    {}", message);
}

/// Print a sub-item with preceding blank line (Level 3 - Operations with spacing)
pub fn print_sub_item_with_spacing(message: &str) {
    info!("");
    info!("    {}", message);
}

/// Print completion with status
pub fn print_completion_with_status(success_message: &str, status_label: &str, status_value: &str) {
    print_success(success_message);
    print_status(status_label, status_value, false);
}

/// Print a subsection header (Level 2 - Subsections with 2 spaces indentation and bold)
pub fn print_subsection(title: &str) {
    if should_use_color() {
        info!("  {}", style(title).bold());
    } else {
        info!("  {}", title);
    }
}

/// Print a subsection header at Level 3 (4 spaces indentation - for use within sections)
pub fn print_subsection_level3(title: &str) {
    if should_use_color() {
        info!("    {}", style(title).bold());
    } else {
        info!("    {}", title);
    }
}

/// Print a subsection header at Level 3 with preceding blank line
pub fn print_subsection_level3_with_spacing(title: &str) {
    info!("");
    if should_use_color() {
        info!("    {}", style(title).bold());
    } else {
        info!("    {}", title);
    }
}

/// Initialize a progress bar
fn init_progress_bar(total_secs: f64) -> ProgressBar {
    let pb = ProgressBar::new((total_secs * 1000.0) as u64);
    
    let style = ProgressStyle::default_bar()
        .template("Encoding: {percent:>5.1}% [{bar:30}] ({elapsed_precise} / {eta_precise})")
        .unwrap()
        .progress_chars("##.");
    
    pb.set_style(style);
    pb.set_message("Encoding");
    
    if !std::io::stderr().is_terminal() {
        pb.set_draw_target(ProgressDrawTarget::hidden());
    }
    
    pb.enable_steady_tick(std::time::Duration::from_millis(100));
    pb
}

/// Print a progress bar following the design guide format
pub fn print_progress_bar(
    _percent: f32,
    elapsed_secs: f64,
    total_secs: f64,
    speed: Option<f32>,
    fps: Option<f32>,
    _eta: Option<std::time::Duration>,
) {
    if !std::io::stderr().is_terminal() {
        return;
    }
    
    let mut state = PROGRESS_STATE.lock().unwrap();
    
    // Initialize progress bar if not already done
    if state.progress_bar.is_none() {
        state.progress_bar = Some(init_progress_bar(total_secs));
    }
    
    if let Some(pb) = state.progress_bar.as_ref() {
        pb.set_position((elapsed_secs * 1000.0) as u64);
        
        // Update message with speed/fps if available
        if let (Some(speed_val), Some(fps_val)) = (speed, fps) {
            let msg = format!("Encoding - Speed: {speed_val:.2}x, Avg FPS: {fps_val:.2}");
            pb.set_message(msg);
        }
        
        pb.tick();
    }
}

/// Clear progress bar 
pub fn clear_progress_bar() {
    let mut state = PROGRESS_STATE.lock().unwrap();
    if let Some(pb) = state.progress_bar.take() {
        pb.finish_and_clear();
    }
}