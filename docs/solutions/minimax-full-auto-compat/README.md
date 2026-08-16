---
title: minimax provider 不支持 `--full-auto` flag
problem_type: provider_compatibility
symptoms:
  - 跑 minimax profile + MiniMax-M3 的 E2E 场景时报 "unknown flag: --full-auto"
  - 失败的 5 个 declarative YAML 场景 stderr 含 minimax CLI reject 行
root_cause: minimax provider 是 OpenAI codex CLI 的子集 wrapper，不透传 `--full-auto` 组合 flag
fix_branch: main
fix_commits:
  - e2977175: fix(e2e): replace --full-auto with --sandbox danger-full-access for minimax compatibility
discovered: 2026-08-16
applies_to: ralph-e2e declarative YAML scenarios using `codex exec` custom backend
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
| `crates/ralph-e2e/scenarios/parallel-hat-instances.yaml` | `- --full-auto` → `- --sandbox` + `- danger-full-access` |
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

workspace 证据保留在 `.e2e-tests/parallel-hat-instances*/` 4 个目录
(events.jsonl + agents.json + ralph.yml 都完整)。

## 兼容性

- 不破坏 default Codex profile (用真 OpenAI Codex account 时 `--full-auto` 仍可用)
- 不破坏 minimax 之外的 provider (其他 provider 透传 `--full-auto`)
- 不引入新依赖
- 不改 Rust 代码 (declarative YAML 是 source of truth，code-defined
  legacy Rust 文件仍有 `--full-auto` 残留但已不再 Imperative注册， 不会跑)

## 后续

- **5 个 Rust code-defined legacy 文件** (dead code，不再 Imperative 注册) 仍
  残留 `--full-auto`:
  - `crates/ralph-e2e/src/scenarios/parallel/hat_instances.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/emit_spawn_instance.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/starting_event_inference.rs`
  - `crates/ralph-e2e/src/scenarios/parallel/mod.rs`
  - `crates/ralph-e2e/src/scenarios/parallel_trigger_routing_example.rs`
  - 跟随 Wave 3.4 物理删除 imperative struct 时一起清理
  (LATER_PLANS 已跟踪，Wave 3.4 NO-GO 状态前不动)
- **新增场景 checklist**: 任何走 minimax profile 的 declarative YAML 必须
  用 `--sandbox danger-full-access` 替代 `--full-auto`。
- **minimax `--ask-for-approval` / `--json`**: 矩阵中标 `⚠️` 的 flag 如要
  使用，需先做 live 测试再行。
