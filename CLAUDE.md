# CLAUDE.md — colossus-legal

> **Read this FIRST before any task.**

## Project

**Colossus-Legal** — Legal document analysis and case management system (Awad v. CFS/Phillips)
- **Backend:** Rust + Axum (port 3403) → `backend/`
- **Frontend:** React + Vite + TS (port 5473) → `frontend/`
- **Pipeline DB:** PostgreSQL `colossus_legal_v2` (DEV: `10.10.100.200`)
- **Graph DB:** Neo4j 5.x (DEV: `bolt://10.10.100.200:7687`)
- **Vector DB:** Qdrant (DEV: REST `http://10.10.100.200:6333`, gRPC `:6334`)
- **Auth:** Authentik SSO → Traefik ForwardAuth → X-authentik-* headers
- **Shared Libraries:** colossus-rs workspace (colossus-auth, colossus-rag, colossus-pipeline, colossus-extract)

### Current phase

**Phase PV — colossus-pipeline integration (P4-P5)**
Design doc: COLOSSUS_PIPELINE_DESIGN_v5_2.md
Task tracker: COLOSSUS_PIPELINE_TASK_TRACKER_v1_1.md
Branch: feature/pipeline-v5

Phase 0 (cleanup) is complete. Phases 1-3 are colossus-rs work.
colossus-legal work begins at Phase 4 (step implementations) and Phase 5 (HTTP API + main.rs).

---

## Human Context

**Developer:** Roman — 45 years IT, CS degree, retired, learning Rust.
- Explain every Rust pattern you use with a `## Rust Learning:` doc comment
- Clear explanations over terse code
- Working code over perfect code

---

## The Golden Rules

```
 1. cargo check after EVERY change
 2. Never accumulate more than 10 errors
 3. No module over 300 lines (code lines, excluding doc comments)
 4. No function over 50 lines
 5. Tests MUST pass before cargo build — a clean compile is NOT verification
 6. Never bump version numbers — Roman does that
 7. Every module, struct, trait, and public function MUST have a doc comment
    explaining what it does AND why it exists in this system
 8. No magic strings or numbers — use constants from backend/src/pipeline/constants.rs
 9. No .unwrap() or .expect() in production handlers — use ? or AppError
10. No plaintext secrets in code, config, or Butane files
11. No :latest tags on container images
12. Single repo only — never reference files in colossus-rs or other repos
13. Audit before deploying — verify the full path, not just the component
14. Steps accept &AppContext — never &AppState
15. Call context.llm_provider.invoke() — never AnthropicChunkExtractor directly
16. Call context.embedding_provider.embed() — never rig_fastembed directly
```

---

## Doc Comment Requirement (MANDATORY)

Every file must have a `//!` module doc comment at the top:

```rust
//! backend/src/pipeline/steps/llm_extract.rs
//!
//! LlmExtract step — calls LlmProvider::invoke() per chunk and stores results.
//!
//! One-sentence description of WHY this step exists.
//!
//! ## Rust Learning: [pattern name]
//!
//! Explanation of the key Rust pattern used in this module.
```

Every public struct, enum, trait, and function must have a `///` doc comment.

---

## Deployment & Configuration Rules

### Secrets
- **NEVER** hardcode passwords, API keys, or tokens anywhere
- Secrets belong in Ansible Vault (`vault.yml`) or gitignored `.env` files
- If a secret is accidentally committed, rotate it immediately

### Environment Variables
- All config values that differ between DEV and PROD must be env vars
- Every new env var must be added to the Ansible template (`colossus-legal-backend.env.j2`)
- When adding a new env var: update `config.rs` + Ansible template + group_vars + vault (if secret)

### Timeouts (MANDATORY)
- **Frontend:** Every `authFetch` call must use `AbortController` with a timeout signal
  - Normal endpoints: 30 seconds
  - `/ask` (RAG synthesis): 90 seconds
- **Backend:** Every `reqwest::Client` must be built with `.timeout()` and `.connect_timeout()`
- **Backend:** Share one `reqwest::Client` via AppState — do not create per-request

### Container Images
- Always pin to specific version tags, never `:latest`
- Version in `/health` or `/api/status` must use `env!("CARGO_PKG_VERSION")`

### Deploy sequence (never deviate)
1. `git add -A && git commit`
2. `./scripts/bump-version.sh {ver}` → commit → tag → push with tags
3. `cd ~/Projects/colossus-ansible && ./scripts/build-release.sh v{ver}`
4. Deploy via Semaphore web UI only — never `ansible-playbook` directly

