## 2026-02-07 13:34 +0800 | WORKLOG 续档

- 旧文件因超过 1000 行已归档为 `WORKLOG_$(date '+%Y-%m-%d_%H%M').md`。
- 新日志从本文件继续追加。

## 2026-02-07 13:33 +0800 | 按用户要求: 默认 `ralph hats graph` 也改为拆分多边

- 用户要求: 不要 `experiment.complete / +3 more`,不要合并,要拆分多边。
- 实施:
  - 移除 physical view 下 TerminalPretty 的 Ralph 边折叠逻辑。
  - 统一 Strict/TerminalPretty 均输出一 topic 一条边。
  - 删除摘要函数与相关测试,并改写 TerminalPretty 回归测试为“必须拆分多边”。
- 验证:
  - 定向测试通过(Strict/TerminalPretty 两条)。
  - `cargo build --release` 通过。
  - `cargo test` 全量通过。
  - `cargo fmt --check` 通过。

## 2026-02-08 15:24 +0800 | git 提交: hats graph 默认拆分 Ralph 多 topic 边

- 变更点:
  - hats graph(physical view): Strict/TerminalPretty 均保持一 topic 一条边,不再折叠 Ralph 多 topic.
  - AsciiRenderOptions: 统一构造入口,用 Default + 覆盖的方式消除 clippy needless_update,并补回归测试.
  - WORKLOG: 超 1000 行续档,旧文件已归档为 WORKLOG_2026-02-07_1552.md.
- 验证:
  - cargo fmt --check ✅
  - cargo clippy --all-targets --all-features -- -D warnings ✅
  - cargo test ✅

## 2026-02-11 00:26 +0800 | git 提交: 归档四文件历史版本 + 更新 example PROMPT

- 变更点:
  - 将根目录历史四文件版本文件移动到 `archive/`(git rename),降低根目录噪音,便于后续检索.
  - `task_plan.md` 已按"超过 1000 行续档"规则重新开始,旧版本存档为 `archive/task_plan_2026-02-11_001538.md`.
  - `examples/parallel-experimental-dev-engine/PROMPT.md` 从 TODO 模板更新为一份具体的实验目标与约束示例.
- 验证:
  - `cargo fmt --check` ✅
  - `cargo clippy --all-targets --all-features -- -D warnings` ✅
  - `cargo test` ✅
- 提交:
  - `2b9e508 chore: archive four-file history and update example prompt`
- 备注:
  - 当前仍存在未跟踪文件:`examples/parallel-experimental-dev-engine/PROMPT copy.md`(本次未提交).
