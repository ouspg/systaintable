use lambda_http::{run, service_fn, Body, Error, Request, Response};
use regex_classifier::mcp_server::McpServer;
use regex_classifier::classify;
use regex_classifier::patterns::extraction;
use serde_json::{json, Value};
use std::collections::HashMap;
use aws_config;
use aws_sdk_s3;

async fn process_s3_file(
    bucket: &str, 
    key: &str,
    limit: Option<usize>,
    exclude: Option<&str>,
    sampling_rate: usize
) -> Result<Value, Error> {
    // Initialize AWS SDK
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);
    
    // Get object from S3
    let resp = client.get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    
    // Read content
    let data = resp.body.collect().await?;
    let content = String::from_utf8(data.to_vec())?;
    
    let excluded_categories: Vec<String> = match exclude {
        Some(excl) => excl.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new()
    };

    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut total_classifications = 0;
    let mut line_count = 0;
    let limit_val = limit.unwrap_or(usize::MAX);

    for line in content.lines() {
        line_count += 1;
        
        if line_count > limit_val { break; }
        if sampling_rate > 1 && line_count % sampling_rate != 0 { continue; }

        let mut values = Vec::new();

        // Extract values using extraction methods
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
        if !excluded_categories.contains(&"address".to_string()) {
            for address in extraction::address::extract_addresses(line) {
                values.push(address);
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
    
    // Format as JSON
    let mut category_stats = Vec::new();
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

    Ok(json!({
        "summary": {
            "total_lines_processed": line_count,
            "total_classifications": total_classifications,
            "source": format!("s3://{}/{}", bucket, key)
        },
        "categories": category_stats
    }))
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let server = McpServer::new();
    
    // Parse request
    let body_bytes = event.body().to_vec();
    let request_json = match serde_json::from_slice::<Value>(&body_bytes) {
        Ok(json) => json,
        Err(_) => return Ok(Response::builder()
            .status(400)
            .header("Content-Type", "application/json")
            .body("Invalid JSON request".into())?)
    };
    
    let id = request_json["id"].clone();
    
    // Check if it's an analyze_file request with S3 path
    if request_json["method"].as_str() == Some("tools/call") && 
       request_json["params"]["name"].as_str() == Some("analyze_file") {
        
        if let Some(file_path) = request_json["params"]["arguments"]["file_path"].as_str() {
            // Check if S3 path (s3://bucket/key)
            if file_path.starts_with("s3://") {
                let s3_path = &file_path[5..]; // Remove "s3://" prefix
                let parts: Vec<&str> = s3_path.splitn(2, '/').collect();
                
                if parts.len() == 2 {
                    let bucket = parts[0];
                    let key = parts[1];
                    
                    // Extract parameters
                    let limit = request_json["params"]["arguments"]["limit"]
                        .as_u64().map(|x| x as usize);
                    let exclude = request_json["params"]["arguments"]["exclude"].as_str();
                    let sampling_rate = request_json["params"]["arguments"]["sample"]
                        .as_u64().map(|x| x as usize).unwrap_or(1);
                    
                    // Process S3 file
                    match process_s3_file(bucket, key, limit, exclude, sampling_rate).await {
                        Ok(stats) => {
                            // Format response
                            let mut analysis_text = format!("S3 File Analysis: {}\n\n", file_path);
                            analysis_text.push_str(&format!("Processed {} lines, found {} classifications\n\n", 
                                stats["summary"]["total_lines_processed"], 
                                stats["summary"]["total_classifications"]));
                            
                            // Add categories to text
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
                            
                            let response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": analysis_text
                                    }],
                                    "data": stats
                                }
                            });
                            
                            let response_body = serde_json::to_string(&response)?;
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(Body::from(response_body))?)
                        },
                        Err(e) => {
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -1,
                                    "message": format!("Error processing S3 file: {}", e)
                                }
                            });
                            
                            let response_body = serde_json::to_string(&error_response)?;
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(Body::from(response_body))?)
                        }
                    }
                }
            }
        }
    }
    
    // For all other requests, use the standard MCP server
    match server.handle_request(request_json).await {
        Ok(mut response) => {
            // Ensure proper JSON-RPC format
            if let Some(obj) = response.as_object_mut() {
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                if !id.is_null() {
                    obj.insert("id".to_string(), id);
                }
            }
            
            let response_body = serde_json::to_string(&response)?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(response_body))?)
        },
        Err(err) => {
            let error_response = json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": -1,
                    "message": err
                }
            });
            
            let response_body = serde_json::to_string(&error_response)?;
            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "application/json")
                .body(Body::from(response_body))?)
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize AWS SDK
    aws_config::load_from_env().await;
    
    // Start the Lambda handler
    run(service_fn(function_handler)).await
}