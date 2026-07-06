# Week 3–4 进度汇报

**学生:** SunSunSun689
**导师:** bobdingAI
**分支:** `week3`（基于 `main`，无冲突）
**状态:** ✅ Week 3–5 原计划超前完成

---

## 一、总览：实际完成了什么

原 proposal 中 Week 3–5 的目标是「实现 NodeHarness、MockEventStream、MockOutputSender」。Mock 类型在 Week 2 已提前完成，NodeHarness 在 Week 3 完成核心实现、Week 4 完成补完和死锁修复。

### NodeHarness API（8 个方法，已冻结）

| 方法 | 工作量 | 说明 |
|------|--------|------|
| `new()` | Week 3 实现 | 调用 `DoraNode::init_testing()`，创建 flume input/output channel，启动 daemon 线程 |
| `send_input()` | Week 3 实现 | 运行时通过 live flume channel 注入合成事件 |
| `send_stop()` | Week 3 新增 | 便捷注入 Stop 事件 |
| `send_output()` | Week 3 实现，Week 4 加固 | 委托 `DoraNode::send_output()`；内部自动 close_input 防止死锁 |
| `tick()` | Week 3 实现 | 同步驱动事件循环（`#[test]`，非 tokio），收集输出到内部缓冲区 |
| `recv_output()` | Week 3 重写 | 按 output_id 取出捕获的输出（原 Week 2 是 stub） |
| `close_input()` | Week 4 新增 | 关闭输入通道，唤醒 daemon 线程 |
| `run_to_completion()` | Week 3 占位，Week 4 实现 | 批量跑完所有事件，自动注入 Stop、自动 close_input，返回 Vec<Event> |

### Mock 类型（Week 2 完成，后续稳定）

- `MockEventStream` — 3 个单元测试
- `MockOutputSender` / `OutputCollector` — 3 个单元测试

### E2E 测试（1 → 4 个）

| 测试 | 完成时间 |
|------|----------|
| `e2e_receive_input_and_stop` | Week 3 |
| `e2e_send_output_and_recv` | Week 4 |
| `e2e_run_to_completion_returns_events` | Week 4 |
| `e2e_full_pipeline_input_to_output` | Week 4 |

### CI 状态

```
fmt ✅  |  clippy ✅  |  13/13 tests passing
```

---

## 二、Week 3 vs Week 4：各自做了什么

### Week 3（June 9）— 从空壳到实物

Week 2 结束时的 `harness.rs` 只有 **117 行**，3 个 stub 方法（参数名带 `_` 前缀，全是假的）：

```rust
// Week 2 的 harness —— 一个空壳
pub fn new() -> Self { ... }                                    // 假实现
pub fn send_input<I: Into<String>>(&mut self, _input_id: I,    // 参数未使用
                                    _data: ArrayData) { }       // 空函数体
pub fn recv_output<O: Into<String>>(&mut self, _output_id: O)  // 假实现
    -> Option<Vec<ArrayData>> { None }
```

Week 3 把 **117 行空壳 → 250 行完整实现**，全部 7 个方法 + 1 个 helper 一次性写完：

| 从 | 到 |
|----|----|
| `new() -> Self`（假） | `new() -> Result<Self, NodeError>`（真实 `DoraNode::init_testing`，flume channel 双向接通） |
| `send_input(_, _)`（空函数体） | `send_input(TimedIncomingEvent)`（flume channel `.send()` 运行时注入） |
| （不存在） | `send_stop()` |
| （不存在） | `send_output(id, ArrayData) -> Result<(), NodeError>` |
| （不存在） | `tick() -> Option<Event>`（同步 `EventStream::recv()` + 输出收集） |
| `recv_output(_)`（永远返回 None） | `recv_output(id) -> Option<Vec<Map>>`（真实输出 buffer drain） |
| （不存在） | `run_to_completion()`（`todo!()` 占位，Week 4 补完） |
| （不存在） | `collect_pending_outputs()`（私有 helper） |

同时修复了 3 个 bug（拼写错误、`NodeError::Init` → `NodeError::Output`、移除 `.parse().unwrap()` panic）。

### Week 4（June 14）— 补完 + 死锁修复

