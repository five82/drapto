use std::process::Command;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            hostname: get_hostname(),
        }
    }
}

fn get_hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| {
            Command::new("hostname")
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                .map_err(|_| std::env::VarError::NotPresent)
        })
        .unwrap_or_else(|_| "Unknown".to_string())
}
