use anyhow::{Context, Result, bail};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::fs;
use std::env;
use std::path::{Path, PathBuf};
use std::time::Instant;
use chrono::{DateTime, Utc};

/// Configuration loaded from config.toml
#[derive(Debug, Deserialize)]
struct Config {
    ollama: OllamaConfig,
    directories: DirectoriesConfig,
    defaults: DefaultsConfig,
    neo4j: Neo4jConfig,
}

#[derive(Debug, Deserialize)]
struct OllamaConfig {
    url: String,
    model: String,
    temperature: f32,
    num_predict: u32,
    timeout_seconds: u64,
}

#[derive(Debug, Deserialize)]
struct DirectoriesConfig {
    input_directory: String,
    output_directory: String,
    prompt_directory: String,
}

#[derive(Debug, Deserialize)]
struct DefaultsConfig {
    prompt_template: String,
    output_suffix: String,
}

#[derive(Debug, Deserialize)]
struct Neo4jConfig {
    url: String,
    user: String,
    password: String,
}

/// Represents a single legal claim extracted from a document
#[derive(Debug, Serialize, Deserialize)]
struct Claim {
    id: String,
    quote: String,
    made_by: String,
    page: Option<i32>,
    topic: String,
    severity: i32,
    source_document: String,
}

/// Response structure from LLM
#[derive(Debug, Deserialize)]
struct ClaimResponse {
    claims: Vec<Claim>,
}

/// Log entry for a processing run
#[derive(Debug, Serialize)]
struct ProcessingLog {
    run_info: RunInfo,
    files: FileInfo,
    input_stats: InputStats,
    llm_config: LlmConfig,
    timing: TimingInfo,
    results: ResultInfo,
    errors: ErrorInfo,
}

#[derive(Debug, Serialize)]
struct RunInfo {
    run_date: String,
    document_name: String,
}

#[derive(Debug, Serialize)]
struct FileInfo {
    input_file: String,
    output_file: String,
    prompt_file: String,
    config_file: String,
}

#[derive(Debug, Serialize)]
struct InputStats {
    input_characters: usize,
    input_tokens_estimate: usize,
}

#[derive(Debug, Serialize)]
struct LlmConfig {
    engine: String,
    url: String,
    model: String,
    temperature: f32,
    num_predict: u32,
    timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
struct TimingInfo {
    start_time: String,
    end_time: String,
    elapsed_seconds: u64,
    elapsed_formatted: String,
}

#[derive(Debug, Serialize)]
struct ResultInfo {
    status: String,
    claims_found: usize,
    output_size_bytes: u64,
}

#[derive(Debug, Serialize)]
struct ErrorInfo {
    error_type: String,
    error_message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Run processing and handle result
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

async fn run_processing() -> Result<()> {
    println!("📄 Colossus Legal - Document Processor");
    println!("{}\n", "=".repeat(60));

    // Start timing
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

    // Load configuration
    let (config, config_path) = load_config_with_search(explicit_config.as_deref())?;
    println!("📋 Config: {}\n", config_path.display());

    // Validate directories
    validate_directory(&config.directories.input_directory, "Input directory")?;
    validate_directory(&config.directories.output_directory, "Output directory")?;
    validate_directory(&config.directories.prompt_directory, "Prompt directory")?;

    // Create logs directory if it doesn't exist
    let log_dir = format!("{}/logs", 
        Path::new(&config.directories.input_directory)
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("/home/roman/Documents/colossus-legal-data"));
    fs::create_dir_all(&log_dir).ok();

    // Parse remaining CLI arguments
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
        input_dir_override.as_ref().unwrap_or(&config.directories.input_directory)
    )?;

    let document_name = extract_document_name(&input_path)?;
    println!("📄 Document: {}\n", document_name);

    let output_path = resolve_output_path(
        &input_path,
        output_file.as_deref(),
        output_dir_override.as_ref().unwrap_or(&config.directories.output_directory),
        &config.defaults.output_suffix
    )?;

    let prompt_filename = prompt_template.as_deref().unwrap_or(&config.defaults.prompt_template);
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

    // Extract claims
    let model_name = model.as_deref().unwrap_or(&config.ollama.model);
    println!("🤖 Analyzing with Ollama ({})...", model_name);
    
    let claims_result = extract_claims(
        &text,
        &prompt_template_text,
        &document_name,
        &config.ollama.url,
        model_name,
        config.ollama.temperature,
        config.ollama.num_predict,
        config.ollama.timeout_seconds
    ).await;

