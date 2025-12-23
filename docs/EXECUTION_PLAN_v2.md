# Document Processor v2: Sentence-Based Extraction
## Execution Plan for Claude Code / Sonnet 4.5

**Project**: Colossus Legal - Document Processor  
**Goal**: Replace failing LLM-based quote extraction with guaranteed-grounded sentence-based classification  
**Author**: Claude Opus 4.5 (Planning) → Claude Sonnet 4.5 / Claude Code (Execution)  
**Date**: 2025-12-17

---

## Executive Summary

The current approach asks local LLMs (qwen2.5-16k) to **locate AND quote** verbatim text from legal documents. This fails because:
1. Local LLMs summarize instead of extracting
2. Anchors are empty or fabricated
3. Grounding filter drops all claims

**Solution**: Separate concerns:
- **Rust does the locating** (sentence segmentation with line tracking)
- **LLM does classification** (much simpler: "is this a claim? what type?")
- **Grounding is automatic** (the sentence IS the quote)

---

## Architecture Overview

```
┌──────────────────────────────────────────────────────────────────────────┐
│                        DOCUMENT PROCESSOR v2                             │
├──────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  INPUT: Legal Document (.md)                                             │
│         │                                                                │
│         ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  PHASE 1: PREPROCESSING (Rust - New Module)                     │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌────────────────────┐    │    │
│  │  │ clean_noise  │→ │ segment_     │→ │ IndexedSentence[]  │    │    │
│  │  │ (strip .span)│  │ sentences    │  │ {line, start, end, │    │    │
│  │  └──────────────┘  └──────────────┘  │  text, context}    │    │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│         │                                                                │
│         ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  PHASE 2: BATCH CLASSIFICATION (LLM - Simpler Prompt)           │    │
│  │                                                                 │    │
│  │  Input to LLM (batches of 10-20 sentences):                     │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ [S47] "The estate suffered a net loss of $6000.00..."   │   │    │
│  │  │ [S48] "Defendants charged the estate some $7500.00..."  │   │    │
│  │  │ [S49] "Plaintiff hereby incorporates paragraphs 1..."   │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  │                                                                 │    │
│  │  LLM Response (classification only, no extraction):             │    │
│  │  ┌─────────────────────────────────────────────────────────┐   │    │
│  │  │ {"S47": {"is_claim": true, "type": "financial_harm",    │   │    │
│  │  │          "severity": 7, "made_by": "plaintiff"},        │   │    │
│  │  │  "S48": {"is_claim": true, "type": "financial_harm"...},│   │    │
│  │  │  "S49": {"is_claim": false, "type": "procedural"}}      │   │    │
│  │  └─────────────────────────────────────────────────────────┘   │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│         │                                                                │
│         ▼                                                                │
│  ┌─────────────────────────────────────────────────────────────────┐    │
│  │  PHASE 3: ASSEMBLY & ENRICHMENT (Rust)                          │    │
│  │  - Join classification results with IndexedSentences            │    │
│  │  - Build GroundedClaim structs (quote = original sentence)      │    │
│  │  - Extract entities (dates, amounts, names)                     │    │
│  │  - Date enrichment pass (existing logic)                        │    │
│  └─────────────────────────────────────────────────────────────────┘    │
│         │                                                                │
│         ▼                                                                │
│  OUTPUT: claims.json (100% grounded, verifiable)                        │
│                                                                          │
└──────────────────────────────────────────────────────────────────────────┘
```

---

## File Structure (After Implementation)

```
src/
├── main.rs              # Updated entry point with --mode flag
├── lib.rs               # Updated exports
├── config.rs            # Unchanged
├── paths.rs             # Unchanged
├── logging.rs           # Unchanged
├── prompt.rs            # Keep for legacy, add new prompt builder
│
├── preprocessing/       # NEW MODULE
│   ├── mod.rs
│   ├── cleaner.rs       # Markdown noise removal
│   └── segmenter.rs     # Sentence segmentation + indexing
│
├── extraction/          # NEW MODULE (replaces old approach)
│   ├── mod.rs
│   ├── sentence_classifier.rs   # Batch LLM classification
│   ├── claim_assembler.rs       # Build GroundedClaim from results
│   └── entity_extractor.rs      # Regex-based entity extraction
│
├── legacy/              # OLD CODE (keep for reference/fallback)
│   ├── mod.rs
│   ├── chunking.rs      # Old chunking logic
│   ├── claims.rs        # Old Claim struct + salvage
│   └── llm.rs           # Old extraction logic
│
└── dates.rs             # Keep existing date enrichment
```

---

## Phase 1: Preprocessing Module

