---
name: self-learning.ralph-parallel-secondary-coordinator-prompt-drift
description: |
  修复/规避并行模式下动态创建的备用协调实例(例如 `ralph#2`)因缺少 coordinator prompt/护栏而发生 topic 漂移的问题。
  适用场景: (1) autopilot hard verdict 缺失 `integration.applied` 等 required topics,但 record-session/topic_counts 里出现 `integration.done`;
  (2) 并行 Supervisor 在 `ralph#1` Running 时改投到 `ralph#2`,但 `ralph#2` 输出像“极小兜底 prompt”并发布非协议 topic;
  (3) 回归测试需要锁死 `ralph#2` prompt 必须包含 `KEY SEMANTICS (OFFICIAL)` 与 `RALPH PROMPT (CONFIG)` 段落。
author: Codex CLI
version: 1.0.0
date: 2026-02-19
---

# 并行: 备用协调实例(ralph#2) prompt 漂移导致协议/CI 不稳定

## 问题

并行 Supervisor 为了避免“主协调实例 ralph#1 忙时堵塞”,会按需创建备用协调实例(例如 `ralph#2`)并改投事件。

如果 `ralph#2` 没拿到与 `ralph#1` 等价的 coordinator 指令与护栏:

- 它更容易 prompt 漂移.
- 漂移的直接后果是发布非协议 topic(例如 `integration.done`).
- 这会被 autopilot hard verdict 直接放大成 FAIL:
  - required topic 缺失(例如 `integration.applied`)
  - 或 topic 时间线/终止条件异常

这类失败的“现象”很像模型随机漂移。
但根因通常是“prompt 注入不一致”,属于可工程化修复的问题。

## 上下文 / 触发条件

满足任意一条就用这个 skill:

1. 你在 parallel 模式下跑 autopilot/E2E,hard verdict 不稳定.
2. 你看到 `integration.done` 出现,但 `integration.applied` 缺失.
3. 你最近改动了:
   - coordinator prompt 生成逻辑
   - `event_loop.ralph_prompt` 注入逻辑
   - supervisor 的 instance autoscale / busy-ralph 改投逻辑

## 解决方案

### 1) 先把“漂移”从主观判断变成证据

优先从 out_dir/report 或 record-session JSONL 获取两个硬证据:

- topic_counts: 是否出现 `integration.done` 且缺失 `integration.applied`
- 事件路由: 是否发生了“目标是 ralph#1,但实际改投 ralph#2”

### 2) 锁死一个不会被重构误伤的回归测试

本仓库推荐的测试策略是:

1. 让 `ralph#1` 处于 Running(模拟忙状态)
2. route 一个显式 target_instance=ralph#1 的事件
3. 由 executor 捕获 `ralph#2` 的 prompt
4. 断言 `ralph#2` prompt 必须包含:
   - coordinator 身份行(包含 instance_id)
   - `## KEY SEMANTICS (OFFICIAL)` 段落
   - `## RALPH PROMPT (CONFIG)` 段落
   - `config.event_loop.ralph_prompt` 的锚点文本

参考实现(现成锚点):

- `crates/ralph-core/src/parallel/supervisor/routing_tests.rs`
  - `busy_ralph_secondary_includes_coordinator_instructions_and_config_prompt`

### 3) 代码侧修复原则(稳定性不靠运气)

把这条原则当成并行系统的硬约束:

> 只要一个 instance 被当作 coordinator 使用,它就必须走与主 coordinator 相同的 prompt 生成路径(包含 config 注入与官方语义段落)。

换句话说:

- “隔离顶层 prompt,不污染 worker”是必要的(见已有 skill: `self-learning.llm-multi-agent-prompt-isolation`)。
- 但同样重要的是“所有 coordinator 实例必须拿到 coordinator prompt”,否则会出现对称性 bug。

## 验证

优先跑定向回归测试(如果你不确定测试路径,就跑整个 crate):

- `cargo test -p ralph-core`

然后用一次 autopilot 实跑做端到端确认(可选,但最有说服力):

- hard verdict PASS
- required topics 全出现(`integration.applied` 不再缺失)
- record-session/topic_counts 不再出现 `integration.done` 这种非协议替代

## 示例(典型失败信号)

- autopilot hard verdict FAIL:
  - required_topic: `integration.applied` 缺失
  - topic_counts 里出现: `integration.done`

## 备注

- 这不是“提示词写得不好”.
  这是“同一角色的多个实例,信息与护栏不一致”.
- 一旦你引入动态 coordinator 实例,就必须把 prompt 注入当成协议的一部分,用单测锁死。

