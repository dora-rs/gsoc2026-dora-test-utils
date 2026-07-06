# send_data() Convenience Method — Design Spec

**Date:** 2026-06-21
**Status:** Approved
**Scope:** Week 5 pre-work — closes API gap between proposal and implementation

## Motivation

Proposal §5.3.A shows a simple user-facing API:

```rust
harness.send_input("image", arrow_data);
```

The current `NodeHarness::send_input()` requires constructing a full
`TimedIncomingEvent` — a three-layer nest (`TimedIncomingEvent` →
`IncomingEvent::Input` → `InputData::JsonObject`). This is a material
deviation from the proposal's promise, adding onboarding friction.

## Design

### New Trait: `IntoInputData`

```rust
/// Convert test data into an [`InputData`] variant suitable for
/// injection through [`NodeHarness::send_data`].
pub trait IntoInputData {
    fn into_input_data(self) -> InputData;
}
```

### Impl 1: `serde_json::Value`

```rust
impl IntoInputData for serde_json::Value {
    fn into_input_data(self) -> InputData {
        InputData::JsonObject {
            data: self,
            data_type: None,   // DORA auto-infers Arrow type
        }
    }
}
```

### Impl 2: `arrow::array::ArrayData`

Convert via `arrow_json::writer::write_to_string`, then wrap in
`InputData::JsonObject`. Requires adding `arrow-json = "58"` to
dependencies.

```rust
impl IntoInputData for arrow::array::ArrayData {
    fn into_input_data(self) -> InputData {
        // Arrow → JSON string → serde_json::Value → JsonObject
        let mut buf = Vec::new();
        let mut writer = arrow_json::ArrayWriter::new(&mut buf);
        writer.write(&arrow::array::make_array(self)).unwrap();
        writer.finish().unwrap();
        let json_str = String::from_utf8(buf).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json_str).unwrap();
        InputData::JsonObject {
            data: value,
            data_type: None,
        }
    }
}
```

### New Method: `NodeHarness::send_data()`

```rust
impl NodeHarness {
    /// Convenience: inject input data by ID.
    ///
    /// Wraps the data in a [`TimedIncomingEvent`] and delegates to
    /// [`send_input`](Self::send_input).
    ///
    /// # Panics
    ///
    /// Panics if [`close_input`](Self::close_input) was already called,
    /// if the channel is disconnected, or if `input_id` is not a valid
    /// [`DataId`].
    ///
    /// # Example
    ///
    /// ```ignore
    /// // JSON data — most common case
    /// harness.send_data("image", serde_json::json!({"width": 640}));
    ///
    /// // Arrow data
    /// let array = Int32Array::from(vec![1, 2, 3]).into_data();
    /// harness.send_data("numbers", array);
    /// ```
    pub fn send_data(&mut self, input_id: &str, data: impl IntoInputData) {
        self.send_input(TimedIncomingEvent {
            time_offset_secs: 0.0,
            event: IncomingEvent::Input {
                id: input_id
                    .parse()
                    .expect("NodeHarness::send_data: invalid input_id"),
                metadata: None,
                data: Some(Box::new(data.into_input_data())),
            },
        });
    }
}
```

### End-to-End Usage Comparison

**Before (current API):**
```rust
harness.send_input(TimedIncomingEvent {
    time_offset_secs: 0.0,
    event: IncomingEvent::Input {
        id: "numbers".parse().unwrap(),
        metadata: None,
        data: Some(Box::new(InputData::JsonObject {
            data: serde_json::json!([1, 2, 3]),
            data_type: None,
        })),
    },
});
```

**After (new convenience method):**
```rust
harness.send_data("numbers", serde_json::json!([1, 2, 3]));
```

## Files Touched

| File | Change |
|------|--------|
| `Cargo.toml` | Add `arrow-json = "58"` dependency |
| `src/lib.rs` | Add `pub mod traits;` (or inline in harness.rs) |
| `src/harness.rs` | Add `send_data()` method |

## Trait Location Decision

Two options:

**A. New `src/traits.rs`** — clean separation, extensible for future traits
**B. Inline in `src/harness.rs`** — simpler, single-file change

→ Choose **A** (`src/traits.rs`). The crate only has 5 source files; adding
a dedicated traits module keeps things organized and signals where future
extension traits should live.

## Test Plan

1. **Unit test: `test_send_data_json`** — `send_data("id", json!(...))` → tick → assert Input event received
2. **Unit test: `test_send_data_arrow`** — `send_data("id", arrow_array.into_data())` → tick → assert Input event received
3. **Update existing E2E tests** — replace verbose `send_input(TimedIncomingEvent { ... })` with `send_data("id", json!(...))` where applicable
4. **Panic test: `test_send_data_panics_after_close`** — `close_input()` then `send_data()` → should panic

## Non-Goals

- Extending `recv_output()` API — out of scope
- Adding `send_data()` variants for `ArrowFile` / `InputClosed` — use `send_input()` for those
- Changing `send_input()` signature — existing method stays as-is for full control
