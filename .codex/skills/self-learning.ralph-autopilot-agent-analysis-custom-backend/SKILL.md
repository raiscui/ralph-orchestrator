---
name: self-learning.ralph-autopilot-agent-analysis-custom-backend
description: |
  修复/排查 `ralph autopilot run|analyze` 在 agent analysis 阶段因 custom backend 配置不完整而失败的问题。
  适用场景: (1) report.json 或 stderr 出现 "Custom backend requires a command" 或 "backend=custom,但 cli.command 缺失";
  (2) 子进程 `ralph run --config <out_dir>/analysis_ralph.yml --no-tui` 在 config validate 阶段退出;
  (3) 子进程 exit code=2(护栏退出)但 stdout 已包含 `<event topic="analyze.complete">...`。
author: Codex CLI
version: 1.0.0
date: 2026-02-19
---

# autopilot: agent analysis + custom backend 失败排查与修复要点

## 问题

`ralph autopilot` 的 agent analysis 这一步是通过子进程执行的:

- 父进程: 生成证据包 `analysis_input.json`
- 子进程: `ralph run --no-tui` 读取最小分析配置 `analysis_ralph.yml`
- 子进程 stdout: 必须输出一次 `<event topic="analyze.complete">...JSON...</event>`

当主配置使用 `cli.backend=custom` 时,如果派生出来的 `analysis_ralph.yml` 只写了 backend,
但丢失了 `cli.command/cli.args`(以及 prompt 传参相关字段),
子进程会在 config validate 阶段直接失败,表现为:

- "Custom backend requires a command - set 'cli.command' in config"

这类失败很误导.
你看起来是在“agent 分析输出不对”.
但根因其实是“派生 config 不完整导致子进程根本没跑起来”.

## 上下文 / 触发条件

满足任意一条就用这个 skill:

1. 你运行 `ralph autopilot run` 或 `ralph autopilot analyze`,hard verdict 已 PASS,但 agent analysis 失败.
2. out_dir 里存在 `analysis_ralph.yml`,但子进程报 custom backend validate 错误.
3. 你在 CI/脚本里用 `backend=custom + command=codex + args=[...]` 这种“自定义后端”表达方式.

## 解决方案

### 1) 先用证据定位到底是哪一步炸了

在 out_dir 里优先看这些文件(按顺序):

1. `report.md`: 先确认 hard verdict 与 agent analysis 是否进入.
2. `report.json`: 看 exit_code 语义与 reason.
3. `analysis_ralph.yml`: 看是否包含 custom backend 的完整字段.

你要验证的核心事实是:

- `analysis_ralph.yml` 的 `cli.backend: "custom"` 时,必须同时包含:
  - `cli.command`
  - `cli.args`(如有)
  - `cli.prompt_mode` / `cli.prompt_flag`(保持与主配置一致,避免 prompt 传参方式漂移)

### 2) 代码侧的正确修复形态(防回归心智模型)

如果你在代码里再次实现“派生 config + 启动子进程”这种逻辑,优先遵循这个原则:

> 派生配置时,默认继承完整 `cli` 表,只允许做最小覆盖(例如仅覆盖 backend 字段).

本仓库里对应的落点是:

- `crates/ralph-cli/src/autopilot.rs`
  - `run_agent_analysis()`: 从主配置加载 `cfg.cli`,仅在 `--analysis-backend` 时覆盖 `cli.backend`.
  - `build_min_analysis_config_yaml()`: 当 backend=custom 时,把 `command/args/prompt_mode/prompt_flag` 一并写入 YAML.

### 3) exit code=2 的处理原则(结构化产物优先)

agent analysis 子进程可能因为护栏退出:

- `max_iterations` / `max_runtime_seconds`
- 退出码通常为 2

但 autopilot 真正需要的“核心产物”是:

- `<event topic="analyze.complete">...</event>` 的 JSON payload

因此更稳健的判定方式是:

1. 先从 stdout 解析 `analyze.complete` JSON
2. 若 JSON 可解析成功:
   - exit code=2 视为 warning(记录 stderr_len 等信息即可)
   - 仍判定 agent analysis 成功
3. 若 JSON 不存在或不可解析:
   - 才把非 0 退出码当作硬失败

## 验证

建议至少验证一条“代码级回归信号”:

- `cargo test -p ralph-cli`

如果你修改了并行/事件链路,额外跑:

- `cargo test -p ralph-core smoke_runner`

## 示例(最小 custom backend 配置片段)

```yaml
cli:
  backend: custom
  command: codex
  args:
    - exec
    - --full-auto
```

## 备注

- 这个问题的本质不是“模型不听话”,而是“派生配置丢字段”.
- 一旦你允许 `cli.backend=custom`,你就必须把 `cli` 当成一个整体看待.
  只拷贝 backend 字符串,长期一定会再次踩坑.

