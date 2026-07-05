# Week 4: NodeHarness Completion Design

## Overview

Complete the three remaining items from Week 3:
1. Implement `run_to_completion()`
2. Fix `send_output` deadlock
3. Extend E2E tests to cover output path

## 1. `run_to_completion()`

### Behavior

Loop `tick()` until the event stream is exhausted, then auto-close the input
channel to unblock the daemon thread for safe `send_output`/`recv_output` use.

```rust
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
    // Close input channel so daemon thread unblocks → send_output won't deadlock
    self.close_input();
    events
}
```

### Stop conditions

- `EventStream::recv()` returns `None` (channel disconnected/exhausted)
- `Event::Stop` received
- `Event::InputClosed` received (edge case: all inputs closed)

### Post-condition

After `run_to_completion()` returns, `close_input()` has been called:
- Daemon thread is unblocked (it sees `Disconnected` on the flume rx)
- `send_output()` and `recv_output()` are safe to call

## 2. `send_output` Deadlock Fix

### Root cause

The flume input sender (`input_tx`) is held until the harness is dropped.
After all pre-loaded events are consumed by `tick()`, the daemon thread
blocks in `rx.recv()` waiting for the next input.  Any `send_output()`
call sends a `DaemonRequest::SendMessage` that the daemon thread can
never process while blocked.

### Fix: `Option<input_tx>` + `close_input()`

Change `input_tx` from `flume::Sender<...>` to `Option<flume::Sender<...>>`:

```rust
pub(crate) input_tx: Option<flume::Sender<TimedIncomingEvent>>,
```

Add `close_input()`:

```rust
pub fn close_input(&mut self) {
    // Drop the sender → daemon thread's rx.recv() returns Disconnected
    self.input_tx.take();
}
```

Update `send_input` to handle `None`:

```rust
pub fn send_input(&mut self, event: TimedIncomingEvent) {
    self.input_tx
        .as_ref()
        .expect("input channel closed — call close_input() only after all ticks")
        .send(event)
        .expect("NodeHarness: input channel disconnected");
}
```

### Drop-order note

The original drop-order comment still applies: `input_tx` must be the FIRST
field so Rust drops it before `event_stream`/`node`.  Wrapping in `Option`
doesn't change this — `Option::take()` drops the inner sender immediately.

## 3. E2E Tests — Output Coverage

### Test 1: `e2e_send_output_and_recv`

```
harness.send_output("out1", array) → harness.recv_output("out1") → assert data
```

Tests the pure output path (no inputs needed). The daemon thread is idle
(not blocked) because no tick has been issued, so `send_output` works.

### Test 2: `e2e_run_to_completion_with_outputs`

```
harness.send_input(Input) → harness.send_stop() → harness.run_to_completion()
→ harness.send_output("out", array) → harness.recv_output("out") → assert data
```

Tests the full pipeline: inputs flow through ticks, then after completion
the output path is still functional because `close_input()` was called.

### Test 3: `e2e_run_to_completion_returns_events`

```
harness.send_input(Input) → harness.send_stop() → events = harness.run_to_completion()
→ assert events contains Input + Stop
```

Tests that `run_to_completion()` correctly collects all events.

## File Changes

| File | Change |
|------|--------|
| `src/harness.rs` | `input_tx` → `Option<Sender<...>>`; add `close_input()`; implement `run_to_completion()`; update `send_input()` |
| `src/lib.rs` | Update status table: `run_to_completion` → Implemented; `send_output` deadlock → Fixed |
| `tests/e2e.rs` | Add 3 new tests |
| `docs/PROGRESS.md` | Mark Week 4 deliverables complete |

## Risks

- **Drop-order**: Must keep `input_tx` as first field. `Option::take()` drops
  contents immediately, so this is safe.
- **Backward compat**: `send_input` now panics with a different message if
  called after `close_input()`. This is acceptable — it was already a panic
  on channel error.
