use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle};
use regex_classifier::classify;
use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::fs;
use serde_json::json;
use std::time::{Instant};

// Import extraction patterns
mod patterns {
    pub mod extraction;
}
use patterns::extraction;

/// Regex classifier for identifying data types
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to log file for classification
    #[arg(index = 1)]
    file_path: PathBuf,

    /// Process only first N lines (optional)
    #[arg(short, long)]
    limit: Option<usize>,

    /// Show only specific categories (comma-separated)
    #[arg(short, long)]
    categories: Option<String>,

    /// Exclude specific categories (comma-separated)
    #[arg(short, long)]
    exclude: Option<String>,

    /// Verbose output with all classifications
    #[arg(short, long)]
    verbose: bool,
    
    /// Show statistics for classified values
    #[arg(short, long)]
    stats: bool,
    
    /// Number of threads to use (default: 4)
    #[arg(short, long, default_value = "4")]
    threads: usize,
    
    /// Sample 1 in N lines (for faster processing of huge files)
    #[arg(short = 'S', long)]
    sample: Option<usize>,
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // Create a SEPARATE counter that's immune to threading issues
    let main_thread_line_count = Arc::new(Mutex::new(0usize));
    
    // Keep your existing counters for compatibility
    let category_counts: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let total_classifications = Arc::new(Mutex::new(0usize));
    let processed_lines = Arc::new(Mutex::new(0usize));   
    
    // Track total lines for stats mode
    let line_count_for_stats = Arc::new(Mutex::new(0usize));
    // Open the file
    let file = File::open(&args.file_path)?;
    let file_size = file.metadata()?.len();

    // Process excluded categories
    let excluded_categories: Vec<String> = match &args.exclude {
        Some(excl) => excl.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new()
    };
    
    // Set up a fast line counter for large files
    let total_lines = if args.stats {
        println!("Estimating line count... (this might take a moment for large files)");
        let start = Instant::now();
        
        // For large files, sample the file to estimate total lines
        let est_lines = if file_size > 100_000_000 { // > 100MB
            // Count lines in first 10MB and use that to estimate
            let mut reader = BufReader::with_capacity(1_000_000, file.try_clone()?);
            let mut count = 0;
            let mut bytes_read = 0;
            let sample_size = 10_000_000; // 10MB
            
            let mut buffer = [0; 16384];
            loop {
                let bytes = reader.read(&mut buffer)?;
                if bytes == 0 { break; }
                
                bytes_read += bytes;
                count += buffer[..bytes].iter().filter(|&&b| b == b'\n').count();
                
                if bytes_read >= sample_size { break; }
            }
            
            // Estimate total lines based on sample
            (count as f64 * (file_size as f64 / bytes_read as f64)) as usize
        } else {
            // For smaller files, just count all lines
            let mut reader = BufReader::with_capacity(1_000_000, file.try_clone()?);
            reader.lines().count()
        };
        
        println!("Estimated {} lines (took {:?})", est_lines, start.elapsed());
        est_lines
    } else {
        0 // Not needed if not in stats mode
    };
    
    // Apply limit if specified
    let limit = args.limit.unwrap_or(usize::MAX);
    let process_total = match args.limit {
        Some(limit) if limit < total_lines => limit,
        _ => total_lines,
    };
    
    // Set up progress bar if stats mode is enabled
    let progress_bar = if args.stats {
        let pb = ProgressBar::new(process_total as u64);
        pb.set_style(ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} lines ({eta})")
            .unwrap()
            .progress_chars("#>-"));
        Some(pb)
    } else {
        None
    };
    
    // Create category counts that can be shared between threads
    let category_counts: Arc<Mutex<HashMap<String, usize>>> = Arc::new(Mutex::new(HashMap::new()));
    let total_classifications = Arc::new(Mutex::new(0usize));
    let processed_lines = Arc::new(Mutex::new(0usize));
    let line_count_for_stats = Arc::new(Mutex::new(0usize));
    
    if !args.stats {
        println!("Processing file: {}", args.file_path.display());
    }
    
    // Filter categories if specified
    let filter_categories: Vec<String> = match &args.categories {
        Some(cats) => cats.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new()
    };
    
    // Create a reader with a large buffer for efficiency
    let reader = BufReader::with_capacity(1_000_000, file);
    
    // Process the file
    let mut line_count = 0;
    let mut found_categories = false;
    let sampling_rate = args.sample.unwrap_or(1);

    for line_result in reader.lines() {
        line_count += 1;
        
        // ALWAYS update the processed lines counter, regardless of threading
        {
            let mut processed = processed_lines.lock().unwrap();
            *processed = line_count;
        }
        {
            let mut main_count = main_thread_line_count.lock().unwrap();
            *main_count = line_count;
        }        
        // Apply limit if specified
        if line_count > limit {
            break;
        }
        
        // Apply sampling if specified
        if sampling_rate > 1 && line_count % sampling_rate != 0 {
            continue;
        }
        
        // Update progress
        if let Some(pb) = &progress_bar {
            if line_count % 1000 == 0 || line_count == 1 {
                pb.set_position(line_count as u64);
            }
        }
        
        let line = line_result?;
        
        // The rest of your processing logic...        
        if !args.stats {
            println!("\nLine {}:", line_count);
        }
        
        // Extract values to classify
        let mut values = Vec::new();
        let mut debug_extractions = Vec::new();

        // Extract IPs
        if !excluded_categories.contains(&"ip".to_string()) {
            for ip in extraction::ip::extract_ips(&line) {
                debug_extractions.push(format!("IP: {}", ip));
                values.push(ip);
            }
        }

        // Extract MACs
        if !excluded_categories.contains(&"mac".to_string()) {
            for mac in extraction::mac::extract_macs(&line) {
                debug_extractions.push(format!("MAC: {}", mac));
                values.push(mac);
            }
        }

        // Extract domain names
        if !excluded_categories.contains(&"dnsname".to_string()) {
            for dns in extraction::dnsname::extract_dnsnames(&line) {
                debug_extractions.push(format!("Domain: {}", dns));
                values.push(dns);
            }
        }

        // Extract emails
        if !excluded_categories.contains(&"email".to_string()) {
            for email in extraction::email::extract_emails(&line) {
                debug_extractions.push(format!("Email: {}", email));
                values.push(email);
            }
        }

        // Extract phone numbers
        if !excluded_categories.contains(&"phonenumber".to_string()) {
            for phone in extraction::phonenumber::extract_phonenumbers(&line) {
                debug_extractions.push(format!("Phone: {}", phone));
                values.push(phone);
            }
        }

        // Extract PIDs
        if !excluded_categories.contains(&"pid".to_string()) {
            for pid in extraction::pid::extract_pids(&line) {
                debug_extractions.push(format!("PID: {}", pid));
                values.push(pid);
            }
        }

        // Extract times
        if !excluded_categories.contains(&"time".to_string()) {
            for time in extraction::time::extract_times(&line) {
                debug_extractions.push(format!("Time: {}", time));
                values.push(time);
            }
        }

        // Extract TTYs
        if !excluded_categories.contains(&"tty".to_string()) {
            for tty in extraction::tty::extract_ttys(&line) {
                debug_extractions.push(format!("TTY: {}", tty));
                values.push(tty);
            }
        }

        // Extract URLs
        if !excluded_categories.contains(&"url".to_string()) {
            for url in extraction::url::extract_urls(&line) {
                debug_extractions.push(format!("URL: {}", url));
                values.push(url);
            }
        }

        // Extract addresses
        if !excluded_categories.contains(&"address".to_string()) {
            for address in extraction::address::extract_addresses(&line) {
                debug_extractions.push(format!("Address: {}", address));
                values.push(address);
            }
        }
        
        // Debug: show all extracted values
        if args.verbose && !args.stats {
            println!("Extracted {} values:", debug_extractions.len());
            for extraction in debug_extractions {
                println!("  {}", extraction);
            }
        }
        
        // Classify each extracted value
        found_categories = false;
        for value in values {
            // Skip very short values and common JSON literals
            if value.len() < 3 || ["null", "true", "false"].contains(&value.as_str()) {
                continue;
            }
            
            let mut categories = classify(&value);
            
            // Filter out excluded categories
            if !excluded_categories.is_empty() {
                categories.retain(|c| !excluded_categories.contains(c));
            }
            
            // Skip if no categories or filter doesn't match
            if categories.is_empty() || 
               (!filter_categories.is_empty() && !categories.iter().any(|c| filter_categories.contains(c))) {
                continue;
            }
            
            // Update statistics - MODIFIED SECTION
            if args.stats {
                let mut counts = category_counts.lock().unwrap();
                let mut total = total_classifications.lock().unwrap(); // Remove the *
                
                for category in &categories {
                    *counts.entry(category.clone()).or_insert(0) += 1;
                    *total += 1; // Now this will work
                }
                // Remove the println statements from here - they belong outside the loop
            }
            
            if !args.stats {
                println!("  \"{}\" => {}", value, categories.join(", "));
                found_categories = true;
            }
        }
        
        if !found_categories && !args.stats {
            println!("  No pattern matches found");
        }
    }
    
    // Move the statistics display OUTSIDE the processing loop

    // Display statistics if enabled
    if args.stats {
        let counts = category_counts.lock().unwrap();
        let total = *total_classifications.lock().unwrap();
        
        // Use the simple line_count variable that we know works
        let processed = line_count; // This is the actual line counter from the loop
        
        // Create JSON statistics
        let mut category_stats: Vec<serde_json::Value> = Vec::new();
        
        // Sort categories by count (highest first)
        let mut categories: Vec<_> = counts.iter().collect();
        categories.sort_by(|a, b| b.1.cmp(a.1));
        
        for (category, count) in categories {
            let percentage = if total > 0 {
                ((*count as f64) / (total as f64) * 100.0).round()
            } else {
                0.0
            };
            
            category_stats.push(json!({
                "category": category,
                "count": count,
                "percentage": percentage
            }));
        }
        
        let stats_json = json!({
            "summary": {
                "total_lines_processed": processed,
                "total_classifications": total,
                "file_path": args.file_path.to_string_lossy()
            },
            "categories": category_stats
        });
        
        // Write to outputstats.json
        match fs::write("outputstats.json", serde_json::to_string_pretty(&stats_json)?) {
            Ok(_) => println!("Statistics written to outputstats.json"),
            Err(e) => eprintln!("Error writing statistics file: {}", e),
        }
    }
    Ok(())
}