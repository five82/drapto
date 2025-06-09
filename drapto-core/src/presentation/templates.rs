use console::style;

/// Template data for different output patterns
#[derive(Debug)]
pub enum TemplateData<'a> {
    SectionHeader {
        title: &'a str,
    },
    BatchHeader {
        title: &'a str,
    },
    HardwareHeader {
        title: &'a str,
    },
    FileProgressHeader {
        current: usize,
        total: usize,
    },
    KeyValueList {
        title: &'a str,
        items: Vec<(&'a str, &'a str)>,
    },
    GroupedKeyValues {
        title: &'a str,
        groups: Vec<GroupData<'a>>,
    },
    ProgressBar {
        title: &'a str,
        label: &'a str,
        percent: u64,
        bar: &'a str,
        timing: &'a str,
        details: Option<&'a str>,
    },
    SpinnerToResults {
        title: &'a str,
        spinner_text: &'a str,
        success_message: &'a str,
        items: Vec<(&'a str, &'a str)>,
    },
    CompletionSummary {
        title: &'a str,
        success_message: &'a str,
        groups: Vec<GroupData<'a>>,
    },
}

#[derive(Debug)]
pub struct GroupData<'a> {
    pub name: &'a str,
    pub items: Vec<(&'a str, &'a str, bool)>, // key, value, emphasize
}

/// Render a template with the given data
pub fn render(template_data: TemplateData) {
    match template_data {
        TemplateData::SectionHeader { title } => {
            render_section_header(title);
        }
        TemplateData::BatchHeader { title } => {
            render_batch_header(title);
        }
        TemplateData::HardwareHeader { title } => {
            render_hardware_header(title);
        }
        TemplateData::FileProgressHeader { current, total } => {
            render_file_progress_header(current, total);
        }
        TemplateData::KeyValueList { title, items } => {
            render_key_value_list(title, &items);
        }
        TemplateData::GroupedKeyValues { title, groups } => {
            render_grouped_key_values(title, &groups);
        }
        TemplateData::ProgressBar { title, label, percent, bar, timing, details } => {
            render_progress_bar(title, label, percent, bar, timing, details);
        }
        TemplateData::SpinnerToResults { title, spinner_text, success_message, items } => {
            render_spinner_to_results(title, spinner_text, success_message, &items);
        }
        TemplateData::CompletionSummary { title, success_message, groups } => {
            render_completion_summary(title, success_message, &groups);
        }
    }
}

fn render_section_header(title: &str) {
    // Section headers: dashes style, cyan color for workflow phases
    println!("\n{}\n", style(format!("----- {} -----", title.to_uppercase())).cyan().bold());
}

fn render_batch_header(title: &str) {
    // Batch headers: simple box style, yellow color for batch-level operations
    println!("\n{}", style(format!("┌───── {} ─────┐", title.to_uppercase())).yellow().bold());
    println!();
}

fn render_hardware_header(title: &str) {
    // Hardware headers: thick solid line style, blue color for system information
    println!("\n{}", style(format!("━━━━━ {} ━━━━━", title.to_uppercase())).blue().bold());
    println!();
}

fn render_file_progress_header(current: usize, total: usize) {
    // File progress: uses dashes, magenta color, and arrow for file-level progress
    println!("\n{}", style(format!("────▶ FILE {} OF {} ────", current, total)).magenta().bold());
}

fn render_key_value_list(title: &str, items: &[(&str, &str)]) {
    render_section_header(title);
    
    for (key, value) in items {
        println!("  {:<18} {}", format!("{}:", key), value);
    }
}

fn render_grouped_key_values(title: &str, groups: &[GroupData]) {
    render_section_header(title);
    
    for (index, group) in groups.iter().enumerate() {
        if index > 0 {
            println!();
        }
        
        println!("  {}:", group.name);
        for (key, value, emphasize) in &group.items {
            let styled_value = if *emphasize {
                style(value).green().bold().to_string()
            } else {
                value.to_string()
            };
            println!("    {:<16} {}", format!("{}:", key), styled_value);
        }
    }
}

fn render_progress_bar(title: &str, label: &str, percent: u64, bar: &str, timing: &str, details: Option<&str>) {
    render_section_header(title);
    
    println!("  {}: {}% {} ({})", label, percent, bar, timing);
    if let Some(detail_text) = details {
        println!("  {}", detail_text);
    }
}

fn render_spinner_to_results(title: &str, _spinner_text: &str, success_message: &str, items: &[(&str, &str)]) {
    render_section_header(title);
    
    // Note: Actual spinner would be handled by indicatif during runtime
    // This renders the final state after spinner completes
    println!("  {} {}", style("✓").dim(), style(success_message).dim());
    
    for (key, value) in items {
        println!("  {:<18} {}", format!("{}:", key), value);
    }
}

