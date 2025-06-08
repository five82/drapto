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
    println!("  {} {}", style("✓").bold(), style(success_message).bold());
    
    for (key, value) in items {
        println!("  {:<18} {}", format!("{}:", key), value);
    }
}

fn render_completion_summary(title: &str, success_message: &str, groups: &[GroupData]) {
    render_section_header(title);
    
    println!("  {} {}", style("✓").bold(), style(success_message).bold());
    
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
}