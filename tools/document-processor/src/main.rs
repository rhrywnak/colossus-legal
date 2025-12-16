use anyhow::{bail, Context, Result};
use chrono::Utc;
use std::env;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::time::Instant;

use document_processor::config::{Config, load_config_with_search};
use document_processor::logging::write_log_entry;
use document_processor::claims::Claim;
use document_processor::paths::{
    validate_directory,
    resolve_input_path,
    extract_document_name,
    resolve_output_path,
};
use document_processor::llm::extract_claims;
use document_processor::dates::enrich_claim_dates;

#[tokio::main]
async fn main() -> Result<()> {
    match run_processing().await {
        Ok(_) => {
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("\n❌ Error: {}", e);
            std::process::exit(1);
        }
    }
}

fn dump_claims_json(path: &Path, document_name: &str, claims: &[Claim]) -> Result<()> {
    let file = File::create(path)
        .with_context(|| format!("Failed to create claims dump: {}", path.display()))?;
    let writer = BufWriter::new(file);

    serde_json::to_writer_pretty(
        writer,
        &serde_json::json!({
            "document": document_name,
            "claim_count": claims.len(),
            "claims": claims,
        }),
    )
    .context("Failed to write claims dump JSON")?;

    Ok(())
}

async fn run_processing() -> Result<()> {
    println!("📄 Colossus Legal - Document Processor");
    println!("{}\n", "=".repeat(60));

    let start_time = Instant::now();
    let start_timestamp = Utc::now();

    // Parse command-line arguments
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && (args[1] == "-h" || args[1] == "--help") {
        print_usage(&args[0]);
        std::process::exit(0);
    }

    if args.len() < 2 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    // Parse --config flag if present
    let mut explicit_config: Option<String> = None;
    let mut i = 2;
    while i < args.len() {
        if args[i] == "--config" || args[i] == "-c" {
            if i + 1 < args.len() {
                explicit_config = Some(args[i + 1].clone());
                break;
            }
        }
        i += 1;
    }

    // Load configuration using the shared config module
    let (config, config_path) = load_config_with_search(explicit_config.as_deref())?;
    println!("📋 Config: {}\n", config_path.display());

    // Validate directories
    validate_directory(&config.directories.input_directory, "Input directory")?;
    validate_directory(&config.directories.output_directory, "Output directory")?;
    validate_directory(&config.directories.prompt_directory, "Prompt directory")?;

    // Create logs directory if it doesn't exist
    let log_dir_str = format!(
        "{}/logs",
        Path::new(&config.directories.input_directory)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/home/roman/Documents/colossus-legal-data")
    );
    fs::create_dir_all(&log_dir_str).ok();
    let log_dir = PathBuf::from(&log_dir_str);

    // Parse remaining args
    let input_file = &args[1];
    let mut output_file: Option<String> = None;
    let mut prompt_template: Option<String> = None;
    let mut model: Option<String> = None;
    let mut input_dir_override: Option<String> = None;
    let mut output_dir_override: Option<String> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "-o" | "--output" => {
                if i + 1 < args.len() {
                    output_file = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    bail!("Error: --output requires a filename");
                }
            }
            "-p" | "--prompt" => {
                if i + 1 < args.len() {
                    prompt_template = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    bail!("Error: --prompt requires a filename");
                }
            }
            "--model" => {
                if i + 1 < args.len() {
                    model = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    bail!("Error: --model requires a model name");
                }
            }
            "--input-dir" => {
                if i + 1 < args.len() {
                    input_dir_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    bail!("Error: --input-dir requires a directory path");
                }
            }
            "--output-dir" => {
                if i + 1 < args.len() {
                    output_dir_override = Some(args[i + 1].clone());
                    i += 2;
                } else {
                    bail!("Error: --output-dir requires a directory path");
                }
            }
            "-c" | "--config" => {
                // already handled above
                i += 2;
            }
            _ => {
                bail!("Error: Unknown option: {}", args[i]);
            }
        }
    }

    if let Some(ref dir) = input_dir_override {
        validate_directory(dir, "Input directory (CLI override)")?;
    }
    if let Some(ref dir) = output_dir_override {
        validate_directory(dir, "Output directory (CLI override)")?;
    }

    // Resolve paths
    let input_path = resolve_input_path(
        input_file,
        input_dir_override
            .as_ref()
            .unwrap_or(&config.directories.input_directory),
    )?;

    let document_name = extract_document_name(&input_path)?;
    println!("📄 Document: {}\n", document_name);

    let output_path = resolve_output_path(
        &input_path,
        output_file.as_deref(),
        output_dir_override
            .as_ref()
            .unwrap_or(&config.directories.output_directory),
        &config.defaults.output_suffix,
    )?;

    let prompt_filename = prompt_template
        .as_deref()
        .unwrap_or(&config.defaults.prompt_template);
    let prompt_path = Path::new(&config.directories.prompt_directory).join(prompt_filename);

    if !prompt_path.exists() {
        bail!(
            "Prompt template not found: {}\nLooked in: {}",
            prompt_filename,
            prompt_path.display()
        );
    }

    println!("📋 Prompt: {}", prompt_path.display());
    let prompt_template_text = fs::read_to_string(&prompt_path)
        .with_context(|| format!("Failed to read prompt: {}", prompt_path.display()))?;

    println!("📖 Reading: {}", input_path.display());
    let text = fs::read_to_string(&input_path)
        .with_context(|| format!("Failed to read input: {}", input_path.display()))?;
    println!("✅ Read {} characters\n", text.len());

    // Extract claims via llm module
    let model_name = model.as_deref().unwrap_or(&config.ollama.model);
    println!("🤖 Analyzing with Ollama ({})...", model_name);

    let claims_result: Result<Vec<Claim>> = extract_claims(
        &text,
        &prompt_template_text,
        &document_name,
        &config.ollama.url,
        model_name,
        config.ollama.temperature,
        config.ollama.num_predict,
        config.ollama.timeout_seconds,
    )
    .await;

    match claims_result {
        Ok(mut claims) => {
            let original_count = claims.len();

            // --- PRE-FILTER DUMP ---
            let ts = start_timestamp.format("%Y-%m-%d_%H-%M-%S").to_string();
            let pre_path = log_dir.join(format!("{}_{}_claims_pre_filter.json", ts, document_name));
            dump_claims_json(&pre_path, &document_name, &claims)?;

            // 🔒 Grounding filter: claim must be verifiable by anchors + quote.
            let filtered: Vec<Claim> = claims
                .into_iter()
                .filter(|c| claim_is_grounded_by_anchors(c, &text))
                .collect();

            let filtered_count = filtered.len();
            let dropped = original_count.saturating_sub(filtered_count);

            println!(
                "🔎 Grounding filter: kept {} of {} claims (dropped {}).",
                filtered_count, original_count, dropped
            );

            // --- POST-FILTER DUMP ---
            let post_path = log_dir.join(format!("{}_{}_claims_post_filter.json", ts, document_name));
            dump_claims_json(&post_path, &document_name, &filtered)?;

            let mut claims = filtered;

            println!("✅ Found {} claims\n", claims.len());

            if claims.is_empty() {
                println!("⚠️  No claims found");

                write_log_entry(
                    &start_timestamp,
                    &start_time,
                    &document_name,
                    &input_path,
                    &output_path,
                    &prompt_path,
                    &config_path,
                    &text,
                    &config,
                    model_name,
                    "success",
                    0,
                    0,
                    "no_claims",
                    "No claims were extracted",
                    &log_dir_str,
                )?;

                return Ok(());
            }

            // SECOND PASS: date enrichment
            println!("📅 Enriching claims with date information...");
            if let Err(e) = enrich_claim_dates(&mut claims, &config, model_name).await {
                eprintln!("⚠️ Date enrichment failed: {}", e);
            }

            // Serialize claims to JSON with enriched dates
            let json_output = serde_json::to_string_pretty(&claims)
                .context("Failed to serialize claims to JSON")?;

            fs::write(&output_path, json_output.as_bytes())
                .with_context(|| format!("Failed to write output: {}", output_path.display()))?;

            println!("💾 Output: {}", output_path.display());

            write_log_entry(
                &start_timestamp,
                &start_time,
                &document_name,
                &input_path,
                &output_path,
                &prompt_path,
                &config_path,
                &text,
                &config,
                model_name,
                "success",
                claims.len(),
                json_output.len() as u64,
                "",
                "",
                &log_dir_str,
            )?;

            Ok(())
        }
        Err(e) => {
            let error_message = e.to_string();
            let error_type = if error_message.contains("timeout") {
                "timeout"
            } else if error_message.contains("Failed to parse JSON") {
                "json_parse_error"
            } else if error_message.contains("Ollama returned error") {
                "ollama_error"
            } else {
                "error"
            };

            println!("\n❌ Failed to extract claims: {}\n", error_message);

            let _ = write_log_entry(
                &start_timestamp,
                &start_time,
                &document_name,
                &input_path,
                &output_path,
                &prompt_path,
                &config_path,
                &text,
                &config,
                model_name,
                "error",
                0,
                0,
                error_type,
                &error_message,
                &log_dir_str,
            );

            Err(e)
        }
    }
}

