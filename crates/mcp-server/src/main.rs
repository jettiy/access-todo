//! MCP stdio server: reads line-delimited JSON-RPC 2.0 from stdin, writes
//! responses to stdout. Forwards all tool calls to the Access REST API.

use mcp_server::tools::{dispatch, tool_catalog, ToolCall};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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

        // JSON-RPC notifications (no id) must never receive a response.
        let Some(id) = id else { continue };

        let resp = match method {
            "initialize" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "access-todo", "version": env!("CARGO_PKG_VERSION") }
                }
            }),
            "ping" => serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "tools/list" => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": { "tools": tool_catalog() }
            }),
            "tools/call" => {
                let name = req["params"]["name"].as_str().unwrap_or("").to_string();
                let arguments = req
                    .get("params")
                    .and_then(|p| p.get("arguments"))
                    .cloned()
                    .unwrap_or_default();
                let result = match dispatch((), ToolCall { name, arguments }).await {
                    Ok(v) => serde_json::json!({
                        "content": [ { "type": "text", "text": serde_json::to_string_pretty(&v).unwrap_or_default() } ]
                    }),
                    Err(e) => serde_json::json!({
                        "isError": true,
                        "content": [ { "type": "text", "text": e.to_string() } ]
                    }),
                };
                serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result })
            }
            // Every other request gets a JSON-RPC error. In particular the
            // modern-era `server/discover` probe (MCP 2026-07-28) is answered
            // with MethodNotFound, so clients that probe versions fall back to
            // the legacy `initialize` handshake immediately instead of waiting
            // out their probe timeout before connecting.
            _ => serde_json::json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("Method not found: {method}") }
            }),
        };
        stdout.write_all(resp.to_string().as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }
    Ok(())
}
