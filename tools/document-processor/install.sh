#!/bin/bash

# Colossus Legal - Document Processor Installation Script
set -e

echo "📦 Installing Colossus Legal Document Processor"
echo "=============================================="

# Get script directory
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Configuration directory (just settings)
CONFIG_DIR="$HOME/.config/colossus-legal"
CONFIG_FILE="$CONFIG_DIR/config.toml"

# Data directory (all user content)
DATA_DIR="$HOME/Documents/colossus-legal-data"
PROMPTS_DIR="$DATA_DIR/prompts"
INPUT_DIR="$DATA_DIR/input"
OUTPUT_DIR="$DATA_DIR/extracted"

# Step 1: Build binary
echo ""
echo "🔨 Building binary..."
cd "$SCRIPT_DIR"
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

# Step 2: Create directory structure
echo ""
echo "📁 Setting up directories..."
mkdir -p "$CONFIG_DIR"
mkdir -p "$PROMPTS_DIR"
mkdir -p "$INPUT_DIR"
mkdir -p "$OUTPUT_DIR"

echo "✅ Created directory structure"

# Step 3: Install config
echo ""
if [ -f "$CONFIG_FILE" ]; then
    echo "⚠️  Config already exists at:"
    echo "   $CONFIG_FILE"
    read -p "   Overwrite with new template? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        cp "$SCRIPT_DIR/config.toml" "$CONFIG_FILE"
        echo "✅ Config updated"
    else
        echo "⏭️  Keeping existing config"
    fi
else
    cp "$SCRIPT_DIR/config.toml" "$CONFIG_FILE"
    echo "✅ Created config at $CONFIG_FILE"
    echo ""
    echo "⚠️  IMPORTANT: Edit config to update paths for your system"
fi

# Step 4: Install example prompts
echo ""
echo "📝 Installing example prompts..."

# Create default prompt template
cat > "$PROMPTS_DIR/prompt_template.md" << 'EOF'
You are analyzing a legal document to extract false claims made by parties.

## Document Information

- **Document Name**: {DOCUMENT_NAME}

## Document Content

{DOCUMENT_TEXT}

## Task

Extract ALL claims made by any party (defendant, plaintiff, witness, attorney).

## What is a "Claim"?

A "claim" is:
- A statement of fact (not a legal argument)
- Something that can be verified or contradicted
- Examples: "I have no income", "Service was never received", "I never had access"

## What is NOT a Claim?

- Legal arguments ("Therefore the motion should be granted")
- Opinions ("The plaintiff is unreasonable")
- Procedural statements ("This motion is filed pursuant to...")

## Output Format

For each claim provide:
- **id**: "claim-[topic]-[number]" (e.g., "claim-income-001")
- **quote**: Exact text from document
- **made_by**: Who made it (defendant/plaintiff/witness/attorney/etc)
- **page**: Page number if mentioned (can be null)
- **topic**: Choose from: income, property, service, access, financial, employment, residence, other
- **severity**: Rate 1-10:
  * 9-10: Sworn statements under penalty of perjury
  * 7-8: Critical factual claims
  * 5-6: Important supporting claims
  * 3-4: Minor claims
  * 1-2: Trivial claims
- **source_document**: Always use "{DOCUMENT_NAME}"

## Response

Return ONLY valid JSON, no markdown, no explanations:
```json
{
  "claims": [
    {
      "id": "claim-income-001",
      "quote": "exact text from document",
      "made_by": "defendant",
      "page": null,
      "topic": "income",
      "severity": 9,
      "source_document": "{DOCUMENT_NAME}"
    }
  ]
}
```
EOF

echo "✅ Created prompt template at:"
echo "   $PROMPTS_DIR/prompt_template.md"

# Copy additional prompts if they exist
if [ -d "$SCRIPT_DIR/prompts" ]; then
    for prompt in "$SCRIPT_DIR/prompts"/*.md; do
        if [ -f "$prompt" ]; then
            filename=$(basename "$prompt")
            if [ "$filename" != "prompt_template.md" ]; then
                cp "$prompt" "$PROMPTS_DIR/"
                echo "✅ Copied: $filename"
            fi
        fi
    done
fi

# Step 5: Install binary
echo ""
echo "🚀 Installing binary..."
cargo install --path "$SCRIPT_DIR" --force

if [ $? -eq 0 ]; then
    echo "✅ Binary installed to ~/.cargo/bin/document-processor"
else
    echo "❌ Binary installation failed!"
    exit 1
fi

# Final summary
echo ""
echo "=============================================="
echo "✨ Installation complete!"
echo ""
echo "📍 Locations:"
echo "   Binary:  ~/.cargo/bin/document-processor"
echo "   Config:  $CONFIG_FILE"
echo "   Data:    $DATA_DIR"
echo ""
echo "📂 Directory structure:"
echo "   $DATA_DIR/"
echo "   ├── input/          (place your .md documents here)"
echo "   ├── extracted/      (JSON output files)"
echo "   └── prompts/        (edit your prompt templates)"
echo ""
echo "⚙️  Next steps:"
echo ""
echo "   1. Edit config (update paths if needed):"
echo "      nano $CONFIG_FILE"
echo ""
echo "   2. Customize prompts:"
echo "      nano $PROMPTS_DIR/prompt_template.md"
echo ""
echo "   3. Place your documents in:"
echo "      $INPUT_DIR/"
echo ""
echo "   4. Run the processor:"
echo "      cd $DATA_DIR"
echo "      document-processor <filename.md>"
echo ""
echo "   5. Check extracted claims:"
echo "      cat $OUTPUT_DIR/<filename.md.claims.json>"
echo ""
echo "📖 Help:"
echo "   document-processor --help"
echo ""