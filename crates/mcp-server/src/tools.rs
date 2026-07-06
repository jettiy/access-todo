//! Tool dispatch: a pure async function over a shared `Store`.
//!
//! Each MCP `tools/call` is routed here. The `agent` argument is read
//! from the call's `arguments` (or defaults to `"unknown"`) and recorded
//! in the resulting todo's audit fields, matching the REST API behavior.

use std::sync::Arc;

use serde_json::Value;
use tokio::sync::Mutex;

use todo_core::model::Priority;
use todo_core::store::{Store, TodoInput, TodoPatch};

/// A single tool invocation: name + arguments object.
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

fn prio(s: &str) -> Priority {
    match s {
        "high" => Priority::High,
        "low" => Priority::Low,
        _ => Priority::Medium,
    }
}

fn agent_of(args: &Value) -> String {
    args.get("agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn id_of(args: &Value) -> anyhow::Result<&str> {
    args.get("id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("id required"))
}

/// Execute a tool call against the given shared store. Returns a JSON value
/// suitable for embedding in an MCP `tools/call` result.
pub async fn dispatch(store: Arc<Mutex<Store>>, call: ToolCall) -> anyhow::Result<Value> {
    let agent = agent_of(&call.arguments);
    let mut s = store.lock().await;
    Ok(match call.name.as_str() {
        "list_todos" => serde_json::json!({ "todos": s.list() }),
        "get_today_todos" => serde_json::json!({ "todos": s.list_today() }),
        "get_todo" => {
            let id = id_of(&call.arguments)?;
            serde_json::json!(s.get(id))
        }
        "add_todo" => {
            let title = call.arguments["title"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("title required"))?
                .to_string();
            let t = s.add(
                TodoInput {
                    title,
                    note: call
                        .arguments
                        .get("note")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    priority: call
                        .arguments
                        .get("priority")
                        .and_then(|v| v.as_str())
                        .map(prio)
                        .unwrap_or_default(),
                    due_date: call
                        .arguments
                        .get("due_date")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    tags: call
                        .arguments
                        .get("tags")
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|x| x.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default(),
                },
                &agent,
            );
            serde_json::json!(t)
        }
        "update_todo" => {
            let id = id_of(&call.arguments)?;
            let patch = TodoPatch {
                title: call
                    .arguments
                    .get("title")
                    .and_then(|v| v.as_str())
                    .map(String::from),
                note: call
                    .arguments
                    .get("note")
                    .map(|v| v.as_str().map(String::from)),
                priority: call
                    .arguments
                    .get("priority")
                    .and_then(|v| v.as_str())
                    .map(prio),
                due_date: call
                    .arguments
                    .get("due_date")
                    .map(|v| v.as_str().map(String::from)),
                tags: call
                    .arguments
                    .get("tags")
                    .and_then(|v| v.as_array())
                    .map(|a| {
                        a.iter()
                            .filter_map(|x| x.as_str().map(String::from))
                            .collect()
                    }),
            };
            serde_json::json!(s.update(id, patch, &agent)?)
        }
        "toggle_todo" => {
            let id = id_of(&call.arguments)?;
            serde_json::json!(s.toggle(id, &agent)?)
        }
        "delete_todo" => {
            let id = id_of(&call.arguments)?;
            s.delete(id, &agent)?;
            serde_json::json!({ "deleted": id })
        }
        "search_todos" => {
            let q = call.arguments["q"].as_str().unwrap_or("");
            serde_json::json!({ "todos": s.search(q) })
        }
        other => anyhow::bail!("unknown tool: {other}"),
    })
}

/// The tool catalog advertised in `tools/list`. Built as a `Vec` because
/// `serde_json::json!` requires allocation (cannot live in a `const`).
pub fn tool_catalog() -> Vec<Value> {
    vec![
    serde_json::json!({
        "name": "list_todos",
        "description": "List all todos.",
        "inputSchema": { "type": "object", "properties": { "agent": { "type": "string" } } }
    }),
    serde_json::json!({
        "name": "get_today_todos",
        "description": "List today's incomplete todos.",
        "inputSchema": { "type": "object", "properties": { "agent": { "type": "string" } } }
    }),
    serde_json::json!({
        "name": "get_todo",
        "description": "Get a single todo by id.",
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "agent": { "type": "string" }
            }
        }
    }),
    serde_json::json!({
        "name": "add_todo",
        "description": "Add a todo. `agent` is recorded as the creator.",
        "inputSchema": {
            "type": "object",
            "required": ["title"],
            "properties": {
                "title": { "type": "string" },
                "note": { "type": "string" },
                "priority": { "type": "string", "enum": ["high", "medium", "low"] },
                "due_date": { "type": "string", "description": "YYYY-MM-DD" },
                "tags": { "type": "array", "items": { "type": "string" } },
                "agent": { "type": "string" }
            }
        }
    }),
    serde_json::json!({
        "name": "update_todo",
        "description": "Update a todo. `note`/`due_date` accept null to clear.",
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "title": { "type": "string" },
                "note": { "type": ["string", "null"] },
                "priority": { "type": "string", "enum": ["high", "medium", "low"] },
                "due_date": { "type": ["string", "null"] },
                "tags": { "type": "array", "items": { "type": "string" } },
                "agent": { "type": "string" }
            }
        }
    }),
    serde_json::json!({
        "name": "toggle_todo",
        "description": "Toggle the done state of a todo. Records who checked/unchecked it.",
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "agent": { "type": "string" }
            }
        }
    }),
    serde_json::json!({
        "name": "delete_todo",
        "description": "Delete a todo by id.",
        "inputSchema": {
            "type": "object",
            "required": ["id"],
            "properties": {
                "id": { "type": "string" },
                "agent": { "type": "string" }
            }
        }
    }),
    serde_json::json!({
        "name": "search_todos",
        "description": "Search todos by title, note, or tag (case-insensitive).",
        "inputSchema": {
            "type": "object",
            "required": ["q"],
            "properties": {
                "q": { "type": "string" },
                "agent": { "type": "string" }
            }
        }
    }),
    ]
}
