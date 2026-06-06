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
| `arrow = "53"` | Matches the Arrow version DORA uses for inter-node data |
| `tokio` for mock channels | `mpsc` channels replace daemon socket; same runtime as DORA |
| Stub-only (no real impl yet) | Week 1–2 scope is design + scaffold; implementation starts Week 3 |
| `dora-node-api` NOT yet a dependency | Need to confirm the exact git rev / crate name with mentor before wiring up |

---

## Week 2 (2026-06-07): DORA Dependency Integration + Mentor Q&A

### Completed

- [x] **Clone & audit DORA source** at `dora/` (commit 45436aad)
- [x] **Post mentor discussion** with 4 questions in GitHub Discussions
- [x] **Update Cargo.toml** with `dora-node-api` = { git = "...", branch = "main" }
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

### Next: Week 3 (Pending Mentor Feedback)

- [ ] **Receive mentor feedback** on 4 questions (Q1: DORA commit, Q2: init_testing usage)
- [ ] **Implement NodeHarness::new()** calling `DoraNode::init_testing()`
- [ ] **Implement send_input / recv_output / tick / run_to_completion**
- [ ] **Write end-to-end unit tests** with real node execution

---

## Week 3–5 (Next: Pending Week 2 PR merge): Core Implementation

### Planned (Week 3)

- [ ] **Receive mentor feedback** on Q1/Q2 (DORA commit + init_testing usage)
- [ ] **Implement NodeHarness::new()** calling `DoraNode::init_testing()`
  - [ ] Create TestingInput / TestingOutput wrappers
  - [ ] Wire mock channels into EventStream
- [ ] **Implement send_input / recv_output / tick**
  - [ ] `send_input(id, ArrayData) -> Result<()>` → inject into mock EventStream
  - [ ] `recv_output(id) -> Vec<ArrayData>` → drain from OutputCollector
  - [ ] `tick() -> Result<()>` → drive one event loop iteration
- [ ] **Write end-to-end unit test:** create harness → send input → tick → verify output

### Planned (Week 4)

- [ ] **Implement run_to_completion()**
  - [ ] Loop tick() until input channel exhausted + no pending events
  - [ ] Write tests: batch-run multi-input scenario
- [ ] **API freeze confirmation** — no more public signature changes after this week

### Planned (Week 5)

- [ ] **TestSourceNode binary** (`src/bin/test_source.rs`)
  - [ ] Read Arrow JSON from file / inline
  - [ ] CLI: `--output-id`, `--data-file`, `--inline-data`
  - [ ] Emit data on specified output
- [ ] **TestSinkNode binary** (`src/bin/test_sink.rs`)
  - [ ] Receive input, compare with expected file
  - [ ] `--expected-file`, `--fail-on-mismatch` flags
  - [ ] Exit code: 0 on match, non-zero on mismatch
- [ ] **Preparatory work for integration tests** (requires both binaries)

### Current file structure

```
gsoc2026-dora-test-utils/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs
│   ├── harness.rs          # NodeHarness stub
│   └── mock/
│       ├── mod.rs
│       ├── event_stream.rs  # MockEventStream stub
│       └── output.rs        # MockOutputSender + OutputCollector stub
├── tests/
│   └── smoke.rs            # 3 passing tests
├── .github/workflows/
│   └── ci.yml
├── CLAUDE.md
├── README.md
├── LICENSE
└── proposal.pdf
```

---

## 📋 TO DO (Prioritized by Week)

### 🔴 Critical Path (Blocking)

#### Week 3 (高优先级)
- [x] **Q1: DORA commit pin** — ✅ Locked to 45436aad (2026-06-07)
- [x] **Q2: init_testing() signature** — ✅ Found in dora source code
- [ ] **Implement NodeHarness::new()** — Core entry point
  - [ ] Wrap `DoraNode::init_testing(TestingInput, TestingOutput, TestingOptions)`
  - [ ] Wire up MockEventStream as event source
  - [ ] Wire up MockOutputSender as output sink
  - [ ] Handle async runtime (tokio)
- [ ] **NodeHarness::send_input()** — Inject test events
  - [ ] Create Event::Input from user-provided ArrayData
  - [ ] Push to MockEventStream mpsc channel
  - [ ] Handle input_id validation
- [ ] **NodeHarness::recv_output()** — Drain captured outputs
  - [ ] Call OutputCollector::drain(output_id)
  - [ ] Return Vec<ArrayData> or None
