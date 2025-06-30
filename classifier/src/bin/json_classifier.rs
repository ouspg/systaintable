use clap::Parser;
use regex_classifier::patterns::extraction;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::PathBuf;
use regex::Regex;
use rayon::prelude::*;

// Process chunks in parallel
let chunk_size = 10_000;
let results: Vec<_> = lines.par_chunks(chunk_size)
    .map(|chunk| process_chunk(chunk))
    .collect();

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
    
    // Define two timestamp patterns - ISO format and syslog format
    let iso_timestamp_regex = Regex::new(r"(\d{4}-\d{2}-\d{2}[T\s]\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:?\d{2})?)").unwrap();
    let syslog_timestamp_regex = Regex::new(r"((?:Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec)\s+\d{1,2}\s+\d{2}:\d{2}:\d{2})").unwrap();
    
    // Process the file
    let mut findings = Vec::new();
    let mut line_count = 0;
    
    for line_result in reader.lines() {
        line_count += 1;
        if line_count > args.limit {
            break;
        }
        
        let line = line_result?;
        
        // Try both timestamp formats - ISO first, then syslog
        let timestamp = iso_timestamp_regex.captures(&line)
            .and_then(|cap| cap.get(1))
            .or_else(|| syslog_timestamp_regex.captures(&line).and_then(|cap| cap.get(1)))
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
                "type": "DNSname",
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
        
        // Usernames (from SSH logs, etc.)
        for username in extraction::username::extract_usernames(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "Username",
                "value": username
            }));
        }
        
        for tty in extraction::tty::extract_ttys(&line) {
            findings.push(json!({
                "line": line_count,
                "timestamp": timestamp,
                "type": "TTY",
                "value": tty
            }));
        }
    }
    
    // Write JSON output
    println!("Found {} patterns in {} lines", findings.len(), line_count);
    let json_output = serde_json::to_string_pretty(&findings)?;
    
    // Write to output file
    std::fs::write(&args.output, json_output)?;
    println!("Results written to {}", args.output.display());
    
    Ok(())
}