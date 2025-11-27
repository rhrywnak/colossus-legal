# Colossus-AI Rig Development Guide

> **The Golden Rule**: `cargo check` after EVERY meaningful change. Never accumulate more than 10 errors.

## 📚 **Documentation Ecosystem**
**Local repo docs live under `docs/` (Architecture, SRS, Phase tracker, session checkpoints). Historical Colossus materials remain under `/home/roman/colossus-docs/shared/` for reference.** 

**📋 [Complete Documentation Index](DOCUMENTATION-INDEX.md)** - See full document map and usage guide

**Quick Reference:**
- **This Document**: Primary development guide and workflow
- **[BUILD-VERIFY.md](BUILD-VERIFY.md)**: Runtime deployment and troubleshooting  
- **[RUST-PATTERNS.md](RUST-PATTERNS.md)**: Rust programming patterns reference

## 🚀 Part 1: Quick Start Checklist

### Before You Start Coding
```bash
# 1. Review open code-review findings and remediation plan
cat docs/CODE-REMEDIATION-TASK-LIST.md
cat /home/roman/Documents/colossus-ai/CODEX-COLOSSUS-AI-CODE\ REVIEW\ .md
cat /home/roman/Documents/colossus-ai/CLAUDE-COLOSSUS-AI-CODE-REVIEW-11-15-25.md
# Confirm you understand every outstanding task before touching code

# 2. Check current backend health
cd backend
cargo check                                      # Must show 0 errors to proceed
find src -name "*.rs" -exec wc -l {} + | sort -nr | head -20  # Check module sizes
# CRITICAL: point all destructive tests at a disposable database.
# export TEST_DATABASE_URL=postgres://localhost/colossus_test before running `cargo test`.
# NEVER run `cargo test` or any SQL that truncates tables against the shared Stage‑0 DB without explicit approval.

# 3. MANDATORY: Verify no modules exceed size limits
find src -name "*.rs" -exec sh -c 'lines=$(wc -l < "$1"); if [ $lines -gt 300 ]; then echo "❌ OVERSIZED: $1 ($lines lines > 300 limit)"; fi' _ {} \;

# 4. Create feature branch
git checkout -b feature/your-feature-name       # Isolate your changes

# 5. Review project status and update checklist progress
cat docs/Phase_Task_Tracker.md                 # Understand current state
todoist add "Update CODE-REMEDIATION-TASK-LIST.md"   # Ensure remediation status stays current
```

### Required Environment Flags

- `OBSERVE_ENABLED` — set to `true`/`1` when `colossus-llm-observe` is reachable. Leave `false` locally to force Tier3 heuristics.
- `OBSERVE_BASE_URL` — base URL for the observe service (e.g., `http://localhost:4205`).
- `OBSERVE_API_KEY` — optional API key forwarded to `/health` and `/api/v1/recommend`.
- `MODEL_DEFAULTS_PATH` — override path to `model_defaults.json` if you maintain personalized mappings; defaults to `backend/config/model_defaults.json`.
- `ALLOW_PARSER_FALLBACK` — set to `true` only in dev/test to permit stub metadata when the parser fails; production should keep this `false` so ingestion errors surface immediately.
- `MAX_WS_PAYLOAD_BYTES` — maximum WebSocket frame size; defaults to 64 KB. Oversized frames trigger a close (code 1009) instead of being processed.
- `INGEST_EMBED_CONCURRENCY` — controls how many embeddings ingestion computes simultaneously (default 4). Lower this if your embedding backend or Qdrant needs more conservative rate limiting.
- `PAPER_CACHE_CAPACITY` / `PAPER_CACHE_TTL_SECS` — size and TTL for the in-memory paper metadata cache (defaults 256 / 300 s). Tune based on dataset size and freshness requirements.
- `API_AUTH_TOKENS` / `VITE_API_KEY` — API auth is enforced end-to-end; when unset we fall back to `dev-token` for local usage. Set explicit secrets for shared/staging environments.

### While Coding - The Continuous Compilation Loop
```bash
# After EVERY file save or meaningful change:
cargo check --lib --message-format short

# If errors > 10: STOP coding, fix errors first
# If errors > 50: Major problem - review your approach
```

