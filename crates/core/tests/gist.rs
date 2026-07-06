use todo_core::gist::GistClient;
use todo_core::model::TodoDoc;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn fetch_parses_doc_and_etag() {
    let server = MockServer::start().await;
    let body = serde_json::json!({
        "files": {
            "todos.json": {
                "content": serde_json::to_string(&TodoDoc::new("u")).unwrap()
            }
        }
    });
    Mock::given(method("GET")).respond_with(
        ResponseTemplate::new(200)
            .insert_header("ETag", "\"abc\"")
            .set_body_json(body),
    )
    .mount(&server)
    .await;

    let client = GistClient::with_base("dummy".into(), server.uri());
    let (doc, etag) = client.fetch("GIST_ID", None).await.unwrap();
    assert_eq!(doc.version, "1.0");
    assert_eq!(etag.as_deref(), Some("\"abc\""));
}

#[tokio::test]
async fn push_sends_patch_and_returns_etag() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"def\"")
                .set_body_json(serde_json::json!({})),
        )
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let etag = client
        .push("GIST_ID", &TodoDoc::new("u"), None)
        .await
        .unwrap();
    assert_eq!(etag.as_deref(), Some("\"def\""));
}

#[tokio::test]
async fn push_error_on_non_success() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(422).set_body_string("validation failed"))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let res = client.push("GIST_ID", &TodoDoc::new("u"), None).await;
    assert!(res.is_err());
}
