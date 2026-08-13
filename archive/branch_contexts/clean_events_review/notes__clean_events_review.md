## [2026-08-11 12:32:00] [Session ID: omx-1786419140441-df5ql8] 笔记: 分析 commit e88b7e3 (ralph clean --events)

### Commit 性质
- 标题: feat(cli): add ralph clean --events (#357)
- 作者: Mikey O'Brien, 2026-07-25
- 提交位置: 已在 origin/main HEAD,e88b7e3 是 main 当前顶部
- 与 merge-base 关系: 此 commit 不在当前 HEAD 分支链上,当前 HEAD 是从 `1d90c1e` 分叉的独立线
- 重复 commit: `98f4028 feat(cli): add ralph clean --events` 在 main 的其他分支存在,内容与 e88b7e3 几乎一致;e88b7e3 是带 PR 编号的合并版本

### 改动范围
- crates/ralph-cli/src/lib.rs: +93 行(新增 event_artifacts + clean_events + 3 测试)
- crates/ralph-cli/src/main.rs: +11 行(CleanArgs 加 events 字段,clean_command 加分支)
- docs/guide/cli-reference.md: +8 行(--events 选项文档 + 默认行为说明)

### HEAD 当前状态(对比)
- lib.rs: 80 行,只有 clean_diagnostics,没有 event_artifacts / clean_events
- main.rs CleanArgs: 只有 dry_run + diagnostics
- clean_command: 只有 --diagnostics 分支,没有 --events 分支
- docs/guide/cli-reference.md: 仍然说 "Clean up .agent/ directory",无 --events 文档;但旧版有提到 --all(已不存在)

### 行为分析(e88b7e3 本身的实现)
核心流程:
1. event_artifacts: 收集 .ralph/ 下 events.jsonl + events-*.jsonl + current-events marker
2. clean_events: 三个分支(空/无文件、dry-run、实际删除),输出带 colors,使用 anyhow Context 包装 IO 错误
3. CLI 层 CleanArgs.events 字段加 conflicts_with = "diagnostics"
4. clean_command 中新增 if args.events { clean_events() } 分支,沿用 std::env::current_dir() 风格

### 跟进候选(对 HEAD)

#### 1. 移植价值
- 场景: HEAD 分支需要 clean --events 功能么?
- 评估: 如果 HEAD 的工作目标与 main 类似,值得 cherry-pick e88b7e3
- 风险: 与 HEAD 已存在的 clean_diagnostics 调用一致,加上后会增加功能,但 HEAD 也没消除已有的 --all 文档失真问题

#### 2. 与 2cfabdb refactor 的协同
- 2cfabdb refactor: wip dedup pass across cli/core (git helpers, palette, termination labels)
- 位置: 在 e88b7e3 之前的 refactor 链上
- 关联: clean_diagnostics 和 clean_events 都有 if use_colors { color println } else { plain println } 重复模式,后期可作为 refactor 目标
- 建议: 如果同时 cherry-pick 这两个 commit,可以一起做 dedup

### 补丁候选(对 e88b7e3 本身的改进)

#### A. 行为正确性

A1. **workspace_root 解析风格不一致** [main.rs clean_command]
- 现象: --diagnostics 和 --events 都用 `std::env::current_dir()` 当 workspace_root;默认分支从 config.core.scratchpad 推算
- 影响: 在子目录执行 `ralph clean --events` 会删当前目录的 .ralph/,而非 config 推算的 workspace
- 建议: 把 workspace_root 计算统一抽到 clean_command 顶部,所有分支复用

A2. **--diagnostics 和 --events 互斥需要执行两次才能同时清** [cli UX]
- 现象: 两个 flag 互斥,要清两边必须执行两次 `ralph clean`
- 影响: 单次命令原子性差,容易忘
- 建议: 加 `--all-events` 或允许 `--diagnostics --events`(虽然 clap conflicts_with 已加,需要改设计)

A3. **current-events 是文件还是 symlink,语义不清** [lib.rs event_artifacts]
- 现象: 删除 current-events 用 fs::remove_file,假设它是普通文件
- 影响: 如果是 symlink 指向某个 events-*.jsonl,删除会断链;若 symlink 指一个已删文件,会留下死链
- 建议: 在 docs 中明确 current-events 是普通文件,或代码处理 symlink

A4. **event_artifacts 匹配规则假设过强** [lib.rs event_artifacts]
- 现象: `name == "events.jsonl" || name.starts_with("events-")`
- 影响: 用户如果命名 `events-yesterday.jsonl.backup` 之类也会被误删(.jsonl 结尾,但不以 events- 开头?不会——首字母是 events-,会被匹配)。但 `events-yesterday.bak` 不会(.bak 不是 jsonl)
- 评估: 实际误伤概率低,可接受

#### B. 错误处理

B1. **event_artifacts 沉默吞错** [lib.rs event_artifacts]
- 现象: `if let Ok(entries) = fs::read_dir(&ralph_dir) { ... }` 不处理 Err
- 影响: 权限错误、IO 失败完全没提示,用户以为没文件可清
- 建议: 至少打 warn log

B2. **同名文件保留测试虽包含 events-notes.md,但缺更系统的保留测试** [lib.rs tests]
- 已包含: events-notes.md(非 events-.jsonl 也不以 events. 开头)、loops.json(.json 非 .jsonl)、merge-queue.jsonl(非 events- 前缀)
- 缺: 略 events 排除由"events-前缀且.jsonl 后缀"两个条件;若有 events-old.jsonl(大小写)是否匹配?

#### C. 测试覆盖

C1. **缺 --diagnostics + --events 同时传的冲突断言测试** [lib.rs tests]
- 现象: code 用 clap conflicts_with,但 lib 层没测 flag 组合
- 影响: clap 行为不在 lib 测试覆盖范围;需 cli 集成测试覆盖
- 建议: 加 CLI 集成测试,或用 clap::Command::try_get_matches_from 测

C2. **缺权限错误、IO 错误路径测试** [lib.rs tests]
- 现象: clean_events 测试都是 happy path
- 建议: 至少加 read-only 文件的测试

C3. **缺 current-events 不存在但有 jsonl 的场景测试** [lib.rs tests]
- 现有测试 fixture 同时包含 marker 和 jsonl;两者独立场景没测

#### D. 文档

D1. **文档与代码路径不一致** [docs/guide/cli-reference.md]
- 现象: "By default, deletes the whole `.ralph/agent` directory"
- 实际: 默认 clean_command 清理的是 `config.core.scratchpad` 的父目录,默认是 `.agent/scratchpad.md` → `.agent/`,而非 `.ralph/agent/`
- 影响: 用户读文档预期有误
- 建议: 改为 "deletes the `.agent/` scratchpad directory" 或 "deletes the scratchpad directory"

D2. **--all 残留文档失真问题** [docs/guide/cli-reference.md 旧版]
- e88b7e3 移除 --all 后,旧文档确实已清理;但历史 --all 文档与代码曾经的 CleanArgs 不一致(没有 all 字段)
- 状态: e88b7e3 没引入新 --all 文档,这点 OK

D3. **缺 --dry-run 与 --events 组合输出示例** [docs/guide/cli-reference.md]
- 已有 `--dry-run` 选项,但没示例展示 dry run with --events 的输出

#### E. 代码风格(ponytail 评估)

E1. **重复的 use_colors 三元组 println 模式** [lib.rs clean_diagnostics + clean_events]
- 现象: 多处 `if use_colors { print!("{}...{}", colors::X, colors::RESET, ...) } else { println!("..."); }`
- 评估: ponytail 「不要过早抽象」——等第三个 caller 出现再抽 helper
- 行动: 不动

E2. **fs::remove_file 失败上下文可更结构化** [lib.rs clean_events]
- 现象: `.with_context(|| format!("Failed to delete '{}'. Check permissions and try again.", ...))`
- 评估: 合理,够用了

### 关键决定

- [决策]: 不立即 cherry-pick 到 HEAD
  [理由]: 用户当前问题只是问「这个 commit 有什么值得跟进和补丁」,是分析任务,不是直接执行任务;
        且 HEAD 与 main 已大分离叉(merge-base 在 1d90c1e,中间分叉很多 commit),cherry-pick 风险评估超出本次范围

- [决策]: 把分析写到 notes__clean_events_review.md
  [理由]: 后续如果用户要行动,这份笔记是依据

- [决策]: 在最终回复里给出优先级排序和可执行补丁建议
  [理由]: 用户要的是 actionable 分析,不是流水账