### Before Committing
```bash
# Quality gate checklist
cargo check          # Must pass with 0 errors
cargo fmt            # Format code
cargo clippy         # Check for improvements
cargo test           # Run tests
./scripts/quality_check.sh  # Final validation

# Only if ALL pass:
git add .
git commit -m "type: description"
git push origin feature/your-feature-name
```

---

## 📋 Part 2: Development Workflow

### The Compilation Discipline
**NEVER** let errors accumulate. Compile after:
- ✅ Every file save (ideal)
- ✅ Every 50 lines of code (maximum)
- ✅ Every 30 minutes (absolute maximum)
- ✅ Any public API change (immediate)
- ✅ Any enum/struct modification (immediate)
- ✅ Any module refactoring (immediate)

### Error Thresholds & Actions

| Error Count | Status | Required Action |
|------------|--------|-----------------|
| 0 | ✅ Green | Continue developing |
| 1-10 | ⚠️ Yellow | Fix before next feature |
| 11-50 | 🛑 Red | STOP. Fix immediately |
| 51-100 | 🚨 Critical | Revert changes, smaller steps |
| 100+ | 💥 Emergency | Seek help, major architecture issue |

### 📏 Module Size Limits & Enforcement

| Lines | Status | Required Action |
|-------|--------|-----------------|
| 0-200 | ✅ **IDEAL** | Perfect size, easy to understand |
| 201-300 | ⚠️ **ACCEPTABLE** | Monitor for growth, consider splitting |
| 301-500 | 🛑 **OVERSIZED** | MANDATORY refactoring required |
| 501+ | 🚨 **PROHIBITED** | Cannot commit - split immediately |

**Module Size Rules:**
- ❌ **NO MODULE > 300 lines** (including necessary comments)
- ❌ **NO FUNCTION > 50 lines** (excluding error handling)
- ✅ **PREFER 150-200 line modules** for optimal maintainability
- ✅ **SINGLE RESPONSIBILITY** - one clear purpose per module

**Comment Requirements for Learning:**
- ✅ **REQUIRED**: Brief inline comments for Rust patterns with references to /home/roman/colossus-project-documents/RUST-PATTERNS.md
- ✅ **EXAMPLE**: `Arc<T>  // Thread-safe shared ownership - see /home/roman/colossus-project-documents/RUST-PATTERNS.md#arc-pattern`
- ❌ **AVOID**: Long tutorial essays that explain theoretical concepts
- ❌ **AVOID**: Comments that just describe WHAT the code does (code should be self-documenting)
- ✅ **FOCUS**: WHY decisions were made and WHICH Rust patterns are used

**Enforcement Commands:**
```bash
# Check all module sizes before any commit
find src -name "*.rs" -exec sh -c 'lines=$(wc -l < "$1"); if [ $lines -gt 300 ]; then echo "❌ OVERSIZED: $1 ($lines lines)"; exit 1; fi' _ {} \;

# Count non-comment lines (more accurate)
find src -name "*.rs" -exec sh -c 'lines=$(grep -v "^[[:space:]]*//\|^[[:space:]]*\*\|^[[:space:]]*$" "$1" | wc -l); if [ $lines -gt 300 ]; then echo "❌ OVERSIZED: $1 ($lines non-comment lines)"; fi' _ {} \;

# Daily module size report
echo "📊 MODULE SIZE REPORT:" && find src -name "*.rs" -exec wc -l {} + | sort -nr | head -10
```

**Refactoring Strategies for Oversized Modules:**
1. **Extract Related Functions** → Create new focused module
2. **Split by Responsibility** → Separate concerns into different files  
3. **Move Constants/Types** → Create dedicated `types.rs` or `constants.rs`
4. **Extract Tests** → Move to separate test modules
5. **Create Sub-modules** → Use `mod.rs` pattern for complex features

