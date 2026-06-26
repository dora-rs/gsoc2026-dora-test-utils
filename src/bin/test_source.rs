//! TestSource binary — CLI wrapper around `dora_test_utils::source::run_test_source`.
//!
//! Emits pre-loaded data on a configured DORA output, then exits.
//! Supports both daemon-based dataflows and standalone testing mode.

use clap::Parser;
use dora_test_utils::source::{run_test_source, SourceConfig};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "test-source", about = "Emit test data on a DORA output")]
struct Cli {
    /// Output identifier to emit data on.
    #[arg(long)]
    output_id: String,

    /// Arrow JSON data file (DORA format: {"data": [...], "data_type": {...}}).
    #[arg(long, group = "data_source")]
    data_file: Option<PathBuf>,

    /// Inline JSON data (DORA format).
    #[arg(long, group = "data_source")]
    inline_data: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let data: serde_json::Value = if let Some(path) = &cli.data_file {
        let contents = match std::fs::read_to_string(path) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("error: failed to read data file '{}': {e}", path.display());
                std::process::exit(1);
            }
        };
        match serde_json::from_str(&contents) {
            Ok(v) => v,
            Err(e) => {
                eprintln!(
                    "error: invalid JSON in data file '{}': {e}",
                    path.display()
                );
                std::process::exit(1);
            }
        }
    } else if let Some(inline) = &cli.inline_data {
        match serde_json::from_str(inline) {
            Ok(v) => v,
            Err(e) => {
                eprintln!("error: invalid inline JSON: {e}");
                std::process::exit(1);
            }
        }
    } else {
        eprintln!("error: one of --data-file or --inline-data is required");
        std::process::exit(1);
    };

    let config = SourceConfig {
        output_id: cli.output_id,
        data,
    };

    if let Err(e) = run_test_source(config) {
        eprintln!("error: {e:#}");
        std::process::exit(1);
    }
}
