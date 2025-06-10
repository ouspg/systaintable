use tokio;
use serde_json::{json, Value};
use std::convert::Infallible;
use warp::{Filter, Reply};
use regex_classifier::mcp_server::McpServer;

async fn handle_mcp_request(
    request: Value,
    server: McpServer,
) -> Result<impl Reply, Infallible> {
    let id = request["id"].clone();
    
    match server.handle_request(request).await {
        Ok(mut response) => {
            // Ensure proper JSON-RPC 2.0 format
            if let Some(obj) = response.as_object_mut() {
                obj.insert("jsonrpc".to_string(), json!("2.0"));
                if !id.is_null() {
                    obj.insert("id".to_string(), id);
                }
            }
            Ok(warp::reply::json(&response))
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
            Ok(warp::reply::json(&error_response))
        }
    }
}
#[tokio::main]
async fn main() {
    let server = McpServer::new();
    
    let cors = warp::cors()
        .allow_any_origin()
        .allow_headers(vec!["content-type"])
        .allow_methods(vec!["POST", "OPTIONS"]);
    
    let mcp_route = warp::path("mcp")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(move |request: Value| {
            let server = server.clone();
            async move {
                handle_mcp_request(request, server).await
            }
        });
    
    let health_route = warp::path("health")
        .and(warp::get())
        .map(|| {
            warp::reply::json(&json!({
                "status": "ok",
                "service": "regex-classifier-mcp"
            }))
        });
    
    let routes = mcp_route
        .or(health_route)
        .with(cors);
    
println!("MCP HTTP server starting on http://localhost:8080");
println!("Health check: http://localhost:8080/health");
println!("MCP endpoint: http://localhost:8080/mcp");
        
warp::serve(routes)
    .run(([127, 0, 0, 1], 8080))
    .await;
}