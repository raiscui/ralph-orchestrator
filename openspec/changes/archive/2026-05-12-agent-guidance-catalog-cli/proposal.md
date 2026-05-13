## Why

第一阶段已经把 guidance schema、prompt contract、manifest 和 Rust verifier 建成最小闭环。但 verifier 目前主要通过 `cargo test` dogfood,资产范围也只覆盖少量核心文件。

第二阶段要把它推进成可日常使用的治理入口: 项目自有 skills 进入 manifest,并提供独立 CLI verifier,让维护者不必跑完整测试也能检查 guidance drift。

## What Changes

- 扩展 `agent-guidance-manifest.toml`,登记项目自有 `.agents/skills/*/SKILL.md` 与 `.codex/skills/*/SKILL.md`。
- 为 `skill` 类型增加 skill-specific 静态验证:
  - skill 路径必须在允许的项目 skill roots 下。
  - skill 文件必须包含 frontmatter。
  - frontmatter 必须包含非空 `name` 和 `description`。
  - manifest 中的 skill id 和 skill name 必须唯一。
- 新增独立 CLI verifier 入口,用于执行 agent guidance manifest 校验。
- CLI verifier 输出人类可读摘要,并在失败时返回非零退出码。
- 保留 `cargo test` 中的 repository manifest dogfood,避免 CLI 和测试门禁分裂。

## Capabilities

### New Capabilities

- `agent-guidance-catalog-cli`: 管理项目 guidance skill catalog,并提供独立 agent guidance verifier CLI 入口。

### Modified Capabilities

- `agent-guidance-contracts`: 扩展既有 manifest verifier 的要求,把 `skill` 类型从普通路径检查升级为带 frontmatter 和 skill root 约束的治理资产。

## Impact

- 受影响区域:
  - `agent-guidance-manifest.toml`: 增加项目自有 skill 条目。
  - `crates/ralph-core/src/agent_guidance_manifest.rs`: 扩展 verifier 结果、skill metadata 检查和错误报告。
  - `crates/ralph-cli/src/main.rs`: 增加独立 verifier 子命令。
  - `openspec/changes/agent-guidance-catalog-cli/`: 新增本阶段规格、设计和任务。
- 不做的事情:
  - 不实现 runtime state operation layer。
  - 不实现 question obligation runtime state。
  - 不实现 team/tmux runtime。
  - 不把所有 docs 一次性纳入 manifest。
