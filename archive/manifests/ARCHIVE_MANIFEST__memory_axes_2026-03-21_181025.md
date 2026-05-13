# Archive Manifest: `__memory_axes` 续档批次 `2026-03-21_181025`

## 六文件摘要（用于决定如何沉淀知识）

- 涉及的上下文集（默认 / 支线后缀）：
  - 活跃支线 `__memory_axes`
  - 本次续档对象:
    - `archive/branch_contexts/memory_axes/snapshots/2026-03-21_181025/notes__memory_axes_2026-03-21_181025.md`
- 任务目标（`task_plan__memory_axes.md`）：
  - 从双轴 memory / `experience.md` 的 explore,推进到 `scoped-experience-system` OpenSpec 与实现。
  - 后续又扩展到 examples 维度 E2E 验证,并排查 `parallel-experimental-dev-engine-example` 的“没有回流”现象。
- 关键决定（`task_plan__memory_axes.md`）：
  - 长期沿用 `__memory_axes` 作为支线上下文集。
  - 长期可复用知识统一用 `experience.md` 语义表达。
  - 双轴 memory 采用:
    - role / topic / project 分层
    - canonical writer
    - promotion / demotion
    - scoped injection
- 关键发现（旧 `notes__memory_axes.md`）：
  - 多 hat “共同维护 topic”不等于并发直写同一文件。
  - project 根 `experience.md` 应采用更严格写入权。
  - `parallel-experimental-dev-engine` 早期录制里,确实出现过 workflow event 落到 stderr/tool transcript 的现象。
  - 新录制里,`experiment.result -> experiment.reviewed` 已 durable 落盘,说明旧的 worker 回流问题不再是唯一主矛盾。
  - `integration.task` 一度被 `<\\/event>` 关闭标签卡住,对应 parser hardening 已进入代码与测试。
- 实际变更（`WORKLOG__memory_axes.md`）：
  - 已完成 `scoped-experience-system` OpenSpec 与主体实现。
  - 已跑过 core / cli / smoke / openspec 验证。
  - 已开始在 `examples/` 上做真实 E2E 验证。
- 支线组摘要（`__memory_axes`）：
  - 这条支线仍然活跃。
  - 当前主题已经从“记忆系统设计”延伸到“examples 真实运行与根因排查”。
- 支线组活跃度判定（活跃 / 未轮转旧支线 / 历史版本）：
  - `task_plan__memory_axes.md` / `WORKLOG__memory_axes.md` / `LATER_PLANS__memory_axes.md` / `EPIPHANY_LOG__memory_axes.md`: 活跃
  - `notes__memory_axes_2026-03-21_181025.md`: 本次覆盖到的历史快照,应归档
- 暂缓事项 / 后续方向（`LATER_PLANS__memory_axes.md`）：
  - 全量 26 条 examples 真后端 fresh report
  - `Ctrl-C` 后残留子进程的中断清理验证
- 错误与根因（`ERRORFIX__memory_axes*.md`，如有）：
  - 当前无对应文件
- 重大风险 / 灾难点 / 重要规律（`EPIPHANY_LOG__memory_axes.md`）：
  - canonical writer 是 topic / role / project 经验体系成立的前提
  - append-only handoff 与结构化 `experience.md` 不能混在同一文件协议里
  - examples 批量 E2E 进行中时,`report-live.md` 才是当前 run 的事实面板
- 可复用点候选（1-3 条）：
  - 并行 example 的真实排查要分清:
    - worker 事件有没有 durable 落盘
    - coordinator / parser / integration 哪一层真正断开
  - 只看 `report.json` / `report.md` 很容易误把旧 run 当本轮结果
  - 当支线 `notes` 过长时,要先续档再继续调查,否则“有效结论”和“旧过程噪音”会重新缠在一起
- 最适合写到哪里：
  - 目前先保留在支线上下文与本归档 manifest
  - 暂不新增 `EXPERIENCE.md` / skill
- 需要同步的现有 `docs/` / `specs/` / plan 文档：
  - 暂无仅因本次续档必须同步的内容
- 是否需要新增或更新 `docs/` / `specs` / plan 文档：否
- 是否提取/更新 skill：否

## 本次归档动作

- 已归档:
  - `notes__memory_axes.md` -> `archive/branch_contexts/memory_axes/snapshots/2026-03-21_181025/notes__memory_axes_2026-03-21_181025.md`
- 保持活跃:
  - `task_plan__memory_axes.md`
  - `WORKLOG__memory_axes.md`
  - `LATER_PLANS__memory_axes.md`
  - `EPIPHANY_LOG__memory_axes.md`
  - 新续档后的 `notes__memory_axes.md`

## 行动建议

- 当前仍有活跃任务,最该继续做的是:
  - 先把“`parallel-experimental-dev-engine-example` 无回流”从现象层压缩成一条当前有效主假设
  - 再补一个最小回归测试,锁定 worktree / prompt / workspace 一致性
  - 最后重新录制该场景,只看 durable topic 链是否闭环到 `integration.applied` 与 `LOOP_COMPLETE`
