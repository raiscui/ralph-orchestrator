---
name: self-learning.ralph-e2e-mock-cli-segmented-replay
description: |
  修复/避免 ralph-e2e 的 mock-mode 在“同一 instance 多次 job/迭代”场景下回放失真：旧 mock-cli 每次调用都会回放该 instance 的全部输出，
  导致 parallel 下 ralph#1 的后续 job 输出（例如 LOOP_COMPLETE）被提前回放，工作流被中断、断言缺事件（build.done 等）。
  解决：mock-cli 引入“按调用次数分段回放”（workspace 内 `.ralph/mock-cli/*.count` 计数）；顺序模式按 `_meta.iteration` 分段，parallel 模式按
  `bus.publish.source_instance==instance` 的经验边界分段；每次 backend spawn 消费下一段。
author: Codex CLI
version: 1.0.0
date: 2026-01-31
---

# ralph-e2e mock-mode：分段回放（支持多轮调用）

## 现象

在 mock-mode（cassette replay）里，Ralph 会多次 spawn backend：

- 顺序模式：每次 spawn ≈ 一轮 iteration
- 并行模式：每次 spawn ≈ 某个 hat instance 的一次 job（例如 `ralph#1` 的 job1 / job2）

如果 mock-cli 每次都把同一 instance 的“全部输出”一次性回放：

- `ralph#1` 的后续 job（常见是输出 `LOOP_COMPLETE`）会在第一轮就被回放
- Supervisor 会提前收敛，导致 workflow 未跑完（`build.done` 等事件缺失）

## 根因

旧 mock-cli 只有：

- `instance_id` 过滤（并行分流）

但缺少：

- “同一 instance 的第 N 次调用，只回放第 N 段输出”的能力

## 解决方案（本仓库已实现）

### 1) 用 workspace 计数器表示“第几次调用”

- 状态目录：`.ralph/mock-cli/`
- 计数文件：
  - 顺序模式：`.ralph/mock-cli/default.count`
  - 并行模式：`.ralph/mock-cli/{instance_id}.count`（例如 `ralph#1.count`）

每次 mock-cli 启动：

1. 读取计数（默认 0）
2. 写回计数+1
3. 用旧值作为 `invocation_index`（0-based）

### 2) 分段规则（Segmenting Rules）

顺序模式（无 instance_id）：

- 按 `_meta.iteration` 分段：每段≈一轮 iteration 的 terminal writes
- cassette 缺少 `_meta.iteration` 时：退化为“整段作为单 segment”

并行模式（有 instance_id）：

- 按 `bus.publish.source_instance==instance` 的经验边界分段
  - 该规则利用“每个 job 通常会 publish 一个事件然后 stop”的惯例
- 若切不出 segment：退化为“该 instance 的所有 terminal writes 作为单 segment”

### 3) 回放

每次调用只回放本次 segment 的 `ux.terminal.write`，并把 timing offset 归一化到 segment 内（避免第 N 段的首次 sleep 过长）。

## 如何验证

1) 录制 cassette（示例：并行 starting_event 推测场景）

```bash
target/release/ralph run \
  -c .e2e-tests/parallel-starting-event-inference/ralph.yml \
  --record-session cassettes/e2e/parallel-starting-event-inference-codex.jsonl \
  --max-iterations 20 \
  --no-tui \
  -p @.e2e-tests/parallel-starting-event-inference/prompt.md
```

2) mock-mode 回归（应稳定通过）

```bash
cargo run -p ralph-e2e -- --mock --filter parallel-starting-event-inference --verbose
```

## 备注

- 这个分段机制的目标是“让 cassette 能覆盖多次 backend spawn 的真实运行形态”，不是复刻真实模型推理。
- 如果你录制的 cassette 不包含 `_meta.iteration` 或没有可用的 `bus.publish.source_instance` 边界，
  分段会退化为单段回放；此时应考虑：
  - 重新录制（让 session recorder 带上必要的 meta/bus 记录），或
  - 在 scenario prompt/结构上让每个 job 至少 publish 1 个事件，产生可分段信号。
