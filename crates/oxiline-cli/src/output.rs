//! Output rendering: human text vs. JSON (`05-cli-spec.md` §5.1).

use oxiline_core::model::{Activity, Category, NowContext, NowItem, Record, RecordState, RoutineBlock, Task, TimelineItem};
use oxiline_core::util;

use crate::lang::L;

pub fn json_pretty<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "null".into())
}

pub fn minute_range(start: Option<u16>, dur: Option<u16>) -> String {
    match (start, dur) {
        (Some(s), Some(d)) => {
            let end = (s as u32 + d as u32).min(1440);
            format!(
                "{}-{}",
                util::minute_to_hhmm(s),
                util::minute_to_hhmm(end as u16)
            )
        }
        (Some(s), None) => util::minute_to_hhmm(s),
        _ => "-".into(),
    }
}

fn line(prefix: char, title: &str, range: &str, extra: &str) -> String {
    format!("  {prefix} {title:<24} {range:<13} {extra}")
}

pub fn timeline_text(lang: L, date: &str, items: &[TimelineItem]) -> String {
    if items.is_empty() {
        return format!("{date}\n  {}", lang.empty_timeline());
    }
    let mut out = String::from(date);
    out.push('\n');
    for it in items {
        let mark = if it.is_done {
            'x'
        } else if it.is_virtual {
            '○'
        } else {
            '●'
        };
        let range = minute_range(it.start_minute, it.duration_minute);
        let virt = if it.is_virtual {
            " (루틴/가상)"
        } else {
            ""
        };
        out.push_str(&line(mark, &it.title, &range, virt));
        out.push('\n');
    }
    out
}

pub fn backlog_text(lang: L, items: &[Task]) -> String {
    let title = format!("{} ({})", lang.backlog(), items.len());
    if items.is_empty() {
        return format!("{title}\n  {}", lang.empty_backlog());
    }
    let mut out = title;
    out.push('\n');
    for t in items {
        let mark = if t.is_done { 'x' } else { '○' };
        out.push_str(&line(mark, &t.title, "", ""));
        out.push('\n');
    }
    out
}

pub fn task_text(task: &Task) -> String {
    let range = minute_range(task.start_minute, task.duration_minute);
    let status = if task.is_done {
        "[done]"
    } else if task.is_skipped {
        "[skipped]"
    } else {
        "[ ]"
    };
    format!("{status} {} {range}\n  id: {}", task.title, task.id)
}

pub fn now_text(lang: L, ctx: &NowContext) -> String {
    let fmt_now = |n: &NowItem, label: &str| -> String {
        let time = n.start_minute.map(util::minute_to_hhmm).unwrap_or_default();
        format!("  {label} · {} ({})", n.title, time)
    };
    let mut out = String::new();
    match &ctx.current {
        Some(c) => {
            out.push_str(&fmt_now(c, lang.now_label()));
            if let Some(rem) = c.remaining_minute {
                out.push_str(&format!("  · {} {}", rem, lang.min_unit()));
                out.push(' ');
                out.push_str(lang.remaining());
            }
        }
        None => match &ctx.next {
            Some(n) => {
                out.push_str(&format!("  {}\n", lang.nothing_now()));
                out.push_str(&fmt_now(n, lang.next_label()));
                if let Some(s) = n.starts_in_minute {
                    out.push_str(&format!("  · {} {} {}", lang.in_min(), s, lang.min_unit()));
                }
            }
            None => out.push_str(&format!("  {}", lang.all_done_today())),
        },
    }
    out
}

pub fn routine_text(r: &RoutineBlock) -> String {
    let days = mask_days(r.weekday_mask);
    let active = if r.is_active { "" } else { " (off)" };
    format!(
        "{}{}  {} · {}min   [{}]\n  id: {}",
        r.title,
        active,
        util::minute_to_hhmm(r.start_minute),
        r.duration_minute,
        days,
        r.id
    )
}

pub fn routine_list_text(rs: &[RoutineBlock]) -> String {
    let mut out = String::new();
    for r in rs {
        out.push_str(&routine_text(r));
        out.push('\n');
    }
    out
}

