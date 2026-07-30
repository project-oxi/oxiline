-- OxiLine Phase 2 settings additions (2026-07-30 spec §2).
-- All time columns remain LOCAL wall-clock minute-of-day integers.

INSERT OR IGNORE INTO settings (key, value, updated_at) VALUES
    ('notifications_enabled',     'false', '2026-07-30T00:00:00Z'),
    ('notification_lead_minutes', '5',     '2026-07-30T00:00:00Z');