- [ ] **NodeHarness::tick()** — Drive one iteration
  - [ ] Poll node event loop once
  - [ ] Collect any outputs via OutputCollector::collect_pending()
- [ ] **End-to-end test** — Verify harness works
  - [ ] Create node via harness
  - [ ] Send synthetic Input event
  - [ ] Call tick()
  - [ ] Assert output received

#### Week 4 (高优先级)
- [ ] **NodeHarness::run_to_completion()** — Batch mode
  - [ ] Loop tick() until input channel exhausted
  - [ ] Break on Event::Stop
  - [ ] Batch test scenario: 3 inputs → verify all outputs
- [ ] **API Freeze** — Lock public signatures
  - [ ] Review all public methods
  - [ ] Document breaking-change policy for Week 5+

#### Week 5 (中优先级)
- [ ] **TestSourceNode binary** (`src/bin/test_source.rs`)
  - [ ] Accept CLI args: `--output-id`, `--data-file` OR `--inline-data`
  - [ ] Load Arrow JSON from file or parse inline JSON
  - [ ] Spawn as DORA node in dataflow
  - [ ] Emit loaded data on specified output
  - [ ] Unit + integration tests
- [ ] **TestSinkNode binary** (`src/bin/test_sink.rs`)
  - [ ] Accept CLI args: `--expected-file`, `--fail-on-mismatch`
  - [ ] Receive input from dataflow
  - [ ] Compare with expected Arrow JSON (byte-for-byte or semantic)
  - [ ] Write result file (`result.json` or similar)
  - [ ] Exit code: 0 (match) / 1 (mismatch)
  - [ ] Unit + integration tests

### 🟡 Medium Priority (Non-blocking)

#### Week 6–8 (中优先级)
- [ ] **Source + Sink integration tests**
  - [ ] 3-node dataflow: TestSource → Node → TestSink
  - [ ] `dora run` end-to-end validation
  - [ ] Multiple input/output scenarios
  - [ ] Error propagation (source read error, sink comparison fail)

#### Week 9–10 (低优先级)
- [ ] **Example pipelines**
  - [ ] Example 1: Single-node unit test with NodeHarness
  - [ ] Example 2: Multi-node YAML + TestSource/Sink
  - [ ] Example 3: Complex dataflow (fan-out, merging)
  - [ ] CI integration: Run examples in workflow
  - [ ] Docs: Each example with README + expected output

#### Week 11–12 (低优先级)
- [ ] **Documentation**
  - [ ] API rustdoc (NodeHarness, MockEventStream, MockOutputSender, OutputCollector)
  - [ ] Setup Guide (clone, build, first test)
  - [ ] Usage Guide (unit testing, integration testing, regression testing)
  - [ ] README polish: Quick start, feature matrix, limitations
- [ ] **Mentor review + polish**
  - [ ] Code review feedback integration
  - [ ] Test coverage audit
  - [ ] Mid-term evaluation prep

### 🟢 Extended Scope (Post-MVP, 350h only)

#### Week 13–14 (扩展功能)
- [ ] **RecordSession** — Capture real dataflow I/O
  - [ ] Intercept all inter-node messages
  - [ ] Serialize to `integration_testing_format.rs` JSON
  - [ ] Write to `tests/fixtures/run_*.json`

#### Week 15–17 (扩展功能)
- [ ] **ReplaySession** — Deterministic replay
  - [ ] Load recorded session file
  - [ ] Reinject events to node
  - [ ] Collect outputs
  - [ ] `assert_no_regression()` helper

#### Week 18–20 (扩展功能)
- [ ] **Python bindings** (PyO3)
  - [ ] Expose NodeHarness to Python
  - [ ] Test Python nodes via harness
- [ ] **CI template** — GitHub Actions preset
  - [ ] Example workflow: unit + integration + regression tests
  - [ ] Reusable job templates

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

| Checkpoint | Target | Current | Status |
|-----------|--------|---------|--------|
| Week 1–2 API design | 7/7 deliverables | 7/7 | ✅ |
| Week 2 Mock impl | 6 tests passing | 6 tests | ✅ (compiled, untested runtime) |
| Week 3 NodeHarness core | 4 methods + E2E test | 0/5 | ⏳ |
| Week 5 Binaries | TestSource + TestSink | 0/2 | ⏳ |
| Week 11 Docs | API + Setup + Usage | 0/3 | ⏳ |
| **Mid-term eval (Week 12)** | MVP complete | TBD | ⏳ |
| **Final submission (Week 20)** | Extended complete | TBD | ⏳ |
