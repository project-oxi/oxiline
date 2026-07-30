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
    /// The integrated timeline for a date (defaults to today).
    Today {
        #[arg(long, value_name = "DATE")]
        date: Option<String>,
    },
    /// Manage tasks.
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
    /// Manage routine blocks.
    Routine {
        #[command(subcommand)]
        action: RoutineAction,
    },
    /// Manage categories.
    Category {
        #[command(subcommand)]
        action: CategoryAction,
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
    /// Dump a date range as JSON (always JSON, read-only).
    Export {
        #[arg(long, value_name = "FROM:TO")]
        range: String,
    },
    /// Self-diagnostic: DB path, schema version, WAL, GUI process.
    Doctor,
}

#[derive(Subcommand)]
pub enum HudAction {
    /// Show the HUD now.
    Show,
}

#[derive(Subcommand)]
pub enum TaskAction {
    /// Add a task (dated or backlog).
    Add {
        title: String,
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        /// Add as a backlog (undated) item.
        #[arg(long)]
        backlog: bool,
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        #[arg(long, value_name = "MIN", default_value_t = 30)]
        duration: u16,
        #[arg(long, value_name = "ID|NAME")]
        category: Option<String>,
        #[arg(long, value_name = "TEXT")]
        notes: Option<String>,
    },
    /// List tasks (default: today).
    List {
        #[arg(long, value_name = "DATE|today|tomorrow|yesterday")]
        date: Option<String>,
        #[arg(long)]
        backlog: bool,
        #[arg(long, value_name = "FROM:TO")]
        range: Option<String>,
    },
    /// Show a single task.
    Show { id: String },
    /// Mark a task (or virtual occurrence) done.
    Done { id: String },
    /// Mark a task not done.
    Undone { id: String },
    /// Skip a routine occurrence for its date only.
    Skip { id: String },
    /// Edit a task's fields.
    Edit {
        id: String,
        #[arg(long, value_name = "YYYY-MM-DD")]
        date: Option<String>,
        #[arg(long)]
        backlog: bool,
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        #[arg(long, value_name = "MIN")]
        duration: Option<u16>,
        #[arg(long, value_name = "ID|NAME")]
        category: Option<String>,
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        #[arg(long, value_name = "TEXT")]
        notes: Option<String>,
    },
    /// Remove a task. Routine occurrences become a skip (hide for this date);
    /// manual tasks are physically deleted.
    Rm { id: String },
}

#[derive(Subcommand)]
pub enum RoutineAction {
    /// Add a routine block.
    Add {
        title: String,
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        #[arg(long, value_name = "MIN", default_value_t = 30)]
        duration: u16,
        #[arg(long, value_name = "mon,tue,…|weekdays|weekends|daily", default_value = "daily")]
        days: String,
        #[arg(long, value_name = "DATE")]
        from: Option<String>,
        #[arg(long, value_name = "DATE")]
        until: Option<String>,
        #[arg(long, value_name = "ID|NAME")]
        category: Option<String>,
        #[arg(long, value_name = "TEXT")]
        notes: Option<String>,
    },
    /// List routine blocks.
    List {
        #[arg(long)]
        active_only: bool,
    },
    /// Show a single routine block.
    Show { id: String },
    /// Edit a routine block's fields.
    Edit {
        id: String,
        #[arg(long, value_name = "HH:MM")]
        at: Option<String>,
        #[arg(long, value_name = "MIN")]
        duration: Option<u16>,
        #[arg(long, value_name = "mon,tue,…|weekdays|weekends|daily")]
        days: Option<String>,
        #[arg(long, value_name = "ID|NAME")]
        category: Option<String>,
        #[arg(long, value_name = "TEXT")]
        title: Option<String>,
        #[arg(long, value_name = "TEXT")]
        notes: Option<String>,
    },
    /// Turn a routine on or off.
    Toggle {
        id: String,
        #[arg(long)]
        on: bool,
        #[arg(long)]
        off: bool,
    },
    /// Remove a routine block.
    Rm { id: String },
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
    Get {
        key: Option<String>,
    },
    /// Set a setting.
    Set {
        key: String,
        value: String,
    },
}
