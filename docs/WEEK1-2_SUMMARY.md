# Week 1–2 Summary: API Design + Crate Scaffold

**Timeline:** May 30 – June 7, 2026  
**Status:** ✅ **COMPLETE**

---

## Completed Deliverables

### 1. Crate Scaffolded ✅
- Created `dora-test-utils` v0.1.0 at repo root
- `Cargo.toml` configured with key dependencies:
  - `arrow = "53"` (matches DORA inter-node data format)
  - `tokio` for async mock channels
- `Cargo.lock` committed for reproducibility
- Cargo mirror configured (USTC) for faster downloads in China

### 2. Core API Designed & Stubbed ✅

#### `NodeHarness` (src/harness.rs)
```rust
pub struct NodeHarness { ... }

impl NodeHarness {
    pub fn new() -> Self                           // Create harness with mock channels
    pub fn send_input(&mut self, id: &str, data: ArrayData) -> Result<()>  // Inject input
    pub fn tick(&mut self) -> Result<()>          // Drive one event loop iteration
    pub fn recv_output(&mut self, id: &str) -> Vec<ArrayData>  // Drain outputs
    pub async fn run_to_completion(&mut self) -> Result<()>    // Batch run until idle
}
```

#### `MockEventStream` (src/mock/event_stream.rs)
```rust
pub struct MockEventStream { /* ... */ }
impl MockEventStream {
    pub fn new() -> (Self, Sender<Event>)         // Create with sender handle
    pub async fn recv(&mut self) -> Option<Event> // Pull from mpsc channel
}
```

#### `MockOutputSender` + `OutputCollector` (src/mock/output.rs)
```rust
pub struct MockOutputSender { /* ... */ }
pub struct OutputCollector { /* ... */ }

impl MockOutputSender {
    pub fn new() -> (Self, OutputCollector)
    pub fn send(&self, output_id: String, data: ArrayData) -> Result<()>
}

impl OutputCollector {
    pub fn drain(&mut self, output_id: &str) -> Vec<ArrayData>
}
```

### 3. Project Structure ✅
```
gsoc2026-dora-test-utils/
├── Cargo.toml
├── Cargo.lock
├── src/
│   ├── lib.rs                      # Module exports
│   ├── harness.rs                  # NodeHarness stub (50 lines)
│   └── mock/
│       ├── mod.rs
│       ├── event_stream.rs         # MockEventStream stub (30 lines)
│       └── output.rs               # MockOutputSender + OutputCollector stub (40 lines)
├── tests/
│   └── smoke.rs                    # 3 compilation-level smoke tests
├── .github/workflows/
│   └── ci.yml                      # check / test / clippy / fmt jobs
├── CLAUDE.md                       # Student/mentor/milestone scaffold
├── README.md
├── LICENSE
├── proposal.pdf
└── docs/
    ├── PROGRESS.md
    ├── WEEKLY_PLAN.md
    └── WEEK1-2_SUMMARY.md (this file)
```

### 4. CI Pipeline Configured ✅
- Workflow: `.github/workflows/ci.yml`
- Jobs: `cargo check`, `cargo test`, `cargo clippy`, `cargo fmt`
- Triggers on: push to all branches, PR to main
- Status: All tests pass ✅

### 5. Smoke Tests ✅
Three compilation-level tests in `tests/smoke.rs`:
- ✅ `test_harness_creation` — NodeHarness constructs
- ✅ `test_mock_event_stream_creation` — MockEventStream + OutputCollector construct
- ✅ `test_mock_output_sender_creation` — MockOutputSender constructs and compiles

---

## Key Decisions Made

| Decision | Rationale |
|----------|-----------|
| **Crate at repo root** (not `libraries/test-utils/`) | Standalone repo during GSoC; path aligns with dora monorepo structure only after upstream merge |
| **arrow = "53"** | Matches Arrow version DORA uses for inter-node data interchange |
| **tokio + mpsc** | `mpsc` channels replace daemon socket; same async runtime as DORA core |
| **Stub-only implementation** | Week 1–2 scope is API design + scaffolding; real impl starts Week 3 |
| **dora-node-api NOT yet a dependency** | Need mentor confirmation of exact git rev / crate name before wiring up |
| **API frozen after Week 2** | Public method signatures locked; internal impl can change during Weeks 3–5 |

