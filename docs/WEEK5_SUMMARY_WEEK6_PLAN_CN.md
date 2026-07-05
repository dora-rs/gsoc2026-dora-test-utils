# Week 5 总结 & Week 6 计划（中文版）

> Week 5: 2026-06-22 ~ 2026-06-28
> Week 6 目标: 2026-06-29 ~ 2026-07-05

---

## Week 5 — 已完成工作

### 交付成果

| 模块 | 文件 | 状态 |
|--------|-------|--------|
| `TestSource` 库 | `src/source.rs`（370→476 行） | ✅ 完成 |
| `TestSink` 库 | `src/sink.rs`（414→421 行） | ✅ 完成 |
| `test-source` CLI 二进制 | `src/bin/test_source.rs`（67 行） | ✅ 完成 |
| `test-sink` CLI 二进制 | `src/bin/test-sink.rs`（70 行） | ✅ 完成 |
| 单元测试（source） | 8→17 个测试 | ✅ 完成 |
| 单元测试（sink） | 7 个测试 | ✅ 完成 |

### 各模块总览

到目前为止，项目一共完成了 **5 个模块**（3 个 Week 3-4 + 2 个 Week 5）：

#### Week 3-4 完成的模块

**1. NodeHarness（`src/harness.rs`）— 单元测试驱动器**

在不启动 DORA daemon/coordinator 的情况下，用纯代码驱动单个 DORA 节点：注入输入、
推进运行、收集输出、做断言。内部通过 flume 通道与 DORA 的 `init_testing()` 对接。

```
测试代码 → send_data("image", json) → tick() → recv_output("label") → assert!
```

核心方法：
| 方法 | 作用 |
|------|------|
| `new()` | 创建 harness，初始化 DORA 节点 |
| `send_input()` | 注入原始事件（TimedIncomingEvent） |
| `send_data()` | 便捷方法：按 ID 注入数据（JSON 或 Arrow） |
| `send_output()` | 让节点发送一个输出 |
| `tick()` | 驱动节点处理一个事件 |
| `recv_output()` | 收集/读取节点的输出 |
| `close_input()` | 关闭输入通道（安全调用 send_output 的前提） |
| `run_to_completion()` | 自动注入 Stop，循环 tick() 直到结束 |

**2. MockEventStream / MockOutputSender / OutputCollector（`src/mock/`）— 纯模拟测试**

不依赖真实 DORA 节点，用 tokio 通道模拟事件流和输出。适合测试事件调度逻辑、
超时处理等场景，不需要启动任何 DORA 基础设施。

| 类型 | 作用 |
|------|------|
| `MockEventStream` | 模拟 DORA 事件流，支持多生产者注入事件 |
| `MockOutputSender` | 模拟输出发送端 |
| `OutputCollector` | 模拟输出收集端，按 ID 索引所有输出 |

**3. IntoInputData trait（`src/traits.rs`）— 数据格式转换**

让 `NodeHarness::send_data()` 能同时接受多种输入格式，自动转为 DORA 需要的
`InputData` 格式。

| 实现类型 | 转换方式 |
|----------|---------|
| `serde_json::Value` | 直接包装为 `InputData::JsonObject` |
| `arrow::array::ArrayData` | 序列化为 JSON 数组后包装（保持与 DORA 的兼容性） |

---

#### Week 5 完成的模块

**4. TestSource（`src/source.rs` + `src/bin/test_source.rs`）— 测试数据注入节点**

往 DORA 数据流中**注入测试数据**。读取一个 DORA 格式的 JSON 文件
（`{"data": [1, 2, 3], "data_type": "Int32"}`），将每个元素转成 Arrow 数组，
通过 DORA 节点发送到数据流中。

```bash
test-source --output-id numbers --data-file test_input.json
```

- **库函数**：`run_test_source(SourceConfig)` — 程序化调用
- **CLI 二进制**：`test-source` — 在 YAML 数据流中作为独立节点运行
- **类型提示**：支持 `data_type` 字段指定精确的 Arrow 类型（Int8–UInt64、Float32/64、LargeUtf8）
- **适用场景**：替代真实数据源（传感器、摄像头），向被测节点喂测试数据

