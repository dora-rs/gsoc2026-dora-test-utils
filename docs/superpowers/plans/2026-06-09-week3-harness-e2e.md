# Week 3: NodeHarness E2E — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `send_output` to `NodeHarness` and write an end-to-end test proving the full `send_input → tick → send_output → recv_output` pipeline.

**Architecture:** `NodeHarness::send_output` delegates to `DoraNode::send_output` (the real thing). Outputs are captured by `TestingOutput::ToChannel` (already wired) and collected by `recv_output`.

**Tech Stack:** Rust, arrow 58, flume 0.10, dora-node-api (local path)

---

### Task 1: Add `send_output` method to `NodeHarness`

**Files:**
- Modify: `src/harness.rs:140-165` (after `send_stop`, before `tick`)

- [ ] **Step 1: Add the method**

```rust
/// Send an output from the node under test.
///
/// Delegates to the underlying [`DoraNode::send_output`].  The output is
/// captured by [`TestingOutput::ToChannel`] and can be retrieved via
/// [`recv_output`](Self::recv_output).
///
/// # Errors
///
/// Returns a [`NodeError`] if the underlying `send_output` call fails.
pub fn send_output(
    &mut self,
    output_id: &str,
    data: impl arrow::array::Array,
) -> Result<(), NodeError> {
    self.node.send_output(
        output_id.parse().unwrap(),
        Default::default(),
        data,
    )
}
```

- [ ] **Step 2: Check compilation**

```bash
cargo check 2>&1
```
Expected: compiles with no errors.

- [ ] **Step 3: Run existing tests**

```bash
cargo test 2>&1
```
Expected: all 9 existing tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src/harness.rs
git commit -m "feat: add NodeHarness::send_output delegating to DoraNode

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 2: Write end-to-end test

**Files:**
- Create: `tests/e2e.rs`

- [ ] **Step 1: Create the test file**

```rust
//! End-to-end test for the dora-test-utils harness.
//!
//! Exercises the full pipeline: send_input → tick → send_output → recv_output.

use arrow::array::{Array, Int32Array};
use dora_node_api::integration_testing::integration_testing_format::{
    IncomingEvent, InputData, TimedIncomingEvent,
};
use dora_test_utils::NodeHarness;

/// Echo node: receives an Input with integer data, sends it back as output.
///
/// Pipeline:
/// 1. Create harness
/// 2. Inject Input event with JSON integer data
/// 3. Tick — receive the event
/// 4. Extract data from event, send as output via harness.send_output
/// 5. Assert recv_output returns the output
#[test]
fn e2e_echo_node() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // ── Send Input event with integer data ────────────────────────
    let input_data = serde_json::json!([1, 2, 3]);
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "numbers".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: input_data,
                data_type: None,
            })),
        },
    });

    // ── Tick: receive the event ───────────────────────────────────
    let event = harness
        .tick()
        .expect("tick should return an event");

    // Extract the Arrow data from the input event
    let received_data = match event {
        dora_node_api::Event::Input { data, .. } => data,
        other => panic!("expected Input event, got {other:?}"),
    };

    // ── Send output (echo the data back) ──────────────────────────
    // Convert the Arrow ArrayData to an Int32Array for re-sending
    let array = Int32Array::from(received_data);
    harness
        .send_output("echo", array)
        .expect("send_output should succeed");

    // ── Assert output was captured ────────────────────────────────
    let outputs = harness.recv_output("echo");
    assert!(
        outputs.is_some(),
        "recv_output should return Some for 'echo'"
    );
    let outputs = outputs.unwrap();
    assert_eq!(
        outputs.len(),
        1,
        "should have exactly one output for 'echo'"
    );

    let output = &outputs[0];
    assert_eq!(
        output.get("id").and_then(|v| v.as_str()),
        Some("echo"),
        "output id should be 'echo'"
    );
    assert!(
        output.contains_key("data"),
        "output should contain 'data' field"
    );

    // ── Clean shutdown ────────────────────────────────────────────
    harness.send_stop();
    harness.tick();
}
```

- [ ] **Step 2: Run the test**

```bash
cargo test e2e_echo_node -- --nocapture 2>&1
```
Expected: PASS

- [ ] **Step 3: Run full test suite**

```bash
cargo fmt -- --check 2>&1 && cargo clippy -- -D warnings 2>&1 && cargo test 2>&1
```
Expected: fmt OK, clippy OK, all 10 tests pass (6 unit + 3 smoke + 1 e2e)

- [ ] **Step 4: Commit**

```bash
git add tests/e2e.rs
git commit -m "test: add end-to-end echo node test for NodeHarness

Exercises the full send_input → tick → send_output → recv_output pipeline.

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```

---

### Task 3: Update docs

**Files:**
- Modify: `src/lib.rs` (status table)

- [ ] **Step 1: Add send_output to status table**

In `src/lib.rs`, after the `recv_output` status row, add:

```rust
//! | [`NodeHarness::send_output()`] | Implemented — delegates to [`DoraNode::send_output`]; outputs captured via `ToChannel` |
```

- [ ] **Step 2: Run doc tests**

```bash
cargo test --doc 2>&1
```
Expected: all ignored/pass, no errors.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "docs: add send_output to implementation status table

Co-Authored-By: Claude Opus 4.8 <noreply@anthropic.com>"
```
