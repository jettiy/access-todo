// Tauri app: spawns one post-it window per agent on startup.

use tauri::{WebviewUrl, WebviewWindowBuilder};

/// Each entry: (window label, display title, x, y, background color)
const AGENT_WINDOWS: &[(&str, &str, f64, f64, &str)] = &[
    ("hermes", "🤖 Hermes", 50.0, 50.0, "#ffd0e8"),
    ("omp", "🛠️ OMP", 390.0, 50.0, "#c8e0f5"),
    ("zcode", "⚡ ZCode", 50.0, 560.0, "#c8f0e0"),
    ("user", "👤 내 할 일", 390.0, 560.0, "#fff0a0"),
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            for (label, title, x, y, bg) in AGENT_WINDOWS {
                let url = WebviewUrl::App(format!("index.html?agent={label}").into());
                WebviewWindowBuilder::new(app, *label, url)
                    .title(*title)
                    .inner_size(320.0, 480.0)
                    .position(*x, *y)
                    .decorations(false)
                    .always_on_top(true)
                    .transparent(false)
                    .resizable(true)
                    .skip_taskbar(true)
                    .build()?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
