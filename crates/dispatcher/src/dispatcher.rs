mod error;

use super::db_repository::DbRepository;
use crate::async_keyed_mutex::AsyncKeyedMutex;
use crate::cancellation_ext::CancellationExt;
use crate::env::EnvParams;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use chrono_tz::Tz::UTC;
pub use error::DispatcherError;
use futures::stream::TryStreamExt;
use shared::{
    AssignedProcess, DispatchState, ProcessingMode, REPORT_STATUS_ERROR, REPORT_STATUS_SUCCESS,
};
use sqlx::mysql::MySqlRow;
use sqlx::Row;
use std::str::FromStr;
use tracing::{error, info, trace};

/// Workaround for sqlx treating VARCHAR columns as VARBINARY under utf8mb4_bin collation.
trait MySqlRowExt {
    fn get_string(&self, column: &str) -> String;
}

impl MySqlRowExt for MySqlRow {
    fn get_string(&self, column: &str) -> String {
        let bytes: Vec<u8> = self
            .try_get(column)
            .unwrap_or_else(|e| panic!("Unexpected '{column}' result from DB: {e}"));
        String::from_utf8(bytes).unwrap_or_else(|_| panic!("Invalid UTF-8 in '{column}'"))
    }
}
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

const TIMEZONE: &str = "Europe/Berlin";

pub struct Dispatcher {
    db_repository: DbRepository,
    source_locks: Arc<AsyncKeyedMutex<u32, tokio::sync::Mutex<()>>>,
    //TODO: move cancellation_token here and use as dispatcher property
}

impl Dispatcher {
    pub async fn new(env_params: &EnvParams) -> Result<Dispatcher, sqlx::Error> {
        let db_repository = DbRepository::new(env_params).await?;
        let source_locks = Arc::new(AsyncKeyedMutex::<u32>::new());
        Ok(Dispatcher {
            db_repository,
            source_locks,
        })
    }

    pub fn start_clean_source_locks(&self) {
        let source_locks = self.source_locks.clone();
        tokio::task::spawn(async move {
            info!("Cleaning locks...");
            loop {
                source_locks.cleanup();
                tokio::time::sleep(Duration::from_secs(30)).await;
            }
        });
    }

    pub async fn prepare_schedule(
        &self,
        cancellation_token: &CancellationToken,
    ) -> Result<u16, DispatcherError> {
        info!("Preparing schedule...");

        let mut created_cnt: u16 = 0;
        //requesting a stream (sending a request to DB without waiting for the response)
        let mut source_ids_to_process = self
            .db_repository
            .available_source_ids_stream()
            .with_cancellation::<DispatcherError>(
                cancellation_token,
                "prepare_schedule:stream_creation",
            )
            .await?;

        //fetching result rows from the stream
        while let Some(row) = source_ids_to_process
            .try_next()
            .with_cancellation::<DispatcherError>(
                cancellation_token,
                "prepare_schedule:stream_processing",
            )
            .await?
        {
            let source_id: u32 = row.try_get("id").expect("unexpected source id result");
            trace!("Processing source id: {}...", source_id);

            let lock = self.source_locks.get_mutex(source_id);
            let res = self.process_source(source_id, cancellation_token).await;
            if let Err(e) = res {
                error!("Error processing source id {}: {}", source_id, e);
            } else {
                created_cnt += res.unwrap();
            }
            drop(lock);
        }

        Ok(created_cnt)
    }

    async fn process_source(
        &self,
        source_id: u32,
        cancellation_token: &CancellationToken,
    ) -> Result<u16, DispatcherError> {
        //searching for potential not finished processes
        let process = self
            .db_repository
            .get_latest_process_for(source_id)
            .with_cancellation::<DispatcherError>(
                cancellation_token,
                "process_source:get_latest_process",
            )
            .await?;
        if process.is_some() {
            let process = process.unwrap();
            let state = DispatchState::new(&process.get_string("state"));

            //not: Completed, Failed
            if !state.is_finished() {
                trace!(
                    "There is already present process in state {} for source id: {}",
                    state,
                    source_id
                );
                return Ok(0);
            }

            let created_at =
                DispatchTimeFormatter::db_to_dt(&process.get_string("created_at"), None);
            let now = DispatchTimeFormatter::now_dt();

            if now.date_naive() == created_at.date_naive() {
                trace!(
                        "There is already present a finished/failed process for today for source id: {} and date: {}",
                        source_id, now.date_naive()
                    );
                return Ok(0);
            }
        }

        let uuid = self
            .db_repository
            .insert_new_process(source_id, DispatchState::Created, ProcessingMode::Regular)
            .with_cancellation::<DispatcherError>(
                cancellation_token,
                "process_source:insert_new_process",
            )
            .await?;

        info!(
            "A new regular process {} for source id: {} has been created",
            uuid, source_id
        );
        Ok(1)
    }

