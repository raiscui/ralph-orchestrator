## Why

`specs/oh-my-codex-learning-analysis.md` 的结论很明确: 现在最值得从 oh-my-codex 借鉴的不是完整 runtime,而是“把 agent guidance 当成可治理契约”。

Ralph 已经有很多强约束入口:

- `AGENTS.md` 定义顶层工作契约。
- `EXPERIENCE.md` 保存持续学习后的项目经验。
- `openspec/changes/*` 保存规格和实现任务。
- `.codex/skills/*` / `.agents/skills/*` 保存可复用工作流。
- `crates/ralph-core/src/instructions.rs`、`prompt_overlay.rs` 等代码负责把指导注入 runtime。

但这些资产目前缺一个统一的治理闭环:

- 哪些 guidance 文件是正式资产,没有单一 manifest。
- prompt / skill / hat 的输出契约没有集中说明。
- 新增或移动 guidance 文件时,没有轻量 verifier 检查路径和索引漂移。
- 现有项目知识大多靠人工记住,长期会产生 prompt drift 和文档漂移。

所以本 change 先做最小治理闭环: schema + contract + manifest + verifier。暂不搬完整 OMX team/tmux runtime,也不改运行时拓扑。

## What Changes

- 新增 agent guidance schema 文档,定义 Ralph 中指导资产的必需章节和职责边界。
- 新增 prompt contract 文档,定义 prompt / skill / hat / final response 的最小行为契约。
- 新增 agent guidance asset manifest,作为 guidance 资产的单一真相源。
- 新增 verifier,检查 manifest 指向的文件存在、类型合法、必填字段完整、关键长期文件已被 `AGENTS.md` 索引。
- 将 verifier 接入 Rust 测试,让 guidance 漂移能在 `cargo test` 中暴露。

## Capabilities

### New Capabilities

- `agent-guidance-contracts`: 管理 agent-facing guidance 文档、prompt contract 和 guidance asset manifest 的验证闭环。

## Impact

- 受影响区域:
  - `docs/`: 新增 guidance schema 和 prompt contract 文档。
  - `AGENTS.md`: 补充新长期文件索引。
  - `crates/ralph-core`: 增加 manifest verifier 或测试入口。
  - `specs/` 或项目根配置: 增加 agent guidance manifest。
- 不做的事情:
  - 不实现 team/tmux runtime。
  - 不实现 question obligation runtime state。
  - 不迁移现有 prompt 注入逻辑。
  - 不把 YAML 注释作为机器可读 metadata。
