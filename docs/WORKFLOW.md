# Colossus-Legal – WORKFLOW.md (Layered, Breadth-First, Option B)

This document defines the **operational engineering workflow** for the Colossus-Legal project.

It connects:
- Phases → Tasks → Layers (L0–L3)
- Human roles → Codex Agents
- Branching → Versioning → Documentation

Codex v0.63 and all developers MUST follow this workflow.

---

## 1. Goals of This Workflow

1. Keep `main` always:
   - Build-clean (backend + frontend)
   - Runnable end-to-end
   - Safe for new Codex sessions

2. Avoid:
   - Giant refactors with hundreds of errors
   - Features that are “almost done” but break the repo
   - Deep implementation in one area while the rest is brittle

3. Enable:
   - Breadth-first, **layered** feature development
   - Frequent deployable increments
   - Clear hand-off between human and Codex work
   - Traceable progress via Task IDs and docs

---

## 2. Core Concepts

### 2.1 Phases

High-level stages of the project over time. Defined in:

- `docs/PHASE_PLAN.md`

Examples:
- Phase 1 – Foundations
- Phase 2 – Claims API v1
- Phase 3 – Core Graph
- ...

### 2.2 Tasks (T-IDs)

Atomic units of work, each with a Task ID:

- Format: `T<phase>.<number>` (e.g., `T2.1`, `T2.1a`)
- Defined and tracked in:
  - `docs/TASK_TRACKER.md`
  - Optionally, dedicated task files under `docs/tasks/`

### 2.3 Layers (L0–L3)

Each feature/domain is implemented in **layers**:

- **L0 – Skeleton**
  - Routes, pages, DTOs exist and compile.
  - Stubs / mock data allowed.
  - End-to-end shell present.

- **L1 – Real Data (Happy Path)**
  - Real data flow:
    - Backend → Neo4j → Backend → Frontend.
  - Minimal validation & error handling.
  - Feature usable for basic flows.

- **L2 – Validation, Errors, Relationships**
  - Strong input validation.
  - Clear error responses.
  - Relationship endpoints and behavior.

- **L3 – Analysis, AI, UX Polish**
  - Analysis endpoints and graph queries.
  - AI suggestion flows.
  - UI dashboards, timelines, polish.

Each layer is **independently deployable** and versioned.

### 2.4 Agents

Codex operates as an **Agent persona**, defined in `AGENTS.md`:

- `BackendAgent` – Rust + Axum + Neo4j, `backend/` only.
- `FrontendAgent` – React/Vite/TS, `frontend/` only.
- `DocsAgent` – docs only.
- `RefactorAgent` – small, local refactors only.

---

## 3. End-to-End Workflow Overview

This is the main lifecycle for any new feature or task.

1. **Decide “where we are”**  
   - Read `PHASE_PLAN.md` to know the current Phase.
   - Read `TASK_TRACKER.md` to see which Task IDs are next.

2. **Select a Task ID and Layer**  
   - Example: `T2.1a – Claims API L0`.
   - Determine:
     - Phase (2 – Claims)
     - Layer (L0 – skeleton)
     - Agent persona (BackendAgent)

3. **Locate or Create the Task File**  
   - Example: `docs/tasks/T2.1a_Claims_API_L0.md`
   - This file contains:
     - title, instructions, context, requirements
     - acceptance criteria
     - file list

4. **Create a Feature Branch**  
   - Always start from `main`:

     ```bash
     git switch main
     git pull
     git switch -c feature/<task-id>   # e.g., feature/T2.1a-claims-api-l0
     ```

5. **Codex Execution (Bounded)**  
   - Codex must read:
     - `AGENTS.md`
     - `docs/DEV_ONBOARDING.md`
     - `docs/TASK_TRACKER.md`
     - `docs/ARCHITECTURE.md`
     - `docs/API_DESIGN.md`
     - `docs/DATA_MODEL.md`
     - `docs/tasks/<TaskID>.md`
   - Codex performs only the work described in that one task file and:
     - Stays within the assigned Layer.
     - Stays within the Agent’s scope.

