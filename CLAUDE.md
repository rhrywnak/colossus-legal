# CLAUDE.md — Colossus-Legal

> **Read this FIRST.** Then read `docs/CLAUDE_CODE_INSTRUCTIONS.md` for full standards.

## Project

**Colossus-Legal** — Legal case analysis system  
- **Backend:** Rust + Axum (port 3403) → `backend/`
- **Frontend:** React + Vite + TS (port 5473) → `frontend/`
- **Database:** Neo4j 5.x → `bolt://10.10.100.50:7687`

**Current Phase:** Phase 2 — Query Layer  
**Current Feature:** F2.1 — Schema Discovery Endpoint

---

## Human Context

**Developer:** Roman — 45 years IT, retired, learning Rust.
- Explain patterns when you use them
- Reference `docs/RUST-PATTERNS.md` for pattern examples
- Clear explanations over terse code
- Working code over perfect code

---

## The Golden Rules

```
1. cargo check after EVERY change
2. Never accumulate more than 10 errors
3. No module over 300 lines
4. No function over 50 lines
5. Pre-Coding Analysis BEFORE any code
6. Wait for "Proceed" before implementing
```

---

## Mandatory Pre-Coding Process

**For EVERY task, provide Pre-Coding Analysis first:**

```markdown
## Pre-Coding Analysis for [Task ID]

### Task Understanding
[What will be implemented]

### Branch Verification
- Current: `feature/xxx`
- Clean: YES/NO

### Files to Modify
| File | Changes |
|------|---------|

### Files to Create  
| File | Purpose | Est. Lines |
|------|---------|------------|

### Rust Patterns to Implement
| Pattern | Example |
|---------|---------|

### Tests to Write
| Test Name | Description |
|-----------|-------------|

### Potential Issues
[Any concerns]
```

**STOP. Wait for "Proceed" before writing code.**

---

## Post-Coding Requirements

```bash
git diff --name-only    # Only approved files?
cargo build             # Compiles?
cargo test              # Tests pass?
cargo clippy            # No warnings?
```

Provide completion report with build/test results.

---

## Key Documents

| Document | When to Read |
|----------|--------------|
| `docs/CLAUDE_CODE_INSTRUCTIONS.md` | Before ANY task |
| `docs/TASK_TRACKER.md` | Check task status |
| `docs/DATA_MODEL_v3.md` | Working on models |
| `docs/CLAIMS_IMPORT_WORKFLOW.md` | Working on import |
| `docs/RUST-PATTERNS.md` | Writing Rust code |
| `docs/DEVELOPMENT_PROCESS.md` | Full workflow |

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
    #[error("message: {0}")]
    Variant(String),
}

// ✅ Optional fields
#[serde(skip_serializing_if = "Option::is_none")]
pub field: Option<String>,

// ❌ NEVER use
option.unwrap()      // Use ? or match
"error".into()       // Use typed errors
```

---

## Commands

```bash
# Backend
cd backend && cargo check    # Quick check
cd backend && cargo test     # Run tests
cd backend && cargo clippy   # Lint

# Git
git branch --show-current
git status
git diff --name-only
```

---

## What NOT To Do

❌ Write code before Pre-Coding Analysis approved  
❌ Modify files not in approved list  
❌ Add features not in task spec  
❌ Use `unwrap()` in production code  
❌ Create modules over 300 lines  
❌ Skip layers (must do L0 before L1)  

---

## If Something Goes Wrong

**STOP all edits.** Report the issue. Read-only operations only until resolved.

---

## Layer System

| Layer | Description |
|-------|-------------|
| L0 | Skeleton — compiles, structure in place |
| L1 | Real Data — happy path works |
| L2 | Validation — error handling complete |
| L3 | Integration — advanced features |

Never skip layers.

---

# End of CLAUDE.md
