use clap::Parser;
use regex_classifier::patterns::extraction;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use regex::Regex;

#[derive(Parser, Debug)]
#[command(author, version, about = "Extract patterns from logs with JSON output")]
struct Args {
    /// Path to log file for classification
    #[arg(index = 1)]
    file_path: PathBuf,

    /// Process only first N lines (default: 1000)
    #[arg(short, long, default_value = "1000")]
    limit: usize,

    /// Output JSON file path
    #[arg(short, long, default_value = "findings.json")]
    output: PathBuf,
}

fn main() -> io::Result<()> {
    let args = Args::parse();
    
    // Open the file
    println!("Processing file: {}", args.file_path.display());
    let file = File::open(&args.file_path)?;
    let reader = BufReader::with_capacity(1_000_000, file);
    
    let timestamp_regex = Regex::new(r"(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)").unwrap();
    
    // Process the file
    let mut findings = Vec::new();
    let mut line_count = 0;
    
    for line_result in reader.lines() {
        line_count += 1;
        if line_count > args.limit {
            break;
        }
        
        let line = line_result?;
        
        // Extract timestamp if present
        let timestamp = timestamp_regex.captures(&line)
            .and_then(|cap| cap.get(1))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "".to_string());
        
        // Process each pattern type
        // Emails
        for email in extraction::email::extract_emails(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "Email",
                "value": email
            }));
        }
        
        // IPs
        for ip in extraction::ip::extract_ips(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "IP",
                "value": ip
            }));
        }
        
        // DNS names
        for dns in extraction::dnsname::extract_dnsnames(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "Hostname",
                "value": dns
            }));
        }
        
        // URLs
        for url in extraction::url::extract_urls(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "URL",
                "value": url
            }));
        }
        
        // You can add more pattern types as needed
    }
    
    // Write JSON output
    println!("Found {} patterns in {} lines", findings.len(), line_count);
    let json_output = serde_json::to_string_pretty(&findings)?;
    
    // Write to output file
    std::fs::write(&args.output, json_output)?;
    println!("Results written to {}", args.output.display());
    
    Ok(())
}