### Task 1.1: Create `src/preprocessing/mod.rs`

```rust
//! Text preprocessing: cleaning and sentence segmentation.

pub mod cleaner;
pub mod segmenter;

pub use cleaner::clean_markdown_noise;
pub use segmenter::{IndexedSentence, segment_sentences};
```

### Task 1.2: Create `src/preprocessing/cleaner.rs`

**Purpose**: Remove markdown artifacts that interfere with extraction.

**Input Example** (from actual document):
```
[Defendant CFS was appointed by Judge Tighe over the objection of Mr.
Awad, a mentally competent adult, who asserted that he did not need to
have a guardian or conservator appointed for him.]{.span1}
```

**Output**:
```
Defendant CFS was appointed by Judge Tighe over the objection of Mr.
Awad, a mentally competent adult, who asserted that he did not need to
have a guardian or conservator appointed for him.
```

**Implementation**:
```rust
//! Markdown noise removal for legal documents.

use regex::Regex;
use once_cell::sync::Lazy;

// Patterns to clean (compiled once)
static SPAN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // Matches: [text]{.spanN} or []{.spanN} or [text]{#id}
    Regex::new(r"\[([^\]]*)\]\{[^}]+\}").unwrap()
});

static EMPTY_BRACKETS: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\[\s*\]").unwrap()
});

static MULTIPLE_SPACES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r" {2,}").unwrap()
});

static MULTIPLE_NEWLINES: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\n{3,}").unwrap()
});

/// Clean markdown noise while preserving document structure.
/// 
/// Returns (cleaned_text, line_number_mapping) where mapping tracks
/// how cleaned line numbers map to original line numbers.
pub fn clean_markdown_noise(input: &str) -> (String, Vec<usize>) {
    let mut result = String::with_capacity(input.len());
    let mut line_mapping: Vec<usize> = Vec::new();
    
    for (orig_line_num, line) in input.lines().enumerate() {
        // Extract content from [text]{.span} patterns
        let cleaned = SPAN_PATTERN.replace_all(line, "$1");
        
        // Remove empty brackets
        let cleaned = EMPTY_BRACKETS.replace_all(&cleaned, "");
        
        // Collapse multiple spaces
        let cleaned = MULTIPLE_SPACES.replace_all(&cleaned, " ");
        
        let trimmed = cleaned.trim();
        
        // Skip purely decorative lines (underscores, plus signs, pipes)
        if is_decorative_line(trimmed) {
            continue;
        }
        
        // Skip empty lines that result from cleaning
        if trimmed.is_empty() {
            continue;
        }
        
        result.push_str(trimmed);
        result.push('\n');
        line_mapping.push(orig_line_num + 1); // 1-indexed for humans
    }
    
    // Final cleanup: collapse excessive newlines
    let result = MULTIPLE_NEWLINES.replace_all(&result, "\n\n").to_string();
    
    (result, line_mapping)
}

/// Check if a line is purely decorative (table borders, separators).
fn is_decorative_line(line: &str) -> bool {
    if line.is_empty() {
        return true;
    }
    
    // Lines that are only underscores, dashes, pipes, plus signs
    let decorative_chars: &[char] = &['_', '-', '|', '+', '='];
    line.chars().all(|c| decorative_chars.contains(&c) || c.is_whitespace())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_span_removal() {
        let input = "[Defendant CFS was appointed]{.span1}";
        let (output, _) = clean_markdown_noise(input);
        assert_eq!(output.trim(), "Defendant CFS was appointed");
    }
    
    #[test]
    fn test_empty_bracket_removal() {
        let input = "[]{#section0001.xhtml}";
        let (output, _) = clean_markdown_noise(input);
        assert!(output.trim().is_empty());
    }
    
    #[test]
    fn test_decorative_line_removal() {
        let input = "Some text\n___________________________\nMore text";
        let (output, _) = clean_markdown_noise(input);
        assert!(!output.contains("___"));
    }
}
```

### Task 1.3: Create `src/preprocessing/segmenter.rs`

**Purpose**: Split cleaned text into indexed sentences with context.

**Key Design Decisions**:
1. Use paragraph boundaries as primary splits (legal docs are paragraph-structured)
2. Then split paragraphs into sentences
3. Preserve line numbers for grounding verification
4. Include surrounding context for LLM classification

