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
async fn health_ok() {
    let app = app_for_test();
    let resp = app.oneshot(req("GET", "/health", "")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
}

// ── Dev Notes API tests ──

#[tokio::test]
async fn devnote_create_and_list() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"회고","body":"적 체력 높음","game_system":"combat","version":"v0.2"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["title"], "회고");
    assert_eq!(v["note_type"], "retrospect");

    // list
    let resp = app
        .clone()
        .oneshot(req("GET", "/devnotes", ""))
        .await
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn devnote_update_body() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"issue","title":"버그","body":"원본"}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/devnotes/{}", id);
    let resp = app
        .oneshot(req("PATCH", &uri, r#"{"body":"수정됨"}"#))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["body"], "수정됨");
}

#[tokio::test]
async fn devnote_delete() {
    let app = app_for_test();
    let resp = app
        .clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"x","body":""}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    let uri = format!("/devnotes/{}", id);
    let resp = app.clone().oneshot(req("DELETE", &uri, "")).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let resp = app.oneshot(req("GET", "/devnotes", "")).await.unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn worklog_generate_from_completed_todo() {
    let app = app_for_test();
    // add a todo with system:combat tag
    let resp = app
        .clone()
        .oneshot(req("POST", "/todos", r#"{"title":"적 패턴","tags":["system:combat","version:v0.2"]}"#))
        .await
        .unwrap();
    let id: String = serde_json::from_str::<serde_json::Value>(&body_str(resp.into_body()).await)
        .unwrap()["id"]
        .as_str()
        .unwrap()
        .into();
    // complete it
    let complete_uri = format!("/todos/{}/complete", id);
    let _ = app
        .clone()
        .oneshot(req("POST", &complete_uri, r#"{"summary":"상태머신 추가"}"#))
        .await
        .unwrap();
    // generate worklog
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let body = format!(r#"{{"date":"{}"}}"#, today);
    let resp = app
        .oneshot(req("POST", "/devnotes/worklog/generate", &body))
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["created"], true);
    assert_eq!(v["todos_counted"], 1);
}

#[tokio::test]
async fn devnotes_filter_by_type() {
    let app = app_for_test();
    app.clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"retrospect","title":"r","body":""}"#))
        .await
        .unwrap();
    app.clone()
        .oneshot(req("POST", "/devnotes", r#"{"note_type":"issue","title":"i","body":""}"#))
        .await
        .unwrap();
    let resp = app
        .oneshot(req("GET", "/devnotes?note_type=issue", ""))
        .await
        .unwrap();
    let v: serde_json::Value =
        serde_json::from_str(&body_str(resp.into_body()).await).unwrap();
    assert_eq!(v["devnotes"].as_array().unwrap().len(), 1);
    assert_eq!(v["devnotes"][0]["title"], "i");
}
