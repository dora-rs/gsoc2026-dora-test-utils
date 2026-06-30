# Week 6 Integration Testing — Design Spec

> Date: 2026-06-30 | Branch: week5 | Target: Week 6 (2026-06-29 ~ 2026-07-05)

## Overview

End-to-end integration tests that run `test-source → echo-node → test-sink` in a real
DORA YAML dataflow via `dora run`, verifying the full pipeline works correctly.

The work is split into two phases:
1. **Phase 1** — Echo node fixture + YAML dataflow template
2. **Phase 2** — Integration test framework with multiple test cases

## Phase 1: Echo Node Fixture

### Files

| File | Purpose |
|------|---------|
| `tests/fixtures/echo-node.rs` | A minimal Rust binary (~30 lines) that receives Input events and sends them back as Output events verbatim |
| `tests/fixtures/echo-dataflow.yml` | YAML dataflow connecting `test-source → echo-node → test-sink` |
| `tests/fixtures/` | Directory for test fixtures (data files, expected outputs) |

### Echo Node Design

```rust
// tests/fixtures/echo-node.rs
use dora_node_api::{DoraNode, Event, MetadataParameters};

fn main() -> eyre::Result<()> {
    let (node, mut events) = DoraNode::init_from_env()?;
    while let Some(event) = events.recv() {
        match event {
            Event::Input { id, data, .. } => {
                node.send_output(id, MetadataParameters::default(), data.0)?;
            }
            Event::Stop(_) | Event::InputClosed { .. } => break,
            _ => {}
        }
    }
    Ok(())
}
```

Key design decisions:
- Echoes the **input ID as the output ID** — this means test-source's `--output-id` must match test-sink's expected input ID
- **No data transformation** — purely a pass-through, making test outcomes deterministic
- Uses `init_from_env()` — supports both daemon mode (via `dora run`) and standalone mode (via `DORA_TEST_WITH_INPUTS`)

### YAML Dataflow Template

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

### Build Configuration

Add a `[[bin]]` entry in `Cargo.toml` for the echo node so `cargo build` compiles it:

```toml
[[bin]]
name = "echo-node"
path = "tests/fixtures/echo-node.rs"
test = false
```

## Phase 2: Integration Test Framework

### Files

| File | Purpose |
|------|---------|
| `tests/integration.rs` | Integration tests using `dora run` |
| `tests/fixtures/source-data.json` | Test data for source (numbers, strings, etc.) |
| `tests/fixtures/expected-output.json` | Expected output matching source data |

### Test Cases

| # | Test Name | Scenario | Assertions |
|---|-----------|----------|------------|
| 1 | `echo_pipeline_exact_match` | source emits `[42, 99, -1]` as Int64 → echo → sink | `SinkResult.r#match == true`, `differences == []`, `expected_count == received_count == 3` |
| 2 | `echo_pipeline_semantic_type_tolerance` | source emits Int32, expected declares Int32 | `match == true` (semantic mode tolerates Int32=Int32) |
| 3 | `echo_pipeline_multiple_elements` | source emits 10 numbers → echo → sink | counts match, all values match |
| 4 | `echo_pipeline_string_data` | source emits `["hello", "world"]` as LargeUtf8 | `match == true` |

### Test Runner Design

Each test follows this pattern:
1. Write temp JSON files (source data, expected output) to a temp directory
2. Write a temp YAML dataflow referencing those files
3. Run `dora run <dataflow.yml> --stop-after 5s`
4. Read `result.json` from the working directory
5. Assert on `SinkResult` fields

```rust
fn run_echo_pipeline_test(
    source_data: serde_json::Value,
    expected_output: serde_json::Value,
    extra_source_args: &[&str],
    extra_sink_args: &[&str],
) -> eyre::Result<SinkResult> {
    // 1. Create temp dir
    // 2. Write source-data.json and expected-output.json
    // 3. Render dataflow YAML with correct paths
    // 4. Run: dora run <yaml> --stop-after 10s
    // 5. Read and parse result.json
    // 6. Return SinkResult
}
```

### Prerequisites / Assumptions

- `dora` CLI must be installed and on PATH
- `dora up` must succeed (coordinator + daemon must be startable)
- Tests run with `--test-threads=1` to avoid port conflicts (port 6013)
- Port 6013 must be free before each test

### Non-Goals (out of scope for Week 6)

- Testing with real sensor/camera data
- Performance/benchmarking tests
- Python echo node variant
- Testing multiple daemons or distributed deployment
- CI integration (will be done in Week 7-8)

## Error Handling

- If `dora` CLI not found → skip tests with a clear message (not a panic)
- If port 6013 is in use → retry with backoff or skip
- If `result.json` is missing → fail with diagnostic (print dora stderr)
- If dataflow times out → fail with `--stop-after` value shown

## Test Data Strategy

All test data is generated in-memory and written to temp files. No committed JSON fixtures
except the echo-node source and YAML template. This keeps tests self-contained and avoids
fixture drift.