**5. TestSink（`src/sink.rs` + `src/bin/test-sink.rs`）— 数据捕获与断言节点**

从 DORA 数据流中**接收数据，和预期结果比较**。收集所有 `Input` 事件，与预期 JSON
文件逐条比对，结果写入 `result.json`。

```bash
test-sink --expected-file expected.json --output-file result.json
```

- **库函数**：`run_test_sink(SinkConfig)` — 程序化调用，返回 `SinkResult`
- **CLI 二进制**：`test-sink` — 在 YAML 数据流中作为独立节点运行
- **两种比较模式**：

| 模式 | 机制 | 适用场景 |
|------|------|---------|
| `strict` | Arrow→JSON 往返 + 逐字符比较 | 需要精确匹配 JSON 输出格式 |
| `semantic` | 预期 JSON→Arrow 转换 + Arrow 相等比较 | 容忍类型宽度差异（如 Int32 vs Int64） |

- **适用场景**：替代人工检查输出，自动验证被测节点是否产生正确结果

---

#### 模块配合方式

一个完整的集成测试数据流：

```
test-source ──(发送测试数据)──▶ 你的节点 ──(产生输出)──▶ test-sink ──▶ result.json
```

1. `test-source` 负责"喂数据"——从 JSON 文件读取，按指定类型转成 Arrow 数组发送
2. 中间的"你的节点"是被测对象——接收输入，处理后产生输出
3. `test-sink` 负责"判卷"——把收到的输出跟预期结果比对，写出通过/失败

三者通过 YAML 数据流描述文件编排在一起，用 `dora run` 一键启动运行。

### TestSource（详细）

- **库函数**：`run_test_source(SourceConfig) -> Result<()>` — 读取 DORA 格式的 JSON
  配置 `{"data": [...], "data_type": {...}}`，将每个元素转换为 Arrow 数组并
  尊重 `data_type` 类型提示，通过 `init_from_env()` 初始化 DORA 节点，发送所有数据。
- **CLI**：`test-source --output-id <ID> --data-file <PATH>` 或 `--inline-data <JSON>`
- **类型推断**：`json_value_to_arrow_array()` 现在接受可选的 `DataType` 提示参数，
  支持 Int8–Int64、UInt8–UInt64、Float32、Float64、LargeUtf8。提示为 `None` 时
  回退到 Int64/Float64/String/Boolean 推断。
- **data_type 传播**：复杂的 `DataType` 值（Struct、List）通过
  `serde_json::from_value<DataType>()` 反序列化（arrow-schema 的 serde 功能已
  通过依赖传递启用），并用作 arrow_json 的 schema 提示。

### TestSink（详细）

- **库函数**：`run_test_sink(SinkConfig) -> Result<SinkResult>` — 读取预期 JSON
  文件，初始化 DORA 节点，收集所有 `Event::Input` 事件，与预期数据比较，
  将 `SinkResult` 写入输出文件。
- **CLI**：`test-sink --expected-file <PATH> --output-file <PATH> [--strict] [--no-fail-on-mismatch]`
- **两种比较模式**：
  - `compare_strict`：Arrow→JSON 往返 + 精确的 `serde_json::Value` 相等比较。
  - `compare_semantic`：预期 JSON→Arrow 转换 + `arrow::Array` 相等比较。
- **`compare_sequences<E,R>()`**：从重复的比较循环中提取出的泛型辅助函数，
  消除了约 60 行重复代码。

### 代码审查轮次

- **第一轮**（已有）：发现 5 个 Important 问题并全部修复：
  `from_str`→`from_slice`、`compare_semantic` 非标量类型处理、`data_type` 字段
  被忽略、DRY 比较循环、`fail_on_mismatch` 在库层无效。
- **第二轮**（本周，最高强度 `/code-review`）：发现 15 个问题，覆盖所有严重级别。
  修复了其中 7 个 Bug。