### Mandatory Code Review Compliance
- 🔁 **Start and end every session** by re-reading `docs/CODE-REMEDIATION-TASK-LIST.md` and both active code-review reports.
- 🧾 **Open tasks are not optional**: do not create new features until all critical/high remediation items in the checklist are unchecked.
- ✅ **Every commit must update the checklist**—if you touch an area tied to a numbered task, edit the Markdown file so status never drifts.
- 🧪 **Security and safety tests** (CORS, request limits, prompt-path validation, etc.) must be added alongside fixes; lack of tests counts as incomplete.
- 📣 **Deviations require documentation**: if a checklist item cannot be finished, record why in the task entry before pausing work.

### Type-Driven Development Process
```rust
// 1. Define types first
pub struct NewFeature {
    // fields
}

// 2. Compile - let errors guide you
cargo check

// 3. Implement missing traits/methods the compiler requests
// 4. Compile again
// 5. Repeat until green
```

### Feature Branch Workflow
```bash
# 1. Start from clean main
git checkout main
git pull
cargo check  # Verify clean start

# 2. Create feature branch
git checkout -b feature/specific-feature

# 3. Make incremental changes
# Small change → cargo check → fix → commit

# 4. Regular commits (every 30-60 min)
git add -p  # Review changes
git commit -m "feat: implement X part of Y"

# 5. Before merging
cargo check
cargo test
cargo fmt
```

---

## 📏 Part 3: Code Standards

#### Size Monitoring
```bash
# Check all module sizes
find src -name "*.rs" -exec wc -l {} + | sort -nr | head -20

# Monitor specific module
wc -l src/path/to/module.rs

# Automated check (add to CI/CD)
./scripts/check_module_sizes.sh
```

#### When to Refactor

### Module Organization Pattern
```
src/
├── domain_name/
│   ├── mod.rs          # Public API & re-exports (~100-200 lines)
│   ├── types.rs        # Core types & structures (~300-500 lines)
│   ├── core.rs         # Main logic (~400-600 lines)
│   ├── errors.rs       # Error types (~200-400 lines)
│   ├── utils.rs        # Helpers (~200-400 lines)
│   └── tests.rs        # Unit tests (no limit)
```

### Rust Best Practices

#### Error Handling
```rust
// ✅ GOOD: Rich error types with context
#[derive(Debug, thiserror::Error)]
pub enum ServiceError {
    #[error("Database error: {0}")]
    Database(#[from] DatabaseError),
    
    #[error("Validation failed: {reason}")]
    Validation { reason: String },
}

// ❌ BAD: Generic errors
Err("Something went wrong".to_string())
```

#### Type Safety
```rust
// ✅ GOOD: Type-safe IDs
pub struct UserId(Uuid);
pub struct SessionId(Uuid);

// ❌ BAD: Primitive obsession
fn get_user(id: String) -> User  // Which ID?
```

#### Documentation
```rust
/// Process documents for agent analysis
/// 
/// # Purpose
/// Demonstrates async processing with error handling
/// 
/// # Errors
/// Returns `ProcessError` if document invalid or processing fails
/// 
/// # Example
/// ```
/// let result = process_document(doc).await?;
/// ```
pub async fn process_document(doc: Document) -> Result<ProcessedDoc, ProcessError> {
    // Implementation
}
```

---

## 🎯 Part 4: Architecture Principles

### Educational First
- All code serves as learning material
- Comments explain "why" not just "what"
- Include learning objectives in module headers
- Reference patterns and concepts

### Incremental Refactoring Rules
1. **Never refactor multiple systems simultaneously**
2. **One module at a time**
3. **Compile after each extraction**
4. **Test after each move**
5. **Commit working state frequently**

### Modular Architecture
- Single responsibility per module
- Clear boundaries with minimal coupling
- Rich re-exports for clean APIs
- Dependency injection over hard coupling

---

## 🚨 Part 5: Lessons from the 491-Error Incident

### What Went Wrong
We accumulated 491 compilation errors by:
- Making multiple architectural changes without compiling
- Refactoring several systems simultaneously  
- Using copy-paste patterns without validation
- Ignoring the compiler for days/weeks

### Prevention Measures

#### 1. Continuous Integration Mindset
```yaml
# .github/workflows/ci.yml
on: [push, pull_request]
jobs:
  check:
    steps:
      - uses: actions/checkout@v2
      - run: cargo check
      - run: cargo fmt -- --check
      - run: cargo clippy -- -D warnings
