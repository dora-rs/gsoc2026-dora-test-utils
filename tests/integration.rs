//! Integration tests for the test-source → echo-node → test-sink pipeline.
//!
//! Each test generates a temporary YAML dataflow, runs it via `dora run`,
//! and asserts on the `SinkResult` written by test-sink.
//!
//! ## Prerequisites
//!
//! - `dora` CLI must be installed and on `$PATH`
//! - Port 6013 must be free
//! - Run with `--test-threads=1` (the `#[serial]` attribute enforces this)

use dora_test_utils::sink::SinkResult;
use serial_test::serial;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Check whether `dora` CLI is available.
fn dora_available() -> bool {
    let dora = dora_binary();
    Command::new(&dora)
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Locate a compiled binary under `target/<profile>/<name>`.
fn bin_path(name: &str) -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".to_string());
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    Path::new(&target_dir).join(profile).join(name)
}

/// Ensure all required binaries are built (cached — only builds once per process).
fn build_binaries() {
    static BUILT: OnceLock<()> = OnceLock::new();
    BUILT.get_or_init(|| {
        let mut args = vec![
            "build",
            "--bin",
            "test_source",
            "--bin",
            "test-sink",
            "--bin",
            "echo-node",
        ];
        if !cfg!(debug_assertions) {
            args.push("--release");
        }
        let status = Command::new("cargo")
            .args(&args)
            .status()
            .expect("cargo build should succeed");
        assert!(status.success(), "cargo build failed");
    });
}

/// Find the dora CLI binary.
fn dora_binary() -> PathBuf {
    // Check the vendored dora workspace build first — try both
    // debug and release profiles (debug is more common in dev).
    for profile in &["debug", "release"] {
        let vendored = Path::new("dora/target").join(profile).join("dora");
        if vendored.exists() {
            return vendored;
        }
    }
    // Fall back to PATH.
    PathBuf::from("dora")
}

/// Run a test-source → echo-node → test-sink pipeline and return the result.
///
/// Generates a temporary YAML dataflow with the given source and expected
/// data, runs it via `dora run --stop-after`, and parses the `SinkResult`
/// from the output file.
fn run_echo_pipeline(
    source_data: &serde_json::Value,
    expected_output: &serde_json::Value,
    source_extra_args: &[&str],
    sink_extra_args: &[&str],
) -> eyre::Result<SinkResult> {
    let tmp = tempfile::TempDir::new()?;

    // ── Write data files ──────────────────────────────────────
    let source_file = tmp.path().join("source.json");
    let expected_file = tmp.path().join("expected.json");
    let output_file = tmp.path().join("result.json");

    std::fs::write(&source_file, serde_json::to_string_pretty(source_data)?)?;
    std::fs::write(
        &expected_file,
        serde_json::to_string_pretty(expected_output)?,
    )?;

    // ── Generate YAML dataflow with absolute paths ────────────
    let source_bin = bin_path("test_source");
    let echo_bin = bin_path("echo-node");
    let sink_bin = bin_path("test-sink");

    let source_extra = if source_extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", source_extra_args.join(" "))
    };

    let sink_extra = if sink_extra_args.is_empty() {
        String::new()
    } else {
        format!(" {}", sink_extra_args.join(" "))
    };

    let yaml = format!(
        r#"nodes:
  - id: test-source
    path: {source_bin}
    args: "--output-id data --data-file {source_file}{source_extra}"
    outputs:
      - data

  - id: echo-node
    path: {echo_bin}
    inputs:
      data: test-source/data
    outputs:
      - data

  - id: test-sink
    path: {sink_bin}
    inputs:
      data: echo-node/data
    args: "--expected-file {expected_file} --output-file {output_file}{sink_extra}"
"#,
        source_bin = source_bin.display(),
        echo_bin = echo_bin.display(),
        sink_bin = sink_bin.display(),
        source_file = source_file.display(),
        expected_file = expected_file.display(),
        output_file = output_file.display(),
        source_extra = source_extra,
        sink_extra = sink_extra,
    );

    let yaml_file = tmp.path().join("dataflow.yml");
    std::fs::write(&yaml_file, yaml)?;

    // ── Run dora run ──────────────────────────────────────────
    let dora = dora_binary();
    let output = Command::new(&dora)
        .args([
            "run",
            yaml_file
                .to_str()
                .expect("temp dir path must be valid UTF-8"),
            "--stop-after",
            "10s",
        ])
        .output()
        .map_err(|e| eyre::eyre!("failed to run dora ({}): {e}", dora.display()))?;

    // ── Try to parse result.json even on dora run failure ─────
    // test-sink writes result.json before exiting, so structured
    // Difference data is available even when fail_on_mismatch
    // causes a non-zero exit.
    let result_json = std::fs::read_to_string(&output_file);
    let sink_result: Option<SinkResult> =
        result_json.ok().and_then(|s| serde_json::from_str(&s).ok());

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Augment the error with SinkResult details when available.
        if let Some(ref sr) = sink_result {
            eyre::bail!(
                "dora run failed with status {}\n\
                 SinkResult: {sr:#?}\n\
                 --- dora stdout ---\n{stdout}\n--- dora stderr ---\n{stderr}",
                output.status
            );
        }
        eyre::bail!(
            "dora run failed with status {}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            output.status
        );
    }

    let result =
        sink_result.ok_or_else(|| eyre::eyre!("result.json missing after successful dora run"))?;

    Ok(result)
}

