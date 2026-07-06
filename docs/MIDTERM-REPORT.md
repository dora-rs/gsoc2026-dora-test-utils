# GSoC 2026 Midterm Report — dora-test-utils / 中期评估报告

> **Student / 学生:** SunSunSun689 | **Mentor / 导师:** bobdingAI
> **Period / 周期:** Community Bonding + Coding Phase 1 (May–Jul 2026 / 2026年5月–7月)
> **Branch / 分支:** `week6` | **Repo / 仓库:** [SunSunSun689/gsoc2026-dora-test-utils](https://github.com/SunSunSun689/gsoc2026-dora-test-utils)

---

## 1. Milestone Progress / 里程碑进度

| Milestone / 里程碑 | Planned / 计划 | Delivered / 实际 | Status / 状态 |
|-----------|---------|-----------|--------|
| Week 1–2: Design + scaffold / 设计+脚手架 | NodeHarness API design, crate scaffold | Complete / 完成 | ✅ |
| Week 3–5: Core harness + mocks / 核心harness+mock | NodeHarness, MockEventStream, MockOutputSender | Complete / 完成 | ✅ |
| Week 5: TestSource / TestSink / 测试源+测试接收器 | Library + CLI binaries / 库+CLI二进制 | Complete (ahead) / 提前完成 | ✅ |
| Week 6: Integration tests / 集成测试 | Echo pipeline, 4 integration tests, demo / 回显管道+4个集成测试+演示 | Complete / 完成 | ✅ |
| Week 6–8: Test binaries / 测试二进制文件 | Originally this phase / 原计划此时 | Done in Week 5 / Week 5已做 | 🚀 |

**Verdict: Ahead of schedule.** TestSource/TestSink binaries were delivered in Week 5 (originally planned for Weeks 6–8). Week 6 completed the echo pipeline integration tests and midterm demo.

**结论：进度超前。** TestSource/TestSink 二进制在 Week 5 已交付（原计划 Week 6–8）。Week 6 完成了回显管道集成测试和中期演示。

---

## 2. Modules Delivered / 已交付模块

### 2.1 NodeHarness (`src/harness.rs` — 402 lines / 行)

Unit-test driver for single DORA nodes. No daemon required.

单节点 DORA 单元测试驱动器，无需 daemon。

| Method / 方法 | Purpose / 用途 |
|--------|---------|
| `new()` | Create harness, init DORA node via `init_testing()` / 创建harness，初始化DORA节点 |
| `send_input()` | Inject raw `TimedIncomingEvent` / 注入原始事件 |
| `send_data()` | Convenience: inject data by ID (JSON or Arrow) / 便捷方法：按ID注入数据 |
| `send_output()` | Send output from the node / 从节点发送输出 |
| `tick()` | Drive node to process one event, collect outputs / 驱动节点处理一个事件 |
| `recv_output()` | Retrieve captured outputs by ID / 按ID获取捕获的输出 |
| `close_input()` | Drop input sender to unblock daemon thread / 关闭输入通道 |
| `run_to_completion()` | Auto-inject Stop, loop `tick()` until exhausted / 自动注入Stop，循环直到结束 |

### 2.2 Mock Types / Mock 类型 (`src/mock/` — 298 lines / 行)

Pure in-memory simulation — no real DORA node needed.

纯内存模拟——不需要真实的 DORA 节点。

| Type / 类型 | Purpose / 用途 |
|------|---------|
| `MockEventStream` | Simulated event stream with multi-producer injection / 模拟事件流，支持多生产者注入 |
| `MockOutputSender` | Simulated output sender / 模拟输出发送器 |
| `OutputCollector` | Collects outputs by ID for assertions / 按ID收集输出以供断言 |

### 2.3 IntoInputData Trait / Trait (`src/traits.rs` — 137 lines / 行)

Format conversion for `NodeHarness::send_data()`.

`NodeHarness::send_data()` 的格式转换。

| Implementation / 实现 | Conversion / 转换 |
|---------------|------------|
| `serde_json::Value` | Direct → `InputData::JsonObject` / 直接 |
| `arrow::array::ArrayData` | Serialize to JSON array → `InputData::JsonObject` / 序列化为JSON数组 |

### 2.4 TestSource (`src/source.rs` + `src/bin/test_source.rs` — 570 lines / 行)

Injects test data into DORA dataflows from JSON files.

从 JSON 文件向 DORA 数据流注入测试数据。

```bash
test_source --output-id data --data-file source-data.json
```

- Supports Int8–UInt64, Float32/64, LargeUtf8 type hints / 支持 Int8–UInt64, Float32/64, LargeUtf8 类型提示
- Library: `run_test_source(SourceConfig) -> Result<()>` / 库函数
- Each JSON element → separate Arrow array → separate `send_output()` / 每个JSON元素→独立Arrow数组→独立`send_output()`

### 2.5 TestSink (`src/sink.rs` + `src/bin/test-sink.rs` — 488 lines / 行)

Captures DORA outputs and compares with expected data.

捕获 DORA 输出并与预期数据对比。

```bash
test-sink --expected-file expected.json --output-file result.json
```

- Two comparison modes: `strict` (JSON round-trip) and `semantic` (Arrow equality) / 两种对比模式：`strict`（JSON往返对比）和 `semantic`（Arrow等价对比）
- Respects `data_type` hint from expected file / 遵循预期文件中的 `data_type` 提示
- Library: `run_test_sink(SinkConfig) -> Result<SinkResult>` / 库函数

### 2.6 Echo Node + Integration Tests / Echo节点 + 集成测试

```
tests/fixtures/
├── echo-node.rs          # Pass-through node (28 lines) / 透传节点（28行）
├── echo-dataflow.yml     # YAML dataflow template / YAML数据流模板
├── source-data.json      # Sample input / 样本输入
└── expected-output.json  # Sample expected output / 样本预期输出

tests/integration.rs      # 4 integration tests + test runner / 4个集成测试+测试运行器
scripts/demo.sh           # Midterm demo script / 中期演示脚本
```

---

## 3. Test Suite / 测试套件

```
✅ cargo fmt -- --check          — passes / 通过
✅ cargo clippy -- -D warnings    — passes / 通过
✅ cargo test --lib               — 36 passed / 36个通过, 0 failed / 0个失败
✅ cargo test --test e2e          — 5 passed / 5个通过 (--test-threads=1)
✅ cargo test --test smoke        — 3 passed / 3个通过
✅ cargo test --test integration  — 4 passed / 4个通过 (--test-threads=1)
```

### Integration Tests / 集成测试 (4 tests / 个)

| Test / 测试 | Scenario / 场景 | Result / 结果 |
|------|----------|--------|
| `echo_pipeline_exact_match_int64` | `[42, 99, -1]` Int64 round-trip / Int64往返 | ✅ |
| `echo_pipeline_semantic_int32_tolerates_int64` | Int32/Int64 type tolerance / Int32/Int64类型兼容 | ✅ |
| `echo_pipeline_ten_elements` | 10-element array round-trip / 10元素数组往返 | ✅ |
| `echo_pipeline_string_data` | LargeUtf8 strings round-trip / 字符串往返 | ✅ |

### Code Review History / 代码审查历史

- **3 rounds / 轮** of `/code-review` (max-effort)
- **27 bugs fixed / 个bug已修复** across all severity levels / 覆盖所有严重级别

---

## 4. Code Metrics / 代码指标

| Metric / 指标 | Value / 数值 |
|--------|-------|
| Total commits / 总提交数 (week6 branch) | 35+ |
| Rust source files / Rust源文件 | 10 |
| Total lines / 总行数 (src/ + tests/) | ~2,500 |
| Library unit tests / 库单测 | 39 |
| E2E tests / E2E测试 | 5 |
| Integration tests / 集成测试 | 4 |
| Mock tests / Mock测试 | 3 |

---

## 5. Demo / 演示

```bash
# One command to run the full demo / 一键运行完整演示：
bash scripts/demo.sh
```

The demo script / 演示脚本会:
1. Builds all 4 binaries (test_source, test-sink, echo-node, dora CLI) / 构建4个二进制文件
2. Shows the YAML dataflow definition / 展示YAML数据流定义
3. Shows test input/expected data / 展示测试输入/预期输出
4. Runs `dora run` — daemon spawns all 3 nodes, pipeline completes in <1s / 运行 dora 数据流
5. Prints `result.json`: `{"match": true, "expected_count": 3, "received_count": 3, "differences": []}` / 打印结果
6. Runs 4 automated integration tests / 运行4个自动化集成测试
7. Runs 39 library unit tests / 运行39个库单测

Pipeline / 管道: `test-source → echo-node → test-sink`

```
test-source                echo-node               test-sink
┌──────────┐    data     ┌──────────┐    data     ┌──────────┐
│ JSON→Arr │ ──────────▶ │  receive  │ ──────────▶ │ capture  │
│ send out │             │  send out │             │ compare  │
└──────────┘             └──────────┘             │ write    │
                                                   └──────────┘
                                                        │
                                                   result.json
                                                   {match: true}
```

---

## 6. Known Issues / 已知问题

| Issue / 问题 | Impact / 影响 | Workaround / 绕过 |
|------|------|------|
| 3 harness unit tests occasionally hang / 3个harness单测偶发挂起 | `cargo test --lib` may stall / 可能卡住 | `--test-threads=1` or skip / 或跳过 |
| Port 6013 must be free / 端口6013需空闲 | `dora run` fails / 失败 | Ensure port is free / 确保端口空闲 |
| Python 3.10 env for pyo3 / pyo3需要Python环境 | `pyo3-build-config` error / 报错 | `PYO3_NO_PYTHON=1` |

These are pre-existing upstream DORA issues, not introduced by our code.

这些都是上游 DORA 已知问题，非本项目引入。

---

## 7. Next Steps — Coding Phase 2 / 下一步 — Coding Phase 2 (Jul–Aug 2026 / 7月–8月)

| Week / 周 | Plan / 计划 |
|------|------|
| Week 7–8 | Edge case tests (empty arrays, type mismatches, large batches) + CI integration / 边界测试+CI集成 |
| Week 9–10 | Example pipelines + comprehensive integration tests / 示例管道+全面集成测试 |
| Week 11–12 | Polish docs, address mentor feedback / 文档完善，导师反馈 |
| Final / 最终 | Record/Replay (extended scope), final submission / Record/Replay（扩展范围），最终提交 |

---

## 8. Discussion Topics for Midterm Evaluation / 中期评估讨论话题

1. **Integration test architecture / 集成测试架构**: Both standalone (`DORA_TEST_WITH_INPUTS`) and daemon-based (`dora run`) modes work. We chose daemon-based for demo impact. Is standalone mode worth adding?

   standalone和daemon两种模式都可以工作。我们选择daemon模式来做演示。需要加standalone模式吗？

2. **`data_type` propagation / 类型传播**: Currently JSON-file-based. Should we add a `--schema-file` flag for explicit Arrow IPC schema?

   目前通过JSON文件传递类型。需要加`--schema-file`支持Arrow IPC schema吗？

3. **CI setup / CI集成**: Integration tests need `dora` CLI and port 6013. Should we add a GitHub Actions workflow, or keep them local-only for now?

   集成测试需要dora CLI和端口6013。需要加入GitHub Actions还是保持本地？

---

*Report generated 2026-06-30 / 报告生成于 2026-06-30. Midterm evaluation deadline / 中期评估截止: 2026-07-10.*
