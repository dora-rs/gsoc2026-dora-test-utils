# Week 5 Binaries Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement TestSourceNode + TestSinkNode CLI binaries, library functions, and Week 4 code review fixes.

**Architecture:** Core logic in `src/source.rs` and `src/sink.rs` as library functions; thin CLIs in `src/bin/test_source.rs` and `src/bin/test_sink.rs` using clap derive. Both binaries use `DoraNode::init_from_env()` for daemon/testing mode auto-detection. JSON ↔ Arrow conversion via a shared helper.

**Tech Stack:** Rust, clap 4 (derive), arrow 58, arrow-json 58, serde_json, dora-node-api, eyre

## Global Constraints

- Use `DoraNode::init_from_env()` for daemon/testing auto-detection
- DORA data format: `{"data": [...], "data_type": {...}}` (data_type optional)
- Exit codes: source=1 on error; sink=0 match, 1 mismatch, 2 setup error
- Semantic Arrow comparison as default; `--strict` for exact JSON comparison
- All 21 existing tests must continue passing
- CI gates: `cargo fmt --all -- --check`, `cargo clippy -- -D warnings`, `cargo test`

---

### Task 1: Code Review Fixes (P0–P3)

**Files:**
- Modify: `src/traits.rs:27,50,52`
- Modify: `src/harness.rs:365,384,391`
- Modify: `tests/e2e.rs:31,191`

**Interfaces:**
- Produces: Clean CI baseline with no clippy warnings and 21+1=22 passing tests

- [ ] **Step 1: P0 — Guard against empty ArrayData in IntoInputData**

Add panic guard at top of `into_input_data()` for `ArrayData`:

Open `src/traits.rs`, in `impl IntoInputData for arrow::array::ArrayData`, add as the first line of `fn into_input_data(self) -> InputData`:

```rust
fn into_input_data(self) -> InputData {
    assert!(
        self.len() > 0,
        "IntoInputData: empty ArrayData is not supported — \
         empty data causes the daemon thread to deadlock in tick()"
    );
    // ... rest of existing code
```

- [ ] **Step 2: P1 — Fix clippy::len_zero in harness.rs (2 locations)**

In `src/harness.rs`, change `assert!(data.0.len() > 0, ...)` to `assert!(!data.0.is_empty(), ...)`:

Line 365 (`test_send_data_json`):
```rust
assert!(!data.0.is_empty(), "data should be non-empty");
```

Line 384 (`test_send_data_arrow`):
```rust
assert!(!data.0.is_empty(), "data should be non-empty");
```

- [ ] **Step 3: P1 — Fix clippy::len_zero in e2e.rs (2 locations)**

In `tests/e2e.rs`, change `assert!(data.0.len() > 0, ...)` to `assert!(!data.0.is_empty(), ...)`:

Line 31 (`e2e_receive_input_and_stop`):
```rust
assert!(!data.0.is_empty(), "input data should be non-empty");
```

Line 191 (`e2e_send_data_arrow_input`):
```rust
assert!(!data.0.is_empty(), "data should be non-empty");
```

- [ ] **Step 4: P1 — from_str → from_slice in traits.rs**

In `src/traits.rs`, replace lines 52-53:
```rust
let json_str =
    String::from_utf8(buf).expect("IntoInputData: Arrow JSON output is valid UTF-8");

// Parse JSON. The output is a JSON array of row objects;
// DORA's JSON->Arrow converter handles this correctly.
let value: serde_json::Value = serde_json::from_str(&json_str)
    .expect("IntoInputData: Arrow JSON output is valid JSON");
```

With:
```rust
// Parse JSON directly from the buffer (known-valid UTF-8, skip re-validation).
// The output is a JSON array of row objects;
// DORA's JSON->Arrow converter handles this correctly.
let value: serde_json::Value = serde_json::from_slice(&buf)
    .expect("IntoInputData: Arrow JSON output is valid JSON");
```

This eliminates `json_str` and `String::from_utf8(buf)` — the `from_slice` method handles UTF-8 validation internally.

