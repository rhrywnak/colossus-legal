# Colossus Legal - Document Processor

A professional CLI tool for extracting legal claims from documents using local LLMs with comprehensive logging and analysis capabilities.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [File Structure](#file-structure)
- [Output Format](#output-format)
- [Logging](#logging)
- [Testing & Comparison](#testing--comparison)
- [Troubleshooting](#troubleshooting)
- [Outstanding Tasks](#outstanding-tasks)

---

## Overview

### Features

✅ **Local LLM Processing** - Uses Ollama for privacy-focused claim extraction  
✅ **Configurable Prompts** - Customize extraction logic per document type  
✅ **Professional Logging** - Comprehensive metrics for every run  
✅ **Unique Run IDs** - Never overwrite results, compare different runs  
✅ **Multiple Model Support** - Test with different models (qwen, llama, mistral)  
✅ **Date Extraction** - Automatically extracts dates mentioned in claims  
✅ **Structured Output** - JSON format ready for Neo4j import  
✅ **XDG Compliant** - Follows Linux/Unix standards for config location  

### Requirements

- Rust 1.70+ (for building)
- Ollama running locally with qwen2.5-16k or similar model
- Linux/macOS (Windows not tested)
- 16GB+ RAM recommended
- GPU optional but recommended (2x faster processing)

---

## Architecture

### Design Principles

1. **Code ≠ Config ≠ Data** - Clean separation of concerns
2. **Standard Locations** - Follows XDG Base Directory specification
3. **Non-Destructive** - Unique filenames prevent data loss
4. **Traceable** - Every output linked to its configuration via logs
5. **Testable** - Compare different models/parameters on same input

### Component Overview
```
┌─────────────────┐
│   Binary        │  ~/.cargo/bin/document-processor
│   (Code)        │
└─────────────────┘
        │
        ├─── reads ───→ ┌─────────────────┐
        │               │  Config         │  ~/.config/colossus-legal/config.toml
        │               │  (Settings)     │
        │               └─────────────────┘
        │
        └─── uses ────→ ┌─────────────────┐
                        │  Data           │  ~/Documents/colossus-legal-data/
                        │  (User Content) │  ├── input/
                        │                 │  ├── extracted/
                        │                 │  ├── prompts/
                        │                 │  └── logs/
                        └─────────────────┘
```

---

## Installation

### Quick Install
```bash
cd ~/Projects/colossus-legal/tools/document-processor
./install.sh
```

This will:
1. Build release binary → `~/.cargo/bin/document-processor`
2. Create config → `~/.config/colossus-legal/config.toml`
3. Set up data directories → `~/Documents/colossus-legal-data/`
4. Install default prompt templates

### Manual Installation
```bash
# Build
cargo build --release

# Install binary
cargo install --path .

# Create directories
mkdir -p ~/.config/colossus-legal
mkdir -p ~/Documents/colossus-legal-data/{input,extracted,prompts,logs}

# Copy config
cp config.toml ~/.config/colossus-legal/

# Copy prompts
cp prompts/* ~/Documents/colossus-legal-data/prompts/
```

### Post-Installation

Edit config to match your system:
```bash
nano ~/.config/colossus-legal/config.toml
```

Update paths for your username/environment.

---

## Configuration

### Config File Location

**Search order (highest priority first):**

1. `--config /path/to/config.toml` (CLI flag)
2. `$COLOSSUS_CONFIG` (environment variable)
3. `~/.config/colossus-legal/config.toml` (user config) ⭐ **DEFAULT**
4. `/etc/colossus-legal/config.toml` (system config)

### Config File Structure

**Location:** `~/.config/colossus-legal/config.toml`
```toml
[ollama]
url = "http://localhost:11434"
model = "qwen2.5-16k"           # Default model
temperature = 0.1                # Lower = more deterministic
num_predict = 16000              # Max output tokens
timeout_seconds = 1800           # 30 minutes

[directories]
# All paths must exist (tool will error if missing)
input_directory = "/home/YOUR_USERNAME/Documents/colossus-legal-data/input"
output_directory = "/home/YOUR_USERNAME/Documents/colossus-legal-data/extracted"
prompt_directory = "/home/YOUR_USERNAME/Documents/colossus-legal-data/prompts"

[defaults]
prompt_template = "prompt_template.md"
output_suffix = ".claims.json"

[neo4j]
# For future --commit feature
url = "bolt://10.10.100.50:7687"
user = "neo4j"
password = "YOUR_PASSWORD"
```

### Environment Variable Override
```bash
# Use different config
export COLOSSUS_CONFIG=~/my-test-config.toml
document-processor document.md

# Or inline
COLOSSUS_CONFIG=~/test.toml document-processor document.md
```

---

## Usage

### Basic Usage
```bash
# Process a document (assumes file is in input directory)
document-processor document.md

# Specify full path
document-processor /full/path/to/document.md

# Custom output location
document-processor document.md -o /path/to/output.json
```

### Advanced Options
```bash
# Use different model
document-processor document.md --model qwen2.5-7b
document-processor document.md --model llama3.1-8b

# Use custom prompt
document-processor document.md -p affidavit_prompt.md

# Override directories (for this run only)
document-processor document.md --input-dir /other/input --output-dir /other/output

# Use different config
document-processor document.md --config ~/test-config.toml

# Combine options
document-processor motion.md \
  --model qwen2.5-32b \
  -p motion_prompt.md \
  -o ~/special_output.json
```

### Testing Multiple Models
```bash
# Test same document with different models
document-processor motion.md --model qwen2.5-16k
document-processor motion.md --model qwen2.5-7b
document-processor motion.md --model llama3.1-8b

# Results won't overwrite (unique timestamps + run IDs)
ls -lh ~/Documents/colossus-legal-data/extracted/*motion*
```

### Help
```bash
document-processor --help
document-processor -h
```

---

## File Structure

### Project Structure
```
~/Projects/colossus-legal/tools/document-processor/
├── src/
│   └── main.rs                    # Main application code
├── Cargo.toml                      # Rust dependencies
├── config.toml                     # Template config (for installation)
├── prompts/
│   └── prompt_template.md         # Template prompt (for installation)
├── install.sh                      # Installation script
└── README.md                       # This file
```

### Installed Binary
```
~/.cargo/bin/
└── document-processor             # Executable (in PATH)
```

### Configuration Directory
```
~/.config/colossus-legal/
└── config.toml                    # User configuration
```

### Data Directory
```
~/Documents/colossus-legal-data/
├── input/                         # Source documents (.md files)
│   ├── motion_to_dismiss.md
│   ├── complaint.md
│   └── affidavit.md
│
├── extracted/                     # Extracted claims (JSON)
│   ├── 2024-12-09_15-23-45_a1b2c3_qwen2.5-16k_motion_to_dismiss.claims.json
│   ├── 2024-12-09_15-28-12_d4e5f6_qwen2.5-7b_motion_to_dismiss.claims.json
│   └── 2024-12-09_16-00-00_g7h8i9_llama3.1-8b_motion_to_dismiss.claims.json
│
├── prompts/                       # Prompt templates
│   ├── prompt_template.md        # Default prompt
│   ├── affidavit_prompt.md       # Custom for affidavits
│   └── motion_prompt.md          # Custom for motions
│
└── logs/                          # Processing logs (TOML format)
    ├── 2024-12-09_15-23-45_a1b2c3_qwen2.5-16k_motion_to_dismiss.log
    ├── 2024-12-09_15-28-12_d4e5f6_qwen2.5-7b_motion_to_dismiss.log
    └── 2024-12-09_16-00-00_g7h8i9_llama3.1-8b_motion_to_dismiss.log
```

---

## Output Format

### Filename Structure
```
Pattern:
YYYY-MM-DD_HH-MM-SS_<run-id>_<model>_<document-name>.claims.json

Components:
- Timestamp: When processing started
- Run ID: 8-char unique identifier (links to log file)
- Model: Sanitized model name (qwen2.5-16k → qwen2.5-16k)
- Document: Source document name (without extension)

Examples:
2024-12-09_15-23-45_a1b2c3_qwen2.5-16k_motion_to_dismiss.claims.json
2024-12-09_15-28-12_d4e5f6_qwen2.5-7b_motion_to_dismiss.claims.json
```

### JSON Structure
```json
[
  {
    "id": "claim-income-001",
    "quote": "has had no income whatsoever during calendar year 2023",
    "made_by": "defendant",
    "page": 5,
    "topic": "income",
    "severity": 9,
    "source_document": "motion_to_dismiss",
    "date_mentioned": "2023",
    "date_type": "year"
  },
  {
    "id": "claim-service-001",
    "quote": "Service was never properly effectuated",
    "made_by": "defendant",
    "page": 8,
    "topic": "service",
    "severity": 7,
    "source_document": "motion_to_dismiss",
    "date_mentioned": null,
    "date_type": null
  }
]
```

### Field Descriptions

| Field | Type | Description |
|-------|------|-------------|
| `id` | string | Unique claim identifier |
| `quote` | string | Exact text from document |
| `made_by` | string | Who made the claim (defendant/plaintiff/witness/attorney) |
| `page` | int/null | Page number where claim appears |
| `topic` | string | Category: income, property, service, access, financial, employment, residence, other |
| `severity` | int | Rating 1-10 (10=most serious) |
| `source_document` | string | Document name (without extension) |
| `date_mentioned` | string/null | Date referenced in claim ("2023", "7/30/10", etc.) |
| `date_type` | string/null | Type: "year", "full_date", "month_year", "relative", null |

---

## Logging

### Log File Format

**Filename Structure:**
```
YYYY-MM-DD_HH-MM-SS_<run-id>_<model>_<document-name>.log
```

**Format:** TOML (structured, human-readable)

**Location:** `~/Documents/colossus-legal-data/logs/`

### Log File Contents
```toml
[run_info]
run_id = "a1b2c3d4"
run_date = "2024-12-09T15:23:45Z"
document_name = "motion_to_dismiss"

[files]
input_file = "/home/roman/Documents/colossus-legal-data/input/motion_to_dismiss.md"
output_file = "/home/roman/Documents/colossus-legal-data/extracted/2024-12-09_15-23-45_a1b2c3_qwen2.5-16k_motion_to_dismiss.claims.json"
prompt_file = "/home/roman/Documents/colossus-legal-data/prompts/prompt_template.md"
config_file = "/home/roman/.config/colossus-legal/config.toml"

[input_stats]
input_characters = 27059
input_tokens_estimate = 6765

[llm_config]
engine = "ollama"
url = "http://localhost:11434"
model = "qwen2.5-16k"
temperature = 0.1
num_predict = 16000
timeout_seconds = 1800

[timing]
start_time = "2024-12-09T15:23:45Z"
end_time = "2024-12-09T15:33:12Z"
elapsed_seconds = 567
elapsed_formatted = "9m 27s"

[results]
status = "success"        # or "timeout", "failed", "error"
claims_found = 21
output_size_bytes = 3241

[errors]
error_type = ""           # "timeout", "json_parse_error", "ollama_error", etc.
error_message = ""        # Error details if failed
```

### Log Analysis Examples
```bash
# Find all successful runs
grep 'status = "success"' logs/*.log

# Find timeouts
grep 'status = "timeout"' logs/*.log

# Average processing time
grep "elapsed_seconds" logs/*.log | awk -F'= ' '{sum+=$2; count++} END {print sum/count " seconds"}'

# Compare claims found across models
for log in logs/*_motion_to_dismiss.log; do
  echo -n "$(basename $log): "
  grep "claims_found" $log
done

# Find runs that took > 10 minutes
awk '/elapsed_seconds/ {if ($3 > 600) print FILENAME}' logs/*.log
```

---

## Testing & Comparison

### Comparing Different Models
```bash
# Test with different models (same document)
document-processor motion.md --model qwen2.5-16k
document-processor motion.md --model qwen2.5-7b  
document-processor motion.md --model llama3.1-8b

# Results are preserved with unique filenames:
# 2024-12-09_16-00-00_abc123_qwen2.5-16k_motion.claims.json
# 2024-12-09_16-05-00_def456_qwen2.5-7b_motion.claims.json
# 2024-12-09_16-10-00_ghi789_llama3.1-8b_motion.claims.json
```

### Comparing Results
```bash
# Count claims extracted by each model
jq '. | length' extracted/*_motion.claims.json

# Compare processing time
grep "elapsed_formatted" logs/*_motion.log

# Find differences in extracted claims
diff <(jq -r '.[].id' extracted/*_qwen2.5-16k_motion.claims.json | sort) \
     <(jq -r '.[].id' extracted/*_qwen2.5-7b_motion.claims.json | sort)

# Compare severity ratings
jq -r '.[] | "\(.id): \(.severity)"' extracted/*_motion.claims.json
```

### Parameter Tuning

Test with different temperatures:
```bash
# Edit config.toml, change temperature
nano ~/.config/colossus-legal/config.toml

# Run multiple tests
for temp in 0.0 0.1 0.3 0.5; do
  # Update config temp=$temp
  document-processor motion.md
done

# Analyze variance in results
```

---

## Troubleshooting

### Common Issues

#### 1. Config Not Found
```
Error: Config file not found!
Searched:
- /home/roman/.config/colossus-legal/config.toml
```

**Solution:**
```bash
# Check if config exists
ls ~/.config/colossus-legal/config.toml

# If missing, reinstall
cd ~/Projects/colossus-legal/tools/document-processor
./install.sh
```

#### 2. Directory Does Not Exist
```
Error: Input directory does not exist: /home/roman/Documents/colossus-legal-data/input
```

**Solution:**
```bash
mkdir -p ~/Documents/colossus-legal-data/{input,extracted,prompts,logs}
```

#### 3. Timeout Error
```
Error: Failed to call Ollama API
Caused by: operation timed out
```

**Solution:**
```bash
# Check Ollama is running
docker exec colossus-ollama ollama ps

# Increase timeout in config
nano ~/.config/colossus-legal/config.toml
# Change: timeout_seconds = 3600  # 1 hour

# Or use smaller/faster model
document-processor doc.md --model qwen2.5-7b
```

#### 4. JSON Parse Error
```
Error: Failed to parse JSON
Caused by: EOF while parsing a list
```

**Solution:**
```bash
# Increase num_predict in config
nano ~/.config/colossus-legal/config.toml
# Change: num_predict = 20000

# Document may have too many claims - see log for details
cat logs/latest.log
```

#### 5. Model Not Found
```
Error: Ollama returned error: 404
```

**Solution:**
```bash
# List available models
docker exec colossus-ollama ollama list

# Pull missing model
docker exec colossus-ollama ollama pull qwen2.5:14b

# Create 16k context version
docker exec -i colossus-ollama ollama create qwen2.5-16k << 'EOF'
FROM qwen2.5:14b
PARAMETER num_ctx 16384
EOF
```

### Debug Mode
```bash
# Run with verbose output
RUST_LOG=debug document-processor document.md

# Check what Ollama is doing
docker logs colossus-ollama -f

# Monitor GPU usage
watch -n 1 nvidia-smi
```

### Getting Help
```bash
# Show usage
document-processor --help

# Check version
document-processor --version  # (if implemented)

# View config being used
grep -A 100 "\[ollama\]" ~/.config/colossus-legal/config.toml
```

---

## Outstanding Tasks

### High Priority

- [ ] **Add `--commit` flag** - Create Neo4j nodes directly from extracted claims
- [ ] **Implement chunking** - Handle documents > 20k tokens by splitting and processing in chunks
- [ ] **Add progress indicator** - Show progress during long LLM processing (especially for large docs)
- [ ] **Batch processing** - Process multiple documents in one command
```bash
  document-processor --batch input/*.md
```

### Medium Priority

- [ ] **Comparison report generator** - Automated comparison of different model outputs
```bash
  document-processor compare motion.md --models qwen2.5-16k,llama3.1-8b
```
- [ ] **Prompt validation** - Check prompt templates for required placeholders
- [ ] **Config validation** - Warn if directories don't exist before starting long processing
- [ ] **Resume on failure** - Save partial results and resume from checkpoint
- [ ] **Add `--dry-run` flag** - Show what would be processed without actually running
- [ ] **Model benchmarking mode** - Automated testing across multiple models with report
```bash
  document-processor benchmark motion.md --models all
```

### Low Priority

- [ ] **Web UI** - Simple web interface for viewing results/logs
- [ ] **Export to other formats** - CSV, Excel, SQL inserts
- [ ] **Claim deduplication** - Detect and merge duplicate claims across documents
- [ ] **Interactive mode** - Review/edit claims before saving
- [ ] **Plugin system** - Custom extractors for specific document types
- [ ] **Streaming output** - Show claims as they're extracted (for long documents)
- [ ] **Docker containerization** - Package entire tool + Ollama as Docker Compose setup

### Code Quality

- [ ] **Add unit tests** - Test claim parsing, filename generation, config loading
- [ ] **Add integration tests** - End-to-end test with test documents
- [ ] **Error handling improvements** - More specific error types and better messages
- [ ] **Code documentation** - Add rustdoc comments to all public functions
- [ ] **Performance profiling** - Identify bottlenecks for large documents
- [ ] **Memory optimization** - Reduce memory usage for very large documents

### Documentation

- [ ] **Video tutorial** - Walkthrough of installation and basic usage
- [ ] **Prompt engineering guide** - Best practices for creating effective prompts
- [ ] **Model comparison guide** - When to use which model
- [ ] **FAQ document** - Common questions and solutions
- [ ] **API documentation** - If exposing as library/service

---

## Development

### Building from Source
```bash
cd ~/Projects/colossus-legal/tools/document-processor

# Debug build
cargo build

# Release build (optimized)
cargo build --release

# Run without installing
cargo run -- document.md

# Run tests (when added)
cargo test
```

### Contributing

1. Create feature branch: `git checkout -b feature/my-feature`
2. Make changes
3. Test thoroughly
4. Update README with new features
5. Commit and push

### Project Structure
```rust
src/main.rs:
├── Configuration structs (Config, OllamaConfig, etc.)
├── Data structs (Claim, ClaimResponse, ProcessingLog, etc.)
├── main() → run_processing()
├── Config loading (load_config_with_search, load_config)
├── File path resolution (resolve_input_path, etc.)
├── LLM interaction (extract_claims, parse_claims)
├── Logging (write_log_entry, format_elapsed)
└── Utilities (validate_directory, print_usage)
```

---

## Credits

Part of the Colossus Legal project - A system for tracking false claims in legal cases using graph databases and AI.

**Technologies:**
- Rust (for CLI tool)
- Ollama (for local LLM)
- Qwen 2.5 (LLM model)
- Neo4j (graph database - future integration)
- TOML (configuration format)

---

## License

[Specify license]

---

## Changelog

### v1.0.0 (Current)
- Initial release
- Claim extraction with local LLM
- Comprehensive logging
- Unique run IDs
- Date extraction from claims
- Multiple model support
- Professional filename structure

---

## Quick Reference Card
```bash
# Most common commands
document-processor document.md                    # Process with defaults
document-processor doc.md --model qwen2.5-7b     # Use different model
document-processor doc.md -p custom_prompt.md    # Use custom prompt
document-processor --help                         # Show help

# Important paths
~/.cargo/bin/document-processor                   # Binary
~/.config/colossus-legal/config.toml             # Config
~/Documents/colossus-legal-data/input/           # Input files
~/Documents/colossus-legal-data/extracted/       # Output files
~/Documents/colossus-legal-data/logs/            # Log files

# Quick checks
docker exec colossus-ollama ollama ps            # Check Ollama
ls -lh ~/Documents/colossus-legal-data/logs/     # Recent runs
tail -n 20 logs/latest.log                       # Last log
```

---

**Last Updated:** 2024-12-09
