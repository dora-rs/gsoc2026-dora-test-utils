# Midterm Demo — 操作手册

> 面向 mentor (bobdingAI) 的中期评估演示指南。
> 预计耗时：5 分钟（不含首次构建）

---

## 0. 环境要求

- Rust 1.96+
- Linux (Ubuntu/Debian 或其他)
- 端口 **6013** 空闲（dora daemon 使用）
- Git

```bash
rustc --version   # ≥ 1.96
```

---

## 1. 克隆 & 构建

```bash
git clone https://github.com/SunSunSun689/gsoc2026-dora-test-utils.git
cd gsoc2026-dora-test-utils
git checkout week5
```

**首次构建**（约 5-10 分钟，取决于网络）：

```bash
# 构建本项目的 3 个二进制 + dora CLI
cargo build --bin test_source --bin test-sink --bin echo-node
PYO3_NO_PYTHON=1 cargo build --manifest-path dora/Cargo.toml -p dora-cli
```

---

## 2. 一键 Demo

```bash
bash scripts/demo.sh
```

脚本自动完成 7 个步骤：

| 步骤 | 内容 | 预计耗时 |
|------|------|---------|
| 0 | 检查 Rust 工具链 | <1s |
| 1 | 构建所有二进制（增量秒过） | <1s |
| 2 | 展示 YAML 数据流定义 | — |
| 3 | 展示测试输入/预期输出 | — |
| 4 | `dora run` 启动 pipeline | ~2s |
| 5 | 显示 `result.json` — `"match": true` | — |
| 6 | 4 个自动化集成测试 | ~6s |
| 7 | 39 个库单测 | — |

预期输出：
```
═══ Demo Complete ═══

Summary:
  • Echo pipeline: test-source → echo-node → test-sink
  • Integration tests: 4/4 passing
  • Library unit tests: 39 passed
```

---

## 3. 分步手动演示（可选）

如果不想用脚本，可以逐步演示：

### Step 1: 展示数据流定义

```bash
cat tests/fixtures/echo-dataflow.yml
```

3 个节点：`test-source` → `echo-node` → `test-sink`

### Step 2: 展示测试数据

```bash
cat tests/fixtures/source-data.json     # 输入：[42, 99, -1]
cat tests/fixtures/expected-output.json # 预期：同上
```

### Step 3: 运行数据流

```bash
dora/target/debug/dora run tests/fixtures/echo-dataflow.yml --stop-after 5s
```

输出中会看到：
- `spawning` → 三个节点启动
- `node is ready` → 连接完成
- `finished successfully` → 全部正常退出

### Step 4: 查看结果

```bash
cat result.json
# {"match": true, "expected_count": 3, "received_count": 3, "differences": []}
```

### Step 5: 运行集成测试

```bash
cargo test --test integration -- --test-threads=1 --nocapture
```

4 个测试：
- `echo_pipeline_exact_match_int64` — 精确匹配
- `echo_pipeline_semantic_int32_tolerates_int64` — 跨类型兼容（Int64 vs Int32）
- `echo_pipeline_ten_elements` — 10 元素批量
- `echo_pipeline_string_data` — 字符串数据

---

## 4. 项目架构速览

```
test-source                echo-node               test-sink
┌──────────┐    data     ┌──────────┐    data     ┌──────────┐
│ JSON→Arr │ ──────────▶ │ 收什么   │ ──────────▶ │ 接收数据  │
│ 发送输出  │             │ 发什么   │             │ 与预期对比 │
└──────────┘             └──────────┘             │ 写result │
                                                   └──────────┘
                                                        │
                                                   result.json
                                                   {match: true}
```

### 5 个核心模块

| 模块 | 文件 | 用途 |
|------|------|------|
| NodeHarness | `src/harness.rs` | 单测驱动器，不启动 daemon |
| MockEventStream | `src/mock/` | 纯内存模拟 |
| TestSource | `src/source.rs` + bin | 往数据流注入测试数据 |
| TestSink | `src/sink.rs` + bin | 捕获输出 + 断言 |
| Echo Node | `tests/fixtures/echo-node.rs` | 透传节点（测试用） |

### API 示例

```rust
// TestSource 库函数调用
use dora_test_utils::source::{run_test_source, SourceConfig};
let config = SourceConfig {
    output_id: "data".into(),
    data: serde_json::json!({"data": [42, 99, -1], "data_type": "Int64"}),
};
run_test_source(config)?;

// TestSink 库函数调用
use dora_test_utils::sink::{run_test_sink, SinkConfig};
let config = SinkConfig {
    expected_file: "expected.json".into(),
    output_file: "result.json".into(),
    fail_on_mismatch: true,
    strict: false,
};
let result = run_test_sink(config)?;
assert!(result.r#match);
```

---

## 5. 已知问题

| 问题 | 影响 | 绕过 |
|------|------|------|
| 3 个 harness 测试偶发挂起 | `cargo test --lib` 可能卡住 | `--test-threads=1` 或跳过 |
| 端口 6013 冲突 | `dora run` 失败 | 确保端口空闲 |
| Python 3.10 环境 | `pyo3-build-config` 报错 | `PYO3_NO_PYTHON=1` |

---

## 6. 关键数据

```
Commits:  35+ (week5 branch)
Rust代码: ~2,500 lines
测试:     51 个 (39 unit + 5 e2e + 4 integration + 3 smoke)
代码审查: 3 轮, 27 个 findings 已修复
进度:     超前于计划 (Midterm 原定 Week 6-8, 实际 Week 5 完成)
```

---

## 7. 讨论话题

1. **Integration test 架构** — 当前用 daemon mode (`dora run`)，是否需要 standalone mode (`DORA_TEST_WITH_INPUTS`)？
2. **`data_type` 传播** — 目前通过 JSON 文件传递，是否加 `--schema-file` 支持 Arrow IPC schema？
3. **CI 集成** — 集成测试需要 dora CLI + 端口 6013，如何纳入 CI workflow？
4. **Coding Phase 2 方向** — 优先做 Record/Replay 还是 Python bindings？

---

*Repo: https://github.com/SunSunSun689/gsoc2026-dora-test-utils | Branch: week5*
