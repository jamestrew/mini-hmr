use axum::{
    Router,
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::IntoResponse,
    routing::get,
};
use axum_extra::TypedHeader;
use notify::RecursiveMode;
use notify_debouncer_full::{DebouncedEvent, new_debouncer};
use std::{net::SocketAddr, path::Path, time::Duration};
use tokio::sync::broadcast;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                format!("{}=debug,tower_http=debug", env!("CARGO_CRATE_NAME")).into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, _rx) = broadcast::channel::<Vec<DebouncedEvent>>(100);

    tokio::task::spawn_blocking({
        let tx = tx.clone();
        move || watch_files(Path::new("assets"), tx)
    });

    let app = Router::new()
        .route_service("/", ServeFile::new("assets/index.html"))
        .route_service("/hmr-client.js", ServeFile::new("hmr-client/dist/index.js"))
        .nest_service("/assets", ServeDir::new("assets/"))
        .route("/ws", get(ws_handler))
        .with_state(tx)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    serve(app, 3307).await;
}

fn watch_files(path: &Path, tx: broadcast::Sender<Vec<DebouncedEvent>>) {
    let (notify_tx, notify_rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, notify_tx).unwrap();
    debouncer.watch(path, RecursiveMode::Recursive).unwrap();

    tracing::info!("watching {:?} for changes", path);

    for result in notify_rx {
        match result {
            Ok(events) => {
                for event in &events {
                    tracing::info!(kind = ?event.kind, paths = ?event.paths, "file change");
                }
                let _ = tx.send(events);
            }
            Err(errors) => {
                tracing::error!(?errors, "file watch error");
            }
        }
    }
}

#[axum::debug_handler]
async fn ws_handler(
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
            if socket
                .send(Message::Text(
                    format!(
                        "File change detected: {:?} at paths: {:?}",
                        event.kind, event.paths
                    )
                    .into(),
                ))
                .await
                .is_err()
            {
                tracing::debug!("client at {addr} disconnected");
                return;
            };
        }
    }
}

async fn serve(app: Router, port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(
        listener,
        app.layer(TraceLayer::new_for_http())
            .into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap();
}
