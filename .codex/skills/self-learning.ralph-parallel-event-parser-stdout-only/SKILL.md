---
name: self-learning.ralph-parallel-event-parser-stdout-only
description: |
  修复 Ralph 并行模式下的“假事件/重复事件”问题：Codex/后端回显写到 stderr，包含 `<event ...>`，被 EventParser 误判为真实事件，导致路由与 E2E 统计波动。
  适用场景：(1) 并行 E2E 里 build.task 数量异常偏大，但 build.done/test.done 为 0 或波动；(2) stdout/stderr 里出现 `<event ...>` 示例块；(3) 事件日志出现“看起来像 prompt 示例”的事件。
  方案：只从 worker stdout 解析事件；stderr 仅用于可观测输出，不参与解析。
author: Claude Code
version: 1.0.0
date: 2026-01-29
---

# Ralph 并行模式：只从 stdout 解析 `<event ...>`（忽略 stderr）

## 问题
在并行模式下，每个 hat job 都会产生 stdout/stderr。

但一些后端（尤其 Codex CLI）会把“回显的 prompt/系统日志/诊断信息”写到 stderr。
如果这些 stderr 文本里包含 `<event ...>`（哪怕只是 prompt 里的示例），EventParser 也可能把它当成“真实输出事件”：

- 事件计数异常（例如 `build.task` 暴涨）
- 事件链路被污染（重复路由、错误的下游触发）
- E2E 断言出现“虚假失败/虚假通过”（flaky）

## 上下文 / 触发条件
满足以下任意一个现象，就应该用这个 skill：

1. 并行 E2E 的事件统计异常（常见形态）：
   - `build.task: 很多`
   - `build.done: 0`
   - `test.done: 0`
2. 你在输出里看到 `<event topic="...">`，但它看起来像：
   - prompt 里的示例
   - README/注释里的示例
   - fenced code block 里的示例
3. 事件日志 `.ralph/events.jsonl` 中出现“明显不是 hat 真实执行结果”的事件（内容像说明文字/样例）。

## 解决方案
核心原则：**只把 stdout 当成“可解析事件流”，stderr 只做“可观测日志流”。**

### 步骤（实现侧）
1. 在并行 job 执行器里把输出分成两路：
   - stdout：追加到 `HatJobResult.output`（供 EventParser 解析）
   - stderr：只做流式展示（Supervisor/TUI/log），不要进入 EventParser
2. 确保任何“事件解析器”只读取 stdout 聚合输出。
3. 如果你需要保留 stderr 以便排障：
   - 可以将 stderr 记录到单独字段（例如 `HatJobResult.stderr`）
   - 或者仅在日志中保留，但不要喂给事件解析器

### 本仓库对应落点（便于定位/参考）
- `crates/ralph-cli/src/parallel_runner.rs`
  - `CliHatJobExecutor::handle_output_line`：stdout 进入 `HatJobResult.output`（用于 EventParser），stderr 仅用于可观测输出，不参与解析。

## 验证
推荐用“最容易触发假事件”的方式做回归：

1. 选择一个并行场景，让 prompt 故意包含“伪 `<event>` 示例块”或 fenced code block（但不应该被当成真实事件）。
2. 运行并观察：
   - `.ralph/events.jsonl` 的 topic 计数应当稳定且合理
   - `build.done/test.done` 等完成事件应当能出现（如果场景定义如此）
3. 回归测试（本仓库）：
   - `cargo test -p ralph-core smoke_runner`
   - 并行 E2E（Codex）跑两次变体，确认不 flaky

## 示例（伪代码）
```rust
match stream {
  Stdout => parse_buffer.push(line),  // ✅ 允许解析 <event ...>
  Stderr => render_only(line),        // ✅ 可观测，但不解析
}
```

## 备注
- 这不是“为了测试而删信息”，而是把两个语义彻底分离：
  - stdout = 可被机器当作协议解析的数据通道
  - stderr = 人类可读的诊断通道（可能包含任意文本，不能作为协议输入）
- 只要你允许 stderr 进入解析器，就等于允许“任意文本注入事件总线”，规模越大越容易炸。

## 参考资料
- 无（基于本仓库并行 E2E 的真实 flaky 现象与修复经验沉淀）。
