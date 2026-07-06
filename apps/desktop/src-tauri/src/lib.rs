// Tauri app library entry. Kept minimal: all data flows through the
// local api-server over HTTP from the Svelte frontend.

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
