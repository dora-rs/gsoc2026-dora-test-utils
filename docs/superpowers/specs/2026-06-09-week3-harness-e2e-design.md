# Week 3: NodeHarness E2E — Design Spec

**Date:** 2026-06-09
**Status:** Approved
**Scope:** `NodeHarness::send_output` + end-to-end smoke test

## Goal

Complete the Week 3 deliverable: a working end-to-end test that exercises the
full `send_input → tick → send_output → recv_output` pipeline against a real
DORA node in testing mode.

## Current state

The harness already has:

- `new()` — live `TestingInput::Channel` + `TestingOutput::ToChannel`
- `send_input(TimedIncomingEvent)` — runtime event injection
- `send_stop()` — convenience Stop injector
- `tick() -> Option<Event>` — synchronous, polls `EventStream::recv()`
- `recv_output(id) -> Option<Vec<Map>>` — drains output buffers

What's missing:

- No way for test code to call `send_output` on the node (the harness wraps
  `DoraNode` but doesn't expose its output method).
- No end-to-end test proving the full pipeline works.

## Design

### 1. `NodeHarness::send_output`

Expose `DoraNode::send_output` through the harness so test code can produce
outputs that get captured by `TestingOutput::ToChannel`.

```rust
pub fn send_output(
    &mut self,
    output_id: &str,
    data: arrow::array::ArrayData,
) -> Result<(), NodeError>
```

Delegates to `self.node.send_output(…)`. The output lands in the flume
channel connected to `self.output_rx` — automatically collected by
`recv_output`.

**Error handling:** Returns `NodeError` from the underlying call. No panic.

**Metadata:** Pass `Metadata::default()` with the current timestamp
(via `uhlc::HLC`). This mirrors what a real node does.

### 2. End-to-end test

A single `#[test]` in `tests/e2e.rs` that exercises all harness methods:

1. **Construct** harness via `NodeHarness::new()`
2. **Inject** an `IncomingEvent::Input` with JSON data
3. **Tick** — receive the event from the node
4. **Process** — match `Event::Input`, extract data
5. **Send output** via `harness.send_output("echo", data)`
6. **Assert** via `harness.recv_output("echo")` — output is present

The test sends a `Stop` event after the Input to trigger clean shutdown.

**Data format:** Use `InputData::JsonObject` with a simple integer value.
The "node logic" is the test itself (inline event matching + send_output).

### 3. Files changed

| File | Change |
|------|--------|
| `src/harness.rs` | Add `send_output` method (~10 lines) |
| `tests/e2e.rs` | New file — one end-to-end test (~50 lines) |
| `src/lib.rs` | Update status table: `send_output` entry |

### 4. Out of scope (Week 4)

- `run_to_completion()` — remains `todo!()`
- Ergonomic `send_input` overload for raw `ArrayData` (code-review finding #8)
- API freeze confirmation

## Verification

```bash
cargo fmt -- --check && cargo clippy -- -D warnings && cargo test
```

Success criteria: all existing 9 tests + the new E2E test pass.