- [ ] **Step 5: P2 — Remove redundant drop(writer) in traits.rs**

In `src/traits.rs`, remove line 50:
```rust
drop(writer);
```

(The blank line after `writer.finish()...` should be preserved for readability.)

- [ ] **Step 6: P2 — Add cross-reference in send_data docs**

In `src/harness.rs`, after the `# Panics` section of `send_data()` (after line ~170), add before `# Example`:

```rust
/// After calling this method, the input channel is still open. To safely
/// call [`send_output`](Self::send_output) afterward, you must first
/// close the input channel via [`close_input`](Self::close_input) or
/// [`run_to_completion`](Self::run_to_completion).
```

- [ ] **Step 7: P3 — Strengthen should_panic substring**

In `src/harness.rs`, change line 391-392 from:
```rust
#[should_panic(expected = "input channel closed")]
```
To:
```rust
#[should_panic(expected = "NodeHarness: input channel closed")]
```

- [ ] **Step 8: Add test for empty ArrayData panic**

Add to the `#[cfg(test)] mod tests` block in `src/traits.rs`, after the last test:

```rust
#[test]
#[should_panic(expected = "empty ArrayData is not supported")]
fn test_into_input_data_empty_arraydata_panics() {
    use arrow::array::{Array, Int32Array};
    let arr = Int32Array::from(Vec::<i32>::new());
    let _ = arr.into_data().into_input_data();
}
```

- [ ] **Step 9: Run all tests to verify fixes**

```bash
cargo test
```

Expected: 22/22 tests passing (21 existing + 1 new empty-ArrayData-panic test)

- [ ] **Step 10: Run clippy to verify len_zero fixes**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings

- [ ] **Step 11: Commit**

```bash
git add src/traits.rs src/harness.rs tests/e2e.rs
git commit -m "fix: address Week 4 code review findings (P0–P3)

- P0: guard empty ArrayData in IntoInputData with panic
- P1: fix clippy::len_zero in harness.rs and e2e.rs (4 locations)
- P1: replace from_str with from_slice in traits.rs
- P2: remove redundant drop(writer) in traits.rs
- P2: add close_input cross-reference in send_data docs
- P3: strengthen should_panic substring match

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Add clap Dependency + Module Scaffolding

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/lib.rs`

**Interfaces:**
- Produces: clap available for binaries; `pub mod source; pub mod sink;` declared

- [ ] **Step 1: Add clap to Cargo.toml**

Add under `[dependencies]`:
```toml
# CLI argument parsing for test binaries.
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 2: Add eyre and serde dependencies**

Add under `[dependencies]` (needed by source/sink library functions):
```toml
# Error handling for test binaries.
eyre = "0.6"
# Serialization for SinkResult output.
serde = { version = "1", features = ["derive"] }
```

- [ ] **Step 3: Declare source and sink modules in lib.rs**

Add after `pub mod traits;` in `src/lib.rs`:
```rust
pub mod source;
pub mod sink;
```

- [ ] **Step 4: Verify compilation**

```bash
cargo check
```

Expected: Warning about unused modules (source.rs and sink.rs don't exist yet), but no errors

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/lib.rs
git commit -m "chore: add clap + eyre deps, declare source/sink modules

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Implement `src/source.rs` — TestSource Core Logic

**Files:**
- Create: `src/source.rs`

**Interfaces:**
- Produces: `pub struct SourceConfig { pub output_id: String, pub data: serde_json::Value }`
- Produces: `pub fn run_test_source(config: SourceConfig) -> eyre::Result<()>`

- [ ] **Step 1: Write unit tests (RED)**

Create `src/source.rs` with only tests:

```rust
//! TestSource — programmatic source node for injecting test data into DORA dataflows.
//!
//! The [`run_test_source`] function creates a DORA node via
//! [`DoraNode::init_from_env`] and emits pre-loaded data on a
//! configured output.  Designed for both daemon-based dataflows
//! and standalone testing mode (`DORA_TEST_WITH_INPUTS` env var).

