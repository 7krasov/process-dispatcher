use std::fmt::Display;

#[derive(Debug)]
pub enum DispatcherError {
    DbError(sqlx::Error),
    TerminatingSignalReceived,
}

impl Display for DispatcherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DispatcherError::DbError(e) => write!(f, "DbError: {}", e),
            DispatcherError::TerminatingSignalReceived => write!(f, "TerminatingSignalReceived"),
        }
    }
}

impl std::error::Error for DispatcherError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            DispatcherError::DbError(e) => Some(e),
            DispatcherError::TerminatingSignalReceived => None,
        }
    }
}

impl From<sqlx::Error> for DispatcherError {
    fn from(e: sqlx::Error) -> Self {
        DispatcherError::DbError(e)
    }
}
