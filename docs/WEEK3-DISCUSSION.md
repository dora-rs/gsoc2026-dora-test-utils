# Week 3–4 Discussion — NodeHarness 核心实现完成

> 待提交到 GitHub Discussions · Category: `Weekly Sync`

---

## 实际完成情况

原计划 Week 3–5 完成 NodeHarness + MockEventStream + MockOutputSender 实现。
Mock 类型在 Week 2 已提前完成，NodeHarness 在 Week 3–4 全部完成，超前于原计划。

| 原计划 | 实际完成 | 状态 |
|--------|----------|------|
| Week 3: MockEventStream 实现 | Week 2 已完成 | ✅ |
| Week 4: MockOutputSender 实现 | Week 2 已完成 | ✅ |
| Week 5: NodeHarness + E2E 测试 | **Week 3–4 已完成** | ✅ |

---

## Week 3 交付（June 9）

- ✅ **`NodeHarness::send_output()`** — 委托给 `DoraNode::send_output`，返回 `Result<(), NodeError>`，不 panic
- ✅ **E2E 测试** — `tests/e2e.rs`: send_input(Input) + send_stop → tick×2 → 验证收到 Input + Stop 事件
- ✅ **已修复 Bug：**
  - 拼写错误：`Convience` → `Convenience`
  - `send_output` 中 `NodeError::Init` → `NodeError::Output`（无效 output_id）
  - `send_output` 移除 `.parse().unwrap()` panic，改为返回 `Result`
- ✅ **CI 全绿**: fmt ✅ | clippy ✅ | 10/10 tests ✅

## Week 4 交付（June 14）

- ✅ **`NodeHarness::close_input()`** — 新增方法，drop input sender 唤醒 daemon 线程
- ✅ **`NodeHarness::run_to_completion()`** — 批量模式：
  - 循环 `tick()` 直到 Stop / InputClosed / stream 耗尽
  - 自动注入 `send_stop()` 保证终止（调用方忘加也不会 hang）
  - 自动调 `close_input()` 后返回 `Vec<Event>`
- ✅ **`send_output` 死锁修复** — `send_output()` 内部自动调 `close_input()`，调用方无需手动管理
- ✅ **E2E 测试扩展（1 → 4 个）**：
  - `e2e_send_output_and_recv` — 纯输出管道
  - `e2e_run_to_completion_returns_events` — 批量模式 + 输出后断言
  - `e2e_full_pipeline_input_to_output` — 全管道：输入 → 完成 → 输出 → 取出验证
- ✅ **Code Review 发现并修复 3 个 bug**（10 角度 × 8 候选，15 条发现）
- ✅ **`#[allow(dead_code)]` 移除** — `node` 字段在 `send_output()` 中实际被使用
- ✅ **CI 全绿**: fmt ✅ | clippy ✅ | 13/13 tests ✅

---

## NodeHarness API（8 个方法，已冻结）

| 方法 | 说明 |
|------|------|
| `new()` | 创建 harness，调用 `DoraNode::init_testing()`，接通 live flume channel |
| `send_input(TimedIncomingEvent)` | 运行时通过 flume channel 注入合成事件 |
| `send_stop()` | 便捷注入 Stop 事件 |
| `send_output(id, Array)` | 发送输出；内部自动 close_input 防止死锁，返回 `Result` |
| `tick() -> Option<Event>` | 同步驱动事件循环，收集输出到内部缓冲区（`#[test]`，非 tokio） |
| `recv_output(id) -> Option<Vec<Map>>` | 按 output_id 取出捕获的输出 |
| `close_input()` | 关闭输入通道，唤醒 daemon 线程（通常无需手动调） |
| `run_to_completion() -> Vec<Event>` | 批量跑完所有事件，自动注入 Stop、自动 close_input |

---

## 架构

```
┌──────────────────┐  flume channel (input)  ┌──────────────────┐
│   Test code      │ ──────────────────────▶ │  DORA node       │
│  send_input()    │                         │  (the thing      │
│  tick()          │                         │   under test)    │
│  recv_output() ◀─│── flume channel (output)─│                  │
└──────────────────┘                         └──────────────────┘
```

三线程：
- **Test（主线程）** — 调用 send_input、tick、send_output、recv_output
- **Event stream 线程** — 向 daemon 发送 `NextEvent` 请求，转发回复
- **Daemon 线程** — 逐条处理请求：从 flume 读输入，向 flume 写输出

生命周期：
1. **Input phase** — `send_input()` / `send_stop()` + `tick()`
2. **Completion** — `run_to_completion()` 或 `close_input()` 关闭输入通道
3. **Output phase** — `send_output()`（自动关闭输入）+ `recv_output()`

---

## Vendored DORA 改动

为了让 NodeHarness 在运行时动态注入事件（而非构造函数时一次性写死），在 vendored dora 源码中做了最小改动：

**`integration_testing.rs`** — 新增枚举变体：
```rust
pub enum TestingInput {
    FromJsonFile(PathBuf),
    Input(IntegrationTestInput),
    Channel(flume::Receiver<TimedIncomingEvent>),  // ← 新增
}
```

**`node_integration_testing.rs`** — 重构事件源：
```rust
enum EventSource {
    Vec(std::vec::IntoIter<TimedIncomingEvent>),   // 原有
    Channel(flume::Receiver<TimedIncomingEvent>),  // 新增
}
```

同时将 `check_poisoned()` 提取为独立方法（避免 Channel 路径重复代码）。

目前 `Cargo.toml` 使用 `path = "dora/apis/rust/node"` 临时指向 vendored 源码。

---

## 🔴 需要 Mentor 确认的问题

### Q1: `TestingInput::Channel` 的上游合入策略

- 这个 `Channel` 变体的设计方向 OK 吗？
- 是否需要我向 dora-rs 上游提 PR 加入这个变体？还是 mentor 那边处理？
- 在上游合入之前，vendored 的方式是否可以暂时接受？

### Q2: Week 5 提前开始 TestSourceNode / TestSinkNode？

原计划 Week 5 的二进制开发现在可以开始：
- `TestSourceNode` 二进制（`src/bin/test_source.rs`）：从文件/内联读取 Arrow JSON，emit 到指定 output
- `TestSinkNode` 二进制（`src/bin/test_sink.rs`）：接收 input，与预期文件比对，exit code 0/1

还是希望放慢节奏、在 Week 4 做更多 polish/测试覆盖？

---

## 📊 技术摘要

| 项目 | 详情 |
|------|------|
| 分支 | `week3`（基于 `main`，无冲突） |
| 测试 | 13/13 通过（6 单元 + 3 smoke + 4 E2E） |
| 测试方式 | `#[test]`（同步），因为 `init_testing` 内部用 `std::thread::spawn` + `blocking_recv` |
| vendored dora 改动 | 2 个文件 |
| dora commit pin | `45436aad` |
| Arrow 版本 | 58（与 dora main 一致） |
| PR 状态 | Week 2 PR (#18) 待 mentor 审阅合并；Week 3–4 改动在 `week3` 分支，待 PR |

---

## 下一步

等 mentor 对 Q1–Q2 的回复后，按确认的方向推进 Week 5。届时需要决定如何处理 vendored dora 依赖。

