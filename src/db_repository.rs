use crate::dispatcher::{DispatchState, ProcessingMode};
use crate::env::EnvParams;
use futures::Stream;
use sqlx::types::Uuid;
use sqlx::{mysql::MySqlPoolOptions, MySqlPool};

pub struct DbRepository {
    pd_connection_pool: MySqlPool,
    mvp_connection_pool: MySqlPool,
}

impl DbRepository {
    pub async fn new(env_params: &EnvParams) -> Result<DbRepository, sqlx::Error> {
        let pd_connection_pool = MySqlPoolOptions::new()
            .max_connections(env_params.max_db_connections())
            .connect(env_params.pd_db_url())
            .await?;

        let mvp_connection_pool = MySqlPoolOptions::new()
            .max_connections(env_params.max_db_connections())
            .connect(env_params.mvp_db_url())
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
        processing_mode: ProcessingMode,
    ) -> Result<Uuid, sqlx::Error> {
        let uuid_val = Uuid::new_v4();

        let query = sqlx::query(
            "INSERT INTO dispatcher_processes (uuid, source_id, state, mode) VALUES (?, ?, ?, ?)",
        )
        .bind(uuid_val)
        .bind(source_id)
        .bind(state.to_string())
        .bind(u8::from(processing_mode));

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

    pub async fn get_available_source_processes_stream(
        &self,
        source_id: u32,
        limit: u32,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>>,
        sqlx::Error,
    > {
        let query = sqlx::query(
            "SELECT * FROM dispatcher_processes WHERE source_id = ? AND state IN (?, ?) ORDER BY created_at ASC LIMIT ?",
        )
            .bind(source_id)
            .bind(DispatchState::Created.to_string())
            .bind(DispatchState::Pending.to_string())
            .bind(limit);

        let processes_stream: std::pin::Pin<
            Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>,
        > = query.fetch(&self.mvp_connection_pool);

        Ok(processes_stream)
    }

    pub async fn get_available_processes_sources_stream(
        &self,
        supervisor_id: Uuid,
        limit: u32,
    ) -> Result<
        std::pin::Pin<Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>>,
        sqlx::Error,
    > {
        let query = sqlx::query(
            "SELECT source_id FROM dispatcher_processes
                 WHERE (state IN (?, ?) AND supervisor_id IS NULL) OR 
                       (state = ? AND supervisor_id IS ?)
                 ORDER BY created_at ASC LIMIT ?",
        )
        .bind(DispatchState::Created.to_string())
        .bind(DispatchState::Pending.to_string())
        .bind(DispatchState::Error.to_string())
        .bind(supervisor_id)
        .bind(limit);

        let processes_stream: std::pin::Pin<
            Box<dyn Stream<Item = Result<sqlx::mysql::MySqlRow, sqlx::Error>> + Send>,
        > = query.fetch(&self.mvp_connection_pool);

        Ok(processes_stream)
    }

    pub async fn assign_process_to_supervisor(
        &self,
        id: Uuid,
        supervisor_id: Uuid,
        assigned_state: DispatchState,
    ) -> Result<(), sqlx::Error> {
        let query = sqlx::query(
            "UPDATE dispatcher_processes SET supervisor_id = ?, state = ? WHERE uuid = ?",
        )
        .bind(supervisor_id)
        .bind(assigned_state.to_string())
        .bind(id);

        query.execute(&self.pd_connection_pool).await?;
        Ok(())
    }
}
