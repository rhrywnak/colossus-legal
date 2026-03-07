# Audit Report: colossus-legal

**Date:** 2026-03-06
**Auditor:** Claude Code
**Branch:** `feature/rag-adoption`
**Cargo.toml version:** 0.5.2

---

## Summary

**31 findings:** 3 CRITICAL, 5 HIGH, 8 MEDIUM, 10 LOW, 5 INFO

| Severity | Count |
|----------|-------|
| CRITICAL | 3 |
| HIGH     | 5 |
| MEDIUM   | 8 |
| LOW      | 10 |
| INFO     | 5 |

---

## Findings

---

### [CRITICAL] F-01: Large `.onnx` model file tracked in git

**File:** `backend/.fastembed_cache/models--nomic-ai--nomic-embed-text-v1.5/snapshots/.../onnx/model.onnx`
**Code:** (binary file, ~523 MB)
**Issue:** The fastembed ONNX model was committed to the repository. This bloats the repo permanently (git history retains it even after deletion). `.fastembed_cache/` is not in `.gitignore`.
**Recommendation:**
1. Add `.fastembed_cache/` to `.gitignore`
2. Remove from tracking: `git rm -r --cached backend/.fastembed_cache/`
3. Consider `git filter-repo` or BFG Repo Cleaner to purge from history
4. Document that the model downloads automatically on first run

---

### [CRITICAL] F-02: No timeouts on any frontend fetch calls

**File:** `frontend/src/services/*.ts` (21+ fetch calls across all service files)
**Code:**
```ts
// auth.ts — the shared wrapper has no timeout
const response = await fetch(url, { credentials: 'include', ...init });
```
**Issue:** Every `authFetch` call (and the raw `fetch` in `api.ts`) has no `AbortController`, no `signal`, and no timeout. If the backend hangs, the UI hangs indefinitely with no way to recover.
**Affected files:** `schema.ts`, `contradictions.ts`, `analysisApi.ts`, `queries.ts`, `allegations.ts`, `personDetail.ts`, `evidence.ts`, `graph.ts`, `caseSummary.ts`, `harms.ts`, `case.ts`, `decomposition.ts`, `documents.ts`, `motionClaims.ts`, `ask.ts`, `evidenceChain.ts`, `api.ts`, `claims.ts`, `search.ts`, `persons.ts`
**Recommendation:** Add a timeout wrapper to `authFetch` using `AbortController`:
```ts
const controller = new AbortController();
const timeoutId = setTimeout(() => controller.abort(), 30000);
try {
  const response = await fetch(url, { ...init, signal: controller.signal });
  ...
} finally {
  clearTimeout(timeoutId);
}
```

---

### [CRITICAL] F-03: No timeouts on backend reqwest clients

**File:** `backend/src/api/search.rs:119`, `backend/src/api/embed.rs:43`
**Code:**
```rust
let http_client = reqwest::Client::new();
```
**Issue:** `reqwest::Client::new()` creates an HTTP client with no timeout configured. Calls to Qdrant REST API or embedding service could hang indefinitely. Additionally, a new client is created per request instead of being shared (connection pool not reused).
**Recommendation:**
```rust
// In AppState or a shared builder:
let http_client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(5))
    .build()?;
```

---

### [HIGH] F-04: Stale version string "0.1.0" in api_status

**File:** `backend/src/main.rs:34`
**Code:**
```rust
version: "0.1.0",
```
**Issue:** The `/api/status` endpoint returns a hardcoded `"0.1.0"` while `Cargo.toml` declares `0.5.2`. This misleads any monitoring or health-check tooling.
**Recommendation:** Use `env!("CARGO_PKG_VERSION")`:
```rust
version: env!("CARGO_PKG_VERSION"),
```

---

### [HIGH] F-05: `.fastembed_cache/` not in `.gitignore`

**File:** `.gitignore`
**Code:** (missing entry)
**Issue:** Without this entry, `cargo run` or tests will re-download the ONNX model into `backend/.fastembed_cache/` and `git add .` will re-commit it.
**Recommendation:** Add to `.gitignore`:
```
.fastembed_cache/
```

---

### [HIGH] F-06: `claude_client` service is dead code

