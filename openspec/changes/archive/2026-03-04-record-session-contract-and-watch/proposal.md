## Why

我们之前的 E2E 主要验证的是"业务语义"是否正确,例如 topic 链路,路由,completion.
但你这次踩到的问题属于"证据流(record-session JSONL)的落盘契约".
它在 run-to-completion 的 E2E 里很难被撞出来.

典型表现是:

- E2E 全绿.
- 真实使用时按 Ctrl+C 或终端关闭后,你第一时间打开 `--record-session` JSONL.
- 文件尾部缺失关键证据,看起来像"只看到 human.message,没有 reply".

要规避这类问题,需要两类 backpressure:

1) contract tests. 专门卡住"写完是否可见,中断后是否仍是可解析 JSONL".
2) watch 工具. 把"运行中/刚中断"时的黑盒状态,变成可扫一眼的可观测信号.

## What Changes

- 新增 `ralph record` 命令族(面向人类排障,不改变 `ralph run` 语义):
  - `ralph record summary <FILE>`: 对一份已完成的 record-session JSONL 做摘要输出.
  - `ralph record watch [FILE]`: 跟随增长中的 JSONL,默认以 raw 方式输出新增的完整 JSONL 行(像 `tail -f`),并容忍末尾半行 JSON.
- 强化 record-session 的 contract backpressure:
  - 增加一条 SIGINT/SIGTERM 的集成测试,覆盖"可控中断"路径.
  - 把"JSONL 必须逐行可解析"与"关键 meta 必须存在"固化为可回归断言.
  - 明确 Ctrl+C 退出语义: 优先写入 `_meta.termination` 并 flush,再退出(证据完整优先).
- 降低解析逻辑漂移:
  - 抽离一份通用的 record-session strict 解析/聚合逻辑,供 `autopilot` 与 `record summary` 复用.
  - `record watch` 默认 raw follow,不强依赖 JSON schema,避免为“看证据是否在增长”引入额外耦合.
- 便捷定位:
  - 启用 `--record-session <FILE>` 时,在 workspace_root 的 `.ralph/record-session.latest` 写入 `<FILE>` 指针,
    使 `ralph record watch` 在不传 `FILE` 时也能自动定位最近一次录制.

## Capabilities

### New Capabilities

- `record-session-contract-and-watch`: 用 contract tests 锁定 record-session 的可审计性,并提供 `record watch` 作为快速排障入口.

### Modified Capabilities

- `autopilot` 的 record-session 解析逻辑会迁移到共享模块(行为不变,只减少重复与漂移).

## Impact

- 受影响代码区域(预计):
  - `crates/ralph-cli`: 新增 `record` 子命令,并抽离 record-session 解析模块,补齐 SIGINT 集成测试.
  - `crates/ralph-core`: 不强制改动. 若需要,只做最小的 record 类型/解析复用增强.
- 受影响文档:
  - delta spec: 记录 record-session contract,以及 `record summary/watch` 的 CLI 行为.
