# Week 4: NodeHarness Completion Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement `run_to_completion()`, fix `send_output` deadlock by adding `close_input()`, extend E2E tests with output coverage.

**Architecture:** The `input_tx` field changes from `flume::Sender` to `Option<flume::Sender>`. A new `close_input()` method drops the sender, unblocking the daemon thread so `send_output` works after all ticks. `run_to_completion()` loops `tick()` until Stop/None, then auto-calls `close_input()`.

**Tech Stack:** Rust, dora-node-api, flume, arrow

---

## File Map

| File | Action | Responsibility |
|------|--------|----------------|
| `src/harness.rs` | Modify | `Option<input_tx>`, `close_input()`, `run_to_completion()` impl, update `send_input` |
| `src/lib.rs` | Modify | Status table: mark `run_to_completion` done, send_output deadlock fixed |
| `tests/e2e.rs` | Modify | Add 3 output-focused tests |
| `docs/PROGRESS.md` | Modify | Mark Week 4 done |

---

### Task 1: Change `input_tx` to `Option<Sender>` in harness struct

**Files:**
- Modify: `src/harness.rs:87`

**Context:** The struct field `input_tx: flume::Sender<TimedIncomingEvent>` needs to become `Option<flume::Sender<TimedIncomingEvent>>` so we can drop the sender on demand via `close_input()`.

- [ ] **Step 1: Change the field type**

Edit `src/harness.rs`, change line 87 from:

```rust
    pub(crate) input_tx: flume::Sender<TimedIncomingEvent>,
```

To:

```rust
    /// Sender for runtime event injection.
    /// Wrapped in `Option` so [`close_input`](Self::close_input) can drop the sender
    /// to unblock the daemon thread, making [`send_output`](Self::send_output) safe.
    pub(crate) input_tx: Option<flume::Sender<TimedIncomingEvent>>,
```

Replace the old doc comment (lines 84-87) with the above.

- [ ] **Step 2: Update `new()` to wrap in `Some()`**

Edit `src/harness.rs`, in the `new()` method, change the `Ok(Self { ... })` block. The `input_tx` field goes from:

```rust
            input_tx,
```

To:

```rust
            input_tx: Some(input_tx),
```

- [ ] **Step 3: Build check**

Run: `cargo check 2>&1`
Expected: Compilation errors in `send_input` (references `self.input_tx.send(...)` needs `.as_ref()`). This is expected — we fix in Task 3.

- [ ] **Step 4: Commit**

```bash
git add src/harness.rs
git commit -m "refactor: wrap input_tx in Option for close_input support"
```

---

### Task 2: Add `close_input()` method

**Files:**
- Modify: `src/harness.rs` (add method after `send_stop`)

- [ ] **Step 1: Add the method**

Insert after `send_stop()` (after line 166), before `send_output()`:

```rust
    /// Close the input channel, unblocking the daemon thread.
    ///
    /// After calling this, no more inputs can be sent via
    /// [`send_input`](Self::send_input). But [`send_output`](Self::send_output)
    /// and [`recv_output`](Self::recv_output) become safe to call without
    /// risk of deadlock — the daemon thread's `rx.recv()` returns
    /// `Disconnected` and it resumes processing `DaemonRequest::SendMessage`.
    ///
    /// [`run_to_completion`](Self::run_to_completion) calls this automatically
    /// after the event stream is exhausted.
    pub fn close_input(&mut self) {
        self.input_tx.take();
    }
```

- [ ] **Step 2: Build check**

Run: `cargo check 2>&1`
Expected: Same errors as Task 1 (send_input needs updating). No new errors from this method.

- [ ] **Step 3: Commit**

```bash
git add src/harness.rs
git commit -m "feat: add close_input() to unblock daemon thread"
```

---

### Task 3: Update `send_input()` and `send_stop()` for `Option<Sender>`

**Files:**
- Modify: `src/harness.rs:152-166`

- [ ] **Step 1: Update `send_input()`**

Edit the `send_input` method (lines 152-156). Replace:

```rust
    pub fn send_input(&mut self, event: TimedIncomingEvent) {
        self.input_tx
            .send(event)
            .expect("NodeHarness: input channel disconnected — node may have panicked");
    }
```

With:

