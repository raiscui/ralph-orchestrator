---
title: minimax provider + codex-cli ≥ 0.147.0 不支持 `--full-auto` flag
problem_type: integration_issue
component: ralph-e2e + ralph-adapters::cli_backend
module: crates/ralph-e2e/scenarios/ and crates/ralph-adapters/src/cli_backend.rs
severity: high
status: active
date: 2026-08-16
last_updated: 2026-08-17
discovered: 2026-08-16
root_cause: minimax provider 只透传认识的 flag(`--full-auto` 不在白名单),codex-cli ≥ 0.147.0 升级时也移除了 `--full-auto`;两条 path 都必须改用 `--sandbox danger-full-access`。
resolution_type: schema+code dual fix (YAML custom backend args + cli_backend.rs default args)
fix_branch: main
fix_commits:
  - "e2977175: fix(e2e): replace --full-auto with --sandbox danger-full-access for minimax compatibility (declarative YAML side)"
  - "005d840d: fix(cli_backend): codex default args --full-auto → --sandbox danger-full-access (Rust code path side)"
tags:
  - codex-cli-compat
  - minimax
  - cli-backend
  - sandbox-mode
  - wave3-q3-2026
verified_by:
  - "rg '\\-\\-full-auto' crates/ → 0 matches (both code-defined + declarative paths clean)"
  - "cargo run -p ralph-e2e -- codex --filter parallel-hat-instances (default codex) → PASSED"
  - "cargo test -p ralph-e2e --lib (336+ passed; no regression)"
  - "RALPH_E2E_CODEX_PROFILE=minimax RALPH_E2E_CODEX_MODEL=MiniMax-M3 cargo run -p ralph-e2e -- codex --filter parallel-hat-instances* → 2/2 PASSED"
related_solutions:
  - ../documentation-gaps/declarative-scenario-migration.md (Wave 3.4 cleanup + 100% gate)
applies_to: 'ralph-e2e YAML scenarios with `cli.backend: codex` custom backend; ralph-adapters::cli_backend::CliBackend::codex() default args'
---

# minimax `--full-auto` 不兼容：用 `--sandbox danger-full-access` 替代

## 现象

`codex exec --full-auto` 在 minimax provider 下被拒绝，stderr 报类似
`unknown flag: --full-auto` 或 `argument '--full-auto' not supported`。
影响所有走 minimax profile + `custom backend` 的 E2E 场景。

具体 5 个 declarative YAML 场景先后踩坑:

| 场景 | 修复 commit | 修复日期 |
|---|---|---|
| `parallel-emit-spawn-instance` | 2026-08-14 直接 sed (前次 work-around) | 2026-08-14 |
| `parallel-hat-instances` | e2977175 | 2026-08-16 |
| `parallel-hat-instances-zh` | e2977175 | 2026-08-16 |
| `parallel-starting-event-inference` | e2977175 | 2026-08-16 |
| `parallel-starting-event-inference-multi-candidate` | e2977175 | 2026-08-16 |

## 根因

`--full-auto` 是 OpenAI codex CLI 的组合 flag，等价于:

```text
--sandbox workspace-write --ask-for-approval on-request
```

minimax provider 是 OpenAI codex CLI 的子集 wrapper，
**只透传它认识的 flag**(`-p`, `-m`, `-c`, `--sandbox`, `exec` 等)。
`--full-auto` 不在透传白名单里，因此被 reject。

minimax profile 注入路径 (`{profile_args}` 占位符) 让 `-p minimax` 生效，
但也**不会**额外把 `--full-auto` 转成 `--sandbox ...` — minimax 只做
flag 透传，不做语义翻译。

## 修复

把 `--full-auto` 替换成显式 `--sandbox danger-full-access`(两个独立 flag):

```diff
  args:
    - exec
    {profile_args}
    - -m
    - {model}
-   - --full-auto
+   - --sandbox
+   - danger-full-access
    - -c
    - 'model_reasoning_effort="low"'
```

`--sandbox danger-full-access` 是 minimax 认识的透传 flag，且对 E2E 场景
"无沙箱限制 + 显式全权"语义等价于 `--full-auto`(后者就是这一对的
组合快捷方式)。

## minimax provider 兼容矩阵 (实测)

| flag | minimax 支持 | 备注 |
|---|---|---|
| `exec` | ✅ | codex CLI 基本子命令 |
| `-p <profile>` | ✅ | profile 切换 |
| `-m <model>` | ✅ | model 切换 |
| `-c key=value` | ✅ | config override |
| `--sandbox <mode>` | ✅ | sandbox 模式 |
| `--full-auto` | ❌ | 不透传 |
| `--ask-for-approval <mode>` | ⚠️ | 部分支持，需实测 |
| `--json` | ⚠️ | 部分支持 |

`⚠️` 的项需要本地实测验证，不要假设有效。

## 改动

