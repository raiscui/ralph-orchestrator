## [2026-08-12 20:53:00] [Session ID: omx-1786419140441-df5ql8] 笔记: Group 1 cherry-pick dry-run 全部冲突,根本性矛盾

### 现象

openspec change `sync-origin-main-features-q3-2026` 的 Group 1 列了 6 个「零风险 cherry-pick」。
实际 dry-run 验证:

| # | Commit | dry-run 结果 | 根因分类 |
|---|--------|-------------|---------|
| 1.1 e88b7e3 | ✅ 已 manual port (HEAD 4624750) | 已验证 6/6 测试 + clap conflict |
| 1.2 db48462 | ❌ 大量冲突 + modify/delete | 本地 main 主动删了 presets/*.yml 和 acp_executor 等 |
| 1.3 192f5f9 | ❌ modify/delete | `crates/ralph-adapters/src/acp_executor.rs` 已删 |
| 1.4 01dd250 | ❌ modify/delete | `crates/ralph-api/src/mcp.rs` 已删 |
| 1.5 86cfb1a | ❌ 内容冲突 | `Cargo.toml` + `main.rs` 本地大量改 |
| 1.6 6aacc6b | ⚠ modify/delete,但大部分新文件可 apply | `.claude-plugin/marketplace.json` + `skills/README.md` 已删 |

### 静态证据

- `presets/autoresearch.yml` (本地 HEAD: 不存在)
- `presets/minimal/roo.yml` (本地 HEAD: 不存在)
- `crates/ralph-cli/presets/autoresearch.yml` (本地 HEAD: 不存在)
- `crates/ralph-cli/presets/merge-loop.yml` (本地 HEAD: 不存在)
- `crates/ralph-core/data/ralph-tools-tasks.md` (本地 HEAD: 不存在)
- `docs/examples/pdd-design.md` (本地 HEAD: 不存在)
- `docs/guide/roo-backend.md` (本地 HEAD: 不存在)
- `.claude-plugin/marketplace.json` (本地 HEAD: 不存在)
- `skills/README.md` (本地 HEAD: 不存在)

本地 main 在某个早期 commit(早于 b3bbe91e,因 b3bbe91e..HEAD 的文件集与 1.2 文件集 0 overlap)主动做了删除。

### 动态证据

每个 cherry-pick 命令的实际输出确认 static 结论:
- `git cherry-pick --no-commit <sha>` 自动失败
- abort 命令无法执行(`error: no cherry-pick or revert in progress`)
- `git reset --hard HEAD` 成功清理 working tree

### 候选根本原因

#### H1: proposal 的「零风险」假设不准确 (高置信度)

- proposal 第 1 节说「这些 commits have no overlap with local main's architectural refactor (3ff4b47 EventLoop 收窄 + PromptExecutor port) or declarative e2e rewrite」
- 实际本地 main 走的删除+重构比 proposal 假设的更深
- 本地 main 的架构调整不止 EventLoop 收窄,还包括大范围删除上游文件、用自己的实现替换
- 这是为什么 Group 1 全部冲突的根本原因

#### H2: 单纯的时间错位 (次候选)

- 本地 main 在 merge-base 之后立刻做了删除,与 upstream 后续 commit 互斥
- 如果当初 rebase 而不是分叉,本不会有冲突
- 但实测看本地 main 主动重写了生态,不只是时序问题

### 结论

H1 高置信度。Group 1 假设(G-1 「零风险」)与本地 main 实际状态有根本性矛盾。

### 后续候选

#### 候选 A: 关闭这个 change,改成 Group 4 (rewrite, not cherry-pick)
- 把所有 6 个 Group 1 项全部划入 Group 4 (rewrite, out of scope)
- 任务只剩 Group 5 (local patches)
- 完成 P5 (`.ralph/specs/` ↔ `specs/` reconcile) + 部分 cherry-pick 1.5 的思路重新设计

#### 候选 B: 单独处理 1.6,其他全部跳过
- 1.6 的核心是新增 `skills/ralph-docs/*` subtree,新文件可以独立 copy
- 但需要本地重建 `.claude-plugin/marketplace.json` 和 `skills/README.md`
- 1.6 部分成功,1.2-1.5 跳过

#### 候选 C: 全部放弃 Group 1, 1.1 manual port 模式延续
- 后续每个 cherry-pick 都做 manual port
- 时间成本:每个 30 分钟-2 小时(取决于改动范围)
- 保留 control,但慢

#### 候选 D: 回归源头,先把 local main 重构成「可 cherry-pick」
- 但是这是反向,成本极高,不推荐

### 推荐

候选 A + 候选 B 组合: 1.5/1.6 走 partial/manual,1.2-1.4 划到 Group 4 (rewrite)。
但是已经超出「继续」的 scope,需要用户决策。
