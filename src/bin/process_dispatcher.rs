use formatted_logger::LineLogger;
use log::{error, info, warn};
use process_dispatcher::dispatcher::Dispatcher;
use process_dispatcher::http_server::start_http_server;
use std::str::FromStr;
use std::sync::Arc;
use tokio::signal;
use tokio::signal::unix::SignalKind;
use tokio_util::sync::CancellationToken;

#[tokio::main]
async fn main() {
    //init env variables
    let env_params = process_dispatcher::env::fetch_env_params();

    //prepare a mechanism for shutdown event processing
    let cancellation_token = prepare_cancellation_token_on_posix_signal();

    let dispatcher = Dispatcher::new(&env_params).await.unwrap();
    let arc_dispatcher = Arc::new(dispatcher);

    //use cleaning of the lock mechanism for source ids
    arc_dispatcher.clone().start_clean_source_locks().await;

    //prepare continuous scheduling of processes
    let dispatcher_arc_clone = arc_dispatcher.clone();
    let cancellation_token_clone = cancellation_token.clone();
    tokio::task::spawn(async move {
        loop {
            match dispatcher_arc_clone
                .prepare_schedule(&cancellation_token_clone)
                .await
            {
                Ok(_) => info!("Cycle completed successfully"),
                Err(process_dispatcher::dispatcher::DispatcherError::TerminatingSignalReceived) => {
                    info!("main:schedule_thread: Schedule preparation cancelled");
                    break;
                }
                Err(e) => error!("Error: {:?}", e),
            }
        }
    });

    start_http_server(
        env_params.http_port(),
        arc_dispatcher.clone(),
        &cancellation_token,
    )
    .await;

    info!("Application shutdown completed");
}

fn prepare_cancellation_token_on_posix_signal() -> CancellationToken {
    let cancellation_token = CancellationToken::new();

    //handle posix signals
    let cancellation_token_clone = cancellation_token.clone();
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

        //notify all features that use cancellation token, so they will cancel their work
        cancellation_token_clone.cancel();
    });
    cancellation_token
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
