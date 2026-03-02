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

# 任务计划: 设计 fail-closed 的外部事件注入边界(ralph emit / turn_action)(2026-03-01 17:38 +0800)

## 目标

- 允许 hats 在运行过程中用 `ralph emit` 点对点沟通,不要求输出 `<event ...>`.
- 同时把 `turn_action=steer|interrupt` 等控制面信号做成 fail-closed,避免模型误触发造成运行时被打断/被劫持.
- 输出一份"边界规则 + 落地点"清单,后续可以直接按清单实现和写回归测试.

## 两种路线(供选择)

1. 最佳方案(更安全,投入更大): 为 `ralph emit` 增加"来源归因"(source/source_instance)并在 Supervisor 侧做权限校验; 对 `<event ...>` 协议引入 guard token.
2. 先能用(更快落地): 先在 `ralph emit` CLI 层基于环境变量(`RALPH_HAT_ID`/`RALPH_HAT_INSTANCE_ID`)做硬拒绝,禁止 hat 发送 steer/interrupt; Supervisor 侧先做最小防御(发现 steer/interrupt 且疑似来自 hat 就拒绝并回送错误).

## 我将采用

- 先走路线2 交付 4.2(快速止血,高 ROI).
- 路线1 作为 4.1/4.3 的增强(需要明确 threat model,避免过度设计).

## 阶段

- [x] 阶段1: 盘点现状(ralph emit/外部事件读取/turn_action 路径)
- [x] 阶段2: 明确边界规则(谁能 steer,谁只能 request)
- [x] 阶段3: 设计 fail-closed 的反馈协议(错误如何回送给 hat)
- [x] 阶段4: 开 OpenSpec/change,再进入实现

## 状态

**目前在阶段4**:

- 已定位关键代码点:
  - `crates/ralph-cli/src/main.rs`(emit args/emit_command)
  - `crates/ralph-core/src/parallel/supervisor.rs`(外部事件->Event)
  - `crates/ralph-cli/src/parallel_runner.rs`(给 hat 进程注入 RALPH_HAT_ID/INSTANCE_ID)
  - `crates/ralph-core/src/event_parser.rs`(`<event>` 协议解析)
- 已确认边界:
  - hats 之间沟通只走 data-plane(`ralph emit topic=...`).
  - `turn_action=steer|interrupt` 仅用于 ExternalInput -> ralph#1(control-plane).
  - hat-to-hat 采用 request/result,由 B 在 job 结束时回传最终结论,不使用 steer.
- 已创建 OpenSpec change:
  - `openspec/changes/emit-control-plane-fail-closed/`
- 已完成 OpenSpec proposal:
  - `openspec/changes/emit-control-plane-fail-closed/proposal.md`
- 下一步: `openspec` 继续写 `specs`(delta spec),再写 `tasks`(实现清单).

## 2026-03-01 22:40 +0800 | 继续: 为 change 写 `proposal.md`(先锁定边界与反馈语义)

- 我正在做什么:
  - 读取 `openspec status/instructions` 获取 proposal 模板与输出路径。
  - 起草并落盘 `openspec/changes/emit-control-plane-fail-closed/proposal.md`。
- 为什么现在做:
  - change 已创建,但没有任何 artifact,当前仍处于“口头结论”.
  - 先把 data-plane vs control-plane 的边界规则写进 proposal,后续 design/specs 才能对齐并可测试。
- 计划产出(本轮最小闭环):
  - `proposal.md` 明确:
    - 哪些字段属于 control-plane(必须 fail-closed)。
    - 哪些属于 data-plane(允许 hats 互通)。
    - fail-closed 的错误反馈应如何让 hat 自行纠正(面向 4.2)。

## 2026-03-01 22:45 +0800 | 完成: proposal.md 已落盘,解锁 design/specs

- 已落盘:
  - `openspec/changes/emit-control-plane-fail-closed/proposal.md`
- `openspec status` 结果:
  - `proposal` = done
  - `design/specs` = ready
  - `tasks` = blocked(等待 design+specs)

