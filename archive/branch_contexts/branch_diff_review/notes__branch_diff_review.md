## [2026-08-11 12:48:00] [Session ID: omx-1786419140441-df5ql8] 笔记: b3bbe91e vs e88b7e3 分支差异分析

### 分支拓扑

- merge-base: `1d90c1e feat(prompt): add OBJECTIVE section to prevent goal drift (#103)`
- 一端(本地 main HEAD): `b3bbe91e docs: 候选6 收尾完成`,147 commits 之后
- 另一端(origin/main): `e88b7e3 feat(cli): add ralph clean --events`,248 commits 之后
- 总规模: 1818 文件,约 24 万行净变更 (180885+/237849-)

### 两边独有文件分布

- origin/main 一边独有 176 个文件,几乎全部在 `.ralph/specs/`(132)和 `.ralph/tasks/`(43) + 1 个 crates doc
- 本地 main 一边独有 176 个文件,主要在 `specs/`(openspec 风格的版本化目录)和 `tasks/*.code-task.md`
- 这两个独有集合**实际是镜像关系**:同一份 spec/tasks,只是上游放在 `.ralph/specs/`(运行时 scratchpad),本地放到 `specs/`(openspec 治理目录)
- 两边**没有 0 个真正的 crates 模块代码独有** — 全部核心模块都被两边同时改动

### 两边核心交付的主题差异

#### origin/main 一边 (248 commits) 的主线

| 主题 | 数量 | 关键 commit |
|------|------|-------------|
| feat(cli) | 10 | e88b7e3 / ee9fa67 / 246336f / ... |
| fix(adapters) | 5 | 4a38b8d / 192f5f9 / ... |
| fix(ci) | 6 | 2cfe7c9 / 685526d / ... |
| feat(tui) | 3 | 317266f / 3454c62 / ... |
| feat(api) | 2+1 | 6972444 robot RPC / 0b61a78 MCP dedup |
| feat(loops) | 2 | 93e170d publish remote review |
| feat(backends) | 1 | 2cfe7c9 Forge CLI |
| feat(hats) | 2 | 25afeb0 local hat imports |
| feat(telemetry) | 1 | d631ef7 context window |
| feat(web) | 2+1 | 3f1e0c3 file-backed web / 6f35575 improve ralph web UX |

