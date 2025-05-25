use std::time::Duration;

fn main() {
    // Test the new format
    let duration = Duration::from_secs(3725); // 1 hour, 2 minutes, 5 seconds
    println\!("Duration 3725s: {}", drapto_core::utils::format_duration(duration));
    
    let duration2 = Duration::from_secs(125); // 2 minutes, 5 seconds
    println\!("Duration 125s: {}", drapto_core::utils::format_duration(duration2));
    
    let duration3 = Duration::from_secs(45); // 45 seconds
    println\!("Duration 45s: {}", drapto_core::utils::format_duration(duration3));
    
    // Test format_duration_seconds directly
    println\!("Duration -10.0s: {}", drapto_core::utils::format_duration_seconds(-10.0));
    println\!("Duration NaN: {}", drapto_core::utils::format_duration_seconds(f64::NAN));
}
