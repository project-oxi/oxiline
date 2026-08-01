-- Recording-centered layer (additive; legacy tasks/routine_blocks/categories remain untouched).
-- See docs/superpowers/specs/2026-08-01-record-layer-design.md §4.

-- activities: the switchable, budgetable unit (subsumes legacy task/routine/card-template)
CREATE TABLE activities (
    id                     TEXT PRIMARY KEY,          -- UUID v7
    name                   TEXT NOT NULL,
    hue_label              TEXT,                      -- red|amber|green|teal|blue|purple (DESIGN.md §3.2)
    icon                   TEXT,                      -- lucide name
    category_id            TEXT REFERENCES categories(id) ON DELETE SET NULL,
    target_minutes_daily   INTEGER,                   -- NULL = no daily budget
    target_minutes_weekly  INTEGER,                   -- NULL = no weekly budget
    is_active              INTEGER NOT NULL DEFAULT 1,
    sort_order             INTEGER NOT NULL DEFAULT 0,
    created_at             TEXT NOT NULL,             -- ISO 8601 UTC
    updated_at             TEXT NOT NULL,
    CHECK (target_minutes_daily  IS NULL OR target_minutes_daily  BETWEEN 1 AND 1440),
    CHECK (target_minutes_weekly IS NULL OR target_minutes_weekly BETWEEN 1 AND 10080)
);
CREATE INDEX idx_activities_active ON activities(is_active);

-- plans: a time slot holding OR alternatives (replaces routine_blocks)
CREATE TABLE plans (
    id              TEXT PRIMARY KEY,
    date            TEXT,              -- ISO date; NULL + weekday_mask present => recurring
    start_minute    INTEGER NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    duration_minute INTEGER NOT NULL CHECK (duration_minute BETWEEN 1 AND 1440),
    weekday_mask    INTEGER NOT NULL DEFAULT 0,   -- 0 = one-shot (uses date); !=0 = recurring on those weekdays
    title           TEXT,                          -- optional group label
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL,
    CHECK ((weekday_mask = 0 AND date IS NOT NULL) OR weekday_mask != 0)
);
CREATE INDEX idx_plans_date ON plans(date);
CREATE INDEX idx_plans_recur ON plans(weekday_mask) WHERE weekday_mask != 0;

-- plan_options: the OR alternatives of a plan (>=1)
CREATE TABLE plan_options (
    id           TEXT PRIMARY KEY,
    plan_id      TEXT NOT NULL REFERENCES plans(id) ON DELETE CASCADE,
    activity_id  TEXT NOT NULL REFERENCES activities(id) ON DELETE CASCADE,
    sort_order   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_options_plan ON plan_options(plan_id);

-- records: actual recorded intervals (one open at a time, §3.3)
CREATE TABLE records (
    id           TEXT PRIMARY KEY,
    activity_id  TEXT NOT NULL REFERENCES activities(id) ON DELETE RESTRICT,  -- history is the product; see §11
    started_at   TEXT NOT NULL,             -- ISO 8601 UTC, second precision
    ended_at     TEXT,                      -- NULL = currently recording
    note         TEXT,
    created_at   TEXT NOT NULL,
    updated_at   TEXT NOT NULL,
    CHECK (ended_at IS NULL OR ended_at > started_at)
);
CREATE INDEX idx_records_activity ON records(activity_id, started_at);
CREATE INDEX idx_records_open     ON records(started_at) WHERE ended_at IS NULL;
CREATE INDEX idx_records_started  ON records(started_at);

-- Default settings (§4 keys table).
INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
  ('record_switch_hotkey','"CmdOrCtrl+Shift+A"','2026-08-01T00:00:00Z'),
  ('record_rounding_minutes','5','2026-08-01T00:00:00Z'),
  ('record_default_stop_on_quit','true','2026-08-01T00:00:00Z'),
  ('record_stale_open_hours','12','2026-08-01T00:00:00Z'),
  ('timetable_default_mode','"both"','2026-08-01T00:00:00Z'),
  ('budget_default_scope','"weekly"','2026-08-01T00:00:00Z');
