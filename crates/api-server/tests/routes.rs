use axum::body::Body;
use http::{Request, StatusCode};
use tower::ServiceExt;

use api_server::app_for_test;

fn req(method: &str, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("X-Agent", "tester")
        .header("Content-Type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn body_str(b: Body) -> String {
    let bytes = axum::body::to_bytes(b, usize::MAX).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

#[tokio::test]
async fn list_returns_empty_array() {
    let app = app_for_test();
    let resp = app
        .oneshot(req("GET", "/todos", ""))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let body = body_str(resp.into_body()).await;
    assert!(body.contains("\"todos\":[]"));
}

#[tokio::test]
async fn add_then_list_and_toggle() {
    let app = app_for_test();
    // add
    let resp = app
        .clone()
        .oneshot(req("POST", "/todos", r#"{"title":"장보기","note":"우유"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let added = body_str(resp.into_body()).await;
    let v: serde_json::Value = serde_json::from_str(&added).unwrap();
    let id = v["id"].as_str().unwrap().to_string();
    assert_eq!(v["created_by"], "tester");
    assert_eq!(v["title"], "장보기");
    // in-memory test store has no Gist configured → synced is always true.
    assert_eq!(v["synced"], true);

    // toggle
    let toggle_uri = format!("/todos/{}/toggle", id);
    let resp = app
        .clone()
        .oneshot(req("POST", &toggle_uri, ""))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let toggled = body_str(resp.into_body()).await;
    let v: serde_json::Value = serde_json::from_str(&toggled).unwrap();
    assert_eq!(v["done"], true);
    assert_eq!(v["completed_by"], "tester");
    assert_eq!(v["synced"], true);

    // list
    let resp = app.oneshot(req("GET", "/todos", "")).await.unwrap();
    let body = body_str(resp.into_body()).await;
    let v: serde_json::Value = serde_json::from_str(&body).unwrap();
    assert_eq!(v["todos"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn update_records_actor() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/todos", r#"{"title":"A"}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/todos/{}", id);
    let resp = app
        .oneshot(req(
            "PATCH",
            &uri,
            r#"{"title":"B","priority":"high"}"#,
        ))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["title"], "B");
    assert_eq!(v["priority"], "high");
    assert_eq!(v["updated_by"], "tester");
}

#[tokio::test]
async fn delete_removes_todo() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/todos", r#"{"title":"A"}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/todos/{}", id);
    let resp = app.clone().oneshot(req("DELETE", &uri, "")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app.oneshot(req("GET", "/todos", "")).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["todos"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn list_strips_history_by_default() {
    let app = app_for_test();
    app.clone()
        .oneshot(req("POST", "/todos", r#"{"title":"A"}"#))
        .await
        .unwrap();
    let resp = app.oneshot(req("GET", "/todos", "")).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    let t = &v["todos"][0];
    assert!(t.get("history").is_none(), "history should be stripped by default");

    // opt-in 으로 포함 가능
    let app = app_for_test();
    app.clone()
        .oneshot(req("POST", "/todos", r#"{"title":"B"}"#))
        .await
        .unwrap();
    let resp = app.oneshot(req("GET", "/todos?include_history=true", "")).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert!(v["todos"][0].get("history").is_some(), "history should be present when opted in");
}


#[tokio::test]
async fn rejects_empty_title_with_422() {
    let app = app_for_test();
    let resp = app
        .oneshot(req("POST", "/todos", r#"{"title":"   "}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert!(v["detail"].as_str().unwrap().contains("title"), "detail field should explain: {v}");
    assert_eq!(v["type"], "https://access.errors/validation");
    assert_eq!(v["status"], 422);
}

#[tokio::test]
async fn rejects_bad_due_date_with_422() {
    let app = app_for_test();
    let resp = app
        .oneshot(req("POST", "/todos", r#"{"title":"A","due_date":"tomorrow"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn accepts_well_formed_todo() {
    let app = app_for_test();
    let resp = app
        .oneshot(req("POST", "/todos", r#"{"title":"  정상  ","due_date":"2026-07-20"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["title"], "정상", "title should be trimmed");
}
#[tokio::test]
async fn health_ok() {
    let app = app_for_test();
    let resp = app.oneshot(req("GET", "/health", "")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

#[tokio::test]
async fn batch_add_uses_x_agent_for_created_by() {
    // X-Agent 헤더가 created_by에 기록되고, body.agent 는 태그로만 들어가야 한다.
    let app = app_for_test();
    let body = r#"{"agent":"hermes","todos":[{"title":"온보딩1"},{"title":"온보딩2"}]}"#;
    let resp = app
        .clone()
        .oneshot(req("POST", "/batch/todos", body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["created"], 2);
    assert_eq!(v["synced"], true);

    // list 한 뒤 두 항목 모두 created_by=tester (헤더), agent:hermes 태그 포함 확인
    let resp = app.oneshot(req("GET", "/todos", "")).await.unwrap();
    let v: serde_json::Value = serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    let todos = v["todos"].as_array().unwrap();
    assert_eq!(todos.len(), 2);
    for t in todos {
        assert_eq!(t["created_by"], "tester", "created_by should come from X-Agent header");
        let tags = t["tags"].as_array().unwrap();
        assert!(tags.iter().any(|x| x == "agent:hermes"), "missing agent:hermes tag");
    }
}
