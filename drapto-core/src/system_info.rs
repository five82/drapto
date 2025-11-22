use std::process::Command;

#[derive(Debug, Clone)]
pub struct SystemInfo {
    pub hostname: String,
    pub os: String,
    pub cpu: String,
    pub memory: String,
}

impl SystemInfo {
    pub fn collect() -> Self {
        Self {
            hostname: get_hostname(),
            os: get_os_info(),
            cpu: get_cpu_info(),
            memory: get_memory_info(),
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

fn get_os_info() -> String {
    #[cfg(target_os = "macos")]
    {
        // Get macOS version
        Command::new("sw_vers")
            .args(["-productName"])
            .output()
            .and_then(|output| {
                let product = String::from_utf8_lossy(&output.stdout).trim().to_string();
                Command::new("sw_vers")
                    .args(["-productVersion"])
                    .output()
                    .map(|version_output| {
                        let version = String::from_utf8_lossy(&version_output.stdout)
                            .trim()
                            .to_string();
                        format!("{} {}", product, version)
                    })
            })
            .unwrap_or_else(|_| "macOS".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        // Try to get distro info from /etc/os-release
        std::fs::read_to_string("/etc/os-release")
            .ok()
            .and_then(|content| {
                let mut name = None;
                let mut version = None;

                for line in content.lines() {
                    if let Some(value) = line.strip_prefix("PRETTY_NAME=") {
                        // PRETTY_NAME usually has the full name and version
                        return Some(value.trim_matches('"').to_string());
                    }
                    if name.is_none() && line.starts_with("NAME=") {
                        name = line
                            .strip_prefix("NAME=")
                            .map(|s| s.trim_matches('"').to_string());
                    }
                    if version.is_none() && line.starts_with("VERSION=") {
                        version = line
                            .strip_prefix("VERSION=")
                            .map(|s| s.trim_matches('"').to_string());
                    }
                }

                match (name, version) {
                    (Some(n), Some(v)) => Some(format!("{} {}", n, v)),
                    (Some(n), None) => Some(n),
                    _ => None,
                }
            })
            .unwrap_or_else(|| {
                // Fallback to uname
                Command::new("uname")
                    .args(["-s", "-r"])
                    .output()
                    .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
                    .unwrap_or_else(|_| "Linux".to_string())
            })
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        std::env::consts::OS.to_string()
    }
}

fn get_cpu_info() -> String {
    #[cfg(target_os = "macos")]
    {
        // On macOS, use sysctl to get CPU info
        Command::new("sysctl")
            .args(["-n", "machdep.cpu.brand_string"])
            .output()
            .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_string())
            .unwrap_or_else(|_| "Unknown CPU".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, parse /proc/cpuinfo
        std::fs::read_to_string("/proc/cpuinfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("model name"))
                    .and_then(|line| line.split(':').nth(1))
                    .map(|name| name.trim().to_string())
            })
            .unwrap_or_else(|| "Unknown CPU".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "Unknown CPU".to_string()
    }
}

fn get_memory_info() -> String {
    #[cfg(target_os = "macos")]
    {
        // On macOS, use sysctl to get memory info
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .trim()
                    .parse::<u64>()
                    .map(|bytes| format_memory_size(bytes))
                    .unwrap_or_else(|_| "Unknown".to_string())
            })
            .unwrap_or_else(|_| "Unknown".to_string())
    }

    #[cfg(target_os = "linux")]
    {
        // On Linux, parse /proc/meminfo
        std::fs::read_to_string("/proc/meminfo")
            .ok()
            .and_then(|content| {
                content
                    .lines()
                    .find(|line| line.starts_with("MemTotal:"))
                    .and_then(|line| {
                        line.split_whitespace()
                            .nth(1)
                            .and_then(|kb_str| kb_str.parse::<u64>().ok())
                            .map(|kb| format_memory_size(kb * 1024)) // Convert KB to bytes
                    })
            })
            .unwrap_or_else(|| "Unknown".to_string())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        "Unknown".to_string()
    }
}

fn format_memory_size(bytes: u64) -> String {
    const GB: f64 = 1024.0 * 1024.0 * 1024.0;
    let gb_float = bytes as f64 / GB;

    // Round up to nearest 4 GB increment (modern memory configurations)
    let gb_rounded = ((gb_float / 4.0).ceil() * 4.0) as u64;
    format!("{} GB", gb_rounded)
}
