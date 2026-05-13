# Agent guidance schema

本文件定义 Ralph 仓库里 agent-facing guidance 资产的最小结构。
它的目标不是增加流程,而是减少漂移: 让维护者知道某条规则应该写在哪里,也让自动校验能确认核心文件没有失联。

## 1. 适用范围

这份 schema 适用于会影响智能体行为的长期资产,包括:

- 根级操作契约
- 项目级经验
- prompt / skill / hat 行为契约
- OpenSpec change
- runbook 或治理文档
- 学习报告和设计报告

它不适用于普通用户文档、一次性日志、构建产物或临时调试输出。

## 2. 资产类型

| 类型 | 责任 | 典型路径 |
| --- | --- | --- |
| `root_contract` | 顶层工作规则和安全边界 | `AGENTS.md` |
| `experience` | 已提炼、可复用的项目经验 | `EXPERIENCE.md` |
| `schema_doc` | 指导资产结构和必需字段 | `docs/agent-guidance-schema.md` |
| `prompt_contract` | prompt / skill / hat / final response 行为要求 | `docs/prompt-contract.md` |
| `openspec_change` | 变更规格、设计、任务和验收要求 | `openspec/changes/<change>/` |
| `skill` | 可复用工作流或故障处理 playbook | `.codex/skills/...` 或 `.agents/skills/...` |
| `report` | 研究、学习、对比分析结果 | `specs/*analysis*.md` |
| `runbook` | 可执行维护流程 | `docs/runbook/*.md` |

## 3. 每类资产的最小要求

### 3.1 root contract

必须说明:

- 当前仓库的工作原则
- 修改代码前的规格要求
- 验证和完成门槛
- 长期知识入口
- 不能做的反模式

`AGENTS.md` 是唯一 root contract。不要再新增第二份根级规则文件。

### 3.2 experience

必须说明:

- 触发条件
- 已验证事实
- 下次遇到同类问题时的动作
- 经验的状态和置信度

经验正文放在 `EXPERIENCE.md`。`AGENTS.md` 只做索引,不要把长篇经验堆进去。

### 3.3 schema doc

必须说明:

- 资产类型
- 每类资产的职责边界
- 与 runtime feature 的边界
- manifest 如何登记这些资产

schema doc 只管 guidance governance,不负责 runtime topology。

### 3.4 prompt contract

必须说明:

- prompt-like 资产的最小输出契约
- 证据要求
- scope boundary
- escalation 条件
- completion claim 的门槛

如果某个 skill 或 hat 需要更强约束,可以在自身文件里追加,但不能弱化 prompt contract。

### 3.5 OpenSpec change

必须包含:

- `proposal.md`
- `design.md` 或明确说明为什么不需要设计
- `tasks.md`
- `specs/<capability>/spec.md`

delta spec 的每个 `### Requirement:` 第一句必须包含 `MUST` 或 `SHALL`,否则 archive 阶段容易失败。

## 4. Manifest 规则

核心 guidance 资产必须登记到 `agent-guidance-manifest.toml`。

manifest 条目使用结构化字段,不要把 YAML 注释、Markdown 注释或 prose header 当作机器可读 metadata。

每个条目至少包含:

- `id`
- `type`
- `path`
- `status`
- `summary`
- `required_in_agents_index`

当 `required_in_agents_index = true` 时,`AGENTS.md` 的 Project Knowledge Index 必须包含该 path。

`skill` 资产还有额外规则:

- 路径必须指向项目内 `.agents/skills/<name>/SKILL.md` 或 `.codex/skills/<name>/SKILL.md`。
- active / draft skill 必须有 YAML frontmatter。
- frontmatter 必须包含非空 `name` 和 `description`。
- active / draft skill 的 frontmatter `name` 必须唯一。
- 如果同一个 skill 同时存在于 `.agents/skills` 和 `.codex/skills`,只能有一个 canonical active 入口。重复的 legacy 条目应标为 `archived`。

## 5. Runtime 边界

这份 schema 不创建新的 runtime 能力。

尤其不要把以下内容混进 guidance governance:

- team/tmux runtime
- question obligation runtime state
- capability invocation
- resource bootstrap selector
- native hook 安装逻辑

这些可以有独立 OpenSpec change,但不属于本 schema 的直接职责。

## 6. 修改流程

新增或移动核心 guidance 资产时:

1. 先判断资产类型。
2. 更新 `agent-guidance-manifest.toml`。
3. 如果是长期入口,同步更新 `AGENTS.md` Project Knowledge Index。
4. 跑 guidance manifest verifier:
   - 快速独立验证: `ralph verify agent-guidance`。
   - 完整测试门禁: `cargo test`。
5. 如果变更改变行为约束,补 OpenSpec change 或更新相关 specs。
