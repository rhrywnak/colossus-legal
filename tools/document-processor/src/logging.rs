//! Logging and run metadata structures for the document processor.

use std::fs;
use std::path::Path;
use std::time::Instant;

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::Serialize;
use toml;

use crate::config::Config;

/// Log entry for a processing run
#[derive(Debug, Serialize)]
pub struct ProcessingLog {
    pub run_info: RunInfo,
    pub files: FileInfo,
    pub input_stats: InputStats,
    pub llm_config: LlmConfig,
    pub timing: TimingInfo,
    pub results: ResultInfo,
    pub errors: ErrorInfo,
}

#[derive(Debug, Serialize)]
pub struct RunInfo {
    pub run_date: String,
    pub document_name: String,
}

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub input_file: String,
    pub output_file: String,
    pub prompt_file: String,
    pub config_file: String,
}

#[derive(Debug, Serialize)]
pub struct InputStats {
    pub input_characters: usize,
    pub input_tokens_estimate: usize,
}

#[derive(Debug, Serialize)]
pub struct LlmConfig {
    pub engine: String,
    pub url: String,
    pub model: String,
    pub temperature: f32,
    pub num_predict: u32,
    pub timeout_seconds: u64,
}

#[derive(Debug, Serialize)]
pub struct TimingInfo {
    pub start_time: String,
    pub end_time: String,
    pub elapsed_seconds: u64,
    pub elapsed_formatted: String,
}

#[derive(Debug, Serialize)]
pub struct ResultInfo {
    pub status: String,
    pub claims_found: usize,
    pub output_size_bytes: u64,
}

#[derive(Debug, Serialize)]
pub struct ErrorInfo {
    pub error_type: String,
    pub error_message: String,
}

/// Write a structured TOML log entry for a run.
///
/// This is the same behavior that used to live in `main.rs::write_log_entry`.
#[allow(clippy::too_many_arguments)]
pub fn write_log_entry(
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
    let toml_string = toml::to_string_pretty(&log).context("Failed to serialize log")?;

    // Write to file
    fs::write(&log_filename, toml_string)
        .with_context(|| format!("Failed to write log: {}", log_filename))?;

    println!("📊 Log: {}", log_filename);

    Ok(())
}

/// Format elapsed time into a human-readable string.
fn format_elapsed(seconds: u64) -> String {
    if seconds < 60 {
        format!("{}s", seconds)
    } else {
        let minutes = seconds / 60;
        let secs = seconds % 60;
        format!("{}m {}s", minutes, secs)
    }
}