```rust
//! Sentence segmentation with position tracking.

use serde::{Deserialize, Serialize};

/// A sentence with its location in the original document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexedSentence {
    /// Unique identifier (S1, S2, ...)
    pub id: String,
    
    /// The sentence text (verbatim from cleaned document)
    pub text: String,
    
    /// Original line number(s) this sentence spans
    pub line_start: usize,
    pub line_end: usize,
    
    /// Character offsets in cleaned document
    pub char_start: usize,
    pub char_end: usize,
    
    /// Previous sentence (for context)
    pub context_before: Option<String>,
    
    /// Next sentence (for context)  
    pub context_after: Option<String>,
    
    /// Paragraph number (for grouping related sentences)
    pub paragraph_num: usize,
}

/// Segment text into indexed sentences.
/// 
/// # Arguments
/// * `text` - Cleaned document text
/// * `line_mapping` - Mapping from cleaned line numbers to original
/// 
/// # Returns
/// Vector of IndexedSentence structs
pub fn segment_sentences(text: &str, line_mapping: &[usize]) -> Vec<IndexedSentence> {
    let mut sentences: Vec<IndexedSentence> = Vec::new();
    let mut sentence_counter = 0usize;
    
    // Split into paragraphs first (double newline)
    let paragraphs: Vec<&str> = text.split("\n\n")
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .collect();
    
    let mut global_char_offset = 0usize;
    let mut current_line = 0usize;
    
    for (para_idx, paragraph) in paragraphs.iter().enumerate() {
        // Split paragraph into sentences
        let para_sentences = split_into_sentences(paragraph);
        
        for sent_text in para_sentences {
            if sent_text.trim().is_empty() {
                continue;
            }
            
            // Skip boilerplate (paragraph incorporations, etc.)
            if is_boilerplate(&sent_text) {
                global_char_offset += sent_text.len() + 1;
                continue;
            }
            
            sentence_counter += 1;
            
            let char_start = global_char_offset;
            let char_end = char_start + sent_text.len();
            
            // Map character offset to line number
            let line_start = char_offset_to_line(text, char_start, line_mapping);
            let line_end = char_offset_to_line(text, char_end.saturating_sub(1), line_mapping);
            
            sentences.push(IndexedSentence {
                id: format!("S{}", sentence_counter),
                text: sent_text.to_string(),
                line_start,
                line_end,
                char_start,
                char_end,
                context_before: None, // Filled in post-pass
                context_after: None,
                paragraph_num: para_idx + 1,
            });
            
            global_char_offset = char_end + 1; // +1 for space/newline
        }
        
        global_char_offset += 2; // paragraph break
    }
    
    // Post-pass: fill in context
    let sentences_clone = sentences.clone();
    for (i, sentence) in sentences.iter_mut().enumerate() {
        if i > 0 {
            sentence.context_before = Some(truncate_context(&sentences_clone[i-1].text, 100));
        }
        if i < sentences_clone.len() - 1 {
            sentence.context_after = Some(truncate_context(&sentences_clone[i+1].text, 100));
        }
    }
    
    sentences
}

/// Split a paragraph into sentences using legal document heuristics.
fn split_into_sentences(paragraph: &str) -> Vec<String> {
    let mut sentences = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = paragraph.chars().collect();
    let len = chars.len();
    
    let mut i = 0;
    while i < len {
        let c = chars[i];
        current.push(c);
        
        // Check for sentence boundary
        if is_sentence_end(c) {
            // Look ahead: is this really end of sentence?
            // Not if followed by lowercase or common abbreviations
            let next_char = chars.get(i + 1).copied();
            let next_next_char = chars.get(i + 2).copied();
            
            let is_real_end = match (next_char, next_next_char) {
                (Some(' '), Some(nc)) if nc.is_uppercase() => true,
                (Some('\n'), _) => true,
                (None, _) => true,
                _ => {
                    // Check for common abbreviations
                    !is_abbreviation(&current)
                }
            };
            
            if is_real_end {
                let trimmed = current.trim().to_string();
                if !trimmed.is_empty() {
                    sentences.push(trimmed);
                }
                current = String::new();
            }
        }
        
        i += 1;
    }
    
    // Don't forget remaining text
    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }
    
    sentences
}

fn is_sentence_end(c: char) -> bool {
    c == '.' || c == '!' || c == '?'
}

fn is_abbreviation(text: &str) -> bool {
    let lower = text.to_lowercase();
    let abbrevs = [
        "mr.", "mrs.", "ms.", "dr.", "jr.", "sr.",
        "inc.", "corp.", "llc.", "pllc.",
        "no.", "nos.", "vs.", "v.",
        "hon.", "esq.",
        "jan.", "feb.", "mar.", "apr.", "jun.", "jul.", "aug.", "sep.", "oct.", "nov.", "dec.",
    ];
    abbrevs.iter().any(|a| lower.ends_with(a))
}

/// Check if sentence is legal boilerplate to skip.
fn is_boilerplate(text: &str) -> bool {
    let lower = text.to_lowercase();
    
    // "Plaintiff hereby incorporates paragraphs 1 through X"
    if lower.contains("hereby incorporates paragraphs") {
        return true;
    }
    
    // "NOW COMES, Plaintiff..."  
    if lower.starts_with("now comes") {
        return true;
    }
    
    // "WHEREFORE, Plaintiff respectfully requests..."
    if lower.starts_with("wherefore") {
        return true;
    }
    
    false
}

fn char_offset_to_line(text: &str, offset: usize, line_mapping: &[usize]) -> usize {
    let line_in_cleaned = text[..offset.min(text.len())]
        .chars()
        .filter(|&c| c == '\n')
        .count();
    
    line_mapping.get(line_in_cleaned).copied().unwrap_or(line_in_cleaned + 1)
}

fn truncate_context(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        text.to_string()
    } else {
        format!("{}...", &text[..max_len])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sentence_splitting() {
        let para = "Mr. Awad was present. The court ruled against him. He appealed.";
        let sentences = split_into_sentences(para);
        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("Mr. Awad"));
    }
    
    #[test]
    fn test_boilerplate_detection() {
        assert!(is_boilerplate("Plaintiff hereby incorporates paragraphs 1 through 71 as though fully reinstated herein."));
        assert!(!is_boilerplate("Defendant CFS was appointed as guardian."));
    }
}
```

