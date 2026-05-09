-- Postiz-Rust multi-account support
-- Adds root_internal_id to support parent-child integration relationships
-- (e.g., Facebook user token → multiple page integrations)

ALTER TABLE integrations ADD COLUMN IF NOT EXISTS root_internal_id TEXT DEFAULT '';
