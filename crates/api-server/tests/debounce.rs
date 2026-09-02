//! Integration test for the debounced Gist flusher.
//!
//! Spins up a mock Gist, points AppState at it, and verifies that a burst
//! of mutations coalesces into a single PATCH within the debounce window.

use api_server::AppState;
use todo_core::store::TodoInput;
use wiremock::matchers::method;
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test(flavor = "current_thread")]
async fn rapid_mutations_coalesce_into_one_push() {
    let server = MockServer::start().await;

    // Initial GET (pull on startup) → empty doc.
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "files": { "todos.json": { "content": r#"{"version":"1.0","updated_at":"2026-01-01T00:00:00Z","updated_by":"x","todos":[]}"# } }
        })))
        .mount(&server)
        .await;

    // PATCH: respond OK to any number of calls — we count below.
    Mock::given(method("PATCH"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({})))
        .mount(&server)
        .await;

    let engine = todo_core::sync::SyncEngine::new(
        todo_core::gist::GistClient::with_base("tok".into(), server.uri()),
        "GIST".into(),
    );
    let state = AppState::with_sync_engine(engine);
    state.clone().spawn_flusher();

    // Three rapid mutations — just flip the dirty flag, no synchronous push.
    for i in 0..3 {
        let mut st = state.store.write().await;
        st.add(
            TodoInput {
                title: format!("t{i}"),
                ..Default::default()
            },
            "tester",
        );
        drop(st);
        assert!(state.mark_push_pending(), "no prior failure → synced=true");
    }

    // Wait for the debounce window (1.5s) plus margin so the flusher has
    // had time to push exactly once.
    tokio::time::sleep(std::time::Duration::from_millis(2200)).await;

    let patch_observed = server
        .received_requests()
        .await
        .map(|r| r.into_iter().filter(|r| r.method == "PATCH").count())
        .unwrap_or(0);
    assert_eq!(
        patch_observed, 1,
        "3 rapid mutations must coalesce into exactly 1 PATCH, got {patch_observed}"
    );
}
