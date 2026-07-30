//! Output rendering: human text vs. JSON (`05-cli-spec.md` §5.1).

use oxiline_core::model::{
    Category, NowContext, NowItem, RoutineBlock, Task, TimelineItem,
};
use oxiline_core::util;

use crate::lang::L;

pub fn json_pretty<T: serde::Serialize>(v: &T) -> String {
    serde_json::to_string_pretty(v).unwrap_or_else(|_| "null".into())
}

pub fn minute_range(start: Option<u16>, dur: Option<u16>) -> String {
    match (start, dur) {
        (Some(s), Some(d)) => {
            let end = (s as u32 + d as u32).min(1440);
            format!("{}-{}", util::minute_to_hhmm(s), util::minute_to_hhmm(end as u16))
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
        let virt = if it.is_virtual { " (루틴/가상)" } else { "" };
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
        let time = n
            .start_minute
            .map(util::minute_to_hhmm)
            .unwrap_or_default();
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
        out.push_str(&format!("● {}  hue={:<6} id: {}{builtin}\n", c.name, c.color_hue, c.id));
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
