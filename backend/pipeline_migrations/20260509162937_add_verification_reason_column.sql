-- Add verification_reason column to extraction_items.
--
-- Surfaces the diagnostic reason behind grounding_status = 'derived_invalid'
-- in the Review tab UI (and downstream tooling) without having to grep
-- server logs after the fact. NULL on every existing row — meaning
-- "no diagnostic recorded" — and only populated by the v5.1 derived
-- provenance validator and the recompute endpoint going forward.
--
-- Per CLAUDE.md §3 Rule 1 (no silent failures): a programmatically-
-- decided "this entity is not grounded" must carry the reason it was
-- decided, durably, where the user reviewing the entity can see it.

ALTER TABLE extraction_items
    ADD COLUMN IF NOT EXISTS verification_reason TEXT;
