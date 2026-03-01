# 任务计划(续档)

- 说明: 旧 `task_plan.md` 已超过 1000 行,已续档为 `archive/task_plan_2026-02-28_1717.md`。
- 新任务从这里开始记录。

# 任务计划: 总结 openclaw 启发 + 解释 macOS Xcode license(2026-02-28 17:17 +0800)

## 目标

- 用更短、更可执行的方式,把 `openclaw/openclaw` 对本仓库的启发总结出来。
- 回答你问的: 为什么 `cargo test` 有时会提示需要接受 Xcode license。
- 把本次答复要点追加到 `notes.md`/`WORKLOG.md`(便于后续检索)。

## 阶段

- [x] 阶段1: 从既有研究笔记中提炼要点(openclaw 可迁移机制 + 已落地/待落地)
- [x] 阶段2: 解释 Xcode license 的根因与规避路径(不把环境问题误归因到 cargo)
- [x] 阶段3: 回写四文件(仅追加到文件尾部)

## 状态

**已完成**:

- 已完成 openclaw 启发总结(聚焦可迁移机制: guardrails/doctor/lanes/watchdog/context-window)。
- 已解释 macOS 下 Xcode license 报错的根因与规避路径(Apple toolchain/xcrun/xcode-select vs cargo)。
- 已回写与归档:
  - `notes.md` 已追加 continuous-learning 的四文件摘要。
  - `WORKLOG.md` 已追加本轮记录。
  - 历史版本已归档:
    - `archive/task_plan_2026-02-28_1717.md`
    - `archive/notes_2026-02-28_1706.md`

# 任务计划: 阅读 zeroclaw 源码,提炼可迁移启发(2026-02-28 18:31 +0800)

## 目标

- 阅读 https://github.com/zeroclaw-labs/zeroclaw 的核心代码与文档.
- 提炼对 ralph-orchestrator 有启发的设计点,并给出可落地的改良建议(优先改良,避免无意义新增).

## 两种路线(供选择)

1. 最佳方案(更花时间): clone 全仓,尽可能跑测试/示例,画架构图,逐模块点评,并列出可直接抄作业的 patch/task 清单.
2. 先能用(更快交付): 重点读 README/架构文档 + 核心模块入口,先总结 10-20 条可迁移模式,再按需要深挖.

## 我将采用

- 先走路线2,尽快产出结论.
- 若发现特别值得深挖的模块,再按路线1 对局部加深(只深挖最有价值的点).

## 阶段

- [x] 阶段1: 获取仓库与快速画像(语言/结构/入口/测试)
- [x] 阶段2: 深读关键模块 + 摘录原文证据
- [x] 阶段3: 映射到 ralph 的可迁移清单(可做/不做/原因)
- [x] 阶段4: 回写 notes/WORKLOG/LATER_PLANS,并答复

## 状态

**已完成**:

- 已完成源码阅读与机制提炼(approval/doctor/security/tools/providers/docs/supply-chain).
- 已把要点追加到三文件:
  - `notes.md`
  - `WORKLOG.md`
  - `LATER_PLANS.md`

# 任务计划: 强化 `ralph doctor` 的结构化输出(JSON)与错误分类(2026-02-28 20:46 +0800)

## 目标

- 为 `ralph doctor` 增加机器可读输出,让 code agent/CI/TUI 不必解析 stdout 文本.
- 输出里要包含稳定的分类字段(例如 check_id/category/status),用于自动判定与自动分流.
- 保持现有文本输出不变,避免破坏既有使用与测试.

## 两种路线(供选择)

1. 最佳方案(更工程化): `--format json` 输出稳定 schema,并把每条 check 都赋予稳定 id + category,同时保留原文本 message 作为人类可读.
2. 先能用(更小改动): 仅提供 `--json` 原样输出 warnings/errors 列表(不保证 schema 稳定),后续再补全 id/category.

## 我将采用

- 采用路线1.
- 原因: 你明确说只对 code agent 负责,那就优先让输出"可被程序消费"且长期稳定.

## 阶段

- [x] 阶段1: 读 spec + 盘点现状(doctor 现有检查项与输出)
- [x] 阶段2: 设计 JSON schema(最小稳定字段) + 约定 check_id/category
- [x] 阶段3: 实现 `--format json` + reporter 统一记录 checks
- [x] 阶段4: 补回归测试 + 跑 `cargo fmt`/`cargo test`
- [x] 阶段5: 回写 notes/WORKLOG/LATER_PLANS,并交付

## 状态

**已完成**:

- 已实现 `ralph doctor --format json/--json`(schema v1,含 check_id/category/status/message/fix).
- 已补齐回归测试,并跑完 `cargo fmt`/`cargo test` 验证.
