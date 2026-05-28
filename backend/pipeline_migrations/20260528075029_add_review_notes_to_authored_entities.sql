-- add_review_notes_to_authored_entities: Add review_notes to authored_entities
--
-- Created: 2026-05-28
-- Target: pipeline database
--
-- Why: the Element detail floating panel on the Home page lets a paralegal /
-- attorney write free-text mapping-review notes against an individual Element
-- (or, in the future, any authored entity). Notes are not part of the
-- canonical entity payload (`item_data`) — they are operator-authored review
-- context layered on top of canonical structure, so they live in their own
-- nullable column instead of being threaded into the JSONB blob.
--
-- Backward compatible: nullable, no default. Existing rows read NULL.
-- No index — lookups are by `entity_id` (already uniquely constrained).

ALTER TABLE authored_entities ADD COLUMN review_notes TEXT;

COMMENT ON COLUMN authored_entities.review_notes IS
    'Free-text paralegal/attorney review notes for this entity. Used by the '
    'Element detail panel for mapping review annotations.';
