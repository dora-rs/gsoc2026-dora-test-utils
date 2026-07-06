# Progress Log

## Week 1–2 (2026-05-30): API Design + Crate Scaffold

### Completed

- **Rust crate scaffolded** at repo root (`dora-test-utils` v0.1.0)
- **`NodeHarness` API designed** and stubbed:
  - `NodeHarness::new()` / `Default`
  - `send_input(id, Arrow ArrayData)` — inject synthetic input
  - `tick()` — drive one event loop iteration
  - `recv_output(id)` — drain captured outputs
  - `run_to_completion()` — batch-run until idle
- **Mock types designed** and stubbed:
  - `MockEventStream` — in-memory replacement for daemon `EventStream`
  - `MockOutputSender` / `OutputCollector` — capture `send_output` calls
- **CI template** at `.github/workflows/ci.yml`: check, test, clippy, fmt
- **3 smoke tests** passing (`cargo test` green)
- **Cargo mirror** configured (USTC) for faster crate downloads in China

### Key decisions

| Decision | Rationale |
|---|---|
| Crate at repo root (not `libraries/test-utils/`) | Standalone repo; path matches dora monorepo only when merged upstream |
| `arrow = "58"` | Matches the Arrow version DORA uses for inter-node data (upgraded from 53 in Week 2) |
| `flume` for output capture | `TestingOutput::ToChannel` uses flume; same channel crate DORA uses internally |
| `tokio` for mock channels | Mock types use `tokio::sync::mpsc` channels; same runtime as DORA |
| Stub-only (no real impl yet) | Week 1–2 scope is design + scaffold; implementation starts Week 3 |
| `dora-node-api` pinned to `45436aad` | Confirmed with mentor; locked to specific commit for reproducibility |

---

## Week 2 (2026-06-07): DORA Dependency Integration + Mentor Q&A

### Completed

- [x] **Clone & audit DORA source** at `dora/` (commit 45436aad)
- [x] **Post mentor discussion** with 4 questions in GitHub Discussions
- [x] **Update Cargo.toml** with `dora-node-api` = { git = "...", rev = "45436aad" }
- [x] **Implement MockOutputSender** (complete with unit tests)
  - [x] `MockOutputSender::send(output_id, ArrayData) -> Result<()>` ✅
  - [x] `OutputCollector` buffer & indexing by output_id ✅
  - [x] `OutputCollector::drain(id) -> Option<Vec<ArrayData>>` ✅
  - [x] `OutputCollector::collect_pending() -> async` ✅
  - [x] Unit tests: multi-output collection & sorting (3 tests passing) ✅
- [x] **Implement MockEventStream** (complete with unit tests)
  - [x] Replace `()` placeholder with real `dora_node_api::Event` ✅
  - [x] Implement `recv() -> Option<Event>` from mpsc channel ✅
  - [x] Unit tests: send events via channel (3 tests passing) ✅
- [x] **Cargo.toml adjustments**
  - [x] Upgrade Arrow from 53 → 58 (matches DORA main branch)
  - [x] Add futures = "0.3" for Stream trait compatibility
- [x] **NodeHarness fully wired** (2026-06-07, rewired 2026-06-09 per mentor feedback)
  - [x] Calls `DoraNode::init_testing()` with `TestingInput::Channel(rx)` + `TestingOutput::ToChannel(tx)`
  - [x] Both legs live: input via flume channel for runtime injection, output via flume for capture
  - [x] Returns `Result<Self, NodeError>` instead of panicking
  - [x] `send_input(TimedIncomingEvent)` implemented — pushes events through live channel
  - [x] `send_stop()` convenience method added
  - [x] `tick()` uses synchronous `EventStream::recv()` (blocking — consistent with `init_testing`)
  - [x] `recv_output()` drains flume-based output buffers (returns JSON maps per DORA format)
  - [x] Field order ensures clean shutdown: `input_tx` dropped first → unblocks daemon thread
  - [x] Upstream dora change: added `TestingInput::Channel(flume::Receiver<TimedIncomingEvent>)` variant
  - [x] Upstream dora change: added `EventSource` enum to `IntegrationTestingEvents` for channel support
  - [x] Smoke test passes: create harness → send_stop → tick → assert event received
