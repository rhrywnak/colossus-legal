# Document Processor - Claude Code Instructions

> **Location**: `~/Projects/colossus-legal/tools/document-processor/CLAUDE.md`
> **Auto-read by**: Claude Code when working in this directory
> **Parent context**: Also reads `../../CLAUDE.md` for project-wide standards

## Component Overview

**document-processor** is a Rust CLI that extracts legal claims from court documents.

**Current State**: Migrating from v1 (chunk-based, failing) to v2 (sentence-based)

---

## Architecture: v2 Sentence-Based Extraction

```
Input (.md) → Clean → Segment → Classify (LLM) → Assemble → Output (.json)
     │          │         │           │             │
     │          │         │           │             └─ GroundedClaim[]
     │          │         │           └─ SentenceClassification[]
     │          │         └─ IndexedSentence[]
     │          └─ (cleaned text, line mapping)
     └─ Raw markdown with []{.span} noise
```

**Key Insight**: Grounding is guaranteed because we extract whole sentences. The sentence text IS the quote.

---

## Current Source Structure

```
tools/document-processor/
├── Cargo.toml
├── config.toml
├── src/
│   ├── main.rs          # Entry point, CLI parsing
│   ├── lib.rs           # Module exports
│   ├── config.rs        # Config loading
│   ├── paths.rs         # Path utilities
│   ├── logging.rs       # Log management
│   ├── prompt.rs        # Prompt handling
│   ├── dates.rs         # Date enrichment
│   │
│   ├── chunking.rs      # LEGACY - move to legacy/
│   ├── claims.rs        # LEGACY - move to legacy/
│   └── llm.rs           # LEGACY - move to legacy/
```

## Target Structure (After v2 Implementation)

```
src/
├── main.rs
├── lib.rs
├── config.rs
├── paths.rs
├── logging.rs
├── prompt.rs
├── dates.rs
│
├── preprocessing/           # NEW
│   ├── mod.rs
│   ├── cleaner.rs          # Markdown noise removal
│   └── segmenter.rs        # Sentence splitting
│
├── extraction/              # NEW
│   ├── mod.rs
│   ├── sentence_classifier.rs
│   ├── claim_assembler.rs
│   └── entity_extractor.rs
│
└── legacy/                  # Preserved old code
    ├── mod.rs
    ├── chunking.rs
    ├── claims.rs
    └── llm.rs
```

---

## Key Data Structures

### IndexedSentence (preprocessing output)

```rust
pub struct IndexedSentence {
    pub id: String,              // "S1", "S2", ...
    pub text: String,            // Exact sentence from document
    pub line_start: usize,       // Original line number
    pub line_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub context_before: Option<String>,
    pub context_after: Option<String>,
    pub paragraph_num: usize,
}
```

### SentenceClassification (LLM output)

```rust
pub struct SentenceClassification {
    pub sentence_id: String,     // Matches IndexedSentence.id
    pub is_claim: bool,
    pub claim_type: Option<String>,
    pub made_by: Option<String>,
    pub severity: Option<i32>,
    pub reason: Option<String>,
}
```

### GroundedClaim (final output)

```rust
pub struct GroundedClaim {
    pub id: String,              // "claim-0001"
    pub quote: String,           // Exact sentence (guaranteed grounded)
    pub line_start: usize,
    pub line_end: usize,
    pub char_start: usize,
    pub char_end: usize,
    pub claim_type: String,
    pub made_by: String,
    pub severity: i32,
    pub paragraph_num: usize,
    pub source_document: String,
    pub asserted_date: Option<String>,
    pub event_date: Option<String>,
    pub date_confidence: Option<String>,
}
```

---

## CLI Usage

```bash
# From tools/document-processor directory:

# v2 mode (sentence-based) - DEFAULT
cargo run -- /path/to/document.md --mode v2

# Legacy mode (chunk-based, for comparison)
cargo run -- /path/to/document.md --mode legacy

# With options
cargo run -- document.md \
    --mode v2 \
    --model llama3.1:8b-instruct \
    --output-dir ~/Documents/colossus-legal-data/output/
```

---

## Configuration

**File**: `./config.toml`

```toml
[directories]
input_directory = "~/Documents/colossus-legal-data/input"
output_directory = "~/Documents/colossus-legal-data/output"
prompt_directory = "~/Documents/colossus-legal-data/prompts"

[ollama]
url = "http://localhost:11434"
model = "llama3.1:8b-instruct"
temperature = 0.1
num_predict = 4096
timeout_seconds = 120

[defaults]
prompt_template = "prompt_template_v1.2.md"
output_suffix = ".claims.json"
```

---

## Implementation Notes

### Markdown Cleaning

Source documents contain noise like:
```
[Defendant CFS was appointed]{.span1}
[]{#section0001.xhtml}
```

Cleaner must:
1. `[text]{.spanN}` → keep `text`
2. `[]{...}` → remove entirely
3. Decorative lines → remove
4. Track line mapping for grounding

### Sentence Segmentation

Handle:
- Abbreviations: Mr., Mrs., Dr., Inc. → don't split
- Boilerplate: "Plaintiff hereby incorporates..." → skip
- Multi-line sentences → preserve as one

### LLM Classification

The prompt asks the LLM to classify, NOT extract:
- `is_claim`: true/false
- `claim_type`: factual_event, financial_harm, misconduct, etc.
- `made_by`: usually "plaintiff"
- `severity`: 1-10

Use `"format": "json"` in Ollama request.

---

## Testing

```bash
# Build
cargo build

# Unit tests
cargo test

# Single test with output
cargo test test_name -- --nocapture

# Clippy
cargo clippy

# Integration test on real document
cargo run -- ~/Documents/colossus-legal-data/input/Awad_v_Catholic_Family_Complaint_1-1-13.md --mode v2
```

**Success criteria for v2**:
- Grounding rate = 100%
- Claim count = 50-150 for test document
- All quotes appear verbatim in source

---

## Dependencies

```toml
[dependencies]
anyhow = "1.0"
chrono = { version = "0.4", features = ["serde"] }
once_cell = "1.19"
regex = "1.10"
reqwest = { version = "0.11", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tokio = { version = "1.0", features = ["full"] }
toml = "0.8"
```

---

## Rust Patterns (For Roman's Learning)

### Lazy Static Regex

```rust
use once_cell::sync::Lazy;
use regex::Regex;

static DATE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\d{1,2}/\d{1,2}/\d{4}").unwrap()
});
```

### Error Context Chain

```rust
let content = fs::read_to_string(&path)
    .with_context(|| format!("Failed to read: {}", path.display()))?;
```

### Module Re-exports

```rust
// In preprocessing/mod.rs
pub mod cleaner;
pub mod segmenter;

pub use cleaner::clean_markdown_noise;
pub use segmenter::{IndexedSentence, segment_sentences};
```

---

## Execution Plan Reference

For detailed implementation steps, see:
- `EXECUTION_PLAN_v2.md` - Technical specification
- `SONNET_ORCHESTRATION_GUIDE.md` - Step-by-step prompts
- `CLAUDE_CODE_INSTRUCTIONS.md` - Implementation guide
