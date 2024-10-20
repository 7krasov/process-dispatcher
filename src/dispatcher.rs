use futures::stream::TryStreamExt;
use sqlx::mysql::MySqlConnection;
use sqlx::Connection;
use sqlx::Row;
use std::env;

pub struct Dispatcher {
    pd_connection: MySqlConnection,
}

impl Dispatcher {
    pub async fn new() -> Result<Dispatcher, sqlx::Error> {
        let db_url = env::var("PD_DATABASE_URL");
        if db_url.is_err() {
            panic!("PD_DATABASE_URL is not set");
        }
        let pd_connection = MySqlConnection::connect(db_url.unwrap().as_str()).await?;
        Ok(Dispatcher { pd_connection })
    }

    pub async fn prepare_schedule(&mut self) -> Result<(), sqlx::Error> {
        println!("Preparing schedule...");
        let mut source_ids_to_process =
            sqlx::query("SELECT id FROM sources").fetch(&mut self.pd_connection);
        while let Some(row) = source_ids_to_process.try_next().await? {
            let source_id: i32 = row.try_get("id").expect("unexpected source id result");
            println!("Processing source id: {}...", source_id);
        }

        Ok(())
    }
}
