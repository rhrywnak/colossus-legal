# Colossus-Legal Development Process

**Version:** 2.0  
**Date:** 2025-12-20

This document defines the airtight development process for the Colossus-Legal project.

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

## Error Thresholds

| Error Count | Status | Action |
|-------------|--------|--------|
| 0 | ✅ Green | Continue |
| 1-10 | ⚠️ Yellow | Fix before next feature |
| 11-50 | 🛑 Red | STOP. Fix immediately |
| 51+ | 🚨 Critical | Revert, take smaller steps |

---

## Module Size Limits

| Lines | Status | Action |
|-------|--------|--------|
| 0-200 | ✅ Ideal | Perfect |
| 201-250 | ⚠️ Warning | Monitor, consider split |
| 251-300 | 🛑 Oversized | Split before adding more |
| 301+ | 🚨 Prohibited | Cannot commit |

---

## Roles

| Role | Actor | Responsibilities |
|------|-------|------------------|
| **Architect** | Claude Opus (Pro Chat) | Design, specs, extraction, review |
| **Implementer** | Claude Sonnet (Claude Code) | Write code, tests |
| **Owner** | Roman | Execute, verify, approve, commit |

---

## Process Overview

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        DEVELOPMENT WORKFLOW                                 │
└─────────────────────────────────────────────────────────────────────────────┘

┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐     ┌──────────┐
│  PLAN    │────►│  SPEC    │────►│  CODE    │────►│  REVIEW  │────►│  MERGE   │
│ (Opus)   │     │ (Opus)   │     │ (Sonnet) │     │ (Opus)   │     │ (Roman)  │
└──────────┘     └──────────┘     └──────────┘     └──────────┘     └──────────┘
     │                │                │                │                │
     ▼                ▼                ▼                ▼                ▼
  Select task    Create prompt    Pre-code resp   Verify quality    Git commit
  from tracker   for Sonnet       + approval      + exit criteria   and merge
                                  then code
```

---

## Phase 1: Task Selection (Opus + Roman)

### Input
- Current `TASK_TRACKER.md` status
- Project priorities

### Process
1. Opus reviews TASK_TRACKER.md
2. Opus recommends next task(s)
3. Roman approves or redirects
4. Opus identifies which tasks form a feature branch

### Output
- Selected Task ID(s)
- Feature branch name

### Template
```markdown
## Task Selection

**Recommended Task:** T5.2.1 — Create import DTOs
**Feature Branch:** feature/P5-F5.2-import-validation
**Dependencies:** F5.1 must be DONE ✓
**Estimated Complexity:** Medium (1-2 hours)

Ready to proceed?
```

---

## Phase 2: Pre-Coding Prompt (Opus)

### Input
- Selected Task ID
- DATA_MODEL_v2.md
- CLAIMS_IMPORT_WORKFLOW.md
- Existing codebase understanding

### Process
Opus creates a detailed prompt for Sonnet that includes:

1. **Task Header**
2. **Context** (what exists, what's needed)
3. **Specification** (exact requirements)
4. **Files** (to modify, to create)
5. **Tests Required** (with names and descriptions)
6. **Exit Criteria** (checklist)
7. **Warnings** (potential pitfalls)

### Output: Pre-Coding Prompt Template

```markdown
═══════════════════════════════════════════════════════════════════════════════
TASK: [Task ID] — [Task Name]
BRANCH: [feature branch name]
═══════════════════════════════════════════════════════════════════════════════

## Context
[Background information - what exists, what this task is part of]

## Specification
[Detailed requirements - be specific and precise]

### Required Structs/Functions
```
StructName {
    field1: Type,
    field2: Type,
}
```

### API Endpoints (if applicable)
```
METHOD /path
Request: { ... }
Response: { ... }
```

## Files to Modify
- `path/to/file.rs` — [specific changes needed]

## Files to Create
- `path/to/new_file.rs` — [purpose and contents]

## Tests Required
| Test Name | Description | Type |
|-----------|-------------|------|
| `test_name_1` | [What it verifies] | Unit |
| `test_name_2` | [What it verifies] | Integration |

## Exit Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] All tests pass
- [ ] `cargo build` succeeds
- [ ] `cargo clippy` has no warnings

## Warnings
- [Potential issue to watch for]
- [Common mistake to avoid]

## References
- `docs/DATA_MODEL_v2.md` — Schema definitions
- `docs/CLAIMS_IMPORT_WORKFLOW.md` — Workflow spec

