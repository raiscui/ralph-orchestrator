## Why

Ralph 当前已经具备两块重要基础:

- `ralph.yml` 缺失时可以退回默认配置继续启动
- `presets/` 与 `config/all_hat.md` 已经证明编译期内嵌资源是可行路径

但它还缺一套统一的“默认启动资源体系”:

- 默认 prompt 仍然绑定到 `PROMPT.md`,导致“无 `PROMPT.md`”在普通 run 下仍会失败
- embedded presets、`ralph init`、`config/all_hat.md`、`presets/minimal/` 彼此独立,还不是统一 resource catalog
- 当前没有用户级资源目录,无法形成“随程序发布 + 首次释放 + 用户后续可改”的闭环
- 如果未来要让 Ralph 自主选择 preset / hat 组合,当前架构也缺少一个正式的 bootstrap 选择阶段

用户希望 Ralph 在“没有 `PROMPT.md`、没有 `ralph.yml`”时仍然能工作,并且:

- 默认文件可在编译期内嵌
- 开发时可修改资源源文件
- 首次执行时可释放到用户资源目录
- 启动 Ralph agent 前,能基于用户选择或工作区信号决定启用哪套 preset
- 在可控边界内支持多份 preset 的结构化组合

用户后续又明确补充了一层更大的诉求:

- 单个 hat 或整套 workflow(`ralph.yml`) 未来希望都能像 skill / tool 一样被 `ralph#1` 运行时调用
- 但这层运行时 capability 调度不应该反向污染当前 change 的启动期边界
- 因此当前 change 需要把 catalog / metadata / startup bootstrap 先钉稳,为后续 runtime capability change 提供基础

这需要一个比“再加几个缺文件特判”更完整的方案。

## What Changes

- 新增统一的 startup resource catalog:
  - workflow presets
  - backend presets
  - prompt templates
  - example bundles
- 为 startup resources 增加结构化 metadata:
  - summary
  - goal
  - selector hints
  - composition role
- 引入用户级 resource root:
  - 支持从 embedded bundle 首次同步
  - 支持后续用户编辑
  - 支持显式覆盖路径
- 引入 bootstrap selector:
  - 仅在“没有显式 config source”时运行
  - 先选择 / 组合资源,产出单份 resolved config
  - 再启动真实 `EventLoop` / `Supervisor`
  - 分阶段路线记录为:
    - v1: 纯规则 selector
    - v2: 规则优先 + LLM fallback selector
- 统一“无 `PROMPT.md`”行为:
  - 不再直接把默认任务输入绑定到工作区文件
  - 改为通过 prompt source resolver 选择 inline prompt / prompt template / idle bootstrap
- 限定多 preset 组合语义:
  - 只允许结构化组合
  - 不支持运行中热切换整套 topology
- 明确 runtime workflow / hat capability 不在本 change 内实现:
  - 当前 change 只负责 startup bootstrap 所需的 catalog 与 metadata 基座
  - 运行时 capability invocation 将在独立 change 中跟进

## Capabilities

### New Capabilities

- `resource-bootstrap`: Ralph 的默认资源目录、catalog、selector 与 resolved config 启动流程

### Modified Capabilities

- `hat-collections`: presets 不再只是“可复制的 YAML 文件”,而是 catalog 中可枚举、可标注角色的资源

## Impact

- 受影响代码区域:
  - `crates/ralph-cli`: startup, init, doctor, preset/resource listing
  - `crates/ralph-core`: config resolution, prompt resolution, resolved config handoff
  - `scripts/sync-embedded-files.sh` 与 embedded 资源注册逻辑
  - docs / getting-started / migration / troubleshooting
- 受影响行为:
  - `ralph run` 在无 `ralph.yml` / 无 `PROMPT.md` 时不再直接报错,而是进入 resource bootstrap 流程
  - presets 将出现“catalog 元数据”和“结构化组合角色”的新语义
  - 用户将拥有一个可持久修改的资源目录,用于覆盖 embedded 默认资源
  - 后续 runtime capability change 将复用本 change 提供的 catalog / metadata,而不是重新发明另一套资源发现机制