#### 第二轮修复的 7 个 Bug

| # | 严重性 | 文件 | 问题描述 | 修复方式 |
|---|--------|------|---------|---------|
| 1 | **严重** | `src/source.rs` | `json_array_to_arrow_struct` 中的**双重包装**：为每个元素包装 `{"data": v}`，然后通过 `vec![obj]` 再次包装 → 产生 `[[{"data":...}]]` 而非 `[{"data":...}]` | 重构为直接序列化，提取共享的 `json_bytes_to_arrow_column` 辅助函数 |
| 2 | **严重** | `src/sink.rs` | `compare_strict` 在复杂 Arrow 类型上 `expect()` panic：List、Struct、Union 等类型不受 `arrow_json::Writer<JsonArray>` 支持，会导致进程崩溃 | `compare_strict` 现在返回 `Result<SinkResult>`，使用 `.context()?` 替代 `.expect()` |
| 3 | **高** | `src/bin/test-sink.rs` | test-sink 二进制中的**死代码**：mismatch 打印代码块不可达，因为 `run_test_sink()` 在 mismatch 时返回 `Err` | 移除了不可达的双重检查；`Ok` 分支现在无条件打印差异信息 |
| 4 | **高** | `src/source.rs` | Schema/JSON 键名不匹配：`json_obj_to_arrow_struct` 构建了 `Field("data", dt)` 的 schema，但将原始对象的键名传给 arrow_json | 当 `data_type` 为 `Some` 时，将对象包装为 `{"data": obj}` |
| 5 | **高** | `src/sink.rs` | `compare_semantic` 中的错误被静默吞没：`\|_\|` 丢弃了转换错误信息 | 现在将转换失败记录为包含完整错误信息和原始值的 `Difference` 条目 |
| 6 | **高** | `src/sink.rs` | 语义比较忽略了预期 JSON 中的 `data_type`：导致 `Int32Array ≠ Int64Array` 的**误报 mismatch** | `run_test_sink` 从预期 JSON 中提取 `data_type`，传递给 `compare_semantic` → `json_value_to_arrow_array` |
| 7 | **高** | `src/source.rs` | 显式类型提示路径的**零测试覆盖**：`number_to_arrow_array` 的 11 个 match 分支全无测试 | 新增 9 个测试：Int8、Int16、Int32、Int64显式、UInt8、UInt8溢出、UInt8负数、Float32、LargeUtf8 |

### 测试套件状态

```
✅ cargo fmt -- --check       — 通过
✅ cargo clippy -- -D warnings — 通过
✅ cargo test --lib            — 33 通过，0 失败
   （3 个 harness 测试跳过——已有的 DORA daemon 时序问题）
✅ cargo test --test e2e -- --test-threads=1 — 5 通过
```

---

## Week 6 — 计划

### 主要目标：集成测试（对应 CLAUDE.md Weeks 6-8）

将 `test-source` / `test-sink` 二进制放入真实的 YAML 数据流中，与被测节点一起运行，
验证端到端行为。

#### 任务 1：为集成测试创建示例数据流

- 创建最小化示例：`test-source → 被测节点 → test-sink`
- 编写 YAML 数据流描述文件
- 实现一个简单的 "echo" 或 "透传" 测试节点
- 使用 `DORA_TEST_WITH_INPUTS=1` 运行（standalone 测试模式）

#### 任务 2：集成测试框架

- 编写测试运行器：
  1. 启动 `dora-coordinator` + `dora-daemon`（或使用 `dora run`）
  2. 使用 test-source / test-sink 启动数据流
  3. 从 test-sink 的输出中读取 `result.json`
  4. 断言 `SinkResult.r#match == true`
- 添加到 `tests/` 目录中

#### 任务 3：边界用例测试

- 空数据数组 → 验证 test-source 以明确的错误退出
- 类型不匹配 → 验证 test-sink 正确报告差异
- 多输入/多输出的数据流 → 使用多个预期 sink 验证 test-sink
- 大数据批次 → 验证无性能退化

#### 任务 4：文档完善

