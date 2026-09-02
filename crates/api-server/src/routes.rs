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
use crate::error::ApiError;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct NewTodo {
    pub title: String,
    pub note: Option<String>,
    pub priority: Option<String>,
    pub due_date: Option<String>,
    pub tags: Option<Vec<String>>,
    pub category_id: Option<String>,
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
    #[serde(default)]
    pub category_id: Option<Option<String>>,
}

#[derive(Deserialize)]
pub struct SearchQ {
    pub q: String,
}

/// Body for POST /todos/:id/complete
#[derive(Deserialize)]
pub struct CompleteBody {
    pub summary: Option<String>,
}

/// Body for POST /categories
#[derive(Deserialize)]
pub struct NewCategory {
    pub agent: String,
    pub name: String,
}

/// Body for PATCH /categories/:id
#[derive(Deserialize)]
pub struct RenameCategory {
    pub name: String,
}

/// Body for POST /categories/reorder
#[derive(Deserialize)]
pub struct ReorderCategories {
    pub agent: String,
    pub ordered_ids: Vec<String>,
}

/// A single item in POST /todos/batch
#[derive(Deserialize)]
pub struct BatchItem {
    pub title: String,
    pub note: Option<String>,
    pub priority: Option<String>,
    pub category_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Body for POST /todos/batch — bulk-add todos for onboarding
#[derive(Deserialize)]
pub struct BatchTodos {
    pub agent: String,
    pub todos: Vec<BatchItem>,
}

/// Query for GET /review
#[derive(Deserialize)]
pub struct ReviewQ {
    pub agent: String,
}

#[derive(Deserialize, Default)]
pub struct ListQ {
    #[serde(default)]
    pub done: Option<bool>,
    #[serde(default)]
    pub agent: Option<String>,
    /// Include per-todo `history` in the response. Default `false` because
    /// the audit trail is large and post-it clients don't render it.
    #[serde(default = "default_false")]
    pub include_history: bool,
}

fn default_false() -> bool {
    false
}

/// Strip `history` from every todo in `v` (in-place). Used by read paths
/// unless the caller opted in via `?include_history=true`.
fn strip_history(v: &mut serde_json::Value) {
    if let Some(arr) = v.get_mut("todos").and_then(|t| t.as_array_mut()) {
        for t in arr {
            if let Some(obj) = t.as_object_mut() {
                obj.remove("history");
            }
        }
    }
}

fn parse_prio(s: String) -> Priority {
    match s.as_str() {
        "high" => Priority::High,
        "low" => Priority::Low,
        _ => Priority::Medium,
    }
}

pub fn router(state: AppState) -> Router {
    use tower_http::cors::{Any, CorsLayer};
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/todos", get(list).post(add))
        .route("/batch/todos", post(add_batch))
        .route("/todos/today", get(today))
        .route("/todos/search", get(search))
        .route("/todos/:id", get(one).patch(update).delete(remove))
        .route("/todos/:id/toggle", post(toggle))
        .route("/todos/:id/complete", post(complete))
        .route("/categories", post(create_category))
        .route("/categories/reorder", post(reorder_category))
        .route("/categories/:id", axum::routing::patch(rename_category).delete(delete_category))
        .route("/review", axum::routing::get(review))
        .route("/sync", post(sync_handler))
        .route("/health", get(|| async { "ok" }))
        .layer(cors)
        .with_state(state)
}

/// POST /categories — create a category for an agent
async fn create_category(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<NewCategory>,
) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let cat = {
        let mut st = s.store.write().await;
        st.add_category(&b.agent, &b.name)
    };
    let synced = s.mark_push_pending();
    let mut v = serde_json::to_value(cat).unwrap_or(serde_json::Value::Null);
    v["synced"] = synced.into();
    Json(v)
}

