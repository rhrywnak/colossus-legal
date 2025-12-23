# CLAUDE.md — Colossus-Legal (Claude Code Instructions)

> **This file is for Claude Code (Sonnet).** Read this FIRST before any task.

**Version:** 2.0  
**Updated:** 2025-12-20

---

## ⚠️ CRITICAL: Read Before ANY Work

1. **Read `docs/CLAUDE_CODE_INSTRUCTIONS.md`** — Full coding standards and process
2. **Read `docs/TASK_TRACKER.md`** — Current tasks and status
3. **Follow the Pre-Coding Process** — No code until approved

---

## Project Overview

**Colossus-Legal** is a legal case analysis system that:
- Stores claims, documents, persons, evidence in Neo4j as a knowledge graph
- Provides a REST API for data access (Rust/Axum backend)
- Provides a web interface for analysis (React/TypeScript frontend)
- Claims are extracted by Claude Opus (via chat), not by local tools

**Current Phase:** Phase 5 — Schema v2 + Claims Import

---

## Human Context

**Developer:** Roman
- 45 years IT experience, retired
- **Learning Rust** — explain patterns and concepts as you code
- Prefers clear explanations over terse code
- Values working code over perfect code

**Communication Style:**
- Explain Rust patterns when you use them (reference `docs/RUST-PATTERNS.md`)
- Add comments for non-obvious logic
- Keep functions small (<50 lines) and focused
- Keep modules small (<250 lines)
- Test incrementally

---

## Architecture

### Components

| Component | Technology | Port | Location |
|-----------|------------|------|----------|
| Backend | Rust + Axum | 3403 | `backend/` |
| Frontend | React + Vite + TS | 5473 | `frontend/` |
| Database | Neo4j 5.x | 7687 | `10.10.100.50` (Proxmox VM) |

### Data Flow

```
Legal Documents → Claude Opus (extraction) → JSON files
                                                ↓
                                          Import API
                                                ↓
                                             Neo4j
                                                ↓
                                          REST API
                                                ↓
                                           Frontend
```

---

## Repository Structure

```
colossus-legal/
├── CLAUDE.md                    # This file (read first)
├── AGENTS.md                    # Agent persona definitions
│
├── backend/                     # Rust API server
│   └── src/
│       ├── main.rs
│       ├── api/                 # HTTP handlers
│       ├── models/              # Data structures
│       └── services/            # Business logic
│
├── frontend/                    # React + Vite + TypeScript
│   └── src/
│
├── docs/                        # Documentation
│   ├── CLAUDE_CODE_INSTRUCTIONS.md  # ⚠️ MUST READ
│   ├── TASK_TRACKER.md              # Current tasks
│   ├── DEVELOPMENT_PROCESS.md       # Full workflow
│   ├── DATA_MODEL_v2.md             # Neo4j schema
│   ├── CLAIMS_IMPORT_WORKFLOW.md    # Import specification
│   ├── RUST-PATTERNS.md             # Rust pattern reference
│   ├── ARCHITECTURE.md
│   └── API_DESIGN.md
│
├── docker-compose.yml
└── Makefile
```

---

## The Golden Rules

```
1. cargo check after EVERY meaningful change
2. Never accumulate more than 10 errors
3. No module over 300 lines
4. No function over 50 lines
5. Plan first, edit approved files only, verify with git diff
6. STOP if reality diverges from claims
```

---

## Mandatory Pre-Coding Process

**YOU MUST follow this process for EVERY task:**

### 1. Acknowledge Task
```
Task ID: T5.X.X
Task Name: [name]
Branch: feature/P5-FX.X-description
Layer: L0/L1/L2/L3
```

### 2. Verify Prerequisites
```bash
git branch --show-current    # Correct branch?
git status                   # Clean working tree?
ls -la src/models/           # Files exist?
```

### 3. Read Existing Code
```bash
cat src/models/mod.rs        # What modules exist?
cat Cargo.toml               # What dependencies?
```

### 4. Provide Pre-Coding Analysis
Include:
- Task understanding
- Files to modify (verified they exist)
- Files to create
- **Rust patterns to implement** (with code examples)
- Tests to write
- Potential issues

