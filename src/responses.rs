use crate::dispatcher::{DispatchState, ProcessingMode};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct AssignedProcess {
    id: String,
    source_id: u32,
    state: DispatchState,
    #[serde(rename = "mode")]
    r#mode: ProcessingMode,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    created_at: DateTime<Utc>,
    supervisor_id: String,
}

impl AssignedProcess {
    pub fn new(
        id: String,
        source_id: u32,
        state: DispatchState,
        r#mode: ProcessingMode,
        created_at: DateTime<Utc>,
        supervisor_id: String,
    ) -> Self {
        AssignedProcess {
            id,
            source_id,
            state,
            r#mode,
            created_at,
            supervisor_id,
        }
    }
}
