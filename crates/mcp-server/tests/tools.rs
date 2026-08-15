//! Integration tests for MCP tool dispatch.
//!
//! Spins up a mock HTTP server (wiremock) that imitates the Access REST
//! API, points `dispatch` at it via `ACCESS_API_BASE`, and verifies each
//! tool call produces the correct HTTP request and propagates the response.

use mcp_server::tools::{dispatch, ToolCall};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// env-var 기반 라우팅(ACCESS_API_BASE)은 프로세스 전역이므로
/// 이 파일의 통합 테스트들을 직렬화한다 (병렬 실행 시 mock 서버 레이스).
static ENV_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

async fn with_env_lock<F, Fut>(f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let _guard = ENV_LOCK.lock().await;
    f().await;
}

#[tokio::test]
async fn add_todo_via_dispatch() {
    with_env_lock(|| async {
        let server = MockServer::start().await;
        std::env::set_var("ACCESS_API_BASE", &server.uri());

        Mock::given(method("POST"))
            .and(path("/todos"))
            .and(header("X-Agent", "zcode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "abc-123",
                "title": "테스트",
                "done": false,
                "priority": "medium",
                "created_by": "zcode",
                "created_at": "2026-07-07T00:00:00Z",
                "history": [],
                "tags": [],
            })))
            .mount(&server)
            .await;

        let out = dispatch(
            (),
            ToolCall {
                name: "add_todo".into(),
                arguments: json!({ "title": "테스트", "agent": "zcode" }),
            },
        )
        .await
        .unwrap();
        assert_eq!(out["id"], json!("abc-123"));
        assert_eq!(out["created_by"], "zcode");

        std::env::remove_var("ACCESS_API_BASE");
    }).await;
}

#[tokio::test]
async fn list_todos_via_dispatch() {
    with_env_lock(|| async {
        let server = MockServer::start().await;
        std::env::set_var("ACCESS_API_BASE", &server.uri());

        Mock::given(method("GET"))
            .and(path("/todos"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "todos": [
                    { "id": "1", "title": "A", "done": false },
                    { "id": "2", "title": "B", "done": true },
                ],
                "categories": [],
            })))
            .mount(&server)
            .await;

        let out = dispatch(
            (),
            ToolCall {
                name: "list_todos".into(),
                arguments: json!({}),
            },
        )
        .await
        .unwrap();
        assert_eq!(out["todos"].as_array().unwrap().len(), 2);

        std::env::remove_var("ACCESS_API_BASE");
    }).await;
}

#[tokio::test]
async fn unknown_tool_errors() {
    with_env_lock(|| async {
        let server = MockServer::start().await;
        std::env::set_var("ACCESS_API_BASE", &server.uri());

        let res = dispatch(
            (),
            ToolCall {
                name: "bogus".into(),
                arguments: json!({}),
            },
        )
        .await;
        assert!(res.is_err());

        std::env::remove_var("ACCESS_API_BASE");
    }).await;
}

// ── Dev Note tool tests ──

#[tokio::test]
async fn catalog_includes_devnote_tools() {
    let catalog = mcp_server::tools::tool_catalog();
    let names: Vec<&str> = catalog.iter()
        .filter_map(|t| t["name"].as_str())
        .collect();
    assert!(names.contains(&"list_devnotes"));
    assert!(names.contains(&"add_devnote"));
    assert!(names.contains(&"update_devnote"));
    assert!(names.contains(&"generate_worklog"));
}

#[tokio::test]
async fn add_devnote_via_dispatch() {
    with_env_lock(|| async {
        let server = MockServer::start().await;
        std::env::set_var("ACCESS_API_BASE", &server.uri());

        Mock::given(method("POST"))
            .and(path("/devnotes"))
            .and(header("X-Agent", "zcode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "id": "note-test",
                "note_type": "retrospect",
                "title": "회고",
                "body": "내용",
                "created_by": "zcode",
            })))
            .mount(&server)
            .await;

        let out = dispatch(
            (),
            ToolCall {
                name: "add_devnote".into(),
                arguments: json!({
                    "note_type": "retrospect",
                    "title": "회고",
                    "body": "내용",
                    "agent": "zcode",
                }),
            },
        )
        .await
        .unwrap();
        assert_eq!(out["id"], json!("note-test"));
        assert_eq!(out["note_type"], "retrospect");

        std::env::remove_var("ACCESS_API_BASE");
    }).await;
}

#[tokio::test]
async fn generate_worklog_via_dispatch() {
    with_env_lock(|| async {
        let server = MockServer::start().await;
        std::env::set_var("ACCESS_API_BASE", &server.uri());

        Mock::given(method("POST"))
            .and(path("/devnotes/worklog/generate"))
            .and(header("X-Agent", "zcode"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "created": true,
                "note_id": "note-wl",
                "todos_counted": 3,
                "systems": ["combat", "ui"],
            })))
            .mount(&server)
            .await;

        let out = dispatch(
            (),
            ToolCall {
                name: "generate_worklog".into(),
                arguments: json!({ "agent": "zcode" }),
            },
        )
        .await
        .unwrap();
        assert_eq!(out["created"], true);
        assert_eq!(out["todos_counted"], 3);

        std::env::remove_var("ACCESS_API_BASE");
    }).await;
}