6. **Implement in Small Steps**  
   - After each small chunk of work:
     - Backend: `cargo check --manifest-path backend/Cargo.toml`
     - Frontend: `npm run build` (for bigger changes), `npm run dev` for manual testing.

7. **Verify**  
   - Run the app and manually exercise the new behavior:
     - Backend: `cargo run` + `curl` or browser.
     - Frontend: `npm run dev` + browser.

8. **Update Docs**  
   - Update `docs/TASK_TRACKER.md`:
     - Mark the Task ID status (IN_PROGRESS → DONE).
     - Record notes if needed.
   - Update any impacted docs (API_DESIGN, DATA_MODEL, ARCHITECTURE).

9. **Commit & Merge**  
   - Commit with a clear message (one logical change per commit).
   - Merge back to main (no-FF or via PR).
   - Push changes.

10. **Tag Layer Version (Optional but Recommended)**  
    - Tag the repository with a Layer-aware tag, e.g.:

      ```bash
      git tag v0.2.0-claims-L0
      git push origin --tags
      ```

---

## 4. Layered Execution in Practice

### 4.1 Example: Claims API

**Tasks:**

- `T2.1a – Claims API L0 (Skeleton)`  
- `T2.1b – Claims API L1 (Real Neo4j list)`  
- `T2.1c – Claims API L2 (Validation + errors)`  
- `T2.1d – Claims API L3 (Analysis queries)`

Each is its own Codex task file in `docs/tasks/`.

**Execution:**

1. Implement `T2.1a`:
   - GET `/claims` exists.
   - Returns stub array of ClaimDto.
   - Tag: `v0.2.0-claims-L0`.

2. Implement `T2.1b`:
   - GET `/claims` uses Neo4j.
   - Real Claim nodes returned.
   - Tag: `v0.2.0-claims-L1`.

3. Implement `T2.1c`:
   - Add validation, proper errors.
   - Tag: `v0.2.0-claims-L2`.

4. Implement `T2.1d`:
   - Add analysis endpoints (paths, refuted claims).
   - Tag: `v0.2.0-claims-L3`.

At each step, the system is in a **valid, deployable state**.

---

## 5. Documentation Roles in the Workflow

### 5.1 Foundation Docs

- `AGENTS.md`
  - Defines Codex personas and global rules.
- `DEV_ONBOARDING.md`
  - Explains development best practices, module size, tests, and patterns.
- `PHASE_PLAN.md`
  - Explains the global roadmap and phase ordering.
- `TASK_TRACKER.md`
  - Lists all Task IDs, statuses, and high-level descriptions.

### 5.2 Architecture & Design Docs

- `ARCHITECTURE.md`
  - Overall component structure and tutorial-style examples.
- `API_DESIGN.md`
  - Endpoint contracts, shapes, and examples.
- `DATA_MODEL.md`
  - Graph schema with Cypher examples.

### 5.3 Implementation Docs

- `FIRST_3_TASKS_FOR_CODEX.md`
  - Initial, safe tasks to bootstrap Claims feature.
- `docs/tasks/*.md`
  - Detailed specs for each Task ID, Codex-ready.

### 5.4 Index & Meta Docs

- `DOCUMENTATION_INDEX.md`
  - Maps and explains all docs.
- `WORKFLOW.md` (this file)
  - Defines this entire process.

---

## 6. Branching & Naming Conventions

### 6.1 Branch Names

Use clear, Task-based branch names:

- `feature/T2.1a-claims-api-l0`
- `feature/T2.1b-claims-api-l1`
- `feature/T2.2a-claims-ui-l0`

Format:

- `feature/<TaskID>-<short-description>`

### 6.2 Commit Messages

Use short, imperative commit messages:

- `feat: add stub GET /claims handler`
- `feat: wire ClaimsPage route`
- `fix: handle empty claims list`

One logical change per commit.

---

## 7. Versioning Strategy

### 7.1 Semantic Versioning + Layer Suffixes

Use tags that combine semver + feature + layer:

- `v0.2.0-claims-L0`
- `v0.2.0-claims-L1`
- `v0.2.0-claims-L2`

Rules:

- **MAJOR**: Breaking changes to API/graph.
- **MINOR**: New features, new endpoints/layers.
- **PATCH**: Bugfixes and minor improvements.