跨版本演进:v2.2 → v2.3 → v2.5 → v2.9 → v2.10
关键发布:release 2.10.0 → 2.10.1 (PR #341)

#### 本地 main 一边 (147 commits) 的主线

| 主题 | 数量 | 关键 commit |
|------|------|-------------|
| feat(e2e) | 10 | b9d909d YAML pilot / c4d2044 human-approval-gate / ... |
| fix(e2e) | 5 | 6b4c175 profile_args 缩进 / 231372e payload 断言 |
| chore(openspec) | 4 | 多个候选阶段记录 |
| feat(core) | 3 | 3ff4b47 EventLoop 收窄 + PromptExecutor port |
| refactor(e2e) | 2 | cc87a18 清理未用 import / 4ac05ad clippy |
| refactor(core) | 2 | 3ff4b47 大头 / ... |
| feat(display) | 1 | 大概率是 TUI 渲染层 |
| feat(parallel) | 1 | parallel loop |
| docs(openspec) | 3 | multiple |

主线脉络: cherry-pick 上游 fix (UTF-8 / TUI hang / mock e2e) → 候选5 平台化 → 候选6 declarative e2e 大头 (single-iter / inject / steer / hat-instances / 23 example) → minimax 全量 live 验证。

#### 双方都改的核心模块冲突热点

- `crates/ralph-cli/src/main.rs`: 两边都有 1600+ 行改动 (本地 +1615/-2714, origin +2695/-1596)。结构性变化,非简单反向。
- `crates/ralph-cli/src/lib.rs`: origin/main 一边 +175 行 (e88b7e3 的 clean_events/event_artifacts);本地 main 一边 0 改动。这意味着本地没动 lib.rs,cherry-pick e88b7e3 完全无冲突。
- `crates/ralph-core/src/lib.rs`: 两边各 +82/-106 与 +106/-82,典型结构性交叉改动。
- `crates/ralph-adapters/src/*`: 22 个文件两边都改 (acp_executor、cli_backend、cli_executor、codex_env、copilot_stream、job/mod 等)。
- `crates/ralph-api/src/main.rs`: origin 一边 +22 行,本地一边 -22 行 — 看起来是反向冲突。
- `crates/ralph-e2e/src/runner.rs`: 本地 +87/-197,origin +197/-87 — 也是反向冲突模式。
- `crates/ralph-tui/*`: 大量 widgets/state/changes,两边并行改动。

### 关键观察

1. **本地 main 一边 0 个 crates 模块独有**:意味着本地 main 的所有代码改动都「沉淀到了与 origin/main 共享的代码空间」。这是 high-merge-conflict-risk 信号。

2. **本地 main 的核心架构调整**: `3ff4b47 refactor(core): EventLoop 收窄 + PromptExecutor port`,把 run_loop_impl 从 1247 行收窄到 565 行。这是个深 refactor,影响所有走循环链路的代码。

3. **本地 main 的代表性产品**: declarative e2e framework (`crates/ralph-e2e/src/declarative/`),把"测试 = 代码"改成"测试 = YAML 数据"。

4. **origin/main 的代表性交付**: robot RPC domain (`crates/ralph-api/src/robot_domain.rs`),有完整的 v1 schema,关闭 #243。

### 值得跟进(origin/main 一边可 cherry-pick 的、按价值/风险分)

#### 高价值低风险(本地 main 没碰过,合并即用)

- [PICK-NOW] **`e88b7e3` feat(cli): add ralph clean --events** — 148 行 + 11 行 + 8 行,本地 lib.rs 0 改动,直接 cherry-pick
- [PICK-NOW] **`db48462` fix: canonicalize Ralph artifact paths** — 路径规范化,正交于本地 main 的 event_loop 修改
- [PICK-NOW] **`01dd250` fix(api): inline MCP tool schema roots** — MCP 兼容性,不动本地 main 关心的 e2e/declarative 路径
- [PICK-NOW] **`192f5f9` fix(adapters): drain ACP terminal output before exit** — adapter 终结路径修复,与本地 main 的 3ff4b47 EventLoop 修改正交

#### 中价值中风险

- [PICK-LATER] **`4a38b8d` fix(adapters): wait for Claude stream result events** — fix Claude 流式结果等待。涉及 claude_stream.rs / cli_executor.rs,与本地 main 的 declarative e2e 测试 harness 同区域。需做局部 merge。
- [PICK-LATER] **`cf0ec8d` test: isolate event history payload fixture** — 测试 fixture 隔离,只动测试代码不 production。
- [PICK-LATER] **`0207c8b` fix(event-loop): persist continue state** — EventLoop continue 状态持久化,与本地 main 的 3ff4b47 同区域。会冲突但可解决。

#### 高价值高风险(动 main.rs / ralph-api / ralph-tui,合并冲突成本大)

- [PICK-LATER] **`6972444` feat(api): add robot RPC domain** — 完整 v1 RPC 协议,动 6 个 ralph-api 文件 + protocol.rs。本地 main 也改 ralph-api,合并不小。
- [PICK-LATER] **`2cfe7c9` feat(backends): Forge CLI support** — 动 adapters/auto_detect.rs。本地 main 也改 adapters。
- [PICK-LATER] **`93e170d` feat(loops): publish remote review branches** — 动 loop_domain。本地 main 重构过 event_loop。
- [PICK-LATER] **`25afeb0` feat(hats): support local hat imports in preflight** — 动 hats。本地 main 没动 hats,可能可直接应用,但 verify。

#### 不建议 cherry-pick(破坏本地 main 已有工作或与 declarative 冲突)

- **跳过**: `317266f fix(tui): capture keystrokes in search input mode` — 本地 main 也有 TUI 修改,会冲突且难以赢得合并战
- **跳过**: `3454c62 feat(tui): export iteration buffers on demand` — 同上
- **跳过**: `d631ef7 feat(telemetry): track context window utilization` — 动 prometheus/otel metrics,本地 main 没动,但与 declarative e2e 互斥
- **跳过**: `0b61a78 fix(api): deduplicate MCP tool schemas` — 该 fix 在 `01dd250` 之后才需要,可考虑一起 cherry-pick

### 值得补丁(对本地 main 已有工作的隐患)

#### A. 本地 main 一边 declarative e2e 的「逃生舱」模式

- **现象**: `b9d909d` 明确说「命令式 struct 保留为逃生舱」
- **风险**: 23+ 场景已迁移到 YAML,但 imperative 路径仍然是默认/兼容路径,长期容易分裂
- **建议**: 当 declarative 覆盖度足够后(比如 90%+),就把 imperative 标记 deprecated,防止 drift

#### B. `3ff4b47 refactor(core)` 的回归测试覆盖

- **现象**: run_loop_impl 1247 → 565 行,引入 `PromptExecutor` port
- **风险**: port 接口契约如果没有完整测试,后续 cherry-pick 上游循环相关 commit 时容易破坏
- **建议**: 跑一次 `cargo test --all-features`,确认 PromptExecutor 的 contract test 完整

#### C. e2e runner.rs 的不变量

- **现象**: ralph-e2e/src/runner.rs 一边减 197 行,一边加 87 行(=净减 110)
- **风险**: 如果 origin/main 一边是早期版本,本地 main 删了 197 行意味着减掉历史功能
- **建议**: 验证目前 declarative 路径是否覆盖原 imperative 路径的全部断言能力

#### D. local main 一边的 spec/openspec 目录与 origin/main 的 .ralph/specs 共存

- **现象**: 两边都在维护 spec,但完全独立的目录
- **风险**: 同一份规范在两套位置维护,容易 drift
- **建议**: 决定一方为 canonical,另一边按需同步;或将 spec 内容 setup 为 openspec-mergeable

#### E. local main 一边 ralph-api/src/main.rs 反向 22 行

- **现象**: origin/main 一边 +22,本地 main 一边 -22
- **风险**: 这是反向冲突,意味着本地 main 删了 origin/main 加的东西
- **建议**: 看具体行,确认是「不需该 feature」还是「该 feature 已经迁移到别处」

### 推荐执行路径(按价值/风险比排)

| 顺序 | 项 | 价值 | 风险 | 注释 |
|------|-----|------|------|------|
| 1 | cherry-pick `e88b7e3` clean --events | 高 | 低 | 已分析,可直接做 |
| 2 | cherry-pick `db48462` 路径规范化 | 中 | 低 | 正交,无冲突 |
| 3 | cherry-pick `192f5f9` ACP drain | 中 | 低 | 单文件 |
| 4 | cherry-pick `01dd250` MCP schema root inline | 中 | 低 | 兼容性修复 |
| 5 | 评估并 cherry-pick `4a38b8d` Claude stream wait | 中 | 中 | 需要在 claude_stream 上合并 |
| 6 | rebake `6972444` robot RPC | 高 | 高 | ralph-api 改动大,本地也需要 RPC |
| 7 | rebake `2cfe7c9` Forge CLI | 中 | 中高 | adapters 改动 |

### 关键决定

- [决策]: 不在本次合并;用户问题只是「分析这个 commit 之间的差异有什么值得跟进/补丁」
- [决策]: 推荐 cherry-pick 顺序是高价值低风险的 4 项最先做(commit 1-5)
- [决策]: robot RPC 和 Forge CLI 推荐作为后续 sprint 重做,而不是直接 cherry-pick(因为本地 main 也改了相关文件)
- [决策]: 不动本地 main 独有的 declarative e2e 框架,因为那是它的核心交付


---

## [2026-08-12 14:10:00] [Session ID: omx-1786419140441-df5ql8] 笔记: 实际 cherry-pick 验证:e88b7e3 不是零风险

### 实测:`git cherry-pick e88b7e3` 失败原因

执行 `git cherry-pick -x e88b7e3` 触发 3 个冲突:

1. **`crates/ralph-cli/src/lib.rs`** — e88b7e3 patch anchor `@@ -79,6 +79,94 @@` 假设 lib.rs 第 79 行附近是 `Ok(()) }`,但本地 HEAD lib.rs 第 79 行就是 `Ok(()) }` 然后 EOF。patch 想在 `Ok(()) }` 后追加 `event_artifacts + clean_events + 测试模块`,但 anchor 错位触发冲突。
2. **`crates/ralph-cli/src/main.rs`** — `clean_command` 函数签名不兼容:
   - 本地 HEAD: `fn clean_command(config_path: PathBuf, color_mode: ColorMode, args: CleanArgs) -> Result<()>`
   - e88b7e3: `fn clean_command(config_sources: &[ConfigSource], color_mode: ColorMode, args: CleanArgs) -> Result<()>`
   - 内部还用了 `config.core.scratchpad.path`,本地 HEAD 的 `RalphConfig.core.scratchpad` 是 `pub scratchpad: String`(无嵌套)。
3. **`docs/guide/cli-reference.md`** — clean 段描述和选项表完全替换。

### 真实依赖链(按提交顺序)

`git log -S 'struct ConfigSource' ... -- crates/ralph-cli/src/main.rs` 找到 `37d66c9 feat: add CLI ergonomics (backend flag, builtin presets, URL configs)` 引入了 `ConfigSource` enum。
`git log -S 'scratchpad.path' ... -- crates/ralph-cli/src/main.rs` 找到 `041e9f7 feat(loops): refactor scratchpad configuration to support per-hat (#186)` 引入了嵌套的 `Scratchpad.path` 字段。

所以 e88b7e3 实际依赖:
1. `041e9f7` scratchpad 字段重构(per-hat)
2. `37d66c9` ConfigSource 引入
3. 中间把 `RalphConfig::from_file(&config_path)` 迁移到 `load_config_with_overrides(&[ConfigSource])` 的所有 commit
4. `ee9fa67` (e88b7e3 的 parent)
5. `e88b7e3` 自身

### 评估修正

- **之前**:`e88b7e3` 标为「零风险 cherry-pick」(Group 1)
- **现在**:应移到 Group 3(中风险,需要 cherry-pick 整条上游 commit 链)
- **或重做**:跳过整条链 cherry-pick,在本地 main 上**手动实现** `ralph clean --events`,保留本地 HEAD 的 `RalphConfig::from_file(&config_path)` 默认行为,只插入 `args.events` 分支。

### 后续决策点

- 是否要把 ticket 03 重新规划为整条链的 cherry-pick?
- 还是要把它降级为「手动实现 ralph clean --events」(Ponytail 推荐,代码量最小)?
- notes/proposal/tickets 三处文件需要同步更新。
