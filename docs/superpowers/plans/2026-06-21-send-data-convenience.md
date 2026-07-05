# send_data() Convenience Method — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `NodeHarness::send_data()` convenience method to close the API gap between proposal (§5.3.A `harness.send_input("image", arrow_data)`) and the current verbose `send_input(TimedIncomingEvent { ... })`.

**Architecture:** New `traits.rs` module defines `IntoInputData` trait with two impls (`serde_json::Value` → `InputData::JsonObject`, `arrow::array::ArrayData` → `InputData::JsonObject` via Arrow JSON serialization). `NodeHarness::send_data()` wraps data into `TimedIncomingEvent` and delegates to the existing `send_input()`.

**Tech Stack:** Rust, arrow 58, arrow-json 58, serde_json

## Global Constraints

- `arrow-json = "58"` added to `Cargo.toml` dependencies
- API panic semantics match existing `send_input()` (panic on closed input channel)
- All new code must pass `cargo fmt`, `cargo clippy -- -D warnings`, `cargo test`
- No changes to existing `send_input()` signature — stay additive
- Trait location: new file `src/traits.rs` (re-exported from `src/lib.rs`)

---

### Task 1: Create `src/traits.rs` — `IntoInputData` trait + `serde_json::Value` impl

**Files:**
- Create: `src/traits.rs`
- Modify: `src/lib.rs` (re-export module)
- Modify: `Cargo.toml` (add `arrow-json = "58"`)

**Interfaces:**
- Produces: `IntoInputData` trait with method `fn into_input_data(self) -> InputData`
- Produces: `impl IntoInputData for serde_json::Value`

- [ ] **Step 1: Add `arrow-json` to `Cargo.toml`**

Open `Cargo.toml`. After the `arrow = "58"` line, add:

```toml
arrow-json = "58"
```

- [ ] **Step 2: Create `src/traits.rs` with trait definition and `serde_json::Value` impl**

```rust
//! Extension traits for converting test data into DORA input formats.
//!
//! The [`IntoInputData`] trait allows [`NodeHarness::send_data`] to
//! accept multiple data types — currently [`serde_json::Value`] and
//! [`arrow::array::ArrayData`].

use dora_node_api::integration_testing::integration_testing_format::InputData;

/// Convert test data into an [`InputData`] variant.
///
/// Implementations produce [`InputData`] values suitable for injection
/// through [`NodeHarness::send_data`](crate::NodeHarness::send_data).
pub trait IntoInputData {
    fn into_input_data(self) -> InputData;
}

impl IntoInputData for serde_json::Value {
    fn into_input_data(self) -> InputData {
        InputData::JsonObject {
            data: self,
            data_type: None,
        }
    }
}
```

- [ ] **Step 3: Register module in `src/lib.rs`**

Add after the existing `pub mod harness;` line:

```rust
pub mod traits;
```

Then add a re-export after the existing re-exports:

```rust
pub use traits::IntoInputData;
```

- [ ] **Step 4: Verify compilation**

Run: `cargo check`
Expected: Compiles cleanly, new module visible.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/traits.rs src/lib.rs
git commit -m "feat: add IntoInputData trait with serde_json::Value impl

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Implement `IntoInputData` for `arrow::array::ArrayData`

**Files:**
- Modify: `src/traits.rs`

**Interfaces:**
- Consumes: `IntoInputData` trait (from Task 1)
- Produces: `impl IntoInputData for arrow::array::ArrayData`

- [ ] **Step 1: Add the Arrow impl to `src/traits.rs`**

Append to `src/traits.rs`:

```rust
impl IntoInputData for arrow::array::ArrayData {
    fn into_input_data(self) -> InputData {
        use arrow::array::RecordBatch;
        use arrow::datatypes::{Field, Schema};
        use arrow_json::writer::{JsonArray, Writer};
        use std::sync::Arc;

        let data_type = self.data_type().clone();

        // Build a single-column RecordBatch wrapping this array.
        let array_ref = arrow::array::make_array(self);
        let schema = Schema::new(vec![Field::new(
            "data",
            data_type,
            true,
        )]);
        let batch = RecordBatch::try_new(Arc::new(schema), vec![array_ref])
            .expect("IntoInputData: failed to create RecordBatch from ArrayData");

        // Serialize the batch to JSON array format.
        let mut buf = Vec::new();
        let mut writer = Writer::<_, JsonArray>::new(&mut buf);
        writer.write(&batch).expect("IntoInputData: Arrow → JSON write failed");
        writer.finish().expect("IntoInputData: Arrow → JSON finish failed");
        drop(writer);

        let json_str = String::from_utf8(buf)
            .expect("IntoInputData: Arrow JSON output is valid UTF-8");

        // Parse JSON. The output is a JSON array of row objects;
        // DORA's JSON→Arrow converter handles this correctly.
        let value: serde_json::Value = serde_json::from_str(&json_str)
            .expect("IntoInputData: Arrow JSON output is valid JSON");

        InputData::JsonObject {
            data: value,
            data_type: None,
        }
    }
}
```