Layer suffix is **informational**, but strongly recommended for tracking feature maturity.

### 7.2 When to Tag

- After each Layer completion (L0–L3) for a given feature.
- After large multi-task milestones.

---

## 8. Codex Workflow Checklist

Before Codex writes any code:

1. Confirm repo:
   - `colossus-legal`
2. Read:
   - `AGENTS.md`
   - `docs/WORKFLOW.md`
   - `docs/TASK_TRACKER.md`
   - `docs/ARCHITECTURE.md`
   - `docs/API_DESIGN.md`
   - `docs/DATA_MODEL.md`
3. Identify:
   - Task ID (e.g., `T2.1a`)
   - Layer (L0–L3)
   - Persona (BackendAgent, FrontendAgent, etc.)
4. Use the specific task file:
   - `docs/tasks/T2.1a_Claims_API_L0.md`
5. Ask for:
   - Branch name to use
   - Any environment specifics

Then proceed strictly within that scope.

---

## 9. Human Developer Workflow Checklist

When a human works:

1. Start of session:
   - `git status` (ensure clean)
   - `git switch main && git pull`
   - Open `TASK_TRACKER.md` and `PHASE_PLAN.md`
2. Choose a Task ID + Layer.
3. Create branch:
   - `git switch -c feature/<task-id>`
4. Implement in small steps:
   - Compile after small changes.
5. Test:
   - Backend + frontend as appropriate.
6. Update docs:
   - `TASK_TRACKER.md`
   - Possibly design docs.
7. Merge and/or tag.

---

## 10. Definitions of Done

A Task is **DONE** when:

- Code:
  - Compiles cleanly (backend/frontend as applicable).
  - Implements all acceptance criteria in its task file.
- Tests:
  - At least basic tests added for non-trivial logic (especially L1+).
- Docs:
  - TASK_TRACKER updated for that Task ID.
  - Any impacted design docs updated (API/DATA_MODEL/ARCHITECTURE).
- Git:
  - All changes in a feature branch merged to main.
  - Optional tag created if it represents a Layer completion.

---

---

# 11. Safety Rules (Critical)

These safety rules govern how Codex must behave during every task.  
Codex MUST follow these rules **before any file read, write, or modification.**

## 11.1 File Existence & Stop-on-Missing Rule

Before Codex reads or modifies any file, it MUST:

1. **Check if the file exists at the exact path specified.**
2. If the file does **NOT** exist:
   - Codex must **STOP immediately**.
   - Codex must **NOT** invent the file, guess its content, or proceed.
   - Codex must respond with:

     > “Requested file `<path>` not found. Stopping task per Safety Rule 11.1.  
     > Please confirm the correct path or create the file.”

3. Codex may only proceed after the human developer confirms the path or the file has been created.

This rule prevents Codex from fabricating content and ensures strict correctness.

---

## 11.2 Multi-File Modification Rule

Codex must NOT modify multiple files unless:

- The task file explicitly permits editing each of them, OR
- Codex asks for explicit human confirmation before every additional file outside the task list.

---

## 11.3 No-Invention Rule (Project Structure & Behavior)

Codex must NOT:

- Invent missing files (other than those the task explicitly instructs to create)
- Infer undocumented behavior
- Create new directories not listed in AGENTS.md
- Generate new architecture or workflows

If Codex is uncertain, it must STOP and ask.

---

## 11.4 Branch Safety Rule

Codex MUST:

1. Confirm the branch name before modifying any files.  
2. NEVER work on `main` unless explicitly instructed.  
3. NEVER create or merge branches automatically without permission.

---

## 11.5 Task Boundary Enforcement

Codex MUST:

- Work ONLY inside the scope of the current Task ID  
- Work ONLY within the assigned Layer (L0–L3)  
- Work ONLY within the persona’s allowed folder  
- STOP if asked to do something the persona is forbidden to do

---

## 11.6 Test Requirement Rule

From Layer 1 upward:

- Codex MUST include and update tests  
- Must not consider the task “DONE” unless all test criteria are met  
- Must run or simulate running the test commands before completion (`cargo test`, Vitest, etc.)

---

# End of Safety Rules

# End of WORKFLOW.md
