## Why

当前 no-config startup bootstrap 已经能在缺失 `ralph.yml` / `PROMPT.md` 时启动,但它使用的默认 workflow 仍然是旧的 `workflow:feature-minimal`。这导致生成的 `.ralph/resolved-config.yml` 与当前项目维护的默认并行配置发生语义漂移,用户看到的默认运行行为与仓库主配置不一致。

现在需要把“默认 bootstrap 配置的真相源”重新钉住,避免 startup bootstrap、项目根 `ralph.yml`、live dogfood gate 和后续默认行为继续各自演进。

## What Changes

- 为 no-config/no-prompt startup bootstrap 定义一份明确的 canonical default bootstrap config。
- 修改默认 bootstrap selector,不再指向旧 `workflow:feature-minimal`,而是指向新的 canonical default bootstrap resource。
- 明确 `.ralph/resolved-config.yml` 不要求与项目根 `ralph.yml` 字节级相同,但核心运行语义必须一致。
- 为项目根 `ralph.yml` 与 canonical bootstrap resource 建立机械 drift gate,防止再次漂移。
- 扩展 startup bootstrap focused tests 和 live integration gate,锁定关键 `cli` / `parallel` 字段的语义对齐。

## Capabilities

### New Capabilities
- None.

### Modified Capabilities
- `resource-bootstrap`: 默认 no-config bootstrap 的 canonical config source、resolved config contract 和验证 gate 发生变化。

## Impact

- 受影响代码:
  - `crates/ralph-cli/src/startup_resources.rs`
  - `crates/ralph-cli/presets/` 内置 startup workflow 资源
  - `crates/ralph-cli/tests/integration_startup_resources.rs`
  - `ralph.yml`
- 受影响行为:
  - 无配置目录执行 `ralph run` 时生成的 `.ralph/resolved-config.yml` 将从旧 `claude` 默认转为 canonical default parallel config。
  - startup bootstrap tests 将从“只验证 parallel.enabled=true”升级为验证关键 `cli` / `parallel` 语义对齐。
- 不引入新的 runtime topology mutation,不修改显式 `--config` source 的绕过语义。