```

#### 2. Pre-commit Hooks
```bash
# .git/hooks/pre-commit
#!/bin/bash
cargo check || exit 1
cargo fmt --check || exit 1
```

#### 3. Architecture Change Protocol
For any change affecting >3 files:
1. Document the change plan
2. Create a feature branch
3. Make changes incrementally
4. Compile after EACH file
5. Fix errors before proceeding
6. Never leave branch with errors

#### 4. The "Stop and Fix" Rule
- See an error? Fix it NOW
- Changed an enum? Update ALL usages NOW
- Modified a struct? Find ALL constructions NOW
- Renamed a method? Update ALL calls NOW

---

## 🔍 Part 6: Pre-Development Verification Protocol

**MANDATORY VERIFICATION BEFORE WRITING ANY CODE**: To prevent compilation errors and ensure clean integration, you MUST follow this systematic verification process:

### 6.1 Read Existing Implementation FIRST
- **Always use Read tool** to examine the target file before adding any code
- Read the entire file to understand existing structure, functions, and patterns
- Never assume what exists - always verify with actual file content

### 6.2 Verify Function/Method Existence
- **Use Grep to check** if methods already exist: `Grep pattern:"pub.*fn method_name"`
- **Check async functions**: `Grep pattern:"async fn method_name"`
- **Prevent duplicate definitions** that cause compilation errors
- **Verify trait implementations**: `Grep pattern:"impl.*StructName"`

### 6.3 Validate Type Definitions Before Use
- **Verify structs exist**: `Grep pattern:"pub struct TypeName"`
- **Check enums exist**: `Grep pattern:"pub enum TypeName"`
- **Validate field names**: Read struct definitions to check exact field names
- **Verify imports**: `Grep pattern:"use.*TypeName"` to see if types are imported

### 6.4 Module Structure Verification
- **Check directory structure**: Use LS to see existing module organization
- **Read mod.rs files**: Understand what's exported with `Read <mod.rs>`
- **Verify no naming conflicts**: Check existing module names before adding new ones

### 6.5 Incremental Development Process
- **Start with skeleton functions** using `unimplemented!()` that compile
- **Run `cargo check` after every 10-20 lines** of code added
- **Fix compilation errors IMMEDIATELY** - never accumulate multiple errors
- **Test each component** as it's added rather than building everything then testing

### 6.6 Integration Point Verification
- **Check parameter types match** expected function signatures
- **Verify return types** are compatible with calling code
- **Test module boundaries** work with `cargo check --lib`

### Protocol Benefits
**RATIONALE**: We consistently spend significant time fighting compilation errors because we write code without verifying the existing codebase state. This protocol should reduce compilation errors by 95%+ and make development much more efficient.

**SUCCESS METRICS**:
- Initial compilation should have < 5 errors per major implementation
- Zero duplicate type/function definitions
- Clean integration with existing modules
- Faster development cycles due to fewer error-fix loops

**ENFORCEMENT**: Any session where this protocol isn't followed and results in excessive compilation errors should be treated as a process failure requiring immediate correction.

This protocol prevents technical debt accumulation and ensures robust, production-ready implementations.

---

## 📊 Part 7: Quality Metrics

### Health Indicators
- ✅ **Healthy**: 0 errors, all modules <800 lines
- ⚠️ **Attention**: <10 errors, some modules 800-900 lines  
- 🛑 **Unhealthy**: >10 errors, modules >900 lines
- 💀 **Critical**: >50 errors, modules >1000 lines

### Session Success Criteria
A successful development session has:
- [ ] Zero compilation errors at end
- [ ] All modules under 1000 lines
- [ ] Tests passing
- [ ] Code formatted
- [ ] Changes committed to feature branch

---

## 🛠️ Part 8: Tooling & Automation

### Essential Aliases (.bashrc or .zshrc)
```bash
alias cc='cargo check --lib --message-format short'
alias ccf='cargo check --lib 2>&1 | grep "error\[" | wc -l'
alias ct='cargo test'
alias cf='cargo fmt'
alias modsize='find src -name "*.rs" -exec wc -l {} + | sort -nr | head -20'
```

### VS Code Settings
```json
{
  "rust-analyzer.checkOnSave.command": "check",
  "rust-analyzer.checkOnSave.allTargets": false,
  "editor.formatOnSave": true,
  "editor.rulers": [80, 100],
}
```

### Continuous Monitoring Script
```bash
#!/bin/bash
# save as scripts/watch_health.sh
while true; do
    clear
    echo "=== Colossus Health Monitor ==="
    echo -n "Compilation Errors: "
    cargo check 2>&1 | grep -c "error:"
    echo -e "\nLarge Modules:"
    find src -name "*.rs" -exec wc -l {} + | sort -nr | head -5
    sleep 30
