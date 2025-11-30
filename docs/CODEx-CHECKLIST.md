# CODEx-CHECKLIST.md
This checklist is the quick, session-by-session operational guide for using Codex safely within the `colossus-legal` repository.
It complements `CODEx-SESSION-RULES.md` by providing a **simple, repeatable workflow**.

---

# ✅ 1. Pre-Session Human Setup

Before invoking Codex:

1. **Verify correct branch**
   ```bash
   git branch --show-current
   ```
   Must be a task-specific feature branch (e.g. `feature/T3.1b-document-api-l1`).

2. **Verify clean working tree**
   ```bash
   git status
   ```
   Should show:
   - nothing to commit **or**
   - only the intended task’s uncommitted changes.

3. **Ensure task is selected**
   - Identify Task ID (e.g. T3.1b).
   - Identify Persona (BackendAgent / FrontendAgent / DocsAgent).
   - Identify Layer (L0–L3).
   - Open the task spec in `docs/tasks/`.

4. **Open CODEx-SESSION-RULES.md**
   - Quickly skim the rules before starting.

---

# ✅ 2. Codex Pre-Flight (Planning-Only Phase)

Codex **must** begin the session by:

### ✔ A. Confirming metadata
- Task ID  
- Persona  
- Layer  
- Branch  

### ✔ B. Listing the files it will read
- AGENTS.md  
- WORKFLOW.md  
- TASK_TRACKER.md  
- The relevant task doc (`docs/tasks/T*.md`)  
- Only the code modules for this task  

### ✔ C. Producing a clean PLANNING response
This must include:

1. Summary of the task requirements  
2. Summary of the current code state (after reading files, not guessing)  
3. A numbered implementation plan  
4. A list of **allowed** files for modification  
5. STOP (no edits yet)

Human must approve before Codex writes anything.

---

# ✅ 3. Human Approval Before Editing

Human must verify the plan:

- Does the scope match the task?
- Are only the correct files listed for modification?
- Is there **no scope creep**?
- Did Codex avoid making assumptions about missing files?

Then human explicitly says:

```
Proceed with implementation under these constraints.
```

Codex may not proceed without that phrase.

---

# ✅ 4. Codex Editing Phase (Strict Mode)

During implementation Codex must:

### ✔ Edit ONLY approved files  
If Codex touches an unapproved file, STOP the session immediately.

### ✔ Run `ls` or `cat` before claiming a file exists  
No hallucinated files allowed.

### ✔ After making changes, run:
```
git diff --name-only
```
and confirm only allowed files were modified.

### ✔ After changes, run:
```
cargo check
```
(or `npm run build` in frontend tasks)

### ✔ STOP after finishing edits
Codex must not continue into the next task or modify docs unless explicitly instructed.

---

# ✅ 5. Human Post-Flight Verification

After Codex says “done”, human must:

### A. Verify changed files:
```bash
git diff --name-only
```
Matches the allowed set?  
If not → revert and restart.

### B. Inspect actual file contents:
- Confirm handler signatures  
- Confirm DTOs match task spec  
- Confirm routes exist  
- Confirm no unrelated changes

### C. Compile/test:
```bash
cargo check
cargo test --tests
```

### D. Accept the task only if every acceptance criterion is satisfied.

---

# ✅ 6. Git Commit Checklist

Once verified:

1. Stage only intended files:
   ```bash
   git add <explicit files>
   git diff --cached --name-only
   ```
2. Commit with Task ID:
   ```bash
   git commit -m "feat: <description> (<TaskID>)"
   ```
3. Push:
   ```bash
   git push -u origin <branch>
   ```

---

# ✅ 7. Forensic Mode Trigger

Switch to forensic mode if:

- Codex claims it created a file that doesn’t exist  
- Codex output doesn’t match filesystem reality  
- `git diff` shows unexpected paths  
- Compilation fails in places Codex did not touch  

In forensic mode Codex may only:

- Read files  
- Run `ls`, `git status`, `git diff --name-only`, `cat`  
- Produce diagnostic reports  

**No code edits allowed** until forensic analysis is complete.

---

# 🎯 Summary (One-Line Safety Rule)

**Codex must plan first, edit only approved files, verify changes with git diff, and STOP if reality diverges from what it claims.**