    // Handle result and create log
    match claims_result {
        Ok(claims) => {
            println!("✅ Found {} claims\n", claims.len());

            if claims.is_empty() {
                println!("⚠️  No claims found");
                
                // Write log for empty result
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
                    "",
                    "",
                    &log_dir,
                )?;
                
                return Ok(());
            }

            // Save to JSON
            let json_output = serde_json::to_string_pretty(&claims)
                .context("Failed to serialize claims")?;
            
            fs::write(&output_path, json_output)
                .with_context(|| format!("Failed to write: {}", output_path.display()))?;

            // Get output file size
            let output_size = fs::metadata(&output_path)
                .map(|m| m.len())
                .unwrap_or(0);

            // Write success log
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
                output_size,
                "",
                "",
                &log_dir,
            )?;

            // Summary
            println!("{}", "=".repeat(60));
            println!("✨ Complete!");
            println!("\n📊 Summary:");
            println!("   Document: {}", document_name);
            println!("   Claims: {}", claims.len());
            println!("   Output: {}", output_path.display());
            println!("   Elapsed: {}", format_elapsed(start_time.elapsed().as_secs()));

            Ok(())
        }
        Err(e) => {
            // Determine error type
            let error_message = format!("{}", e);
            let (error_type, status) = if error_message.contains("timeout") {
                ("timeout", "timeout")
            } else if error_message.contains("Failed to parse JSON") {
                ("json_parse_error", "failed")
            } else if error_message.contains("Ollama") {
                ("ollama_error", "failed")
            } else {
                ("unknown_error", "error")
            };

            // Write error log
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
                status,
                0,
                0,
                error_type,
                &error_message,
                &log_dir,
            )?;

            Err(e)
        }
    }
}

/// Write log entry
#[allow(clippy::too_many_arguments)]
fn write_log_entry(
    start_timestamp: &DateTime<Utc>,
    start_time: &Instant,
    document_name: &str,
    input_path: &Path,
    output_path: &Path,
    prompt_path: &Path,
    config_path: &Path,
    text: &str,
    config: &Config,
    model_name: &str,
    status: &str,
    claims_found: usize,
    output_size: u64,
    error_type: &str,
    error_message: &str,
    log_dir: &str,
) -> Result<()> {
    let elapsed = start_time.elapsed();
    let elapsed_secs = elapsed.as_secs();

    let log = ProcessingLog {
        run_info: RunInfo {
            run_date: start_timestamp.to_rfc3339(),
            document_name: document_name.to_string(),
        },
        files: FileInfo {
            input_file: input_path.display().to_string(),
            output_file: output_path.display().to_string(),
            prompt_file: prompt_path.display().to_string(),
            config_file: config_path.display().to_string(),
        },
        input_stats: InputStats {
            input_characters: text.len(),
            input_tokens_estimate: text.len() / 4,
        },
        llm_config: LlmConfig {
            engine: "ollama".to_string(),
            url: config.ollama.url.clone(),
            model: model_name.to_string(),
            temperature: config.ollama.temperature,
            num_predict: config.ollama.num_predict,
            timeout_seconds: config.ollama.timeout_seconds,
        },
        timing: TimingInfo {
            start_time: start_timestamp.to_rfc3339(),
            end_time: Utc::now().to_rfc3339(),
            elapsed_seconds: elapsed_secs,
            elapsed_formatted: format_elapsed(elapsed_secs),
        },
        results: ResultInfo {
            status: status.to_string(),
            claims_found,
            output_size_bytes: output_size,
        },
        errors: ErrorInfo {
            error_type: error_type.to_string(),
            error_message: error_message.to_string(),
        },
    };

    // Generate log filename
    let log_filename = format!(
        "{}/{}_{}.log",
        log_dir,
        start_timestamp.format("%Y-%m-%d_%H-%M-%S"),
        document_name
    );

    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&log)
        .context("Failed to serialize log")?;

    // Write to file
    fs::write(&log_filename, toml_string)
        .with_context(|| format!("Failed to write log: {}", log_filename))?;

    println!("📊 Log: {}", log_filename);

    Ok(())
}

/// Format elapsed time
fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    }
}

