-- Recolor builtin categories to the oxi six-hue label palette (DESIGN.md §3.2).
-- Idempotent UPDATEs: safe on fresh DBs (V1 seed) and on existing DBs alike.
-- cat_other uses the achromatic sentinel (-1); colors.ts renders C=0 for hue<0.
UPDATE categories SET color_hue = 250 WHERE id = 'cat_work';     -- blue  (working / in-progress)
UPDATE categories SET color_hue = 145 WHERE id = 'cat_health';   -- green (positive / success)
UPDATE categories SET color_hue = 195 WHERE id = 'cat_study';    -- teal  (reference / informational)
UPDATE categories SET color_hue = 75  WHERE id = 'cat_rest';     -- amber (idea / pending)
UPDATE categories SET color_hue = 310 WHERE id = 'cat_personal'; -- purple(personal / inspiration)
UPDATE categories SET color_hue = -1  WHERE id = 'cat_other';    -- achromatic sentinel
