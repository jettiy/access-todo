//! HTTP route handlers for the 8 REST endpoints mirroring the MCP tools.

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;

use todo_core::model::Priority;
use todo_core::store::{TodoInput, TodoPatch};

use crate::auth::agent_from_headers;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct NewTodo {
    pub title: String,
    pub note: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
pub struct PatchTodo {
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub note: Option<Option<String>>,
    #[serde(default)]
    pub priority: Option<String>,
    #[serde(default)]
    pub due_date: Option<Option<String>>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct SearchQ {
    pub q: String,
}

#[derive(Deserialize, Default)]
pub struct ListQ {
    #[serde(default)]
    pub done: Option<bool>,
}

fn parse_prio(s: String) -> Priority {
    match s.as_str() {
        "high" => Priority::High,
        "low" => Priority::Low,
        _ => Priority::Medium,
    }
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/todos", get(list).post(add))
        .route("/todos/today", get(today))
        .route("/todos/search", get(search))
        .route("/todos/:id", get(one).patch(update).delete(remove))
        .route("/todos/:id/toggle", post(toggle))
        .route("/sync", post(sync_handler))
        .route("/health", get(|| async { "ok" }))
        .with_state(state)
}

/// Manually trigger a Gist pull (GET remote → merge → local).
async fn sync_handler(State(s): State<AppState>) -> Json<serde_json::Value> {
    match s.pull("sync-endpoint").await {
        Ok(()) => {
            let st = s.store.lock().await;
            Json(serde_json::json!({ "ok": true, "count": st.list().len() }))
        }
        Err(e) => Json(serde_json::json!({ "ok": false, "error": e })),
    }
}

async fn list(
    State(s): State<AppState>,
    q: Query<ListQ>,
    h: HeaderMap,
) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let st = s.store.lock().await;
    let mut todos = st.list();
    if let Some(done) = q.done {
        todos.retain(|t| t.done == done);
    }
    Json(serde_json::json!({ "todos": todos }))
}

async fn today(State(s): State<AppState>, h: HeaderMap) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let st = s.store.lock().await;
    Json(serde_json::json!({ "todos": st.list_today() }))
}

async fn search(
    State(s): State<AppState>,
    q: Query<SearchQ>,
    h: HeaderMap,
) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let st = s.store.lock().await;
    Json(serde_json::json!({ "todos": st.search(&q.q) }))
}

async fn one(State(s): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let st = s.store.lock().await;
    match st.get(&id) {
        Some(t) => Json(serde_json::to_value(t).unwrap_or(serde_json::Value::Null)),
        None => Json(serde_json::Value::Null),
    }
}

async fn add(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<NewTodo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let t = {
        let mut st = s.store.lock().await;
        st.add(
            TodoInput {
                title: b.title,
                note: b.note,
                priority: parse_prio(b.priority.unwrap_or_default()),
                due_date: b.due_date,
                tags: b.tags.unwrap_or_default(),
            },
            &actor,
        )
    };
    // Best-effort push to Gist (ignore network errors).
    if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
    Ok(Json(serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
}

async fn update(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
    Json(b): Json<PatchTodo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.lock().await;
        let patch = TodoPatch {
            title: b.title,
            note: b.note,
            priority: b.priority.map(parse_prio),
            due_date: b.due_date,
            tags: b.tags,
        };
        st.update(&id, patch, &actor)
    };
    match result {
        Ok(t) => {
            if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
            Ok(Json(serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn toggle(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.lock().await;
        st.toggle(&id, &actor)
    };
    match result {
        Ok(t) => {
            if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
            Ok(Json(serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

async fn remove(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.lock().await;
        st.delete(&id, &actor)
    };
    match result {
        Ok(()) => {
            if let Err(e) = s.push(&actor).await { eprintln!("warn: gist push failed: {e}"); }
            Ok(Json(serde_json::json!({ "deleted": id })))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
