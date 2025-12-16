use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
};
use axum_extra::TypedHeader;
use notify_debouncer_full::DebouncedEvent;
use std::net::SocketAddr;
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum WsMessage {
    FileChange {
        kind: String,
        paths: Vec<String>,
    },
}

impl WsMessage {
    fn from_event(event: &DebouncedEvent) -> Self {
        Self::FileChange {
            kind: format!("{:?}", event.kind),
            paths: event.paths.iter().map(|p| p.display().to_string()).collect(),
        }
    }

    fn into_text(self) -> String {
        match self {
            Self::FileChange { kind, paths } => {
                format!("File change detected: {kind} at paths: {paths:?}")
            }
        }
    }
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
    while let Ok(events) = rx.recv().await {
        for event in &events {
            let msg = WsMessage::from_event(event);
            if socket
                .send(Message::Text(msg.into_text().into()))
                .await
                .is_err()
            {
                tracing::debug!("client at {addr} disconnected");
                return;
            };
        }
    }
}

