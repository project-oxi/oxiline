//! Minimal bilingual (ko/en) message table for CLI output. Hand-rolled rather
//! than pulling in an i18n crate: the message set is small and stable, and a
//! dependency for a two-language CLI would be overkill.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Lang {
    Ko,
    En,
}

impl Lang {
    pub fn from_code(code: &str) -> Self {
        match code.to_ascii_lowercase().as_str() {
            "ko" => Lang::Ko,
            _ => Lang::En,
        }
    }
}

#[derive(Clone, Copy)]
pub struct L(pub Lang);

impl L {
    pub fn db_path(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "DB 경로",
            Lang::En => "DB path",
        }
    }
    pub fn schema_version(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "스키마 버전",
            Lang::En => "Schema version",
        }
    }
    pub fn wal_active(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "WAL 모드 활성화됨",
            Lang::En => "WAL mode active",
        }
    }
    pub fn latest(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "최신",
            Lang::En => "latest",
        }
    }
    pub fn categories_count(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "카테고리",
            Lang::En => "Categories",
        }
    }
    pub fn nothing_now(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "지금은 예정된 일이 없어요.",
            Lang::En => "Nothing scheduled right now.",
        }
    }
    pub fn all_done_today(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "오늘 예정된 일이 모두 끝났어요",
            Lang::En => "All done for today",
        }
    }
    pub fn now_label(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "지금",
            Lang::En => "now",
        }
    }
    pub fn next_label(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "다음",
            Lang::En => "next",
        }
    }
    pub fn remaining(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "남음",
            Lang::En => "left",
        }
    }
    pub fn in_min(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "후",
            Lang::En => "in",
        }
    }
    pub fn min_unit(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "분",
            Lang::En => "min",
        }
    }
    pub fn category_added(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "카테고리를 추가했어요",
            Lang::En => "Category added",
        }
    }
    pub fn removed(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "삭제했어요",
            Lang::En => "removed",
        }
    }
    pub fn hud_signal(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "HUD 표시를 요청했어요 (실행 중인 GUI가 있으면 표시됩니다)",
            Lang::En => "HUD show requested (displays if a GUI is running)",
        }
    }
    /// Generic resource label for a successful activity add.
    pub fn activity_added(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "활동이 추가됐어요",
            Lang::En => "Activity added",
        }
    }
    /// Marker for an inactive activity row (list / show).
    pub fn activity_inactive(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "비활성",
            Lang::En => "off",
        }
    }
    /// When `record` (bare) has no active session.
    pub fn record_idle(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "녹화 중인 활동이 없어요",
            Lang::En => "No active recording",
        }
    }

    /// Marker for live-recording duration in `record state` text.
    pub fn record_recording(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "녹화 중",
            Lang::En => "recording",
        }
    }
    /// When `record log` produced no rows.
    pub fn record_log_empty(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "기록이 없어요",
            Lang::En => "no records",
        }
    }
    /// Generic resource label for a successful plan add.
    pub fn plan_added(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "계획을 추가했어요",
            Lang::En => "Plan added",
        }
    }
    /// When `plan list` (bare/recurring) produced no rows.
    pub fn plan_list_empty(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "계획이 없어요",
            Lang::En => "no plans",
        }
    }
    /// When `report` has no activities to show.
    pub fn report_empty(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "기록된 활동이 없어요",
            Lang::En => "no activities to report",
        }
    }
    /// Neutral compliance state: under target (never "failure/missed").
    pub fn compliance_under(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "미달",
            Lang::En => "under",
        }
    }
    /// Neutral compliance state: met target.
    pub fn compliance_met(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "달성",
            Lang::En => "met",
        }
    }
    /// Neutral compliance state: over target — rendered as "초과 +Xm" / "over +Xm".
    pub fn compliance_over(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "초과",
            Lang::En => "over",
        }
    }
    /// Neutral compliance state: no target set.
    pub fn compliance_unbudgeted(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "목표 없음",
            Lang::En => "no target",
        }
    }
}
