# CODEx-SESSION-RULES.md

**Scope:** These rules govern how Codex (v0.63) is allowed to operate on the `colossus-legal` repository.  
**Goal:** Keep the repo trustworthy, changes small, and prevent Codex from silently drifting away from reality.

Codex is **not** a free-running developer. Codex is:

- A **planner** (reads, summarizes, proposes).
- A **patch generator** (suggests code/diffs).
- Optionally a **limited editor** under very strict constraints.

The **filesystem and git** are always the source of truth, not Codex’s previous messages.

---

## 1. Allowed Roles for Codex

Codex may be used in three modes:

1. **Planner Mode (preferred)**  
   - Reads files.  
   - Summarizes current behavior.  
   - Proposes a plan and suggested changes (code snippets, diffs).  
   - *Human applies the changes manually.*

2. **Patch Mode (optional)**  
   - Proposes small, surgical patches (e.g., “replace this function with…”).  
   - Human reviews and applies via editor or `git apply`.  
   - Codex does **not** run tools that directly edit disk.

3. **Editor Mode (rare / risky)**  
   - Codex is allowed to use editing tools (e.g. `apply_patch`) **only for files explicitly listed**.  
   - Strict pre-flight and post-flight checks are **mandatory** (see below).  
   - Any violation ends the session and requires a manual reset.

**Default rule:** For backend (`backend/`), Codex should usually be in **Planner Mode** only.  
Editor Mode is reserved for very small, well-bounded tasks.

---

## 2. Hard Behavioral Rules

Codex must:

1. **Obey AGENTS + WORKFLOW + SAFETY**
   - Always re-read:
     - `AGENTS.md`
     - `docs/WORKFLOW.md`
     - `docs/TASK_TRACKER.md`
     - Relevant `docs/tasks/T*.md` for the Task ID
   - Respect persona, layer, and file scope.

2. **Never invent state**
   - Codex must not say “file X now exists” unless it has just read it from the filesystem.
   - Any claim about the repo must be backed by a recent `ls` / `cat` / `git` view.

3. **Stop on Missing Files (Safety Rule 11.1)**
   - If a referenced file path does not exist:
     - STOP the task.
     - Report the missing path.
     - Do not “fake” or assume it exists.

4. **One Task → One Scope**
   - One Task ID per session (`T3.1a`, `T3.1b`, etc.).
   - Only the directories/files for that task and persona may be touched.
   - Codex must refuse to modify unrelated areas.

5. **No Large Refactors**
   - Codex must not:
     - Rename modules broadly.
     - Change project structure.
     - Update many files “just to clean up”.

---

## 3. Pre-Flight Checklist (Before Codex Touches Anything)

Before starting a Codex session on this repo:

1. **Human does:**

   - Ensure correct branch:
     ```bash
     git branch --show-current
     # e.g. feature/T3.1a-document-api-l0 or feature/T3.1b-document-api-l1
     ```
   - For **Editor Mode**, branch **must be dedicated** to a single task:
     - No unrelated changes.
     - Working tree ideally clean.

   - Run:
     ```bash
     git status
     ```
     Confirm:
     - Either fully clean, or
     - Only known, intentional changes for this task.

2. **Codex must start with a PLANNING-ONLY response:**

   - Confirm:
     - Task ID
     - Persona
     - Layer
     - Branch name
   - List the files it will read (docs + code).
   - Summarize:
     - Requirements from the task doc.
     - Current behavior (based on reading the files).
   - Propose a **numbered implementation plan**.
   - **Stop and wait** for human approval.

3. **Human reviews the plan:**
   - Check:
     - Only expected files will be touched.
     - No “helpful” scope creep.
   - If acceptable, explicitly say:
     - “Proceed with implementation under these constraints…”

---

## 4. Post-Flight Checklist (After Codex Claims “Done”)

Whenever Codex says “task complete” or “changes applied”, the **human must verify** using this checklist:

1. **Check which files actually changed:**
   ```bash
   git status
   git diff --name-only
   ```

   - The list must match the **allowed file list** for this task.
   - If any extra files appear:
     - STOP.
     - Consider `git restore <path>` for those files.
     - Optionally terminate the session and redo on a clean branch.

2. **Inspect key files manually:**
   - Open the files Codex claimed to modify.
   - Verify:
     - New struct/handler/route actually exists.
     - Names and types match the task spec.
     - No obvious collateral damage.

3. **Compile & test:**
   - For backend:
     ```bash
     cd backend
     cargo check
     # and for testable layers:
     cargo test --tests
     ```
   - For frontend:
     ```bash
     cd frontend
     npm run build
     npm test   # once test suites exist
     ```

4. **Regenerate the acceptance checklist from the task doc:**
   - For each acceptance criteria bullet:
     - Identify the file(s) that satisfy it.
     - Confirm they exist and behave correctly.

5. **Only then commit:**
   - Stage only the intended files:
     ```bash
     git add <explicit-list-of-files>
     git diff --cached --name-only
     ```
   - Commit with Task ID in message:
     ```bash
     git commit -m "feat: <short description> (T3.1a)"
     ```

---

## 5. “Forensic Mode” Rules (When Something Smells Wrong)

If at any point:

- Codex claims a file exists but `ls` says it doesn’t.
- `git diff --name-only` shows unexpected files.
- Behavior does not match Codex’s description.

Then:

1. **Stop all editing.**
2. Put Codex into **read-only forensic mode**:
   - It may run:
     - `git status`
     - `git diff --name-only`
     - `ls`, `cat`, `sed -n`, etc.
   - It may **not** write or modify files.
3. Have Codex produce a short report:
   - What the task spec requires.
   - What actually exists on disk.
   - Which acceptance criteria are unmet.
4. Human decides:
   - **Either** salvage via manual fixes,
   - **Or** reset from `main` and re-implement on a fresh feature branch.

No new Codex edits are allowed until the discrepancy is fully understood.

---

## 6. Where Codex Is Not Allowed (for Now)

Given prior failures, Codex should **not directly edit**:

- `backend/src/` (Rust backend) — except in very small, clearly bounded Editor Mode sessions, and only after human approval.
- Graph schema files (`docs/DATA_MODEL.md`) and core architecture files (`docs/ARCHITECTURE.md`) — Codex may suggest text, but human should edit.

Codex **may** edit (with less risk):

- Task docs: `docs/tasks/T*.md`
- `docs/TASK_TRACKER.md` (with care)
- Other non-critical docs, as long as the human reviews diffs.

---

## 7. Summary

- Codex is a **planner and assistant**, not an autonomous maintainer.
- Every Codex session must:
  - Start with **planning only**.
  - End with a **human-run post-flight checklist**.
- The repo’s health is always validated by:
  - `git status` / `git diff`
  - `cargo check` / `cargo test`
  - Human inspection of key files.

If these rules are followed consistently, Codex remains a powerful helper **without being able to wreck the repo**.