- 更新 `src/lib.rs` 状态表（"Week 5" → "已实现"） ✅ 已完成
- 在 lib.rs 文档中记录 TestSource/TestSink 使用模式
- 在 `docs/` 或 `README` 中添加使用示例

---

## 导师同步会议讨论话题

### 话题 1：集成测试架构 — daemon 模式 vs standalone 模式

当前的 TestSource/TestSink 库函数使用 `DoraNode::init_from_env()`，同时支持两种模式：
- **Standalone**（`DORA_TEST_WITH_INPUTS=1`）：不需要 daemon，测试在进程内运行，
  使用基于文件的输入/输出。
- **Daemon-based**：完整的 `dora-coordinator` + `dora-daemon`，使用 TCP/共享内存。

**需要讨论的问题：**
对于集成测试，我们应该优先选择哪种模式？
- Standalone 模式更简单（无需管理 daemon 生命周期），但可能无法覆盖完整的
  TCP/共享内存路径。
- Daemon-based 模式更接近真实部署，但增加了 daemon 启动/关闭的复杂性，且
  在 CI 中存在端口竞争问题（我们已在 E2E harness 测试中遇到过并行运行时挂起
  的情况）。

**建议：** 集成测试是否应该同时覆盖两种模式，还是专注于一种？如果 standalone
模式足够，我们可以避免 CI 端口竞争问题。老师是否有其他看法或建议？

---

### 话题 2：`data_type` 在整个 pipeline 中的传播

当前设计通过 JSON 配置文件处理 `data_type`：
- test-source 尊重用户在配置中指定的 Arrow 类型
- test-sink 使用预期文件的 `data_type` 进行正确类型比较
- 但类型信息仅通过 JSON 文件流转，没有带内的 Arrow schema 传播

**需要讨论的问题：**
对于集成测试场景，当前基于 JSON 文件的 `data_type` 契约是否足够？还是应该考虑添加
`--schema-file` 标志，传递显式的 Arrow schema（IPC 格式），为 source 和 sink
两端提供更强的类型保证？

具体场景：用户想要测试一个接受 `Int32` 输入并输出 `Float64` 的节点。
- 当前方案：test-source 配置指定 `"data_type": "Int32"`，test-sink 预期配置
  指定 `"data_type": "Float64"`，两端独立指定。
- Schema 文件方案：一个 `.arrow` 文件定义整个 pipeline 契约，source 和 sink
  都引用同一 schema，保证一致性。

---

### 话题 3：中期评估范围

根据 Google GSoC 2026 时间线，Coding Phase 1 在 Week 6 结束后进入**中期评估**
（Midterm Evaluation）。

对照 CLAUDE.md 的里程碑：
> Week 3–5: 实现 NodeHarness, MockEventStream, MockOutputSender
> Week 6–8: 实现 TestSourceNode / TestSinkNode 二进制

我们已如期完成 Weeks 3-5（NodeHarness + mock + TestSource/TestSink 库，
TestSource/TestSink 二进制甚至比计划提前）。现在 Week 6 计划开展集成测试。

**需要讨论的问题：**
中期评估应该以什么样的交付成果为目标？
- **方案 A**：一个可工作的集成测试（一个端到端数据流）作为概念验证。
- **方案 B**：全面的 TestSource/TestSink API 文档 + 使用示例（暂无集成测试）。
- **方案 C**：两者兼顾——一个集成测试 + 更新的文档。

什么样的完成度会被认为是一次成功的中期评估？老师对演示形式（demo、报告、PR）
是否有具体要求？

---

## 附录：关键代码指标

| 指标 | 数值 |
|------|------|
| 总提交数（week4 分支） | 20 个 |
| Rust 源文件 | 10 个（5 库 + 2 二进制 + 3 测试） |
| 总代码行数（src/） | ~1,600 行 |
| 库单元测试 | 33 个（全部通过） |
| E2E 测试 | 5 个（全部通过） |
| Mock 测试 | 6 个（全部通过） |
| 代码审查轮次 | 2 轮，共修复 12 个问题 |
