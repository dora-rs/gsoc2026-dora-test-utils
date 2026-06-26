//! TestSink binary — CLI wrapper around `dora_test_utils::sink::run_test_sink`.
//!
//! Receives inputs from a DORA dataflow, compares them against expected
//! data from a file, and writes the result to `result.json`.

use std::path::PathBuf;

use clap::Parser;
use dora_test_utils::sink::{run_test_sink, SinkConfig};

#[derive(Parser, Debug)]
#[command(name = "test-sink", about = "Capture DORA outputs and compare with expected data")]
struct Cli {
    /// Path to the expected output file (DORA JSON format).
    #[arg(long)]
    expected_file: PathBuf,

    /// Path to write comparison result (default: ./result.json).
    #[arg(long, default_value = "./result.json")]
    output_file: PathBuf,

    /// Exit non-zero on mismatch (default: true).
    #[arg(long, default_value = "true")]
    fail_on_mismatch: bool,

    /// Use exact JSON string comparison instead of Arrow semantic comparison.
    #[arg(long, default_value = "false")]
    strict: bool,
}

fn main() {
    let cli = Cli::parse();

    let config = SinkConfig {
        expected_file: cli.expected_file,
        output_file: cli.output_file,
        fail_on_mismatch: cli.fail_on_mismatch,
        strict: cli.strict,
    };

    match run_test_sink(config) {
        Ok(result) => {
            if !result.r#match && cli.fail_on_mismatch {
                eprintln!(
                    "mismatch: {} differences found (expected {} items, got {})",
                    result.differences.len(),
                    result.expected_count,
                    result.received_count,
                );
                for diff in &result.differences {
                    eprintln!("  - {}", diff.message);
                }
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("error: {e:#}");
            std::process::exit(2);
        }
    }
}
