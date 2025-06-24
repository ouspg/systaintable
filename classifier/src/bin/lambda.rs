use lambda_http::{run, service_fn, Body, Error, Request, Response};
use regex_classifier::mcp_server::McpServer;
use regex_classifier::patterns::extraction;
use serde_json::{json, Value};
use std::collections::HashMap;
use aws_config;
use aws_sdk_s3;
use uuid::Uuid;
use aws_sdk_s3::presigning::PresigningConfig;

async fn process_s3_file(
    bucket: &str, 
    key: &str,
    limit: Option<usize>,
    exclude: Option<&str>,
    sampling_rate: usize
) -> Result<Value, Error> {
    // Set up S3 client
    let config = aws_config::load_from_env().await;
    let s3_client = aws_sdk_s3::Client::new(&config);
    
    // Get the object from S3
    let response = s3_client.get_object()
        .bucket(bucket)
        .key(key)
        .send()
        .await?;
    
    let data = response.body.collect().await?.to_vec();
    let content = String::from_utf8(data.to_vec())?;
    
    let excluded_categories: Vec<String> = match exclude {
        Some(excl) => excl.split(',').map(|s| s.trim().to_string()).collect(),
        None => Vec::new()
    };

    let mut category_counts: HashMap<String, usize> = HashMap::new();
    let mut total_classifications = 0;
    let mut line_count = 0;
    let limit = limit.unwrap_or(usize::MAX);
    
    let mut found_patterns = Vec::new();
    
    for line in content.lines() {
        line_count += 1;
        
        if line_count > limit { break; }
        if sampling_rate > 1 && line_count % sampling_rate != 0 { continue; }
        
        // Extract patterns from the line
        if !excluded_categories.contains(&"email".to_string()) {
            for email in extraction::email::extract_emails(line) {
                found_patterns.push(json!({
                    "category": "email",
                    "value": email
                }));
                *category_counts.entry("email".to_string()).or_insert(0) += 1;
                total_classifications += 1;
            }
        }
        
        if !excluded_categories.contains(&"ip".to_string()) {
            for ip in extraction::ip::extract_ips(line) {
                found_patterns.push(json!({
                    "category": "ip",
                    "value": ip
                }));
                *category_counts.entry("ip".to_string()).or_insert(0) += 1;
                total_classifications += 1;
            }
        }
        
        if !excluded_categories.contains(&"url".to_string()) {
            for url in extraction::url::extract_urls(line) {
                found_patterns.push(json!({
                    "category": "url",
                    "value": url
                }));
                *category_counts.entry("url".to_string()).or_insert(0) += 1;
                total_classifications += 1;
            }
        }
        
        if !excluded_categories.contains(&"dns_name".to_string()) {
            for dns in extraction::dnsname::extract_dnsnames(line) {
                found_patterns.push(json!({
                    "category": "dns_name",
                    "value": dns
                }));
                *category_counts.entry("dns_name".to_string()).or_insert(0) += 1;
                total_classifications += 1;
            }
        }
    }
    
    // Build categories from counts
    let mut categories = Vec::new();
    for (category, count) in &category_counts {
        categories.push(json!({
            "category": category,
            "count": count,
            "percentage": ((*count as f64) / (total_classifications as f64) * 100.0).round()
        }));
    }
    
    // Sort categories by count (descending)
    categories.sort_by(|a, b| {
        let count_a = a["count"].as_u64().unwrap_or(0);
        let count_b = b["count"].as_u64().unwrap_or(0);
        count_b.cmp(&count_a)
    });
    
    // Include found patterns as findings in the response
    Ok(json!({
        "categories": categories,
        "findings": found_patterns,  // This is the key addition
        "summary": {
            "total_lines_processed": line_count,
            "total_classifications": total_classifications,
            "source": format!("s3://{}/{}", bucket, key)
        }
    }))
}

async fn generate_presigned_url(bucket: &str, key: &str) -> Result<String, Error> {
    let config = aws_config::load_from_env().await;
    let client = aws_sdk_s3::Client::new(&config);
    
    let presigned_request = client
        .put_object()
        .bucket(bucket)
        .key(key)
        .presigned(PresigningConfig::expires_in(
            std::time::Duration::from_secs(3600),
        )?)
        .await?;
    
    Ok(presigned_request.uri().to_string())
}

