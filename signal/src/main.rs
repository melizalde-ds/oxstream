use anyhow::Result;
use axum::extract::WebSocketUpgrade;
use axum::extract::ws::{Message, WebSocket};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use futures_util::{sink::SinkExt, stream::StreamExt};
use tracing::info;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    info!("Starting Signalling server...");
    let app = Router::new().route("/", get(ws_handler));
    info!("Router configured successfully");

    info!("Opening TCP listener on 127.0.0.1:8080");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("TCP listener opened successfully on 127.0.0.1:8080");

    info!("Starting Axum server...");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(32);

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let tx2 = tx.clone();

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let _ = tx2.send(msg).await;
        }
    });

    tokio::select! {
        _ = &mut send_task => recv_task.abort(),
        _ = &mut recv_task => send_task.abort(),
    }
}

fn init_tracing() {
    tracing_subscriber::fmt::init();
}