---

## Phase 2: Sentence Classification Module

### Task 2.1: Create `src/extraction/mod.rs`

```rust
//! Extraction module: sentence classification and claim assembly.

pub mod sentence_classifier;
pub mod claim_assembler;
pub mod entity_extractor;

pub use sentence_classifier::{classify_sentences, SentenceClassification};
pub use claim_assembler::{assemble_claims, GroundedClaim};
pub use entity_extractor::extract_entities;
```

### Task 2.2: Create `src/extraction/sentence_classifier.rs`

**Purpose**: Send sentence batches to LLM for classification (NOT extraction).

**Key Insight**: This is a MUCH simpler task for the LLM. No verbatim copying required.

```rust
//! Batch sentence classification via LLM.

use std::collections::HashMap;
use std::time::Duration;

use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::preprocessing::IndexedSentence;

/// Classification result for a single sentence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SentenceClassification {
    /// Sentence ID (S1, S2, ...)
    pub sentence_id: String,
    
    /// Is this sentence a factual claim?
    pub is_claim: bool,
    
    /// Claim type (if is_claim = true)
    #[serde(default)]
    pub claim_type: Option<String>,
    
    /// Who is making/asserting this claim
    #[serde(default)]
    pub made_by: Option<String>,
    
    /// Severity 1-10 (if is_claim = true)
    #[serde(default)]
    pub severity: Option<i32>,
    
    /// Brief reason for classification (optional, for debugging)
    #[serde(default)]
    pub reason: Option<String>,
}

/// LLM response structure
#[derive(Debug, Deserialize)]
struct ClassificationResponse {
    classifications: Vec<SentenceClassification>,
}

/// Classify sentences in batches.
/// 
/// # Arguments
/// * `sentences` - Indexed sentences to classify
/// * `ollama_url` - Ollama API URL
/// * `model` - Model name
/// * `batch_size` - Sentences per LLM call (recommended: 15-20)
/// 
/// # Returns
/// Map of sentence_id -> classification
pub async fn classify_sentences(
    sentences: &[IndexedSentence],
    ollama_url: &str,
    model: &str,
    temperature: f32,
    timeout_seconds: u64,
    batch_size: usize,
) -> Result<HashMap<String, SentenceClassification>> {
    let client = Client::new();
    let mut results: HashMap<String, SentenceClassification> = HashMap::new();
    
    // Process in batches
    for (batch_idx, batch) in sentences.chunks(batch_size).enumerate() {
        println!("  Classifying batch {}/{}", batch_idx + 1, 
                 (sentences.len() + batch_size - 1) / batch_size);
        
        let prompt = build_classification_prompt(batch);
        
        let response = client
            .post(format!("{}/api/generate", ollama_url))
            .json(&json!({
                "model": model,
                "prompt": prompt,
                "stream": false,
                "format": "json",
                "options": {
                    "temperature": temperature,
                    "num_predict": 4096,
                }
            }))
            .timeout(Duration::from_secs(timeout_seconds))
            .send()
            .await
            .context("Failed to call Ollama API")?;
        
        if !response.status().is_success() {
            bail!("Ollama returned error: {}", response.status());
        }
        
        let result: serde_json::Value = response.json().await
            .context("Failed to parse Ollama response")?;
        
        let response_text = result["response"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("No response from LLM"))?;
        
        // Parse classifications
        let classifications = parse_classifications(response_text, batch)?;
        
        for classification in classifications {
            results.insert(classification.sentence_id.clone(), classification);
        }
    }
    
    Ok(results)
}

/// Build the classification prompt for a batch of sentences.
fn build_classification_prompt(sentences: &[IndexedSentence]) -> String {
    let mut prompt = String::from(CLASSIFICATION_PROMPT_HEADER);
    
    prompt.push_str("\n\n## SENTENCES TO CLASSIFY:\n\n");
    
    for sentence in sentences {
        // Include context if available
        if let Some(ref before) = sentence.context_before {
            prompt.push_str(&format!("(Context: ...{})\n", before));
        }
        
        prompt.push_str(&format!("[{}] \"{}\"\n", sentence.id, sentence.text));
        
        if let Some(ref after) = sentence.context_after {
            prompt.push_str(&format!("(Followed by: {}...)\n", after));
        }
        
        prompt.push_str("\n");
    }
    
    prompt.push_str(CLASSIFICATION_PROMPT_FOOTER);
    
    prompt
}

const CLASSIFICATION_PROMPT_HEADER: &str = r#"# Legal Sentence Classification Task

You are classifying sentences from a legal complaint. For each sentence, determine:

1. **is_claim**: Is this sentence a FACTUAL CLAIM (an assertion of fact that could be proven true or false)?
   - TRUE for: allegations, statements of events, dates, actions taken, damages claimed
   - FALSE for: legal conclusions, procedural statements, requests for relief, boilerplate

2. **claim_type**: If is_claim=true, categorize as one of:
   - "factual_event" - Something that happened (dates, actions, events)
   - "financial_harm" - Money amounts, damages, losses
   - "misconduct" - Allegations of wrongdoing, fraud, breach
   - "procedural" - Court actions, filings, orders
   - "relationship" - Parties, roles, organizational structure

3. **made_by**: Who is asserting this? Usually "plaintiff" in a complaint.

4. **severity**: 1-10 scale (10 = most serious allegation)

5. **reason**: Brief explanation (1 sentence max)

## IMPORTANT RULES:
- Respond with ONLY valid JSON
- Include ALL sentence IDs in your response
- If unsure, set is_claim=false
- Do NOT modify or summarize the sentences
"#;

const CLASSIFICATION_PROMPT_FOOTER: &str = r#"

## RESPONSE FORMAT:
```json
{
  "classifications": [
    {
      "sentence_id": "S1",
      "is_claim": true,
      "claim_type": "factual_event",
      "made_by": "plaintiff", 
      "severity": 7,
      "reason": "Specific dated allegation of defendant action"
    },
    {
      "sentence_id": "S2",
      "is_claim": false,
      "reason": "Procedural incorporation by reference"
    }
  ]
}
```

Respond with ONLY the JSON object, no other text.
"#;

/// Parse LLM response into classifications.
fn parse_classifications(
    response_text: &str, 
    batch: &[IndexedSentence]
) -> Result<Vec<SentenceClassification>> {
    // Clean response
    let cleaned = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    
    // Try to parse
    let parsed: ClassificationResponse = serde_json::from_str(cleaned)
        .context("Failed to parse classification JSON")?;
    
    // Validate all sentences got classified
    let batch_ids: std::collections::HashSet<&str> = batch.iter()
        .map(|s| s.id.as_str())
        .collect();
    
    let response_ids: std::collections::HashSet<&str> = parsed.classifications.iter()
        .map(|c| c.sentence_id.as_str())
        .collect();
    
    let missing: Vec<&&str> = batch_ids.difference(&response_ids).collect();
    if !missing.is_empty() {
        eprintln!("Warning: LLM missed sentences: {:?}", missing);
        // Don't fail - just note the missing ones
    }
    
    Ok(parsed.classifications)
}
```

