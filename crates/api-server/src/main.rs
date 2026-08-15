//! Entry point: bind the router and serve.
//!
//! Startup runs recovery: load the local data file, force-pull the Gist
//! (ETag bypassed), merge, then persist — so a crash or a stale Gist can
//! never wipe the working set.

use std::net::SocketAddr;

use api_server::{router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("TODO_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7878);

    let state = AppState::with_gist_from_env();

    // Recovery: local file + forced Gist pull → merge → persist.
    let (todos, cats) = state.recover().await.unwrap_or((0, 0));
    println!(
        "Recovered {} todos / {} categories (local file + Gist merge)",
        todos, cats
    );

    let gist_on = state.sync.is_some();
    let app = router(state);
    // 기본은 127.0.0.1 (localhost 전용). TODO_BIND=0.0.0.0 으로 WSL/네트워크 접근 허용.
    let host: std::net::IpAddr = std::env::var("TODO_BIND")
        .unwrap_or_else(|_| "127.0.0.1".into())
        .parse()
        .unwrap_or("127.0.0.1".parse().unwrap());
    let addr = SocketAddr::from((host, port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("api-server listening on http://{addr}");
    println!(
        "  Gist sync: {} | SSE: /events | local file: {}",
        if gist_on { "ON" } else { "OFF" },
        std::env::var("ACCESS_DATA_FILE").unwrap_or_else(|_| "<exe-dir>/access-data.json".into())
    );
    axum::serve(listener, app).await?;
    Ok(())
}