---

## Pre-Coding Process

For every task, report these before writing any code:

```
### Files to read (report contents before modifying)
### Files to modify (exact paths)
### Files to create (exact paths)
### Env vars / config changes
### Tests to write (names and what they verify)
### Deployment impact
### Potential issues
```

STOP after reporting. Wait for explicit approval before writing code.

---

## Post-Coding Requirements

```bash
cargo test              # Tests pass FIRST
cargo build             # Then build
git diff --name-only    # Only approved files changed?
cargo clippy            # No new warnings?
```

Provide completion report: commit hash, test count, error/warning count.

Before marking any task DONE:
- If new env vars added → confirm Ansible template updated
- If new endpoints added → confirm frontend calls use timeout
- If new HTTP clients created → confirm timeout configured

---

## Pipeline-Specific Rules (Phase 4-5)

- All status values come from `DOC_STATUS_*` constants in `backend/src/pipeline/constants.rs`
- All step names come from `step_name_of::<T>()` — never manually typed strings
- Steps must be idempotent — check before doing work, every time
- Steps must check `cancel.is_cancelled()` between expensive operations
- No `tokio::spawn` inside step implementations — steps are called by the Worker
- All multi-table writes inside steps must use PostgreSQL transactions
- `on_cancel()` and `on_delete()` must reverse all external side effects

---

## Rust Quick Reference

```rust
// ✅ Required derives
#[derive(Debug, Clone, Serialize, Deserialize)]

// ✅ Enums with snake_case
#[serde(rename_all = "snake_case")]

// ✅ Error handling
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("descriptive message: {0}")]
    Variant(String),
}

// ✅ Optional fields
#[serde(skip_serializing_if = "Option::is_none")]
pub field: Option<String>,

// ✅ HTTP client with timeout (MANDATORY)
let client = reqwest::Client::builder()
    .timeout(Duration::from_secs(30))
    .connect_timeout(Duration::from_secs(5))
    .build()?;

// ✅ Use constants, never literals
use crate::pipeline::constants::DOC_STATUS_COMPLETED;

// ✅ Version from Cargo.toml
version: env!("CARGO_PKG_VERSION"),

// ❌ NEVER
option.unwrap()              // Use ? or match
"COMPLETED".to_string()      // Use DOC_STATUS_COMPLETED constant
reqwest::Client::new()       // Use builder with timeout
AnthropicChunkExtractor      // Use context.llm_provider.invoke()
AppState in step args        // Use &AppContext
tokio::spawn in steps        // Steps are synchronous units of work
```

---

## Infrastructure Reference

| Role | IP |
|------|----|
| DEV app | 10.10.100.220 |
| DEV DB | 10.10.100.200 |
| PROD app | 10.10.100.120 |
| PROD DB | 10.10.100.110 |

DEV PostgreSQL access:
```bash
ssh core@10.10.100.200 "sudo podman exec colossus-postgres psql \
  -U postgres -d colossus_legal_v2 -c \"YOUR QUERY\""
```

Logging rule: NEVER `journalctl` without `-n 5` or less AND `grep` with specific search term.

---

## What NOT To Do

❌ Write code before pre-coding analysis approved
❌ Modify files not in approved list
❌ Add features not in task spec
❌ Use `unwrap()` or `expect()` in production handlers
❌ Create modules over 300 lines
❌ Bump version numbers — Roman does that
❌ Use string literals where constants exist
❌ Create HTTP clients without timeouts
❌ Create fetch calls without AbortController
❌ Hardcode secrets, passwords, or API keys
❌ Use `:latest` tags on container images
❌ Reference files in colossus-rs, colossus-ansible, or colossus-homelab
❌ Use AppState in step implementations — use AppContext
❌ Call AnthropicChunkExtractor — use context.llm_provider.invoke()
❌ Call rig_fastembed directly — use context.embedding_provider.embed()
❌ tokio::spawn inside step implementations

---

## If Something Goes Wrong

**STOP all edits.** Report the exact compiler error or test failure.
Read-only operations only until the issue is understood.
Never fix a test to make it pass — fix the code.

---

## Commands

```bash
# Backend
cd backend && cargo check
cd backend && cargo test
cd backend && cargo build
cd backend && cargo clippy

# Frontend
cd frontend && npm run build

# Git
git branch --show-current    # Must return: feature/pipeline-v5
git status
git diff --name-only
```

---

# End of CLAUDE.md
