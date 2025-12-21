use axum::{Router, routing::get};
use notify::{RecommendedWatcher, RecursiveMode, event::CreateKind};
use notify_debouncer_full::{
    DebounceEventHandler, DebouncedEvent, Debouncer, RecommendedCache, new_debouncer,
};
use std::{net::SocketAddr, path::Path, time::Duration};
use tokio::sync::broadcast;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::{DefaultMakeSpan, TraceLayer},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod ws;

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

    // Hold on to the debouncer guard so the watcher thread keeps running.
    let _debouncer = watch_files(Path::new("assets"), tx.clone());

    let app = Router::new()
        .route_service("/", ServeFile::new("assets/index.html"))
        .route_service("/hmr-client.js", ServeFile::new("client/dist/index.js"))
        .nest_service("/assets", ServeDir::new("assets/"))
        .route("/ws", get(ws::ws_handler))
        .with_state(tx)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        );

    serve(app, 3307).await;
}

struct Watcher(broadcast::Sender<Vec<DebouncedEvent>>);

impl Watcher {
    const EXTENSIONS: [&'static str; 5] = ["html", "css", "js", "ts", "json"];

    fn new(tx: broadcast::Sender<Vec<DebouncedEvent>>) -> Self {
        Self(tx)
    }

    fn filter_valid_ft(&self, event: &DebouncedEvent) -> bool {
        event.paths.iter().any(|path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext_str| Self::EXTENSIONS.contains(&ext_str))
        })
    }
}

impl DebounceEventHandler for Watcher {
    fn handle_event(&mut self, event: notify_debouncer_full::DebounceEventResult) {
        for ev in event.unwrap().iter().filter(|ev| self.filter_valid_ft(ev)) {
            match &ev.kind {
                notify::EventKind::Create(create_kind) => {
                    if matches!(create_kind, CreateKind::File) {
                        tracing::debug!("file created: {:?}", ev.paths);
                        let _ = self.0.send(vec![ev.clone()]);
                    }
                }
                notify::EventKind::Modify(_modify_kind) => todo!(),
                notify::EventKind::Remove(_remove_kind) => {
                    let _ = self.0.send(vec![ev.clone()]);
                }
                _ => {}
            }
        }
    }
}

fn watch_files(
    path: &Path,
    tx: broadcast::Sender<Vec<DebouncedEvent>>,
) -> Debouncer<RecommendedWatcher, RecommendedCache> {
    let watcher = Watcher::new(tx);
    let mut debouncer = new_debouncer(Duration::from_millis(200), None, watcher).unwrap();
    debouncer.watch(path, RecursiveMode::Recursive).unwrap();
    tracing::info!("watching {:?} for changes", path);
    debouncer
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

