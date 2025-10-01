mod route_handlers;

use crate::dispatcher::Dispatcher;
use axum::routing::post;
use axum::Router;
use log::{info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Clone)]
struct AppState {
    dispatcher: Arc<Dispatcher>,
}

pub async fn start_http_server(
    http_port: u16,
    dispatcher: Arc<Dispatcher>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    let router = Router::new()
        .route(
            "/assign_process/{supervisor_id}",
            post(route_handlers::assign_process_handler),
        )
        .with_state(Arc::new(AppState { dispatcher }));
    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    println!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, router)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            warn!("HTTP server received shutdown signal, initiating graceful shutdown")
        })
        .await
        .unwrap();
    info!("HTTP server shutdown completed");
}