async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    // Extract the path and method
    let path = event.uri().path();
    let method = event.method().as_str();
    
    // Create MCP server instance to reuse its logic
    let server = McpServer::new();
    
    // Handle MCP protocol for Cursor integration
    if path.ends_with("/mcp") && method == "POST" {
        eprintln!("Handling MCP request");
        
        let body = match event.body() {
            Body::Text(text) => text.clone(),
            Body::Binary(bytes) => String::from_utf8_lossy(bytes).to_string(),
            _ => return Ok(Response::builder()
                .status(400)
                .body("Invalid request body".into())?)
        };
        
        eprintln!("MCP request body: {}", body);
        
        // Check if this is a direct parameter from Cursor or full JSON-RPC
        let mut request_json: Value = serde_json::from_str(&body)?;
        let id;
        
        // Handle Cursor's direct format: {"text": "content"}
        if !request_json.get("jsonrpc").is_some() && !request_json.get("method").is_some() {
            eprintln!("Detected direct format from Cursor");
            // Convert to JSON-RPC format
            id = json!(1);
            let direct_params = request_json.clone();
            request_json = json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": {
                    "name": "analyze_text", 
                    "arguments": direct_params
                }
            });
            eprintln!("Converted to JSON-RPC format: {}", request_json);
        } else {
            id = request_json["id"].clone();
        }
        
        // Special handling for file upload capability
        if request_json["method"].as_str() == Some("tools/call") {
            let tool_name = request_json["params"]["name"].as_str().unwrap_or("");
            eprintln!("Processing tool call: {}", tool_name);             
            if tool_name == "analyze_file" {
                eprintln!("Found tool: analyze_file - processing with direct implementation");
                let file_path = request_json["params"]["arguments"]["file_path"].as_str();
                
                if let Some(file_path) = file_path {
                    eprintln!("Processing file path: {}", file_path);
                    
                    // Get other parameters
                    let limit = request_json["params"]["arguments"]["limit"].as_u64().map(|x| x as usize);
                    let exclude = request_json["params"]["arguments"]["exclude"].as_str();
                    let sample = request_json["params"]["arguments"]["sample"].as_u64().map(|x| x as usize).unwrap_or(1);
                    
                    // Only allow S3 paths in Lambda
                    if !file_path.starts_with("s3://") {
                        let result = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": format!("ERROR: The file path '{}' is not accessible.\n\nOnly S3 paths (s3://bucket/key) are supported.", file_path)
                                }]
                            }
                        });
                        
                        return Ok(Response::builder()
                            .status(200)
                            .header("Content-Type", "application/json")
                            .body(result.to_string().into())?);
                    }
                    
                    // Process the S3 file directly rather than calling process_file_analysis
                    let parts: Vec<&str> = file_path.strip_prefix("s3://").unwrap().splitn(2, '/').collect();
                    if parts.len() != 2 {
                        let result = json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": format!("ERROR: Invalid S3 path format: {}\n\nExpected: s3://bucket/key", file_path)
                                }]
                            }
                        });
                        
                        return Ok(Response::builder()
                            .status(200)
                            .header("Content-Type", "application/json")
                            .body(result.to_string().into())?);
                    }
                    
                    // Process S3 file
                    match process_s3_file(parts[0], parts[1], limit, exclude, sample).await {
                        Ok(stats) => {
                            // Format a nice response for Cursor
                            let mut analysis_text = format!("File Analysis Complete: {}\n\n", file_path);
        
                            if let Some(summary) = stats.get("summary") {
                                analysis_text.push_str(&format!("Processed {} lines, found {} classifications\n\n", 
                                    summary["total_lines_processed"], 
                                    summary["total_classifications"]));
                            }
                            
                            // Add breakdown by category
                            analysis_text.push_str("PII Categories Detected:\n");
                            
                            if let Some(categories) = stats["categories"].as_array() {
                                if categories.is_empty() {
                                    analysis_text.push_str("No PII detected in file\n");
                                } else {
                                    for category in categories {
                                        if let (Some(cat_name), Some(count)) = (
                                            category["category"].as_str(),
                                            category["count"].as_u64()
                                        ) {
                                            analysis_text.push_str(&format!("• {}: {} occurrences\n", 
                                                cat_name, count));
                                        }
                                    }
                                }
                            }
                            
                            // Add detailed findings
                            if let Some(findings) = stats["findings"].as_array() {
                                if !findings.is_empty() {
                                    analysis_text.push_str("\nDetailed Findings (sample):\n");
                                    // Limit to first 25 findings to avoid overflow
                                    let max_findings = 25.min(findings.len());
                                    for i in 0..max_findings {
                                        if let (Some(category), Some(value)) = (
                                            findings[i]["category"].as_str(),
                                            findings[i]["value"].as_str()
                                        ) {
                                            analysis_text.push_str(&format!("• {}: {}\n", category, value));
                                        }
                                    }
                                    
                                    if findings.len() > max_findings {
                                        analysis_text.push_str(&format!("\n(Showing {}/{} findings)\n", max_findings, findings.len()));
                                    }
                                }
                            }
                            
                            let result = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": analysis_text
                                    }],
                                    "_meta": stats
                                }
                            });
                            
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(result.to_string().into())?);
                        },
                        Err(e) => {
                            eprintln!("Error processing S3 file: {}", e);
                            let error_response = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "content": [{
                                        "type": "text",
                                        "text": format!("ERROR: Failed to process S3 file: {}\n\nPlease check that the file exists and is accessible.", e)
                                    }]
                                }
                            });
                            
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(error_response.to_string().into())?);
                        }
                    }
                } 
            } else if tool_name == "analyze_content" || tool_name == "analyze_text" {
                eprintln!("Found tool: {} - processing with direct implementation", tool_name);
                let content_param = if tool_name == "analyze_content" { "content" } else { "text" };
                
                if let Some(content) = request_json["params"]["arguments"][content_param].as_str() {
                    // Collect patterns directly using extraction functions
                    let mut found_patterns = Vec::new();
                    
                    // Email patterns
                    for email in extraction::email::extract_emails(content) {
                        found_patterns.push(json!({
                            "category": "email",
                            "value": email
                        }));
                    }
                    
                    // IP patterns
                    for ip in extraction::ip::extract_ips(content) {
                        found_patterns.push(json!({
                            "category": "ip",
                            "value": ip
                        }));
                    }
                    
                    // URL patterns
                    for url in extraction::url::extract_urls(content) {
                        found_patterns.push(json!({
                            "category": "url",
                            "value": url
                        }));
                    }
                    
                    // DNS names
                    for dns in extraction::dnsname::extract_dnsnames(content) {
                        found_patterns.push(json!({
                            "category": "dns_name",
                            "value": dns
                        }));
                    }
                    
                    // Build response
                    let mut analysis_text = format!("Pattern Analysis Results:\n\n");

                    // Add breakdown by category
                    let email_count = found_patterns.iter().filter(|p| p["category"].as_str() == Some("email")).count();
                    let ip_count = found_patterns.iter().filter(|p| p["category"].as_str() == Some("ip")).count();
                    let dns_count = found_patterns.iter().filter(|p| p["category"].as_str() == Some("dns_name")).count();
                    let url_count = found_patterns.iter().filter(|p| p["category"].as_str() == Some("url")).count();
                    
                    analysis_text.push_str(&format!("Found {} patterns:\n", found_patterns.len()));
                    if email_count > 0 { analysis_text.push_str(&format!("• {} email addresses\n", email_count)); }
                    if ip_count > 0 { analysis_text.push_str(&format!("• {} IP addresses\n", ip_count)); }
                    if dns_count > 0 { analysis_text.push_str(&format!("• {} DNS names\n", dns_count)); }
                    if url_count > 0 { analysis_text.push_str(&format!("• {} URLs\n", url_count)); }
                    
                    // Add the actual found patterns
                    if !found_patterns.is_empty() {
                        analysis_text.push_str("\nDetailed findings:\n");
                        for pattern in &found_patterns {
                            if let (Some(category), Some(value)) = (pattern["category"].as_str(), pattern["value"].as_str()) {
                                analysis_text.push_str(&format!("• {}: {}\n", category, value));
                            }
                        }
                    }
                    
                    // Build response with content array for Cursor
                    let result = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "content": [{
                                "type": "text",
                                "text": analysis_text
                            }],
                            "_meta": {
                                "categories": found_patterns,
                                "summary": {
                                    "total_patterns": found_patterns.len()
                                }
                            }
                        }
                    });
                    
                    eprintln!("Returning direct tool response: {}", result);
                    return Ok(Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(result.to_string().into())?);
                }
            }
        }
        
        // Regular MCP protocol handling
        match server.handle_request(request_json).await {
            Ok(mut response) => {
                eprintln!("MCP server returned response: {}", response);
                // Ensure proper JSON-RPC 2.0 format
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("jsonrpc".to_string(), json!("2.0"));
                    if !id.is_null() {
                        obj.insert("id".to_string(), id);
                    }
                }
                let response_body = response.to_string();
                eprintln!("Final formatted response: {}", response_body);
                
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(response_body.into())?)
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
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(error_response.to_string().into())?)
            }
        }
    }
    // Handle REST API endpoints for direct file analysis
    else if path.ends_with("/analyze") && method == "POST" {
        // Check content type to determine how to handle the upload
        let content_type = event.headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        
        // Handle JSON request
        if content_type.contains("application/json") {
            let body = match event.body() {
                Body::Text(text) => text.clone(),
                Body::Binary(bytes) => String::from_utf8_lossy(bytes).to_string(),
                _ => return Ok(Response::builder()
                    .status(400)
                    .body("Invalid request body".into())?)
            };
            
            let request: Value = serde_json::from_str(&body)?;
            
            // Option 1: Analyze direct text content
            if let Some(content) = request["content"].as_str() {
                // Collect patterns directly
                let mut found_patterns = Vec::new();
                
                // Email patterns
                for email in extraction::email::extract_emails(content) {
                    found_patterns.push(json!({
                        "category": "email",
                        "value": email
                    }));
                }
                
                // IP patterns
                for ip in extraction::ip::extract_ips(content) {
                    found_patterns.push(json!({
                        "category": "ip",
                        "value": ip
                    }));
                }
                
                // URL patterns
                for url in extraction::url::extract_urls(content) {
                    found_patterns.push(json!({
                        "category": "url",
                        "value": url
                    }));
                }
                
                // DNS names
                for dns in extraction::dnsname::extract_dnsnames(content) {
                    found_patterns.push(json!({
                        "category": "dns_name",
                        "value": dns
                    }));
                }
                
                return Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(json!({
                        "categories": found_patterns,
                        "summary": {
                            "total_patterns": found_patterns.len()
                        }
                    }).to_string().into())?);
            }

            // Option 2: Analyze file in S3
            else if let Some(s3_path) = request["file_path"].as_str() {
                if !s3_path.starts_with("s3://") {
                    return Ok(Response::builder()
                        .status(400)
                        .body(json!({
                            "error": "Invalid S3 path format. Expected s3://bucket/key"
                        }).to_string().into())?);
                }
                
                let parts: Vec<&str> = s3_path.strip_prefix("s3://")
                    .unwrap_or(s3_path)
                    .splitn(2, '/')
                    .collect();
                
                if parts.len() == 2 {
                    let bucket = parts[0];
                    let key = parts[1];
                    let limit = request["limit"].as_u64().map(|l| l as usize);
                    let exclude = request["exclude"].as_str();
                    let sampling_rate = request["sampling_rate"].as_u64().unwrap_or(1) as usize;
                    
                    match process_s3_file(bucket, key, limit, exclude, sampling_rate).await {
                        Ok(analysis) => {
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(analysis.to_string().into())?);
                        },
                        Err(e) => {
                            return Ok(Response::builder()
                                .status(500)
                                .header("Content-Type", "application/json")
                                .body(json!({
                                    "error": format!("Failed to process S3 file: {}", e)
                                }).to_string().into())?);
                        }
                    }
                } else {
                    return Ok(Response::builder()
                        .status(400)
                        .body(json!({
                            "error": "Invalid S3 path format. Expected s3://bucket/key"
                        }).to_string().into())?);
                }
            }
            
            // Option 3: Get presigned URL for upload
            else if let Some(true) = request["get_upload_url"].as_bool() {
                let file_name = request["file_name"]
                    .as_str()
                    .unwrap_or("unnamed-file.txt");
                
                let bucket = std::env::var("S3_BUCKET")
                    .unwrap_or_else(|_| "regex-classifier-uploads".to_string());
                    
                let key = format!("uploads/{}-{}", Uuid::new_v4(), file_name);
                
                match generate_presigned_url(&bucket, &key).await {
                    Ok(url) => {
                        return Ok(Response::builder()
                            .status(200)
                            .header("Content-Type", "application/json")
                            .body(json!({
                                "upload_url": url,
                                "file_path": format!("s3://{}/{}", bucket, key)
                            }).to_string().into())?);
                    },
                    Err(e) => {
                        return Ok(Response::builder()
                            .status(500)
                            .header("Content-Type", "application/json")
                            .body(json!({
                                "error": format!("Failed to generate presigned URL: {}", e)
                            }).to_string().into())?);
                    }
                }
            }
            
            return Ok(Response::builder()
                .status(400)
                .body(json!({
                    "error": "Missing required parameters. Provide either 'content', 'file_path', or 'get_upload_url'"
                }).to_string().into())?);
        }
        // Handle direct file upload (binary data)
        else {
            // Setup S3 client
            let config = aws_config::load_from_env().await;
            let s3_client = aws_sdk_s3::Client::new(&config);
            let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "regex-classifier-uploads".to_string());
            
            // Generate a unique filename
            let filename = event.headers()
                .get("x-filename")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("uploaded-file.txt");
                
            let key = format!("direct-uploads/{}-{}", Uuid::new_v4(), filename);
            
            // Extract file content
            let file_content = match event.body() {
                Body::Text(text) => text.as_bytes().to_vec(),
                Body::Binary(bytes) => bytes.to_vec(),
                _ => return Ok(Response::builder()
                    .status(400)
                    .body("Invalid request body".into())?)
            };
            
            // Upload to S3
            match s3_client.put_object()
                .bucket(bucket.clone())
                .key(key.clone())
                .body(file_content.into())
                .send()
                .await {
                    Ok(_) => {
                        // Analyze the uploaded content
                        let s3_path = format!("s3://{}/{}", bucket, key);
                        let parts: Vec<&str> = s3_path.strip_prefix("s3://")
                            .unwrap_or(&s3_path)
                            .splitn(2, '/')
                            .collect();
                        
                        if parts.len() == 2 {
                            let bucket = parts[0];
                            let key = parts[1];
                            
                            match process_s3_file(bucket, key, None, None, 1).await {
                                Ok(analysis) => {
                                    return Ok(Response::builder()
                                        .status(200)
                                        .header("Content-Type", "application/json")
                                        .body(json!({
                                            "status": "success",
                                            "message": "File uploaded and analyzed",
                                            "file_path": s3_path,
                                            "analysis": analysis
                                        }).to_string().into())?);
                                },
                                Err(e) => {
                                    return Ok(Response::builder()
                                        .status(500)
                                        .header("Content-Type", "application/json")
                                        .body(json!({
                                            "status": "error",
                                            "message": format!("File uploaded but analysis failed: {}", e),
                                            "file_path": s3_path
                                        }).to_string().into())?);
                                }
                            }
                        } else {
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(json!({
                                    "status": "success",
                                    "message": "File uploaded successfully",
                                    "file_path": s3_path
                                }).to_string().into())?);
                        }
                    },
                    Err(e) => {
                        return Ok(Response::builder()
                            .status(500)
                            .header("Content-Type", "application/json")
                            .body(json!({
                                "status": "error",
                                "message": format!("Failed to upload file: {}", e)
                            }).to_string().into())?);
                    }
                }
        }
    }
    // Health check endpoint
    else if path.ends_with("/health") {
        Ok(Response::builder()
            .status(200)
            .header("Content-Type", "application/json")
            .body(json!({
                "status": "ok",
                "service": "regex-classifier"
            }).to_string().into())?)
    }
    // Handle unknown paths
    else {
        Ok(Response::builder()
            .status(404)
            .header("Content-Type", "application/json")
            .body(json!({
                "status": "error",
                "message": "Not found"
            }).to_string().into())?)
    }
}

async fn process_file_analysis(
    file_path: &str,
    limit: Option<usize>,
    exclude: Option<&str>,
    sampling_rate: usize
) -> Result<Value, Error> {
    eprintln!("Processing file analysis for: {}", file_path);
    // If it's an S3 path
    if file_path.starts_with("s3://") {
        eprintln!("Detected S3 path");
        let parts: Vec<&str> = file_path.strip_prefix("s3://")
            .unwrap_or(file_path)
            .splitn(2, '/')
            .collect();
        
        if parts.len() == 2 {
            eprintln!("Processing S3 file: bucket={}, key={}", parts[0], parts[1]);
            return process_s3_file(parts[0], parts[1], limit, exclude, sampling_rate).await;
        }
    }
    
    // Otherwise, call McpServer's implementation
    eprintln!("Using local file processing for: {}", file_path);
    let server = McpServer::new();
    let categories = None; // Not used in the original
    server.process_file(file_path, limit, exclude, categories, sampling_rate)
        .map_err(|e| e.into())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize AWS SDK
    aws_config::load_from_env().await;
    
    // Run the service
    run(service_fn(function_handler)).await
}