```rust
    /// Inject a synthetic input event at runtime.
    ///
    /// The event is delivered to the node through the live
    /// [`TestingInput::Channel`].  The node receives it on its next
    /// [`EventStream::recv`] call.
    ///
    /// # Panics
    ///
    /// Panics if [`close_input`](Self::close_input) has already been called
    /// (input channel closed), or if the node's background thread has
    /// terminated (channel disconnected).
    pub fn send_input(&mut self, event: TimedIncomingEvent) {
        self.input_tx
            .as_ref()
            .expect("NodeHarness: input channel closed — close_input() was already called")
            .send(event)
            .expect("NodeHarness: input channel disconnected — node may have panicked");
    }
```

- [ ] **Step 2: Verify `send_stop()` still works**

`send_stop()` calls `self.send_input(...)` internally, so it automatically benefits from the fix. No changes needed to `send_stop()`.

- [ ] **Step 3: Build check**

Run: `cargo check 2>&1`
Expected: Clean compile. No errors.

- [ ] **Step 4: Run existing tests**

Run: `cargo test 2>&1`
Expected: All 10 tests pass (6 unit + 3 smoke + 1 E2E). The smoke test `harness_construction_and_tick` exercises `send_stop()` → `tick()` with the new Option wrapper.

- [ ] **Step 5: Commit**

```bash
git add src/harness.rs
git commit -m "fix: update send_input for Option<input_tx>; close_input support"
```

---

### Task 4: Implement `run_to_completion()`

**Files:**
- Modify: `src/harness.rs:232-234` (replace the `todo!()` stub)

- [ ] **Step 1: Replace the stub**

Replace lines 232-234:

```rust
    pub fn run_to_completion(&mut self) {
        todo!("run_to_completion — will loop tick() until idle (Week 4)")
    }
```

With:

```rust
    /// Run the node to completion, pumping events until the event stream
    /// is exhausted or a [`Stop`](Event::Stop) is received.
    ///
    /// Returns all events processed during the run.  After this method
    /// returns, [`close_input`](Self::close_input) has been called
    /// automatically — [`send_output`](Self::send_output) and
    /// [`recv_output`](Self::recv_output) are safe to use.
    ///
    /// # Usage
    ///
    /// Pre-load all inputs (including a [`Stop`](Event::Stop)) via
    /// [`send_input`](Self::send_input) / [`send_stop`](Self::send_stop),
    /// then call this method to drive the node through all events.
    ///
    /// ```ignore
    /// harness.send_input(my_input);
    /// harness.send_stop();
    /// let events = harness.run_to_completion();
    /// assert!(events.iter().any(|e| matches!(e, Event::Stop(..))));
    ///
    /// // Now safe: daemon thread is unblocked
    /// harness.send_output("out", my_array).unwrap();
    /// let outputs = harness.recv_output("out");
    /// ```
    pub fn run_to_completion(&mut self) -> Vec<Event> {
        let mut events = Vec::new();
        while let Some(event) = self.tick() {
            let is_stop = matches!(event, Event::Stop(..));
            let is_input_closed = matches!(event, Event::InputClosed { .. });
            events.push(event);
            if is_stop || is_input_closed {
                break;
            }
        }
        // Unblock the daemon thread so send_output won't deadlock.
        self.close_input();
        events
    }