Week 3 的 harness 能用但有两个设计级别的缺陷。Week 4 补齐了最后一块拼图：

**`run_to_completion()` 从 `todo!()` 变为真实实现：**

```rust
pub fn run_to_completion(&mut self) -> Vec<Event> {
    self.send_stop();           // 自动注入 Stop —— 调用方忘加也不会 hang
    let mut events = Vec::new();
    while let Some(event) = self.tick() {
        // ... 收集事件直到 Stop/InputClosed/stream 耗尽
    }
    self.close_input();         // 唤醒 daemon 线程
    events
}
```

**修复两个死锁 bug（10 角度 code review 发现并确认）：**

| Bug | 触发条件 | 修复 |
|-----|----------|------|
| `send_output()` 死锁 | 直接调用 `send_output()` 而不先调 `close_input()` | `send_output()` 内部自动调 `close_input()` |
| `run_to_completion()` 永久挂起 | 忘记预加载 Stop 事件 | 方法开头自动注入 `send_stop()` |

**三个新 E2E 测试**，覆盖输出管道、批量模式、全管道。

---

## 三、架构说明

```
┌──────────────────┐  flume channel (input)  ┌──────────────────┐
│   Test code      │ ──────────────────────▶ │  DORA node       │
│  send_input()    │                         │  (被测试的节点)   │
│  tick()          │                         │                  │
│  recv_output() ◀─│── flume channel (output)─│                  │
└──────────────────┘                         └──────────────────┘
```

三线程协作：
- **Test（主线程）**：调用 send_input / tick / send_output / recv_output
- **Event stream 线程**：向 daemon 发送 `NextEvent` 请求，转发回复
- **Daemon 线程**：逐条处理请求，从 flume 读输入、向 flume 写输出

生命周期：
1. **Input phase**：`send_input()` / `send_stop()` + `tick()` — 注入并驱动事件
2. **Completion**：`run_to_completion()` 或 `close_input()` — 关闭输入通道，唤醒 daemon
3. **Output phase**：`send_output()`（自动关闭输入）+ `recv_output()` — 发送和断言输出

---

## 四、上游 DORA 改动（vendored）

为了让 NodeHarness 能在运行时动态注入事件（而非构造函数时一次性写死），在 vendored dora 源码中做了最小改动：

- `integration_testing.rs`：新增 `TestingInput::Channel(flume::Receiver<TimedIncomingEvent>)` 变体
- `node_integration_testing.rs`：新增 `EventSource` 枚举支持 Channel 路径；提取 `check_poisoned()`

当前 `Cargo.toml` 临时使用 `path = "dora/apis/rust/node"`。

---

## 五、技术指标

| 项目 | 详情 |
|------|------|
| 分支 | `week3` |
| 测试 | 13/13 通过（6 单元 + 3 smoke + 4 E2E） |
| 测试方式 | `#[test]`（同步），因为 `init_testing` 内部用 `blocking_recv`，无法在 tokio runtime 运行 |
| vendored dora 改动 | 2 个文件 |
| dora commit pin | `45436aad` |
| Arrow 版本 | 58 |
| CI | fmt ✅ / clippy ✅ / 13 tests ✅ |

---

## 六、需要 Mentor 确认

### Q1: `TestingInput::Channel` 上游合入

`Channel` 变体的设计方向 OK 吗？需要我向 dora-rs 上游提 PR，还是 mentor 那边处理？在上游合入前，vendored 方式是否可以暂时接受？

### Q2: Week 5 开始 TestSourceNode / TestSinkNode？

原计划 Week 5 的二进制开发现在可以开始：
- `TestSourceNode`（`src/bin/test_source.rs`）：从文件/内联 JSON 读取 Arrow 数据，emit 到指定 output
- `TestSinkNode`（`src/bin/test_sink.rs`）：接收 input 与预期文件比对，exit code 0/1

还是希望放慢节奏，做更多测试覆盖？

---

## 七、下一步

等 mentor 对 Q1/Q2 回复后，按确认方向推进 Week 5。届时需处理 vendored dora 依赖。

🤖 Generated with [Claude Code](https://claude.com/claude-code)
