use formatted_logger::LineLogger;
use log::{error, info};
use process_dispatcher::dispatcher::Dispatcher;
use process_dispatcher::http_server::start_http_server;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let env_params = process_dispatcher::env::fetch_env_params();
    let dispatcher = Dispatcher::new(&env_params).await.unwrap();
    // let arc_dispatcher = Arc::new(RwLock::new(dispatcher));
    let arc_dispatcher = Arc::new(dispatcher);

    arc_dispatcher.clone().start_clean_source_locks().await;

    //continuously prepare the schedule
    let dispatcher_arc_clone = arc_dispatcher.clone();
    let _ = tokio::task::spawn(async move {
        loop {
            let result = dispatcher_arc_clone.prepare_schedule().await;
            if result.is_err() {
                error!("Error: {:?}", result.err());
            } else {
                info!("Cycle completed successfully");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
    .await;

    start_http_server(env_params.http_port(), arc_dispatcher.clone()).await;
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