### 5. STOP and Wait for Approval

**Do NOT write code until human says "Proceed."**

---

## Rust Coding Standards

### Required Patterns

```rust
// Error handling with thiserror
#[derive(Debug, thiserror::Error)]
pub enum MyError {
    #[error("Description: {0}")]
    Variant(String),
}

// Serde for all DTOs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MyStruct { ... }

// Enums with snake_case
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MyEnum { ... }

// Optional fields
#[serde(skip_serializing_if = "Option::is_none")]
pub field: Option<String>,

// Result propagation with context
use anyhow::{Context, Result};
let value = operation().context("Failed to do X")?;
```

### Forbidden Patterns

```rust
// ❌ NEVER use unwrap() in production
let x = option.unwrap();

// ❌ NEVER use expect() without justification
let x = option.expect("msg");

// ❌ NEVER use generic string errors
return Err("error".into());

// ❌ NEVER skip Debug derive
#[derive(Serialize)]  // Missing Debug!
pub struct Bad { }
```

### Size Limits

| Metric | Limit |
|--------|-------|
| Module lines | 250 (soft), 300 (hard) |
| Function lines | 50 max, prefer 20-30 |
| Nesting depth | 3 levels max |

---

## Post-Coding Requirements

After implementation:

```bash
# 1. Verify only approved files changed
git diff --name-only

# 2. Verify compilation
cargo build

# 3. Run tests
cargo test

# 4. Check for warnings
cargo clippy
```

Provide completion report with:
- Files modified/created
- Rust patterns implemented (with line numbers)
- Anti-patterns avoided checklist
- Build/test/clippy results
- Exit criteria status

---

## Layer System

| Layer | Name | Description |
|-------|------|-------------|
| L0 | Skeleton | Compiles, structure in place |
| L1 | Real Data | Happy path works |
| L2 | Validation | Error handling, validation |
| L3 | Integration | Advanced features, polish |

**Never skip layers.** Complete L0 before L1, etc.

---

## Key Commands

```bash
# Backend
cd backend
cargo check          # Quick compile check
cargo build          # Full build
cargo test           # Run tests
cargo clippy         # Lint
cargo fmt            # Format

# Frontend
cd frontend
npm run dev          # Dev server (port 5473)
npm run build        # Production build

# Git
git branch --show-current
git status
git diff --name-only
```

---

## Neo4j Connection

- **URI:** `bolt://10.10.100.50:7687`
- **Browser:** `http://10.10.100.50:7474`
- **Crate:** `neo4rs`

---

## Key Documents

| Document | Purpose | When to Read |
|----------|---------|--------------|
| `docs/CLAUDE_CODE_INSTRUCTIONS.md` | Full process and standards | Before ANY task |
| `docs/TASK_TRACKER.md` | Task status | Before starting work |
| `docs/DATA_MODEL_v2.md` | Neo4j schema | When working on models |
| `docs/RUST-PATTERNS.md` | Rust patterns | When writing Rust |
| `docs/CLAIMS_IMPORT_WORKFLOW.md` | Import spec | When working on import |

---

## Safety Rules

1. **Verify files exist** before claiming they do (`ls`, `cat`)
2. **Confirm branch** before making changes
3. **Stay within task scope** — don't expand beyond what's asked
4. **Never skip layers** — L0 before L1, etc.
5. **Stop if reality diverges** — enter forensic mode

---

## Forensic Mode

If something is wrong (file doesn't exist, unexpected diff, etc.):

1. **STOP all edits**
2. **Report the divergence**
3. **Only read operations allowed** (`ls`, `cat`, `git status`)
4. **Wait for instructions**

---

## What NOT To Do

❌ Redesign architecture without approval  
❌ Add features not in the task spec  
❌ Skip pre-coding analysis  
❌ Write code before approval  
❌ Modify files not in approved list  
❌ Use `unwrap()` in production code  
❌ Create oversized modules (>300 lines)  
❌ Continue to next task without instruction  

---

# End of CLAUDE.md
