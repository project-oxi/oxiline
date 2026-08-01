//! Small shared helpers: UUID v7 ids, UTC ISO-8601 timestamps, time conversion.

use chrono::Utc;

/// Generate a time-sortable UUID v7 string (`id` column convention).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Current UTC time as an ISO-8601 `YYYY-MM-DDTHH:MM:SSZ` string.
pub fn now_iso() -> String {
    Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

/// Today's date in the local timezone as `YYYY-MM-DD`.
pub fn today_local() -> String {
    chrono::Local::now().format("%Y-%m-%d").to_string()
}

/// Format an arbitrary local NaiveDate as `YYYY-MM-DD`.
pub fn fmt_date(d: chrono::NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// Parse a `YYYY-MM-DD` string into a `NaiveDate`.
pub fn parse_date(s: &str) -> Result<chrono::NaiveDate, chrono::format::ParseError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
}

/// Current minute-of-day (0..1439) in the local timezone.
pub fn now_minute_local() -> u16 {
    let l = chrono::Local::now();
    let h: u16 = l.format("%H").to_string().parse().unwrap_or(0);
    let m: u16 = l.format("%M").to_string().parse().unwrap_or(0);
    (h * 60 + m).min(1439)
}

/// Convert `HH:MM` (24h) to a minute-of-day.
pub fn hhmm_to_minute(s: &str) -> Option<u16> {
    let (h, m) = s.split_once(':')?;
    let h: u16 = h.trim().parse().ok()?;
    let m: u16 = m.trim().parse().ok()?;
    if h > 23 || m > 59 {
        return None;
    }
    Some(h * 60 + m)
}

/// Convert a minute-of-day to `HH:MM`.
pub fn minute_to_hhmm(min: u16) -> String {
    let (h, m) = (min / 60, min % 60);
    format!("{h:02}:{m:02}")
}

/// ISO weekday as a 0-indexed mask bit position (Mon=0 … Sun=6), matching
/// `weekday_mask` bit ordering.
pub fn weekday_mask_bit(d: chrono::Weekday) -> u8 {
    use chrono::Weekday::*;
    match d {
        Mon => 0,
        Tue => 1,
        Wed => 2,
        Thu => 3,
        Fri => 4,
        Sat => 5,
        Sun => 6,
    }
}

/// Add N days to a `YYYY-MM-DD` string, returning the resulting `YYYY-MM-DD`.
pub fn add_days(date_str: &str, days: i64) -> Option<String> {
    let d = parse_date(date_str).ok()?;
    Some(fmt_date(d + chrono::Duration::days(days)))
}

/// Resolve a relative date keyword (`today`/`tomorrow`/`yesterday`) to a date.
/// Returns `None` for anything else — natural-language parsing is the agent's
/// job, not the CLI's (`05-cli-spec.md` §5.1).
pub fn resolve_date_keyword(keyword: &str) -> Option<String> {
    let today = chrono::Local::now().date_naive();
    match keyword.to_ascii_lowercase().as_str() {
        "today" => Some(fmt_date(today)),
        "tomorrow" => Some(fmt_date(today + chrono::Duration::days(1))),
        "yesterday" => Some(fmt_date(today - chrono::Duration::days(1))),
        _ => None,
    }
}

/// Round a duration in seconds up/down to the nearest multiple of
/// `increment_minutes`. Half-up: the midpoint rounds up. `increment_minutes == 0`
/// disables rounding and returns the input unchanged. 0 always stays 0.
pub fn round_duration(seconds: u64, increment_minutes: u32) -> u64 {
    if increment_minutes == 0 {
        return seconds;
    }
    let step = u64::from(increment_minutes) * 60;
    let q = (seconds + step / 2) / step; // half-up
    q * step
}
