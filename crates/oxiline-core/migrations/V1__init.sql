-- OxiLine initial schema (03-data-model.md §3.3–3.8).
-- All time columns are LOCAL wall-clock, minute-of-day integers (0..1439).
-- Timestamps (created_at / updated_at / done_at) are ISO 8601 UTC.

-- ---- routine_groups (Phase 2 UI, schema present from v1) -------------------
CREATE TABLE routine_groups (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    icon        TEXT,
    is_active   INTEGER NOT NULL DEFAULT 1,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- ---- categories (color tags) -----------------------------------------------
CREATE TABLE categories (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    color_hue   REAL NOT NULL,          -- OKLCH H (0-360); L/C are global tokens
    icon        TEXT,
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_builtin  INTEGER NOT NULL DEFAULT 0,
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);

-- ---- routine_blocks (the recurring skeleton of a day) ----------------------
CREATE TABLE routine_blocks (
    id              TEXT PRIMARY KEY,          -- UUID v7
    group_id        TEXT REFERENCES routine_groups(id) ON DELETE SET NULL,
    title           TEXT NOT NULL,
    category_id     TEXT REFERENCES categories(id) ON DELETE SET NULL,
    start_minute    INTEGER NOT NULL CHECK (start_minute BETWEEN 0 AND 1439),
    duration_minute INTEGER NOT NULL CHECK (duration_minute BETWEEN 1 AND 1440),
    weekday_mask    INTEGER NOT NULL,          -- bit0=Mon … bit6=Sun
    effective_from  TEXT,                      -- ISO date
    effective_until TEXT,
    is_active       INTEGER NOT NULL DEFAULT 1,
    color_override  TEXT,                      -- OKLCH string
    notes           TEXT,
    sort_order      INTEGER NOT NULL DEFAULT 0,
    created_at      TEXT NOT NULL,
    updated_at      TEXT NOT NULL
);
CREATE INDEX idx_routine_blocks_active ON routine_blocks(is_active);

-- ---- tasks (manual + materialized routine occurrences) ---------------------
CREATE TABLE tasks (
    id                      TEXT PRIMARY KEY,
    date                    TEXT,                  -- ISO date; NULL = backlog
    title                   TEXT NOT NULL,
    category_id             TEXT REFERENCES categories(id) ON DELETE SET NULL,
    start_minute            INTEGER,
    duration_minute         INTEGER,
    is_done                 INTEGER NOT NULL DEFAULT 0,
    done_at                 TEXT,
    is_skipped              INTEGER NOT NULL DEFAULT 0,
    source                  TEXT NOT NULL CHECK (source IN ('manual','routine')),
    source_routine_block_id TEXT REFERENCES routine_blocks(id) ON DELETE SET NULL,
    notes                   TEXT,
    sort_order              INTEGER NOT NULL DEFAULT 0,
    created_at              TEXT NOT NULL,
    updated_at              TEXT NOT NULL
);
CREATE INDEX idx_tasks_date ON tasks(date);
CREATE INDEX idx_tasks_routine_origin ON tasks(source_routine_block_id, date);
CREATE UNIQUE INDEX uq_tasks_materialized_occurrence
    ON tasks(source_routine_block_id, date)
    WHERE source_routine_block_id IS NOT NULL;

-- ---- settings (key-value, JSON-encoded values) -----------------------------
CREATE TABLE settings (
    key         TEXT PRIMARY KEY,
    value       TEXT NOT NULL,          -- JSON string
    updated_at  TEXT NOT NULL
);

-- ---- seed categories (06-design-system.md §6.2 palette) --------------------
INSERT INTO categories (id, name, color_hue, icon, sort_order, is_builtin, created_at, updated_at) VALUES
    ('cat_work',     '업무',     250, 'briefcase',    0, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('cat_health',   '건강',     145, 'heart-pulse',  1, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('cat_study',    '학습',     300, 'book-open',    2, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('cat_rest',     '휴식',     350, 'moon',         3, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('cat_personal', '개인',      90, 'user',         4, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'),
    ('cat_other',    '기타',     250, 'circle',       5, 1, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
-- Note: cat_other uses C=0 (achromatic) at the application layer via hue sentinel.

-- ---- seed settings (03-data-model.md §3.8) ---------------------------------
INSERT INTO settings (key, value, updated_at) VALUES
    ('locale',                 '"system"', '2026-01-01T00:00:00Z'),
    ('theme',                  '"system"', '2026-01-01T00:00:00Z'),
    ('global_hotkey',          '"CmdOrCtrl+Shift+O"', '2026-01-01T00:00:00Z'),
    ('hud_duration_ms',        '2000', '2026-01-01T00:00:00Z'),
    ('day_start_hour',         '5',    '2026-01-01T00:00:00Z'),
    ('day_end_hour',           '26',   '2026-01-01T00:00:00Z'),
    ('week_starts_on',         '"mon"', '2026-01-01T00:00:00Z'),
    ('launch_at_login',        'true', '2026-01-01T00:00:00Z'),
    ('workload_warning_minutes','600', '2026-01-01T00:00:00Z');