done
```

---

## 📝 Part 9: Commit Message Standards

```
type: brief description (max 50 chars)

Longer explanation if needed (wrap at 72 chars)

- Bullet points for multiple changes
- Reference issue numbers: #123
- Breaking changes marked with BREAKING CHANGE:
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring
- `docs`: Documentation
- `test`: Test changes
- `perf`: Performance improvements
- `chore`: Maintenance tasks

---

## 🎓 Part 10: Learning Resources

### When Stuck
1. Check compiler error messages carefully
2. Run `cargo check --verbose` for more detail
3. Use `cargo explain E0XXX` for error codes
4. Consult The Rust Book for patterns
5. Ask for help if blocked >30 minutes

### Key Rust Patterns to Master
- Result/Option handling
- Trait bounds and generics
- Lifetime annotations
- Async/await patterns
- Module organization
- Error propagation with `?`

---

## 📋 Part 11: Task Completion Workflow

### **Post-Task Documentation Updates (MANDATORY)**
After completing any project task, you MUST update these documents:

#### 1. Project Status Update
```bash
# Update main project tracking
vim PROJECT-STATUS-AND-NEXT-STEPS.md
# - Mark task as completed with date
# - Update progress percentages
# - Add key features and metrics
```

#### 2. Technical Documentation
```bash
# Update detailed tracking (if exists)
vim RESEARCH-AGENT-TASK-TRACKING.md  # or current tracking doc
# - Add comprehensive implementation details
# - Include files modified and line counts
# - Document test results and coverage
```

#### 3. Consistency Check
- [ ] Task status is marked as completed in all documents
- [ ] Technical details are consistent across documents  
- [ ] Dates and metrics align between documents
- [ ] All acceptance criteria are documented as met

### **Task Success Criteria (MANDATORY)**
Each task is only considered complete when ALL of the following criteria are met:

#### Core Implementation Requirements
- ✅ **Compilation Success**: `cargo check` passes without errors
- ✅ **Functionality Complete**: All specified features are implemented and working
- ✅ **Integration Tested**: Code integrates properly with existing systems

#### **MANDATORY: Comprehensive Test Suite**
Every task MUST include a comprehensive test suite following these standards:

**Test Coverage Requirements:**
- **Unit Tests**: Test all public functions, methods, and core logic
- **Integration Tests**: Test interaction with other modules/systems
- **Edge Case Tests**: Test boundary conditions, error cases, and invalid inputs
- **Pattern Tests**: For validation/enforcement systems, test all supported patterns
- **Async Tests**: Use `#[tokio::test]` for async functions

**Test Quality Standards:**
- **Descriptive Names**: Test names clearly describe what is being tested
- **Comprehensive Assertions**: Tests verify expected behavior, error conditions, and side effects
- **Representative Data**: Tests use realistic data and scenarios
- **Failure Testing**: Tests verify proper error handling and validation failures
- **Performance Boundaries**: Tests validate performance constraints where applicable

**Test Count Guidelines:**
- **Simple Tasks**: Minimum 5-8 focused test cases
- **Complex Tasks**: 15-20+ comprehensive test cases covering all major functionality
- **Validation Systems**: Test each validation rule, pattern, and error condition
- **Pattern Enforcement**: Test each supported pattern and compliance scenario

