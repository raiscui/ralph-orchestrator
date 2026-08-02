## [2026-05-28 14:22:13] [Session ID: codex-20260528-135644] 任务名称: 项目演进机会分析

### 任务内容
- 对 `ralph-orchestrator` 当前项目状态做只读分析,梳理可以演进的方向。
- 重点查看 runtime/evidence/TUI/OpenSpec/docs/测试与高风险大文件。

### 完成过程
- 读取默认六文件和记忆索引,确认本轮应启用 `__evolution_analysis` 支线上下文,避免污染已超过 1000 行的默认 `notes.md`。
- 使用 CodeGraph 查看项目结构和核心符号关系。
- 读取 README、Cargo workspace、docs site、OpenSpec active changes、current-runtime/evidence specs、recoverable retry specs、TUI mdfried tasks 和关键 Rust 模块。
- 统计 Rust 大文件,确认多个核心 runtime/TUI/CLI 文件超过 1000 行。
- 将证据和推断记录到 `notes__evolution_analysis.md`。

### 总结感悟
- 当前项目最有价值的演进不是继续堆新功能,而是把已经启动的 runtime/evidence/retry/TUI 能力收束成可靠、可观察、可验证的产品面。
- `agent-cli-recoverable-failure-retry` 是最高优先级,因为它直接影响 live agent backend 在 429 / retry limit 场景下的恢复体验。
- TUI mdfried 线发现 spec/tasks 与当前实现状态不一致,后续不能直接按剩余 5.x 推进,必须先对账。
- 大文件拆分应以"保持 public API 不变"为边界,优先拆 aggregate/render/runtime state/helper,不要在拆分时顺手重写行为。
- docs 不是没有治理,而是发布站点和旧文档树已经分层;后续应做 legacy 边界和搜索污染治理。