pub fn mask_days(mask: u8) -> String {
    let names = ["M", "T", "W", "T", "F", "S", "S"];
    let mut s = String::new();
    for i in 0..7u8 {
        if mask & (1 << i) != 0 {
            s.push_str(names[i as usize]);
            s.push(' ');
        } else {
            s.push_str("_ ");
        }
    }
    s.trim_end().into()
}

pub fn category_list_text(cats: &[Category]) -> String {
    let mut out = String::new();
    for c in cats {
        let builtin = if c.is_builtin { " (builtin)" } else { "" };
        out.push_str(&format!(
            "● {}  hue={:<6} id: {}{builtin}\n",
            c.name, c.color_hue, c.id
        ));
    }
    out
}
pub fn activity_text(a: &Activity, inactive_label: &str) -> String {
    let off = if a.is_active { "" } else { " (" };
    let closing = if a.is_active { "" } else { ")" };
    let daily = a
        .target_minutes_daily
        .map(|m| format!("{m}m"))
        .unwrap_or_else(|| "—".to_string());
    let weekly = a
        .target_minutes_weekly
        .map(|m| format!("{m}m"))
        .unwrap_or_else(|| "—".to_string());
    let hue = a.hue_label.clone().unwrap_or_else(|| "—".to_string());
    format!(
        "● {name}{off}{off_label}{closing}  daily={daily}  weekly={weekly}  hue={hue}  icon={icon}  id={id}",
        name = a.name,
        off = off,
        off_label = inactive_label,
        closing = closing,
        daily = daily,
        weekly = weekly,
        hue = hue,
        icon = a.icon.clone().unwrap_or_else(|| "—".to_string()),
        id = a.id,
    )
}

pub fn activity_list_text(as_: &[Activity], inactive_label: &str) -> String {
    let mut out = String::new();
    for a in as_ {
        out.push_str(&activity_text(a, inactive_label));
        out.push('\n');
    }
    out
}

pub fn settings_text(map: &serde_json::Map<String, serde_json::Value>) -> String {
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort();
    let mut out = String::new();
    for k in keys {
        out.push_str(&format!("{} = {}\n", k, map[k]));
    }
    out
}

// ---- report rendering -----------------------------------------------------

use oxiline_core::model::{CategoryBreakdown, DayTotals, RangeReport, RoutineStreak, WeekReport};

fn pct(r: Option<f64>) -> String {
    match r {
        Some(v) => format!("{}%", (v * 100.0).round() as i64),
        None => "—".into(),
    }
}

pub fn week_report_text(lang: L, r: &WeekReport) -> String {
    let mut out = format!(
        "{} ~ {} ({})\n",
        r.week_start,
        r.week_end,
        lang.report_this_week()
    );
    out.push_str(&totals_line(lang, &r.totals));
    out.push_str(&format!(
        "{} {}   {} {}\n\n",
        lang.report_rate(),
        pct(r.completion_rate),
        lang.report_prev_week(),
        pct(r.prev_completion_rate)
    ));
    out.push_str(&cat_block(lang, &r.categories));
    out.push_str(&streak_block(lang, &r.streaks));
    out
}

pub fn range_report_text(lang: L, r: &RangeReport) -> String {
    let mut out = format!("{} ~ {}\n", r.from, r.to);
    out.push_str(&format!(
        "{} {}\n\n",
        lang.report_rate(),
        pct(r.completion_rate)
    ));
    out.push_str(&cat_block(lang, &r.categories));
    out.push_str(&streak_block(lang, &r.streaks));
    out
}

pub fn streak_list_text(lang: L, streaks: &[RoutineStreak]) -> String {
    let mut out = String::new();
    for s in streaks {
        out.push_str(&format!(
            "  {:<16} {}{}\n",
            s.title,
            s.current,
            lang.report_day()
        ));
    }
    if out.is_empty() {
        out = format!("  ({})\n", lang.report_no_routines());
    }
    out
}

