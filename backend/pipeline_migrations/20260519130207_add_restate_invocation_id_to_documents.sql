-- add_restate_invocation_id_to_documents: Add restate invocation id to documents
--
-- Created: 2026-05-19 13:02:07
-- Target: pipeline database
--
-- Why: The Restate ingress `/send` call returns an invocation id (`inv_…`)
-- that uniquely identifies the workflow journal for a document. We need
-- that id at delete time to call the Restate admin API's purge endpoint
-- (`PATCH /invocations/{invocation_id}/purge`) — Restate's keyed
-- service-name form does not support purge on this deployment. Without
-- persisting the id, a deleted document leaves an orphan workflow
-- journal in Restate, and re-uploading the same document_id later
-- 409s with `PreviouslyAccepted`.
--
-- Nullable: existing rows have no recorded invocation, and documents
-- that have never had Process clicked never will. Both cases must
-- remain valid. The delete handler skips the purge call when the
-- column is NULL.
--
-- No index needed: the only read path is the delete handler's single-
-- document lookup, which already filters by `id` (primary key).

ALTER TABLE documents
    ADD COLUMN restate_invocation_id TEXT;
