use futures::Stream;
use sqlx::types::Uuid;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};
use std::env;

pub struct DbRepository {
    pd_connection_pool: MySqlPool,
    mvp_connection_pool: MySqlPool,
}

impl DbRepository {
    pub async fn new() -> Result<DbRepository, sqlx::Error> {
        let db_url = env::var("PD_DATABASE_URL");
        if db_url.is_err() {
            panic!("PD_DATABASE_URL is not set");
        }
        let pd_connection_pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(db_url.unwrap().as_str())
            .await?;

        let db_url = env::var("MVP_DATABASE_URL");
        if db_url.is_err() {
            panic!("MVP_DATABASE_URL is not set");
        }
        let mvp_connection_pool = MySqlPoolOptions::new()
            .max_connections(2)
            .connect(db_url.unwrap().as_str())
            .await?;

        Ok(DbRepository {
            pd_connection_pool,
            mvp_connection_pool,
        })
    }

    pub async fn available_source_ids_stream(
        &mut self,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>>,
        sqlx::Error,
    > {
        let source_ids_to_process: std::pin::Pin<
            Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>,
        > = sqlx::query("SELECT id FROM sources").fetch(&self.mvp_connection_pool);
        Ok(source_ids_to_process)
    }

    pub async fn insert_new_process(
        &mut self,
        source_id: u32,
        state: String,
    ) -> Result<(), sqlx::Error> {
        let uuid_val = Uuid::new_v4();

        let query = sqlx::query(
            "INSERT INTO dispatcher_processes (uuid, source_id, state) VALUES (?, ?, ?)",
        )
        .bind(uuid_val)
        .bind(source_id)
        .bind(state);

        // println!("{:?}", String::from(query.sql()));

        query.execute(&self.pd_connection_pool).await?;
        // dbg!(query.execute(&self.pd_connection_pool).await?);
        Ok(())
    }

    pub async fn active_processes_stream_for_source<'a>(
        &mut self,
        source_id: u32,
        states_to_exclude: Vec<String>,
        query_string: &'a mut String,
    ) -> Result<
        std::pin::Pin<
            Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send + 'a>,
        >,
        sqlx::Error,
    > {
        // Convert Vec<String> to a single comma-separated String with each element wrapped in single quotes
        let states_to_exclude_str = &states_to_exclude
            .iter()
            .map(|s| format!("'{}'", s))
            .collect::<Vec<_>>()
            .join(",");

        // // Construct the full SQL query string
        *query_string = format!(
            "SELECT * FROM dispatcher_processes WHERE source_id = {} AND state NOT IN ({})",
            source_id, states_to_exclude_str
        );
        println!("{:?}", query_string);

        let query = sqlx::query(query_string);

        // let mut builder: QueryBuilder<'_, MySql> = QueryBuilder::new(
        //     "SELECT * FROM dispatcher_processes WHERE source_id = ? AND state NOT IN ("
        // );
        // builder.push_bind(source_id);
        // let mut separated = builder.separated(", ");
        // for state in states_to_exclude {
        //     separated.push_bind(state);
        // }
        // separated.push_unseparated(")");
        // let query = builder.build();

        // let query = sqlx::query(
        //     "SELECT * FROM dispatcher_processes WHERE source_id = ? AND state NOT IN (?)",
        // )
        // .bind(source_id)
        // .bind(states_to_exclude_str);

        let processes = query.fetch(&self.pd_connection_pool);
        Ok(processes)
    }

    pub async fn get_latest_process_for(
        &mut self,
        source_id: u32,
    ) -> Result<Option<sqlx::mysql::MySqlRow>, sqlx::Error> {
        let query = sqlx::query("SELECT * FROM dispatcher_processes WHERE source_id = ? ORDER BY created_at DESC LIMIT 1")
            .bind(source_id);
        let process = query.fetch_optional(&self.pd_connection_pool).await?;
        Ok(process)
    }
}
