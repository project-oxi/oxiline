//! Settings key-value store (`03-data-model.md` §3.8). Values are JSON-encoded.

use crate::error::{CoreError, Result};
use crate::model::{TraySlotKind, TraySlotPref};
use rusqlite::{Connection, params};
use serde_json::Value;

pub fn get_raw(conn: &Connection, key: &str) -> Result<Value> {
    let s: String = conn
        .query_row(
            "SELECT value FROM settings WHERE key = ?",
            params![key],
            |r| r.get(0),
        )
        .map_err(CoreError::from)?;
    serde_json::from_str(&s)
        .map_err(|e| CoreError::Internal(format!("settings decode '{key}': {e}")))
}

pub fn get_all(conn: &Connection) -> Result<serde_json::Map<String, Value>> {
    let mut stmt = conn.prepare("SELECT key, value FROM settings")?;
    let rows = stmt.query_map([], |r| {
        let k: String = r.get(0)?;
        let v: String = r.get(1)?;
        Ok((k, v))
    })?;
    let mut map = serde_json::Map::new();
    for r in rows {
        let (k, v) = r?;
        let val: Value = serde_json::from_str(&v).unwrap_or(Value::String(v));
        map.insert(k, val);
    }
    Ok(map)
}

pub fn set(conn: &Connection, key: &str, value: &Value) -> Result<()> {
    let encoded = serde_json::to_string(value)
        .unwrap_or_else(|_| serde_json::to_string(&Value::String(value.to_string())).unwrap());
    let now = crate::util::now_iso();
    conn.execute(
        "INSERT INTO settings (key, value, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, encoded, now],
    )?;
    Ok(())
}

/// Set a key from a raw string. Attempts to parse as JSON (number/bool/null);
/// falls back to a JSON string so `set locale ko` stores `"ko"`.
pub fn set_from_str(conn: &Connection, key: &str, raw: &str) -> Result<()> {
    let value: Value = if raw == "null" {
        Value::Null
    } else if raw == "true" || raw == "false" {
        Value::Bool(raw.parse::<bool>().unwrap())
    } else if let Ok(n) = raw.parse::<i64>() {
        Value::from(n)
    } else if let Ok(f) = raw.parse::<f64>() {
        Value::from(f)
    } else {
        // Strip surrounding quotes if the caller already quoted it.
        let trimmed = raw.trim();
        if trimmed.len() >= 2 && trimmed.starts_with('"') && trimmed.ends_with('"') {
            Value::String(trimmed[1..trimmed.len() - 1].to_string())
        } else {
            Value::String(raw.to_string())
        }
    };
    set(conn, key, &value)
}

/// Typed helper: read a setting as `i64` with a default.
pub fn get_i64(conn: &Connection, key: &str, default: i64) -> i64 {
    get_raw(conn, key)
        .ok()
        .and_then(|v| v.as_i64())
        .unwrap_or(default)
}

/// Typed helper: read a setting as a string with a default.
pub fn get_string(conn: &Connection, key: &str, default: &str) -> String {
    get_raw(conn, key)
        .ok()
        .and_then(|v| v.as_str().map(str::to_string))
        .unwrap_or_else(|| default.to_string())
}

/// Typed helper: read a boolean setting with a default.
pub fn get_bool(conn: &Connection, key: &str, default: bool) -> bool {
    get_raw(conn, key)
        .ok()
        .and_then(|v| v.as_bool())
        .unwrap_or(default)
}

/// v1 default state for the tray menu-bar slots.
pub fn defaults() -> Vec<TraySlotPref> {
    vec![
        TraySlotPref {
            kind: TraySlotKind::NowRecording,
            on: true,
            order: 0,
        },
        TraySlotPref {
            kind: TraySlotKind::NowNext,
            on: true,
            order: 1,
        },
        TraySlotPref {
            kind: TraySlotKind::StateDot,
            on: false,
            order: 2,
        },
    ]
}

