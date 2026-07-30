//! Filesystem path resolution. Shared verbatim by GUI and CLI so both binaries
//! always agree on the SQLite file location (`04-architecture.md` §4.8).

use std::path::PathBuf;

/// Resolve the OxiLine database path.
///
/// Resolution order:
/// 1. `$OXILINE_DB_PATH` env override (used by agents/CI/sandboxed test DBs).
/// 2. `~/Library/Application Support/OxiLine/oxiline.db` on macOS.
/// 3. Platform data dir fallback via the `dirs` crate.
///
/// The directory is created if missing.
pub fn db_path() -> PathBuf {
    if let Ok(p) = std::env::var("OXILINE_DB_PATH") {
        let path = PathBuf::from(p);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        return path;
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = dirs::home_dir() {
            let dir = home.join("Library/Application Support/OxiLine");
            let _ = std::fs::create_dir_all(&dir);
            return dir.join("oxiline.db");
        }
    }

    let dir = dirs::data_dir()
        .unwrap_or_else(|| std::path::Path::new(".").to_path_buf())
        .join("OxiLine");
    let _ = std::fs::create_dir_all(&dir);
    dir.join("oxiline.db")
}

/// Default path for the per-instance lock / single-instance marker (reserved).
#[allow(dead_code)]
pub fn data_dir() -> PathBuf {
    db_path().parent().map(|p| p.to_path_buf()).unwrap_or_default()
}
