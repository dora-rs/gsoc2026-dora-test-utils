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

### Next: Week 3–5

Implement the real logic:
- Wire up `dora-node-api` dependency (git or path)
- Implement `MockEventStream` with real `Event` type behind `mpsc`
- Implement `MockOutputSender` capturing `(String, ArrayData)` pairs
- Implement `NodeHarness` driving a real `DoraNode` via mock channels
- Write unit tests that actually tick a node

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
