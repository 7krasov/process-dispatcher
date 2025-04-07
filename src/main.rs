use dispatcher::Dispatcher;
use formatted_logger::LineLogger;
use log::{error, info};
use std::str::FromStr;
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;

mod db_repository;
mod dispatcher;

#[tokio::main]
async fn main() {
    let dispatcher = Dispatcher::new().await.unwrap();
    let arc_dispatcher = Arc::new(Mutex::new(dispatcher));
    let dispatcher_arc_clone = Arc::clone(&arc_dispatcher);

    let _ = tokio::task::spawn(async move {
        loop {
            let dispatcher_guard = dispatcher_arc_clone.lock().await;
            let result = dispatcher_guard.prepare_schedule().await;
            if result.is_err() {
                error!("Error: {:?}", result.err());
            } else {
                info!("Cycle completed successfully");
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
    .await;
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