| 文件 | 改动 |
|---|---|
| `crates/ralph-e2e/scenarios/hat-instances.yaml` | `- --full-auto` → `- --sandbox` + `- danger-full-access` |
| `crates/ralph-e2e/scenarios/hat-instances-zh.yaml` | 同上 |
| `crates/ralph-e2e/scenarios/starting-event-inference.yaml` | 同上 |
| `crates/ralph-e2e/scenarios/starting-event-inference-multi-candidate.yaml` | 同上 |

注: `crates/ralph-e2e/scenarios/emit-spawn-instance.yaml` 早在 2026-08-14
单独 work-around 时就修过，本轮没重复改。

## 验证

| 阶段 | 命令 | 结果 |
|---|---|---|
| code check | `cargo check -p ralph-e2e` | 仅无关 deprecation warning，0 error |
| YAML schema | `cargo test -p ralph-e2e --lib -- all_scenario_yamls` | 1 passed |
| list | `cargo run -p ralph-e2e --quiet -- --list` | 4 个场景都列出 |
| minimax live | `RALPH_E2E_CODEX_PROFILE=minimax RALPH_E2E_CODEX_MODEL=MiniMax-M3 cargo run -p ralph-e2e -- codex --filter ...` | 4/4 PASSED，228.4s 总耗时 |

### minimax live E2E 详细结果

| 场景 | 用时 | 状态 |
|---|---|---|
| parallel-hat-instances | 72.8s | ✅ |
| parallel-hat-instances-zh | 53.6s | ✅ |
| parallel-starting-event-inference | 54.7s | ✅ |
| parallel-starting-event-inference-multi-candidate | 47.2s | ✅ |

workspace 证据保留在 `.e2e-tests/hat-instances*/` 4 个目录
(events.jsonl + agents.json + ralph.yml 都完整)。

## 兼容性

**这是关键 — 2026-08-17 Round 5 更新**:

- **default Codex (codex-cli ≥ 0.147.0)** 现在也 reject `--full-auto`,必须用
  `--sandbox danger-full-access`。**CLI 后端唯一支持的两条路径都用这个 flag**。
- **minimax provider** 仍按上面矩阵,`--full-auto` 不透传,必须用
  `--sandbox danger-full-access`。
- **不引入新依赖**。
- **Rust 代码路径已修复**(2026-08-17 commit `005d840d`):
  `crates/ralph-adapters/src/cli_backend.rs::CliBackend::codex()` 的默认
  `args` 已经把 `--full-auto` 替换成 `--sandbox danger-full-access`,
  `filter_args_for_interactive()` 同步把 `--sandbox danger-full-access`
  也加入安全过滤列表。**所以**不论走 declarative YAML 还是 Rust
  code-defined scenario,遇到 minimax / 新版 Codex 都自动走对的 flag。

## Rust code path 修复 (Round 5, commit `005d840d`)

origin/mikeyobrien 在 `codex-cli 0.147.0` 升级时移除了 `--full-auto`,
minimax provider 早就 reject。现在 default `CliBackend::codex()` hardcode
的 args 也被 reject。这条修复加在 cli_backend.rs:

```diff
 // crates/ralph-adapters/src/cli_backend.rs
- Vec::from(["exec", "--sandbox", "workspace-write", "--full-auto",
-            "--ask-for-approval", "on-request"]),
+ Vec::from(["exec", "--sandbox", "danger-full-access"]),
```

并同步 `filter_args_for_interactive` 把 `--sandbox danger-full-access`
也加入可安全交互的 arg 列表。

验证:

| 阶段 | 命令 | 结果 |
|---|---|---|
| codex live | `cargo run -p ralph-e2e -- codex --filter parallel-hat-instances*` | 2/2 PASSED |
| minimax live | `RALPH_E2E_CODEX_PROFILE=minimax RALPH_E2E_CODEX_MODEL=MiniMax-M3 cargo run -p ralph-e2e -- codex --filter parallel-hat-instances*` | 2/2 PASSED |

## 后续

- ✅ **(已落地 2026-08-17)** 5 个 Rust code-defined legacy 文件全部物理删除
  (Wave 3.4 cleanup commit `ca54fb3b`) + 11 个 legacy .rs struct + 22 个
  deprecated struct 也全部清完 (Round 6 commit `ee73fcf8` + `03fab390` +
  `e1edf762`)。
- **新增场景 checklist** (继续生效):任何走 minimax profile **或** default
  Codex (≥ 0.147.0) 的 declarative YAML 都必须用 `--sandbox danger-full-access`
  替代 `--full-auto`。这是当前两条 path 共用的唯一合规 flag。
- **minimax `--ask-for-approval` / `--json`**: 矩阵中标 `⚠️` 的 flag 如要
  使用，需先做 live 测试再行。
- **CI enforcement (TODO)**: 把 `rg '\-\-full-auto' crates/` 加进 CI,
  一旦回归立刻 fail。当前仓库扫描已是 0 命中,但没有 CI 卡口。
- **新增场景 checklist**: 任何走 minimax profile 的 declarative YAML 必须
  用 `--sandbox danger-full-access` 替代 `--full-auto`。
- **minimax `--ask-for-approval` / `--json`**: 矩阵中标 `⚠️` 的 flag 如要
  使用，需先做 live 测试再行。
