# Colossus-Legal — TASK_TRACKER.md

This is the **improved task tracker** for Colossus-Legal.

It keeps the original Phase/Task structure (T0.x, T1.x, T2.x, …) and adds:

- Status (DONE / IN_PROGRESS / PLANNED / FUTURE / BLOCKED)
- Layer (L0–L3, when applicable)
- Suggested Agent persona
- Suggested branch name
- Acceptance criteria (for active/future tasks)
- Integration-first test requirements (for L1+ tasks)

For full workflow details, see `docs/WORKFLOW.md`.

---

## Status Codes

- `DONE` — Task complete.
- `IN_PROGRESS` — Actively being implemented.
- `PLANNED` — Approved and queued.
- `FUTURE` — Not scheduled yet.
- `BLOCKED` — Cannot proceed (dependency or external issue).

---

# Phase 0 — Wiring & Bring-Up (Smoke Test)

> **Goal:** Ensure the project runs end-to-end before implementing features.

### T0.1 — Add `/api/status` Endpoint (Backend)  
- **Status:** DONE (2025-11-22)  
- **Layer:** L0  
- **Persona:** BackendAgent  
- **Summary:** Add GET `/api/status` returning `{ app, version, status }`.  
- **Acceptance:** Backend responds 200 OK with correct JSON.

### T0.2 — Frontend Status Panel  
- **Status:** DONE (2025-11-22)  
- **Layer:** L0  
- **Persona:** FrontendAgent  
- **Summary:** Frontend uses `/api/status` and displays backend health.

### T0.3 — Dev CORS between frontend and backend  
- **Status:** DONE (2025-11-22)

---

# Phase 1 — Foundations & Manual Workflow

> **Goal:** Backend + Frontend minimal foundations (no real Neo4j yet).

### T1.1 — Backend Skeleton  
- **Status:** DONE  
- **Layer:** L0  
- **Persona:** BackendAgent  
- **Summary:** Axum server with `/health` and logging.

### T1.2 — Core Models & DTOs (Backend)  
- **Status:** DONE (2025-11-22)  
- **Layer:** L0/L1  
- **Persona:** BackendAgent  

### T1.3 — Basic CRUD Endpoints (Stubbed)  
- **Status:** DONE (2025-11-22)  
- **Layer:** L0  
- **Persona:** BackendAgent  

### T1.4 — Frontend Skeleton Pages  
- **Status:** DONE (2025-11-22)  
- **Layer:** L0  
- **Persona:** FrontendAgent  

### T1.5 — Backend Dev Env Configuration (Runtime Readiness & Test Isolation)  
- **Status:** DONE (2025-11-26)  
- **Layer:** L0  
- **Persona:** BackendAgent + DocsAgent  
- **Summary:**  
  - Implement dotenv-based backend env loading so `cargo run` works without env-var panics.
  - Ensure Claims integration tests only insert/delete **test-marked** Claim nodes (do not delete all :Claim nodes).
  - Document backend env setup and test behavior in DEV_ONBOARDING.md.
- **Acceptance Criteria:**  
  - With `backend/.env` configured (copied from `.env.example`), `cargo run --manifest-path backend/Cargo.toml` starts without missing-env panics.
  - Claims integration tests in `backend/tests/claims_list.rs`:
    - Use a marker (e.g. `source: "test"` or `test_run_id`) on created Claim nodes.
    - Only delete nodes with that marker; other Claim data is untouched.
  - `cargo test --tests --manifest-path backend/Cargo.toml` passes.
  - `docs/DEV_ONBOARDING.md` includes a clear "Backend dev env setup" section.
  - This T1.5 entry is updated to **DONE** with date once all above are satisfied.
- **Task Doc:** `docs/tasks/T1.5_Dev_Env_Config.md`.


---

# Phase 2 — Claims API v1 (Current Focus)

> **Goal:** Build a real, layered Claims feature (API + UI), breadth-first.

---

### T2.1a — Claims API L0 (Skeleton Routes + Stubs, Compile-Only)  
- **Status:** DONE (2025-11-24, compile-level)  
- **Layer:** L0  
- **Persona:** BackendAgent  
- **Suggested Branch:** `feature/T2.1a-claims-api-l0`  
- **Acceptance Criteria:**  
  - `GET /claims` exists and compiles. ✔  
  - Returns stubbed `ClaimDto` list (or empty). ✔  
  - No Neo4j logic added. ✔  
  - Runtime verification deferred to T1.5.  
- **Tests:** Optional for L0.  
- **Task Doc:** `docs/tasks/T2.1a_Claims_API_L0.md`.

---

### T2.1b — Claims API L1 (Real Neo4j List, Happy Path)  
- **Status:** DONE (2025-11-26)  
- **Layer:** L1  
- **Persona:** BackendAgent  
- **Suggested Branch:** reuse `feature/T2.1a-claims-api-l0`  
- **Acceptance Criteria:**  
  - `GET /claims` uses Neo4j to return live data (no stubs).  
  - Happy path works end-to-end.  
  - **At least one backend integration test exists and passes:**
    - Example test file: `backend/tests/claims_list.rs`.  
    - Test with Claim nodes → non-empty JSON.  
    - Test with no Claim nodes → empty JSON.  
