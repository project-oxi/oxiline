//! App state shared across Tauri commands: a pooled SQLite connection.

use oxiline_core::{open_and_migrate, paths};
use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use std::path::PathBuf;

pub type DbPool = Pool<SqliteConnectionManager>;

pub struct AppState {
    pub pool: DbPool,
    pub db_path: PathBuf,
}

impl AppState {
    /// Create + migrate the DB file once, then build a connection pool whose
    /// connections apply the required PRAGMAs (`03-data-model.md` §3.9).
    pub fn new() -> Self {
        let path = paths::db_path();
        // Ensure the file exists and is migrated before pooling.
        if let Err(e) = open_and_migrate(&path) {
            eprintln!("oxiline: initial migrate failed: {e}");
        }
        let manager = SqliteConnectionManager::file(&path).with_init(|c| {
            c.execute_batch(
                "PRAGMA journal_mode=WAL;\
                 PRAGMA synchronous=NORMAL;\
                 PRAGMA foreign_keys=ON;\
                 PRAGMA busy_timeout=5000;",
            )
        });
        let pool = Pool::builder()
            .max_size(8)
            .build(manager)
            .expect("failed to build DB pool");
        Self { pool, db_path: path }
    }

    /// Borrow a pooled connection. Panics only if the pool is exhausted, which
    /// should not happen for a single-user local app.
    pub fn conn(&self) -> r2d2::PooledConnection<SqliteConnectionManager> {
        self.pool
            .get()
            .expect("DB pool exhausted; increase max_size")
    }
}
