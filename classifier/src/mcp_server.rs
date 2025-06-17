use serde_json::{json, Value};
use std::collections::HashMap;
use std::io::{self, BufRead, BufReader};
use std::fs::File;
use crate::classify;
use crate::patterns::extraction;

#[derive(Clone)]
pub struct McpServer;

impl McpServer {
    pub fn new() -> Self {
        Self
    }

    pub async fn handle_request(&self, request: Value) -> Result<Value, String> {
        let method = request["method"].as_str().unwrap_or("");
        
        match method {
            "initialize" => self.handle_initialize(),
            "tools/list" => self.handle_tools_list(),
            "tools/call" => self.handle_tool_call(request["params"].clone()),
            _ => Err(format!("Unknown method: {}", method))
        }
    }

    fn handle_initialize(&self) -> Result<Value, String> {
        Ok(json!({
            "result": {
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "regex-classifier",
                    "version": "1.0.0"
                }
            }
        }))
    }
    fn handle_tools_list(&self) -> Result<Value, String> {
        Ok(json!({
            "result": {
                "tools": [
                    {
                        "name": "analyze_file",
                        "description": "Analyze a file for PII patterns and return classification statistics",
                        "inputSchema": {  // Change from "schema" to "inputSchema"
                            "type": "object",
                            "properties": {
                                "file_path": {
                                    "type": "string",
                                    "description": "Path to the file to analyze"
                                },
                                "limit": {
                                    "type": "integer",
                                    "description": "Process only first N lines (optional)"
                                },
                                "exclude": {
                                    "type": "string",
                                    "description": "Exclude specific categories (comma-separated)"
                                },
                                "sample": {
                                    "type": "integer",
                                    "description": "Sample 1 in N lines for faster processing"
                                }
                            },
                            "required": ["file_path"]
                        }
                    },
                    {
                        "name": "analyze_text",
                        "description": "Analyze text content directly for PII patterns",
                        "inputSchema": {  // Change from "schema" to "inputSchema"
                            "type": "object",
                            "properties": {
                                "text": {
                                    "type": "string",
                                    "description": "Text content to analyze"
                                },
                                "exclude": {
                                    "type": "string",
                                    "description": "Exclude specific categories (comma-separated)"
                                }
                            },
                            "required": ["text"]
                        }
                    }
                ]
            }
        }))
    }
    
    fn handle_tool_call(&self, params: Value) -> Result<Value, String> {
        let tool_name = params["name"].as_str()
            .ok_or("Missing tool name")?;
    
        let arguments = &params["arguments"];
    
        match tool_name {
            "analyze_file" => self.analyze_file(arguments),
            "analyze_text" => self.analyze_text(arguments),
            "analyze_content" => self.analyze_text(arguments), // Reuse the analyze_text implementation
            "get_upload_url" => {
                // This should be handled directly in lambda.rs, but we need to avoid returning an error
                Ok(json!({
                    "result": {
                        "message": "Please use the API directly for this operation"
                    }
                }))
            },
            _ => Err(format!("Unknown tool: {}", tool_name))
        }
    }

    fn analyze_file(&self, args: &Value) -> Result<Value, String> {
        let file_path = args["file_path"].as_str()
            .ok_or("file_path is required")?;
        
        let limit = args["limit"].as_u64().map(|x| x as usize);
        let exclude = args["exclude"].as_str();
        let categories = args["categories"].as_str();
        let sample = args["sample"].as_u64().map(|x| x as usize).unwrap_or(1);
    
        let stats = self.process_file(file_path, limit, exclude, categories, sample)
            .map_err(|e| format!("Error processing file: {}", e))?;
        
        // Format a clear response with categories emphasized
        let mut analysis_text = format!("File Analysis Complete: {}\n\n", file_path);
        analysis_text.push_str(&format!("Processed {} lines, found {} classifications\n\n", 
            stats["summary"]["total_lines_processed"], 
            stats["summary"]["total_classifications"]));
        
        // Add detailed breakdown by category
        analysis_text.push_str("PII Categories Detected:\n");
        
        if let Some(categories) = stats["categories"].as_array() {
            if categories.is_empty() {
                analysis_text.push_str("No PII detected in file\n");
            } else {
                for category in categories {
                    if let (Some(cat_name), Some(count), Some(percentage)) = (
                        category["category"].as_str(),
                        category["count"].as_u64(),
                        category["percentage"].as_f64()
                    ) {
                        analysis_text.push_str(&format!("- {}: {} occurrences ({}%)\n", 
                            cat_name, count, percentage));
                    }
                }
            }
        }
    
        Ok(json!({
            "result": {
                "content": [{
                    "type": "text",
                    "text": analysis_text
                }],
                "data": stats  // Keep the raw data for potential further processing
            }
        }))
    }

    fn analyze_text(&self, args: &Value) -> Result<Value, String> {
        let text = args["text"].as_str()
            .ok_or("text is required")?;
        
        let exclude = args["exclude"].as_str();
        let excluded_categories: Vec<String> = match exclude {
            Some(excl) => excl.split(',').map(|s| s.trim().to_string()).collect(),
            None => Vec::new()
        };
    
        let mut category_counts: HashMap<String, usize> = HashMap::new();
        let mut total_classifications = 0;
    
        // Split text into lines and process each
        for line in text.lines() {
            let mut values = Vec::new();
    
            // Extract values using your existing extraction logic
            if !excluded_categories.contains(&"ip".to_string()) {
                for ip in extraction::ip::extract_ips(line) {
                    values.push(ip);
                }
            }
            if !excluded_categories.contains(&"email".to_string()) {
                for email in extraction::email::extract_emails(line) {
                    values.push(email);
                }
            }
            if !excluded_categories.contains(&"phonenumber".to_string()) {
                for phone in extraction::phonenumber::extract_phonenumbers(line) {
                    values.push(phone);
                }
            }
            if !excluded_categories.contains(&"mac".to_string()) {
                for mac in extraction::mac::extract_macs(line) {
                    values.push(mac);
                }
            }
            if !excluded_categories.contains(&"url".to_string()) {
                for url in extraction::url::extract_urls(line) {
                    values.push(url);
                }
            }
            if !excluded_categories.contains(&"address".to_string()) {
                for address in extraction::address::extract_addresses(line) {
                    values.push(address);
                }
            }
            if !excluded_categories.contains(&"dnsname".to_string()) {
                for dns in extraction::dnsname::extract_dnsnames(line) {
                    values.push(dns);
                }
            }
            if !excluded_categories.contains(&"time".to_string()) {
                for time in extraction::time::extract_times(line) {
                    values.push(time);
                }
            }
            if !excluded_categories.contains(&"tty".to_string()) {
                for tty in extraction::tty::extract_ttys(line) {
                    values.push(tty);
                }
            }
            if !excluded_categories.contains(&"pid".to_string()) {
                for pid in extraction::pid::extract_pids(line) {
                    values.push(pid);
                }
            }
    
            // Classify each value
            for value in values {
                if value.len() < 3 || ["null", "true", "false"].contains(&value.as_str()) { 
                    continue; 
                }
                
                let mut categories = classify(&value);
                
                if !excluded_categories.is_empty() {
                    categories.retain(|c| !excluded_categories.contains(c));
                }
    
                for category in categories {
                    *category_counts.entry(category).or_insert(0) += 1;
                    total_classifications += 1;
                }
            }
        }
    
        // Store the count before moving category_counts
        let num_categories = category_counts.len();
        let stats = self.build_stats_json(text.lines().count(), total_classifications, category_counts, "text_input");
    
        // Return proper MCP response format
        Ok(json!({
            "result": {
                "content": [{
                    "type": "text",
                    "text": format!("Text analysis complete. Found {} classifications across {} categories", 
                        total_classifications, num_categories)
                }],
                "_meta": stats
            }
        }))
    }

    fn process_file(&self, file_path: &str, limit: Option<usize>, exclude: Option<&str>, 
                   _categories: Option<&str>, sampling_rate: usize) -> io::Result<Value> {
        let file = File::open(file_path)?;
        let reader = BufReader::with_capacity(1_000_000, file);
        
        let excluded_categories: Vec<String> = match exclude {
            Some(excl) => excl.split(',').map(|s| s.trim().to_string()).collect(),
            None => Vec::new()
        };

        let mut category_counts: HashMap<String, usize> = HashMap::new();
        let mut total_classifications = 0;
        let mut line_count = 0;
        let limit = limit.unwrap_or(usize::MAX);

        for line_result in reader.lines() {
            line_count += 1;
            
            if line_count > limit { break; }
            if sampling_rate > 1 && line_count % sampling_rate != 0 { continue; }

            let line = line_result?;
            let mut values = Vec::new();

            // Use your existing extraction logic
            if !excluded_categories.contains(&"ip".to_string()) {
                for ip in extraction::ip::extract_ips(&line) {
                    values.push(ip);
                }
            }
            if !excluded_categories.contains(&"email".to_string()) {
                for email in extraction::email::extract_emails(&line) {
                    values.push(email);
                }
            }
            if !excluded_categories.contains(&"phonenumber".to_string()) {
                for phone in extraction::phonenumber::extract_phonenumbers(&line) {
                    values.push(phone);
                }
            }
            // Add other extractors as needed...

            // Classify each extracted value
            for value in values {
                if value.len() < 3 { continue; }
                
                let mut categories = classify(&value);
                
                if !excluded_categories.is_empty() {
                    categories.retain(|c| !excluded_categories.contains(c));
                }

                for category in categories {
                    *category_counts.entry(category).or_insert(0) += 1;
                    total_classifications += 1;
                }
            }
        }

        Ok(self.build_stats_json(line_count, total_classifications, category_counts, file_path))
    }

    fn build_stats_json(&self, lines_processed: usize, total_classifications: usize, 
                       category_counts: HashMap<String, usize>, source: &str) -> Value {
        let mut category_stats: Vec<Value> = Vec::new();
        let mut categories: Vec<_> = category_counts.iter().collect();
        categories.sort_by(|a, b| b.1.cmp(a.1));

        for (category, count) in categories {
            let percentage = if total_classifications > 0 {
                ((*count as f64) / (total_classifications as f64) * 100.0).round()
            } else {
                0.0
            };

            category_stats.push(json!({
                "category": category,
                "count": count,
                "percentage": percentage
            }));
        }

        json!({
            "summary": {
                "total_lines_processed": lines_processed,
                "total_classifications": total_classifications,
                "source": source
            },
            "categories": category_stats
        })
    }
}