use dora_node_api::{DoraNode, MetadataParameters};
use eyre::{Context, Result};

/// Configuration for a test source run.
#[derive(Debug, Clone)]
pub struct SourceConfig {
    /// Output identifier to emit data on.
    pub output_id: String,
    /// DORA-format JSON payload: `{"data": [...], "data_type": {...}}`.
    pub data: serde_json::Value,
}

/// Run a test source: create a DORA node and emit loaded data.
///
/// # Errors
///
/// Returns an error if:
/// - The `data` JSON is missing the `"data"` field
/// - The `data` array is empty
/// - `DoraNode::init_from_env()` fails
/// - `send_output()` fails
pub fn run_test_source(config: SourceConfig) -> Result<()> {
    // ── 1. Validate and extract data ──────────────────────────────
    let data_array = config
        .data
        .get("data")
        .ok_or_else(|| eyre::eyre!("missing 'data' field in DORA-format input JSON"))?;

    let elements = data_array.as_array().ok_or_else(|| {
        eyre::eyre!("'data' field must be a JSON array, got: {}", data_array)
    })?;

    if elements.is_empty() {
        eyre::bail!("'data' array is empty — nothing to emit");
    }

    // ── 2. Convert each JSON element to an Arrow array ────────────
    let arrays: Vec<_> = elements
        .iter()
        .map(json_value_to_arrow_array)
        .collect::<Result<Vec<_>>>()?;

    // ── 3. Initialize DORA node ───────────────────────────────────
    let (mut node, _events) =
        DoraNode::init_from_env().context("failed to initialize DORA node")?;

    let output_id = config
        .output_id
        .parse()
        .map_err(|e| eyre::eyre!("invalid output_id '{}': {e}", config.output_id))?;

    // ── 4. Emit each array as a separate output message ───────────
    for array in arrays {
        node.send_output(output_id.clone(), MetadataParameters::default(), array)
            .context("send_output failed")?;
    }

    Ok(())
}

/// Convert a single JSON value to an Arrow array.
///
/// Infers the Arrow type from the JSON value:
/// - JSON number (integer) → Int64Array
/// - JSON number (float) → Float64Array
/// - JSON string → StringArray
/// - JSON bool → BooleanArray
/// - JSON array → wraps in a single-column StructArray via arrow_json
fn json_value_to_arrow_array(value: &serde_json::Value) -> Result<arrow::array::ArrayRef> {
    use arrow::array::{Array, BooleanArray, Float64Array, Int64Array, StringArray};
    use std::sync::Arc;

    match value {
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(Arc::new(Int64Array::from(vec![i])))
            } else if let Some(f) = n.as_f64() {
                Ok(Arc::new(Float64Array::from(vec![f])))
            } else {
                eyre::bail!("unsupported number value: {n}")
            }
        }
        serde_json::Value::String(s) => Ok(Arc::new(StringArray::from(vec![s.as_str()]))),
        serde_json::Value::Bool(b) => Ok(Arc::new(BooleanArray::from(vec![*b]))),
        serde_json::Value::Array(arr) => {
            // Nested array — wrap in a single-column struct via arrow_json
            json_array_to_arrow_struct(arr)
        }
        serde_json::Value::Object(_) => {
            // Object — wrap in a single-row struct via arrow_json
            json_obj_to_arrow_struct(value)
        }
        serde_json::Value::Null => {
            eyre::bail!("null values are not supported as standalone output")
        }
    }
}

