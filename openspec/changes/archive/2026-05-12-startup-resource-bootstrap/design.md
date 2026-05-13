## Context

这次变更处理的是 Ralph 的“启动前资源选择”问题,不是运行时 orchestration 细节问题。
用户后续提出的“workflow / hat 像 skill 一样被 `ralph#1` 运行时调用”诉求很重要,但它应该建立在 startup catalog 之上,并在独立 change 中处理。

当前系统的真实状态是:

- `ralph run` 在默认 `ralph.yml` 缺失时会回退 `RalphConfig::default()`
- 默认 prompt source 仍是 `PROMPT.md`
- 并行路径对“默认 prompt 缺失”有一个 `idle_start` 特例
- `presets` 和 `config/all_hat.md` 都已经通过 `include_str!` 编译期内嵌
- `ralph init` 可以把 embedded preset 写到当前目录,但没有用户级资源目录

因此当前问题的本质不是“少一个文件”,而是“startup resources 还没有形成统一抽象”。

## Goals / Non-Goals

**Goals**

- 让 Ralph 在没有工作区 `ralph.yml`、没有工作区 `PROMPT.md` 的情况下仍可启动
- 统一 catalog / embedded / 用户目录 / 启动前选择 的语义
- 保持开发时资源可直接修改,发布时资源可随二进制分发
- 允许 Ralph 在正式 run 前选择合适的 preset 组合
- 明确多 preset 组合规则,避免 YAML 文本级随意拼接
- 为未来 runtime workflow / hat capability 复用这套 catalog metadata 打基础

**Non-Goals**

- 不在本 change 中实现运行中热切换整套 `ralph.yml`
- 不在本 change 中实现“运行中的 workflow / hat capability invocation”
- 不把 examples 全部当成 selector 的默认候选 workflow
- 不开放“任意多个 preset 自由混拼”的无约束能力
- 不在本 change 中新增复杂交互式 wizard UI

## Decisions

### 1) Resource metadata 必须是结构化数据,不能把 YAML 注释当运行时主数据源

**选择**

startup resource catalog 中除了资源路径,还必须保存可机读 metadata。

最小 metadata:

- `id`
- `kind`
- `summary`
- `goal`
- `selector_eligible`
- `materialize_on_sync`
- `composition_role`
- `prompt_mode`

允许保留文件头注释作为人类可读说明,但运行时 selector / resolver 不依赖注释是否存在。

`hat.description` 继续作为 hat 级 metadata 的正式字段复用。

**理由**

- 当前 `preset` 简介虽然已经能编译进二进制,但它是手工登记在 `crates/ralph-cli/src/presets.rs` 里的常量,不是运行时从 YAML 注释解析出来的。
- `serde_yaml` / `RalphConfig` 也不会保留注释,因此“把 workflow 描述写在注释头里给 selector 用”不是稳定边界。
- 如果后续还要支持 runtime capability catalog,就更不能把机器语义绑在注释上。

### 2) Resource catalog 必须先于具体文件存在

**选择**

先引入 catalog 抽象,再讨论文件放哪里。

catalog 中每个资源条目至少需要描述:

- `id`
- `kind`:
  - `workflow_preset`
  - `backend_preset`
  - `prompt_template`
  - `example_bundle`
- `selector_eligible`
- `materialize_on_sync`
- `composition_role`
- `prompt_mode`:
  - `self_contained`
  - `requires_task_input`
  - `idle_capable`

**理由**

当前仓库里已经存在三类“看起来都像 preset,其实角色不同”的资源:

- 常规 workflow presets
- `presets/minimal/*` 这类 backend/lightweight 模板
- `examples/*` 这类 bundle

如果没有 catalog,后面只会继续靠手写清单和路径约定硬连。

### 3) 用户资源目录要有统一解析器,不要把路径写死在业务逻辑里

**选择**

v1 实现层使用集中式资源根目录解析器,并提供显式覆盖能力:

