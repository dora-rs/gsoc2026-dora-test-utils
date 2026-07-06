# Week 3–4 Summary: NodeHarness Core Implementation + Completion

**Timeline:** June 9 – June 14, 2026
**Status:** ✅ **COMPLETE** (ahead of original Week 3–5 schedule)

---

## Week 3: NodeHarness Core (June 9)

### send_output + E2E Test

- **`NodeHarness::send_output()`** — delegates to `DoraNode::send_output()`. Parses `output_id` from `&str`
  into `DataId`, wraps parse errors in `NodeError::Output` (not `NodeError::Init` — fixed after review).
  Returns `Result<(), NodeError>` instead of panicking on `.parse().unwrap()`.

- **E2E test** (`tests/e2e.rs`) — `e2e_receive_input_and_stop`: send_input(Input) + send_stop →
  tick → verify Input received → tick → verify Stop received → tick → assert stream exhausted.

### Bug Fixes

| Commit | Fix |
|--------|-----|
| `3af1b79` | Typo: `Convience` → `Convenience`; removed `.unwrap()` from send_output |
| `b4a1f47` | `NodeError::Init` → `NodeError::Output` for invalid output_id (semantic correctness) |
| `c3c801e` | Updated PROGRESS.md — Week 3 deliverables marked done |

### CI State (end of Week 3)

- `cargo fmt` ✅
- `cargo clippy -- -D warnings` ✅
- 10/10 tests passing (6 unit + 3 smoke + 1 E2E)

---

## Week 4: NodeHarness Completion + Deadlock Fixes (June 14)

### run_to_completion()

Implemented from scratch (was `todo!()`):

```rust
pub fn run_to_completion(&mut self) -> Vec<Event> {
    self.send_stop();           // auto-inject Stop — always terminates
    let mut events = Vec::new();
    while let Some(event) = self.tick() {
        let is_stop = matches!(event, Event::Stop(..));
        let is_input_closed = matches!(event, Event::InputClosed { .. });
        events.push(event);
        if is_stop || is_input_closed { break; }
    }
    self.close_input();          // unblocks daemon thread for safe send_output
    events
}
```