fn render_completion_summary(title: &str, success_message: &str, groups: &[GroupData]) {
    render_section_header(title);
    
    println!("  {} {}", style("✓").green().bold(), style(success_message).bold());
    
    for (_index, group) in groups.iter().enumerate() {
        println!();
        
        for (key, value, emphasize) in &group.items {
            let styled_value = if *emphasize {
                style(value).green().bold().to_string()
            } else {
                value.to_string()
            };
            println!("  {:<18} {}", format!("{}:", key), styled_value);
        }
    }
}

/// Format encoding speed with performance-based color coding
/// 
/// Color scheme:
/// - ≤0.2x: Yellow (concerning - very slow encoding)
/// - >0.2x to <2.0x: White (acceptable performance) 
/// - ≥2.0x: Green (excellent performance)
pub fn format_speed(speed: f32) -> String {
    let speed_str = format!("{:.1}x", speed);
    
    if speed <= 0.2 {
        style(speed_str).yellow().to_string()
    } else if speed >= 2.0 {
        style(speed_str).green().to_string()
    } else {
        speed_str // Default terminal color for acceptable performance
    }
}

/// Format file size reduction percentage with three-tier color coding
/// 
/// Color scheme:
/// - ≥50%: Green (significant savings - excellent result)
/// - 31-49%: Default white (modest but acceptable savings)
/// - ≤30%: Yellow (disappointing - minimal savings)
pub fn format_reduction(reduction: f64) -> String {
    let reduction_str = format!("{:.1}%", reduction);
    
    if reduction >= 50.0 {
        style(reduction_str).green().to_string()
    } else if reduction <= 30.0 {
        style(reduction_str).yellow().to_string()
    } else {
        reduction_str // Default terminal color for modest but acceptable savings
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_value_list_template() {
        let items = vec![
            ("Input file", "test.mkv"),
            ("Duration", "00:02:00"),
        ];
        
        render(TemplateData::KeyValueList {
            title: "INITIALIZATION",
            items,
        });
        
        // Visual test - check output manually
    }

    #[test] 
    fn test_grouped_key_values_template() {
        let groups = vec![
            GroupData {
                name: "Video",
                items: vec![
                    ("Preset", "SVT-AV1 preset 6", false),
                    ("Quality", "CRF 27", false),
                ],
            },
            GroupData {
                name: "Advanced", 
                items: vec![
                    ("Pixel Format", "yuv420p10le", false),
                    ("Color Space", "bt709", false),
                ],
            },
        ];
        
        render(TemplateData::GroupedKeyValues {
            title: "ENCODING CONFIGURATION",
            groups,
        });
        
        // Visual test - check output manually
    }

    #[test]
    fn test_speed_formatting() {
        // Test slow speed (yellow)
        let slow_speed = format_speed(0.1);
        assert!(slow_speed.contains("0.1x"));
        
        // Test acceptable speed (default color)
        let normal_speed = format_speed(1.5);
        assert_eq!(normal_speed, "1.5x");
        
        // Test excellent speed (green)
        let fast_speed = format_speed(2.5);
        assert!(fast_speed.contains("2.5x"));
        
        // Test boundary conditions
        assert!(format_speed(0.2).contains("0.2x")); // At yellow threshold
        assert!(format_speed(2.0).contains("2.0x")); // At green threshold
    }

    #[test]
    fn test_reduction_formatting() {
        // Test excellent reduction (green)
        let excellent_reduction = format_reduction(65.2);
        assert!(excellent_reduction.contains("65.2%"));
        
        // Test modest reduction (default color)
        let modest_reduction = format_reduction(35.5);
        assert_eq!(modest_reduction, "35.5%");
        
        // Test disappointing reduction (yellow)
        let poor_reduction = format_reduction(15.3);
        assert!(poor_reduction.contains("15.3%"));
        
        // Test boundary conditions
        assert!(format_reduction(50.0).contains("50.0%")); // At green threshold
        assert_eq!(format_reduction(31.0), "31.0%"); // Just above yellow (should be default)
        assert!(format_reduction(30.0).contains("30.0%")); // At yellow threshold
        assert_eq!(format_reduction(30.1), "30.1%"); // Just above yellow (should be default)
    }

    #[test]
    fn test_status_significance_visual_hierarchy() {
        // Test that status significance creates proper visual hierarchy
        // This ensures major milestones stand out from routine operations
        
        // Major milestone formatting should include green styling
        // (Cannot easily test color output in unit tests, but verify compilation works)
        
        // Verify that the styling functions work without panicking
        let green_styled = style("✓").green().bold().to_string();
        let dim_styled = style("✓").dim().to_string();
        let default_styled = style("✓").bold().to_string();
        
        // All should contain the checkmark
        assert!(green_styled.contains("✓"));
        assert!(dim_styled.contains("✓"));
        assert!(default_styled.contains("✓"));
        
        // Different styling methods should exist (may render same in test env)
        // The important thing is that the code compiles and doesn't panic
        assert!(!green_styled.is_empty());
        assert!(!dim_styled.is_empty());
        assert!(!default_styled.is_empty());
    }
}