    pub async fn assign_process(
        &self,
        supervisor_id: Uuid,
    ) -> Result<Option<AssignedProcess>, sqlx::Error> {
        info!("Searching for process to assigning...");
        //get list of source ids that have active processes in DB
        let mut sources_stream = self
            .db_repository
            .get_available_processes_sources_stream(supervisor_id, 10)
            .await?;

        loop {
            let row_option = sources_stream.try_next().await?;
            if row_option.is_none() {
                info!("No available source ids found for assigning.");
                return Ok(None);
            }
            let process_row = row_option.unwrap();
            let source_id: u32 = process_row
                .try_get("source_id")
                .expect("unexpected source id result");

            //lock any DB operations while we process with the current source
            let lock = self.source_locks.get_mutex(source_id);

            //get available processes for the current source
            let mut processes_stream = self
                .db_repository
                .get_available_source_processes_stream(source_id, 1)
                .await?;

            loop {
                let row_option = processes_stream.try_next().await?;
                if row_option.is_none() {
                    info!(
                        "No available processes found for source {} to assigning.",
                        source_id
                    );
                    drop(lock);
                    break;
                }
                //we have a new non-assigned process
                let process_row = row_option.unwrap();
                let process_id: Uuid = process_row.try_get("uuid").expect("unexpected uuid result");
                let supervisor_id_option: Option<Vec<u8>> = process_row
                    .try_get("supervisor_id")
                    .expect("unexpected supervisor id result");
                let state = DispatchState::new(&process_row.get_string("state"));
                let processing_mode: u8 = process_row
                    .try_get("mode")
                    .expect("Unexpected 'mode' result value from DB");
                let processing_mode = ProcessingMode::new(processing_mode as isize);
                let created_at_string = process_row.get_string("created_at");
                let created_at = DispatchTimeFormatter::db_to_dt(&created_at_string, Some(UTC));

                //we should get only active and unassigned process
                if !state.is_finished() && supervisor_id_option.is_none() {
                    info!(
                        "Assigning process {} for source id: {} with state: {} and processing type: {} in DB...",
                        process_id, source_id, state, processing_mode
                    );
                    let new_state = DispatchState::Processing;
                    self.db_repository
                        .assign_process_to_supervisor(process_id, supervisor_id, new_state.clone())
                        .await?;

                    let assigned_process = AssignedProcess::new(
                        process_id.into(),
                        source_id,
                        new_state,
                        processing_mode,
                        created_at.to_utc(),
                        supervisor_id.into(),
                    );
                    return Ok(Some(assigned_process));
                }
            }
        }
    }

    pub async fn report_process_finish(
        &self,
        process_id: Uuid,
        result: &str,
    ) -> Result<(), ReportFinishError> {
        let new_state = match result {
            REPORT_STATUS_SUCCESS => DispatchState::Completed,
            REPORT_STATUS_ERROR => DispatchState::Failed,
            other => return Err(ReportFinishError::InvalidResult(other.to_owned())),
        };

        info!(
            %process_id,
            state = %new_state,
            "Reporting process finish..."
        );

        let rows_affected = self
            .db_repository
            .update_process_state(process_id, new_state)
            .await
            .map_err(ReportFinishError::Db)?;

        if rows_affected == 0 {
            return Err(ReportFinishError::NotFound(process_id));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub enum ReportFinishError {
    InvalidResult(String),
    NotFound(Uuid),
    Db(sqlx::Error),
}

impl std::fmt::Display for ReportFinishError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReportFinishError::InvalidResult(v) => write!(f, "invalid result value '{}'", v),
            ReportFinishError::NotFound(id) => write!(f, "process {} not found", id),
            ReportFinishError::Db(e) => write!(f, "db error: {}", e),
        }
    }
}

struct DispatchTimeFormatter;

impl DispatchTimeFormatter {
    pub fn db_to_dt(db_datetime: &str, timezone: Option<Tz>) -> DateTime<Tz> {
        let datetime_format = "%Y-%m-%d %H:%M:%S%.f"; // Format for MySQL TIMESTAMP(3)
        let created_at_utc = NaiveDateTime::parse_from_str(db_datetime, datetime_format)
            .expect("Failed to parse datetime");
        DateTime::<Utc>::from_naive_utc_and_offset(created_at_utc, Utc)
            .with_timezone(&timezone.unwrap_or(Self::timezone()))
    }

    pub fn now_dt() -> DateTime<Tz> {
        let utc_now = Utc::now();
        utc_now.with_timezone(&Self::timezone())
    }

    fn timezone() -> Tz {
        Tz::from_str(TIMEZONE).expect("invalid timezone")
    }
}