**File:** `backend/src/services/claude_client.rs`, `backend/src/services/mod.rs:4`
**Code:**
```rust
pub mod claude_client;
```
**Issue:** `claude_client` is declared in `services/mod.rs` but never imported or used outside the `services/` directory. It was superseded by `colossus-rag`'s `RigSynthesizer`. Dead code adds maintenance burden and confusion.
**Recommendation:** Remove `claude_client.rs` and its `pub mod` declaration from `mod.rs`.

---

### [HIGH] F-07: Dead `health_check` function with `#[allow(dead_code)]`

**File:** `backend/src/main.rs:26-28`
**Code:**
```rust
#[allow(dead_code)]
async fn health_check() -> &'static str {
    "OK"
}
```
**Issue:** This shadow function is suppressed with `#[allow(dead_code)]` because the real health check is in `api/mod.rs:82`. The `#[allow(dead_code)]` annotation masks the problem instead of fixing it.
**Recommendation:** Delete the dead function from `main.rs`.

---

### [HIGH] F-08: `.unwrap()` in production handler

**File:** `backend/src/api/documents.rs:196`
**Code:**
```rust
.body(body)
.unwrap())
```
**Issue:** `unwrap()` in `get_document_file()` — a production request handler. If the `http::response::Builder` fails (e.g., invalid header state), this panics and crashes the request.
**Recommendation:** Replace with `?` or `.map_err(|e| ...)` to return a proper 500 error.

---

### [MEDIUM] F-09: Hardcoded CORS fallback with LAN IP

**File:** `backend/src/main.rs:91-93`
**Code:**
```rust
.unwrap_or_else(|_| {
    "http://localhost:5473,http://localhost:3403,http://10.10.0.99:5473".to_string()
})
```
**Issue:** If `CORS_ALLOWED_ORIGINS` is unset in production, CORS allows three origins including a specific LAN IP (`10.10.0.99`). This is a dev convenience that could become a security issue.
**Recommendation:** Log a warning when using the fallback. Consider making `CORS_ALLOWED_ORIGINS` required in production (panic if not set when `DEPLOY_ENV=production`).

---

### [MEDIUM] F-10: Fragile Qdrant gRPC port derivation

**File:** `backend/src/main.rs:187`
**Code:**
```rust
let qdrant_grpc_url = config.qdrant_url.replace(":6333", ":6334");
```
**Issue:** Derives gRPC URL by string-replacing the REST port. Breaks if the URL contains `:6333` elsewhere (e.g., in a path) or if Qdrant uses non-standard ports.
**Recommendation:** Add a separate `QDRANT_GRPC_URL` config variable, or parse the URL properly and increment the port.

---

### [MEDIUM] F-11: Hardcoded default Claude model

**File:** `backend/src/config.rs:40`
**Code:**
```rust
.unwrap_or_else(|_| "claude-sonnet-4-20250514".to_string());
```
**Issue:** Model name is hardcoded as fallback. When Claude releases a newer model, this default becomes stale silently.
**Recommendation:** This is acceptable as a reasonable default, but document it and consider logging when the default is used.

---

### [MEDIUM] F-12: Mixed `/api/` prefix pattern on routes

**File:** `backend/src/api/mod.rs`, `backend/src/main.rs:131`
**Code:**
```rust
// In mod.rs:
.route("/api/me", get(me_handler))
.route("/api/logout", get(logout::logout))
.route("/health", get(health_check))
.route("/claims", get(list_claims))
// ... 28 more routes without /api/ prefix

// In main.rs:
.route("/api/status", get(api_status))
```
**Issue:** 3 routes use `/api/` prefix (`/api/me`, `/api/logout`, `/api/status`), while 31 routes do not. This inconsistency makes it harder to set up reverse proxies or apply middleware by path prefix.
**Recommendation:** Either prefix all routes with `/api/` using `.nest("/api", router)`, or remove the prefix from the 3 that have it. Be consistent.

---

### [MEDIUM] F-13: `queries/:id/run` uses `Option<AuthUser>`

**File:** `backend/src/api/queries.rs:26`
**Code:**
```rust
pub async fn run_query(_user: Option<AuthUser>, ...)
```
**Issue:** The `run_query` handler executes arbitrary saved queries against Neo4j but does not require authentication (`Option<AuthUser>` means unauthenticated users can call it). This is a potential data exposure risk.
**Recommendation:** Change to `AuthUser` (required) and add appropriate group check (e.g., `require_ai` or `require_edit`).