**Example Test Structure:**
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_functionality() {
        // Test core positive case
    }

    #[tokio::test]
    async fn test_async_operations() {
        // Test async functionality
    }

    #[test]
    fn test_error_handling() {
        // Test error conditions and recovery
    }

    #[test]
    fn test_edge_cases() {
        // Test boundary conditions
    }

    #[test]
    fn test_integration_points() {
        // Test interaction with other components
    }
}
```

#### **MANDATORY Documentation Requirements**
- ✅ **CRITICAL**: `docs/Phase_Task_Tracker.md` MUST BE UPDATED with task status, scope notes, and any scope shifts.
  - Mark checklist items complete/pending, include short implementation notes, and capture new risks.
- ✅ **CRITICAL**: `docs/SESSIONS/SESSION-CHECKPOINT-*.md` MUST BE UPDATED each day with work completed, commands, and next steps.
- ✅ **Test Metrics Documented**: Record test counts (unit/integration/frontend) in the relevant session checkpoint or tracker entry.
- ✅ **Implementation Summary**: List major files touched + rationale (session checkpoint or pull request description).

**FAILURE TO UPDATE TASK TRACKING DOCUMENT IS A PROCESS VIOLATION**

### **Task Success Criteria**
A task is complete only when:
- [ ] Zero compilation errors (`cargo check`)
- [ ] All tests pass (`cargo test`)
- [ ] Code is formatted (`cargo fmt`)
- [ ] Documentation is updated
- [ ] Changes are committed to feature branch
- [ ] All referenced documents show consistent status

### **Session Structure Template**
1. **Check status** (5 min) - Review current health with scripts
2. **TodoWrite planning** (3 min) - Plan session tasks  
3. **Focus work** (45-50 min) - Single large task or 2-3 smaller tasks
4. **Quality check** (5 min) - Run quality scripts before commit
5. **Documentation update** (2 min) - Update tracking documents

---

## 🌐 Part 12: Frontend Development (React + Vite)

### **Frontend Development Loop**
```bash
# 1. Ensure backend scaffold is running
curl -s http://localhost:8080/health

# 2. Start React dev server
cd frontend
npm install              # first run only
npm run dev -- --host 0.0.0.0 --port 5173

# 3. Keep DevTools open
# - Monitor console for React warnings/effect loops
# - Watch Network tab for excessive API calls
```

### **React Anti-Patterns to Avoid**
- ❌ `useEffect` without dependency arrays → infinite loops / request storms.
- ❌ Updating state inside render logic → re-render loops.
- ❌ Fetching data in event handlers without abort controllers → dangling requests when navigating away.
- ❌ Ignoring TanStack Query cache invalidation → stale or duplicated requests.

**Preferred Patterns**
- ✅ Use `useEffect` with explicit dependency lists or rely on TanStack Query hooks for data fetching.
- ✅ Co-locate query/mutation definitions inside `lib/api.ts` helpers for reuse.
- ✅ Use React Context sparingly; prefer prop drilling or dedicated stores when data is localized.
- ✅ Memoize expensive derived data (`useMemo`, `useCallback`) when passing into large component graphs.

### **Frontend Debugging Checklist**
- [ ] Console is free of warnings (React strict mode, prop type issues, failed requests).
- [ ] Network tab shows expected request cadence (no thundering herds, abort on unmount).
- [ ] Components unsubscribe/cleanup on unmount (`useEffect` returns a cleanup function).
- [ ] Styles responsive across window sizes (Stage 0 UI = Document + Insights panes).
- [ ] Hotkeys or pointer interactions tested in Chrome + Firefox (where possible).

### **Frontend Quality Gates**
```bash
cd frontend
npm run lint            # add ESLint once configured; run `npm run test` for Vitest suites
npm run build           # ensures Vite production build passes

