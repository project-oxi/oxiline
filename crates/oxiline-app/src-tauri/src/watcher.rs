//! DB file watcher. When the CLI (or anything) writes to the SQLite file, the
//! GUI re-renders via a Tauri event — the file is the only shared truth
//! (`04-architecture.md` §4.5).

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use parking_lot::Mutex;
use tauri::{AppHandle, Emitter};

const DEBOUNCE_MS: u128 = 150;

pub fn spawn(app: AppHandle) {
    let db_path: PathBuf = oxiline_core::paths::db_path();
    std::thread::spawn(move || {
        let last = Arc::new(Mutex::new(Instant::now()));
        let app_for_cb = app.clone();

        let mut watcher = match RecommendedWatcher::new(
            move |res: Result<notify::Event, notify::Error>| {
                let Ok(ev) = res else { return };
                let interesting = matches!(
                    ev.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                );
                if !interesting {
                    return;
                }
                let now = Instant::now();
                let mut guard = last.lock();
                if now.duration_since(*guard).as_millis() < DEBOUNCE_MS {
                    return;
                }
                *guard = now;
                drop(guard);
                let _ = app_for_cb.emit("oxiline://db-changed", ());
            },
            notify::Config::default(),
        ) {
            Ok(w) => w,
            Err(e) => {
                eprintln!("oxiline: watcher init failed: {e}");
                return;
            }
        };

        // Watch the DB file and its WAL sibling.
        {
            let mut p = db_path.clone();
            let _ = watcher.watch(&p, RecursiveMode::NonRecursive);
            p.set_extension("db-wal");
            if p.exists() {
                let _ = watcher.watch(&p, RecursiveMode::NonRecursive);
            }
        }
        // Keep the watcher alive for the app lifetime.
        loop {
            std::thread::sleep(std::time::Duration::from_secs(3600));
        }
    });
}