═══════════════════════════════════════════════════════════════════════════════
INSTRUCTIONS: 
1. Read CLAUDE_CODE_INSTRUCTIONS.md
2. Provide Pre-Coding Analysis (do NOT write code yet)
3. Wait for approval before implementing
═══════════════════════════════════════════════════════════════════════════════
```

---

## Phase 3: Pre-Coding Analysis (Sonnet)

### Input
- Pre-Coding Prompt from Opus

### Process
Sonnet reads the prompt and existing code, then provides analysis WITHOUT writing code.

### Output: Pre-Coding Response Template

```markdown
## Pre-Coding Analysis for [Task ID]

### Task Understanding
[Restate the task in own words to confirm understanding]

### Files to Modify
| File | Changes |
|------|---------|
| `path/to/file.rs` | [Specific changes] |

### Files to Create
| File | Purpose | Estimated Lines |
|------|---------|-----------------|
| `path/to/new.rs` | [Purpose] | ~XXX |

### Dependencies Verified
- [x] `module_name` exists at `src/module_name.rs`
- [x] `crate_name` is in Cargo.toml
- [ ] ⚠️ ISSUE: [Missing dependency]

### Implementation Plan
1. [Step 1 - be specific]
2. [Step 2 - be specific]
3. [Step 3 - be specific]

### Tests to Write
| Test Name | Tests | Approach |
|-----------|-------|----------|
| `test_name_1` | [What] | [How] |

### Questions/Concerns
- [Any ambiguities or concerns]

### Ready to Proceed
[YES / NO - reason if no]
```

---

## Phase 4: Analysis Review (Opus + Roman)

### Input
- Sonnet's Pre-Coding Analysis

### Process
1. Opus reviews the analysis
2. Checks for misunderstandings
3. Checks for missed dependencies
4. Checks for incomplete test coverage
5. Roman confirms or raises concerns

### Output
- **APPROVED**: Proceed to coding
- **REVISION NEEDED**: Provide corrections
- **BLOCKED**: Identify blocker and resolution

### Template
```markdown
## Pre-Coding Review for [Task ID]

### Analysis Quality
- [ ] Task correctly understood
- [ ] All files identified
- [ ] Dependencies verified
- [ ] Implementation plan is sound
- [ ] Tests are comprehensive

### Issues Found
[List any issues, or "None"]

### Corrections Required
[List any corrections, or "None"]

### Decision
**[APPROVED / REVISION NEEDED / BLOCKED]**

[If approved]: Proceed with implementation.
[If revision]: Address the following before proceeding: ...
[If blocked]: Blocked by: ...
```

---

## Phase 5: Implementation (Sonnet)

### Input
- Approved Pre-Coding Analysis
- Pre-Coding Prompt

### Process
1. Create feature branch (if not exists)
2. Write code following the plan
3. Write all required tests
4. Run `cargo build`
5. Run `cargo test`
6. Run `cargo clippy`
7. Fix any issues
8. Provide Completion Report

### Output: Completion Report Template

```markdown
## Completion Report for [Task ID]

### Implementation Summary
[Brief description of what was implemented]

### Files Modified
| File | Changes | Lines |
|------|---------|-------|
| `path/to/file.rs` | [What changed] | +XX/-YY |

### Files Created
| File | Purpose | Lines |
|------|---------|-------|
| `path/to/new.rs` | [Purpose] | XXX |

### Tests Implemented
| Test | Status | Notes |
|------|--------|-------|
| `test_name_1` | ✅ PASS | |
| `test_name_2` | ✅ PASS | |

### Build Results
```
cargo build: ✅ SUCCESS
cargo test:  ✅ SUCCESS (X passed, 0 failed)
cargo clippy: ✅ SUCCESS (0 warnings)
```

### Exit Criteria Checklist
- [x] Criterion 1
- [x] Criterion 2
- [x] All tests pass
- [x] No compiler warnings

### Manual Verification Steps for Roman
1. [ ] [Step 1]
2. [ ] [Step 2]

### Known Limitations
[Any limitations or future improvements]

### Ready for Review
**YES**
```

---

## Phase 6: Code Review (Opus)

### Input
- Sonnet's Completion Report
- Code diff (Roman can share)

### Process
1. Review implementation against spec
2. Check code quality
3. Verify tests are meaningful
4. Check for edge cases
5. Verify exit criteria met

### Output: Review Decision

```markdown
## Code Review for [Task ID]

