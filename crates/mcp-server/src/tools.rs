//! Tool dispatch: forwards MCP tool calls to the Access REST API server.
//!
//! This keeps MCP and the post-it widget on the same data: both talk to
//! the api-server at http://127.0.0.1:7878 (which syncs to Gist).

use serde_json::Value;

const API_BASE: &str = "http://127.0.0.1:7878";

/// A single tool invocation: name + arguments object.
pub struct ToolCall {
    pub name: String,
    pub arguments: Value,
}

fn agent_of(args: &Value) -> String {
    args.get("agent")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string()
}

fn id_of(args: &Value) -> anyhow::Result<String> {
    args.get("id")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow::anyhow!("id required"))
}

/// Execute a tool call by forwarding it to the REST API.
pub async fn dispatch(_store: (), call: ToolCall) -> anyhow::Result<Value> {
    let agent = agent_of(&call.arguments);
    let client = reqwest::Client::new();

    Ok(match call.name.as_str() {
        "list_todos" => {
            // agent 인수가 있으면 ?agent= 필터로 해당 에이전트의 할 일만 조회
            let url = if !agent.is_empty() && agent != "unknown" {
                format!("{API_BASE}/todos?agent={agent}")
            } else {
                format!("{API_BASE}/todos")
            };
            let r = client
                .get(&url)
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "get_today_todos" => {
            let r = client
                .get(format!("{API_BASE}/todos/today"))
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "get_todo" => {
            let id = id_of(&call.arguments)?;
            let r = client
                .get(format!("{API_BASE}/todos/{id}"))
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "add_todo" => {
            let title = call.arguments["title"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("title required"))?;
            let mut body = serde_json::json!({
                "title": title,
                "priority": call.arguments.get("priority").and_then(|v| v.as_str()).unwrap_or("medium"),
            });
            if let Some(note) = call.arguments.get("note").and_then(|v| v.as_str()) {
                body["note"] = Value::String(note.into());
            }
            if let Some(dd) = call.arguments.get("due_date").and_then(|v| v.as_str()) {
                body["due_date"] = Value::String(dd.into());
            }
            if let Some(tags) = call.arguments.get("tags") {
                body["tags"] = tags.clone();
            }
            if let Some(cat_id) = call.arguments.get("category_id").and_then(|v| v.as_str()) {
                body["category_id"] = Value::String(cat_id.into());
            }
            let r = client
                .post(format!("{API_BASE}/todos"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "update_todo" => {
            let id = id_of(&call.arguments)?;
            let r = client
                .patch(format!("{API_BASE}/todos/{id}"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&call.arguments)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "toggle_todo" => {
            let id = id_of(&call.arguments)?;
            let r = client
                .post(format!("{API_BASE}/todos/{id}/toggle"))
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "complete_todo" => {
            // 작업 완료 + 요약 기록. summary 인수로 무엇을 했는지 전달.
            let id = id_of(&call.arguments)?;
            let summary = call.arguments["summary"]
                .as_str()
                .unwrap_or("작업 완료");
            let body = serde_json::json!({ "summary": summary });
            let r = client
                .post(format!("{API_BASE}/todos/{id}/complete"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "delete_todo" => {
            let id = id_of(&call.arguments)?;
            let r = client
                .delete(format!("{API_BASE}/todos/{id}"))
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "search_todos" => {
            let q = call.arguments["q"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("q required"))?;
            let r = client
                .get(format!("{API_BASE}/todos/search"))
                .query(&[("q", q)])
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "add_todos_batch" => {
            // 여러 할 일을 한 번에 등록 (온보딩용).
            let batch_agent = call.arguments["agent"]
                .as_str()
                .unwrap_or(&agent)
                .to_string();
            let todos = call.arguments["todos"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("todos array required"))?;
            let body = serde_json::json!({ "agent": batch_agent, "todos": todos });
            let r = client
                .post(format!("{API_BASE}/batch/todos"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "create_category" => {
            let cat_agent = call.arguments["agent"]
                .as_str()
                .unwrap_or(&agent)
                .to_string();
            let name = call.arguments["name"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("name required"))?;
            let body = serde_json::json!({ "agent": cat_agent, "name": name });
            let r = client
                .post(format!("{API_BASE}/categories"))
                .header("X-Agent", &agent)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await?
                .json::<Value>().await?;
            r
        }
        "access_review" => {
            // 세션 점검 + 진행상황 종합 요약
            let review_agent = call.arguments["agent"]
                .as_str()
                .unwrap_or(&agent)
                .to_string();
            let r = client
                .get(format!("{API_BASE}/review"))
                .query(&[("agent", review_agent.as_str())])
                .header("X-Agent", &agent)
                .send().await?
                .json::<Value>().await?;
            r
        }
        other => anyhow::bail!("unknown tool: {other}"),
    })
}

/// The tool catalog advertised in `tools/list`.
pub fn tool_catalog() -> Vec<Value> {
    vec![
        serde_json::json!({
            "name": "list_todos",
            "description": "Access 포스트잇의 모든 할 일을 조회합니다.",
            "inputSchema": { "type": "object", "properties": { "agent": { "type": "string", "description": "호출하는 에이전트 이름" } } }
        }),
        serde_json::json!({
            "name": "get_today_todos",
            "description": "오늘 마감인 미완료 할 일을 조회합니다.",
            "inputSchema": { "type": "object", "properties": { "agent": { "type": "string" } } }
        }),
        serde_json::json!({
            "name": "get_todo",
            "description": "단일 할 일을 id로 조회합니다.",
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
            "description": "Access 포스트잇에 새 할 일을 추가합니다. agent 필드에 에이전트 이름을 넣으면 추적됩니다. tags에 agent:<이름>을 넣으면 포스트잇에서 해당 그룹으로 분류됩니다.",
            "inputSchema": {
                "type": "object",
                "required": ["title"],
                "properties": {
                    "title": { "type": "string", "description": "할 일 제목" },
                    "note": { "type": "string", "description": "메모 (선택)" },
                    "priority": { "type": "string", "enum": ["high", "medium", "low"], "description": "우선순위" },
                    "due_date": { "type": "string", "description": "마감일 YYYY-MM-DD (선택)" },
                    "tags": { "type": "array", "items": { "type": "string" }, "description": "태그. 예: [\"agent:zcode\"]" },
                    "category_id": { "type": "string", "description": "카테고리 ID (선택). list_todos 응답의 categories에서 확인" },
                    "agent": { "type": "string", "description": "에이전트 이름 (예: zcode, hermes, claude)" }
                }
            }
        }),
        serde_json::json!({
            "name": "update_todo",
            "description": "기존 할 일을 수정합니다.",
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
            "description": "할 일의 완료 상태를 토글합니다. 누가 체크했는지 completed_by에 기록됩니다.",
            "inputSchema": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" },
                    "agent": { "type": "string", "description": "체크한 에이전트 이름" }
                }
            }
        }),
        serde_json::json!({
            "name": "complete_todo",
            "description": "작업을 완료로 표시하고 summary에 작업 내용 요약을 기록합니다. 코드/파일 수정을 마친 후 호출하세요. summary에는 무엇을 변경했는지 간결히 작성하세요 (예: 'job_select.lua 51줄 nil 체크 추가').",
            "inputSchema": {
                "type": "object",
                "required": ["id", "summary"],
                "properties": {
                    "id": { "type": "string", "description": "완료할 할 일 id" },
                    "summary": { "type": "string", "description": "작업 요약. 무엇을 했는지, 어떤 파일을 변경했는지" },
                    "agent": { "type": "string", "description": "에이전트 이름" }
                }
            }
        }),
        serde_json::json!({
            "name": "delete_todo",
            "description": "할 일을 삭제합니다.",
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
            "description": "제목, 메모, 태그에서 키워드로 검색합니다 (대소문자 무관).",
            "inputSchema": {
                "type": "object",
                "required": ["q"],
                "properties": {
                    "q": { "type": "string", "description": "검색어" },
                    "agent": { "type": "string" }
                }
            }
        }),
        serde_json::json!({
            "name": "add_todos_batch",
            "description": "여러 할 일을 한 번에 등록합니다. 새 프로젝트 온보딩, .access/rules.json에서 읽은 할 일 일괄 등록에 사용.",
            "inputSchema": {
                "type": "object",
                "required": ["todos"],
                "properties": {
                    "agent": { "type": "string", "description": "등록할 에이전트 이름" },
                    "todos": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "required": ["title"],
                            "properties": {
                                "title": { "type": "string" },
                                "note": { "type": "string" },
                                "priority": { "type": "string", "enum": ["high", "medium", "low"] },
                                "category_id": { "type": "string" },
                                "tags": { "type": "array", "items": { "type": "string" } }
                            }
                        }
                    }
                }
            }
        }),
        serde_json::json!({
            "name": "create_category",
            "description": "에이전트에 새 카테고리를 생성합니다. 온보딩 시 프로젝트 구조에 맞춰 카테고리를 미리 만들 때 사용.",
            "inputSchema": {
                "type": "object",
                "required": ["name"],
                "properties": {
                    "name": { "type": "string", "description": "카테고리 이름" },
                    "agent": { "type": "string", "description": "에이전트 이름" }
                }
            }
        }),
        serde_json::json!({
            "name": "access_review",
            "description": "에이전트의 전체 진행상황을 종합 요약합니다. 세션 시작/종료 시 호출하여 어디까지 작업했고 다음에 뭘 해야 하는지 파악. 진행률, 완료된 작업(요약포함), 우선순위별 남은 작업, 긴급 다음 작업을 반환.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "agent": { "type": "string", "description": "점검할 에이전트 이름" }
                }
            }
        }),
    ]
}
