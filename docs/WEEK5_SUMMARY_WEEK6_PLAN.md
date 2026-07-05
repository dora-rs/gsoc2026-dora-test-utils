# Week 5 Summary & Week 6 Plan

> Week 5: 2026-06-22 ~ 2026-06-28
> Week 6 target: 2026-06-29 ~ 2026-07-05

---

## Week 5 — Completed Work

### Deliverables

| Module | Files | Status |
|--------|-------|--------|
| `TestSource` library | `src/source.rs` (370→476 lines) | ✅ |
| `TestSink` library | `src/sink.rs` (414→421 lines) | ✅ |
| `test-source` CLI binary | `src/bin/test_source.rs` (67 lines) | ✅ |
| `test-sink` CLI binary | `src/bin/test-sink.rs` (70 lines) | ✅ |
| Unit tests (source) | 8→17 tests | ✅ |
| Unit tests (sink) | 7 tests | ✅ |

### TestSource (`src/source.rs`, `src/bin/test_source.rs`)

- **Library**: `run_test_source(SourceConfig) -> Result<()>` — reads DORA-format JSON
  `{"data": [...], "data_type": {...}}`, converts each element to Arrow arrays
  respecting the `data_type` hint, initializes a DORA node via `init_from_env()`,
  and emits all arrays.
- **CLI**: `test-source --output-id <ID> --data-file <PATH>` or `--inline-data <JSON>`
- **Type inference**: `json_value_to_arrow_array()` now accepts an optional
  `DataType` hint, supports Int8–Int64, UInt8–UInt64, Float32, Float64,
  LargeUtf8.  Falls back to Int64/Float64/String/Boolean inference when hint is
  `None`.
- **data_type propagation**: Complex `DataType` values (Struct, List) are
  deserialized via `serde_json::from_value<DataType>()` (arrow-schema serde
  enabled transitively) and used as arrow_json schema hints.

### TestSink (`src/sink.rs`, `src/bin/test-sink.rs`)

- **Library**: `run_test_sink(SinkConfig) -> Result<SinkResult>` — reads expected
  JSON file, initializes DORA node, accumulates all `Event::Input` events,
  compares against expected data, writes `SinkResult` to output file.
- **CLI**: `test-sink --expected-file <PATH> --output-file <PATH> [--strict] [--no-fail-on-mismatch]`
- **Two comparison modes**: `compare_strict` (Arrow→JSON round-trip + exact
  serde_json::Value equality) and `compare_semantic` (expected JSON→Arrow
  conversion + arrow::Array equality).
- **`compare_sequences<E,R>()`**: Generic helper extracted from the duplicated
  comparison loop — eliminates ~60 lines of copy-paste.

### Code Review Rounds

- **Round 1** (pre-existing): 5 Important issues identified, then fixed:
  `from_str`→`from_slice`, non-scalar type handling in `compare_semantic`,
  `data_type` field ignoring, DRY comparison loop, `fail_on_mismatch` dead in
  library.
- **Round 2** (this week, max-effort /code-review): 15 findings across all
  severities.  7 bugs fixed:
  1. **Double-wrapping** in `json_array_to_arrow_struct` — each element was
     wrapped in `{"data": v}` by the caller, then `vec![obj]` wrapped again →
     `[[{"data":...}]]` instead of `[{"data":...}]`.
  2. **`expect()` panic on complex Arrow types** in `compare_strict` — types
     like List, Struct, Union are unsupported by `arrow_json::Writer<JsonArray>`
     and would crash the process.
  3. **Dead code in test-sink binary** — the mismatch-printing block was
     unreachable because `run_test_sink()` now returns `Err` on mismatch.
  4. **Schema/JSON key mismatch** — `json_obj_to_arrow_struct` built schema
     `Field("data", dt)` but passed raw object keys to arrow_json.
  5. **Error swallowed** in `compare_semantic` — `|_|` discarded conversion
     error info; now recorded as `Difference` entries.
  6. **Semantic comparison ignored `data_type`** — expected JSON's `data_type`
     was ignored, causing `Int32Array ≠ Int64Array` false mismatches.
  7. **Zero test coverage for explicit type hints** — added 9 tests covering
     Int8/16/32/64, UInt8 (normal+overflow+negative), Float32, LargeUtf8.