---

### [MEDIUM] F-14: Health check is shallow — no downstream verification

**File:** `backend/src/api/mod.rs:82`
**Code:**
```rust
async fn health_check(State(_state): State<AppState>) -> (StatusCode, &'static str) {
    (StatusCode::OK, "OK")
}
```
**Issue:** Health check returns OK without verifying Neo4j, Qdrant, or the RAG pipeline are reachable. Container orchestrators will see "healthy" even when the backend can't serve requests.
**Recommendation:** Add a "deep" health check that pings Neo4j (`RETURN 1`) and optionally Qdrant. Keep the shallow check for liveness, add a `/health/ready` for readiness.

---

### [MEDIUM] F-15: `reqwest::Client` created per request (not shared)

**File:** `backend/src/api/search.rs:119`, `backend/src/api/embed.rs:43`
**Code:**
```rust
let http_client = reqwest::Client::new();
```
**Issue:** A new `reqwest::Client` is created on every request. This wastes the connection pool, DNS cache, and TLS session cache that `reqwest::Client` maintains internally.
**Recommendation:** Create one `reqwest::Client` at startup and store it in `AppState`.

---

### [MEDIUM] F-16: `.expect()` in logout handler on dynamic value

**File:** `backend/src/api/logout.rs:32`
**Code:**
```rust
HeaderValue::from_str(cookie).expect("valid cookie header")
```
**Issue:** While the cookie string is constructed from known parts, using `.expect()` in a request handler means a malformed cookie value would panic the task. Low actual risk but violates the project's "no unwrap in production" rule.
**Recommendation:** Replace with `?` or `.map_err()`.

---

### [LOW] F-17: Hardcoded default Qdrant URL in config

**File:** `backend/src/config.rs:29`
**Code:**
```rust
.unwrap_or_else(|_| "http://localhost:6333".to_string());
```
**Issue:** Fallback Qdrant URL includes hardcoded port. Acceptable for dev, but could cause confusion in production if env var is accidentally unset.
**Recommendation:** Log a warning when using the default.

---

### [LOW] F-18: Hardcoded frontend API fallback

**File:** `frontend/src/services/api.ts:50`
**Code:**
```ts
|| "http://localhost:3403";
```
**Issue:** Final fallback for `API_BASE_URL` is hardcoded localhost with port. Standard for dev but worth documenting.
**Recommendation:** Acceptable as-is. Document in deployment guide that `VITE_API_URL` must be set.

---

### [LOW] F-19: `logout` handler has no auth check

**File:** `backend/src/api/logout.rs:13`
**Code:**
```rust
pub async fn logout(State(state): State<AppState>) -> impl IntoResponse {
```
**Issue:** No `AuthUser` extractor — anyone can hit `/api/logout`. Low risk since logout is idempotent and only clears a cookie.
**Recommendation:** Acceptable as-is. Logout should be accessible even with expired sessions.

---

### [LOW] F-20: Multiple `pub async fn` handlers use `Option<AuthUser>` for read endpoints

**File:** Various — `persons.rs`, `contradictions.rs`, `analysis.rs`, `graph.rs`, `documents.rs` (list/get/file), `allegations.rs`, `evidence.rs`, `decomposition.rs`, `case_summary.rs`, `schema.rs`, `evidence_chain.rs`, `case.rs`, `harms.rs`, `claims.rs` (list/get)
**Issue:** All read-only endpoints accept unauthenticated access. This is by design but worth documenting — if auth is required later, every handler needs updating.
**Recommendation:** Document this as an intentional design decision. Consider a middleware-based approach for easier future changes.

---

### [LOW] F-21: Startup `.expect()` calls in main.rs

**File:** `backend/src/main.rs:50,54,58,80,137,139`
**Code:**
```rust
AppConfig::from_env().expect("Failed to load configuration");
create_neo4j_graph(&config).await.expect("Failed to connect to Neo4j");
// ... etc
```
**Issue:** Multiple `.expect()` calls at startup. This is **standard Rust practice** — panicking on misconfiguration at startup is the correct behavior (fail fast).
**Recommendation:** No action needed. These are idiomatic.

---

### [LOW] F-22: `colossus-rag` git dependency pinned to tag

