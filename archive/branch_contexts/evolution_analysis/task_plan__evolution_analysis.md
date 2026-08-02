# 任务计划: 项目演进机会分析

## [2026-05-28 13:59:00] [Session ID: codex-20260528-135644] 计划: 建立只读分析工作线

目标:
- 基于仓库证据,梳理 `ralph-orchestrator` 当前最值得演进的方向,并明确证据、推断、风险和推荐优先级。

方案选项:
- 不惜代价,最佳方案: 完整扫描架构、OpenSpec、测试、文档、运行态 evidence、历史上下文,输出按投资回报和风险排序的演进路线图。
- 先能用,后面再优雅: 只看 README / AGENTS / Cargo workspace / OpenSpec active changes / 高风险大文件,输出一版轻量候选清单。

本轮决定:
- 采用接近"不惜代价,最佳方案"的只读分析,但控制范围在当前回合能验证的证据内。
- 不修改 Rust 源码,不推进 active OpenSpec implementation。
- 支线上下文统一使用 `__evolution_analysis` 后缀。

阶段:
- [x] 阶段1: 读取默认六文件和记忆索引,确认历史背景与上下文边界。
- [x] 阶段2: 收集仓库结构、文档、OpenSpec、任务和测试证据。
- [x] 阶段3: 用 CodeGraph 查看核心模块结构和调用边界。
- [x] 阶段4: 形成演进候选,按证据强度和收益排序。
- [x] 阶段5: 写入 notes / WORKLOG 并交付分析结果。

关键约束:
- 代码只读。
- 结论必须区分"证据"、"推断"、"未知"。
- 发现坏味道时只提出优化建议,不直接动代码。

状态:
- 本轮支线分析已完成,最终排序见 2026-05-28 14:22:13 记录。

## [2026-05-28 14:19:18] [Session ID: codex-20260528-135644] 进展: 证据收集与 CodeGraph 结构查看完成

已完成:
- 读取 README / Cargo workspace / docs site / OpenSpec active changes / tasks / LATER_PLANS / EXPERIENCE 局部证据。
- 使用 CodeGraph 确认索引健康: 386 files,8726 nodes,25974 edges。
- 使用 CodeGraph 查看核心结构: record-session,parallel supervisor,hat instance,TUI state,recoverable failure ledger。
- 统计 Rust 大文件与测试分布,确认多个 runtime/TUI/e2e 文件超过 1000 行。

阶段状态:
- [x] 阶段2: 收集仓库结构、文档、OpenSpec、任务和测试证据。
- [x] 阶段3: 用 CodeGraph 查看核心模块结构和调用边界。
- [ ] 阶段4: 形成演进候选,按证据强度和收益排序。
- [ ] 阶段5: 写入 notes / WORKLOG 并交付分析结果。

状态:
- 目前在阶段4: 正在把证据整理成演进优先级。

## [2026-05-28 14:22:13] [Session ID: codex-20260528-135644] 完成: 演进候选已排序并记录

演进候选排序:
1. 收口 `agent-cli-recoverable-failure-retry` 的 4.x/5.x/6.x,让 retry lifecycle 可手动继续、可观察、可集成验证。
2. 拆分 runtime/evidence/TUI 高风险大文件,保持 public API 不变,先拆职责边界再加新功能。
3. 先对账 `tui-mdfried-viewer` 的 tasks 与当前实现,再决定恢复 Big Headers / `ratatui-image` 还是修正任务状态。
4. 治理旧 docs tree 的搜索污染,保留发布站点现有新架构,给旧文档明确 legacy/archived 边界。
5. 将 release-fast gate 产品化为固定命令/脚本,减少每条 runtime/evidence change 的人工拼装验证成本。

阶段状态:
- [x] 阶段1: 读取默认六文件和记忆索引,确认历史背景与上下文边界。
- [x] 阶段2: 收集仓库结构、文档、OpenSpec、任务和测试证据。
- [x] 阶段3: 用 CodeGraph 查看核心模块结构和调用边界。
- [x] 阶段4: 形成演进候选,按证据强度和收益排序。
- [x] 阶段5: 写入 notes / WORKLOG 并交付分析结果。

状态:
- 本轮只读分析已完成,可交付。
