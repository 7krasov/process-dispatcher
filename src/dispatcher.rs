use super::db_repository::DbRepository;
use chrono::{DateTime, NaiveDateTime, Utc};
use chrono_tz::Tz;
use core::fmt;
use futures::stream::TryStreamExt;
use sqlx::Row;
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
        println!("Preparing schedule...");

        let mut source_ids_to_process = self.db_repository.available_source_ids_stream().await?;
        while let Some(row) = source_ids_to_process.try_next().await? {
            let source_id: u32 = row.try_get("id").expect("unexpected source id result");
            println!("Processing source id: {}...", source_id);

            //searching for potential not finished processes
            let process = self.db_repository.get_latest_process_for(source_id).await?;
            if process.is_some() {
                let process = process.unwrap();
                // let state: String = process.try_get("state").expect("unexpected state result");
                //why Vec<u8>? see https://github.com/launchbadge/sqlx/issues/3387
                let state: Vec<u8> = process
                    .try_get("state")
                    .expect("Unexpected 'state' result value from DB");
                let state = String::from_utf8(state).expect("unexpected state result");
                
                //if not finished
                if ![
                    DispatchState::Completed.to_string(),
                    DispatchState::Failed.to_string(),
                ]
                .contains(&state)
                {
                    println!(
                        "There is already present a completed or failed process for source id: {}",
                        source_id
                    );
                    continue;
                }
                //Completed, Failed

                let created_at_string: String = process
                    .try_get("created_at")
                    .expect("unexpected 'created_at' result");

                let created_at = DispatchTimeFormatter::db_to_dt(&created_at_string);
                let now = DispatchTimeFormatter::now_dt();

                if now.date_naive() == created_at.date_naive() {
                    println!(
                        "There is already present a finished/failed process for today for source id: {} and date: {}",
                        source_id, now.date_naive()
                    );
                    continue;
                }
            }

            self.db_repository
                .insert_new_process(source_id, DispatchState::Created.to_string())
                .await?;
        }
        Ok(())
    }
}

pub enum DispatchState {
    Created,
    // Pending,
    // Processing,
    Completed,
    Failed,
}

impl fmt::Display for DispatchState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            DispatchState::Created => write!(f, "created"),
            // DispatchState::Pending => write!(f, "pending"),
            // DispatchState::Processing => write!(f, "processing"),
            DispatchState::Completed => write!(f, "completed"),
            DispatchState::Failed => write!(f, "failed"),
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