**File:** `backend/Cargo.toml:14`
**Code:**
```toml
colossus-rag = { git = "https://github.com/rhrywnak/colossus-rs.git", tag = "v0.1.0-rag", features = ["full"] }
```
**Issue:** Pinned to `tag = "v0.1.0-rag"` which is the initial release. The comment on line 13 says "Once merged to main, change this to `branch = \"main\"`" but this hasn't been done.
**Recommendation:** Update to `branch = "main"` or the latest tag once colossus-rag changes are merged.

---

### [LOW] F-23: Old services still needed for non-RAG endpoints

**File:** `backend/src/services/` (all files except `claude_client.rs`)
**Issue:** `embedding_service.rs`, `qdrant_service.rs`, `graph_expander.rs`, `embedding_pipeline.rs` are still used by `/search`, `/admin/embed-all`, and other endpoints. These duplicate functionality now in `colossus-rag` but can't be removed until those endpoints are migrated.
**Recommendation:** Track as tech debt. Plan migration of `/search` and `/admin/embed-all` to use colossus-rag components.

---

### [LOW] F-24: Stale remote branches

**Issue:** Multiple remote branches exist that may be merged or abandoned. Not checked in detail (requires git remote access).
**Recommendation:** Periodically prune merged branches: `git branch -r --merged main | grep -v main | xargs git push origin --delete`

---

### [LOW] F-25: `#[cfg(test)]` model string

**File:** `backend/src/models/import.rs:221`
**Code:**
```rust
"extraction_model":"claude-3-opus"
```
**Issue:** Hardcoded model name in test fixture. Only affects tests but could cause confusion if model names change.
**Recommendation:** No action needed — test fixture data.

---

### [INFO] F-26: CORS configuration is properly configurable

**File:** `backend/src/main.rs:84-105`
**Issue:** None — CORS uses `CORS_ALLOWED_ORIGINS` env var. Just noting that credentials are allowed and origins are explicitly listed (not `any()`). Good practice.
**Recommendation:** No action needed.

---

### [INFO] F-27: Auth integration is feature-complete

**File:** `backend/src/api/*.rs`
**Issue:** None — write endpoints require `AuthUser` with role checks (`require_admin`, `require_ai`, `require_edit`). Read endpoints use `Option<AuthUser>` by design.
**Recommendation:** No action needed.

---

### [INFO] F-28: Backend Dockerfile exists with `.dockerignore`

**File:** `backend/Dockerfile`, `backend/.dockerignore`
**Issue:** None — both exist. Docker build is configured.
**Recommendation:** No action needed.

---

### [INFO] F-29: Frontend Dockerfile exists with `.dockerignore`

**File:** `frontend/Dockerfile`, `frontend/.dockerignore`
**Issue:** None — both exist.
**Recommendation:** No action needed.

---

### [INFO] F-30: Tracing is present in API handlers

**File:** `backend/src/api/*.rs`
**Issue:** None — `tracing::info`, `tracing::warn`, `tracing::error` are used across handlers. Logging coverage is reasonable.
**Recommendation:** No action needed.

---

### [INFO] F-31: `.gitignore` covers essentials

**File:** `.gitignore`
**Issue:** Covers `target/`, `node_modules/`, `.env` files. Missing `.fastembed_cache/` (see F-05).
**Recommendation:** See F-05.

---

## Priority Action Items

| Priority | Finding | Effort |
|----------|---------|--------|
| 1 | F-01: Remove `.onnx` from git, add `.fastembed_cache/` to `.gitignore` | 30 min |
| 2 | F-02: Add timeout wrapper to `authFetch` | 15 min |
| 3 | F-03: Add timeout to backend reqwest clients + share client via AppState | 30 min |
| 4 | F-04: Use `env!("CARGO_PKG_VERSION")` for status endpoint | 2 min |
| 5 | F-08: Replace `.unwrap()` in documents.rs with `?` | 5 min |
| 6 | F-06: Remove dead `claude_client` module | 5 min |
| 7 | F-07: Remove dead `health_check` from main.rs | 2 min |
| 8 | F-12: Standardize route prefix pattern | 30 min |
| 9 | F-13: Require auth on `run_query` endpoint | 5 min |
| 10 | F-14: Add deep health check | 30 min |