### Task 2.3: Create `src/extraction/claim_assembler.rs`

**Purpose**: Combine IndexedSentence + SentenceClassification into final GroundedClaim.

```rust
//! Assemble classified sentences into grounded claims.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::preprocessing::IndexedSentence;
use super::sentence_classifier::SentenceClassification;

/// A claim that is guaranteed to be grounded in the source document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GroundedClaim {
    /// Unique identifier
    pub id: String,
    
    /// The exact sentence from the document (verbatim, guaranteed grounded)
    pub quote: String,
    
    /// Line number(s) in original document
    pub line_start: usize,
    pub line_end: usize,
    
    /// Character offsets for precise location
    pub char_start: usize,
    pub char_end: usize,
    
    /// Classification metadata
    pub claim_type: String,
    pub made_by: String,
    pub severity: i32,
    
    /// Paragraph context
    pub paragraph_num: usize,
    
    /// Date fields (to be enriched in later pass)
    #[serde(default)]
    pub asserted_date: Option<String>,
    #[serde(default)]  
    pub event_date: Option<String>,
    #[serde(default)]
    pub date_confidence: Option<String>,
    
    /// Source document name
    pub source_document: String,
}

/// Assemble grounded claims from sentences and classifications.
pub fn assemble_claims(
    sentences: &[IndexedSentence],
    classifications: &HashMap<String, SentenceClassification>,
    document_name: &str,
) -> Vec<GroundedClaim> {
    let mut claims: Vec<GroundedClaim> = Vec::new();
    let mut claim_counter = 0usize;
    
    for sentence in sentences {
        // Look up classification
        let classification = match classifications.get(&sentence.id) {
            Some(c) => c,
            None => continue, // Sentence wasn't classified
        };
        
        // Skip non-claims
        if !classification.is_claim {
            continue;
        }
        
        claim_counter += 1;
        
        let claim = GroundedClaim {
            id: format!("claim-{:04}", claim_counter),
            quote: sentence.text.clone(),
            line_start: sentence.line_start,
            line_end: sentence.line_end,
            char_start: sentence.char_start,
            char_end: sentence.char_end,
            claim_type: classification.claim_type.clone()
                .unwrap_or_else(|| "unknown".to_string()),
            made_by: classification.made_by.clone()
                .unwrap_or_else(|| "plaintiff".to_string()),
            severity: classification.severity.unwrap_or(5),
            paragraph_num: sentence.paragraph_num,
            asserted_date: None,
            event_date: None,
            date_confidence: None,
            source_document: document_name.to_string(),
        };
        
        claims.push(claim);
    }
    
    claims
}

/// Verify a claim is actually grounded in the document.
/// This should always return true for our sentence-based extraction,
/// but provides a sanity check.
pub fn verify_grounding(claim: &GroundedClaim, document_text: &str) -> bool {
    // Normalize whitespace for comparison
    let doc_normalized: String = document_text
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    
    let quote_normalized: String = claim.quote
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    
    doc_normalized.contains(&quote_normalized)
}
```