### Test Suite Status

```
✅ cargo fmt -- --check       — passes
✅ cargo clippy -- -D warnings — passes
✅ cargo test --lib            — 33 passed, 0 failed
   (3 harness tests skipped — pre-existing DORA daemon timing issue)
✅ cargo test --test e2e -- --test-threads=1 — 5 passed
```

---

## Week 6 — Plan

### Primary Goal: Integration Tests (per CLAUDE.md Weeks 6-8)

Drop `test-source` / `test-sink` binaries into real YAML dataflows alongside a
node under test, verifying end-to-end behavior.

#### Task 1: Example Dataflow for Integration Testing

- Create a minimal example: `test-source → node_under_test → test-sink`
- Write a YAML dataflow descriptor
- Implement a simple "echo" or "pass-through" test node
- Run with `DORA_TEST_WITH_INPUTS=1` (standalone testing mode)

#### Task 2: Integration Test Harness

- Write a test runner that:
  1. Starts `dora-coordinator` + `dora-daemon` (or uses `dora run`)
  2. Launches the dataflow with test-source / test-sink
  3. Reads `result.json` from test-sink's output
  4. Asserts `SinkResult.r#match == true`
- Add to `tests/` directory

#### Task 3: Edge Case Tests

- Empty data arrays → verify test-source exits with clear error
- Type mismatches → verify test-sink correctly reports differences
- Dataflow with multiple inputs/outputs → verify test-sink with multiple expected sinks
- Large data batches → verify no performance regressions

#### Task 4: Documentation Polish

- Update `src/lib.rs` status table ("Week 5" → "Implemented")
- Document TestSource/TestSink usage patterns in lib.rs doc
- Add example to `docs/` or a `README`

---

## Discussion Topics for Mentor Sync

### Topic 1: Integration test architecture — daemon vs. standalone mode

The current TestSource/TestSink library functions use
`DoraNode::init_from_env()`, which works in both modes:
- **Standalone** (`DORA_TEST_WITH_INPUTS=1`): no daemon needed, test runs
  in-process with file-based input/output.
- **Daemon-based**: full `dora-coordinator` + `dora-daemon` with TCP/shared-memory.

For integration tests, which mode should we prioritize?
- Standalone mode is simpler (no daemon lifecycle management), but may not
  exercise the full TCP/shared-memory path.
- Daemon-based mode matches real deployment but adds daemon startup/teardown
  complexity and port contention in CI.

**Question**: Should we write integration tests for both modes, or focus on one?
If standalone is sufficient, we can avoid the CI port-contention issues we've
seen with the E2E harness tests.

### Topic 2: `data_type` propagation across the pipeline

The current design handles `data_type` in test-source (respects user-specified
Arrow types) and test-sink (uses expected file's `data_type` for correct type
comparison).  However, the type information only flows through the JSON config
files — there's no in-band Arrow schema propagation.

**Question**: Is the current JSON-file-based `data_type` contract sufficient for
the integration testing use case?  Or should we consider adding a
`--schema-file` flag that passes an explicit Arrow schema (IPC format) to both
source and sink for stronger type guarantees?

### Topic 3: Midterm evaluation scope

Coding Phase 1 ends after Week 6 (midterm evaluation).  Per the CLAUDE.md
milestones, the deliverable is:
> Week 3–5: Implement NodeHarness, MockEventStream, MockOutputSender
> Week 6–8: Implement TestSourceNode / TestSinkNode binaries

We've completed Weeks 3-5 solidly (NodeHarness + mocks + TestSource/TestSink
ahead of schedule).  With the integration tests now planned for Week 6,
what should the midterm evaluation deliverable look like?
- Option A: A working integration test (one end-to-end dataflow) as a
  proof-of-concept.
- Option B: Comprehensive TestSource/TestSink API documentation + examples
  (no integration tests yet).
- Option C: Both — one integration test + updated docs.

**Question**: What level of completion would you expect for a successful
midterm evaluation?
