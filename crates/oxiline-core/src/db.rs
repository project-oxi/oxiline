//! SQLite connection setup, PRAGMAs, and migrations.
//!
//! The single source of schema truth — both `oxiline-app` and `oxiline-cli`
//! call [`open_and_migrate`] so migration logic can never drift between them
//! (`03-data-model.md` §3.10).

use crate::error::{CoreError, Result};
use rusqlite::Connection;
use rusqlite_migration::{M, Migrations};

/// The initial migration, embedded at compile time.
const V1_INIT: &str = include_str!("../migrations/V1__init.sql");
const V2_PHASE2: &str = include_str!("../migrations/V2__phase2.sql");

fn migrations() -> Migrations<'static> {
    Migrations::new(vec![M::up(V1_INIT), M::up(V2_PHASE2)])
}

/// Apply PRAGMAs required for concurrent GUI+CLI access
/// (`03-data-model.md` §3.9).
fn apply_pragmas(conn: &Connection) -> Result<()> {
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "busy_timeout", 5000_i64)?;
    Ok(())
}

/// Open (creating if needed) and migrate the OxiLine database at `path`.
///
/// PRAGMAs are applied before migrations so WAL is active during the initial
/// create.
pub fn open_and_migrate(path: &std::path::Path) -> Result<Connection> {
    let mut conn =
        Connection::open(path).map_err(|e| CoreError::Internal(format!("open db: {e}")))?;
    apply_pragmas(&conn)?;
    migrations().to_latest(&mut conn).map_err(CoreError::from)?;
    Ok(conn)
}

/// Current schema version (count of applied migrations) for `doctor`.
pub fn schema_version(conn: &Connection) -> Result<usize> {
    Ok(migrations().current_version(conn)?.into())
}