## 2026-03-01 23:54 +0800 | 继续: 为 change 写 `design.md`(收敛实现决策与风险)

- 我正在做什么:
  - 读取 `openspec instructions design` 与 proposal,起草并落盘 `openspec/changes/emit-control-plane-fail-closed/design.md`。
- 为什么这样做:
  - proposal 只回答 why/what,但本 change 属于安全边界收敛,需要把“怎么做”写清楚:
    - 哪些校验在 CLI 做,哪些在 Supervisor 做(防御纵深)。
    - 哪些行为必须拒绝而不是降级(真正 fail-closed)。
    - 对 TUI/manual JSONL 注入的影响与兼容策略。
- 计划产出:
  - design.md 至少包含:
    - 关键决策与备选方案对比(为什么不用 guard token/为什么先做 4.2)。
    - 失败场景与 mitigation(误触发、误投递、无回应排查)。
    - 测试策略(哪些路径必须有回归测试)。

## 2026-03-01 23:57 +0800 | 完成: design.md 已落盘,下一步写 specs

- 已落盘:
  - `openspec/changes/emit-control-plane-fail-closed/design.md`
- `openspec status` 结果:
  - `proposal` = done
  - `design` = done
  - `specs` = ready
  - `tasks` = blocked(等待 specs)

## 2026-03-02 00:28 +0800 | 快进: $openspec-ff-change 生成 specs + tasks(到 apply-ready)

- 我正在做什么:
  - 按 proposal 的 Modified Capabilities,一次性生成 delta specs:
    - `specs/parallel-hat-instances/spec.md`
    - `specs/parallel-trigger-routing/spec.md`
  - 生成 `tasks.md`,把 4.2 落地拆成可执行清单(含测试与文档同步)。
- 为什么现在做:
  - 你明确要 fast-forward,目标是尽快进入实现阶段.
  - applyRequires=tasks,所以把 specs+tasks 补齐后就可以直接 `/opsx:apply`。

## 2026-03-02 00:31 +0800 | 完成: specs + tasks 已齐全(change apply-ready)

- 已新增 delta specs:
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-hat-instances/spec.md`
  - `openspec/changes/emit-control-plane-fail-closed/specs/parallel-trigger-routing/spec.md`
- 已新增 tasks:
  - `openspec/changes/emit-control-plane-fail-closed/tasks.md`
- `openspec status` 结果:
  - `proposal/design/specs/tasks` 均为 done
  - `isComplete=true`

## 2026-03-02 00:36 +0800 | 补充: 把 hat-to-hat “只回最终结论,不在中途 reply”写入 change

- 我正在做什么:
  - 更新 `openspec/changes/emit-control-plane-fail-closed/design.md`,增加 D5 决策,明确 request/result 协作约束.
  - 更新 `openspec/changes/emit-control-plane-fail-closed/tasks.md`,把该约束加入 docs 同步任务(4.3).
- 为什么这样做:
  - 这是你明确提出并认可的高 ROI 行为约束,但之前只记录在 `notes.md/task_plan.md`,没有进入 OpenSpec change 的“可实现契约”.
  - 把它写进 design/tasks,实现阶段才不会忘,也避免后续人员误用“中途 reply”导致上游漂移.

## 2026-03-02 12:34 +0800 | 决策落盘: external-only + rejection escalate + 修正 all_hat 示范

- 你给出的反馈与决策:
  - `<event ...>` 这条 in-band 路暂不收口(4.2 只做 external JSONL 注入的 fail-closed).
  - all_hat 示例可以改,避免 hats 学到“对 worker steer”的用法.
  - Supervisor 拒绝 external control-plane 注入时,要让 `ralph#1` 明确看到(不只写日志).
- 我做的同步:
  - 已更新 OpenSpec change 文档与 specs/tasks,明确:
    - external-only 的适用范围.
    - 拒绝时复用 `routing.escalate` 告警到 `ralph#1`.
    - tasks 增加 `config/all_hat.md` 同步任务.
