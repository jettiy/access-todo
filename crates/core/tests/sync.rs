use todo_core::gist::GistClient;
use todo_core::model::{Priority, TodoDoc};
use todo_core::store::{Store, TodoInput};
use todo_core::sync::SyncEngine;
use wiremock::matchers::{header, method};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn remote_doc_with(title: &str) -> serde_json::Value {
    let mut s = Store::new();
    s.add(
        TodoInput {
            title: title.into(),
            note: None,
            priority: Priority::Low,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "remote-actor",
    );
    serde_json::json!({
        "files": {
            "todos.json": {
                "content": serde_json::to_string(&TodoDoc {
                    version: "1.0".into(),
                    updated_at: chrono::Utc::now(),
                    updated_by: "remote-actor".into(),
                    todos: s.list(),
                    categories: vec![],
                }).unwrap()
            }
        }
    })
}

#[tokio::test]
async fn pull_merges_remote_into_local() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(remote_doc_with("remote-item")))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    let doc = engine.pull(&mut local, "user").await.unwrap().expect("expected Modified");
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
    engine.push(&mut local, "user").await.unwrap();
}

#[tokio::test]
async fn push_retries_on_conflict_until_convergence() {
    // 시나리오: 첫 PATCH → 412 Conflict → GET (새 원격) → PATCH → 200 OK.
    // 엔진이 자동으로 pull-merge 후 재시도해야 한다.
    let server = MockServer::start().await;

    // 첫 PATCH: 412 반환 (원격이 바뀌었음)
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(412).set_body_string("conflict"))
        .up_to_n_times(1)
        .mount(&server)
        .await;

    // conflict 후 pull할 때 쓰일 GET 응답
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(remote_doc_with("remote-item")))
        .mount(&server)
        .await;

    // 재시도 PATCH: 성공
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    local.add(
        TodoInput {
            title: "local-item".into(),
            note: None,
            priority: Priority::Medium,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    engine.push(&mut local, "user").await.expect("push converges after one retry");

    // pull-merge로 원격 항목도 local에 들어와야 한다.
    let titles: Vec<_> = local.list().iter().map(|t| t.title.clone()).collect();
    assert!(titles.contains(&"local-item".to_string()));
    assert!(titles.contains(&"remote-item".to_string()));
}

#[tokio::test]
async fn push_returns_conflict_after_max_retries() {
    // 시나리오: PATCH가 계속 412만 반환 → MAX_PUSH_RETRIES 후 Conflict 에러.
    let server = MockServer::start().await;

    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(412).set_body_string("always conflict"))
        .mount(&server)
        .await;
    // pull 단계에서 쓰일 GET 응답 (빈 문서)
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(remote_doc_with("remote")))
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    local.add(
        TodoInput {
            title: "x".into(),
            note: None,
            priority: Priority::Medium,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    let res = engine.push(&mut local, "user").await;
    assert!(res.is_err(), "should fail after retries");
    let msg = format!("{}", res.unwrap_err());
    assert!(msg.contains("conflict") || msg.contains("Conflict"), "got: {msg}");
}

#[tokio::test]
async fn push_sends_if_match_when_etag_present() {
    // etag가 있으면 If-Match 헤더를 보내야 한다.
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(header("If-Match", "\"v1\""))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    // 먼저 한 번 pull해서 etag를 채운다.
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("ETag", "\"v1\"")
                .set_body_json(remote_doc_with("seeded")),
        )
        .mount(&server)
        .await;

    let client = GistClient::with_base("tok".into(), server.uri());
    let engine = SyncEngine::new(client, "GIST".into());
    let mut local = Store::new();
    engine.pull(&mut local, "u").await.unwrap();

    local.add(
        TodoInput {
            title: "new".into(),
            note: None,
            priority: Priority::Medium,
            due_date: None,
            tags: vec![],
            category_id: None,
        },
        "user",
    );
    engine.push(&mut local, "user").await.expect("push should match If-Match and succeed");
}
