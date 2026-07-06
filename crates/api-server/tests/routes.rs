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
