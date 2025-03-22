use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        eprintln!("Usage: {} <filepath>", args[0]);
        return;
    }
    
    let filepath = &args[1];
    let path = Path::new(filepath);
    
    if !path.exists() {
        eprintln!("Error: File does not exist: {}", filepath);
        return;
    }
    
    match fs::metadata(path) {
        Ok(metadata) => {
            let size = metadata.len();
            println!("File: {}", filepath);
            println!("Size (fs::metadata): {} bytes", size);
            println!("             or {:.2} MB", size as f64 / 1024.0 / 1024.0);
        }
        Err(e) => {
            eprintln!("Error getting metadata: {}", e);
        }
    }
    
    // Try command line tools for comparison
    let output = std::process::Command::new("ls")
        .args(["-la", filepath])
        .output();
        
    if let Ok(output) = output {
        if output.status.success() {
            println!("\nls -la output:");
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
    
    let output = std::process::Command::new("du")
        .args(["-h", filepath])
        .output();
        
    if let Ok(output) = output {
        if output.status.success() {
            println!("du -h output:");
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }
    }
}