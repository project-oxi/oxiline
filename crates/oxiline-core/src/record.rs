//! Recording lifecycle: start, stop, and inspect the single active session.

use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};

use crate::error::{CoreError, Result};
use crate::model::{ActiveSession, Record, RecordState};
use crate::util::{new_id, round_duration};

fn timestamp(now: DateTime<Utc>) -> String {
    now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn row_from(row: &rusqlite::Row<'_>) -> rusqlite::Result<Record> {
    Ok(Record {
        id: row.get("id")?,
        activity_id: row.get("activity_id")?,
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        note: row.get("note")?,
    })
}

pub fn start(
    conn: &Connection,
    activity_id: &str,
    now: DateTime<Utc>,
    today: &str,
) -> Result<RecordState> {
    let now_text = timestamp(now);
    let tx = conn.unchecked_transaction()?;
    tx.execute(
        "UPDATE records SET ended_at = ?1, updated_at = ?1 WHERE ended_at IS NULL",
        params![now_text],
    )?;
    tx.execute(
        "INSERT INTO records
             (id, activity_id, started_at, ended_at, note, created_at, updated_at)
         VALUES (?1, ?2, ?3, NULL, NULL, ?3, ?3)",
        params![new_id(), activity_id, now_text],
    )?;
    tx.commit()?;
    current(conn, now, today)
}

pub fn stop(conn: &Connection, now: DateTime<Utc>, _today: &str) -> Result<RecordState> {
    let now_text = timestamp(now);
    conn.execute(
        "UPDATE records SET ended_at = ?1, updated_at = ?1 WHERE ended_at IS NULL",
        params![now_text],
    )?;
    Ok(RecordState {
        active: None,
        today: vec![],
        generated_at: now_text,
    })
}

pub fn current(conn: &Connection, now: DateTime<Utc>, _today: &str) -> Result<RecordState> {
    let now_text = timestamp(now);
    let mut stmt = conn.prepare(
        "SELECT id, activity_id, started_at, ended_at, note
         FROM records WHERE ended_at IS NULL
         ORDER BY started_at DESC, id DESC",
    )?;
    let records = stmt
        .query_map([], row_from)?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    drop(stmt);

    let record = records.first().cloned();
    if records.len() > 1 {
        for stale in &records[1..] {
            conn.execute(
                "UPDATE records
                 SET ended_at = CASE WHEN started_at >= ?1
                                     THEN strftime('%Y-%m-%dT%H:%M:%SZ', started_at, '+1 second')
                                     ELSE ?1 END,
                     updated_at = ?1
                 WHERE id = ?2 AND ended_at IS NULL",
                params![now_text, stale.id],
            )?;
        }
    }

    let active = if let Some(record) = record {
        let activity = crate::activities::get_activity(conn, &record.activity_id)?;
        let started_at = DateTime::parse_from_rfc3339(&record.started_at)
            .map_err(|e| CoreError::Internal(format!("invalid record started_at: {e}")))?
            .with_timezone(&Utc);
        let elapsed = (now - started_at).num_seconds().max(0) as u64;
        let increment = crate::settings::get_i64(conn, "record_rounding_minutes", 5).max(0) as u32;
        Some(ActiveSession {
            record,
            activity,
            elapsed_seconds: round_duration(elapsed, increment),
        })
    } else {
        None
    };

    Ok(RecordState {
        active,
        today: vec![],
        generated_at: now_text,
    })
}
