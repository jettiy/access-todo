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
    // Pull initial state from the Gist on startup.
    if let Err(e) = state.pull("api-server").await {
        eprintln!("warn: initial Gist pull failed: {e}");
    } else if state.sync.is_some() {
        println!("Synced from Gist on startup.");
    }

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