- [x] **Bug fix: MockEventStream hanging tests**
  - [x] `test_mock_event_stream_multiple_events` — drop `tx` before checking `None`
  - [x] `test_mock_event_stream_multiple_senders` — drop `tx1` + `tx2` before checking `None`
  - [x] Root cause: `mpsc::Receiver::recv().await` returns `None` only when ALL senders dropped
- [x] **Documentation enhancements**
  - [x] `lib.rs` — implementation status table, architecture overview, cross-references
  - [x] `harness.rs` — module-level architecture diagram, per-method docs with `# Panics`
  - [x] `mock/mod.rs` — fixed intra-doc links
  - [x] All 5 rustdoc warnings resolved
- [x] **CI checks pass** — `cargo check` ✅ | `cargo test` ✅ | `cargo fmt` ✅ | `cargo clippy` ✅
- [x] **All 9 tests verified passing** (6 unit + 3 smoke) after bug fixes

### Key Findings from DORA Source

**Event type:** `dora/apis/rust/node/src/event_stream/event.rs`
```rust
#[non_exhaustive]
pub enum Event {
    Input { id: DataId, metadata: Metadata, data: ArrowData },
    InputClosed { id: DataId },
    InputRecovered { id: DataId },
    NodeRestarted { id: NodeId },
    Stop(StopCause),
    Reload { operator_id: Option<OperatorId> },
    ParamUpdate { key: String, value: serde_json::Value },
    ParamDeleted { key: String },
    NodeFailed { affected_input_ids: Vec<DataId>, error: String, source_node_id: NodeId },
    Error(String),
}
```

**init_testing() signature:** `dora/apis/rust/node/src/node/mod.rs`
```rust
pub fn init_testing(
    input: TestingInput,
    output: TestingOutput,
    options: TestingOptions,
) -> NodeResult<(DoraNode, EventStream)>
```

**EventStream:** Implements `futures::Stream<Item = Event>`; uses `tokio::sync::mpsc::Receiver`

### Next: Week 3 (COMPLETED 2026-06-09)

- [x] **Implement send_input / recv_output / tick** ✅
  - [x] `send_input(TimedIncomingEvent)` — pushes events through live flume channel
  - [x] `send_stop()` — convenience wrapper for Stop events
  - [x] `tick()` — synchronous, polls `EventStream::recv()`, collects outputs
  - [x] `recv_output(id)` — drains output buffers; returns `Option<Vec<Map>>`
- [x] **Added `TestingInput::Channel` variant upstream** (in vendored dora source)
  - [x] `TestingInput::Channel(flume::Receiver<TimedIncomingEvent>)` in `integration_testing.rs`
  - [x] `EventSource` enum in `node_integration_testing.rs` — supports Vec + Channel
  - [x] `check_poisoned()` extracted as reusable helper
- [x] **Added `NodeHarness::send_output()`** — delegates to `DoraNode::send_output`
  - [x] Known limitation: deadlocks if called between `tick()` calls (event-stream prefetch blocks daemon)
- [x] **Wrote end-to-end test** (`tests/e2e.rs`)
  - [x] `e2e_receive_input_and_stop`: send_input(Input) + send_stop → tick ×2 → verify events
- [x] **Fixed bugs from code review**
  - [x] Typo: "Convience" → "Convenience"
  - [x] `send_output`: `NodeError::Init` → `NodeError::Output` for invalid output_id
  - [x] `send_output`: removed `.parse().unwrap()` panic, returns `Result` instead
- [x] **CI gates pass**: fmt ✅ | clippy ✅ | 10/10 tests ✅ (6 unit + 3 smoke + 1 E2E)
- [x] **Mentor feedback resolved**: Option B confirmed (init_testing + Channel), pure-mock discarded

---

## Week 3–4 (COMPLETED 2026-06-14): Core Harness + IntoInputData

### Completed

- [x] **NodeHarness fully implemented** (`src/harness.rs` — 402 lines)
  - [x] `new()`, `send_input()`, `send_data()`, `send_stop()`, `send_output()`
  - [x] `tick()`, `recv_output()`, `close_input()`, `run_to_completion()`
