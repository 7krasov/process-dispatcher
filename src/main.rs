use dispatcher::Dispatcher;

mod dispatcher;

#[tokio::main]
async fn main() {
    let mut dispatcher = Dispatcher::new().await.unwrap();
    dispatcher.prepare_schedule().await;
}
