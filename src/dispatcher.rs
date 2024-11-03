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
        let db_repository = DbRepository::new().await.unwrap();
        Ok(Dispatcher { db_repository })
    }

    pub async fn prepare_schedule(&mut self) -> Result<(), sqlx::Error> {
        println!("Preparing schedule...");

        let mut source_ids_to_process = self.db_repository.available_source_ids_stream().await?;
        while let Some(row) = source_ids_to_process.try_next().await? {
            let source_id: u32 = row.try_get("id").expect("unexpected source id result");
            println!("Processing source id: {}...", source_id);
            let utc_now = chrono::Utc::now();
            let timezone = Tz::from_str(TIMEZONE).expect("invalid timezone");
            let now_in_tz = utc_now.with_timezone(&timezone);

            //TODO: fix dirty fix
            //cannot have local dynamic sql query string within a method (which is used below),
            //as the string var is used within "processes" result which lives here out of method's local scope
            let mut dirty_fix_string_var = String::new();

            let mut processes = self
                .db_repository
                .active_processes_stream_for_source(
                    source_id,
                    vec![
                        DispatchState::Completed.to_string(),
                        DispatchState::Failed.to_string(),
                    ],
                    &mut dirty_fix_string_var,
                )
                .await?;
            let process = processes.try_next().await?;

            if process.is_some() {
                println!(
                    "An active process is already exist for source id: {}",
                    source_id
                );
                continue;
            }

            let process = self.db_repository.get_latest_process_for(source_id).await?;
            if process.is_some() {
                let created_at_string: String = process
                    .unwrap()
                    .try_get("created_at")
                    .expect("unexpected created_at result");
                let datetime_format = "%Y-%m-%d %H:%M:%S%.f"; // Format for MySQL TIMESTAMP(3)
                let created_at_utc =
                    NaiveDateTime::parse_from_str(&created_at_string, datetime_format)
                        .expect("Failed to parse datetime");
                let created_at_in_timezone =
                    DateTime::<Utc>::from_utc(created_at_utc, Utc).with_timezone(&timezone);

                if now_in_tz.date_naive() == created_at_in_timezone.date_naive() {
                    println!(
                        "There is already present a finished process for today for source id: {} and date: {}",
                        source_id, now_in_tz.date_naive()
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
