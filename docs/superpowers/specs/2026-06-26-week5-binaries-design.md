# Week 5 Design: TestSourceNode + TestSinkNode Binaries & Code Review Follow-up

**Date:** 2026-06-26 | **Status:** Approved | **Branch:** week4→week5

---

## Overview

Implement two standalone CLI binaries (`test_source`, `test_sink`) as reusable
DORA nodes for integration testing dataflows, plus address Week 4 code review
findings on the `IntoInputData` trait and `NodeHarness`.

Both binaries use `DoraNode::init_from_env()` — the standard DORA pattern that
auto-detects daemon mode (via env vars set by `dora run`) or testing mode (via
`DORA_TEST_WITH_INPUTS`). Core logic lives in library functions under `src/`
for programmatic reuse; binaries are thin CLI wrappers.

---

## File Structure

```
src/
├── lib.rs          # + pub mod source; pub mod sink;
├── harness.rs      # (unchanged — code review fixes only)
├── traits.rs       # (unchanged — code review fixes only)
├── source.rs       # NEW: TestSource core logic
├── sink.rs         # NEW: TestSink core logic
└── mock/           # (unchanged)

src/bin/
├── test_source.rs  # NEW: CLI parsing → source::run()
└── test_sink.rs    # NEW: CLI parsing → sink::run()
```

**New dependency in `Cargo.toml`:**
```toml
clap = { version = "4", features = ["derive"] }
```

No other new dependencies — `arrow`, `arrow-json`, `serde_json`, and
`dora-node-api` are already present. Arrow array comparison uses the
built-in `PartialEq` on `dyn Array`.

---

## 1. TestSourceNode

### CLI

```
dora-test-utils test-source \
    --output-id <ID>          # Output identifier (required)
    --data-file <PATH>        # Arrow JSON data file (mutually exclusive with --inline-data)
    --inline-data <JSON>      # Inline JSON data (mutually exclusive with --data-file)
```

### Behavior

1. Parse CLI args via clap derive
2. Load data: read file and parse JSON, or parse inline JSON directly
3. **JSON → Arrow conversion:** Use `data_type` to determine the target Arrow
   type; fall back to type inference from JSON values (integers → Int64,
   floats → Float64, strings → Utf8). Convert the `data` payload into an
   Arrow array via `arrow_json::ReaderBuilder`.
4. Call `DoraNode::init_from_env()` — auto-detects daemon or testing mode
5. If the resulting Arrow array has multiple columns or batches, emit each
   row as a separate `send_output` call. If single-row, emit once.
6. Exit (success)

### Data Format (DORA integration testing format)

File or inline JSON must conform to DORA format:

```json
{
  "data": [1, 2, 3, 4, 5],
  "data_type": {"DataType": "Int32", "bitWidth": 32}
}
```

- The `data` field holds the payload — either a single JSON value or an array.
  When an array, each element is emitted as a separate output message.
- `data_type` is optional metadata describing the Arrow type.

### Error Handling

| Scenario | Behavior |
|----------|----------|
| File not found | `eprintln!` + exit code 1 |
| Invalid JSON | `eprintln!` + exit code 1 |
| Missing `data` field | `eprintln!` + exit code 1 |
| `send_output` failure | `eprintln!` + exit code 1 |
| Missing `--output-id` | clap auto-error + help |

### Library Signature (`src/source.rs`)

```rust
pub struct SourceConfig {
    pub output_id: String,
    pub data: serde_json::Value,
}

pub fn run_test_source(config: SourceConfig) -> eyre::Result<()>;
```

---

## 2. TestSinkNode

### CLI

```
dora-test-utils test-sink \
    --expected-file <PATH>      # Expected output file — Arrow JSON (required)
    --fail-on-mismatch          # Exit non-zero on mismatch (default: true)
    --output-file <PATH>        # Result file path (default: ./result.json)
    --strict                    # Use exact string comparison instead of semantic Arrow comparison
```

### Behavior

1. Parse CLI args via clap derive
2. Load expected data from `--expected-file`
3. Call `DoraNode::init_from_env()` — auto-detects daemon or testing mode
4. Event loop: accumulate `Event::Input { data, .. }` into `Vec<ArrowData>`
5. Break on `Event::Stop` or `Event::InputClosed`
6. Compare received data against expected:
   - **Semantic (default):** Parse expected JSON and received data into Arrow
     arrays, compare column-by-column using `dyn Array::PartialEq`
   - **Strict (`--strict`):** Serialize received Arrow data back to JSON,
     compare with expected file via `serde_json::Value` exact equality
7. Write comparison result to `result.json`
8. Exit: `0` on match, `1` on mismatch, `2` on setup error

### `result.json` Format

**Match:**
```json
{
  "match": true,
  "expected_count": 3,
  "received_count": 3,
  "differences": []
}
```

**Mismatch:**
```json
{
  "match": false,
  "expected_count": 3,
  "received_count": 2,
  "differences": [
    {
      "index": null,
      "message": "count mismatch: expected 3 but got 2"
    },
    {
      "index": 0,
      "message": "value mismatch at column 'data' row 0: expected Int32(42), got Int32(99)"
    }
  ]
}
```

### Error Handling

| Scenario | Behavior |
|----------|----------|
| Expected file not found | `eprintln!` + exit code 2 |
| Expected file invalid JSON | `eprintln!` + exit code 2 |
| Stop before any input received | Compare empty vec against expected → write result → exit 1 |
| `init_from_env` failure | `eprintln!` + exit code 2 |

Exit codes: `0` = match, `1` = mismatch, `2` = setup error

### Library Signature (`src/sink.rs`)

