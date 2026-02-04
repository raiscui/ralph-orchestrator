# 并行实验开发永动机（Parallel Experimental Dev Engine）

这是一份“配置方案型”的 example。
它的目标不是演示某个具体业务功能。
它的目标是提供一份**适合探索型开发任务**的 `ralph.yml`：

- 并行实现（多实例 runner 并发跑）
- 批量验证（每个实验必须给出验证证据）
- 多轮实验性开发（失败→下一轮实验建议；成功→收敛）

你可以把它当作：
“我不知道该怎么改最稳。
我需要多次试验、多次验证。
而且我希望并行跑起来”的默认起手式。

---

## 工作流（高层）

```mermaid
flowchart LR
  Ralph["ralph#1 (coordinator)"] -->|"experiment.task (windowed)"| Runner["experiment_runner#*"]
  Runner -->|"experiment.result"| Auditor["experiment_auditor"]
  Auditor -->|"experiment.reviewed"| Ralph
  Ralph -->|"integration.task"| Integrator["experiment_integrator"]
  Integrator -->|"integration.applied / rejected"| Ralph
  Ralph -->|"experiment.complete"| End((LOOP_COMPLETE))
```

核心 topic：

- `experiment.start`：入口事件（payload 是 EXPERIMENT_PLAN）
- `experiment.task`：单个实验任务（包含 what/how/verify）
- `experiment.result`：单个实验结果（包含验证证据 + patch；commit 可选）
- `experiment.reviewed`：审计结果（证据是否足够；证据不足则拒绝收敛）
- `integration.task`：集成任务（由 ralph#1 发布，驱动 integrator 在主工作区 apply patch + 最终验收）
- `integration.applied`：集成成功（含最终验收证据，推荐包含最终 commit hash）
- `integration.rejected`：不采纳/集成失败（含原因与证据；此时不得收敛）
- `integration.blocked`：集成阻塞（外部依赖/权限/环境问题；此时不得收敛）
- `experiment.complete`：收敛完成事件（由 ralph#1 发布）

---

## 如何使用

### 1) 填写你的实验计划（最重要）

打开 `examples/parallel-experimental-dev-engine/ralph.yml`。
在 `event_loop.prompt` 里找到 `EXPERIMENT_PLAN`。
把里面的内容改成你自己的任务。

这个配置方案的核心约束是：
“做什么 / 怎么做 / 怎么验证”都由你提供。
Ralph 负责把它并行化、结构化。
并且会**自适应决定并行度**（激进起步 + AIMD 动态调参）。
同时强制产出验证证据。

### 2) 运行

在仓库根目录执行：

```bash
# 只使用配置（目标 prompt 已通过 event_loop.prompt 内联）
cargo run --bin ralph -- run \
  -c examples/parallel-experimental-dev-engine/ralph.yml \
  --no-tui
```

可选：在 CLI 上覆盖 backend（如果你默认 backend 没配好，建议显式指定）：

```bash
cargo run --bin ralph -- run \
  -c examples/parallel-experimental-dev-engine/ralph.yml \
  -b codex \
  --no-tui
```

---

## 你应该看到什么（最小成功标准）

一次正常的“跑通并收敛”至少包含：

1. `ralph#1` 按“在途窗口（in-flight window）”分批发出 N 个 `experiment.task`
   - 并行度会运行中动态调参（激进 + AIMD），不会一次性洪水式派发
2. 多个 `experiment_runner#*` 并行产出 `experiment.result`
3. `experiment_auditor` 对每个 `experiment.result` 产出 `experiment.reviewed`
   - 证据充分：`evidence_ok=true`
   - 证据不足：`evidence_ok=false` + `needs_more_evidence`（此时不得收敛）
4. 只有当所有实验都拿到 `evidence_ok=true` 的 `experiment.reviewed` 后：
   - `ralph#1` 才会发布 `integration.task`（选择一个候选方案进入“主工作区集成/验收”）
5. `experiment_integrator` 必须对 `integration.task` 产出：
   - 成功：`integration.applied`
   - 失败：`integration.rejected`（此时不得收敛）
   - 阻塞：`integration.blocked`（此时不得收敛）
6. 只有当收到 `integration.applied` 后：
   - `ralph#1` 才会发布 `experiment.complete`
   - 并输出 `LOOP_COMPLETE`

---

## 安全与生产建议

这个 example 为了“一条命令跑通”，默认把：

- `parallel.permissions.worktree: allow`
- `parallel.permissions.hooks: allow`

在真实团队/生产环境里，建议至少把 `worktree` 改成 `ask`（需要显式 gate 批准）：

等价写法（便于 grep/交流）：`parallel.permissions.worktree: ask`、`parallel.permissions.hooks: allow`。

```yaml
parallel:
  permissions:
    worktree: ask
    # 约定：hooks 默认不需要批准（避免每次 on_acquire/on_release 都打断流程）
    hooks: allow
```

这样当 workflow 想做高风险操作（例如 worktree acquire）时，会走 `gate.request` / `gate.resolve` 的审批流程。
如果你希望“等待过久能自动继续/失败”，可以配合 `parallel.gate.default_timeout_secs` 使用：

- `default_timeout_secs: 0` 表示不超时（一直等）
- `default_timeout_secs: 60` 表示等 60 秒后超时（会触发 `gate.timeout` 语义）
