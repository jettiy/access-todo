//! Integration tests for MCP tool dispatch.
//!
//! Spins up a mock HTTP server (wiremock) that imitates the Access REST
//! API, then calls `dispatch` with the server's URL injected directly.
//! Each test owns its own mock server, so cases can run fully in parallel
//! without racing on a process-wide env var.

use mcp_server::tools::{dispatch, ToolCall};
use serde_json::json;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn add_todo_via_dispatch() {
    let server = MockServer::start().await;

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
        server.uri().as_str(),
        ToolCall {
            name: "add_todo".into(),
            arguments: json!({ "title": "테스트", "agent": "zcode" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(out["id"], json!("abc-123"));
    assert_eq!(out["created_by"], "zcode");
}

#[tokio::test]
async fn list_todos_via_dispatch() {
    let server = MockServer::start().await;

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
        server.uri().as_str(),
        ToolCall {
            name: "list_todos".into(),
            arguments: json!({}),
        },
    )
    .await
    .unwrap();
    assert_eq!(out["todos"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn unknown_tool_errors() {
    let server = MockServer::start().await;

    let res = dispatch(
        server.uri().as_str(),
        ToolCall {
            name: "bogus".into(),
            arguments: json!({}),
        },
    )
    .await;
    assert!(res.is_err());
}

#[tokio::test]
async fn list_todos_filters_by_agent_when_provided() {
    // agent 인수가 있으면 ?agent=<name> 쿼리로 필터링해야 한다.
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/todos"))
        .and(wiremock::matchers::query_param("agent", "hermes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "todos": [{ "id": "h1", "title": "hermes-only", "done": false }],
            "categories": [],
        })))
        .mount(&server)
        .await;

    let out = dispatch(
        server.uri().as_str(),
        ToolCall {
            name: "list_todos".into(),
            arguments: json!({ "agent": "hermes" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(out["todos"].as_array().unwrap().len(), 1);
    assert_eq!(out["todos"][0]["title"], json!("hermes-only"));
}
