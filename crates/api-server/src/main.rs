//! Entry point: bind the router to 127.0.0.1 and serve.

use std::net::SocketAddr;

use api_server::{router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let port: u16 = std::env::var("TODO_API_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(7878);
    let app = router(AppState::new());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    println!("api-server listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
