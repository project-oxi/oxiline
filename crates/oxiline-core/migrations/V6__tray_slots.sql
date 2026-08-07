-- Menu-bar multi-slot preferences (idempotent; respects existing user choice).
INSERT OR IGNORE INTO settings (key, value, updated_at)
VALUES (
    'tray_slots',
    json_object(
        'slots', json_array(
            json_object('id', 'now_recording', 'on', 1, 'order', 0),
            json_object('id', 'now_next',      'on', 1, 'order', 1),
            json_object('id', 'state_dot',     'on', 0, 'order', 2)
        )
    ),
    '2026-08-08T00:00:00Z'
);