/// Convert a JSON object to a single-row Arrow StructArray.
fn json_obj_to_arrow_struct(obj: &serde_json::Value) -> Result<arrow::array::ArrayRef> {
    use arrow::array::RecordBatch;
    use arrow::datatypes::{Field, Schema};
    use arrow_json::ReaderBuilder;
    use std::io::BufReader;
    use std::sync::Arc;

    let json_bytes = serde_json::to_vec(&vec![obj])?;
    let reader = BufReader::new(&json_bytes[..]);

    // Use empty schema for auto-inference
    let schema = Arc::new(Schema::empty());
    let mut json_reader = ReaderBuilder::new(schema).build(reader).map_err(|e| {
        eyre::eyre!("failed to build arrow_json reader: {e}")
    })?;

    let mut batches = Vec::new();
    while let Some(result) = json_reader.next() {
        let batch: RecordBatch = result.map_err(|e| eyre::eyre!("arrow_json read error: {e}"))?;
        batches.push(batch);
    }

    if batches.is_empty() {
        eyre::bail!("arrow_json produced no batches from object value");
    }

    // Merge all batches into one and extract the first column
    let merged = arrow::compute::concat_batches(&batches[0].schema(), &batches)
        .map_err(|e| eyre::eyre!("failed to concat batches: {e}"))?;

    if merged.num_columns() == 0 {
        eyre::bail!("arrow_json produced zero columns");
    }

    Ok(merged.column(0).clone())
}