- **Task Doc:** `docs/tasks/T2.1b_Claims_API_L1.md`.

---

### T2.1c — Claims API L2 (Validation & Error Handling)
- **Status:** DONE (2025-11-27)
- **Layer:** L2
- **Persona:** BackendAgent
- **Acceptance Criteria:**
  - Invalid payloads return structured 400 errors.
  - Non-existent IDs return 404.
  - Tests cover success + error cases.
- **Task Doc:** `docs/tasks/T2.1c_Claims_API_L2_Validation.md`.
- **Notes:** Added title/status validation and structured 400/404/500 responses for Claims; new integration tests
in `backend/tests/claims_validation.rs` cover invalid payloads, missing IDs, and happy path; `cargo test` and
`cargo check` pass.

---

### T2.1d — Claims API L3 (Analysis Endpoints)  
- **Status:** FUTURE  
- **Layer:** L3  
- **Persona:** BackendAgent  
- **Acceptance Criteria:**  
  - Analysis endpoints (`/analysis/...`) implemented.  
  - Graph traversal tests included.  
- **Task Doc:** TBD.

---

### T2.2a — Claims UI L0 (Skeleton Page + Stub Service)  
- **Status:** DONE (2025-11-27)  
- **Layer:** L0  
- **Persona:** FrontendAgent  
- **Suggested Branch:** `feature/T2.2a-claims-ui-l0`  
- **Acceptance Criteria:**  
  - `/claims` route exists in frontend.  
  - ClaimsPage uses stub `getClaimsStub()`.  
  - Loading, empty, and error states implemented.  
  - (Optional) Basic test scaffold allowed.  
- **Task Doc:** `docs/tasks/T2.2a_Claims_UI_L0.md`.
- **Notes:** Claims UI L0 at /claims using stub `getClaimsStub()`; loading/error/empty/success states implemented; Vitest stub service test added in `frontend/src/services/__tests__/claims.test.ts`; `npm run test` and `npm run build` pass.

---

### T2.2b — Claims UI L1 (Real API Integration + Basic Tests)
- **Status:** DONE (2025-11-27)
- **Layer:** L1
- **Persona:** FrontendAgent
- **Acceptance Criteria:**
  - ClaimsPage calls real backend `/claims`.
  - Shows loading, empty, success, and error states.
  - **Vitest tests exist for services and/or page.**
- **Task Doc:** `docs/tasks/T2.2b_Claims_UI_L1.md`.
- **Notes:** ClaimsPage now calls real `/claims`; loading/empty/success/error states verified; Vitest service
  tests in `frontend/src/services/__tests__/claims.test.ts` pass; `npm run test` and `npm run build` pass.

---

### T2.3 — Claims End-to-End Integration + Docs Update  
- **Status:** PLANNED  
- **Layer:** L1  
- **Persona:** DocsAgent (verifying backend + frontend)  
- **Acceptance Criteria:**  
  - Backend + frontend integrated.  
  - TASK_TRACKER updated.  
  - PHASE_PLAN updated.  
  - No new warnings or build errors.  
- **Task Doc:** `docs/tasks/T2.3_Claims_Integration.md` (or TBD).

---

# Phase 3 — Core Graph (Documents/Evidence/People/Hearings/Decisions)

Mirrors Claims workflow (L0–L3).  
All **FUTURE** until Claims v1 stabilizes.

### T3.1 — Document API + UI  
### T3.2 — Evidence API + UI  
### T3.3 — Person API + UI  
### T3.4 — Hearing API + UI  
### T3.5 — Decision API + UI  

(All FUTURE.)

---

# Phase 4 — Relationship APIs & Basic Analysis

### T4.1 — Relationship Endpoints (APPEARS_IN, RELIES_ON, PRESENTED_AT, ...)  
- **Status:** FUTURE  

### T4.2 — Basic Analysis (Refuted Claims, Paths, Timeline)  
- **Status:** FUTURE  

---

# Phase 5 — Document Upload & Text Extraction

All FUTURE tasks.

### T5.1 — File Upload Endpoint  
### T5.2 — PDF/DOCX/OCR Extraction Pipeline  
### T5.3 — Upload UI  

---

# Phase 6 — AI Suggestion Pipeline

### T6.1 — LLM Integration  
### T6.2 — AI Suggestion Model  

All FUTURE.

---

# Phase 7 — Reporting & Visualization

### T7.1 — PDF/Doc Export  
### T7.2 — Graph Views & Timelines  
### T7.3 — UX Polish  

---

# Notes

- All tasks must follow AGENTS.md + WORKFLOW.md.
- All L1+ tasks require tests.

# End of TASK_TRACKER.md