1. `RALPH_HOME/resources`(最高优先级)
2. `$HOME/.ralph/resources`
3. `.ralph/resources` 作为无 home 可见时的 workspace fallback

后续如果引入 `ProjectDirs` / 平台规范 app config/data 目录,必须继续经过同一个解析器,不能把路径散写在业务逻辑里。

**理由**

- 用户确实需要一个“首次释放后可编辑”的资源仓
- 但路径选择应该跨平台稳定
- 资源根目录解析和业务逻辑要解耦,否则后面 Windows/macOS/Linux 会出现分叉实现

### 4) 采用两阶段启动,而不是运行中热切换 topology

**选择**

把“Ralph 自主选择 preset”设计成 bootstrap selector:

1. `ralph run` 进入 startup resolution
2. 若有显式 config source:
   - 直接解析并跳过 selector
3. 若无显式 config source:
   - 加载 resource catalog
   - 结合用户默认项、工作区信号、selector 规则,产出 `resolved config`
4. 用 `resolved config` 启动真实 run

**理由**

当前大量 guardrails 都依赖“启动前已有最终拓扑”:

- config validation
- reserved trigger 检查
- `complete_publishes` 发布者检查
- parallel topic contract 生效边界
- `HatRegistry` / `Supervisor` 初始化

如果正式 run 开始后再热切换整套 topology,这些护栏都会被推成动态重建问题。

### 5) 多 preset 组合必须是结构化组合

**选择**

selector 只能产出如下结构:

- 0..1 `backend_preset`
- 1 `workflow_preset`
- 0..N `overlay`
- 0..1 `prompt_template`

并采用确定性的 merge 顺序:

1. backend preset
2. workflow preset
3. overlays
4. resolved prompt source injection
5. 现有 CLI override

字段冲突规则:

- `cli`, `adapters`, backend profile: 后者覆盖前者
- `event_loop`: 明确覆盖,但保留需要 merge 的数组字段语义
- `hats`, `events`, `parallel.topic_contracts`: 以 key 级 merge 为主,冲突时报错而不是静默覆盖
- `example_bundle`: 不参与正常 selector 组合,只能显式 materialize 或显式引用

**理由**

“支持多份 `ralph.yml` 的 hat 混合调度”本身不是问题。  
真正的问题是如果 merge 规则不清楚,最后会变成不可解释系统。

### 6) “无 `PROMPT.md`”要改成 prompt source resolver,而不是继续做缺文件特判

**选择**

引入统一 prompt source resolver,优先级如下:

1. CLI `-p`
2. CLI `-P`
3. config inline `event_loop.prompt`
4. config `event_loop.prompt_file`
5. selected `prompt_template`
6. selected idle bootstrap strategy

结果:

- 自带 prompt 的 workflow 可以直接跑
- 需要任务输入的 workflow 在缺少任务文件时,不再直接崩溃
- 可转入 bootstrap prompt / idle mode / 提示用户补任务输入

**理由**

当前“默认 prompt = `PROMPT.md`”只是历史默认,不是合理抽象。

### 7) examples 默认不是 workflow selector 候选

**选择**

`example_bundle` 进入 catalog,但默认 `selector_eligible=false`。

examples 主要用于:

- `ralph init` / materialize
- 文档演示
- 显式运行模板

不默认参与“用户没给 config 时我该选哪套 workflow”。

**理由**

examples 往往带有场景材料、README、固定目录结构。  
它们更像 bundle/template,不是默认工作流基座。

### 8) selector 路线明确分成 v1 / v2,但当前 change 先落地 v1

**选择**

- v1:
  - 纯规则 selector
  - 只使用结构化 metadata、用户默认项、工作区信号、显式 CLI 线索
- v2:
  - 规则优先
  - 当规则无法收敛时,允许进入 LLM fallback 选择
  - fallback 只能在 catalog 边界内挑选,不能绕过结构化 merge 规则

**理由**

