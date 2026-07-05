# GSoC 2026 Midterm Report — dora-test-utils

> **Student:** SunSunSun689 | **Mentor:** bobdingAI
> **Period:** Community Bonding + Coding Phase 1 (May–Jul 2026)
> **Branch:** `week5` | **Repo:** [SunSunSun689/gsoc2026-dora-test-utils](https://github.com/SunSunSun689/gsoc2026-dora-test-utils)

---

## 1. Milestone Progress

| Milestone | Planned | Delivered | Status |
|-----------|---------|-----------|--------|
| Week 1–2: Design + scaffold | NodeHarness API design, crate scaffold | Complete | ✅ |
| Week 3–5: Core harness + mocks | NodeHarness, MockEventStream, MockOutputSender | Complete | ✅ |
| Week 5: TestSource / TestSink | Library + CLI binaries | Complete (ahead) | ✅ |
| Week 6: Integration tests | Echo pipeline, 4 integration tests, demo | Complete | ✅ |
| Week 6–8: Test binaries | Originally this phase — done in Week 5 | Ahead | 🚀 |

**Verdict: Ahead of schedule.** TestSource/TestSink binaries were delivered in Week 5 (originally planned for Weeks 6–8). Week 6 completed the echo pipeline integration tests and midterm demo.

---

## 2. Modules Delivered

### 2.1 NodeHarness (`src/harness.rs` — 402 lines)

Unit-test driver for single DORA nodes. No daemon required.

| Method | Purpose |
|--------|---------|
| `new()` | Create harness, init DORA node via `init_testing()` |
| `send_input()` | Inject raw `TimedIncomingEvent` |
| `send_data()` | Convenience: inject data by ID (JSON or Arrow) |
| `send_output()` | Send output from the node (auto-closes input channel) |
| `tick()` | Drive node to process one event, collect outputs |
| `recv_output()` | Retrieve captured outputs by ID |
| `close_input()` | Drop input sender to unblock daemon thread |
| `run_to_completion()` | Auto-inject Stop, loop `tick()` until exhausted |

### 2.2 Mock Types (`src/mock/` — 298 lines)

Pure in-memory simulation — no real DORA node needed.

| Type | Purpose |
|------|---------|
| `MockEventStream` | Simulated event stream with multi-producer injection |
| `MockOutputSender` | Simulated output sender |
| `OutputCollector` | Collects outputs by ID for assertions |

### 2.3 IntoInputData Trait (`src/traits.rs` — 137 lines)

Format conversion for `NodeHarness::send_data()`.

| Implementation | Conversion |
|---------------|------------|
| `serde_json::Value` | Direct → `InputData::JsonObject` |
| `arrow::array::ArrayData` | Serialize to JSON array → `InputData::JsonObject` |

### 2.4 TestSource (`src/source.rs` + `src/bin/test_source.rs` — 570 lines)

Injects test data into DORA dataflows from JSON files.

```bash
test_source --output-id data --data-file source-data.json
```

- Supports Int8–UInt64, Float32/64, LargeUtf8 type hints
- Library: `run_test_source(SourceConfig) -> Result<()>`
- Each JSON element → separate Arrow array → separate `send_output()`

### 2.5 TestSink (`src/sink.rs` + `src/bin/test-sink.rs` — 488 lines)

Captures DORA outputs and compares with expected data.

```bash
test-sink --expected-file expected.json --output-file result.json
```

- Two comparison modes: `strict` (JSON round-trip) and `semantic` (Arrow equality)
- Respects `data_type` hint from expected file
- Library: `run_test_sink(SinkConfig) -> Result<SinkResult>`

### 2.6 Echo Node + Integration Tests

```
tests/fixtures/
├── echo-node.rs          # Pass-through node (28 lines)
├── echo-dataflow.yml     # YAML dataflow template
├── source-data.json      # Sample input
└── expected-output.json  # Sample expected output

tests/integration.rs      # 4 integration tests + test runner
scripts/demo.sh           # Midterm demo script
```

---

## 3. Test Suite

```
✅ cargo fmt -- --check          — passes
✅ cargo clippy -- -D warnings    — passes
✅ cargo test --lib               — 36 passed, 0 failed
✅ cargo test --test e2e          — 5 passed (--test-threads=1)
✅ cargo test --test smoke        — 3 passed
✅ cargo test --test integration  — 4 passed (--test-threads=1)
```

### Integration Tests (4 tests)

| Test | Scenario | Result |
|------|----------|--------|
| `echo_pipeline_exact_match_int64` | `[42, 99, -1]` Int64 round-trip | ✅ |
| `echo_pipeline_semantic_int32_tolerates_int64` | Int32/Int64 type tolerance | ✅ |
| `echo_pipeline_ten_elements` | 10-element array round-trip | ✅ |
| `echo_pipeline_string_data` | LargeUtf8 strings round-trip | ✅ |

### Code Review History

- **2 rounds** of `/code-review` (max-effort)
- **12 bugs fixed** across all severity levels (2 Critical, 4 High, 5 Important, 1 Medium)

---

## 4. Code Metrics

| Metric | Value |
|--------|-------|
| Total commits (week5 branch) | 28 |
| Rust source files | 10 |
| Total lines (src/) | ~1,900 |
| Library unit tests | 36 |
| E2E tests | 5 |
| Integration tests | 4 |
| Mock tests | 3 |

---

## 5. Demo

```bash
# One command to run the full demo:
bash scripts/demo.sh
```

The demo script:
1. Builds all 4 binaries (test_source, test-sink, echo-node, dora CLI)
2. Shows the YAML dataflow definition
3. Shows test input/expected data
4. Runs `dora run` — daemon spawns all 3 nodes, pipeline completes in <1s
5. Prints `result.json`: `{"match": true, "expected_count": 3, "received_count": 3, "differences": []}`
6. Runs 4 automated integration tests
7. Runs 36 library unit tests

Pipeline: `test-source → echo-node → test-sink`

---

## 6. Known Issues

- **3 harness unit tests** (`test_send_data_arrow`, `test_send_data_panics_after_close_input`, `e2e_run_to_completion_returns_events`) occasionally hang due to pre-existing DORA daemon timing/port contention. These are unrelated to our code — they're in the upstream `init_testing()` path. Workaround: `--test-threads=1`.
- **Port 6013** must be free before running daemon-based tests.

---

## 7. Next Steps — Coding Phase 2 (Jul–Aug 2026)

| Week | Plan |
|------|------|
| Week 7–8 | Edge case tests (empty arrays, type mismatches, large batches), CI integration |
| Week 9–10 | Example pipelines + comprehensive integration tests |
| Week 11–12 | Polish docs, address mentor feedback |
| Final | Record/Replay (extended scope), final submission |

---

## 8. Discussion Topics for Midterm Evaluation

1. **Integration test architecture**: Both standalone (`DORA_TEST_WITH_INPUTS`) and daemon-based (`dora run`) modes work. We chose daemon-based for demo impact. Is standalone mode worth adding?

2. **`data_type` propagation**: Currently JSON-file-based. Should we add a `--schema-file` flag for explicit Arrow IPC schema?

3. **CI setup**: Integration tests need `dora` CLI and port 6013. Should we add a GitHub Actions workflow, or keep them local-only for now?

---

*Report generated 2026-06-30. Midterm evaluation deadline: 2026-07-10.*
