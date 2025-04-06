// drapto-cli/src/logging.rs
//
// Handles logging setup and the combined console/file log callback.

use std::fs::File;
use std::io::{BufWriter, Write};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

// --- Helper Functions (Timestamp) ---
pub fn get_timestamp() -> String {
    chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
}

// --- Log Callback Creation ---
// Creates the logging closure that writes to both console (with color) and a file.
pub fn create_log_callback(
    log_file: File,
) -> Result<Box<dyn FnMut(&str) + Send + 'static>, Box<dyn std::error::Error>> {
    // Wrap shared state in Arc<Mutex> for thread safety
    let logger = Arc::new(Mutex::new(BufWriter::new(log_file)));
    let last_was_progress = Arc::new(Mutex::new(false));
    let last_progress_file_log_time = Arc::new(Mutex::new(None::<Instant>));
    let throttle_duration = Duration::from_secs(30); // Throttle interval

    // The closure captures the Arc<Mutex<...>> variables, making it Send + Clone + 'static
    let log_callback = move |msg: &str| {
        // --- File Logging (Throttled for progress) ---
        let is_progress = msg.contains('\r');
        let should_log_to_file = if is_progress {
            let now = Instant::now();
            let mut last_time_opt_guard = last_progress_file_log_time.lock().unwrap();
            match *last_time_opt_guard {
                Some(last_time) if now.duration_since(last_time) < throttle_duration => false,
                _ => {
                    *last_time_opt_guard = Some(now);
                    true
                }
            }
        } else {
            true
        };

        if should_log_to_file {
            let mut logger_guard = logger.lock().unwrap();
            if is_progress {
                writeln!(logger_guard, "{}", msg).ok();
            } else if msg.ends_with('\n') || msg.ends_with('\r') {
                write!(logger_guard, "{}", msg).ok();
            } else {
                writeln!(logger_guard, "{}", msg).ok();
            }
            logger_guard.flush().ok();
        }

        // --- Console Logging (Colored) ---
        // Create a new stdout handle each time to avoid capturing non-Send/Clone type
        let mut stdout = StandardStream::stdout(ColorChoice::Auto);
        let msg_trimmed = msg.trim_end();
        let mut last_was_progress_guard = last_was_progress.lock().unwrap(); // Lock for console logic

        if is_progress {
            stdout.set_color(ColorSpec::new().set_fg(Some(Color::Blue)).set_bold(false).set_dimmed(false).set_intense(false)).ok();
            write!(&mut stdout, "{}", msg).ok();
            stdout.reset().ok();
            stdout.flush().ok();
            *last_was_progress_guard = true;
        } else {
            if *last_was_progress_guard {
                writeln!(&mut stdout).ok();
            }

            let mut handled = false;
            let bold_label_prefixes = [
                "Input path:", "Output directory:", "Log directory:", "Main log file:",
                "Total encode execution time:", "Drapto Encode Run Finished:", "Drapto Encode Run Started:",
                "Running on host:",
            ];
            let summary_value_prefixes = [
                "  Encode time: ", "  Input size:  ", "  Output size: ", "  Reduced by:  ",
            ];

            // Style: Bold Labels, Normal Values
            for prefix in bold_label_prefixes {
                if msg_trimmed.starts_with(prefix) {
                    if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                        stdout.set_color(ColorSpec::new().set_bold(true)).ok();
                        write!(&mut stdout, "{}", prefix).ok();
                        stdout.reset().ok();
                        writeln!(&mut stdout, "{}", value).ok();
                        handled = true;
                        break;
                    }
                }
            }

            // Style: Normal Labels, Bold Values
            if !handled {
                for prefix in summary_value_prefixes {
                    if msg_trimmed.starts_with(prefix) {
                        if let Some(value) = msg_trimmed.strip_prefix(prefix) {
                            write!(&mut stdout, "{}", prefix).ok();
                            stdout.set_color(ColorSpec::new().set_bold(true)).ok();
                            writeln!(&mut stdout, "{}", value).ok();
                            stdout.reset().ok();
                            handled = true;
                            break;
                        }
                    }
                }
            }

            // Style: Success Count
            if !handled && msg_trimmed.starts_with("Successfully encoded ") {
                 if let Some(rest) = msg_trimmed.strip_prefix("Successfully encoded ") {
                    if let Some((count_str, _suffix)) = rest.split_once(" file(s).") {
                        write!(&mut stdout, "Successfully encoded ").ok();
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green)).set_bold(true)).ok();
                        write!(&mut stdout, "{}", count_str).ok();
                        stdout.reset().ok();
                        stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok();
                        writeln!(&mut stdout, " file(s).").ok();
                        stdout.reset().ok();
                        handled = true;
                    }
                }
            }

            // Style: Status Prefixes
            if !handled {
                let status_prefixes = [
                    ("[OK]", ColorSpec::new().set_fg(Some(Color::Green)).clone()),
                    ("[INFO]", ColorSpec::new().set_fg(Some(Color::Cyan)).clone()),
                    ("[WARN]", ColorSpec::new().set_fg(Some(Color::Yellow)).clone()),
                    ("[ERROR]", ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone()),
                    ("[FAIL]", ColorSpec::new().set_fg(Some(Color::Red)).set_bold(true).clone()),
                    ("[DEBUG]", ColorSpec::new().set_fg(Some(Color::Magenta)).clone()),
                    ("[TRACE]", ColorSpec::new().set_fg(Some(Color::Blue)).clone()),
                ];
                for (prefix, spec) in status_prefixes {
                    let prefix_with_space = format!("{} ", prefix);
                    if msg_trimmed.starts_with(&prefix_with_space) {
                        if let Some(rest) = msg_trimmed.strip_prefix(&prefix_with_space) {
                            stdout.set_color(&spec).ok();
                            write!(&mut stdout, "{}", prefix).ok();
                            stdout.reset().ok();
                            writeln!(&mut stdout, " {}", rest).ok();
                            handled = true;
                            break;
                        }
                    } else if msg_trimmed == prefix {
                        stdout.set_color(&spec).ok();
                        writeln!(&mut stdout, "{}", prefix).ok();
                        stdout.reset().ok();
                        handled = true;
                        break;
                    }
                }
            }

            // Style: Specific Lines
            if !handled {
                if msg_trimmed == "External dependency check passed." {
                    stdout.set_color(ColorSpec::new().set_fg(Some(Color::Green))).ok();
                    writeln!(&mut stdout, "{}", msg_trimmed).ok();
                    stdout.reset().ok();
                    handled = true;
                } else if let Some(filename) = msg_trimmed.strip_prefix("Processing: ") {
                    stdout.set_color(ColorSpec::new().set_bold(true)).ok();
                    write!(&mut stdout, "Processing: ").ok();
                    stdout.reset().ok();
                    writeln!(&mut stdout, "{}", filename).ok();
                    handled = true;
                }
            }

            // Fallback Styling
            if !handled {
                let mut color_spec = ColorSpec::new();
                match msg_trimmed {
                    m if m.starts_with("===") || m.starts_with("---") => {
                        color_spec.set_fg(Some(Color::Cyan)).set_bold(true);
                    }
                    m if m.starts_with("Encoding Summary:") => {
                        color_spec.set_bold(true);
                    }
                    m if m.starts_with("No processable .mkv files") || m.starts_with("No files were successfully encoded.") => {
                        color_spec.set_fg(Some(Color::Yellow));
                    }
                    _ => {
                        if !msg_trimmed.starts_with(' ') && !msg_trimmed.starts_with("===") && !msg_trimmed.starts_with("---") && !msg_trimmed.starts_with("Encoding Summary:") {
                            color_spec.set_bold(true);
                        }
                    }
                }
                stdout.set_color(&color_spec).ok();
                writeln!(&mut stdout, "{}", msg_trimmed).ok();
                stdout.reset().ok();
            }

            *last_was_progress_guard = false;
            stdout.flush().ok();
        }
    };

    // Box the closure and return it
    Ok(Box::new(log_callback))
}