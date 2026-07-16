//! TestSource binary — CLI wrapper around `dora_test_utils::source::run_test_source`.
//!
//! Emits pre-loaded data on one or more configured DORA outputs.
//! Supports both daemon-based dataflows and standalone testing mode.

use clap::Parser;
use dora_test_utils::source::{OutputSpec, SourceConfig};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "test-source", about = "Emit test data on DORA outputs")]
struct Cli {
    /// Output ID (single-output mode, backward compatible).
    #[arg(long)]
    output_id: Option<String>,

    /// Data file (single-output mode, backward compatible).
    #[arg(long)]
    data_file: Option<PathBuf>,

    /// Inline JSON data (single-output mode, backward compatible).
    #[arg(long)]
    inline_data: Option<String>,

    /// Multi-output specs: "output_id:data_file.json". Repeatable.
    #[arg(long = "output", value_name = "ID:FILE")]
    outputs: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    let mut specs: Vec<OutputSpec> = Vec::new();

    // Multi-output mode: --output id:file (repeatable)
    for spec_str in &cli.outputs {
        let (output_id, path_str) = match spec_str.split_once(':') {
            Some(parts) => parts,
            None => {
                eprintln!(
                    "error: invalid --output format '{}'. Expected 'output_id:file.json'",
                    spec_str
                );
                std::process::exit(1);
            }
        };

        let path = PathBuf::from(path_str);
        let contents = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to read data file '{}': {e}", path.display());
                std::process::exit(1);
            }
        };
        let data: serde_json::Value = match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: invalid JSON in data file '{}': {e}", path.display());
                std::process::exit(1);
            }
        };

        specs.push(OutputSpec {
            output_id: output_id.to_string(),
            data,
        });
    }

    // Single-output backward compat: --output-id + (--data-file or --inline-data)
    if specs.is_empty() {
        let output_id = cli.output_id.as_deref().unwrap_or("data");
        let data: serde_json::Value = if let Some(inline) = &cli.inline_data {
            serde_json::from_str(inline).unwrap_or_else(|e| {
                eprintln!("error: invalid inline JSON: {e}");
                std::process::exit(1);
            })
        } else if let Some(data_file) = &cli.data_file {
            let contents = std::fs::read_to_string(data_file).unwrap_or_else(|e| {
                eprintln!(
                    "error: failed to read data file '{}': {e}",
                    data_file.display()
                );
                std::process::exit(1);
            });
            serde_json::from_str(&contents).unwrap_or_else(|e| {
                eprintln!("error: invalid JSON in '{}': {e}", data_file.display());
                std::process::exit(1);
            })
        } else {
            eprintln!("error: --data-file, --inline-data, or --output is required");
            std::process::exit(1);
        };
        specs.push(OutputSpec {
            output_id: output_id.to_string(),
            data,
        });
    }

    let config = SourceConfig { outputs: specs };

    if let Err(e) = dora_test_utils::source::run_test_source(config) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