### Review Checklist
- [ ] Implementation matches spec
- [ ] Code follows project standards
- [ ] Error handling is complete
- [ ] Tests cover happy path and errors
- [ ] No security issues
- [ ] Documentation adequate

### Issues Found
[List any issues, or "None"]

### Required Changes
[List required changes, or "None - approved"]

### Decision
**[APPROVED / CHANGES REQUIRED]**
```

---

## Phase 7: Merge (Roman)

### Input
- Approved Code Review
- Completed Manual Verification

### Process
1. Complete manual verification steps
2. Commit changes with proper message
3. Push to remote
4. Merge to develop (or create PR)
5. Update TASK_TRACKER.md

### Commit Message Format
```
[Task ID] Brief description

- Implemented X
- Added tests for Y
- Updated Z

Exit criteria verified.
```

### Example
```bash
git add .
git commit -m "[T5.2.1] Create import DTOs

- Added ImportRequest, ValidationResult, ImportReport structs
- Added ValidationError enum
- Added 3 unit tests

Exit criteria verified."

git push origin feature/P5-F5.2-import-validation
```

### After Merge
Update `TASK_TRACKER.md`:
- Change task status to `DONE`
- Add completion date
- Note any follow-up items

---

## Quick Reference: Complete Task Flow

```
1. OPUS:   "Next task is T5.2.1. Here's the pre-coding prompt..."
2. ROMAN:  [Copies prompt to Claude Code]
3. SONNET: [Provides pre-coding analysis]
4. ROMAN:  [Copies analysis back to Opus chat]
5. OPUS:   "Analysis approved. Proceed."
6. ROMAN:  [Tells Sonnet to proceed]
7. SONNET: [Implements and provides completion report]
8. ROMAN:  [Copies completion report to Opus chat]
9. OPUS:   "Code approved. Ready for merge."
10. ROMAN: [Runs manual verification]
11. ROMAN: [Git commit and merge]
12. ROMAN: [Updates TASK_TRACKER.md]
```

---

## Emergency Procedures

### If Sonnet Gets Stuck
1. Stop Sonnet
2. Describe the issue to Opus
3. Opus provides guidance
4. Resume with Sonnet

### If Build Fails After Sonnet Says It Passes
1. Copy exact error to Opus
2. Opus diagnoses
3. Opus provides fix or revised prompt
4. Sonnet implements fix

### If Task Scope Creeps
1. Stop work
2. Return to Opus
3. Opus splits task or revises scope
4. Continue with clearer boundaries

### If Errors Exceed Threshold
| Errors | Action |
|--------|--------|
| 1-10 | Sonnet fixes before proceeding |
| 11-50 | Stop, Roman copies errors to Opus |
| 51+ | Revert changes, Opus redesigns approach |

### If Module Exceeds Size Limit
1. Stop adding code
2. Sonnet proposes split strategy
3. Opus approves split
4. Sonnet refactors before continuing

---

## Test Requirements

### Test Count Guidelines
| Task Type | Minimum Tests |
|-----------|---------------|
| Simple (L0 types) | 5-8 tests |
| Medium (L1 CRUD) | 8-12 tests |
| Complex (L2 logic) | 15-20 tests |
| Integration (L3) | 10-15 tests |

### Required Test Categories
- **Happy path**: Normal successful operations
- **Error cases**: Invalid input, missing data
- **Edge cases**: Boundary conditions, empty collections
- **Async**: Use `#[tokio::test]` for async functions

### Test Naming Convention
```
test_<function>_<scenario>_<expected_result>
```

Examples:
- `test_validate_claim_valid_input_returns_ok`
- `test_validate_claim_empty_quote_returns_error`
- `test_parse_json_malformed_returns_parse_error`

---

## Metrics

Track for each task:
- Pre-coding prompt quality (revisions needed?)
- Implementation accuracy (changes required in review?)
- Test coverage
- Time from start to merge

---

## Document Locations

```
~/Projects/colossus-legal/
├── docs/
│   ├── TASK_TRACKER.md           # Task status
│   ├── DATA_MODEL_v2.md          # Schema
│   ├── CLAIMS_IMPORT_WORKFLOW.md # Import spec
│   ├── DEVELOPMENT_PROCESS.md    # This document
│   └── CLAUDE_CODE_INSTRUCTIONS.md # Sonnet instructions
```