---

## Current Blockers (4 Questions for Mentor)

**These must be resolved before Week 3 implementation begins.**

### 🔴 Critical (blocks code)

#### Q1: Which DORA commit should I pin?
- **Why:** `MockEventStream` needs the real `Event` type; `NodeHarness::new()` needs `init_testing()`.
- **Without it:** Can't replace `()` placeholders with real types.
- **What to ask:** 
  > "Which DORA commit SHA or release tag should I lock in Cargo.toml? (e.g., commit `abc1234` or v0.3.0)"

#### Q2: What is `init_testing()` signature?
- **Why:** Core API entry point; need exact parameters and return type.
- **Without it:** Can't implement `NodeHarness::new()`.
- **What to ask:** 
  > "Can you share the signature of `DoraNode::init_testing()` or point me to its implementation? (parameters, return type, any examples?)"

### 🟡 Optional (can clarify while coding)

#### Q3: Does `EventStream` have a trait?
- **Why:** Determines whether mock and real versions can be mutually swappable.
- **What to ask:** 
  > "Is there a Rust trait (e.g., `EventStreamTrait`) that both real and mock `EventStream` should implement?"

#### Q4: Where in dora monorepo?
- **Why:** Affects final directory structure (could be `dora/libraries/test-utils/` or `dora/crates/test-utils/`).
- **What to ask:** 
  > "When this crate merges upstream, which path in dora monorepo should it live at? Can I reorganize now or later?"

---

## Next Steps (Week 3–5)

### Week 3: MockEventStream Implementation
1. Confirm DORA commit with mentor (Q1)
2. Update `Cargo.toml` to add `dora-node-api` dependency
3. Replace `()` placeholders with real `dora_node_api::Event`
4. Implement `MockEventStream::recv()` to pull from mpsc
5. Write unit tests: send events via channel, verify `recv()` returns them

### Week 4: MockOutputSender Implementation
1. Implement `MockOutputSender::send()` to push `(output_id, ArrayData)` to channel
2. Implement `OutputCollector` to buffer and index outputs by ID
3. Write unit tests: send multiple outputs, verify collection by ID

### Week 5: NodeHarness + End-to-End Tests
1. Confirm `init_testing()` signature with mentor (Q2)
2. Implement `NodeHarness::new()` calling `DoraNode::init_testing()`
3. Implement `send_input()` / `recv_output()` / `tick()` / `run_to_completion()`
4. Write end-to-end test: inject Arrow data → tick → assert outputs

---

## Metrics

| Metric | Value |
|--------|-------|
| **Crate LOC** | ~250 (stubs only) |
| **Tests** | 3 passing smoke tests |
| **API methods stubbed** | 9 (5 on NodeHarness, 2 on MockEventStream, 2 on MockOutputSender/OutputCollector) |
| **Dependencies locked** | arrow=53, tokio, serde, serde_json |
| **CI jobs** | 4 (check, test, clippy, fmt) |
| **Deliverables on track** | ✅ 7/7 (API, mock types, crate scaffold, CI, tests, cargo mirror, API freeze) |

---

## Mentor Discussion Prep

**Recommended approach for next Weekly Sync:**

1. **Open GitHub Discussion** in "Weekly Sync" category
2. **Post the 4 questions above** (Q1 and Q2 marked 🔴 priority)
3. **Link to this summary** for full context
4. **Expected turnaround:** Next meeting or async reply within 2–3 days

**Once answers received:**
- Update `Cargo.toml` with confirmed commit/version
- Move Q1+Q2 to "resolved" section
- Begin Week 3 implementation immediately

---

**Document created:** June 7, 2026  
**Next review:** After mentor feedback (expected by June 9–10, 2026)
