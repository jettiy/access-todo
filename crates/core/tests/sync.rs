use todo_core::gist::GistClient;
use todo_core::model::{Priority, TodoDoc};
use todo_core::store::{Store, TodoInput};
use todo_core::sync::SyncEngine;
use wiremock::{matchers::method, Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn pull_merges_remote_into_local() {
    let server = MockServer::start().await;
    // remote has one todo with title "remote-item"
    let mut remote_store = Store::new();
    remote_store.add(
        TodoInput {
            title: "remote-item".into(),
            note: None,
            priority: Priority::Low,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "remote-actor",
    );
    let remote_doc = TodoDoc {
        version: "1.0".into(),
        updated_at: chrono::Utc::now(),
        updated_by: "remote-actor".into(),
        todos: remote_store.list(),
        categories: vec![],
    };
    let body = serde_json::json!({
        "files": {
            "todos.json": {
                "content": serde_json::to_string(&remote_doc).unwrap()
            }
        }
    });
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    let doc = engine.pull(&mut local, "user").await.unwrap();
    assert_eq!(doc.todos.len(), 1);
    assert_eq!(local.list()[0].title, "remote-item");
}

#[tokio::test]
async fn push_writes_local_doc_to_gist() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    local.add(
        TodoInput {
            title: "pushed".into(),
            note: None,
            priority: Priority::Medium,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    engine.push(&local, "user").await.unwrap();
}