- [x] **IntoInputData trait** (`src/traits.rs` — 137 lines)
  - [x] `serde_json::Value` impl, `arrow::array::ArrayData` impl
- [x] **MockEventStream + MockOutputSender** (`src/mock/` — 298 lines)
- [x] **E2E tests** (`tests/e2e.rs`) — 5 tests covering full pipeline

---

## Week 5 (COMPLETED 2026-06-28): TestSource / TestSink

### Completed

- [x] **TestSource library** (`src/source.rs` — 504 lines)
  - [x] `run_test_source(SourceConfig) -> Result<()>`
  - [x] `json_value_to_arrow_array()` with `data_type` hint (Int8–UInt64, Float32/64, LargeUtf8)
  - [x] Type inference: Int64/Float64/String/Boolean defaults
  - [x] Batch JSON parsing via `arrow_json::ReaderBuilder`
  - [x] 17 unit tests
- [x] **TestSource CLI** (`src/bin/test_source.rs` — 66 lines)
  - [x] `--output-id`, `--data-file`, `--inline-data` flags
- [x] **TestSink library** (`src/sink.rs` — 488 lines)
  - [x] `run_test_sink(SinkConfig) -> Result<SinkResult>`
  - [x] `compare_strict` (JSON round-trip) + `compare_semantic` (Arrow equality with cross-type tolerance)
  - [x] `SinkResult` / `Difference` serializable types
  - [x] `compare_sequences<E,R>()` generic helper
  - [x] 8 unit tests (including cross-type semantic comparison)
- [x] **TestSink CLI** (`src/bin/test-sink.rs` — 69 lines)
  - [x] `--expected-file`, `--output-file`, `--strict`, `--no-fail-on-mismatch` flags
- [x] **2 code review rounds** — 12 bugs fixed (2 Critical, 4 High, 5 Important, 1 Medium)

---

## Week 6 (COMPLETED 2026-06-30): Integration Tests + Midterm Prep

### Completed