```

- [ ] **Step 2: Build check**

Run: `cargo check 2>&1`
Expected: Clean compile.

- [ ] **Step 3: Run existing tests**

Run: `cargo test 2>&1`
Expected: All 10 tests pass. Existing tests don't call `run_to_completion()` yet, but the new code compiles and doesn't break anything.

- [ ] **Step 4: Commit**

```bash
git add src/harness.rs
git commit -m "feat: implement run_to_completion with auto close_input"
```

---

### Task 5: Write E2E test — `e2e_send_output_and_recv`

**Files:**
- Modify: `tests/e2e.rs` (append at end of file)

**Context:** Test the pure output path. Before any tick, the daemon thread is idle (waiting for requests in the `blocking_recv` loop), so `send_output` is safe and doesn't deadlock.

- [ ] **Step 1: Add the test**

Append to `tests/e2e.rs`:

```rust
/// Output path: send_output → recv_output (no tick needed).
///
/// Before any tick, the daemon thread is idle in its request loop,
/// so send_output is safe (no deadlock risk).
#[test]
fn e2e_send_output_and_recv() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Send an output via the harness (delegates to DoraNode::send_output).
    let output_id = "test_output";
    let array = arrow::array::Int32Array::from(vec![10, 20, 30]);
    harness
        .send_output(output_id, array)
        .expect("send_output should succeed before any tick");

    // Retrieve the output.
    let outputs = harness
        .recv_output(output_id)
        .expect("should have captured output for 'test_output'");
    assert_eq!(outputs.len(), 1, "expected one output message");
    assert!(outputs[0].contains_key("data"), "output should contain data");
}
```

Add the import at the top of `tests/e2e.rs` (if not already present):

```rust
use arrow::array::Int32Array;
```

- [ ] **Step 2: Run the new test (expect FAIL — output may not round-trip)**

Run: `cargo test e2e_send_output_and_recv 2>&1`
Expected: The test runs. It may fail if the output doesn't round-trip through the daemon thread correctly (e.g., output format mismatch). If it fails, note the error — we'll debug in the next step.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e.rs
git commit -m "test: add e2e_send_output_and_recv"
```

---

### Task 6: Write E2E test — `e2e_run_to_completion_returns_events`

**Files:**
- Modify: `tests/e2e.rs` (append)

- [ ] **Step 1: Add the test**

Append to `tests/e2e.rs`:

```rust
/// run_to_completion: pre-load Input + Stop, verify all events returned.
///
/// After run_to_completion() returns, the input channel is closed
/// (close_input was called), so send_output is safe.
#[test]
fn e2e_run_to_completion_returns_events() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Pre-load an Input event with data.
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "step1".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: serde_json::json!([42]),
                data_type: None,
            })),
        },
    });

    // Pre-load Stop so the daemon thread won't block.
    harness.send_stop();

    // Run to completion.
    let events = harness.run_to_completion();

    // Should have received both Input and Stop.
    assert!(
        events.len() >= 2,
        "expected at least 2 events (Input + Stop), got {}",
        events.len()
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Input { .. })),
        "should contain an Input event"
    );
    assert!(
        events.iter().any(|e| matches!(e, Event::Stop(..))),
        "should contain a Stop event"
    );

    // After run_to_completion(), send_output should work (close_input was called).
    let array = arrow::array::Int32Array::from(vec![99]);
    harness
        .send_output("post_run", array)
        .expect("send_output should succeed after run_to_completion");

    let outputs = harness.recv_output("post_run");
    assert!(outputs.is_some(), "should have captured output after run");
}
```

Add required imports at the top if not already present:

```rust
use dora_node_api::Event;
```

- [ ] **Step 2: Run the test**

