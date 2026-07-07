//! Integration tests for MCP tool dispatch.
//!
//! Spins up a mock HTTP server (wiremock) that imitates the Access REST
//! API, points `dispatch` at it via `ACCESS_API_BASE`, and verifies each
//! tool call produces the correct HTTP request and propagates the response.

use mcp_server::tools::{dispatch, ToolCall};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn add_todo_via_dispatch() {
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
}

#[tokio::test]
async fn list_todos_via_dispatch() {
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
}

#[tokio::test]
async fn unknown_tool_errors() {
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
}