/// Load configuration using standard search order
fn load_config_with_search(explicit_path: Option<&str>) -> Result<(Config, PathBuf)> {
    let config_path = if let Some(path) = explicit_path {
        let p = PathBuf::from(path);
        if !p.exists() {
            bail!("Config file not found: {}", p.display());
        }
        p
    } else if let Ok(path) = env::var("COLOSSUS_CONFIG") {
        let p = PathBuf::from(path);
        if !p.exists() {
            bail!("Config file not found (from COLOSSUS_CONFIG): {}", p.display());
        }
        p
    } else {
        let home = env::var("HOME")
            .context("HOME environment variable not set")?;
        let user_config = Path::new(&home).join(".config/colossus-legal/config.toml");
        
        if user_config.exists() {
            user_config
        } else {
            let system_config = PathBuf::from("/etc/colossus-legal/config.toml");
            if system_config.exists() {
                system_config
            } else {
                bail!(
                    "Config file not found!\n\n\
                    Searched:\n\
                    - {}\n\
                    - /etc/colossus-legal/config.toml\n\n\
                    Create config at: ~/.config/colossus-legal/config.toml",
                    user_config.display()
                );
            }
        }
    };

    let config = load_config(&config_path)?;
    Ok((config, config_path))
}

fn load_config(path: &Path) -> Result<Config> {
    let config_text = fs::read_to_string(path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    
    let config: Config = toml::from_str(&config_text)
        .with_context(|| format!("Failed to parse config: {}", path.display()))?;
    
    Ok(config)
}

fn validate_directory(path: &str, name: &str) -> Result<()> {
    let dir_path = Path::new(path);
    
    if !dir_path.exists() {
        bail!(
            "{} does not exist: {}\nPlease create the directory",
            name,
            path
        );
    }
    
    if !dir_path.is_dir() {
        bail!("{} is not a directory: {}", name, path);
    }
    
    Ok(())
}

fn resolve_input_path(filename: &str, input_dir: &str) -> Result<PathBuf> {
    let path = Path::new(filename);
    
    if path.is_absolute() || filename.contains('/') || filename.contains('\\') {
        if !path.exists() {
            bail!("Input file does not exist: {}", filename);
        }
        Ok(path.to_path_buf())
    } else {
        let full_path = Path::new(input_dir).join(filename);
        if !full_path.exists() {
            bail!(
                "Input file not found: {}\nLooked in: {}",
                filename,
                full_path.display()
            );
        }
        Ok(full_path)
    }
}

fn extract_document_name(path: &Path) -> Result<String> {
    let filename = path
        .file_stem()
        .and_then(|n| n.to_str())
        .context("Invalid input filename")?;
    
    Ok(filename.to_string())
}

fn resolve_output_path(
    input_path: &Path,
    explicit_output: Option<&str>,
    output_dir: &str,
    suffix: &str
) -> Result<PathBuf> {
    if let Some(output) = explicit_output {
        Ok(PathBuf::from(output))
    } else {
        let input_filename = input_path
            .file_name()
            .and_then(|n| n.to_str())
            .context("Invalid input filename")?;
        
        let output_filename = format!("{}{}", input_filename, suffix);
        Ok(Path::new(output_dir).join(output_filename))
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

async fn extract_claims(
    text: &str,
    prompt_template: &str,
    document_name: &str,
    ollama_url: &str,
    model: &str,
    temperature: f32,
    num_predict: u32,
    timeout_seconds: u64
) -> Result<Vec<Claim>> {
    let client = Client::new();

    let prompt = prompt_template
        .replace("{DOCUMENT_TEXT}", text)
        .replace("{DOCUMENT_NAME}", document_name);

    let response = client
        .post(format!("{}/api/generate", ollama_url))
        .json(&json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "format": "json",
            "options": {
                "temperature": temperature,
                "num_predict": num_predict,
            }
        }))
        .timeout(std::time::Duration::from_secs(timeout_seconds))
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

    parse_claims(response_text, document_name)
}

fn parse_claims(response_text: &str, document_name: &str) -> Result<Vec<Claim>> {
    let cleaned = response_text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json_start = cleaned.find('{').unwrap_or(0);
    let json_end = cleaned.rfind('}').map(|i| i + 1).unwrap_or(cleaned.len());
    let json_str = &cleaned[json_start..json_end];

    let mut response: ClaimResponse = serde_json::from_str(json_str)
        .with_context(|| format!("Failed to parse JSON. First 500 chars:\n{}", 
            &json_str.chars().take(500).collect::<String>()))?;

    for claim in &mut response.claims {
        claim.source_document = document_name.to_string();
    }

    Ok(response.claims)
}