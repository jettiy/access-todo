//! Entry point: bind the router to 127.0.0.1 and serve.
//! Uses a Gist-backed store when GITHUB_TOKEN + TODO_GIST_ID are set.

use std::net::SocketAddr;

use api_server::{router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("TODO_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7878);

    let state = AppState::with_gist_from_env();
    // Restore from local cache first (covers no-Gist and Gist-unreachable
    // cases). If a Gist is configured, the pull below will overwrite with
    // the canonical remote state.
    if let Err(e) = state.load_local_cache().await {
        eprintln!("warn: local cache load failed: {e}");
    }
    // Pull initial state from the Gist on startup (if configured).
    if let Err(e) = state.pull("api-server").await {
        eprintln!("warn: initial Gist pull failed: {e}");
    } else if state.sync.is_some() {
        println!("Synced from Gist on startup.");
    }
    // Start the debounced Gist flusher (no-op if neither Gist nor cache).
    state.clone().spawn_flusher();
    let app = router(state);
    // 기본은 127.0.0.1 (localhost 전용). TODO_BIND=0.0.0.0 으로 WSL/네트워크 접근 허용.
    let host: std::net::IpAddr = std::env::var("TODO_BIND")
        .unwrap_or_else(|_| "127.0.0.1".into())
        .parse()
        .unwrap_or("127.0.0.1".parse().unwrap());
    let addr = SocketAddr::from((host, port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("api-server listening on http://{addr}");
    println!("  Gist sync: {}", if std::env::var("GITHUB_TOKEN").is_ok() { "ON" } else { "OFF (in-memory only)" });
    axum::serve(listener, app).await?;
    Ok(())
}