### Task 2.4: Create `src/extraction/entity_extractor.rs`

**Purpose**: Extract structured entities (dates, amounts, names) from claims using regex.

```rust
//! Entity extraction from claim text using regex patterns.

use regex::Regex;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};

/// Extracted entities from a claim.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ExtractedEntities {
    /// Dates found in the claim
    pub dates: Vec<String>,
    
    /// Dollar amounts
    pub amounts: Vec<String>,
    
    /// Named parties/people
    pub parties: Vec<String>,
    
    /// Legal case references
    pub case_refs: Vec<String>,
}

// Compile patterns once
static DATE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(january|february|march|april|may|june|july|august|september|october|november|december)\s+\d{1,2},?\s+\d{4}|\d{1,2}/\d{1,2}/\d{2,4}").unwrap()
});

static AMOUNT_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\$[\d,]+(?:\.\d{2})?").unwrap()
});

static PARTY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // Matches "Defendant X", "Plaintiff Y", common legal party references
    Regex::new(r"(?i)(Defendant|Plaintiff|Mr\.|Mrs\.|Ms\.|Dr\.)\s+[A-Z][a-zA-Z]+(?:\s+[A-Z][a-zA-Z]+)?").unwrap()
});

static MCLA_PATTERN: Lazy<Regex> = Lazy::new(|| {
    // Michigan Compiled Laws reference
    Regex::new(r"MCLA\s+[\d.]+").unwrap()
});

/// Extract entities from claim text.
pub fn extract_entities(text: &str) -> ExtractedEntities {
    let mut entities = ExtractedEntities::default();
    
    // Extract dates
    for cap in DATE_PATTERN.find_iter(text) {
        entities.dates.push(cap.as_str().to_string());
    }
    
    // Extract amounts
    for cap in AMOUNT_PATTERN.find_iter(text) {
        entities.amounts.push(cap.as_str().to_string());
    }
    
    // Extract party names
    for cap in PARTY_PATTERN.find_iter(text) {
        entities.parties.push(cap.as_str().to_string());
    }
    
    // Extract legal references
    for cap in MCLA_PATTERN.find_iter(text) {
        entities.case_refs.push(cap.as_str().to_string());
    }
    
    entities
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_date_extraction() {
        let text = "On April 21, 2009, the court ruled. Then on 5/15/2010 an appeal was filed.";
        let entities = extract_entities(text);
        assert_eq!(entities.dates.len(), 2);
    }
    
    #[test]
    fn test_amount_extraction() {
        let text = "The estate lost $6,000.00 while defendants took $7,500.00";
        let entities = extract_entities(text);
        assert_eq!(entities.amounts.len(), 2);
        assert!(entities.amounts.contains(&"$6,000.00".to_string()));
    }
    
    #[test]
    fn test_party_extraction() {
        let text = "Defendant CFS and Mr. Awad appeared before the court.";
        let entities = extract_entities(text);
        assert!(entities.parties.len() >= 2);
    }
}
```

