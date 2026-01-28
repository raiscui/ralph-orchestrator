---
name: self-learning.llm-multi-agent-prompt-isolation
description: |
  修复/规避多智能体（多角色）并行系统里“顶层 prompt 污染 worker”导致行为漂移的问题。
  适用场景：(1) coordinator 的 prompt 被无差别注入到所有 worker，worker 不按自身职责产出（缺少预期事件/产物）；(2) E2E 场景卡到 max_runtime，events 里只有 task/build.task，没有 build.done/test.done；(3) 你看到 worker 在做 coordinator 的事（规划/等待/观察/跑无关 tests）。
  方案：把顶层 prompt 变成“定向事件”，只投递给 coordinator；worker 仅接收自身 instructions + incoming events；测试里让 coordinator prompt 更“机械化”，把等待/收尾交给 orchestrator（drain/guard）。
author: Codex CLI
version: 1.0.0
date: 2026-01-27
---

# 多智能体：隔离顶层 prompt，避免角色污染（提升 E2E 稳定性）

## 问题
在“一个协调者 + 多个 worker（writer/tester/reviewer/…）并行”的系统里，
如果你把同一段顶层 prompt（通常是协调者要做的事）无差别注入到所有 worker：

- worker 会被“角色污染”，开始做协调者的事（规划、等待、观察、跑无关验证）。
- 结果就是：该产出的事件/产物没产出（例如 `build.done/test.done`），E2E 只能卡到 `max_runtime`。

这种失败很误导：
你看到的是“E2E 卡死/超时”，但根因其实是“prompt 语义路由错了”。

## 上下文 / 触发条件
当你满足下面任一条，就应该用这个 skill：

1. 你在做并行/多实例（multi-agent / multi-hat）运行时或 E2E 场景。
2. 你观察到 worker 的输出像 coordinator（例如反复解释、等待条件、跑测试），而不是执行它自己的单一职责。
3. `events.jsonl` 里只看到“启动/触发类事件”（如 `task.start`、`build.task`），却缺少预期完成事件（如 `build.done/test.done`）。
4. 你的 E2E 依赖“LLM 自己等到某个条件满足再结束”（例如“看到 build.done 才 LOOP_COMPLETE”），并出现明显漂移/卡住。

## 解决方案

### 1）把“顶层 prompt”从字符串变成“可路由的 payload”
不要把 `-p "<top prompt>"` 直接拼进所有 worker 的 prompt。
推荐把它当成一个结构化事件的 payload（例如 `task.start(prompt)`），由路由层决定谁能看到。

目标效果：

- coordinator：能看到顶层 prompt（它负责发起/协调/收尾）。
- worker：只看自己的 instructions + 运行中收到的事件（例如 `build.task`），不看 coordinator prompt。

### 2）让 worker 的输入“窄而确定”
worker 的输入建议只包含两类信息：

1. 它自己的固定职责说明（instructions）
2. Supervisor 路由进来的事件（incoming events）

并明确禁止 worker 执行“会拖慢或污染 E2E”的动作：

- 禁止跑测试/命令（除非这个 worker 的职责就是跑测试）
- 禁止做等待/观察（等待应该由 orchestrator 负责）
- 禁止改动文件（如果它是一个“只发事件”的 actor）

### 3）E2E 场景里，让 coordinator prompt 尽量“机械化”
经验规律：越把“等待/观察/判断完成”交给模型，E2E 越容易漂移。

更稳的做法是：

- coordinator 只做确定动作：发起事件（例如 `build.task`），然后输出 `LOOP_COMPLETE`
- orchestrator 负责收尾：实现 completion drain / max_runtime / max_iterations 等“硬门槛”

这样 E2E 的“可判定性”来自机械规则，而不是模型自律。

## 验证
用可重复的信号验证“确实解决了污染”：

1. stdout：能看到 worker 按职责输出（而不是协调者式长篇规划/等待）。
2. events：`events.jsonl` 出现预期完成事件（例如 `build.done/test.done`），并能归因到具体实例（如果系统支持 `source_instance`）。
3. E2E：场景不再卡到 `max_runtime`，能稳定在合理时间内结束。

## 示例（本仓库）
在本仓库的并行 E2E 场景里，典型表现是：

- 只有 `build.task`，缺少 `build.done/test.done`，导致 E2E 超时。
- 根因是顶层 prompt 被注入到 worker，导致 writer/tester 按 coordinator 的语义跑偏。

修复思路落点可以对照：

- `specs/parallel-hat-instances/e2e.md`：`task.start(prompt)` 只投递给 `ralph#1`（coordinator）
- `crates/ralph-core/src/parallel/instance.rs`：避免把顶层 prompt 注入到非 coordinator 的实例（防污染）
- `crates/ralph-core/src/parallel/supervisor.rs`：completion drain + max_runtime/max_iterations（让收尾更机械）

## 备注
- 这不是“提示词工程”问题，本质是“信息路由/权限边界”问题。
- 一旦你允许并行与多角色长期演进，prompt 隔离是必选项：
  - 不隔离就会用无数补丁对抗漂移；
  - 隔离后很多边界问题会自然消失。