- Loops `tick()` until Stop, InputClosed, or stream exhaustion
- Auto-injects Stop at end of queue (caller doesn't need to pre-load one)
- Auto-calls `close_input()` before returning
- Returns `Vec<Event>` for full assertion flexibility

### close_input()

New public method that drops the `input_tx` flume sender:

```rust
pub fn close_input(&mut self) {
    self.input_tx.take();
}
```

- Unblocks the daemon thread (its `rx.recv()` returns `Disconnected`)
- Makes `send_output()` safe to call
- Idempotent by design (`Option::take()` on `None` is a no-op)
- `run_to_completion()` calls it automatically

### send_output Deadlock Fix

**Root cause:** The daemon thread is single-threaded — it blocks on `input_rx.recv()` while
processing the eagerly-issued `NextEvent` request from the event stream thread. Calling
`send_output()` sends `DaemonRequest::SendMessage` via the control channel, but the daemon
thread can't process it while blocked.

**Fix:** `send_output()` now auto-calls `close_input()` before delegating to
`self.node.send_output()`. This guarantees the daemon thread is unblocked before the
`SendMessage` request is sent.

### E2E Tests Extended (1 → 4)

| Test | What it covers |
|------|----------------|
| `e2e_receive_input_and_stop` | send_input(Input) + send_stop → tick×2 → verify Input + Stop (Week 3) |
| `e2e_send_output_and_recv` | send_output → recv_output — pure output path, no tick needed |
| `e2e_run_to_completion_returns_events` | send_input(Input) → run_to_completion() → assert events + send_output after |
| `e2e_full_pipeline_input_to_output` | send_input → run_to_completion → send_output×2 → recv_output assert count+data |

### Code Review Findings Fixed

Three bugs discovered by max-effort code review (10 angles × 8 candidates, 15 findings total):

| # | Finding | Severity | Fix |
|---|---------|----------|-----|
| 1 | `send_output()` deadlocks without `close_input()` | 🔴 CONFIRMED | Auto-call `close_input()` inside `send_output()` |
| 2 | `run_to_completion()` hangs without pre-loaded Stop | 🔴 CONFIRMED | Auto-inject `send_stop()` at start of `run_to_completion()` |
| 3 | `#[allow(dead_code)]` on `node` field is stale | 🟡 | Removed — `node` is used in `send_output()` |

---

## Architecture Overview

```
┌──────────────────┐  flume channel (input)  ┌──────────────────┐
│   Test code      │ ──────────────────────▶ │  DORA node       │
│  send_input()    │                         │  (the thing      │
│  tick()          │                         │   under test)    │
│  recv_output() ◀─│── flume channel (output)─│                  │
└──────────────────┘                         └──────────────────┘
```

Three threads involved:

| Thread | Role |
|--------|------|
| Test (main) | Calls send_input, tick, send_output, recv_output |
| Event stream | Sends `NextEvent` requests to daemon, forwards replies |
| Daemon | Processes requests one at a time: reads inputs from flume, writes outputs to flume |

### Lifecycle

1. **Input phase:** `send_input()` / `send_stop()` + `tick()` — inject and process events
2. **Completion:** `run_to_completion()` or `close_input()` — closes input channel, unblocks daemon
3. **Output phase:** `send_output()` (auto-closes input if needed) + `recv_output()` — send and assert outputs

---

## File Structure (end of Week 4)

```
src/
├── lib.rs              # Crate docs + status table + re-exports
├── harness.rs          # NodeHarness — 8 public methods, fully implemented
└── mock/
    ├── mod.rs          # Mock module docs
    ├── event_stream.rs # MockEventStream (full impl + 3 tests)
    └── output.rs       # MockOutputSender + OutputCollector (full impl + 3 tests)
tests/
├── smoke.rs            # 3 smoke tests
└── e2e.rs              # 4 E2E tests
dora/                   # Vendored dora source (TestingInput::Channel + EventSource patches)
docs/
├── PROGRESS.md
├── WEEK1-2_SUMMARY.md
├── WEEK3-DISCUSSION.md
    ├── specs/          # Design specs
    └── plans/          # Implementation plans
```

---

## NodeHarness API (Final, Frozen)

| Method | Signature | Description |
|--------|-----------|-------------|
| `new()` | `-> Result<Self, NodeError>` | Create harness with live DORA node in testing mode |
| `send_input()` | `(&mut self, TimedIncomingEvent)` | Inject synthetic input event at runtime |
| `send_stop()` | `(&mut self)` | Inject Stop event (convenience wrapper) |
| `send_output()` | `(&mut self, &str, impl Array) -> Result<(), NodeError>` | Send output; auto-closes input to prevent deadlock |
| `tick()` | `(&mut self) -> Option<Event>` | Drive one event loop iteration (synchronous) |
| `recv_output()` | `(&mut self, impl Into<String>) -> Option<Vec<Map>>` | Drain captured outputs by ID |
| `close_input()` | `(&mut self)` | Drop input sender; unblock daemon for safe output |
| `run_to_completion()` | `(&mut self) -> Vec<Event>` | Batch-run all events; auto-injects Stop; auto-closes input |

---

## Metrics (end of Week 4)

| Checkpoint | Target | Current | Status |
|-----------|--------|---------|--------|
| Week 1–2 API design | 7/7 deliverables | 7/7 | ✅ |
| Week 2 Mock impl | 6 unit + 3 smoke | 9/9 | ✅ |
| Week 3 NodeHarness core | 6 methods + E2E test | 10/10 tests | ✅ |
| Week 4 completion | close_input + run_to_completion + deadlock fix + 3 new E2E | 13/13 tests | ✅ |
| Week 5 Binaries | TestSource + TestSink | 0/2 | ⏳ |

**CI:** fmt ✅ | clippy ✅ | 13/13 tests passing

---

## Next: Week 5

- **TestSourceNode binary** (`src/bin/test_source.rs`): emit test data from file/inline JSON
- **TestSinkNode binary** (`src/bin/test_sink.rs`): receive + compare with expected, exit code 0/1

