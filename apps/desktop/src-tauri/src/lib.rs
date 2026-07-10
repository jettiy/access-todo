// Tauri app: spawns one post-it window per agent on startup.
// Window positions are saved/restored automatically via tauri-plugin-window-state.

use tauri::{WebviewUrl, WebviewWindowBuilder};
use tauri_plugin_window_state::StateFlags;

/// Each entry: (window label, display title, default x, default y, background color)
const AGENT_WINDOWS: &[(&str, &str, f64, f64, &str)] = &[
    ("hermes", "🤖 Hermes", 50.0, 50.0, "#ffd0e8"),
    ("omp", "🛠️ OMP", 390.0, 50.0, "#c8e0f5"),
    ("zcode", "⚡ ZCode", 50.0, 560.0, "#c8f0e0"),
    ("user", "👤 내 할 일", 390.0, 560.0, "#fff0a0"),
    ("devnotes", "📝 게임 개발노트", 730.0, 50.0, "#c8f0e0"),
];

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
                    .always_on_top(true)
                    .transparent(false)
                    .resizable(true)
                    .skip_taskbar(false)
                    .build()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
