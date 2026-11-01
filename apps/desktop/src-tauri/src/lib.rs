// Tauri app: spawns one post-it window per agent on startup.
// Window positions are saved/restored automatically via tauri-plugin-window-state.
//
// ORDER-004 (2026-11-01): every HTTP call to the api-server goes through the
// Rust `api_call` command and one Rust-owned /events subscription is fanned
// out as the `todos-changed` Tauri event — the webviews keep zero direct
// sockets to :7878, so window count can never exhaust WebView2's per-host
// connection limit ("Failed to fetch" root cause, 2026-10-18 실측 7창 12연결).
// The frontend opts in at build time with VITE_USE_PROXY=1 (default off).

use serde_json::Value;
use tauri::{Emitter, WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_window_state::StateFlags;

/// Each entry: (window label, display title, default x, default y, background color)
const AGENT_WINDOWS: &[(&str, &str, f64, f64, &str)] = &[
    ("hermes", "🤖 Hermes", 50.0, 50.0, "#ffd0e8"),
    ("omp", "🛠️ OMP", 390.0, 50.0, "#c8e0f5"),
    ("zcode", "⚡ ZCode", 50.0, 560.0, "#c8f0e0"),
    ("user", "👤 내 할 일", 390.0, 560.0, "#fff0a0"),
    ("devnotes", "📝 게임 개발노트", 730.0, 50.0, "#c8f0e0"),
    ("codex", "🧠 Codex", 730.0, 560.0, "#e0d0f5"),
    ("dsh", "🐋 DSH", 1070.0, 50.0, "#ffd8c8"),
];

fn api_base() -> String {
    std::env::var("ACCESS_API_BASE").unwrap_or_else(|_| "http://127.0.0.1:7878".into())
}

/// Proxy one HTTP call to the local api-server on behalf of a webview.
/// Uses reqwest (outside the browser socket pool); the webview's `agent`
/// is forwarded as the X-Agent header so attribution keeps working.
#[tauri::command]
async fn api_call(
    agent: String,
    method: String,
    path: String,
    body: Option<Value>,
) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let m = reqwest::Method::from_bytes(method.to_uppercase().as_bytes())
        .map_err(|e| e.to_string())?;
    let mut req = client
        .request(m, format!("{}{}", api_base(), path))
        .header("X-Agent", &agent);
    if let Some(b) = &body {
        req = req.json(b);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("{status} {text}"));
    }
    if text.trim().is_empty() {
        return Ok(Value::Null);
    }
    Ok(serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

/// The single /events (SSE) subscription for the whole app. Every `changed`
/// event is re-emitted as a Tauri event; webviews listen instead of opening
/// their own EventSource. Reconnects with a short backoff while the
/// api-server is down so windows recover without user action.
async fn sse_bridge(app: tauri::AppHandle) {
    let client = reqwest::Client::new();
    loop {
        match client.get(format!("{}/events", api_base())).send().await {
            Ok(resp) if resp.status().is_success() => {
                use futures_util::StreamExt;
                let mut stream = resp.bytes_stream();
                // SSE chunks may split mid-line; carry partial lines over.
                let mut carry = String::new();
                loop {
                    match stream.next().await {
                        Some(Ok(chunk)) => {
                            carry.push_str(&String::from_utf8_lossy(&chunk));
                            while let Some(pos) = carry.find('\n') {
                                let line: String = carry.drain(..=pos).collect();
                                // axum SSE data lines look like "data: changed";
                                // keep-alive comments start with ':'.
                                if let Some(data) = line.trim().strip_prefix("data:") {
                                    if data.trim() == "changed" {
                                        let _ = app.emit("todos-changed", ());
                                    }
                                }
                            }
                        }
                        Some(Err(_)) | None => break,
                    }
                }
            }
            _ => {}
        }
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_window_state::Builder::new()
                .with_state_flags(
                    StateFlags::POSITION | StateFlags::SIZE | StateFlags::MAXIMIZED,
                )
                .build(),
        )
        .invoke_handler(tauri::generate_handler![api_call])
        .setup(|app| {
            for (label, title, x, y, _bg) in AGENT_WINDOWS {
                let url = WebviewUrl::App(format!("index.html?agent={label}").into());
                // 개발노트 창은 더 크게, 일반 포스트잇은 작게
                let (w, h) = if *label == "devnotes" { (380.0, 600.0) } else { (320.0, 480.0) };
                WebviewWindowBuilder::new(app, *label, url)
                    .title(*title)
                    .inner_size(w, h)
                    .position(*x, *y)
                    .decorations(false)
                    .always_on_top(*label != "devnotes") // 개발노트 창은 기본적으로 핀 해제
                    .transparent(false)
                    .resizable(true)
                    .skip_taskbar(false)
                    .build()?;
            }
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(sse_bridge(handle));
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
