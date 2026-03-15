use anyhow::Result;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::WebSocket;
use axum::{Router, routing::get};

#[tokio::main]
async fn main() -> Result<()> {
    let app = Router::new().route("/", get(ws_handler));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    axum::serve(listener, app).await?;
    Ok(())
}

use axum::response::IntoResponse;

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    if let Some(Ok(msg)) = socket.recv().await {
        let _ = socket.send(msg).await;
    }
}
