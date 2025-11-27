## Backend dev environment setup

1) Copy the example env file: `cp backend/.env.example backend/.env`.
2) Set these values in `backend/.env`:
   - `NEO4J_URI` (e.g., `bolt://localhost:7687`)
   - `NEO4J_USER`
   - `NEO4J_PASSWORD`
   - `BACKEND_PORT` (defaults to 3403 if unset)
3) Ensure Neo4j is running and reachable at the configured URI.
4) Run the backend with `cargo run --manifest-path backend/Cargo.toml`.
5) Verify endpoints:
   - `GET http://localhost:3403/health`
   - `GET http://localhost:3403/api/status`
6) For tests, use a dev/test Neo4j database. Claims integration tests create nodes with a `source: "test"` marker and clean up only those nodes. Avoid running tests against a shared production graph.

---

## 10. Testing Strategy (Integration-First)

Colossus-Legal uses an integration-first testing philosophy:

- The most important question is: **"Does the real system work?"**
- We test the graph, HTTP surface, and UI via real flows, not mocks.

### 10.1 Backend Testing

- Integration tests live under `backend/tests/`.
- They should:
  - Start the Axum app (or relevant router) in test mode where feasible, or assume a running dev instance.
  - Use a real Neo4j connection suitable for dev/test.
  - Insert a small amount of test data using Cypher.
  - Hit real endpoints (`/claims`, `/analysis/...`) and assert responses.

Typical patterns:
- Happy-path tests for each endpoint as it reaches L1+.
- Validation/error tests for each endpoint when it reaches L2+.
- Analysis tests for graph traversal features (L3).

### 10.2 Frontend Testing

- Use Vitest + React Testing Library (once added).
- Focus on:
  - Data-fetching services (`src/services/*.ts`).
  - Major pages (`src/pages/*.tsx`) and their loading/empty/error/success states.

Testing is incremental with the layers:
- L0: tests optional (compile and wiring are the target).
- L1: tests required for core data flows.
- L2/L3: tests required for validation, analysis, and complex UX.

### Claims end-to-end verification checklist (L1/L2 slice)

1) Ensure Neo4j dev/test is running and configured via `backend/.env` (do not run tests against production data).
2) Start backend: `cargo run --manifest-path backend/Cargo.toml`.
3) Start frontend: `npm run dev` in `frontend/`.
4) Visit `/claims` in the browser:
   - Loading → empty state if no data.
   - Success state shows Claim items from backend.
   - Error state if backend is unreachable.
5) Optional verification commands:
   - Backend: `cargo test --tests --manifest-path backend/Cargo.toml` (uses test markers `source: "test"`, `test_run_id`).
   - Frontend: `npm run test` and `npm run build`.
