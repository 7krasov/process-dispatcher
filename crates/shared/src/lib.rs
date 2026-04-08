use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Display;

pub const REPORT_STATUS_SUCCESS: &str = "success";
pub const REPORT_STATUS_ERROR: &str = "error";

const DISPATCH_STATE_CREATED: &str = "created";
const DISPATCH_STATE_PENDING: &str = "pending";
const DISPATCH_STATE_PROCESSING: &str = "processing";
const DISPATCH_STATE_ERROR: &str = "error";
const DISPATCH_STATE_COMPLETED: &str = "completed";
const DISPATCH_STATE_FAILED: &str = "failed";

#[derive(PartialEq, Serialize, Deserialize, Clone, Debug)]
pub enum DispatchState {
    Created,
    Pending,
    Processing,
    Error,
    Completed,
    Failed,
}

impl DispatchState {
    pub fn new(value: &str) -> DispatchState {
        match value {
            DISPATCH_STATE_CREATED => DispatchState::Created,
            DISPATCH_STATE_PENDING => DispatchState::Pending,
            DISPATCH_STATE_PROCESSING => DispatchState::Processing,
            DISPATCH_STATE_ERROR => DispatchState::Error,
            DISPATCH_STATE_COMPLETED => DispatchState::Completed,
            DISPATCH_STATE_FAILED => DispatchState::Failed,
            _ => panic!("Unexpected DispatchState value"),
        }
    }

    pub fn is_finished(&self) -> bool {
        matches!(self, DispatchState::Completed | DispatchState::Failed)
    }
}

impl Display for DispatchState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatchState::Created => write!(f, "{}", DISPATCH_STATE_CREATED),
            DispatchState::Pending => write!(f, "{}", DISPATCH_STATE_PENDING),
            DispatchState::Processing => write!(f, "{}", DISPATCH_STATE_PROCESSING),
            DispatchState::Error => write!(f, "{}", DISPATCH_STATE_ERROR),
            DispatchState::Completed => write!(f, "{}", DISPATCH_STATE_COMPLETED),
            DispatchState::Failed => write!(f, "{}", DISPATCH_STATE_FAILED),
        }
    }
}

const PROCESSING_MODE_REGULAR: isize = 1;
const PROCESSING_MODE_SANDBOX: isize = 2;

#[derive(PartialEq, Serialize, Deserialize, Debug)]
pub enum ProcessingMode {
    Regular = PROCESSING_MODE_REGULAR,
    Sandbox = PROCESSING_MODE_SANDBOX,
}

impl ProcessingMode {
    pub fn new(value: isize) -> ProcessingMode {
        match value {
            PROCESSING_MODE_REGULAR => ProcessingMode::Regular,
            PROCESSING_MODE_SANDBOX => ProcessingMode::Sandbox,
            _ => panic!("Unexpected ProcessingMode value"),
        }
    }
}

impl From<ProcessingMode> for u8 {
    fn from(processing_mode: ProcessingMode) -> Self {
        processing_mode as u8
    }
}

impl Display for ProcessingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProcessingMode::Regular => write!(f, "Regular"),
            ProcessingMode::Sandbox => write!(f, "Sandbox"),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AssignedProcess {
    pub id: String,
    pub source_id: u32,
    pub state: DispatchState,
    #[serde(rename = "mode")]
    pub r#mode: ProcessingMode,
    #[serde(with = "chrono::serde::ts_milliseconds")]
    pub created_at: DateTime<Utc>,
    pub supervisor_id: String,
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