# Manual verification before commit:
# - npm run dev, load http://localhost:5173
# - Toggle prompt presets, verify API traffic & streaming
# - Check hover/highlight interactions (once Stage 0 UI lands)
```

---

## 🔧 Part 13: Maintainability & Technical Debt

### **Priority-Based Refactoring**
When modules exceed size limits, prioritize by:
1. **Size + Impact**: Largest modules in critical paths first
2. **Educational Value**: Core learning modules get priority  
3. **Development Velocity**: Modules that slow down development

### **Refactoring Session Template**
For modules >900 lines:
```bash
# 1. Analyze structure - Identify logical boundaries
find src -name "*.rs" -exec wc -l {} + | sort -nr | head -10

# 2. Plan breakdown - Design sub-module organization  
mkdir src/module_name/
touch src/module_name/{mod.rs,core.rs,types.rs,errors.rs}

# 3. Move related functionality - Extract cohesive units
# 4. Update exports - Maintain clean public API
# 5. Test compilation - Verify no broken dependencies
cargo check --lib

# 6. Educational review - Ensure learning narrative preserved
```

### **Quality Health Dashboard**
Track automatically:
```bash
# Module size compliance  
echo "Modules >1000 lines: $(find src -name '*.rs' -exec wc -l {} + | awk '$1>1000' | wc -l)"

# Compilation health
cargo check --lib > /dev/null 2>&1 && echo "✅ Compiles" || echo "❌ Errors"

# Documentation coverage
echo "Missing docs: $(find src -name '*.rs' -exec grep -L '^//!' {} \; | wc -l)"
```

### **Technical Debt Prevention**
- **TodoWrite tool** - Track all tasks and progress systematically
- **Educational mindset** - Ask "what does this teach?" for every addition
- **Modular thinking** - Design for separation from the start  
- **Line count monitoring** - Check `wc -l module.rs` before major additions

### **Debt Reduction Principles**
- **One module per session** - Systematic refactoring approach
- **Quality first** - Fix critical issues before new features
- **Educational preservation** - Maintain learning value during cleanup
- **Incremental progress** - Small, consistent improvements

---

## ⚡ Quick Reference Card

```bash
# Backend Commands (Most Used)
cd backend
cargo check                 # After every change
cargo fmt                   # Before commits  
cargo test                  # After features
cargo clippy                # Code improvements
find src -name "*.rs" -exec wc -l {} + | sort -nr  # Module sizes

# Frontend Commands (React)
cd frontend
npm run dev                 # Start Vite dev server
npm run build               # Production build
npm run test                # (Add Vitest suites during Stage 0)

# Full Stack Development
curl -s http://localhost:8080/health  # Backend health
curl -I http://localhost:5173         # Frontend dev server

# Remote infra quick checks (CoreOS VM @ 10.10.100.50)
psql postgresql://colossus_user:colossus_password@10.10.100.50:5432/colossus -c "SELECT 1"
curl -s http://10.10.100.50:6333/collections | jq '.status'   # Qdrant collections list
curl -s -u neo4j:'Drwho2010$' \
     http://10.10.100.50:7474/db/neo4j/tx/commit \
     -H 'Content-Type: application/json' \
     -d '{"statements":[{"statement":"RETURN 1 as ok"}]}' | jq '.errors'  # Neo4j tx ping

# Quality Gates
./scripts/quality_check.sh           # Comprehensive quality verification
./scripts/check_module_sizes.sh      # Module size enforcement

# Error Recovery
git stash                  # Save broken state
git checkout main          # Return to safety
cargo check                # Verify clean state
git stash pop              # Try again smaller

# The Nuclear Option (when nothing works)
git checkout main
git pull
cargo clean
cargo build

# Health Monitoring
cargo check --lib --message-format short  # Quick error count
wc -l src/path/to/module.rs               # Check module size

# Frontend Debugging
# Open browser dev tools (F12) → Console/Network tabs
# Watch for: React warnings, thundering herd requests, stale caches
```

---

## 🚦 Remember: The Compiler Is Your Friend

**Use it continuously. Never fight it. Let it guide you.**

Every error message is a teacher. Every compilation is a victory.

Small steps with frequent validation beat large leaps into darkness.

---

*Last Updated: Based on lessons from the 491-error incident*
*Version: 2.0 - Unified Guide*