fn totals_line(lang: L, t: &DayTotals) -> String {
    format!(
        "{} {} · {} {} · {} {} · {} {}\n",
        lang.report_done(),
        t.done,
        lang.report_skipped(),
        t.skipped,
        lang.report_not_recorded(),
        t.not_recorded,
        lang.report_upcoming(),
        t.upcoming
    )
}

fn cat_block(lang: L, cats: &[CategoryBreakdown]) -> String {
    let mut out = format!("{}\n", lang.report_categories());
    for c in cats {
        let denom = c.done + c.not_recorded;
        out.push_str(&format!(
            "  {:<8} {}/{}  {}\n",
            c.category_name,
            c.done,
            denom,
            pct(c.completion_rate)
        ));
    }
    out
}

fn streak_block(lang: L, streaks: &[RoutineStreak]) -> String {
    if streaks.is_empty() {
        return String::new();
    }
    format!(
        "\n{}\n{}",
        lang.report_streaks(),
        streak_list_text(lang, streaks)
    )
}

// ---- record layer rendering (Task 9) ------------------------------------

use chrono::{DateTime, Utc};

/// Format a duration in seconds as `H:MM` or `M:SS` (minutes under one hour).
fn h_mm(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 {
        let h = s / 3600;
        let m = (s % 3600) / 60;
        format!("{h}:{m:02}")
    } else {
        let m = s / 60;
        let ss = s % 60;
        format!("{m}:{ss:02}")
    }
}

/// Render the bare `record` view: active session + today's compliance.
pub fn record_state_text(lang: L, st: &RecordState) -> String {
    let mut out = String::new();
    match &st.active {
        Some(active) => {
            let dur = h_mm(active.elapsed_seconds as i64);
            out.push_str(&format!(
                "● {}  {dur} ({})\n",
                active.activity.name,
                lang.record_recording()
            ));
        }
        None => {
            out.push_str(&format!("({})\n", lang.record_idle()));
        }
    }
    if !st.today.is_empty() {
        out.push('\n');
        for c in &st.today {
            let recorded = h_mm(c.recorded_seconds as i64);
            let target = c
                .target_seconds
                .map(|t| h_mm(t as i64))
                .unwrap_or_else(|| "—".into());
            out.push_str(&format!(
                "  {:<16} {recorded} / {target}\n",
                c.activity.name
            ));
        }
    }
    out
}

/// Render `record log` view: each row as `HH:MM:SS  <name>  (<duration>)`.
pub fn record_log_text(lang: L, records: &[Record], now: DateTime<Utc>) -> String {
    if records.is_empty() {
        return format!("({})\n", lang.record_log_empty());
    }
    let mut out = String::new();
    for r in records {
        let end = match &r.ended_at {
            Some(e) => e.clone(),
            None => {
                // Live record: stamp it with `now` so the duration is meaningful.
                now.format("%Y-%m-%dT%H:%M:%SZ").to_string()
            }
        };
        let start_ts = DateTime::parse_from_rfc3339(&r.started_at)
            .ok()
            .map(|d| d.with_timezone(&Utc));
        let end_ts = DateTime::parse_from_rfc3339(&end).ok().map(|d| d.with_timezone(&Utc));
        let dur = match (start_ts, end_ts) {
            (Some(s), Some(e)) => (e - s).num_seconds(),
            _ => 0,
        };
        let hh = format!(
            "{:02}:{:02}:{:02}",
            start_ts.map(|d| d.format("%H").to_string()).unwrap_or_else(|| "--".into()).parse::<u32>().unwrap_or(0),
            start_ts.map(|d| d.format("%M").to_string()).unwrap_or_else(|| "--".into()).parse::<u32>().unwrap_or(0),
            start_ts.map(|d| d.format("%S").to_string()).unwrap_or_else(|| "--".into()).parse::<u32>().unwrap_or(0),
        );
        let marker = if r.ended_at.is_none() { "▶" } else { " " };
        let activity_name = r.activity_id.clone();
        out.push_str(&format!(
            "{marker} {hh}  {activity}  ({dur})\n",
            activity = activity_name,
            dur = h_mm(dur),
        ));
    }
    out
}
