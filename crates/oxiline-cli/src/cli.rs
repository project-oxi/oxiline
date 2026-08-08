//! clap definitions for the `oxiline` CLI (`05-cli-spec.md` §5.2).

use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "oxiline",
    version,
    about = "OxiLine — manage the flow of your day, not a calendar of appointments"
)]
pub struct Cli {
    /// JSON output mode (machine-readable; success payload on stdout, errors on stderr).
    #[arg(long, global = true)]
    pub json: bool,

    /// Override the database path for this invocation.
    #[arg(long, global = true, value_name = "PATH")]
    pub db: Option<PathBuf>,

    /// Override output language for this invocation (ko|en).
    #[arg(long, global = true, value_name = "LANG")]
    pub lang: Option<String>,

    /// Preview a write command without applying it.
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Suppress success output (exit code only).
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    #[command(subcommand)]
    pub cmd: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// What's happening now + what's next (identical to the HUD data).
    Now,
    /// Manage categories.
    Category {
        #[command(subcommand)]
        action: CategoryAction,
    },
    /// Manage activities (switchable, budgetable units of work).
    Activity {
        #[command(subcommand)]
        action: ActivityAction,
    },
    /// Read/modify settings.
    Settings {
        #[command(subcommand)]
        action: SettingsAction,
    },
    /// Force-show the floating HUD (signals a running GUI).
    Hud {
        #[command(subcommand)]
        action: HudAction,
    },
    /// Completion report for a week or date range.
    Report {
        /// Current week (default).
        #[arg(long)]
        week: bool,
        /// Last N days (e.g. --last 30).
        #[arg(long, value_name = "N")]
        last: Option<u32>,
        /// Explicit inclusive range FROM:TO (YYYY-MM-DD).
        #[arg(long, value_name = "FROM:TO")]
        range: Option<String>,
    },
    /// Manage recording sessions (start/stop/log). Bare `record` emits state.
    Record {
        #[command(subcommand)]
        action: Option<RecordAction>,
    },
    /// Manage plans (OR choice-sets materialized into slots per date).
    Plan {
        #[command(subcommand)]
        action: PlanAction,
    },
    /// Run a self-diagnostic (DB path, schema version, WAL state, categories).
    Doctor,
}

#[derive(Subcommand)]
pub enum HudAction {
    /// Show the HUD now.
    Show,
}

#[derive(Subcommand)]
pub enum CategoryAction {
    /// Add a category.
    Add {
        name: String,
        #[arg(long, value_name = "0-360", default_value_t = 250.0)]
        hue: f64,
        #[arg(long, value_name = "NAME")]
        icon: Option<String>,
    },
    /// List categories.
    List,
    /// Remove a category.
    Rm { id: String },
}

#[derive(Subcommand)]
pub enum SettingsAction {
    /// Get a setting (omit key for all).
    Get { key: Option<String> },
    /// Set a setting.
    Set { key: String, value: String },
}

/// Tri-state wrapper for nullable numeric budgets (`--daily`, `--weekly`).
///
/// Maps the CLI's "0 means clear" convention onto the double-Option field
/// shape that `oxiline_core::activities::update_activity` expects:
/// - outer `None`        ⇒ user didn't pass the flag → leave unchanged
/// - `Some(None)`        ⇒ user passed `--daily 0` → clear to NULL
/// - `Some(Some(n))`     ⇒ user passed `--daily n` (n>0) → set to n
#[derive(Clone, Copy, Debug)]
pub struct MinuteBudget(pub Option<u32>);

impl std::str::FromStr for MinuteBudget {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let n: u32 = s
            .parse()
            .map_err(|e| format!("invalid minute budget '{s}': {e}"))?;
        Ok(MinuteBudget(if n == 0 { None } else { Some(n) }))
    }
}