// ═══════════════════════════════════════════════════════════════
// Test cases
// ═══════════════════════════════════════════════════════════════

#[test]
#[serial]
fn echo_pipeline_exact_match_int64() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;
    }
    build_binaries();

    let source = serde_json::json!({
        "data": [42, 99, -1],
        "data_type": "Int64"
    });
    let expected = serde_json::json!({
        "data": [42, 99, -1],
        "data_type": "Int64"
    });

    let result =
        run_echo_pipeline(&source, &expected, &[], &[]).expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 3);
    assert_eq!(result.received_count, 3);
    assert!(
        result.differences.is_empty(),
        "expected no differences, got {:?}",
        result.differences
    );
}

#[test]
#[serial]
fn echo_pipeline_semantic_int32_tolerates_int64() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;
    }
    build_binaries();

    // Source emits with the default Int64 inference (no data_type hint),
    // but the expected file declares Int32.  Semantic comparison should
    // still match because the numeric values [1, 2, 3] are within Int32
    // range and the cast Int64→Int32 succeeds.
    let source = serde_json::json!({
        "data": [1, 2, 3]
    });
    let expected = serde_json::json!({
        "data": [1, 2, 3],
        "data_type": "Int32"
    });

    let result =
        run_echo_pipeline(&source, &expected, &[], &[]).expect("echo pipeline should succeed");

    assert!(
        result.r#match,
        "semantic mode should tolerate compatible types, got {result:#?}"
    );
    assert_eq!(result.differences.len(), 0);
}

#[test]
#[serial]
fn echo_pipeline_ten_elements() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;
    }
    build_binaries();

    let values: Vec<i64> = (0..10).collect();
    let source = serde_json::json!({
        "data": values,
        "data_type": "Int64"
    });
    let expected = serde_json::json!({
        "data": (0..10).collect::<Vec<i64>>(),
        "data_type": "Int64"
    });

    let result =
        run_echo_pipeline(&source, &expected, &[], &[]).expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 10);
    assert_eq!(result.received_count, 10);
    assert!(result.differences.is_empty());
}

#[test]
#[serial]
fn echo_pipeline_string_data() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;
    }
    build_binaries();

    let source = serde_json::json!({
        "data": ["hello", "world"],
        "data_type": "LargeUtf8"
    });
    let expected = serde_json::json!({
        "data": ["hello", "world"],
        "data_type": "LargeUtf8"
    });

    let result =
        run_echo_pipeline(&source, &expected, &[], &[]).expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 2);
    assert_eq!(result.received_count, 2);
    assert!(result.differences.is_empty());
}
