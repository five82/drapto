use log::LevelFilter;
use log4rs::{
    append::file::FileAppender,
    config::{Appender, Config, Root},
    encode::pattern::PatternEncoder,
};
use std::path::Path;
use anyhow::Result;

pub fn setup_file_logging(log_file: &Path, log_level: LevelFilter) -> Result<()> {
    // Create log directory if it doesn't exist
    if let Some(parent) = log_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // File appender with clean format for log files
    let file_appender = FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new(
            "{d(%Y-%m-%d %H:%M:%S)} [{l}] {m}{n}"
        )))
        .build(log_file)?;

    let config = Config::builder()
        .appender(Appender::builder().build("file", Box::new(file_appender)))
        .build(Root::builder().appender("file").build(log_level))?;

    log4rs::init_config(config)?;
    
    Ok(())
}