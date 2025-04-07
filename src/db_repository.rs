use crate::dispatcher::{DispatchState, ProcessingType};
use futures::Stream;
use sqlx::types::Uuid;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::env;

const ENV_PD_DATABASE_URL: &str = "PD_DATABASE_URL";
const ENV_MVP_DATABASE_URL: &str = "MVP_DATABASE_URL";

pub struct DbRepository {
    pd_connection_pool: MySqlPool,
    mvp_connection_pool: MySqlPool,
}

impl DbRepository {
    pub async fn new() -> Result<DbRepository, sqlx::Error> {
        let db_url = env::var(ENV_PD_DATABASE_URL);
        if db_url.is_err() {
            panic!("{}", format!("{} is not set", ENV_PD_DATABASE_URL));
        }
        let pd_connection_pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(&db_url.unwrap())
            .await?;

        let db_url = env::var(ENV_MVP_DATABASE_URL);
        if db_url.is_err() {
            panic!("{}", format!("{} is not set", ENV_MVP_DATABASE_URL));
        }
        let mvp_connection_pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(&db_url.unwrap())
            .await?;

        Ok(DbRepository {
            pd_connection_pool,
            mvp_connection_pool,
        })
    }

    pub async fn available_source_ids_stream(
        &self,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>>,
        sqlx::Error,
    > {
        let source_ids_to_process: std::pin::Pin<
            Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>,
        > = sqlx::query("SELECT id FROM sources where status = 'run'")
            .fetch(&self.mvp_connection_pool);
        Ok(source_ids_to_process)
    }

    pub async fn insert_new_process(
        &self,
        source_id: u32,
        state: DispatchState,
        processing_type: ProcessingType,
    ) -> Result<Uuid, sqlx::Error> {
        let uuid_val = Uuid::new_v4();

        let query = sqlx::query(
            "INSERT INTO dispatcher_processes (uuid, source_id, state, type) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid_val)
        .bind(source_id)
        .bind(state.to_string())
        .bind(u8::from(processing_type));

        // println!("{:?}", String::from(query.sql()));

        query.execute(&self.pd_connection_pool).await?;

        // dbg!(query.execute(&self.pd_connection_pool).await?);
        Ok(uuid_val)
    }

    pub async fn get_latest_process_for(
        &self,
        source_id: u32,
    ) -> Result<Option<sqlx::mysql::MySqlRow>, sqlx::Error> {
        let query = sqlx::query("SELECT * FROM dispatcher_processes WHERE source_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(source_id);
        let process = query.fetch_optional(&self.pd_connection_pool).await?;
        Ok(process)
    }
}
