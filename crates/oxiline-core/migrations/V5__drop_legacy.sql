-- Task 8 Sub5: drop the legacy task/routine tables.
-- The recording layer (activities/plans/plan_options/records/compliance)
-- fully replaces them. All FKs were ON DELETE SET NULL and pointed inward
-- (legacy→legacy or legacy→categories), so no keeper table is affected.
-- Drop child-first for clarity (SQLite does not FK-check DROP TABLE).
DROP TABLE IF EXISTS tasks;
DROP TABLE IF EXISTS routine_blocks;
DROP TABLE IF EXISTS routine_groups;