Run: `cargo test e2e_run_to_completion_returns_events 2>&1`
Expected: The test should pass — `run_to_completion()` processes the pre-loaded Input and Stop events, then closes the input channel.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e.rs
git commit -m "test: add e2e_run_to_completion_returns_events"
```

---

### Task 7: Write E2E test — `e2e_full_pipeline_input_to_output`

**Files:**
- Modify: `tests/e2e.rs` (append)

- [ ] **Step 1: Add the test**

Append to `tests/e2e.rs`:

```rust
/// Full pipeline: send_input → tick through input → send_output → recv_output.
///
/// Verifies that the input and output paths both work within the same
/// harness lifecycle.
#[test]
fn e2e_full_pipeline_input_to_output() {
    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Phase 1: Send input + stop, drive to completion.
    harness.send_input(TimedIncomingEvent {
        time_offset_secs: 0.0,
        event: IncomingEvent::Input {
            id: "data_in".parse().unwrap(),
            metadata: None,
            data: Some(Box::new(InputData::JsonObject {
                data: serde_json::json!([1, 2, 3, 4, 5]),
                data_type: None,
            })),
        },
    });
    harness.send_stop();

    let events = harness.run_to_completion();
    assert!(
        events.iter().any(|e| matches!(e, Event::Stop(..))),
        "should have received Stop"
    );

    // Phase 2: After completion, send outputs (close_input was called).
    let array1 = arrow::array::Float64Array::from(vec![1.1, 2.2, 3.3]);
    harness
        .send_output("results", array1)
        .expect("send_output should succeed after run_to_completion");

    let array2 = arrow::array::Float64Array::from(vec![4.4, 5.5]);
    harness
        .send_output("results", array2)
        .expect("second send_output should also succeed");

    // Phase 3: Retrieve all outputs for "results".
    let outputs = harness
        .recv_output("results")
        .expect("should have captured outputs for 'results'");
    assert_eq!(outputs.len(), 2, "expected 2 output messages for 'results'");
    for output in &outputs {
        assert!(
            output.contains_key("data"),
            "each output should contain 'data'"
        );
    }
}
```

- [ ] **Step 2: Run the test**

Run: `cargo test e2e_full_pipeline_input_to_output 2>&1`
Expected: Should pass — verifies the full input → tick → output pipeline.

- [ ] **Step 3: Commit**

```bash
git add tests/e2e.rs
git commit -m "test: add e2e_full_pipeline_input_to_output"
```

---

### Task 8: Update `lib.rs` status table

**Files:**
- Modify: `src/lib.rs:58-60` (three lines in the status table)

- [ ] **Step 1: Update status table entries**

Change line 58 from:

```
| | [`NodeHarness::send_output()`] | Implemented — delegates to [`DoraNode::send_output`]; known limitation: deadlocks if daemon thread is blocked in `next_event()` (see harness docs) |
```

To:

```
| | [`NodeHarness::send_output()`] | Implemented — delegates to [`DoraNode::send_output`]; safe after [`close_input`](NodeHarness::close_input) or [`run_to_completion`](NodeHarness::run_to_completion) |
```

Change line 62 (currently missing `run_to_completion` entry). The status table doesn't have a row for `run_to_completion`. Add one after the `recv_output` row (after line 61):

```
| | [`NodeHarness::run_to_completion()`] | Implemented — loops tick() until Stop/None, auto-calls close_input() |
```

- [ ] **Step 2: Build check**

Run: `cargo check 2>&1`
Expected: Clean compile. No rustdoc warnings.

- [ ] **Step 3: Commit**

```bash
git add src/lib.rs
git commit -m "docs: update lib.rs status table for Week 4 completion"
```

---

### Task 9: Run full test suite + CI gates

**Files:** None (verification only)

- [ ] **Step 1: Run all tests**

```bash
cargo test 2>&1
```

Expected: All tests pass. Count should be ≥12 (6 unit + 3 smoke + 1 original E2E + 3 new E2E = 13 tests).

- [ ] **Step 2: Run cargo fmt**

```bash
cargo fmt -- --check
```

Expected: No formatting issues.

- [ ] **Step 3: Run cargo clippy**

```bash
cargo clippy -- -D warnings
```

Expected: No warnings.

- [ ] **Step 4: Commit if any format/clippy fixes**

```bash
git add -u
git commit -m "chore: fmt + clippy fixes for Week 4"
```

---

### Task 10: Update PROGRESS.md

**Files:**
- Modify: `docs/PROGRESS.md`

- [ ] **Step 1: Mark Week 4 deliverables complete**

In the "Week 4 (高优先级)" section, update the checklist:

```markdown
#### Week 4 (高优先级)
- [x] **NodeHarness::run_to_completion()** — Batch mode
  - [x] Loop tick() until Stop or stream exhausted
  - [x] Auto-calls close_input() to unblock daemon thread
  - [x] Returns Vec<Event> for assertion
- [x] **send_output deadlock fix**
  - [x] input_tx changed to Option<Sender>
  - [x] Added close_input() to drop sender on demand
  - [x] run_to_completion auto-calls close_input()
- [x] **E2E test coverage extended**
  - [x] e2e_send_output_and_recv — pure output path
  - [x] e2e_run_to_completion_returns_events — batch mode
  - [x] e2e_full_pipeline_input_to_output — full pipeline
- [x] **API Freeze** — Lock public signatures
  - [x] All NodeHarness public methods finalized
```

Update the metrics checkpoint:

```markdown
| Week 4 NodeHarness complete | run_to_completion + close_input + 3 E2E tests | 13/13 tests passing | ✅ |
```

- [ ] **Step 2: Commit**

```bash
git add docs/PROGRESS.md
git commit -m "docs: update PROGRESS.md — Week 4 complete"
```
