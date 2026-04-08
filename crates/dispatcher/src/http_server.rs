mod route_handlers;

use crate::cancellation_ext::{CancellationError, CancellationExt};
use crate::dispatcher::Dispatcher;
use axum::routing::post;
use axum::Router;
use log::{info, warn};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct AppState {
    dispatcher: Arc<Dispatcher>,
}

#[derive(Debug)]
enum HttpServerError {
    Cancelled,
    Io(std::io::Error),
}

impl From<CancellationError> for HttpServerError {
    fn from(_: CancellationError) -> Self {
        HttpServerError::Cancelled
    }
}

impl From<std::io::Error> for HttpServerError {
    fn from(e: std::io::Error) -> Self {
        HttpServerError::Io(e)
    }
}

pub async fn start_http_server(
    http_port: u16,
    dispatcher: Arc<Dispatcher>,
    cancellation_token: &CancellationToken,
) {
    let router = Router::new()
        .route(
            "/assign_process/{supervisor_id}",
            post(route_handlers::assign_process_handler),
        )
        .with_state(Arc::new(AppState { dispatcher }));
    let addr = SocketAddr::from(([0, 0, 0, 0], http_port));
    println!("listening on {}", addr);

    let listener = match tokio::net::TcpListener::bind(addr)
        .with_cancellation::<HttpServerError>(cancellation_token, "http_server:tcp_bind")
        .await
    {
        Ok(l) => l,
        Err(HttpServerError::Cancelled) => {
            info!("HTTP server bind cancelled");
            return;
        }
        Err(HttpServerError::Io(e)) => {
            warn!("Failed to bind HTTP server: {}", e);
            return;
        }
    };

    let shutdown_token = cancellation_token.clone();
    let shutdown_future = async move {
        shutdown_token.cancelled().await;
        warn!("HTTP server received cancellation signal, initiating graceful shutdown");
    };

    if let Err(e) = axum::serve(listener, router)
        .with_graceful_shutdown(shutdown_future)
        .await
    {
        warn!("HTTP server serve error: {}", e);
    }

    info!("HTTP server shutdown completed");
}