---

## Phase 3: Integration

### Task 3.1: Update `src/lib.rs`

```rust
//! Document Processor Library
//! 
//! Provides legal document claim extraction with guaranteed grounding.

pub mod config;
pub mod paths;
pub mod logging;

// New v2 modules
pub mod preprocessing;
pub mod extraction;

// Legacy modules (for fallback/comparison)
pub mod legacy {
    pub mod chunking;
    pub mod claims;
    pub mod llm;
}

pub mod dates;
pub mod prompt;
```

### Task 3.2: Update `src/main.rs`

Add `--mode` flag to switch between legacy and v2 extraction:

```rust
// Add to argument parsing section:

let mut extraction_mode = "v2".to_string(); // Default to new mode

// In the argument parsing loop:
"--mode" => {
    if i + 1 < args.len() {
        extraction_mode = args[i + 1].clone();
        i += 2;
    } else {
        bail!("Error: --mode requires 'v2' or 'legacy'");
    }
}

// In run_processing(), after loading document:

match extraction_mode.as_str() {
    "v2" => {
        run_v2_extraction(&text, &document_name, &config, model_name, &output_path).await
    }
    "legacy" => {
        run_legacy_extraction(&text, &prompt_template_text, &document_name, &config, model_name, &output_path).await
    }
    _ => bail!("Unknown mode: {}. Use 'v2' or 'legacy'", extraction_mode)
}
```

### Task 3.3: Create v2 extraction function

```rust
async fn run_v2_extraction(
    text: &str,
    document_name: &str,
    config: &Config,
    model_name: &str,
    output_path: &Path,
) -> Result<()> {
    use document_processor::preprocessing::{clean_markdown_noise, segment_sentences};
    use document_processor::extraction::{classify_sentences, assemble_claims, verify_grounding};
    
    // Phase 1: Preprocessing
    println!("📝 Phase 1: Cleaning and segmenting document...");
    let (cleaned_text, line_mapping) = clean_markdown_noise(text);
    let sentences = segment_sentences(&cleaned_text, &line_mapping);
    println!("   Found {} sentences", sentences.len());
    
    // Phase 2: Classification
    println!("🤖 Phase 2: Classifying sentences with {}...", model_name);
    let classifications = classify_sentences(
        &sentences,
        &config.ollama.url,
        model_name,
        config.ollama.temperature,
        config.ollama.timeout_seconds,
        15, // batch_size
    ).await?;
    
    let claim_count = classifications.values().filter(|c| c.is_claim).count();
    println!("   Classified {} claims", claim_count);
    
    // Phase 3: Assembly
    println!("🔧 Phase 3: Assembling grounded claims...");
    let mut claims = assemble_claims(&sentences, &classifications, document_name);
    
    // Verify grounding (should always pass, but sanity check)
    let grounded_count = claims.iter()
        .filter(|c| verify_grounding(c, text))
        .count();
    println!("   Verified {}/{} claims are grounded", grounded_count, claims.len());
    
    // Phase 4: Date enrichment (reuse existing logic)
    println!("📅 Phase 4: Enriching dates...");
    // Note: May need to adapt dates.rs to work with GroundedClaim
    // For now, skip or create adapter
    
    // Output
    let output = serde_json::json!({
        "document": document_name,
        "extraction_mode": "v2_sentence_based",
        "sentence_count": sentences.len(),
        "claim_count": claims.len(),
        "claims": claims,
    });
    
    let json_output = serde_json::to_string_pretty(&output)?;
    std::fs::write(output_path, &json_output)?;
    
    println!("💾 Output: {}", output_path.display());
    println!("✅ Extracted {} grounded claims", claims.len());
    
    Ok(())
}
```

---

## Phase 4: Testing & Validation

### Task 4.1: Create test script

