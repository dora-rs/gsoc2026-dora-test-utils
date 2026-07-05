# Week 6 Integration Testing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build echo-node fixture + YAML dataflow template, then write integration tests that run `test-source → echo-node → test-sink` via `dora run`.

**Architecture:** Phase 1 creates a minimal Rust echo node binary and static YAML dataflow for manual demo. Phase 2 builds an integration test framework that dynamically generates YAML dataflows with absolute paths, runs `dora run`, and asserts on `SinkResult`.

**Tech Stack:** Rust, dora-node-api, serde_json, tempfile, serial_test

## Global Constraints

- All binaries must build with `cargo build` from crate root
- Integration tests require `dora` CLI on PATH; skip gracefully if missing
- Tests run with `--test-threads=1` (enforced via `serial_test` crate) to avoid port 6013 conflicts
- Echo node must be deterministic — same input always produces same output
- Test data generated in-memory, written to temp files; no committed JSON fixtures beyond Phase 1 samples

---

## Phase 1 — Echo Node Fixture

### Task 1: Create tests/fixtures/ directory and echo-node binary

**Files:**
- Create: `tests/fixtures/echo-node.rs`

**Interfaces:**
- Produces: `echo-node` binary — receives any Input event via `DoraNode::init_from_env()`, sends the same data back as Output with the same ID; stops on Stop/InputClosed

- [ ] **Step 1: Create the fixtures directory**

```bash
mkdir -p tests/fixtures
```

- [ ] **Step 2: Write the echo-node binary**

Write `tests/fixtures/echo-node.rs`:

```rust
//! Echo node — receives Input events and sends them back as Output
//! events verbatim.  Used as a pass-through in integration test
//! dataflows so that the pipeline `test-source → echo → test-sink`
//! exercises the full DORA routing machinery.

use dora_node_api::{DoraNode, Event, MetadataParameters};

fn main() -> eyre::Result<()> {
    let (node, mut events) =
        DoraNode::init_from_env().map_err(|e| eyre::eyre!("echo-node: failed to init DORA node: {e}"))?;

    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, .. } => {
                node.send_output(id, MetadataParameters::default(), data.0)
                    .map_err(|e| eyre::eyre!("echo-node: send_output failed: {e}"))?;
            }
            Event::Stop(_) | Event::InputClosed { .. } => break,
            _ => {}
        }
    }

    Ok(())
}
```

- [ ] **Step 3: Add [[bin]] entry to Cargo.toml**

Modify `Cargo.toml` — add after the `[dev-dependencies]` section:

```toml
[[bin]]
name = "echo-node"
path = "tests/fixtures/echo-node.rs"
test = false
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build --bin echo-node 2>&1
```

Expected: `Compiling dora-test-utils v0.1.0` ... `Finished`

- [ ] **Step 5: Commit**

```bash
git add tests/fixtures/echo-node.rs Cargo.toml
git commit -m "feat: add echo-node binary for integration testing

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Create sample data fixtures

**Files:**
- Create: `tests/fixtures/source-data.json`
- Create: `tests/fixtures/expected-output.json`

**Interfaces:**
- Produces: Two JSON files in DORA-format (`{"data": [...], "data_type": "..."}`) — source-data.json for test-source input, expected-output.json for test-sink comparison

- [ ] **Step 1: Write source-data.json**

Write `tests/fixtures/source-data.json`:

```json
{
  "data": [42, 99, -1],
  "data_type": "Int64"
}
```

- [ ] **Step 2: Write expected-output.json**

Write `tests/fixtures/expected-output.json`:

```json
{
  "data": [42, 99, -1],
  "data_type": "Int64"
}
```

- [ ] **Step 3: Commit**

```bash
git add tests/fixtures/source-data.json tests/fixtures/expected-output.json
git commit -m "feat: add sample data fixtures for echo pipeline demo

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Create static YAML dataflow template for manual demo

**Files:**
- Create: `tests/fixtures/echo-dataflow.yml`

**Interfaces:**
- Produces: YAML dataflow that wires `test-source → echo-node → test-sink`, runnable via `dora run tests/fixtures/echo-dataflow.yml --stop-after 5s` from crate root

- [ ] **Step 1: Write echo-dataflow.yml**

Write `tests/fixtures/echo-dataflow.yml`:

