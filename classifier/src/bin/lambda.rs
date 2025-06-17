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
        
        // Add other pattern types as needed
    }
    
    // Prepare statistics
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
    
    Ok(json!({
        "categories": found_patterns,
        "summary": {
            "total_lines_processed": line_count,
            "total_classifications": total_classifications,
            "source": format!("s3://{}/{}", bucket, key)
        },
        "statistics": category_stats
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
        let body = match event.body() {
            Body::Text(text) => text.clone(),
            Body::Binary(bytes) => String::from_utf8_lossy(bytes).to_string(),
            _ => return Ok(Response::builder()
                .status(400)
                .body("Invalid request body".into())?)
        };
        
        let request_json: Value = serde_json::from_str(&body)?;
        let id = request_json["id"].clone();
        
        // Special handling for file upload capability
        if request_json["method"].as_str() == Some("tools/call") {
            let tool_name = request_json["params"]["name"].as_str().unwrap_or("");
            
            if tool_name == "get_upload_url" {
                if let Some(file_name) = request_json["params"]["arguments"]["file_name"].as_str() {
                    let bucket = std::env::var("S3_BUCKET").unwrap_or_else(|_| "regex-classifier-uploads".to_string());
                    let key = format!("uploads/{}-{}", Uuid::new_v4(), file_name);
                    
                    match generate_presigned_url(&bucket, &key).await {
                        Ok(url) => {
                            let result = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "result": {
                                    "upload_url": url,
                                    "file_path": format!("s3://{}/{}", bucket, key)
                                }
                            });
                            
                            return Ok(Response::builder()
                                .status(200)
                                .header("Content-Type", "application/json")
                                .body(result.to_string().into())?);
                        },
                        Err(e) => {
                            let error = json!({
                                "jsonrpc": "2.0",
                                "id": id,
                                "error": {
                                    "code": -32000,
                                    "message": format!("Failed to generate presigned URL: {}", e)
                                }
                            });
                            
                            return Ok(Response::builder()
                                .status(500)
                                .header("Content-Type", "application/json")
                                .body(error.to_string().into())?);
                        }
                    }
                }
            } else if tool_name == "analyze_content" || tool_name == "analyze_text" {
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
                    let result = json!({
                        "jsonrpc": "2.0",
                        "id": id,
                        "result": {
                            "categories": found_patterns,
                            "summary": {
                                "total_patterns": found_patterns.len()
                            }
                        }
                    });
                    
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
                // Ensure proper JSON-RPC 2.0 format
                if let Some(obj) = response.as_object_mut() {
                    obj.insert("jsonrpc".to_string(), json!("2.0"));
                    if !id.is_null() {
                        obj.insert("id".to_string(), id);
                    }
                }
                Ok(Response::builder()
                    .status(200)
                    .header("Content-Type", "application/json")
                    .body(response.to_string().into())?)
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

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize AWS SDK
    aws_config::load_from_env().await;
    
    // Run the service
    run(service_fn(function_handler)).await
}