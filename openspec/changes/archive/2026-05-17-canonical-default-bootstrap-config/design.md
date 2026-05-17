## Context

当前 `resource-bootstrap` 稳定 spec 只要求默认 startup bootstrap 在无 `ralph.yml` / 无 `PROMPT.md` 时成功启动,并要求 `.ralph/resolved-config.yml` 包含 `parallel.enabled=true`。实现层因此把默认 bootstrap workflow 固定在 `workflow:feature-minimal`,再在解析后强制打开 `parallel.enabled`。

这条路径解决了“能启动”的问题,但没有解决“默认运行语义由谁负责”的问题。与此同时,项目根 `ralph.yml` 已经演进成 `custom + codex + parallel` 的默认并行配置,所以用户自然会把它视为当前产品默认行为。两条真相源并存,就导致 no-config bootstrap 与项目主配置发生漂移。

## Goals / Non-Goals

**Goals:**
- 让 no-config startup bootstrap 的默认运行语义与当前 canonical default config 一致。
- 建立单一 canonical source,避免 embedded bootstrap resource 与项目根 `ralph.yml` 长期分叉。
- 扩大 startup bootstrap 的验证面,让 focused tests 和 live gate 都锁定关键字段。
- 为 root `ralph.yml` 与 canonical embedded resource 增加机械 drift gate。
- 保持现有 bootstrap 两阶段架构不变: 先 resolved config,再启动真实 run。

**Non-Goals:**
- 不让安装后二进制在运行时依赖源码仓库根 `ralph.yml` 路径。
- 不在本 change 中引入新的 runtime capability invocation 语义。
- 不改变显式 `--config ...` 绕过 bootstrap 的规则。
- 不要求 `.ralph/resolved-config.yml` 与 `ralph.yml` 完全字节一致; 默认展开字段仍可来自 `RalphConfig` 默认值。
- 不把所有当前 repo 默认值都提升为长期 stable spec 常量。

## Decisions

### 1. Canonical source 必须是内置 startup resource,不是运行时直接读取仓库根 `ralph.yml`

- 选择:
  - 新的默认 bootstrap 语义必须由一份 embedded startup resource 承担。
  - 项目根 `ralph.yml` 作为仓库可编辑入口,需要与该 embedded resource 保持语义同步。
- 理由:
  - 安装后二进制无法稳定依赖源码仓库根路径。
  - startup bootstrap 既然已经走 embedded resource catalog,继续沿这条路径最稳定。
- 备选方案:
  - 直接把仓库根 `ralph.yml` 当运行时 source。
  - 放弃原因: 安装路径不稳定,不符合发布后的运行边界。

### 2. Default bootstrap selector 必须改指向新的 canonical workflow resource

- 选择:
  - 将 `DEFAULT_BOOTSTRAP_WORKFLOW_ID` 从 `workflow:feature-minimal` 切换到新的 canonical default bootstrap resource。
- 理由:
  - 当前 drift 的直接根因就是 selector 仍指向旧 preset。
  - 只在解析后局部 patch `cli` 字段会制造第二套隐式 merge 规则,不够干净。
- 备选方案:
  - 保留 `feature-minimal`,在 `resolve_workflow_with_prompt_template()` 里额外覆盖 `cli` / `parallel.autoscale`。
  - 放弃原因: 会把 canonical source 再次拆成 preset + hardcoded patch 两半。

### 3. Stable spec 锁字段级 contract,具体默认值交给 gate 锁定

- 选择:
  - stable spec 只要求 resolved bootstrap artifact 与 canonical startup resource 在用户可见的 bootstrap runtime fields 上语义一致。
  - 最小字段集合包括:
    - `cli.backend`
    - `cli.command`
    - `cli.prompt_mode`
    - `cli.args`
    - `parallel.enabled`
  - 像 `parallel.autoscale.max_running_jobs` 这类更像 repo 当前默认策略的具体值,通过 focused/live/drift gate 锁定,不直接硬编码到长期 stable spec 常量。
- 理由:
  - resolved artifact 会经过 `RalphConfig` 序列化,天然包含默认展开字段。
  - 真正稳定的契约是“哪些字段必须对齐”,不一定是“这些字段永远取当前这一组值”。

### 4. 必须增加 root `ralph.yml` 与 canonical embedded resource 的机械 drift gate

- 选择:
  - 增加一条 repo-owned drift gate,使用同一 helper 或同一 assertion surface 比较:
    - 项目根 `ralph.yml`
    - canonical embedded startup resource
    - 必要时再加 resolved bootstrap artifact
- 理由:
  - 只写“保持同步”还不够,必须有机器能跑的 gate 才能真正防漂移。

### 5. 测试要从“证明能 bootstrap”升级成“证明 bootstrap 语义正确”

- 选择:
  - 更新 `startup_resources` focused unit tests,验证 default resolution 选中的 workflow resource 和关键 `cli` / `parallel` 字段。
  - 更新 `integration_startup_resources` live gate,验证生成的 `.ralph/resolved-config.yml` 与 canonical default config 的关键字段一致。
  - 新增或扩展 drift gate,验证 root `ralph.yml` 与 canonical embedded resource 的关键字段对齐。
- 理由:
  - 当前测试把旧 `workflow:feature-minimal` 锁死了,不改测试就无法真正改契约。

## Risks / Trade-offs

- [Risk] 项目根 `ralph.yml` 和 embedded canonical resource 再次漂移。
  → Mitigation: 用 repo-owned drift gate 比较关键字段,而不是只写文档约束。

- [Risk] 现有用户可能依赖 no-config bootstrap 的 `claude` 默认行为。
  → Mitigation: 这是默认契约修正,不影响显式 `--config` 和显式 backend 选择; change 说明里要明确这是有意的默认行为切换。

- [Risk] 只锁字段级 contract 会让部分具体值漂移到测试层。
  → Mitigation: 让具体默认值由 focused/live/drift gate 锁住,保持 stable spec 稳定、测试证据具体。

## Migration Plan

1. 新增 canonical default bootstrap resource。
2. 修改 selector 默认 workflow 指向。
3. 增加 root `ralph.yml` 与 canonical embedded resource 的 drift gate。
4. 更新 focused / integration bootstrap tests。
5. 复跑 OpenSpec / focused tests / live gate,再进入实现评审。

## Open Questions

- canonical default bootstrap resource 的命名最终采用 `workflow:default-parallel` 还是更贴近产品语义的其他名字?
- drift gate 最终放在 `startup_resources` focused tests 里,还是单独作为 repo-owned integration test 更清晰?
