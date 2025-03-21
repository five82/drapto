use drapto_core::validation;
use drapto_core::media::MediaInfo;
use std::env;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if a filename was provided
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: {} <media_file>", args[0]);
        return Ok(());
    }
    
    let filename = &args[1];
    println!("Analyzing file: {}", filename);
    
    // Get media info
    let media_info = MediaInfo::from_path(filename)?;
    
    // Count streams
    let subtitle_streams = media_info.subtitle_streams();
    println!("Found {} subtitle stream(s)", subtitle_streams.len());
    
    // Run validation
    let mut report = validation::ValidationReport::new();
    validation::subtitles::validate_subtitles(&media_info, &mut report);
    
    // Print report
    println!("\nValidation Report:");
    println!("{}", report);
    
    Ok(())
}