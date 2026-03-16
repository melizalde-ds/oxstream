use std::sync::Arc;

use anyhow::Result;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::{Router, routing::get};
use dashmap::DashMap;
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tracing::info;

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SignalMessage {
    SetPeerStatus(PeerStatusMessage),
    List,
    ListResponse(ListResponseMessage),
    StartSession(StartSessionMessage),
    EndSession(EndSessionMessage),
    Peer(PeerMessage),
    Error(ErrorMessage),
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerStatusMessage {
    pub peer_id: String,
    pub roles: Vec<PeerRole>,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum PeerRole {
    Producer,
    Consumer,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponseMessage {
    pub producers: Vec<ProducerInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProducerInfo {
    pub id: String,
    pub meta: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartSessionMessage {
    pub peer_id: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EndSessionMessage {
    pub session_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PeerMessage {
    pub session_id: String,
    pub sdp: Option<SdpPayload>,
    pub ice: Option<IcePayload>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SdpPayload {
    #[serde(rename = "type")]
    pub sdp_type: String,
    pub sdp: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IcePayload {
    pub candidate: String,
    pub sdp_m_line_index: u32,
    pub sdp_mid: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorMessage {
    pub details: String,
}

pub type Tx = mpsc::Sender<Message>;

pub struct PeerEntry {
    pub role: PeerRole,
    pub tx: Tx,
    pub meta: Option<serde_json::Value>,
}

pub struct SessionEntry {
    pub producer_id: String,
    pub consumer_id: String,
}

#[derive(Default)]
pub struct AppState {
    pub peers: DashMap<String, PeerEntry>,
    pub sessions: DashMap<String, SessionEntry>,
}

pub type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    info!("Initializing application state...");
    let state = Arc::new(AppState::default());
    info!("Application state initialized successfully");

    info!("Starting Signalling server...");
    let app = Router::new().route("/", get(ws_handler)).with_state(state);
    info!("Router configured successfully");

    info!("Opening TCP listener on 127.0.0.1:8080");
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8080").await?;
    info!("TCP listener opened successfully on 127.0.0.1:8080");

    info!("Starting Axum server...");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<SharedState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, _state: SharedState) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Message>(32);

    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            let _ = tx.send(msg).await;
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
