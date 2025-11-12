use clap::Parser;
use regex_classifier::patterns::extraction;
use serde_json::{json, Value};
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::path::PathBuf;
use regex::Regex;
use rayon::prelude::*;
use std::time::Instant;

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

// Process a chunk of lines in parallel
fn process_chunk(lines: &[String], regex_iso: &Regex, regex_syslog: &Regex, line_offset: usize) -> Vec<Value> {
    lines.iter().enumerate().flat_map(|(idx, line)| {
        let line_num = line_offset + idx + 1;
        
        // Extract timestamp
        let timestamp = regex_iso.captures(line)
            .and_then(|cap| cap.get(1))
            .or_else(|| regex_syslog.captures(line).and_then(|cap| cap.get(1)))
            .map(|m| m.as_str().to_string())
            .unwrap_or_else(|| "".to_string());
        
        // Process all pattern types for this line
        let mut line_findings = Vec::new();
        
        // Emails
        for email in extraction::email::extract_emails(line) {
            line_findings.push(json!({
                "line": line_num,
                "timestamp": timestamp,
                "type": "Email",
                "value": email
            }));
        }
        
        // IPs
        for ip in extraction::ip::extract_ips(line) {
            line_findings.push(json!({
                "line": line_num, 
                "timestamp": timestamp,
                "type": "IP",
                "value": ip
            }));
        }
        
        // DNSnames
        for dns in extraction::dnsname::extract_dnsnames(line) {
            line_findings.push(json!({
                "line": line_num,
                "timestamp": timestamp,
                "type": "DNSName",
                "value": dns
            }));
        }
        
        // URLs
        for url in extraction::url::extract_urls(line) {
            line_findings.push(json!({
                "line": line_num,
                "timestamp": timestamp,
                "type": "URL",
                "value": url
            }));
        }
        
        for username in extraction::username::extract_usernames(line) {
            line_findings.push(json!({
                "line": line_num,
                "timestamp": timestamp,
                "type": "Username", 
                "value": username
            }));
        }
        
        // TTY
        for tty in extraction::tty::extract_ttys(line) {
            line_findings.push(json!({
                "line": line_num,
                "timestamp": timestamp,
                "type": "TTY",
                "value": tty
            }));
        }
        
        line_findings
    }).collect()
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
    
    // Read lines into memory for parallel processing
    let lines: Vec<String> = reader.lines()
        .take(args.limit)
        .filter_map(Result::ok)
        .collect();
    
    println!("Processing {} lines in parallel chunks", lines.len());
    
    // Process in parallel chunks
    let chunk_size = 10_000;
    let mut findings = Vec::new();
    
    // Process chunks in parallel
    if !lines.is_empty() {
        let chunks: Vec<_> = lines.chunks(chunk_size).collect();
        println!("Processing {} chunks of up to {} lines each", chunks.len(), chunk_size);
        
        let start = Instant::now();
        
        // Use Rayon's parallel iterator
        let results: Vec<Vec<Value>> = chunks.par_iter()
            .enumerate()
            .map(|(chunk_idx, chunk)| {
                process_chunk(chunk, &iso_timestamp_regex, &syslog_timestamp_regex, chunk_idx * chunk_size)
            })
            .collect();
            
        // Flatten results
        for chunk_result in results {
            findings.extend(chunk_result);
        }
        
        println!("Parallel processing completed in {:.2?}", start.elapsed());
    }
    
    // Write JSON output
    println!("Found {} patterns in {} lines", findings.len(), lines.len());
    let json_output = serde_json::to_string_pretty(&findings)?;
    
    // Write to output file
    std::fs::write(&args.output, json_output)?;
    println!("Results written to {}", args.output.display());
    
    Ok(())
}