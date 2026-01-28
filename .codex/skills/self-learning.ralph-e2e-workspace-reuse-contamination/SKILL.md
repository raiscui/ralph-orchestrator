---
name: self-learning.ralph-e2e-workspace-reuse-contamination
description: |
  修复 Ralph E2E 复跑污染：使用 `--keep-workspace`（或复用同一路径）后再次运行相同 scenario，会把历史 `.ralph/events.jsonl` 叠加到新 run，导致事件计数断言“虚假通过/虚假失败”。
  适用场景：(1) 第一次跑通过/失败，第二次结果异常波动；(2) events.jsonl 计数越来越大；(3) 复跑时 workspace 目录没有被清理。
  方案：E2E 每次创建 workspace 前先清理旧目录（remove_dir_all → recreate），保证每次 run 从干净环境开始。
author: Claude Code
version: 1.0.0
date: 2026-01-29
---

# Ralph E2E：workspace 复跑污染（keep-workspace / events.jsonl 累积）

## 问题
`ralph-e2e` 的很多断言依赖 `.ralph/events.jsonl` 的统计结果（topic 计数、实例归因等）。

如果你在调试时使用了 `--keep-workspace`，下一次再次运行同一个 scenario：

- workspace 目录可能被复用（同一路径继续写）
- `.ralph/events.jsonl` 会把两次运行的事件堆在一起

后果是：
- 计数类断言出现“虚假通过”（历史事件把计数抬高）
- 或“虚假失败”（历史噪音干扰真实链路判断）
- 排查时也会被历史输出污染，浪费大量时间

## 上下文 / 触发条件
满足以下任意一个现象，就应该用这个 skill：

1. 你运行 E2E 时使用了 `--keep-workspace`，之后复跑同一 scenario，结果开始波动
2. 你打开 `.e2e-tests/<scenario>/.ralph/events.jsonl`，发现内容明显来自多次运行（时间戳跨多次 run）
3. 事件计数随复跑次数“只增不减”

## 解决方案
核心原则：**E2E 每次 run 必须从干净 workspace 开始。**

### 步骤（实现侧）
1. 在 E2E harness 的 workspace 创建逻辑里：
   - 如果目标 workspace 路径已经存在：先 `remove_dir_all`
   - 然后再 `create_dir_all`
2. 保留 `--keep-workspace` 的语义为：
   - 本次 run 结束后不删除（用于你手动 inspect）
   - 但下一次 run 仍然应该先清理（避免污染）

### 本仓库对应落点（便于定位/参考）
- `crates/ralph-e2e/src/workspace.rs`
  - `WorkspaceManager::create_workspace()`：如果目录已存在则先清理，再创建。

## 验证
推荐做一个“最直接的复现/验证”：

1. 用 `--keep-workspace` 跑同一 scenario 两次
2. 观察第二次开始前 workspace 是否会被清理（目录内旧 `.ralph/events.jsonl` 不应残留）
3. 事件计数断言在两次运行中应保持一致（不随次数上升）

## 示例（现象描述）
- 现象：
  - 第一次：`build.task: 3, build.done: 2, test.done: 1`
  - 第二次（污染后）：`build.task: 6, build.done: 4, test.done: 2`（看似“更好了”，实则是历史叠加）
- 修复后：两次都应保持接近同一数量级，且不累积。

## 备注
- 这是典型的“调试便利性 vs 可重复性”冲突：
  - `--keep-workspace` 方便 inspect
  - 但必须保证下一次 run 的隔离性，否则 E2E 失去意义
- 另一种方案是“workspace 路径带 run_id/时间戳”（每次生成新目录），但会让 `.e2e-tests/` 快速膨胀；本仓库选择“复用路径 + 运行前清理”。

## 参考资料
- 无（基于本仓库 E2E 复跑污染现象与修复经验沉淀）。