fn print_usage(program_name: &str) {
    eprintln!("Colossus Legal - Document Processor");
    eprintln!("\nUsage: {} <input_file.md> [OPTIONS]", program_name);
    eprintln!("\nArguments:");
    eprintln!("  <input_file.md>         Input markdown file");
    eprintln!("\nOptions:");
    eprintln!("  -c, --config <file>     Config file path");
    eprintln!("  -o, --output <file>     Output JSON file");
    eprintln!("  -p, --prompt <file>     Prompt template filename");
    eprintln!("  --model <name>          Ollama model name");
    eprintln!("  --input-dir <path>      Input directory override");
    eprintln!("  --output-dir <path>     Output directory override");
    eprintln!("  -h, --help              Show this help");
}

/// Normalize by collapsing whitespace and lowercasing (safe for exact-match checks).
fn normalize_ws(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// Verify a claim using: anchor_before + quote + anchor_after as a single exact snippet.
///
/// Requirements:
/// - anchor_before length >= 10 (we expect 20, but allow slight model slop)
/// - anchor_after length >= 10
/// - quote non-empty
/// - normalized(snippet) must appear in normalized(document)
fn claim_is_grounded_by_anchors(claim: &Claim, document_text: &str) -> bool {
    let qb = claim.anchor_before.trim();
    let qa = claim.anchor_after.trim();
    let q = claim.quote.trim();

    if q.is_empty() {
        return false;
    }
    if qb.len() < 10 || qa.len() < 10 {
        return false;
    }

    let snippet = format!("{}{}{}", qb, q, qa);

    let doc_norm = normalize_ws(document_text);
    let snip_norm = normalize_ws(&snippet);

    doc_norm.contains(&snip_norm)
}