```yaml
nodes:
  - id: test-source
    build: cargo build -p dora-test-utils --bin test-source
    path: ../../target/debug/test-source
    args:
      - --output-id
      - data
      - --data-file
      - tests/fixtures/source-data.json
    outputs:
      - data

  - id: echo-node
    build: cargo build --bin echo-node
    path: ../../target/debug/echo-node
    inputs:
      data: test-source/data
    outputs:
      - data

  - id: test-sink
    build: cargo build -p dora-test-utils --bin test-sink
    path: ../../target/debug/test-sink
    inputs:
      data: echo-node/data
    args:
      - --expected-file
      - tests/fixtures/expected-output.json
      - --output-file
      - result.json
```

- [ ] **Step 2: Commit**

```bash
git add tests/fixtures/echo-dataflow.yml
git commit -m "feat: add echo pipeline YAML dataflow template

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

## Phase 2 — Integration Test Framework

### Task 4: Add serial_test dev-dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add serial_test to Cargo.toml**

Modify `Cargo.toml` — in `[dev-dependencies]`, add:

```toml
serial_test = "3"
```

- [ ] **Step 2: Verify dependency resolves**

```bash
cargo check 2>&1
```

Expected: no errors related to serial_test

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "chore: add serial_test dev-dependency for integration tests

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Create integration test module

**Files:**
- Create: `tests/integration.rs`

**Interfaces:**
- Consumes: `echo-node` binary (Task 1), `dora` CLI on PATH, `SinkResult` from `dora_test_utils::sink`
- Produces: 4 integration tests:
  - `echo_pipeline_exact_match` — Int64 numbers pass through exactly
  - `echo_pipeline_semantic_type_tolerance` — Int32 in expected tolerates Int64 received
  - `echo_pipeline_multiple_elements` — 10-element array round-trips correctly
  - `echo_pipeline_string_data` — LargeUtf8 strings pass through exactly

- [ ] **Step 1: Write the import block and helpers**

Write `tests/integration.rs`:

```rust
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
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check whether `dora` CLI is available.
fn dora_available() -> bool {
    Command::new("dora")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Locate a compiled binary under `target/<profile>/<name>`.
fn bin_path(name: &str) -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .unwrap_or_else(|_| "target".to_string());
    let profile = if cfg!(debug_assertions) { "debug" } else { "release" };
    Path::new(&target_dir).join(profile).join(name)
}

