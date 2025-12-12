use axum::Router;
use notify::RecursiveMode;
use notify_debouncer_full::new_debouncer;
use std::{net::SocketAddr, path::Path, time::Duration};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
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

    let app = Router::new()
        .route_service("/", ServeFile::new("assets/index.html"))
        .nest_service("/assets", ServeDir::new("assets/"));

    tokio::task::spawn_blocking(|| watch_files(Path::new("assets")));

    serve(app, 3307).await;
}

fn watch_files(path: &Path) {
    let (tx, rx) = std::sync::mpsc::channel();
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, tx).unwrap();
    debouncer.watch(path, RecursiveMode::Recursive).unwrap();

    tracing::info!("watching {:?} for changes", path);

    for result in rx {
        for event in result.unwrap() {
            tracing::info!(kind = ?event.kind, paths = ?event.paths, "file change");
        }
    }
}

async fn serve(app: Router, port: u16) {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on http://{}", listener.local_addr().unwrap());
    axum::serve(listener, app.layer(TraceLayer::new_for_http()))
        .await
        .unwrap();
}