/// Read the persisted `tray_slots` row; falls back to `defaults()` when the
/// key is missing or unparseable.
pub fn get_tray_slots(conn: &Connection) -> Vec<TraySlotPref> {
    let value = match get_raw(conn, "tray_slots") {
        Ok(v) => v,
        Err(_) => return defaults(),
    };
    parse_tray_slots(&value).unwrap_or_else(defaults)
}

/// Persist `tray_slots` as a single JSON row under the `tray_slots` key.
pub fn save_tray_slots(conn: &Connection, prefs: &[TraySlotPref]) -> Result<()> {
    let slots: Vec<Value> = prefs
        .iter()
        .map(|p| {
            serde_json::json!({
                "id": crate::tray_slots::slot_kind_to_id(p.kind),
                "on": p.on,
                "order": p.order,
            })
        })
        .collect();
    let value = Value::Object({
        let mut map = serde_json::Map::new();
        map.insert("slots".to_string(), Value::Array(slots));
        map
    });
    set(conn, "tray_slots", &value)
}

/// Parse the persisted `tray_slots` JSON value into typed prefs. Unknown slot
/// ids are silently dropped (the renderer can handle missing kinds).
fn parse_tray_slots(value: &Value) -> Option<Vec<TraySlotPref>> {
    let slots = value.get("slots")?.as_array()?;
    let mut out = Vec::with_capacity(slots.len());
    for s in slots {
        if let Some(pref) = parse_slot(s) {
            out.push(pref);
        }
    }
    Some(out)
}

fn parse_slot(value: &Value) -> Option<TraySlotPref> {
    let id = value.get("id")?.as_str()?;
    let kind = crate::tray_slots::slot_id_to_kind(id)?;
    let on = value
        .get("on")
        .and_then(Value::as_bool)
        .or_else(|| value.get("on").and_then(Value::as_i64).map(|n| n != 0))
        .unwrap_or(true);
    let order = value
        .get("order")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0);
    Some(TraySlotPref { kind, on, order })
}

/// Ensure all known default keys exist (idempotent; called after migrate).
#[allow(dead_code)]
pub fn ensure_defaults(conn: &Connection) -> Result<()> {
    let known = [
        ("locale", Value::String("system".into())),
        ("theme", Value::String("system".into())),
        ("global_hotkey", Value::String("CmdOrCtrl+Shift+O".into())),
        ("hud_duration_ms", Value::from(2000)),
        ("day_start_hour", Value::from(5)),
        ("day_end_hour", Value::from(26)),
        ("week_starts_on", Value::String("mon".into())),
        ("launch_at_login", Value::Bool(true)),
        ("workload_warning_minutes", Value::from(600)),
    ];
    for (k, v) in known {
        if get_raw(conn, k).is_err() {
            set(conn, k, &v)?;
        }
    }
    Ok(())
}

/// Build a typed snapshot of all known settings.
pub fn snapshot(conn: &Connection) -> crate::model::SettingsSnapshot {
    use crate::model::SettingsSnapshot;
    SettingsSnapshot {
        locale: get_string(conn, "locale", "system"),
        theme: get_string(conn, "theme", "system"),
        global_hotkey: get_string(conn, "global_hotkey", "CmdOrCtrl+Shift+O"),
        hud_duration_ms: get_i64(conn, "hud_duration_ms", 2000),
        day_start_hour: get_i64(conn, "day_start_hour", 5),
        day_end_hour: get_i64(conn, "day_end_hour", 26),
        week_starts_on: get_string(conn, "week_starts_on", "mon"),
        launch_at_login: get_bool(conn, "launch_at_login", true),
        workload_warning_minutes: get_i64(conn, "workload_warning_minutes", 600),
        onboarding_done: get_bool(conn, "onboarding_done", false),
        notifications_enabled: get_bool(conn, "notifications_enabled", false),
        notification_lead_minutes: get_i64(conn, "notification_lead_minutes", 5) as u32,
        tray_slots: get_tray_slots(conn),
    }
}