```rust
pub struct SinkConfig {
    pub expected_file: PathBuf,
    pub output_file: PathBuf,
    pub fail_on_mismatch: bool,
    pub strict: bool,
}

pub struct SinkResult {
    pub r#match: bool,
    pub expected_count: usize,
    pub received_count: usize,
    pub differences: Vec<Difference>,
}

pub struct Difference {
    pub index: Option<usize>,
    pub message: String,
}

pub fn run_test_sink(config: SinkConfig) -> eyre::Result<SinkResult>;
```

---

## 3. JSON ↔ Arrow Conversion Strategy

Both binaries need to convert between DORA-format JSON and Arrow arrays.

### JSON → Arrow (TestSource emission, TestSink expected-data loading)

```
DORA JSON: {"data": [1,2,3], "data_type": {"DataType": "Int32", "bitWidth": 32}}
                │
                ▼
          arrow_json::ReaderBuilder
                │
                ▼
          arrow::array::ArrayData / RecordBatch
                │
                ▼
          node.send_output()  [TestSource]
          comparison target   [TestSink]
```

Use `arrow_json::ReaderBuilder` with the schema inferred from `data_type`
(when present) or auto-inferred from JSON values. This is the standard
Arrow JSON integration path already used by `arrow-json`.

### Arrow → JSON (TestSink strict mode, result.json)

```
Event::Input { data: ArrowData }
        │
        ▼
  arrow_json::Writer (JsonArray format)
        │
        ▼
  serde_json::Value  →  compare with expected (strict mode) or serialize to result.json
```

### Error cases

| Scenario | Handling |
|----------|----------|
| `data_type` missing | Infer from JSON values (int→Int64, float→Float64, string→Utf8, bool→Boolean) |
| `data_type` incompatible with `data` values | Return error — "data_type Int32 but found string value" |
| Empty `data` array | Return error — "data array is empty, nothing to emit" |

---

## 4. Week 4 Code Review Follow-up

Six findings from the Week 4 max-effort code review, addressed here:

### P0 — Guard against empty `ArrayData` in `IntoInputData`

**File:** `src/traits.rs`, `impl IntoInputData for ArrayData`

Add a check at the top of `into_input_data()`:
```rust
assert!(
    self.len() > 0,
    "IntoInputData: empty ArrayData is not supported — \
     empty data causes the daemon thread to deadlock in tick()"
);
```

### P1 — Fix `clippy::len_zero` (4 locations)

Replace `assert!(x.len() > 0, ...)` with `assert!(!x.is_empty(), ...)`:
- `src/harness.rs:365` — `test_send_data_json`
- `src/harness.rs:384` — `test_send_data_arrow`
- `tests/e2e.rs:31` — `e2e_receive_input_and_stop`
- `tests/e2e.rs:191` — `e2e_send_data_arrow_input`

### P1 — Replace `from_str` with `from_slice`

**File:** `src/traits.rs:52`

`serde_json::from_str(&json_str)` → `serde_json::from_slice(&buf)`.
`buf` is known-valid UTF-8 from `arrow_json::Writer`, so the extra
UTF-8 validation in `from_str` is wasteful.

### P2 — Remove redundant `drop(writer)`

**File:** `src/traits.rs:50`

Remove `drop(writer);` — `writer.finish()?` already flushes all data.

### P2 — Cross-reference in `send_data` docs

**File:** `src/harness.rs`, `send_data()` doc comment

Add:
```
/// After calling this method, the input channel is still open. To safely
/// call [`send_output`](Self::send_output) afterward, you must first
/// close the input channel via [`close_input`](Self::close_input) or
/// [`run_to_completion`](Self::run_to_completion).
```

### P3 — Strengthen `should_panic` substring

**File:** `src/harness.rs:391`

Change `#[should_panic(expected = "input channel closed")]` to
`#[should_panic(expected = "NodeHarness: input channel closed")]`
for a more precise match.

---

## 5. Testing Strategy

### TestSourceNode tests

| Test | Description |
|------|-------------|
| `test_source_from_file` | Load data from a temp file, verify `send_output` called |
| `test_source_from_inline` | Parse `--inline-data`, verify output emitted |
| `test_source_invalid_json` | Malformed JSON → error exit |
| `test_source_missing_data_field` | JSON without `data` field → error exit |

Tests use `DoraNode::init_testing()` with `TestingOutput::ToChannel` so
outputs can be captured and asserted.

### TestSinkNode tests

| Test | Description |
|------|-------------|
| `test_sink_exact_match` | Received == expected → match=true, exit 0 |
| `test_sink_count_mismatch` | Received fewer than expected → match=false |
| `test_sink_value_mismatch` | Different values → match=false with diff details |
| `test_sink_strict_mode` | `--strict` flag → exact JSON comparison |
| `test_sink_empty_input` | Stop before any input → match=false |

Tests inject inputs via `TestingInput::Input(IntegrationTestInput)` then
run the sink logic and assert on `SinkResult`.

### Code review fix tests

| Test | Description |
|------|-------------|
| `test_empty_arraydata_panics` | Verify `into_input_data()` panics on empty ArrayData |
| Existing tests | All 21 existing tests must continue passing |

---

## 6. Implementation Order

1. **Code review fixes** (P0–P3) — unblock clean CI baseline
2. **`src/source.rs`** — `run_test_source()` + unit tests
3. **`src/bin/test_source.rs`** — CLI wrapper
4. **`src/sink.rs`** — `run_test_sink()` + unit tests
5. **`src/bin/test_sink.rs`** — CLI wrapper
6. **`src/lib.rs`** — add `pub mod source; pub mod sink;`
7. **`Cargo.toml`** — add `clap`
8. **CI pass** — `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`
