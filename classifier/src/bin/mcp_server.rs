use tokio;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use regex_classifier::mcp_server::McpServer;

#[tokio::main]
async fn main() -> io::Result<()> {
    let server = McpServer::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = line?;
        
        match serde_json::from_str::<Value>(&line) {
            Ok(request) => {
                let id = request["id"].clone();
                
                let response = match server.handle_request(request).await {
                    Ok(mut resp) => {
                        // Add the id to the response if it exists
                        if !id.is_null() {
                            if let Some(obj) = resp.as_object_mut() {
                                obj.insert("id".to_string(), id);
                            }
                        }
                        resp
                    },
                    Err(err) => {
                        json!({
                            "id": id,
                            "error": {
                                "code": -1,
                                "message": err
                            }
                        })
                    }
                };
                
                writeln!(stdout, "{}", serde_json::to_string(&response).unwrap())?;
                stdout.flush()?;
            }
            Err(_) => {
                let error_response = json!({
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                });
                writeln!(stdout, "{}", serde_json::to_string(&error_response).unwrap())?;
                stdout.flush()?;
            }
        }
    }

    Ok(())
}