//! Output rendering: human text vs. JSON (`05-cli-spec.md` §5.1).

use oxiline_core::model::{Activity, Category, Compliance, NowEntry, NowSummary, Plan, PlanSlot, Record, RecordState, RoutineBlock, Task, TimelineItem};
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

pub fn now_text(lang: L, ctx: &NowSummary) -> String {
    let fmt_now = |n: &NowEntry, label: &str| -> String {
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

// ---- plan rendering -------------------------------------------------------

/// Render a recurring/one-shot plan row (no option names — those live on slots).
pub fn plan_text(p: &Plan) -> String {
    let range = minute_range(Some(p.start_minute), Some(p.duration_minute));
    let when = match &p.date {
        Some(d) => d.clone(),
        None => render_mask(p.weekday_mask),
    };
    let title = p
        .title
        .as_deref()
        .filter(|s| !s.is_empty())
        .map(|t| format!("  {t}"))
        .unwrap_or_default();
    format!("● {range}  {when}{title}  id={id}", id = p.id)
}

pub fn plan_list_text(plans: &[Plan]) -> String {
    let mut out = String::new();
    for p in plans {
        out.push_str(&plan_text(p));
        out.push('\n');
    }
    out
}

/// Render materialized slots for a date: range + OR option names + resolution.
pub fn plan_slot_list_text(slots: &[PlanSlot]) -> String {
    let mut out = String::new();
    for s in slots {
        let range = minute_range(Some(s.start_minute), Some(s.duration_minute));
        let names: Vec<&str> = s.options.iter().map(|a| a.name.as_str()).collect();
        let mark = if s.is_resolved { "✔" } else { "○" };
        out.push_str(&format!("{mark} {range}  {}\n", names.join(" / ")));
    }
    out
}

/// Human label for a weekday mask. Bit 0 = Monday (matches plan.rs).
fn render_mask(mask: u8) -> String {
    match mask {
        0 => "one-shot".into(),
        0b111_1111 => "daily".into(),
        0b001_1111 => "weekdays".into(),
        _ => {
            const ABBR: [&str; 7] = ["mon", "tue", "wed", "thu", "fri", "sat", "sun"];
            let mut out = Vec::new();
            for b in 0..7u8 {
                if mask & (1 << b) != 0 {
                    out.push(ABBR[b as usize]);
                }
            }
            out.join(",")
        }
    }
}

// ---- neutral compliance rendering (report, Task 11) ---------------------

/// Per active activity: recorded vs target, ratio, and a neutral state
/// label. Over is a surplus ("초과 +Xm"), never a failure.
pub fn compliance_text(lang: L, comps: &[Compliance]) -> String {
    let mut out = String::new();
    for c in comps {
        let recorded = h_mm(c.recorded_seconds as i64);
        let target = c
            .target_seconds
            .map(|t| h_mm(t as i64))
            .unwrap_or_else(|| "—".into());
        let ratio = pct(c.ratio);
        out.push_str(&format!(
            "● {:<16} {:>6} / {:<6} {:>5}  {}\n",
            c.activity.name, recorded, target, ratio, compliance_state_label(lang, c),
        ));
    }
    out
}

fn compliance_state_label(lang: L, c: &Compliance) -> String {
    use oxiline_core::model::ComplianceState::*;
    match c.state {
        Under => lang.compliance_under().into(),
        Met => lang.compliance_met().into(),
        Over => {
            let over_secs = c
                .recorded_seconds
                .saturating_sub(c.target_seconds.unwrap_or(0));
            format!("{} +{}m", lang.compliance_over(), (over_secs as f64 / 60.0).round() as i64)
        }
        Unbudgeted => lang.compliance_unbudgeted().into(),
    }
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

use oxiline_core::model::RoutineStreak;

fn pct(r: Option<f64>) -> String {
    match r {
        Some(v) => format!("{}%", (v * 100.0).round() as i64),
        None => "—".into(),
    }
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
