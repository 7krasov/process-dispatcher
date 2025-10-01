use formatted_logger::LineLogger;
use log::{error, info, warn};
use process_dispatcher::dispatcher::Dispatcher;
use process_dispatcher::http_server::start_http_server;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tokio::signal::unix::SignalKind;
use tokio::sync::broadcast;

#[tokio::main]
async fn main() {
    let env_params = process_dispatcher::env::fetch_env_params();

    //create a channel for posix signals
    let (shutdown_tx, _shutdown_rx): (broadcast::Sender<()>, broadcast::Receiver<()>) =
        broadcast::channel(1);
    let shutdown_tx_clone = shutdown_tx.clone();

    let dispatcher = Dispatcher::new(&env_params).await.unwrap();
    // let arc_dispatcher = Arc::new(RwLock::new(dispatcher));
    let arc_dispatcher = Arc::new(dispatcher);

    //handle posix signals
    tokio::task::spawn(async move {
        let mut sigterm = signal::unix::signal(SignalKind::terminate())
            .expect("failed to install signal handler");
        let mut sigint = signal::unix::signal(SignalKind::interrupt())
            .expect("failed to install signal handler");
        let mut sigquit =
            signal::unix::signal(SignalKind::quit()).expect("failed to install signal handler");

        tokio::select! {
            _ = sigterm.recv() => {
                warn!("Received SIGTERM...");
            }
            _ = sigint.recv() => {
                warn!("Received SIGINT...");
            }
            _ = sigquit.recv() => {
                warn!("Received SIGQUIT...");
            }
        }

        //sending shutdown signal to all subscribers
        shutdown_tx_clone.send(()).unwrap();
    });

    arc_dispatcher.clone().start_clean_source_locks().await;

    //prepare continuous scheduling
    let dispatcher_arc_clone = arc_dispatcher.clone();
    let shutdown_tx_for_schedule = shutdown_tx.clone();
    tokio::task::spawn(async move {
        let mut shutdown_rx = shutdown_tx_for_schedule.subscribe();
        loop {
            let result = dispatcher_arc_clone
                .prepare_schedule(&mut shutdown_rx)
                .await;
            if result.is_err() {
                let err = result.err().unwrap();
                if matches!(
                    err,
                    process_dispatcher::dispatcher::DispatcherError::TerminatingSignalReceived
                ) {
                    info!("main:schedule_thread: shutdown signal received, leaving the cycle...");
                    break;
                }
                error!("Error: {:?}", err);
            } else {
                info!("Cycle completed successfully");
            }
        }
    });

    let shutdown_for_http = shutdown_tx.subscribe();
    start_http_server(
        env_params.http_port(),
        arc_dispatcher.clone(),
        shutdown_for_http,
    )
    .await;

    info!("Application shutdown completed");
}

pub fn init_logger() {
    // let logger = JsonLogger::new(
    let logger = LineLogger::new(
        None, None,
        // Some(vec![
        //     // "sqlx::query".to_owned(),
        // ]),
    );
    // let logger = JsonLogger::new(None, None);
    log::set_boxed_logger(Box::new(logger)).unwrap();
    //log::LOG_LEVEL_NAMES
    // let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "trace".to_string());
    let log_level = std::env::var("LOG_LEVEL").unwrap_or_else(|_| "trace".to_string());
    log::set_max_level(log::LevelFilter::from_str(log_level.as_str()).unwrap());
}
