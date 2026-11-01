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

    // Connection self-healing (P1): re-register the Access MCP entry in any
    // enabled agent whose config lost it (e.g. the tool updated and rewrote
    // its config). Runs at startup and every 6 hours. Schema errors are
    // reported, never auto-fixed.
    let heal_state = state.clone();
    tokio::spawn(async move {
        loop {
            for row in api_server::run_heal(&heal_state).await {
                match row.status {
                    "healed" => println!(
                        "[self-heal] {} MCP 재등록 완료 (도구 업데이트로 해제된 것으로 추정)",
                        row.id
                    ),
                    "error" => eprintln!("[self-heal] {} 실패: {}", row.id, row.detail.unwrap_or_default()),
                    _ => {}
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(6 * 60 * 60)).await;
        }
    });

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