- 用户已经明确要求 v1 / v2 都要正式记录,避免后续上下文丢失时只剩“先做了个 v1”。
- 当前 startup change 的第一目标是把无文件启动和 catalog 闭环做稳。
- selector 一上来就做成 LLM-first,容易把 startup bootstrap 变成第二套 orchestrator。

## Boundary With Runtime Capability Invocation

用户现在想要的“workflow / hat 像 skill 一样被 `ralph#1` 在运行时调用”不是 startup bootstrap 的子问题。

它和当前 change 的关系应该是:

- 当前 change:
  - 提供 catalog
  - 提供结构化 metadata
  - 提供 startup selector
  - 提供 resolved config artifact
- 后续 runtime capability change:
  - 复用这套 metadata
  - 让 `ralph#1` 先看轻量 capability 摘要
  - 再按需调用 workflow capability 或 hat capability
  - 但不能直接热切换当前 live topology

这样分层后:

- startup bootstrap 仍然保持“先 resolve,再启动”
- runtime capability 也不会被迫去改写当前 `EventLoop` / `Supervisor` 的初始化假设

## Architecture

### Startup Resolution Flow

```text
ralph run
  -> parse explicit CLI sources
  -> resolve resource root
  -> ensure user resource sync
  -> load catalog
  -> if explicit config source exists:
       parse explicit source
     else:
       run bootstrap selector
       emit resolved config
  -> resolve prompt source
  -> validate final config
  -> start real EventLoop / Supervisor
```

### Resource Layout

逻辑布局:

- `resources/catalog/*.toml` or `*.yml`
- `resources/workflows/*.yml`
- `resources/backends/*.yml`
- `resources/prompts/*.md`
- `resources/examples/**`

materialized 到用户目录后仍保留同构布局,便于覆盖与调试。

### Resolved Config Artifact

bootstrap selector 的产物不是“直接在内存里改一点点”,而是一份可审计的 resolved artifact。

最小要求:

- 记录选中了哪些资源
- 记录 merge 顺序
- 记录最终 `RalphConfig`
- 可被 doctor / debug / future replay 查看

建议路径:

- workspace 运行期证据: `.ralph/resolved-config.yml`
- selector 决策摘要: `.ralph/bootstrap-selection.json`

## Risks / Trade-offs

- [Risk] 首次同步 embedded 资源后,用户修改与后续版本更新的关系变复杂
  - Mitigation: 引入 manifest/version + “不自动覆盖用户已改文件”的策略
- [Risk] selector 过度智能化,变成第二套 orchestrator
  - Mitigation: v1 先只做规则驱动,v2 才允许规则优先 + LLM fallback
- [Risk] 资源 catalog 太自由,导致后续维护成本上升
  - Mitigation: 先强约束资源类型与组合角色
- [Risk] “无 `PROMPT.md`”语义不清导致 headless 行为混乱
  - Mitigation: 明确 bootstrap prompt / idle mode / explicit task input 三类路径
- [Risk] 用户把运行时 capability 诉求和 startup selector 混在一起,最后 scope 爆炸
  - Mitigation: 在本 change 中显式写清 startup-only 边界,并用独立 follow-up change 承接 runtime capability

## Migration Plan

1. 引入 resource root resolver 与 embedded bundle manifest
2. 把现有 embedded presets / minimal presets / prompt templates 注册进 catalog
3. 引入 startup bootstrap selector v1(纯规则),但先只支持单 workflow + 单 backend
4. 统一 prompt source resolver,去掉“默认 `PROMPT.md` 缺失即硬失败”的单点假设
5. 再逐步开放 overlay 与更多 catalog 元数据
6. 在 follow-up change 中复用 catalog metadata,建设 runtime workflow / hat capability invocation

## Open Questions

- 用户资源目录是否需要显式的 `ralph catalog sync` / `ralph catalog doctor` 子命令?
- `resolved config` 是否要允许用户显式保存回工作区 `ralph.yml`?
