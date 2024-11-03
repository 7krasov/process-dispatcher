use std::{sync::Arc, time::Duration};

use dispatcher::Dispatcher;
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
            let mut dispatcher_guard = dispatcher_arc_clone.lock().await;
            let result = dispatcher_guard.prepare_schedule().await;
            print!("Result: {:?}", result);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    })
    .await;
}
