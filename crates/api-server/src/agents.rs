//! `/agents` and `/settings` endpoints — the API surface behind the P1
//! settings window: agent detection, MCP registration toggles, pilot
//! knowledge level, and connection self-healing (제품기획 v1.2).

use agent_registry::{Outcome, Registry, SPECS};
use axum::{extract::Path, extract::State, Json};
use serde::Deserialize;
use serde_json::Value;

use crate::state::AppState;

// ── /settings ──

#[derive(Deserialize, Default)]
pub struct SettingsPatch {
    #[serde(default)]
    pub pilot_level: Option<String>,
    #[serde(default)]
    pub sync_mode: Option<String>,
    #[serde(default)]
    pub enabled_agents: Option<Vec<String>>,
}

/// GET /settings — current settings + whether Gist sync is live.
pub async fn get_settings(State(s): State<AppState>) -> Json<Value> {
    let st = s.settings.lock().await.clone();
    Json(serde_json::json!({
        "pilot_level": st.pilot_level,
        "sync_mode": st.sync_mode,
        "enabled_agents": st.enabled_agents,
        "gist_sync": s.sync.is_some(),
        "valid_levels": crate::settings::LEVELS,
    }))
}

/// PUT /settings — partial update, persisted to access-settings.json.
pub async fn put_settings(
    State(s): State<AppState>,
    Json(patch): Json<SettingsPatch>,
) -> Json<Value> {
    let mut st = s.settings.lock().await;
    if let Some(level) = &patch.pilot_level {
        st.pilot_level = level.clone();
    }
    if let Some(mode) = &patch.sync_mode {
        st.sync_mode = mode.clone();
    }
    if let Some(agents) = &patch.enabled_agents {
        st.enabled_agents = agents.clone();
    }
    st.normalize();
    let saved = st.save(&s.settings_path);
    Json(serde_json::json!({
        "ok": saved.is_ok(),
        "error": saved.err(),
        "settings": *st,
    }))
}

// ── /agents ──

/// GET /agents — detected agent tools with enabled/registered flags, plus
/// enabled custom (REST) agents that need no file registration.
pub async fn list_agents(State(s): State<AppState>) -> Json<Value> {
    let enabled = s.settings.lock().await.enabled_agents.clone();
    let mut agents: Vec<Value> = s
        .registry
        .detect()
        .into_iter()
        .map(|d| {
            let e = enabled.iter().any(|a| a == &d.id);
            serde_json::to_value(&d).map(|mut v| {
                v["enabled"] = Value::Bool(e);
                v
            }).unwrap_or(Value::Null)
        })
        .collect();
    // Custom REST agents (hermes/omp/…) the pilot enabled manually.
    for id in &enabled {
        let known = agents
            .iter()
            .any(|a| a["id"].as_str() == Some(id.as_str()));
        if !known {
            agents.push(serde_json::json!({
                "id": id,
                "display": id,
                "integration": "rest",
                "experimental": false,
                "installed": true,
                "config_path": "",
                "registered": true,
                "enabled": true,
            }));
        }
    }
    Json(serde_json::json!({ "agents": agents }))
}

/// POST /agents/:id/register — toggle ON: register the MCP server (known
/// tools) and/or mark enabled. Unknown ids become custom REST agents.
pub async fn register_agent(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let known = SPECS.iter().any(|sp| sp.id == id);
    let (outcome, error) = if known {
        if !s.mcp_exe.exists() {
            ("error", Some(format!("Access-MCP.exe not found at {}", s.mcp_exe.display())))
        } else {
            match s.registry.register(&id, &s.mcp_exe) {
                Ok(Outcome::AlreadyRegistered) => ("already-registered", None),
                Ok(Outcome::Registered { .. }) => ("registered", None),
                Ok(Outcome::NotInstalled) => ("not-installed", None),
                Ok(Outcome::Unregistered { .. }) => ("already-registered", None),
                Err(e) => ("error", Some(e.to_string())),
            }
        }
    } else {
        // Custom agent: REST integration, no file registration needed.
        ("enabled-rest", None)
    };
    let mut st = s.settings.lock().await;
    st.enable(&id);
    let saved = st.save(&s.settings_path);
    drop(st);
    Json(serde_json::json!({
        "id": id,
        "outcome": outcome,
        "error": error,
        "settings_saved": saved.is_ok(),
    }))
}

/// POST /agents/:id/unregister — toggle OFF: remove the MCP entry
/// (backup kept) and drop the agent from the enabled list.
pub async fn unregister_agent(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> Json<Value> {
    let known = SPECS.iter().any(|sp| sp.id == id);
    let (outcome, error) = if known {
        match s.registry.unregister(&id) {
            Ok(Outcome::Unregistered { .. }) => ("unregistered", None),
            Ok(_) => ("nothing-to-remove", None),
            Err(e) => ("error", Some(e.to_string())),
        }
    } else {
        ("disabled-rest", None)
    };
    let mut st = s.settings.lock().await;
    st.disable(&id);
    let saved = st.save(&s.settings_path);
    drop(st);
    Json(serde_json::json!({
        "id": id,
        "outcome": outcome,
        "error": error,
        "settings_saved": saved.is_ok(),
    }))
}

// ── self-healing ──

#[derive(Debug, Clone, serde::Serialize)]
pub struct HealRow {
    pub id: String,
    /// ok | healed | not-installed | error
    pub status: &'static str,
    pub detail: Option<String>,
}

/// Health check + auto re-registration for every enabled MCP agent.
/// Called at startup, every 6 hours, and from GET /agents/health.
/// Schema errors are NEVER auto-fixed — they surface as `error` rows so the
/// settings window can show a manual-fix badge instead.
pub async fn run_heal(state: &AppState) -> Vec<HealRow> {
    let enabled = state.settings.lock().await.enabled_agents.clone();
    let mut rows = vec![];
    for id in &enabled {
        let Some(_spec) = SPECS.iter().find(|sp| sp.id == *id) else {
            continue; // custom REST agent — nothing to heal
        };
        if state.registry.is_registered_ok(id) {
            rows.push(HealRow { id: id.clone(), status: "ok", detail: None });
            continue;
        }
        if !state.mcp_exe.exists() {
            rows.push(HealRow {
                id: id.clone(),
                status: "error",
                detail: Some(format!("Access-MCP.exe not found at {}", state.mcp_exe.display())),
            });
            continue;
        }
        match state.registry.register(id, &state.mcp_exe) {
            Ok(Outcome::Registered { .. }) => {
                rows.push(HealRow { id: id.clone(), status: "healed", detail: None })
            }
            Ok(Outcome::NotInstalled) => {
                rows.push(HealRow { id: id.clone(), status: "not-installed", detail: None })
            }
            Ok(_) => rows.push(HealRow { id: id.clone(), status: "ok", detail: None }),
            Err(e) => rows.push(HealRow {
                id: id.clone(),
                status: "error",
                detail: Some(e.to_string()),
            }),
        }
    }
    rows
}

/// GET /agents/health — run the self-heal pass now and report.
pub async fn agents_health(State(s): State<AppState>) -> Json<Value> {
    let rows = run_heal(&s).await;
    Json(serde_json::json!({ "checked": rows.len(), "results": rows }))
}
