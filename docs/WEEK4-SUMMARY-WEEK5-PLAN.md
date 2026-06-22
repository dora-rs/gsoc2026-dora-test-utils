# Week 4 Summary & Week 5 Plan

**Date:** 2026-06-22 | **Status:** Week 4 Complete, Week 5 Ready

---

## Week 4 Achievements

### 1. NodeHarness Completion (June 14)

| Method | Description | Status |
|--------|-------------|--------|
| `close_input()` | Drop input sender to unblock daemon thread | Implemented |
| `run_to_completion()` | Batch-run all events; auto-injects Stop; auto-closes input | Implemented |

**Deadlock fix:** `send_output()` now auto-calls `close_input()` before delegating to `DoraNode::send_output()`. The daemon thread is single-threaded — dropping the input sender unblocks it from `rx.recv()`, allowing `DaemonRequest::SendMessage` to be processed.

**API freeze:** All 8 public methods finalized. No further signature changes planned.

### 2. send_data() Convenience Method (June 21–22)

Closed the API gap between the proposal (`harness.send_input("image", arrow_data)`) and the verbose `send_input(TimedIncomingEvent { ... })`.

| Component | File | Description |
|-----------|------|-------------|
| `IntoInputData` trait | `src/traits.rs` (new) | Converts test data → `InputData` |
| `impl for serde_json::Value` | `src/traits.rs` | Wraps JSON in `InputData::JsonObject` |
| `impl for arrow::array::ArrayData` | `src/traits.rs` | Arrow → JSON → `InputData::JsonObject` |
| `NodeHarness::send_data()` | `src/harness.rs` | Convenience: `send_data(id, data)` → delegates to `send_input()` |

**Before (8 lines):**
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

**After (1 line):**
```rust
harness.send_data("numbers", serde_json::json!([1, 2, 3]));
```

### 3. Test Coverage

| Layer | Tests | Count |
|-------|-------|-------|
| Unit — `IntoInputData` trait | JSON value, Arrow int32, empty, string | 4 |
| Unit — `NodeHarness::send_data()` | JSON, Arrow, panic-after-close | 3 |
| E2E — `tests/e2e.rs` | Input pipeline, output path, run_to_completion, full pipeline, Arrow round-trip | 5 |
| Smoke — `tests/smoke.rs` | Harness construction, mock pairs | 3 |
| **Total** | | **21** (plus 6 mock unit tests = 27) |

**CI:** `cargo fmt` ✅ | `cargo clippy -- -D warnings` ✅ | 21/21 tests passing ✅

### 4. Code Review (max effort, 10 angles)

12 confirmed findings:
- **2 Critical:** Empty `ArrayData` deadlocks `tick()`; Arrow→JSON→Arrow round-trip loses type/structure
- **3 Important:** `clippy::len_zero`; `from_slice` optimization; `data_type:None` structural incompatibility
- **7 Minor:** Redundant `drop(writer)`, `.expect()` panics, Debug-format grep in tests, doc cross-reference gaps

### 5. Documentation

| Document | Purpose |
|----------|---------|
| `docs/superpowers/specs/2026-06-21-send-data-convenience-design.md` | Design spec |
| `docs/superpowers/plans/2026-06-21-send-data-convenience.md` | Implementation plan |
| `docs/WEEK3-4_SUMMARY.md` | Week 3–4 progress |
| `docs/WEEK3-4-MENTOR-REPORT.md` | Mentor report (Chinese) |
| `docs/WEEK3-DISCUSSION.md` | Discussion notes |

### 6. File Structure (end of Week 4)

```
src/
├── lib.rs              # Crate docs + status table + re-exports (IntoInputData)
├── harness.rs          # NodeHarness — 8 public methods
├── traits.rs           # IntoInputData trait + 2 impls (NEW)
└── mock/
    ├── mod.rs
    ├── event_stream.rs # MockEventStream
    └── output.rs       # MockOutputSender + OutputCollector
tests/
├── smoke.rs            # 3 smoke tests
└── e2e.rs              # 5 E2E tests
dora/                   # Vendored dora source (TestingInput::Channel patches)
docs/
├── PROGRESS.md
├── WEEK1-2_SUMMARY.md
├── WEEK3-DISCUSSION.md
├── WEEK3-4_SUMMARY.md
├── WEEK3-4-MENTOR-REPORT.md
├── WEEKLY_PLAN.md
├── proposal.pdf
└── superpowers/
    ├── specs/           # 3 design specs
    └── plans/           # 3 implementation plans
```

---