- [ ] **Step 2: Update `use` imports at top of `src/traits.rs`**

The top of the file should look like:

```rust
use dora_node_api::integration_testing::integration_testing_format::InputData;
```

No additional top-level imports needed — `arrow`, `arrow_json`, `serde_json` are crate dependencies and used inline with full paths in the impl.

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles cleanly. Both impls exist.

- [ ] **Step 4: Commit**

```bash
git add src/traits.rs
git commit -m "feat: add IntoInputData impl for arrow::array::ArrayData

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Add `NodeHarness::send_data()` method

**Files:**
- Modify: `src/harness.rs`

**Interfaces:**
- Consumes: `IntoInputData` trait (from Task 1), `IncomingEvent`, `TimedIncomingEvent`, `InputData` (from dora_node_api)
- Produces: `pub fn send_data(&mut self, input_id: &str, data: impl IntoInputData)`

- [ ] **Step 1: Add `send_data()` to `NodeHarness` impl block**

After the `send_input()` method (around line 157), insert:

```rust
    /// Convenience: inject input data by ID.
    ///
    /// Wraps `data` in a [`TimedIncomingEvent`] and delegates to
    /// [`send_input`](Self::send_input).  The data type must implement
    /// [`IntoInputData`] — currently [`serde_json::Value`] and
    /// [`arrow::array::ArrayData`].
    ///
    /// # Panics
    ///
    /// Panics if [`close_input`](Self::close_input) was already called,
    /// if the channel is disconnected, or if `input_id` is not a valid
    /// [`DataId`](dora_node_api::DataId).
    ///
    /// # Example
    ///
    /// ```ignore
    /// // JSON data — the most common case
    /// harness.send_data("image", serde_json::json!({"width": 640}));
    ///
    /// // Arrow data
    /// let array = Int32Array::from(vec![1, 2, 3]).into_data();
    /// harness.send_data("numbers", array);
    /// ```
    pub fn send_data(&mut self, input_id: &str, data: impl crate::traits::IntoInputData) {
        use dora_node_api::integration_testing::integration_testing_format::{
            IncomingEvent, TimedIncomingEvent,
        };

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
```

- [ ] **Step 2: Make `IntoInputData` import available in harness.rs**

The `harness.rs` already uses `use dora_node_api::...`. It does NOT need a direct `use crate::traits::IntoInputData` because the method uses the fully qualified path `impl crate::traits::IntoInputData` in the signature. (Alternatively, add `use crate::traits::IntoInputData;` to the imports and simplify the signature to `impl IntoInputData`.)

- [ ] **Step 3: Verify compilation**

Run: `cargo check`
Expected: Compiles cleanly.

- [ ] **Step 4: Commit**

```bash
git add src/harness.rs
git commit -m "feat: add NodeHarness::send_data() convenience method

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Write unit tests

**Files:**
- Modify: `src/traits.rs` (add `#[cfg(test)]` module)
- Modify: `src/harness.rs` (add `#[cfg(test)]` module)

**Interfaces:**
- Tests: `IntoInputData` impls + `send_data()` integration with harness

- [ ] **Step 1: Add trait unit tests to `src/traits.rs`**

Append to `src/traits.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn into_input_data_from_json_value() {
        let value = serde_json::json!([1, 2, 3]);
        let input_data = value.into_input_data();
        match input_data {
            InputData::JsonObject { data, data_type } => {
                assert_eq!(data, serde_json::json!([1, 2, 3]));
                assert!(data_type.is_none());
            }
            other => panic!("expected JsonObject, got {other:?}"),
        }
    }

    #[test]
    fn into_input_data_from_json_object() {
        let value = serde_json::json!({"key": "value", "num": 42});
        let input_data = value.into_input_data();
        match input_data {
            InputData::JsonObject { data, .. } => {
                assert_eq!(data["key"], "value");
                assert_eq!(data["num"], 42);
            }
            other => panic!("expected JsonObject, got {other:?}"),
        }
    }

    #[test]
    fn into_input_data_from_arrow_array_data() {
        use arrow::array::Int32Array;
        use arrow::datatypes::Int32Type;

        let array = Int32Array::from(vec![10, 20, 30]);
        let data = array.into_data();
        let input_data = data.into_input_data();

        match &input_data {
            InputData::JsonObject { data: value, .. } => {
                // Writer produces [{"data": 10}, {"data": 20}, {"data": 30}]
                let arr = value.as_array().expect("should be a JSON array");
                assert_eq!(arr.len(), 3);
                assert_eq!(arr[0]["data"], 10);
                assert_eq!(arr[1]["data"], 20);
                assert_eq!(arr[2]["data"], 30);
            }
            other => panic!("expected JsonObject, got {other:?}"),
        }
    }
}
```

- [ ] **Step 2: Add `send_data` integration tests to `src/harness.rs`**

Append to `src/harness.rs` (inside a `#[cfg(test)] mod tests { ... }` block at the bottom of the file):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_send_data_json() {
        let mut harness = NodeHarness::new().expect("harness should be created");

        harness.send_data("test_id", serde_json::json!([1, 2, 3]));

        // After send_data, the input should be queued. Drive with tick.
        let event = harness.tick().expect("should receive Input event");
        match event {
            dora_node_api::Event::Input { id, data, .. } => {
                assert_eq!(id.to_string(), "test_id");
                assert!(data.0.len() > 0, "data should be non-empty");
            }
            other => panic!("expected Input event, got {other:?}"),
        }
    }

    #[test]
    fn test_send_data_arrow() {
        use arrow::array::Int32Array;

        let mut harness = NodeHarness::new().expect("harness should be created");

        let array = Int32Array::from(vec![42, 99]).into_data();
        harness.send_data("arrow_in", array);

        let event = harness.tick().expect("should receive Input event");
        match event {
            dora_node_api::Event::Input { id, data, .. } => {
                assert_eq!(id.to_string(), "arrow_in");
                assert!(data.0.len() > 0, "data should be non-empty");
            }
            other => panic!("expected Input event, got {other:?}"),
        }
    }

    #[test]
    #[should_panic(expected = "input channel closed")]
    fn test_send_data_panics_after_close_input() {
        let mut harness = NodeHarness::new().expect("harness should be created");
        harness.close_input();
        harness.send_data("x", serde_json::json!(42));
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: 3 new tests in `traits` + 3 new tests in `harness` — all pass. Total: 19 tests (13 existing + 6 new).

- [ ] **Step 4: Verify formatting and clippy**

Run: `cargo fmt -- --check && cargo clippy -- -D warnings`
Expected: Both pass.

- [ ] **Step 5: Commit**

```bash
git add src/traits.rs src/harness.rs
git commit -m "test: add unit tests for IntoInputData and send_data()

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Update E2E tests to use `send_data()`

**Files:**
- Modify: `tests/e2e.rs`

**Interfaces:**
- Consumes: `NodeHarness::send_data()` (from Task 3)

- [ ] **Step 1: Rewrite E2E tests using `send_data()`**

Replace the verbose `send_input(TimedIncomingEvent { ... })` calls in `tests/e2e.rs` with `send_data()`.

In `e2e_receive_input_and_stop` (lines 23-33), replace:

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

with:

```rust
    harness.send_data("numbers", serde_json::json!([1, 2, 3]));
```

In `e2e_run_to_completion_returns_events` (lines 106-117), replace:

```rust
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
```

with:

```rust
    harness.send_data("step1", serde_json::json!([42]));
```

In `e2e_full_pipeline_input_to_output` (lines 156-166), replace:

```rust
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
```

with:

```rust
    harness.send_data("data_in", serde_json::json!([1, 2, 3, 4, 5]));
```

After these changes, the `IncomingEvent`, `InputData`, and `TimedIncomingEvent` imports in `tests/e2e.rs` may produce unused-import warnings if no other tests use them. Remove unused imports — keep only what's still needed (`IncomingEvent` for Stop, `Event` for matching).

Updated import block:

```rust
use dora_node_api::Event;
use dora_test_utils::NodeHarness;
```

The `IncomingEvent`, `InputData`, and `TimedIncomingEvent` imports are no longer needed — remove them.

- [ ] **Step 2: Add a new E2E test for `send_data` with Arrow input**

Append to `tests/e2e.rs`:

```rust
/// send_data with Arrow ArrayData: verify Arrow→JSON→Input round-trip.
#[test]
fn e2e_send_data_arrow_input() {
    use arrow::array::Int32Array;

    let mut harness = NodeHarness::new().expect("NodeHarness::new should succeed");

    // Inject Arrow data via the convenience method.
    let array = Int32Array::from(vec![100, 200, 300]).into_data();
    harness.send_data("arrow_numbers", array);
    harness.send_stop();

    // Tick — verify the data was received.
    let event = harness.tick().expect("should receive Input");
    match event {
        Event::Input { id, data, .. } => {
            assert_eq!(id.to_string(), "arrow_numbers");
            assert!(data.0.len() > 0, "data should be non-empty");
        }
        other => panic!("expected Input, got {other:?}"),
    }
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test`
Expected: All tests pass (13 existing + 6 new unit + 1 new E2E = 20 total).

- [ ] **Step 4: Verify formatting and clippy**

Run: `cargo fmt -- --check && cargo clippy -- -D warnings`
Expected: Both pass.

- [ ] **Step 5: Commit**

```bash
git add tests/e2e.rs
git commit -m "test: update E2E tests to use send_data() convenience method

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Final verification

- [ ] **Step 1: Full test suite**

Run: `cargo test`
Expected: 20/20 tests pass.

- [ ] **Step 2: Full CI gate check**

Run: `cargo fmt -- --check && cargo clippy -- -D warnings`
Expected: Both pass.

- [ ] **Step 3: Verify rustdoc**

Run: `cargo doc --no-deps --document-private-items 2>&1 | grep -i warning`
Expected: No warnings.

- [ ] **Step 4: Final commit (if any doc tweaks)**

```bash
git add -A
git commit -m "chore: final polish for send_data() convenience method

Co-Authored-By: Claude <noreply@anthropic.com>"
```