/// PATCH /categories/:id — rename a category
async fn rename_category(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
    Json(b): Json<RenameCategory>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.write().await;
        st.rename_category(&id, &b.name)
    };
    match result {
        Ok(c) => {
            let synced = s.mark_push_pending();
            let mut v = serde_json::to_value(c).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            v["synced"] = synced.into();
            Ok(Json(v))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /categories/reorder — reorder categories for an agent
async fn reorder_category(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<ReorderCategories>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.write().await;
        st.reorder_categories(&b.agent, &b.ordered_ids)
    };
    match result {
        Ok(()) => {
            let synced = s.mark_push_pending();
            Ok(Json(serde_json::json!({ "ok": true, "synced": synced })))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// DELETE /categories/:id — delete a category. Todos in it become uncategorized.
async fn delete_category(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.write().await;
        st.delete_category(&id)
    };
    match result {
        Ok(()) => {
            let synced = s.mark_push_pending();
            Ok(Json(serde_json::json!({ "deleted": id, "synced": synced })))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /todos/batch — bulk-add todos (onboarding).
///
/// `X-Agent` (header) is recorded as `created_by`, mirroring single `add`,
/// so audit attribution stays consistent across endpoints. `body.agent` is
/// only used to tag the todos (`agent:<name>`) so they show up on the
/// target agent's post-it — onboarding one agent from another does not
/// misattribute authorship.
async fn add_batch(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<BatchTodos>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = agent_from_headers(&h);
    let agent_tag = format!("agent:{}", b.agent);
    let mut ids: Vec<String> = vec![];
    {
        let mut st = s.store.write().await;
        for item in &b.todos {
            let mut tags = item.tags.clone().unwrap_or_default();
            if !tags.contains(&agent_tag) {
                tags.push(agent_tag.clone());
            }
            let mut input = TodoInput {
                title: item.title.clone(),
                note: item.note.clone(),
                priority: parse_prio(item.priority.clone().unwrap_or_default()),
                due_date: None,
                tags,
                category_id: item.category_id.clone(),
            };
            if let Err(msg) = input.validate() {
                return Err(ApiError::Validation(format!(
                    "{msg} (at index {})",
                    ids.len()
                )));
            }
            let t = st.add(input, &actor);
            ids.push(t.id);
        }
    }
    let synced = s.mark_push_pending();
    Ok(Json(serde_json::json!({ "created": ids.len(), "ids": ids, "synced": synced })))
}

/// GET /review?agent=<name> — 에이전트 진행상황 종합 요약.
/// 완료된 작업, 진행 중인 작업, 우선순위별 남은 작업을 집계.
async fn review(
    State(s): State<AppState>,
    q: Query<ReviewQ>,
) -> Json<serde_json::Value> {
    let st = s.store.read().await;
    let todos = st.list();
    let categories = st.list_categories(&q.agent);

    // 에이전트 필터
    let tag = format!("agent:{}", q.agent);
    let my_todos: Vec<_> = todos.iter()
        .filter(|t| t.tags.iter().any(|x| x == &tag) || t.created_by == q.agent)
        .collect();

    let total = my_todos.len();
    let done = my_todos.iter().filter(|t| t.done).count();
    let pending = total - done;

    // 우선순위별 집계
    let high = my_todos.iter().filter(|t| !t.done && matches!(t.priority, Priority::High)).count();
    let medium = my_todos.iter().filter(|t| !t.done && matches!(t.priority, Priority::Medium)).count();
    let low = my_todos.iter().filter(|t| !t.done && matches!(t.priority, Priority::Low)).count();

    // 최근 완료 작업 (요약 포함)
    let recent_done: Vec<_> = my_todos.iter()
        .filter(|t| t.done)
        .map(|t| serde_json::json!({
            "title": t.title,
            "summary": t.summary.as_deref().unwrap_or(""),
            "completed_by": t.completed_by.as_deref().unwrap_or(""),
        }))
        .collect();

    // 남은 높은 우선순위 작업
    let urgent: Vec<_> = my_todos.iter()
        .filter(|t| !t.done && matches!(t.priority, Priority::High))
        .map(|t| {
            let cat_name = categories.iter()
                .find(|c| Some(&c.id) == t.category_id.as_ref())
                .map(|c| c.name.as_str())
                .unwrap_or("미분류");
            serde_json::json!({ "title": t.title, "category": cat_name })
        })
        .collect();

    let progress_pct = if total > 0 { done * 100 / total } else { 0 };

    Json(serde_json::json!({
        "agent": q.agent,
        "progress": format!("{done}/{total} ({progress_pct}%)"),
        "summary": {
            "total": total,
            "completed": done,
            "pending": pending,
            "by_priority": { "high": high, "medium": medium, "low": low }
        },
        "recently_completed": recent_done,
        "urgent_next": urgent,
        "categories": categories.len(),
    }))
}

/// Manually trigger a round-trip: pull from Gist, then force-push the
/// merged result (skipping the debouncer). Useful as a "sync now" button.
async fn sync_handler(State(s): State<AppState>) -> Json<serde_json::Value> {
    match s.pull("sync-endpoint").await {
        Ok(()) => {
            let pushed = s.push_now("sync-endpoint").await;
            let st = s.store.read().await;
            Json(serde_json::json!({
                "ok": true,
                "count": st.list().len(),
                "pushed": pushed,
            }))
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
    let st = s.store.read().await;
    let mut todos = st.list();
    if let Some(done) = q.done {
        todos.retain(|t| t.done == done);
    }
    // 에이전트별 필터: tags의 "agent:<name>" 또는 created_by 일치
    if let Some(agent) = &q.agent {
        let tag = format!("agent:{agent}");
        todos.retain(|t| t.tags.iter().any(|x| x == &tag) || t.created_by == *agent);
    }
    // 해당 에이전트의 카테고리도 함께 반환
    let categories = if let Some(agent) = &q.agent {
        st.list_categories(agent)
    } else {
        vec![]
    };
    let mut v = serde_json::json!({ "todos": todos, "categories": categories });
    if !q.include_history {
        strip_history(&mut v);
    }
    Json(v)
}

async fn today(State(s): State<AppState>, h: HeaderMap) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let st = s.store.read().await;
    let mut v = serde_json::json!({ "todos": st.list_today() });
    strip_history(&mut v);
    Json(v)
}

async fn search(
    State(s): State<AppState>,
    q: Query<SearchQ>,
    h: HeaderMap,
) -> Json<serde_json::Value> {
    let _actor = agent_from_headers(&h);
    let st = s.store.read().await;
    let mut v = serde_json::json!({ "todos": st.search(&q.q) });
    strip_history(&mut v);
    Json(v)
}

async fn one(State(s): State<AppState>, Path(id): Path<String>) -> Json<serde_json::Value> {
    let st = s.store.read().await;
    match st.get(&id) {
        Some(t) => {
            let mut v = serde_json::to_value(t).unwrap_or(serde_json::Value::Null);
            if let Some(obj) = v.as_object_mut() {
                obj.remove("history");
            }
            Json(v)
        }
        None => Json(serde_json::Value::Null),
    }
}


async fn add(
    State(s): State<AppState>,
    h: HeaderMap,
    Json(b): Json<NewTodo>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let actor = agent_from_headers(&h);
    let mut input = TodoInput {
        title: b.title,
        note: b.note,
        priority: parse_prio(b.priority.unwrap_or_default()),
        due_date: b.due_date,
        tags: b.tags.unwrap_or_default(),
        category_id: b.category_id,
    };
    if let Err(msg) = input.validate() {
        return Err(ApiError::Validation(msg));
    }
    let t = {
        let mut st = s.store.write().await;
        st.add(input, &actor)
    };
    let synced = s.mark_push_pending();
    let mut v = serde_json::to_value(t)?;
    v["synced"] = synced.into();
    Ok(Json(v))
}

async fn update(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
    Json(b): Json<PatchTodo>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let result = {
        let mut st = s.store.write().await;
        let patch = TodoPatch {
            title: b.title,
            note: b.note,
            priority: b.priority.map(parse_prio),
            due_date: b.due_date,
            tags: b.tags,
            category_id: b.category_id,
        };
        st.update(&id, patch, &actor)
    };
    match result {
        Ok(t) => {
            let synced = s.mark_push_pending();
            let mut v = serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            v["synced"] = synced.into();
            Ok(Json(v))
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
        let mut st = s.store.write().await;
        st.toggle(&id, &actor)
    };
    match result {
        Ok(t) => {
            let synced = s.mark_push_pending();
            let mut v = serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            v["synced"] = synced.into();
            Ok(Json(v))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}

/// POST /todos/:id/complete — 체크 + 작업 요약 기록
async fn complete(
    State(s): State<AppState>,
    Path(id): Path<String>,
    h: HeaderMap,
    Json(b): Json<CompleteBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let actor = agent_from_headers(&h);
    let summary = b.summary.unwrap_or_else(|| "작업 완료".into());
    let result = {
        let mut st = s.store.write().await;
        st.complete_with_summary(&id, &summary, &actor)
    };
    match result {
        Ok(t) => {
            let synced = s.mark_push_pending();
            let mut v = serde_json::to_value(t).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            v["synced"] = synced.into();
            Ok(Json(v))
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
        let mut st = s.store.write().await;
        st.delete(&id, &actor)
    };
    match result {
        Ok(_) => {
            let synced = s.mark_push_pending();
            Ok(Json(serde_json::json!({ "deleted": id, "synced": synced })))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