/// Convert a JSON array to a single-column Arrow StructArray.
fn json_array_to_arrow_struct(arr: &[serde_json::Value]) -> Result<arrow::array::ArrayRef> {
    // Wrap each element in {"data": <element>} so arrow_json can parse it
    let wrapped: Vec<serde_json::Value> = arr
        .iter()
        .map(|v| serde_json::json!({"data": v}))
        .collect();

    json_obj_to_arrow_struct(&serde_json::Value::Array(wrapped))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a minimal SourceConfig for testing.
    fn source_config(data: serde_json::Value) -> SourceConfig {
        SourceConfig {
            output_id: "test_out".to_string(),
            data,
        }
    }

    #[test]
    fn test_missing_data_field() {
        let config = source_config(serde_json::json!({"not_data": [1, 2]}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("missing 'data' field"),
            "error should mention missing 'data' field"
        );
    }

    #[test]
    fn test_empty_data_array() {
        let config = source_config(serde_json::json!({"data": []}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("empty"),
            "error should mention empty array"
        );
    }

    #[test]
    fn test_data_not_array() {
        let config = source_config(serde_json::json!({"data": 42}));
        let result = run_test_source(config);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("must be a JSON array"),
            "error should mention must be array"
        );
    }

    #[test]
    fn test_json_to_arrow_int64() {
        let arr = json_value_to_arrow_array(&serde_json::json!(42)).unwrap();
        assert_eq!(arr.len(), 1);
        let int_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::Int64Array>()
            .expect("should be Int64Array");
        assert_eq!(int_arr.value(0), 42);
    }

    #[test]
    fn test_json_to_arrow_float64() {
        let arr = json_value_to_arrow_array(&serde_json::json!(3.14)).unwrap();
        assert_eq!(arr.len(), 1);
        let float_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::Float64Array>()
            .expect("should be Float64Array");
        assert!((float_arr.value(0) - 3.14).abs() < 0.001);
    }

    #[test]
    fn test_json_to_arrow_string() {
        let arr = json_value_to_arrow_array(&serde_json::json!("hello")).unwrap();
        assert_eq!(arr.len(), 1);
        let str_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::StringArray>()
            .expect("should be StringArray");
        assert_eq!(str_arr.value(0), "hello");
    }

    #[test]
    fn test_json_to_arrow_bool() {
        let arr = json_value_to_arrow_array(&serde_json::json!(true)).unwrap();
        assert_eq!(arr.len(), 1);
        let bool_arr = arr
            .as_any()
            .downcast_ref::<arrow::array::BooleanArray>()
            .expect("should be BooleanArray");
        assert!(bool_arr.value(0));
    }

    #[test]
    fn test_json_to_arrow_null_panics() {
        let result = json_value_to_arrow_array(&serde_json::Value::Null);
        assert!(result.is_err());
        assert!(
            format!("{}", result.unwrap_err()).contains("null"),
            "error should mention null"
        );
    }
}
```

- [ ] **Step 2: Run tests — they should FAIL (no impl yet)**

```bash
cargo test -p dora-test-utils --lib source
```

Expected: Tests fail — `run_test_source` and helpers not implemented yet. (Actually, since we wrote the implementation inline, this should pass. But `using arrow::compute::concat_batches` and `arrow_json::ReaderBuilder` need the imports. Tests will fail at compile time if missing.)

- [ ] **Step 3: Fix compilation, run tests**

```bash
cargo test -p dora-test-utils --lib source
```

Expected: All source tests pass

- [ ] **Step 4: Commit**

```bash
git add src/source.rs
git commit -m "feat: add TestSource library function (src/source.rs)

- SourceConfig + run_test_source() with DORA-format JSON support
- JSON→Arrow type inference (int→Int64, float→Float64, string→String, bool→Boolean)
- Error handling: missing data field, empty array, init/env failures
- 8 unit tests for validation and type inference

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Implement `src/bin/test_source.rs` — CLI Wrapper

**Files:**
- Create: `src/bin/test_source.rs`

**Interfaces:**
- Consumes: `dora_test_utils::source::{SourceConfig, run_test_source}`
- Produces: `test_source` binary with clap CLI

- [ ] **Step 1: Write the binary**

```rust
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
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

Expected: Compiles cleanly. Note that we may see a warning about unused `test_sink` binary if we haven't created it yet — that's fine.

- [ ] **Step 3: Commit**

```bash
git add src/bin/test_source.rs
git commit -m "feat: add test-source CLI binary

- clap derive CLI with --output-id, --data-file, --inline-data
- Error handling: file-not-found, invalid-JSON, missing-data-source
- Delegates to source::run_test_source()

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Implement `src/sink.rs` — TestSink Core Logic

**Files:**
- Create: `src/sink.rs`

**Interfaces:**
- Produces: `pub struct SinkConfig`, `pub struct SinkResult`, `pub struct Difference`
- Produces: `pub fn run_test_sink(config: SinkConfig) -> eyre::Result<SinkResult>`

- [ ] **Step 1: Write the implementation with inline tests (RED → GREEN)**

Create `src/sink.rs`:

```rust
//! TestSink — programmatic sink node for capturing and asserting DORA outputs.
//!
//! The [`run_test_sink`] function creates a DORA node via
//! [`DoraNode::init_from_env`], accumulates all incoming [`Event::Input`]
//! events, and compares them against expected data loaded from a file.

use std::path::PathBuf;

use dora_node_api::{DoraNode, Event};
use eyre::Context;
use serde::{Deserialize, Serialize};

/// Configuration for a test sink run.
#[derive(Debug, Clone)]
pub struct SinkConfig {
    /// Path to the expected output file (DORA JSON format).
    pub expected_file: PathBuf,
    /// Path to write the comparison result to.
    pub output_file: PathBuf,
    /// If true, exit with non-zero on mismatch.
    pub fail_on_mismatch: bool,
    /// If true, use exact JSON string comparison instead of Arrow semantic comparison.
    pub strict: bool,
}

/// Result of a sink comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkResult {
    /// Whether the received data matched the expected data.
    pub r#match: bool,
    /// Number of expected data items.
    pub expected_count: usize,
    /// Number of received data items.
    pub received_count: usize,
    /// List of differences found.
    pub differences: Vec<Difference>,
}

/// A single difference between expected and received data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    /// Index of the differing value, or None for structural mismatches.
    pub index: Option<usize>,
    /// Human-readable description of the difference.
    pub message: String,
}

/// Run a test sink: receive inputs, compare with expected, write result.
///
/// # Errors
///
/// Returns an error if:
/// - The expected file cannot be read or parsed
/// - `DoraNode::init_from_env()` fails
pub fn run_test_sink(config: SinkConfig) -> eyre::Result<SinkResult> {
    // ── 1. Load expected data ────────────────────────────────────
    let expected_json: serde_json::Value = {
        let contents = std::fs::read_to_string(&config.expected_file)
            .with_context(|| format!("failed to read expected file '{}'", config.expected_file.display()))?;
        serde_json::from_str(&contents)
            .with_context(|| format!("invalid JSON in expected file '{}'", config.expected_file.display()))?
    };

    let expected_data = expected_json
        .get("data")
        .ok_or_else(|| eyre::eyre!("expected file missing 'data' field"))?;

    let expected_elements: Vec<&serde_json::Value> = if let Some(arr) = expected_data.as_array() {
        arr.iter().collect()
    } else {
        vec![expected_data]
    };

    // ── 2. Initialize DORA node ──────────────────────────────────
    let (_node, mut events) =
        DoraNode::init_from_env().context("failed to initialize DORA node")?;

    // ── 3. Accumulate input events ────────────────────────────────
    let mut received: Vec<arrow::array::ArrayData> = Vec::new();
    while let Some(event) = events.recv() {
        match event {
            Event::Input { data, .. } => {
                received.push(data);
            }
            Event::Stop(_) | Event::InputClosed { .. } => break,
            _ => {}
        }
    }

    // ── 4. Compare ────────────────────────────────────────────────
    let result = if config.strict {
        compare_strict(&expected_elements, &received)
    } else {
        compare_semantic(&expected_elements, &received)
    };

    // ── 5. Write result ──────────────────────────────────────────
    let result_json = serde_json::to_string_pretty(&result)?;
    std::fs::write(&config.output_file, result_json)
        .with_context(|| format!("failed to write result to '{}'", config.output_file.display()))?;

    Ok(result)
}

/// Strict comparison: serialize received Arrow data back to JSON, compare with serde_json::Value equality.
fn compare_strict(
    expected: &[&serde_json::Value],
    received: &[arrow::array::ArrayData],
) -> SinkResult {
    use arrow::array::RecordBatch;
    use arrow::datatypes::{Field, Schema};
    use arrow_json::writer::{JsonArray, Writer};
    use std::sync::Arc;

    let mut differences = Vec::new();

    // Serialize received data to JSON
    let received_json: Vec<serde_json::Value> = received
        .iter()
        .map(|data| {
            let array = arrow::array::make_array(data.clone());
            let schema = Schema::new(vec![Field::new("data", data.data_type().clone(), true)]);
            let batch =
                RecordBatch::try_new(Arc::new(schema), vec![array]).expect("valid batch");

            let mut buf = Vec::new();
            let mut writer = Writer::<_, JsonArray>::new(&mut buf);
            writer.write(&batch).expect("write should succeed");
            writer.finish().expect("finish should succeed");

            let json_str = String::from_utf8(buf).expect("valid utf-8");
            serde_json::from_str::<serde_json::Value>(&json_str).unwrap_or(serde_json::Value::Null)
        })
        .collect();

    // Flatten JSON array output into individual elements for comparison
    let received_flat: Vec<&serde_json::Value> = received_json
        .iter()
        .flat_map(|v| {
            if let Some(arr) = v.as_array() {
                arr.iter().collect::<Vec<_>>()
            } else {
                vec![v]
            }
        })
        .collect();

    if received_flat.len() != expected.len() {
        differences.push(Difference {
            index: None,
            message: format!(
                "count mismatch: expected {} but got {}",
                expected.len(),
                received_flat.len()
            ),
        });
    }

    let max_len = expected.len().max(received_flat.len());
    for i in 0..max_len {
        let exp = expected.get(i);
        let rec = received_flat.get(i);
        match (exp, rec) {
            (Some(e), Some(r)) if e == r => {} // match
            (Some(e), Some(r)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("value mismatch at index {i}: expected {e}, got {r}"),
                });
            }
            (Some(_), None) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("missing received value at index {i}"),
                });
            }
            (None, Some(_)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("unexpected extra value at index {i}"),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    SinkResult {
        r#match: differences.is_empty(),
        expected_count: expected.len(),
        received_count: received_flat.len(),
        differences,
    }
}

/// Semantic comparison: parse expected JSON into Arrow arrays, compare with received Arrow data.
fn compare_semantic(
    expected: &[&serde_json::Value],
    received: &[arrow::array::ArrayData],
) -> SinkResult {
    let mut differences = Vec::new();

    // Convert expected JSON values to Arrow arrays
    let expected_arrays: Vec<arrow::array::ArrayRef> = expected
        .iter()
        .filter_map(|v| {
            // Use the same conversion logic as source (inline for simplicity)
            match v {
                serde_json::Value::Number(n) => {
                    if let Some(i) = n.as_i64() {
                        Some(std::sync::Arc::new(arrow::array::Int64Array::from(vec![i])) as arrow::array::ArrayRef)
                    } else if let Some(f) = n.as_f64() {
                        Some(std::sync::Arc::new(arrow::array::Float64Array::from(vec![f])) as arrow::array::ArrayRef)
                    } else {
                        None
                    }
                }
                serde_json::Value::String(s) => {
                    Some(std::sync::Arc::new(arrow::array::StringArray::from(vec![s.as_str()])) as arrow::array::ArrayRef)
                }
                serde_json::Value::Bool(b) => {
                    Some(std::sync::Arc::new(arrow::array::BooleanArray::from(vec![*b])) as arrow::array::ArrayRef)
                }
                _ => None,
            }
        })
        .collect();

    // Convert received ArrowData to ArrayRef
    let received_arrays: Vec<arrow::array::ArrayRef> = received
        .iter()
        .map(|data| arrow::array::make_array(data.clone()))
        .collect();

    if received_arrays.len() != expected_arrays.len() {
        differences.push(Difference {
            index: None,
            message: format!(
                "count mismatch: expected {} but got {}",
                expected_arrays.len(),
                received_arrays.len()
            ),
        });
    }

    let max_len = expected_arrays.len().max(received_arrays.len());
    for i in 0..max_len {
        let exp = expected_arrays.get(i);
        let rec = received_arrays.get(i);
        match (exp, rec) {
            (Some(e), Some(r)) if e == r => {} // match
            (Some(e), Some(r)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!(
                        "value mismatch at index {i}: expected {e:?}, got {r:?}"
                    ),
                });
            }
            (Some(_), None) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("missing received value at index {i}"),
                });
            }
            (None, Some(_)) => {
                differences.push(Difference {
                    index: Some(i),
                    message: format!("unexpected extra value at index {i}"),
                });
            }
            (None, None) => unreachable!(),
        }
    }

    SinkResult {
        r#match: differences.is_empty(),
        expected_count: expected_arrays.len(),
        received_count: received_arrays.len(),
        differences,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    /// Helper: write expected data to a temp file and return the path.
    fn write_expected_file(data: &serde_json::Value) -> tempfile::NamedTempFile {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        write!(file, "{}", serde_json::to_string(data).unwrap()).unwrap();
        file
    }

    /// Helper: create a SinkConfig pointing at a temp expected file.
    fn sink_config(expected: &serde_json::Value) -> (SinkConfig, tempfile::NamedTempFile) {
        let file = write_expected_file(expected);
        let output = tempfile::NamedTempFile::new().unwrap();
        let config = SinkConfig {
            expected_file: file.path().to_path_buf(),
            output_file: output.path().to_path_buf(),
            fail_on_mismatch: true,
            strict: false,
        };
        (config, file)
    }

    #[test]
    fn test_compare_semantic_exact_match() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(42), &serde_json::json!(99)];
        // Create equivalent Arrow arrays
        let received: Vec<_> = vec![
            arrow::array::Int64Array::from(vec![42]).into_data(),
            arrow::array::Int64Array::from(vec![99]).into_data(),
        ];
        let result = compare_semantic(&expected, &received);
        assert!(result.r#match);
        assert_eq!(result.expected_count, 2);
        assert_eq!(result.received_count, 2);
        assert!(result.differences.is_empty());
    }

    #[test]
    fn test_compare_semantic_count_mismatch() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(1), &serde_json::json!(2)];
        let received: Vec<_> = vec![arrow::array::Int64Array::from(vec![1]).into_data()];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.expected_count, 2);
        assert_eq!(result.received_count, 1);
        assert!(!result.differences.is_empty());
    }

    #[test]
    fn test_compare_semantic_value_mismatch() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(42)];
        let received: Vec<_> = vec![arrow::array::Int64Array::from(vec![99]).into_data()];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.differences.len(), 1);
        assert_eq!(result.differences[0].index, Some(0));
    }

    #[test]
    fn test_compare_semantic_empty_input() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(1)];
        let received: Vec<arrow::array::ArrayData> = vec![];
        let result = compare_semantic(&expected, &received);
        assert!(!result.r#match);
        assert_eq!(result.received_count, 0);
        assert_eq!(result.expected_count, 1);
    }

    #[test]
    fn test_compare_strict_match() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(42)];
        let received: Vec<_> = vec![arrow::array::Int64Array::from(vec![42]).into_data()];
        let result = compare_strict(&expected, &received);
        assert!(result.r#match);
    }

    #[test]
    fn test_compare_strict_mismatch() {
        let expected: Vec<&serde_json::Value> = vec![&serde_json::json!(42)];
        let received: Vec<_> = vec![arrow::array::Int64Array::from(vec![99]).into_data()];
        let result = compare_strict(&expected, &received);
        assert!(!result.r#match);
    }
}
```

- [ ] **Step 2: Add tempfile dev-dependency**

Add to `[dev-dependencies]` in `Cargo.toml`:
```toml
tempfile = "3"
```

- [ ] **Step 3: Run tests**

```bash
cargo test -p dora-test-utils --lib sink
```

Expected: All 6 sink tests pass

- [ ] **Step 4: Commit**

```bash
git add src/sink.rs Cargo.toml
git commit -m "feat: add TestSink library function (src/sink.rs)

- SinkConfig, SinkResult, Difference types
- run_test_sink() — receive inputs, compare, write result.json
- Semantic comparison via Arrow PartialEq
- Strict comparison via JSON round-trip equality
- 6 unit tests covering match, mismatch, count error, empty input

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Implement `src/bin/test_sink.rs` — CLI Wrapper

**Files:**
- Create: `src/bin/test_sink.rs`

**Interfaces:**
- Consumes: `dora_test_utils::sink::{run_test_sink, SinkConfig}`

- [ ] **Step 1: Write the binary**

```rust
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
```

- [ ] **Step 2: Verify compilation**

```bash
cargo check
```

Expected: All code compiles cleanly.

- [ ] **Step 3: Commit**

```bash
git add src/bin/test_sink.rs
git commit -m "feat: add test-sink CLI binary

- clap derive CLI: --expected-file, --output-file, --fail-on-mismatch, --strict
- Exit codes: 0=match, 1=mismatch, 2=setup error
- Delegates to sink::run_test_sink()

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Final Verification — All Gates

**Files:**
- (all files from previous tasks)

**Interfaces:**
- None (final validation)

- [ ] **Step 1: Run cargo fmt**

```bash
cargo fmt --all -- --check
```

Expected: No formatting issues.

Fix any formatting issues with `cargo fmt --all` and re-check.

- [ ] **Step 2: Run cargo clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings.

If warnings, fix them. Common issues:
- `unused imports` in source/sink if tests don't use everything
- Add `#![allow(dead_code)]` to binary files if needed

- [ ] **Step 3: Run all tests**

```bash
cargo test
```

Expected: All tests pass (21 existing + 1 empty-ArrayData-panic + 8 source + 6 sink = 36 total, plus any additional mock tests)

- [ ] **Step 4: Commit final adjustments**

```bash
git add -A
git commit -m "chore: final formatting and clippy fixes for Week 5

Co-Authored-By: Claude <noreply@anthropic.com>"
```

- [ ] **Step 5: Print test summary**

```bash
cargo test -- --list 2>&1 | tail -5
cargo test 2>&1 | grep "test result"
```

Expected output shows all tests passing.
