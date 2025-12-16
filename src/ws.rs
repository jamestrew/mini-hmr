use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use axum_extra::TypedHeader;
use notify_debouncer_full::DebouncedEvent;
use serde::Serialize;
use std::{
    net::SocketAddr,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::broadcast;

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum HMRPayload {
    Connected,
    #[allow(dead_code)]
    Ping,
    Update {
        updates: Vec<UpdatePayload>,
    },
    #[allow(dead_code)]
    FullReload,
    #[allow(dead_code)]
    Error,
}

#[derive(Debug, Serialize)]
enum UpdateType {
    JsUpdate,
    CssUpdate,
}

#[derive(Debug, Serialize)]
struct UpdatePayload {
    #[serde(rename = "type")]
    type_: UpdateType,
    path: PathBuf,
    timestamp: u64,
}

#[axum::debug_handler]
pub async fn ws_handler(
    State(tx): State<broadcast::Sender<Vec<DebouncedEvent>>>,
    ws: WebSocketUpgrade,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    let user_agent = if let Some(TypedHeader(user_agent)) = user_agent {
        user_agent.to_string()
    } else {
        String::from("Unknown browser")
    };

    tracing::debug!("`{user_agent}` at {addr} connected.");
    let rx = tx.subscribe();
    ws.on_upgrade(move |socket| handle_socket(socket, rx, addr))
}

async fn handle_socket(
    mut socket: WebSocket,
    mut rx: broadcast::Receiver<Vec<DebouncedEvent>>,
    addr: SocketAddr,
) {
    if send_payload(&mut socket, &HMRPayload::Connected)
        .await
        .is_err()
    {
        tracing::debug!("client at {addr} disconnected");
        return;
    }

    while let Ok(events) = rx.recv().await {
        let updates = events
            .iter()
            .flat_map(|event| event.paths.iter())
            .map(|path| UpdatePayload {
                type_: match path.extension().and_then(|ext| ext.to_str()) {
                    Some("css") => UpdateType::CssUpdate,
                    _ => UpdateType::JsUpdate,
                },
                path: path.to_path_buf(),
                timestamp: now_ms(),
            })
            .collect::<Vec<_>>();

        if send_payload(&mut socket, &HMRPayload::Update { updates })
            .await
            .is_err()
        {
            tracing::debug!("client at {addr} disconnected");
            return;
        }
    }
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

async fn send_payload(socket: &mut WebSocket, payload: &HMRPayload) -> Result<(), ()> {
    let text = serde_json::to_string(payload).map_err(|err| {
        tracing::error!("failed to serialize HMRPayload: {err}");
    })?;

    socket
        .send(Message::Text(text.into()))
        .await
        .map_err(|_| ())
}