```bash
#!/bin/bash
# test_v2_extraction.sh

echo "=== Testing Document Processor v2 ==="

INPUT="$HOME/Documents/colossus-legal-data/input/Awad_v_Catholic_Family_Complaint_1-1-13.md"
OUTPUT_DIR="$HOME/Documents/colossus-legal-data/output"

# Test v2 mode
echo ""
echo "Testing v2 (sentence-based) extraction..."
cargo run -- "$INPUT" --mode v2 --output-dir "$OUTPUT_DIR"

# Compare with legacy (optional)
echo ""
echo "Testing legacy extraction for comparison..."  
cargo run -- "$INPUT" --mode legacy --output-dir "$OUTPUT_DIR" \
    --output "legacy_output.json"

echo ""
echo "=== Comparing Results ==="
echo "V2 claims:"
jq '.claim_count' "$OUTPUT_DIR/Awad_v_Catholic_Family_Complaint_1-1-13.md.claims.json"

echo "Legacy claims (after filter):"
jq '.claim_count' "$OUTPUT_DIR/legacy_output.json"
```

### Task 4.2: Validation criteria

A successful v2 extraction should:

1. **100% grounding rate**: Every claim.quote appears verbatim in source
2. **No empty anchors**: Anchors replaced by line numbers
3. **Meaningful claims**: Financial amounts, dates, specific allegations
4. **No duplicates**: Same sentence not classified multiple times
5. **Reasonable count**: ~50-150 claims for a 700-line complaint

---

## Cargo.toml Dependencies

Ensure these are present:

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
```

---

## Execution Checklist for Claude Code

### Setup
- [ ] Create `src/preprocessing/` directory
- [ ] Create `src/extraction/` directory  
- [ ] Create `src/legacy/` directory
- [ ] Move old files to legacy

### Phase 1: Preprocessing
- [ ] Implement `preprocessing/mod.rs`
- [ ] Implement `preprocessing/cleaner.rs` with tests
- [ ] Implement `preprocessing/segmenter.rs` with tests
- [ ] Test on actual document: verify cleaning removes `[]{.span}` noise

### Phase 2: Classification
- [ ] Implement `extraction/mod.rs`
- [ ] Implement `extraction/sentence_classifier.rs`
- [ ] Implement `extraction/claim_assembler.rs`
- [ ] Implement `extraction/entity_extractor.rs` with tests
- [ ] Test classification prompt with Ollama

### Phase 3: Integration
- [ ] Update `lib.rs` exports
- [ ] Update `main.rs` with --mode flag
- [ ] Create `run_v2_extraction` function
- [ ] Adapt date enrichment for GroundedClaim

### Phase 4: Testing
- [ ] Run on Awad complaint
- [ ] Verify 100% grounding
- [ ] Compare claim quality vs legacy
- [ ] Document results

---

## Expected Output Example

After successful v2 extraction on `Awad_v_Catholic_Family_Complaint_1-1-13.md`:

```json
{
  "document": "Awad_v_Catholic_Family_Complaint_1-1-13.md",
  "extraction_mode": "v2_sentence_based",
  "sentence_count": 245,
  "claim_count": 87,
  "claims": [
    {
      "id": "claim-0001",
      "quote": "Although Judge Tighe ruled from the bench on April 21, 2009 that Defendant CFS was to be appointed as Mr. Awad's guardian and conservator, a formal order was never entered before or after his death in May of 2009.",
      "line_start": 145,
      "line_end": 148,
      "char_start": 4521,
      "char_end": 4756,
      "claim_type": "factual_event",
      "made_by": "plaintiff",
      "severity": 8,
      "paragraph_num": 12,
      "event_date": "2009-04-21",
      "source_document": "Awad_v_Catholic_Family_Complaint_1-1-13.md"
    },
    {
      "id": "claim-0002", 
      "quote": "The estate suffered a net loss of approximately $6000.00 as a result of the 'auction' on worthless junk while CFS and PHILLIPS enriched their pockets by a combined total of $7,500.00.",
      "line_start": 298,
      "line_end": 300,
      "claim_type": "financial_harm",
      "made_by": "plaintiff",
      "severity": 7,
      "paragraph_num": 38
    }
  ]
}
```

---

## Model Recommendations

Given 2x RTX 5060 TI (16GB each), try these models in order:

1. **`llama3.1:8b-instruct`** - Best instruction following at this size
2. **`mistral:7b-instruct-v0.2`** - Good at classification tasks
3. **`qwen2.5:14b`** (if VRAM allows with quantization) - Your current model family

Test classification quality with each before full runs.

---

## Notes for Executing Agent

1. **Read this entire document first** before writing any code
2. **Test incrementally**: preprocessing first, then classification, then integration
3. **Preserve working code**: Move old files to `legacy/`, don't delete
4. **Log liberally**: Add debug output to track what's happening
5. **Verify grounding**: The key metric is 100% of claims appearing in source

Good luck! 🚀
