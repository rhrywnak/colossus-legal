# MVP Test Checklist v0.2 — Colossus‑Legal

This document defines the manual end‑to‑end test plan for validating the Colossus‑Legal MVP (v0.2).  
It verifies backend, frontend, Neo4j, and real-case workflows.

---

# 1. Environment & System Sanity

## 1.1 Neo4j Ready
- Neo4j dev/test instance is running.
- Neo4j Browser: `RETURN 1;` executes successfully.
- Database is empty or contains only intended test data.

## 1.2 Backend Ready
From `backend/`:
```
cargo test --tests --manifest-path backend/Cargo.toml
cargo run  --manifest-path backend/Cargo.toml
```
Expected:
- All tests pass.
- Backend listens on `http://localhost:3403`.

Test directly:
```
curl http://localhost:3403/health
curl http://localhost:3403/api/status
```

## 1.3 Frontend Ready
From `frontend/`:
```
npm install
npm run dev
```
Visit:
```
http://localhost:5173/
```

Ensure:
- No console errors.
- Status panel loads (`GET /api/status`).

---

# 2. Claims Slice (Already Complete)

## 2.1 Claims List Page
- Navigate to `/claims`.
- Confirm loading → empty or populated list.
- Verify claim count matches Neo4j.

## 2.2 Create Claim
Through UI or API:
```
POST /claims
```
- Provide title + optional description.
- Verify:
  - Appears in list.
  - Exists in Neo4j via:
    ```
    MATCH (c:Claim) RETURN c;
    ```

## 2.3 Edit Claim
- Open `/claims/<id>`.
- Change title or description.
- Save.
- Reload page and verify update.
- Verify change in Neo4j.

## 2.4 Error Handling
- Try creating a claim with empty title → expect 400 validation error.
- Try fetching non‑existent claim ID → expect 404.

---

# 3. Documents Slice — CRUD + Detail

## 3.1 Documents List
Navigate to:
```
http://localhost:5173/documents
```
Verify:
- Loading → empty or populated list.
- Matches Neo4j:
  ```
  MATCH (d:Document) RETURN d;
  ```

## 3.2 Create Document (via API)
```
POST /documents
{
  "title": "Test Document 1",
  "type": "complaint",
  "created_at": "<now>"
}
```
Verify:
- Appears in UI.
- Node present in Neo4j.

## 3.3 Document Detail + Edit
- Click a document row → navigate to `/documents/<id>`.
- Change `title` or `type`.
- Save → verify UI update & Neo4j update.

## 3.4 Document Validation (L2)
- Empty title → 400.
- Empty type → 400.
- Fetch non‑existent ID → 404 with structured error.

---

# 4. Insight Endpoint — `/documents/recent`

## 4.1 View Recent Documents
Test via browser:
```
http://localhost:3403/documents/recent
```

Expected:
- JSON array sorted by most recent.
- Default limit: 10.

## 4.2 Limit Parameter
```
http://localhost:3403/documents/recent?limit=3
```
Verify:
- Exactly 3 items returned (or fewer if fewer exist).
- Correct descending sort by recentness.

## 4.3 Missing / Null `ingested_at`
Insert a document with `ingested_at: null`.
Verify it does **not** appear in `/documents/recent`.

---

# 5. Real Data Test (Recommended)

Load a small real case:
- 3–5 documents (complaint, motions, order, transcript).
- 3–6 claims.
- Edit a couple of them in the UI.

Verify:
- Claims show correctly.
- Documents list + detail+edit works.
- `/documents/recent` shows newest items.
- No broken UI states or console errors.

---

# 6. Error Resilience

## 6.1 Neo4j Down
Stop Neo4j momentarily.

Test:
- Backend `/api/status` should show `"degraded"` or `"error"`.
- Frontend should show appropriate error state on `/claims` or `/documents`.

Restart Neo4j and verify system self‑recovers.

## 6.2 Network / API Errors
- Block backend port temporarily (using ufw or local firewall).
- UI should display an error state, not hang.

---

# 7. Completion Checklist

The MVP is verified when:

- [ ] Backend tests all pass.
- [ ] Frontend loads successfully with status panel.
- [ ] Claims list, create, edit work fully.
- [ ] Documents list + detail + edit work fully.
- [ ] `/documents/recent` works with correct ordering + limit.
- [ ] Validation + 404 behaviors confirmed.
- [ ] A small real case has been loaded and verified.
- [ ] System handles Neo4j down / network errors gracefully.