- [x] **Echo node fixture** (`tests/fixtures/echo-node.rs` — 28 lines)
  - [x] Pass-through: receives Input, sends same data as Output
  - [x] Handles `InputClosed` gracefully (doesn't break, waits for `Stop`)
- [x] **YAML dataflow template** (`tests/fixtures/echo-dataflow.yml`)
  - [x] `test-source → echo-node → test-sink` pipeline
- [x] **Sample data fixtures** (`tests/fixtures/source-data.json`, `expected-output.json`)
- [x] **Integration test framework** (`tests/integration.rs` — 300 lines)
  - [x] `run_echo_pipeline()` — generates YAML, runs `dora run`, asserts `SinkResult`
  - [x] `build_binaries()` — cached (OnceLock), profile-aware
  - [x] `dora_binary()` — checks vendored debug + release paths, falls back to PATH
  - [x] 4 integration tests (exact match, cross-type tolerance, 10 elements, strings)
- [x] **Demo script** (`scripts/demo.sh`)
  - [x] Build → show dataflow → run pipeline → show result → integration tests → unit tests
  - [x] Robust error handling: exit codes tracked, timeouts distinguished from failures
- [x] **Midterm report** (`docs/MIDTERM-REPORT.md`)
- [x] **Max-effort code review** — 15 findings, all fixed

### Current file structure

```
gsoc2026-dora-test-utils/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs                     # Crate docs + status table + re-exports
│   ├── harness.rs                 # NodeHarness (402 lines)
│   ├── traits.rs                  # IntoInputData trait (137 lines)
│   ├── source.rs                  # TestSource library (515 lines)
│   ├── sink.rs                    # TestSink library (480 lines)
│   ├── mock/
│   │   ├── mod.rs                 # Mock module (23 lines)
│   │   ├── event_stream.rs        # MockEventStream (111 lines)
│   │   └── output.rs              # MockOutputSender + OutputCollector (164 lines)
│   └── bin/
│       ├── test_source.rs         # test-source CLI (66 lines)
│       └── test-sink.rs           # test-sink CLI (69 lines)
├── tests/
│   ├── smoke.rs                   # 3 smoke tests
│   ├── e2e.rs                     # 5 E2E tests
│   ├── integration.rs             # 4 integration tests (+ test runner)
│   └── fixtures/
│       ├── echo-node.rs           # Echo pass-through node (28 lines)
│       ├── echo-dataflow.yml      # YAML dataflow template
│       ├── source-data.json       # Sample input
│       └── expected-output.json   # Sample expected output
├── scripts/
│   └── demo.sh                    # Midterm demo script
├── dora/                          # Vendored dora source
├── docs/
│   ├── PROGRESS.md                # This file
│   ├── MIDTERM-REPORT.md          # GSoC midterm evaluation report
│   ├── proposal.pdf               # Accepted GSoC proposal
│       ├── specs/
│       └── plans/
├── .github/workflows/
│   └── ci.yml
├── CLAUDE.md
├── README.md
└── LICENSE
```

---

## 📋 Next: Week 7–8 (Coding Phase 2)

### Week 7–8: Edge Cases + CI
- [ ] **Edge case tests**
  - [ ] Empty data arrays → verify clear error
  - [ ] Type mismatches → verify correct Difference reporting
  - [ ] Large data batches → verify no performance regression
  - [ ] Multi-input/multi-output dataflows
- [ ] **CI integration**
  - [ ] Build dora CLI in CI workflow
  - [ ] Run integration tests in CI (requires dora + port 6013)
  - [ ] Add integration test job to `.github/workflows/ci.yml`

### Week 9–10: Example Pipelines
- [ ] Example dataflows (echo, classifier, multi-node)
- [ ] Comprehensive integration tests
- [ ] README usage examples

### Week 11–12: Polish + Midterm Evaluation
- [ ] Documentation (API docs, setup guide, usage guide)
- [ ] Mentor feedback integration
- [ ] Midterm evaluation submission (deadline: 2026-07-10)

### Extended Scope (Post-Midterm)
- [ ] Record/Replay (Week 13–17)
- [ ] Python bindings (Week 18–20)

---

## 🚧 Blockers & Risks

| Blocker | Status | Mitigation |
|---------|--------|-----------|
| **Q1: DORA commit pin** | ✅ Resolved | Locked to 45436aad (2026-06-07) |
| **Q2: init_testing() usage** | ✅ Resolved | Analyzed source code; ready to implement |
| **Arrow 53 vs 58 compat** | ✅ Resolved | Upgraded to 58; matches DORA main |
| **Event enum non_exhaustive** | ✅ Known | Match on all variants; use `_ => {}` for future-proofing |
| **Async runtime in tests** | ✅ Handled | Using `#[tokio::test]` macro |

---

## 📊 Metrics & Checkpoints

| Checkpoint | Target | Actual | Status |
|-----------|--------|--------|--------|
| Week 1–2 API design | 7/7 deliverables | 7/7 | ✅ |
| Week 2 Mock impl | 6 unit + 3 smoke tests | 9/9 passing | ✅ |
| Week 3 NodeHarness core | 6 methods + E2E test | 10/10 passing | ✅ |
| Week 4 NodeHarness complete | close_input + run_to_completion | 13/13 passing | ✅ |
| Week 5 Binaries | TestSource + TestSink | 25 tests | ✅ |
| Week 6 Integration tests | Echo pipeline + demo | 4/4 integration, 39 unit | ✅ |
| **Mid-term eval (Week 6)** | MVP complete | Ahead of schedule | 🚀 |
| Week 7–8 Edge cases + CI | TBD | TBD | ⏳ |
| **Final submission** | Extended complete | TBD | ⏳ |

### Code Metrics (Week 6 snapshot)

| Metric | Value |
|--------|-------|
| Total Rust source files | 10 |
| Total lines (src/ + tests/) | ~2,500 |
| Library unit tests | 39 |
| E2E tests | 5 |
| Integration tests | 4 |
| Mock tests | 3 |
| CI gates (fmt, clippy, test) | All passing |
| Code review rounds | 3 (max-effort, 27 findings fixed) |
