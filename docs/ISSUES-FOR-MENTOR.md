# Issues for Mentor Review — Week 8

Issues to discuss with mentor (bobdingAI) at the next weekly sync.

---

## Issue 1: flume 0.10 spinlock causes permanent deadlock on 2-vCPU CI runners

**Severity**: 🔴 Critical  
**Status**: Workaround in place, needs upstream fix  
**Labels**: `bug`, `ci`, `upstream`

### Symptoms

`cargo test` job on GitHub Actions (2 vCPU) permanently deadlocks, runs for 6 hours, then gets killed. On local dev machines (6+ cores), the deadlock is intermittent (~30% pass rate). On CI, it's near-100%.

### Root Cause

`TestingInput::Channel` uses `flume::Receiver` (flume 0.10.x). flume 0.10 uses a spinlock internally for synchronization. On a preemptive kernel, the spinlock holder (daemon thread) can be preempted. When the main thread tries `Sender::drop()`, it spins waiting for the lock, but the lock holder never gets scheduled back — permanent deadlock.

This is a known dora upstream issue: `dora-rs/dora#1603`. The dora event stream already switched to `tokio::sync::mpsc` to avoid this, but `TestingInput::Channel` still uses flume.

### Current Workaround

Three-layer defense:

1. **Code**: `send_input()` calls `yield_now()` + `Drop` sleeps 500ms
2. **CI**: retry ×5 with `timeout 120s` per attempt
3. **CI**: harness tests run with `continue-on-error: true`, core tests use `--skip harness`

Full details in `docs/CI-DEADLOCK-FIX.md`.

### Recommended Fix

Dora upstream should replace `TestingInput::Channel(flume::Receiver)` with `TestingInput::Channel(tokio::sync::mpsc::Receiver)`, matching the event stream migration already done.

### Impact of Current Workaround

| Aspect | Status |
|--------|--------|
| Core tests (sink, source, traits, mock) | ✅ Run reliably on CI |
| Harness unit tests (3 tests) | ⚠️ Skipped from main run; retry×5 with `continue-on-error: true` |
| E2E tests (5 tests) | ⚠️ Same as harness — retried separately, failures tolerated |
| Integration tests | ✅ Run serially, unaffected |
| Local development | ⚠️ ~77% pass rate for e2e; `--test-threads=1` required |

The workaround is functional but brittle: harness/e2e test failures on CI are silently tolerated, and the root cause (flume spinlock) will affect any future tests that use `TestingInput::Channel`.

### Discussion Point for Mentor

> **Should I open a PR against `dora-rs/dora` to migrate `TestingInput::Channel` from `flume::Receiver` to `tokio::sync::mpsc::Receiver`?**
>
> Considerations:
> - Pro: eliminates the deadlock at the source; harness tests can run reliably on CI without workarounds; aligns with the event stream migration already done in dora
> - Con: touches upstream dora code; needs review from dora maintainers; may have merge timeline uncertainty
> - Alternative: wait for a planned flume 0.11 migration (if one exists)
>
> If the mentor approves, this can be scoped as a Week 9–10 task alongside example pipelines.

---

## Issue 2: Integration tests silently pass (green) when dora CLI is not on PATH

**Severity**: 🟡 Medium  
**Status**: Open  
**Labels**: `bug`, `testing`, `dx`

### Symptoms

All tests in `tests/integration.rs` silently pass (green) when the `dora` CLI binary is not available on `PATH`:

```rust
fn echo_pipeline_exact_match_int64() {
    if !dora_available() {
        eprintln!("SKIP: dora CLI not found on PATH");
        return;  // Silent return — test shows as "passed"
    }
    // ... actual assertions never execute
}
```

A developer running `cargo test` on a machine without dora installed sees all integration tests pass and assumes the pipeline is verified. In reality, zero assertions ran.

### Recommended Fix

Option A: Use `#[ignore]` on tests that require dora CLI, and run them explicitly on CI.  
Option B: `panic!` or `assert!` when `dora_available()` returns false outside of CI (detect via env var).

---

## Issue 3: [FIXED] Multi-output test-source created separate DoraNode per output

**Severity**: 🔴 Critical (was)  
**Status**: ✅ Fixed in commit `5e73228`  
**Labels**: `bug`, `fixed`, `documentation`

### Problem

The original refactored `run_test_source` called `emit_output()` for each `OutputSpec`, and `emit_output()` called `DoraNode::init_from_env()` internally. For N `--output` arguments, this created N independent daemon connections, each sending `Register`/`OutputsDone`. The daemon would reject duplicate `Register` messages, causing all outputs after the first to fail silently.

### Fix

Restructured to create **one** `DoraNode` shared across all `OutputSpec`s:

```rust
pub fn run_test_source(config: SourceConfig) -> Result<()> {
    validate_all_specs(&config.outputs)?;
    let (mut node, _events) = DoraNode::init_from_env()?;
    for spec in &config.outputs {
        emit_output(&mut node, spec)?;
    }
    Ok(())
}
```

Added `validate_spec()` for early validation before touching the daemon (unit-test safe).

---

## Issue 4: Binary naming inconsistency (`test_source` vs `test-source`)

**Severity**: 🟡 Medium  
**Status**: ✅ Fixed in commit `5e73228`  
**Labels**: `bug`, `fixed`

### Problem

Cargo.toml declared `name = "test-source"` (hyphen) but `build_binaries()` in integration tests used `--bin test_source` (underscore). On a clean CI build, `cargo build --bin test_source` would fail because no target with that name exists.

### Fix

Unified to `test-source` (hyphen) everywhere: Cargo.toml, `build_binaries()`, `bin_path()`, and fixture YAML files.

---

## Issue 5: Missing `--inline-data` support in refactored test-source

**Severity**: 🟡 Medium  
**Status**: ✅ Fixed in commit `5e73228`  
**Labels**: `bug`, `fixed`, `regression`

### Problem

The multi-output refactor removed the `--inline-data` CLI flag. Scripts and ad-hoc usage that passed JSON inline (`--inline-data '{"data": [1,2]}'`) would fail with an unknown argument error. The `lib.rs` documentation still referenced the flag.

### Fix

Restored `--inline-data` as an alternative to `--data-file` in the backward-compatible single-output path.

---

## Summary for Mentor

| # | Issue | Severity | Status |
|---|-------|----------|--------|
| 1 | flume 0.10 spinlock CI deadlock | 🔴 Critical | 🔔 **Needs mentor decision** — upstream PR? |
| 2 | Integration tests silently skip | 🟡 Medium | Open — mentor input wanted |
| 3 | Multiple DoraNode per output | 🔴 Critical (was) | ✅ Fixed |
| 4 | Binary naming inconsistency | 🟡 Medium | ✅ Fixed |
| 5 | Missing --inline-data | 🟡 Medium | ✅ Fixed |

**🔔 Decision needed (Issue 1)**: Should we file a PR against `dora-rs/dora` to migrate `TestingInput::Channel` from flume to `tokio::sync::mpsc`? Or is there a planned flume 0.11 migration that would address this? If approved, this can be scoped as a Week 9–10 task.
