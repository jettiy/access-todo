use std::sync::Arc;

use mcp_server::tools::{dispatch, ToolCall};
use serde_json::json;
use tokio::sync::Mutex;

#[tokio::test]
async fn add_todo_via_dispatch() {
    let store = Arc::new(Mutex::new(todo_core::store::Store::new()));
    let call = ToolCall {
        name: "add_todo".into(),
        arguments: json!({ "title": "테스트", "agent": "zcode" }),
    };
    let out = dispatch(store.clone(), call).await.unwrap();
    assert!(out["id"].as_str().is_some());
    assert_eq!(out["created_by"], "zcode");
    assert_eq!(store.lock().await.list().len(), 1);
}

#[tokio::test]
async fn toggle_then_delete_roundtrip() {
    let store = Arc::new(Mutex::new(todo_core::store::Store::new()));
    let added = dispatch(
        store.clone(),
        ToolCall {
            name: "add_todo".into(),
            arguments: json!({ "title": "A", "agent": "hermes" }),
        },
    )
    .await
    .unwrap();
    let id = added["id"].as_str().unwrap().to_string();

    let toggled = dispatch(
        store.clone(),
        ToolCall {
            name: "toggle_todo".into(),
            arguments: json!({ "id": id, "agent": "hermes" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(toggled["done"], true);
    assert_eq!(toggled["completed_by"], "hermes");

    dispatch(
        store.clone(),
        ToolCall {
            name: "delete_todo".into(),
            arguments: json!({ "id": id, "agent": "hermes" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(store.lock().await.list().len(), 0);
}

#[tokio::test]
async fn list_and_search() {
    let store = Arc::new(Mutex::new(todo_core::store::Store::new()));
    dispatch(
        store.clone(),
        ToolCall {
            name: "add_todo".into(),
            arguments: json!({ "title": "Buy groceries", "note": "milk", "tags": ["errand"], "agent": "user" }),
        },
    )
    .await
    .unwrap();

    let listed = dispatch(
        store.clone(),
        ToolCall {
            name: "list_todos".into(),
            arguments: json!({}),
        },
    )
    .await
    .unwrap();
    assert_eq!(listed["todos"].as_array().unwrap().len(), 1);

    let searched = dispatch(
        store,
        ToolCall {
            name: "search_todos".into(),
            arguments: json!({ "q": "milk" }),
        },
    )
    .await
    .unwrap();
    assert_eq!(searched["todos"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn unknown_tool_errors() {
    let store = Arc::new(Mutex::new(todo_core::store::Store::new()));
    let res = dispatch(
        store,
        ToolCall {
            name: "bogus".into(),
            arguments: json!({}),
        },
    )
    .await;
    assert!(res.is_err());
}
