//! MCP stdio server: reads line-delimited JSON-RPC 2.0 from stdin, writes
//! responses to stdout. Supports `initialize`, `tools/list`, `tools/call`.

use std::sync::Arc;

use mcp_server::tools::{dispatch, tool_catalog, ToolCall};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::Mutex;

use todo_core::store::Store;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let store = Arc::new(Mutex::new(Store::new()));
    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        if reader.read_line(&mut line).await? == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let req: serde_json::Value = match serde_json::from_str(trimmed) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let id = req.get("id").cloned();
        let method = req.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let result = match method {
            "initialize" => serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "desktop-todo-agents", "version": env!("CARGO_PKG_VERSION") }
            }),
            "tools/list" => serde_json::json!({ "tools": tool_catalog() }),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("").to_string();
                let arguments = req
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_default();
                match dispatch(store.clone(), ToolCall { name, arguments }).await {
                    Ok(v) => serde_json::json!({
                        "content": [ { "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() } ]
                    }),
                    Err(e) => serde_json::json!({
                        "isError": true,
                        "content": [ { "type": "text", "text": e.to_string() } ]
                    }),
                }
            }
            _ => {
                let _ = id;
                continue;
            }
        };
        let resp = serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result });
        stdout.write_all(resp.to_string().as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }
    Ok(())
}
