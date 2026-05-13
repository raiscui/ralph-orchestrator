## [2026-03-21 12:41:58] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: examples 全量真后端回归与中断清理后续动作

### 延后事项
- 在无人值守窗口完整执行一次:
  - `cargo run -p ralph-e2e -- codex --filter example --report both --skip-analysis`
  - 目标是拿到 fresh 的 `report.json` / `report.md`,而不是只看 `report-live.md`
- 单独做一次中断路径验证:
  - 复现 `Ctrl-C` 后 `ralph-e2e` 退出但 `ralph run` / `codex app-server` 仍残留的现象
  - 再判断是否需要修 harness 的子进程回收逻辑

### 为什么先记下来
- 本轮交互里已经拿到了:
  - 1 条完整通过的 example 证据
  - 1 条复杂 example 的动态探针证据
- 但还没有拿到:
  - 全量 26 条 examples 的最终 fresh report
  - 中断清理问题的根因与正式修复

## [2026-03-21 18:38:00] [Session ID: 68546] 主题: 单独跟踪 `parallel-experimental-dev-engine-example` 旧 `job 5` 尾巴

### 延后事项
- 单独做一次针对旧失败口径的复现与证据保留:
  - 目标现象:
    - `No new jobs after LOOP_COMPLETE (example)` 失败
    - `completion_seen=true, new_jobs_after=[("ralph#1", 5)]`
- 如果后续继续追,优先先比对两套 completion 口径:
  - Supervisor 内部 completion 判定
  - scenario 对 stdout 中 `LOOP_COMPLETE` 的判定
- 如需增强证据,考虑把 stdout artifact 从“偏前段截断”改成“保留尾段或双端保留”,方便直接看 `LOOP_COMPLETE` 后面到底还输出了什么

### 为什么先记下来
- fresh 真录制已经证明“无回流”主问题当前已解除。
- 但历史上那次 `job 5` 尾巴失败仍然是真实记录过的现象。
- 这类尾巴如果后面再出现,应该按独立 flaky 方向处理,不要再回滚成“回流链没修好”的旧结论。

## [2026-03-21 22:00:49] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 如果要彻底消灭旧 `job 5` 尾巴,下一步优先考虑“completion 冻结 pending”

### 延后事项
- 评估是否在 runtime 引入更明确的 completion 收敛语义:
  - completion 之后只允许 drain 当前正在 Running 的 job
  - 不再允许已存在于 instance `pending` 里的新 job 继续起跑
- 如果要做,优先考虑的设计方向:
  - 增加一个“freeze / stop_after_current”类命令给各 instance
  - 或在 Supervisor 看到 completion 后显式清空/冻结 `ralph#1.pending`
- 同时保留现在新增的 characterization test,把它从“证明机制存在”升级成“证明修复生效”

### 为什么先记下来
- 这轮已经拿到动态证据:
  - 当前 runtime 的确允许 prequeued `ralph` job 在 completion 后起跑
- 但我们还没有正式决定:
  - 这是不是必须收紧的产品语义
  - 还是只把它当成旧 flaky 的调查结论保留

## [2026-03-24 12:39:51] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 继续追旧 `job 5` 前,先解决 example 当前停在 `exp-001 reviewed` 的前置卡点

### 延后事项
- 单独追一轮 `parallel-experimental-dev-engine-example` 的“只 durable 派发了 exp-001”现象:
  - `ralph#1` 的计划里写的是首批应派发 `exp-001` + `exp-002`
  - 但主 `.ralph/events.jsonl` 只看到 `exp-001`
- 重点区分两种可能:
  - `exp-002` 从未真正输出
  - `exp-002` 曾在模型输出中出现,但没有进入 durable 事件流
- 把这个前置卡点解决后,再重新跑真后端 example,届时才有资格继续验证:
  - completion 后是否还会出现旧 `job 5` 尾巴

### 为什么先记下来
- 本轮真实 run 没有复现旧 `job 5`
- 但也没有到达 completion,所以不能把“未复现”误说成“已验证修好”
- 当前更前面的阻塞点已经足够明确,应该先把注意力收回到这里

## [2026-03-25 21:09:31] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: 评估是否需要给 example 类 E2E 再加一层 `config/all_hat.md` 降噪

### 延后事项
- 当前 fix 已通过覆盖 workspace 根 `AGENTS.md` 把 `parallel-experimental-dev-engine-example` 拉回 PASS。
- 但从最新 stdout 仍可见:
  - 编译期内嵌的 `config/all_hat.md` 还会继续注入实例级 `ralph/log/<hat>/...` 文件上下文规则
- 后续可评估:
  - 是否为 example / E2E 场景提供一个更轻的 all-hat overlay
  - 或在 runtime 加一个显式开关,允许 example 关闭这些开发型提示词

### 为什么先记下来
- 当前 PASS 已足够支持本轮收尾,不需要立刻继续改更多层。
- 但如果未来还想把 example 真后端耗时再压低,或者进一步减少 worker 长尾,这会是最值得继续挖的一层。

## [2026-03-31 02:34:16] [Session ID: 019d003c-4cdd-7ee2-95c5-d061873df462] 主题: `config/all_hat.md` 降噪延期项已落地,本条关闭

### 已完成事项
- 已为 runtime 增加 `core.all_hat_prompt` 覆写能力:
  - `compiled`
  - `disabled`
  - `inline`
  - `file`
- `parallel-experimental-dev-engine-example` 已在 E2E patched `ralph.yml` 中使用轻量 `inline` overlay
- 真后端验证已通过:
  - `.e2e-tests/report-live.md` -> `Passed: 1 | Failed: 0`
  - `.e2e-tests/report.json` -> `passed: true`

### 关闭说明
- 2026-03-25 21:09:31 那条“评估是否需要给 example 类 E2E 再加一层 `config/all_hat.md` 降噪”已不再是待办。
- 如果后续继续做,主题会升级为:
  - 如何把这套 runtime override 能力并入 workflow / preset 首次释放与选择机制
  - 而不是继续讨论“要不要给 example 加轻量 overlay”
