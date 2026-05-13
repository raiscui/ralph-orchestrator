## Why

Ralph 当前只有单一的 `.agent/memories.md` 长期记忆语义,这已经不足以支撑用户希望的多层经验体系。现在既需要项目级 `experience.md`,也需要岗位级 `.ralph/roles/<hat_id>/experience.md`,还需要把 topic 共享结论和 instance 原始轨迹分开,否则运行时选择 workflow / hat 时会很快被噪音污染。

这个 change 需要把“经验的作用域、写入权限、晋升/降级纪律、默认注入顺序”先正式钉住。否则后续无 `PROMPT.md` / 无 `ralph.yml` 启动、runtime capability invocation、workflow 动态选择都会缺少稳定的知识层边界。

## What Changes

- 引入分层的 experience / context 体系:
  - runtime work graph: `.agent/tasks.jsonl`
  - instance context: `.ralph/log/<instance_id>/...`
  - topic shared context: `task_plan__topic.md` / `notes__topic.md` / `WORKLOG__topic.md`
  - role experience: `.ralph/roles/<hat_id>/experience.md`
  - project experience: 项目根 `experience.md`
- 为 topic / role / project 三层共享知识定义 canonical writer 规则:
  - topic canonical writer
  - role canonical writer
  - project canonical writer
- 定义 experience 的 promotion / demotion 规则:
  - topic -> role
  - topic -> project
  - role -> project
  - project -> role
  - role -> topic
- 定义默认 injection / read policy:
  - 普通 hats 的注入顺序
  - `ralph#1` 的注入顺序
  - 按需读取与摘要优先原则
- 为 role experience 与 project experience 定义统一 entry 结构,避免后续 parser 和注入逻辑分叉
- 明确当前实现现实:
  - 文档已出现 `memories.path` 口径
  - 但代码仍固定读取 `.agent/memories.md`
  - 本 change 需要以此为迁移起点,而不是假设多路径 experience 已经存在

## Capabilities

### New Capabilities
- `experience-scopes`: 定义 runtime、instance、topic、role、project 五层 experience/context 的职责与存储边界
- `canonical-writer`: 定义 topic / role / project 三层共享知识的单 writer 规则与 handoff 协议
- `experience-promotion`: 定义 experience 的晋升、降级、失活与审计链路规则
- `experience-injection`: 定义不同 agent 在不同阶段如何按需读取并注入 project / role / topic / instance 经验

### Modified Capabilities
- None.

## Impact

- 受影响代码区域:
  - `crates/ralph-core`: memory config, injection flow, context resolution, writer/promotion state model
  - `crates/ralph-cli`: memory/experience 命令面、doctor/debug 输出、迁移入口
  - `.agent/` 与 `.ralph/` 下的默认文件布局和初始化逻辑
  - embedded presets / workflow bootstrap / runtime capability chooser 的上下文读取策略
  - docs / memory-system / getting-started / troubleshooting
- 受影响行为:
  - 长期经验不再只有单一 `.agent/memories.md` 语义
  - project 级全局经验和 role 级岗位经验将被明确区分
  - topic 文件不再允许所有 hats 直接并发写入
  - `ralph#1` 在无显式 `PROMPT.md` / `ralph.yml` 场景下可以基于 project experience 与 workflow metadata 做更稳的首轮选择
