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
    pub fn backlog(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "백로그",
            Lang::En => "Backlog",
        }
    }
    pub fn empty_backlog(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "아직 정해지지 않은 할일이 없어요.",
            Lang::En => "No undated tasks yet.",
        }
    }
    pub fn empty_timeline(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "오늘 예정된 일이 없어요.",
            Lang::En => "Nothing scheduled.",
        }
    }
    pub fn task_added(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "할일을 추가했어요",
            Lang::En => "Task added",
        }
    }
    pub fn routine_added(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "루틴을 추가했어요",
            Lang::En => "Routine added",
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
    pub fn done(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "완료했어요",
            Lang::En => "done",
        }
    }
    pub fn undone(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "완료를 취소했어요",
            Lang::En => "undone",
        }
    }
    pub fn skipped(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "오늘만 건너뛰었어요",
            Lang::En => "skipped for today",
        }
    }
    pub fn hud_signal(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "HUD 표시를 요청했어요 (실행 중인 GUI가 있으면 표시됩니다)",
            Lang::En => "HUD show requested (displays if a GUI is running)",
        }
    }
    pub fn report_this_week(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "이번 주",
            Lang::En => "this week",
        }
    }
    pub fn report_prev_week(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "저번 주",
            Lang::En => "last week",
        }
    }
    pub fn report_rate(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "완료율",
            Lang::En => "completion",
        }
    }
    pub fn report_done(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "완료",
            Lang::En => "done",
        }
    }
    pub fn report_skipped(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "건너뜀",
            Lang::En => "skipped",
        }
    }
    pub fn report_not_recorded(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "체크인 없음",
            Lang::En => "no check-in",
        }
    }
    pub fn report_upcoming(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "예정",
            Lang::En => "upcoming",
        }
    }
    pub fn report_categories(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "카테고리",
            Lang::En => "categories",
        }
    }
    pub fn report_streaks(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "루틴 연속",
            Lang::En => "streaks",
        }
    }
    pub fn report_day(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "일",
            Lang::En => "d",
        }
    }
    pub fn report_no_routines(&self) -> &'static str {
        match self.0 {
            Lang::Ko => "활성 루틴 없음",
            Lang::En => "no active routines",
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
}