/// Manage activities (the switchable, budgetable unit of work).
#[derive(Subcommand)]
pub enum ActivityAction {
    /// Add a new activity.
    Add {
        name: String,
        #[arg(long, value_name = "MIN")]
        daily: Option<MinuteBudget>,
        #[arg(long, value_name = "MIN")]
        weekly: Option<MinuteBudget>,
        #[arg(long, value_name = "LABEL")]
        hue: Option<String>,
        #[arg(long, value_name = "NAME")]
        icon: Option<String>,
        #[arg(long, value_name = "ID|NAME")]
        category: Option<String>,
    },
    /// List activities.
    List {
        #[arg(long)]
        active_only: bool,
    },
    /// Show one activity by id or name.
    Show { id: String },
    /// Edit an activity by id or name (0 on --daily/--weekly clears the budget).
    Edit {
        id: String,
        #[arg(long, value_name = "TEXT")]
        name: Option<String>,
        #[arg(long, value_name = "MIN")]
        daily: Option<MinuteBudget>,
        #[arg(long, value_name = "MIN")]
        weekly: Option<MinuteBudget>,
        #[arg(long, value_name = "LABEL")]
        hue: Option<String>,
        #[arg(long, value_name = "NAME")]
        icon: Option<String>,
    },
    /// Activate or deactivate an activity.
    Toggle {
        id: String,
        #[arg(long, conflicts_with = "off")]
        on: bool,
        #[arg(long, conflicts_with = "on")]
        off: bool,
    },
    /// Remove an activity. Refuses if records exist unless --force is given.
    Rm {
        id: String,
        #[arg(long)]
        force: bool,
    },
}

/// Manage recording sessions (`05-cli-spec.md` §5.4).
#[derive(Subcommand)]
pub enum RecordAction {
    /// Emit the current recording state (active session + today's compliance).
    State,
    /// Start a new recording session for an activity (closes any prior open one).
    Start {
        /// Activity name or id to record.
        activity: String,
        /// Backdate the switch instant to the given ISO 8601 UTC timestamp.
        /// The prior record (if any) is closed at the same instant.
        #[arg(long, value_name = "ISO")]
        at: Option<String>,
    },
    /// Close the currently-open record (if any).
    Stop,
    /// List records (today by default; filter by --activity / --date / --range).
    Log {
        #[arg(long, value_name = "ID|NAME")]
        activity: Option<String>,
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        #[arg(long, value_name = "FROM:TO")]
        range: Option<String>,
    },
}

/// Manage plans — recurring (`--days`) or one-shot (`--date`) OR choice-sets
/// (`05-cli-spec.md` §5.4, plan group).
#[derive(Subcommand)]
pub enum PlanAction {
    /// Add a plan. `--days` makes it recurring; `--date` makes it one-shot.
    Add {
        /// Start time as HH:MM (local). Defaults to the current minute.
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        /// Duration in minutes.
        #[arg(long, value_name = "MIN", default_value_t = 60)]
        duration: u32,
        /// Recurring days: comma-list `mon,tue,...`, `weekdays`, or `daily`.
        /// Bit 0 = Monday … bit 6 = Sunday (matches `plan::slots_for_date`).
        #[arg(long, value_name = "DAYS")]
        days: Option<String>,
        /// One-shot date YYYY-MM-DD (exclusive with `--days`).
        #[arg(long, value_name = "DATE", conflicts_with = "days")]
        date: Option<String>,
        /// Optional slot title.
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        /// Comma-separated activity ids/names forming the OR choice-set.
        #[arg(long, value_name = "A,B,C")]
        options: String,
    },
    /// List plans: all, `--recurring` only, or materialized `--date` slots.
    List {
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
        #[arg(long)]
        recurring: bool,
    },
    /// Edit a plan by id. Omitted fields keep their current value.
    Edit {
        id: String,
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        #[arg(long, value_name = "MIN")]
        duration: Option<u32>,
        #[arg(long, value_name = "DAYS")]
        days: Option<String>,
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        #[arg(long, value_name = "A,B,C")]
        options: Option<String>,
    },
    /// Remove a plan by id (cascades its options).
    Rm { id: String },
}
