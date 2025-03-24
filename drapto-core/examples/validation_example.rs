//! Media Validation Example
//!
//! This example demonstrates drapto's validation capabilities:
//! 1. Running comprehensive validation on a single media file
//! 2. Comparing input and output files to validate encoding quality and correctness
//! 3. Working with validation reports to identify issues and warnings
//! 4. Handling different validation scenarios with appropriate checks
//!
//! The example supports two modes:
//! - Single file validation: Quality checks, stream validation, etc.
//! - Input/output comparison: Ensuring encoded output matches the source appropriately
//!
//! Run with:
//! ```
//! # Validate a single file
//! cargo run --example validation_example <media_file>
//!
//! # Compare input and output files
//! cargo run --example validation_example <input_file> <output_file>
//! ```

use std::env;
use std::path::Path;
use drapto_core::validation::{comprehensive_validation, validate_output};
use drapto_core::error::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    
    if args.len() < 2 {
        println!("Usage:");
        println!("  validation_example <media_file>              - Run comprehensive validation on a single file");
        println!("  validation_example <input_file> <output_file> - Compare input and output files");
        return Ok(());
    }
    
    if args.len() == 2 {
        // Single file validation
        let file_path = Path::new(&args[1]);
        
        println!("Running comprehensive validation on: {}", file_path.display());
        
        let report = comprehensive_validation(file_path, None)?;
        
        println!("\n{}\n", report);
        
        if report.passed {
            println!("✅ Validation passed with {} warning(s)", report.warnings().len());
        } else {
            println!("❌ Validation failed with {} error(s) and {} warning(s)", 
                    report.errors().len(), report.warnings().len());
        }
    } else if args.len() >= 3 {
        // Compare input and output
        let input_path = Path::new(&args[1]);
        let output_path = Path::new(&args[2]);
        
        println!("Comparing input: {} with output: {}", 
                input_path.display(), output_path.display());
        
        let report = validate_output(input_path, output_path, None)?;
        
        println!("\n{}\n", report);
        
        if report.passed {
            println!("✅ Comparison passed with {} warning(s)", report.warnings().len());
        } else {
            println!("❌ Comparison failed with {} error(s) and {} warning(s)", 
                    report.errors().len(), report.warnings().len());
        }
    }
    
    Ok(())
}