## Week 5 Plan

### 1. TestSourceNode Binary

**File:** `src/bin/test_source.rs` (new)

```
CLI interface:
  --output-id <ID>       Output identifier
  --data-file <PATH>     Arrow JSON file to emit
  --inline-data <JSON>   Inline JSON data to emit

Behavior:
  1. Parse CLI args
  2. Load data from file or inline JSON
  3. Spawn as DORA node in testing mode
  4. Emit loaded data on configured output
  5. Exit when data exhausted
```

### 2. TestSinkNode Binary

**File:** `src/bin/test_sink.rs` (new)

```
CLI interface:
  --expected-file <PATH>    Expected output file (Arrow JSON)
  --fail-on-mismatch        Exit non-zero on mismatch (default: true)

Behavior:
  1. Parse CLI args
  2. Receive inputs from dataflow
  3. Compare with expected (byte-for-byte or semantic)
  4. Write comparison result to result.json
  5. Exit code: 0 (match) / 1 (mismatch)
```

### 3. Week 4 Code Review Follow-up

| Priority | Item | Location |
|----------|------|----------|
| P0 | Guard against empty `ArrayData` in `IntoInputData` | `src/traits.rs:27` |
| P1 | Fix `clippy::len_zero` (4 locations) | harness, e2e |
| P1 | Replace `from_str` with `from_slice` | `src/traits.rs:52` |
| P2 | Remove redundant `drop(writer)` | `src/traits.rs:50` |
| P2 | Cross-reference `send_output`→`close_input` in `send_data` docs | `src/harness.rs:168` |
| P3 | Strengthen `should_panic` substring | `src/harness.rs:391` |

### 4. Metrics Target

| Checkpoint | Target | Status |
|-----------|--------|--------|
| Week 1–2: API design + scaffold | 7/7 deliverables | ✅ |
| Week 2: Mock types | 9/9 tests | ✅ |
| Week 3: NodeHarness core | 10/10 tests | ✅ |
| Week 4: NodeHarness completion | 13/13 tests | ✅ |
| Week 4: send_data() convenience | 21/21 tests | ✅ |
| **Week 5: TestSource + TestSink** | **2 binaries** | **⏳** |

---

## Week 5 计划

### 1. TestSourceNode 二进制

**文件:** `src/bin/test_source.rs`（新建）

```
CLI 接口：
  --output-id <ID>       输出标识符
  --data-file <PATH>     要发送的 Arrow JSON 文件
  --inline-data <JSON>   内联 JSON 数据

行为：
  1. 解析 CLI 参数
  2. 从文件或内联 JSON 加载数据
  3. 以测试模式启动 DORA 节点
  4. 在配置的输出上发送加载的数据
  5. 数据发送完毕后退出
```

### 2. TestSinkNode 二进制

**文件:** `src/bin/test_sink.rs`（新建）

```
CLI 接口：
  --expected-file <PATH>    预期的输出文件（Arrow JSON）
  --fail-on-mismatch        不匹配时返回非零退出码（默认：true）

行为：
  1. 解析 CLI 参数
  2. 从数据流接收输入
  3. 与预期值比对（逐字节或语义比对）
  4. 将比对结果写入 result.json
  5. 退出码：0（匹配）/ 1（不匹配）
```

### 3. Week 4 代码审查跟进

| 优先级 | 事项 | 位置 |
|--------|------|------|
| P0 | 在 `IntoInputData` 中拦截空 `ArrayData` | `src/traits.rs:27` |
| P1 | 修复 `clippy::len_zero`（4 处） | harness, e2e |
| P1 | 将 `from_str` 替换为 `from_slice` | `src/traits.rs:52` |
| P2 | 删除冗余 `drop(writer)` | `src/traits.rs:50` |
| P2 | 在 `send_data` 文档中交叉引用 `send_output`→`close_input` | `src/harness.rs:168` |
| P3 | 增强 `should_panic` 子串匹配 | `src/harness.rs:391` |

### 4. 进度指标

| 检查点 | 目标 | 状态 |
|--------|------|------|
| Week 1–2：API 设计 + 脚手架 | 7/7 交付物 | ✅ |
| Week 2：Mock 类型 | 9/9 测试 | ✅ |
| Week 3：NodeHarness 核心 | 10/10 测试 | ✅ |
| Week 4：NodeHarness 完成 | 13/13 测试 | ✅ |
| Week 4：send_data() 便捷方法 | 21/21 测试 | ✅ |
| **Week 5：TestSource + TestSink** | **2 个二进制** | **⏳** |
