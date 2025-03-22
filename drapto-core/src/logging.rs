use log::{debug, info, warn, LevelFilter};
use std::io::Write;
use std::process::Command;
use std::fs;
use std::path::Path;

/// Initialize the logger for drapto
///
/// Sets up an env_logger with appropriate formatting and log level
pub fn init(verbose: bool) {
    let level = if verbose {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };
    
    init_with_level(level, verbose);
}

/// Initialize the logger with a specific log level
///
/// Sets up an env_logger with appropriate formatting and the specified log level
pub fn init_with_level(level: LevelFilter, _verbose: bool) {
    env_logger::Builder::new()
        .format(|buf, record| {
            let timestamp = buf.timestamp();
            let level_str = match record.level() {
                log::Level::Error => "ERROR",
                log::Level::Warn => "WARN ",
                log::Level::Info => "INFO ",
                log::Level::Debug => "DEBUG",
                log::Level::Trace => "TRACE",
            };
            
            writeln!(
                buf,
                "{} {} {}",
                timestamp,
                level_str,
                record.args()
            )
        })
        .filter(None, level)
        .init();
    
    debug!("Logger initialized with level: {}", level);
}

/// Initialize a file logger for a specific encoding session
///
/// Creates a log file and returns a file handle to it
pub fn init_file_logger<P: AsRef<Path>>(file_name: P) -> std::io::Result<fs::File> {
    // Make sure the parent directory exists
    if let Some(parent) = file_name.as_ref().parent() {
        fs::create_dir_all(parent)?;
    }
    
    let file = fs::File::create(file_name)?;
    Ok(file)
}

/// Log an encoding progress update
pub fn log_progress(stage: &str, progress: f32) {
    if !(0.0..=1.0).contains(&progress) {
        warn!("Invalid progress value: {}", progress);
        return;
    }
    
    let percentage = (progress * 100.0) as u8;
    let bar_length = 20;
    let filled_length = (bar_length as f32 * progress) as usize;
    
    let bar: String = std::iter::repeat("█").take(filled_length)
        .chain(std::iter::repeat("░").take(bar_length - filled_length))
        .collect();
    
    info!("{}: {}% [{}]", stage, percentage, bar);
}

/// Log a command being executed
pub fn log_command(cmd: &Command) {
    let program = cmd.get_program().to_string_lossy();
    let args: Vec<_> = cmd.get_args().map(|arg| arg.to_string_lossy()).collect();
    
    debug!("Executing command: {} {}", program, args.join(" "));
}