/// Ensure all required binaries are built.
fn build_binaries() {
    let status = Command::new("cargo")
        .args(["build", "--bin", "test-source", "--bin", "test-sink", "--bin", "echo-node"])
        .status()
        .expect("cargo build should succeed");
    assert!(status.success(), "cargo build failed");
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
    std::fs::write(&expected_file, serde_json::to_string_pretty(expected_output)?)?;

    // ── Generate YAML dataflow with absolute paths ────────────
    let source_bin = bin_path("test-source");
    let echo_bin = bin_path("echo-node");
    let sink_bin = bin_path("test-sink");

    let yaml = format!(
        r#"nodes:
  - id: test-source
    path: {source_bin}
    args:
      - --output-id
      - data
      - --data-file
      - {source_file}
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
    args:
      - --expected-file
      - {expected_file}
      - --output-file
      - {output_file}
{sink_args}
"#,
        source_bin = source_bin.display(),
        echo_bin = echo_bin.display(),
        sink_bin = sink_bin.display(),
        source_file = source_file.display(),
        expected_file = expected_file.display(),
        output_file = output_file.display(),
        sink_args = sink_extra_args
            .iter()
            .map(|a| format!("      - {a}"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    // Also pass through source extra args
    let yaml = if source_extra_args.is_empty() {
        yaml
    } else {
        // Insert source extra args after --data-file line
        let source_extra = source_extra_args
            .iter()
            .map(|a| format!("      - {a}"))
            .collect::<Vec<_>>()
            .join("\n");
        yaml.replace(
            &format!("      - {source_file}\n    outputs:", source_file = source_file.display()),
            &format!(
                "      - {source_file}\n{source_extra}\n    outputs:",
                source_file = source_file.display(),
                source_extra = source_extra
            ),
        )
    };

    let yaml_file = tmp.path().join("dataflow.yml");
    std::fs::write(&yaml_file, yaml)?;

    // ── Run dora run ──────────────────────────────────────────
    let output = Command::new("dora")
        .args([
            "run",
            yaml_file.to_str().unwrap(),
            "--stop-after",
            "10s",
        ])
        .output()?;

    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        eyre::bail!(
            "dora run failed with status {}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            output.status
        );
    }

    // ── Parse result ──────────────────────────────────────────
    let result_json = std::fs::read_to_string(&output_file)
        .map_err(|e| eyre::eyre!("failed to read result.json at {}: {e}", output_file.display()))?;
    let result: SinkResult = serde_json::from_str(&result_json)
        .map_err(|e| eyre::eyre!("invalid JSON in result.json: {e}\ncontent: {result_json}"))?;

    Ok(result)
}
```

- [ ] **Step 2: Write test 1 — exact match with Int64 numbers**

Append to `tests/integration.rs`:

```rust
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

    let result = run_echo_pipeline(&source, &expected, &[], &[])
        .expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 3);
    assert_eq!(result.received_count, 3);
    assert!(
        result.differences.is_empty(),
        "expected no differences, got {:?}",
        result.differences
    );
}
```

- [ ] **Step 3: Write test 2 — semantic type tolerance (Int32 vs Int64)**

Append to `tests/integration.rs`:

```rust
#[test]
#[serial]
fn echo_pipeline_semantic_int32_tolerates_int64() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;
    }
    build_binaries();

    // Source emits Int32 values, but the default inference produces Int64.
    // Semantic comparison should still match because the numeric values
    // are the same regardless of integer width.
    let source = serde_json::json!({
        "data": [1, 2, 3],
        "data_type": "Int32"
    });
    // Expected also declares Int32 — the sink will convert expected JSON
    // to Int32Array before comparing with the received data.
    let expected = serde_json::json!({
        "data": [1, 2, 3],
        "data_type": "Int32"
    });

    let result = run_echo_pipeline(&source, &expected, &[], &[])
        .expect("echo pipeline should succeed");

    assert!(
        result.r#match,
        "semantic mode should tolerate compatible types, got {result:#?}"
    );
    assert_eq!(result.differences.len(), 0);
}
```

- [ ] **Step 4: Write test 3 — multiple elements (10 numbers)**

Append to `tests/integration.rs`:

```rust
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
        "data": values,
        "data_type": "Int64"
    });

    let result = run_echo_pipeline(&source, &expected, &[], &[])
        .expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 10);
    assert_eq!(result.received_count, 10);
    assert!(result.differences.is_empty());
}
```

- [ ] **Step 5: Write test 4 — string data (LargeUtf8)**

Append to `tests/integration.rs`:

```rust
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

    let result = run_echo_pipeline(&source, &expected, &[], &[])
        .expect("echo pipeline should succeed");

    assert!(result.r#match, "expected match=true, got {result:#?}");
    assert_eq!(result.expected_count, 2);
    assert_eq!(result.received_count, 2);
    assert!(result.differences.is_empty());
}
```

- [ ] **Step 6: Run cargo check on the test file**

```bash
cargo test --test integration --no-run 2>&1
```

Expected: test binary compiles successfully

- [ ] **Step 7: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add echo pipeline integration tests

4 test cases covering exact match, semantic type tolerance,
multi-element arrays, and string data.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Run full QA and verify

- [ ] **Step 1: Run fmt + clippy**

```bash
cargo fmt -- --check 2>&1
cargo clippy -- -D warnings 2>&1
```

Expected: both pass

- [ ] **Step 2: Run existing tests to confirm no regressions**

```bash
cargo test --lib 2>&1
cargo test --test e2e -- --test-threads=1 2>&1
cargo test --test smoke -- --test-threads=1 2>&1
```

Expected: all existing tests pass

- [ ] **Step 3: Run the new integration tests (requires dora CLI)**

```bash
cargo test --test integration -- --test-threads=1 2>&1
```

Expected: tests pass (or skip with clear message if `dora` not installed)

- [ ] **Step 4: Commit if any QA fixes were needed**

```bash
git add -u
git commit -m "chore: QA fixes for Week 6 integration tests

Co-Authored-By: Claude <noreply@anthropic.com>"
```
