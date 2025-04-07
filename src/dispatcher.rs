use super::db_repository::DbRepository;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use futures::stream::TryStreamExt;
use log::{info, trace};
use sqlx::Row;
use std::fmt::Display;
use std::str::FromStr;

const TIMEZONE: &str = "Europe/Berlin";

pub struct Dispatcher {
    db_repository: DbRepository,
}

impl Dispatcher {
    pub async fn new() -> Result<Dispatcher, sqlx::Error> {
        let db_repository = DbRepository::new().await?;
        Ok(Dispatcher { db_repository })
    }

    pub async fn prepare_schedule(&self) -> Result<(), sqlx::Error> {
        info!("Preparing schedule...");

        let mut source_ids_to_process = self.db_repository.available_source_ids_stream().await?;
        while let Some(row) = source_ids_to_process.try_next().await? {
            let source_id: u32 = row.try_get("id").expect("unexpected source id result");
            trace!("Processing source id: {}...", source_id);

            //searching for potential not finished processes
            let process = self.db_repository.get_latest_process_for(source_id).await?;
            if process.is_some() {
                let process = process.unwrap();
                // let state: String = process.try_get("state").expect("unexpected state result");
                //why Vec<u8>? see https://github.com/launchbadge/sqlx/issues/3387
                // let state: Vec<u8> = process
                //     .try_get("state")
                //     .expect("Unexpected 'state' result value from DB");
                // let state = String::from_utf8(state).expect("unexpected state result");
                let state: &str = process
                    .try_get("state")
                    .expect("Unexpected 'state' result value from DB");
                let state = DispatchState::new(state);

                //not: Completed, Failed
                if !state.is_finished() {
                    trace!(
                        "There is already present process in state {} for source id: {}",
                        state,
                        source_id
                    );
                    continue;
                }

                // let processing_type: u8 = process
                //     .try_get("type")
                //     .expect("Unexpected 'type' result value from DB");
                // let processing_type = ProcessingType::new(processing_type as isize);

                let created_at_string: String = process
                    .try_get("created_at")
                    .expect("unexpected 'created_at' result");

                let created_at = DispatchTimeFormatter::db_to_dt(&created_at_string);
                let now = DispatchTimeFormatter::now_dt();

                if now.date_naive() == created_at.date_naive() {
                    trace!(
                        "There is already present a finished/failed process for today for source id: {} and date: {}",
                        source_id, now.date_naive()
                    );
                    continue;
                }
            }

            let uuid = self
                .db_repository
                .insert_new_process(source_id, DispatchState::Created, ProcessingType::Regular)
                .await?;

            info!(
                "A new regular process {} for source id: {} has been created",
                uuid, source_id
            );
        }
        Ok(())
    }
}

const DISPATCH_STATE_CREATED: &str = "created";
const DISPATCH_STATE_PENDING: &str = "pending";
const DISPATCH_STATE_PROCESSING: &str = "processing";
const DISPATCH_STATE_COMPLETED: &str = "completed";
const DISPATCH_STATE_FAILED: &str = "failed";

#[derive(PartialEq)]
pub enum DispatchState {
    Created,
    Pending,
    Processing,
    Completed,
    Failed,
}

impl DispatchState {
    pub fn new(value: &str) -> DispatchState {
        // match value.as_str() {
        match value {
            DISPATCH_STATE_CREATED => DispatchState::Created,
            DISPATCH_STATE_PENDING => DispatchState::Pending,
            DISPATCH_STATE_PROCESSING => DispatchState::Processing,
            DISPATCH_STATE_COMPLETED => DispatchState::Completed,
            DISPATCH_STATE_FAILED => DispatchState::Failed,
            _ => panic!("Unexpected DispatchState value"),
        }
    }

    pub fn is_finished(&self) -> bool {
        //if ![DispatchState::Completed, DispatchState::Failed].contains(&self) {}
        //
        // match self {
        //     DispatchState::Completed | DispatchState::Failed => true,
        //     _ => false,
        // }
        matches!(self, DispatchState::Completed | DispatchState::Failed)
    }
}

impl Display for DispatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchState::Created => write!(f, "{}", DISPATCH_STATE_CREATED),
            DispatchState::Pending => write!(f, "{}", DISPATCH_STATE_PENDING),
            DispatchState::Processing => write!(f, "{}", DISPATCH_STATE_PROCESSING),
            DispatchState::Completed => write!(f, "{}", DISPATCH_STATE_COMPLETED),
            DispatchState::Failed => write!(f, "{}", DISPATCH_STATE_FAILED),
        }
    }
}

struct DispatchTimeFormatter;

impl DispatchTimeFormatter {
    pub fn db_to_dt(db_datetime: &str) -> DateTime<Tz> {
        let datetime_format = "%Y-%m-%d %H:%M:%S%.f"; // Format for MySQL TIMESTAMP(3)
        let created_at_utc = NaiveDateTime::parse_from_str(db_datetime, datetime_format)
            .expect("Failed to parse datetime");
        DateTime::<Utc>::from_naive_utc_and_offset(created_at_utc, Utc)
            .with_timezone(&Self::timezone())
    }

    pub fn now_dt() -> DateTime<Tz> {
        let utc_now = Utc::now();
        utc_now.with_timezone(&Self::timezone())
    }

    fn timezone() -> Tz {
        Tz::from_str(TIMEZONE).expect("invalid timezone")
    }
}

const PROCESSING_TYPE_REGULAR: isize = 1;
const PROCESSING_TYPE_SANDBOX: isize = 2;

#[derive(PartialEq)]
pub enum ProcessingType {
    Regular = PROCESSING_TYPE_REGULAR,
    Sandbox = PROCESSING_TYPE_SANDBOX,
}

impl ProcessingType {
    pub fn new(value: isize) -> ProcessingType {
        match value {
            PROCESSING_TYPE_REGULAR => ProcessingType::Regular,
            PROCESSING_TYPE_SANDBOX => ProcessingType::Sandbox,
            _ => panic!("Unexpected ProcessingType value"),
        }
    }
}

impl From<ProcessingType> for u8 {
    fn from(processing_type: ProcessingType) -> Self {
        processing_type as u8
    }
}
