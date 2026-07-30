//! Error type shared across the core crate.

use thiserror::Error;

/// Stable, machine-readable error codes (mirror the CLI error-code table in
/// `05-cli-spec.md` §5.4) so callers can map a `CoreError` to an exit code /
/// JSON error code without pattern-matching on messages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    NotFound,
    InvalidArgument,
    AmbiguousCategory,
    DbLocked,
    DbMigrationFailed,
    Internal,
}

impl ErrorCode {
    pub fn as_str(self) -> &'static str {
        match self {
            ErrorCode::NotFound => "not_found",
            ErrorCode::InvalidArgument => "invalid_argument",
            ErrorCode::AmbiguousCategory => "ambiguous_category",
            ErrorCode::DbLocked => "db_locked",
            ErrorCode::DbMigrationFailed => "db_migration_failed",
            ErrorCode::Internal => "internal",
        }
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("ambiguous category: {0}")]
    AmbiguousCategory(String),
    #[error("database is locked: {0}")]
    DbLocked(String),
    #[error("database migration failed: {0}")]
    DbMigrationFailed(String),
    #[error("{0}")]
    Internal(String),
}

impl CoreError {
    pub fn code(&self) -> ErrorCode {
        match self {
            CoreError::NotFound(_) => ErrorCode::NotFound,
            CoreError::InvalidArgument(_) => ErrorCode::InvalidArgument,
            CoreError::AmbiguousCategory(_) => ErrorCode::AmbiguousCategory,
            CoreError::DbLocked(_) => ErrorCode::DbLocked,
            CoreError::DbMigrationFailed(_) => ErrorCode::DbMigrationFailed,
            CoreError::Internal(_) => ErrorCode::Internal,
        }
    }

    /// Map to a process exit code per `05-cli-spec.md` §5.5.
    pub fn exit_code(&self) -> i32 {
        match self {
            CoreError::InvalidArgument(_) => 2,
            CoreError::NotFound(_) => 3,
            CoreError::DbLocked(_) => 4,
            CoreError::DbMigrationFailed(_) => 5,
            _ => 1,
        }
    }
}

impl From<rusqlite::Error> for CoreError {
    fn from(err: rusqlite::Error) -> Self {
        use rusqlite::ErrorCode;
        match err {
            rusqlite::Error::SqliteFailure(ffi_err, _) => match ffi_err.code {
                ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked => {
                    CoreError::DbLocked(err.to_string())
                }
                _ => CoreError::Internal(err.to_string()),
            },
            rusqlite::Error::QueryReturnedNoRows => CoreError::NotFound("row not found".into()),
            _ => CoreError::Internal(err.to_string()),
        }
    }
}

impl From<rusqlite_migration::Error> for CoreError {
    fn from(err: rusqlite_migration::Error) -> Self {
        CoreError::DbMigrationFailed(err.to_string())
    }
}

pub type Result<T, E = CoreError> = std::result::Result<T, E>;
