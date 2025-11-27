# Colossus-Legal – RELEASE_NOTES.md

This document captures version history and important changes for Colossus-Legal.

Version tags use a combination of **semantic versioning** and **feature-layer suffixes**:

- `vMAJOR.MINOR.PATCH-feature-L<layer>`
  - Example: `v0.2.0-claims-L0`
  - Example: `v0.2.0-claims-L1`

---

## Template for New Entries

### vX.Y.Z-feature-LN – YYYY-MM-DD

**Feature / Layer:**
- Feature: e.g., Claims
- Layer: L0 (Skeleton), L1 (Real data), L2 (Validation/Relationships), L3 (Analysis/AI/Polish)

**Summary:**
- Short description of what this release enables.

**Details:**
- Backend:
  - …
- Frontend:
  - …
- Neo4j / Data:
  - …
- Docs:
  - …

**Notes:**
- Any migration notes, data concerns, or follow-up tasks.

---

## Example Entries

### v0.2.0-claims-L0 – 2025-11-24

**Feature / Layer:**
- Claims – Layer 0 (Skeleton)

**Summary:**
- Introduces stubbed Claims API and UI skeleton, enabling navigation and basic data shape testing.

**Details:**
- Backend:
  - Added stub `GET /claims` route and handler, returning a hard-coded list or empty array.
- Frontend:
  - Added `/claims` route and `ClaimsPage` using a stub service.
- Neo4j / Data:
  - No real queries yet; stubs only.
- Docs:
  - Updated `ARCHITECTURE.md` and `API_DESIGN.md` tutorial sections.
  - Updated `TASK_TRACKER.md` with T2.1a and T2.2a status.

**Notes:**
- Safe for exploratory dev/testing.
- Next step: implement `v0.2.0-claims-L1` with real Neo4j-backed data.

---

# End of RELEASE_NOTES.md
