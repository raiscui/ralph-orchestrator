---
name: self-learning.ralph-starting-event-semantics
description: |
  修复/避免 Ralph 中 `event_loop.starting_event` 语义被误读：把 starting_event 当作“初始化事件 topic / first event”。
  触发场景：(1) 修改 `EventLoop::initialize()` 时用 starting_event 作为 publish topic；(2) README 把 starting_event 写成 “First event published”；
  (3) 运行时首个事件不再是 task.start，导致 top-level prompt 不再被 `<top-level-prompt>` 包裹、工作流概念混乱。
  解决：fresh run 初始化永远发布 `task.start`；starting_event 仅是“协调后工作流入口事件”提示；未设置时由 `ralph#1` 自行从 objective + 拓扑中选择入口事件，并在 prompt 中明确提示。
author: Codex CLI
version: 1.0.0
date: 2026-01-31
---

# Ralph `starting_event` 语义：不是 first event

## 问题

在 Ralph 的 hat 模式中，`event_loop.starting_event` 很容易被误解成：

- “第一个事件（first event）”，或者
- “初始化阶段发布的事件 topic”。

一旦这样改动，会产生连锁副作用：

- 概念混乱：控制面（task.start/task.resume）与数据面（workflow entry event）混在一起。
- 目标边界变模糊：`EventLoop::format_event()` 只对 `task.start` / `task.resume` 包裹 `<top-level-prompt>`，
  如果首个 topic 不是它们，用户目标会以“普通事件”形式混入上下文。

## 上下文 / 触发条件

你可能正在做以下事情：

1) 试图“修复 starting_event 被忽略”的 bug，于是把它接入 `EventLoop::initialize()`：

- 典型误改：`initialize()` 读取 `config.event_loop.starting_event` 并用它发布初始化事件。

2) 写文档/README 时，把 starting_event 写成 “First event published”。

## 正确语义（要记住的 4 句话）

1. Fresh run 的初始化事件 **永远** 是 `task.start`。
2. Resume run 的初始化事件 **永远** 是 `task.resume`。
3. `event_loop.starting_event` **不是** first event。
4. `event_loop.starting_event` 是“协调后工作流入口事件提示”：
   - 配了就优先遵循；
   - 没配就由 `ralph#1` 基于 objective + topology 自行决定入口事件。

## 解决方案（实现层）

1) 固定初始化事件 topic
- `EventLoop::initialize()`：fresh run 固定发布 `task.start`（不要读 starting_event 当 topic）。
- CLI debug logger（若有单独记录初始事件）：fresh run 也固定记录 `task.start`。

2) 把 starting_event 变成 prompt 指引，而不是“硬编码事件”
- 在 hatless prompt（Ralph 的协调 prompt）里：
  - starting_event 已设置：明确提示 “协调后优先 publish 它启动 workflow”。
  - starting_event 未设置：明确提示 “你 MUST 自行决定入口事件”。

3)（可选但强烈推荐）给出启发式候选入口事件
- 从拓扑中推导候选入口事件：`subscribed_topics - published_topics`（订阅但从未被任何 hat 发布，更像入口事件）。
- 注意：这是启发式辅助，不是强规则；最终仍由 `ralph#1` 决策。

## 验证

- `cargo fmt --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`
- `cargo test -p ralph-core smoke_runner`
- `cargo test -p ralph-core kiro`

## 备注

- `task.start` / `task.resume` 在本项目中属于 Ralph 的控制面事件：
  - 它们承载 top-level prompt（目标边界清晰），不应被当成“业务工作流入口事件”滥用。
  - 配置层面也应避免让 hats 订阅它们（如有校验规则，应明确拒绝）。
