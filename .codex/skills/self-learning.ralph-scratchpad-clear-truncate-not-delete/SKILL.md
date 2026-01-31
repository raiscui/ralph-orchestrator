---
name: self-learning.ralph-scratchpad-clear-truncate-not-delete
description: |
  修复 Ralph 在 fresh run 清理 scratchpad 时“误删文件”导致 `ralph run --continue` 直接失败的问题。
  触发场景：(1) fresh run 清理用 `remove_file`；(2) backend 是 mock/`command: "true"` 等不会生成新 scratchpad；(3) 随后执行 `ralph run --continue` 报错 "scratchpad not found"。
  解决：fresh run 清理应“truncate 为空”（保留文件存在性），必要时创建父目录；resume/continue 才能稳定工作，测试也更确定。
author: Codex CLI
version: 1.0.0
date: 2026-01-31
---

# Ralph scratchpad 清理：truncate，不要 delete

## 问题

在 Ralph 里，“fresh run 清理旧 scratchpad”是合理需求：

- 目标：避免历史残留误导本轮 objective。

但如果实现方式是直接 `remove_file(scratchpad)`，会踩一个很隐蔽的坑：

- `ralph run --continue` 的语义是“基于已存在 scratchpad 继续”；
- 在某些测试/后端场景里（例如 `command: "true"`），fresh run 不会重新生成 scratchpad；
- 于是你刚跑完一次 `ralph run`，马上 `ralph run --continue` 就会报错退出：scratchpad 不存在。

## 上下文 / 触发条件

常见触发方式：

1) 你实现了 fresh run 清理逻辑：
- `if scratchpad.exists() { remove_file(scratchpad) }`

2) 测试或最小配置使用的是“不会产生日志/状态”的 backend：
- 例如 `cli.backend: custom` + `command: "true"`（只退出，不写 scratchpad）

3) 然后跑集成测试或手动跑：
- `ralph run ...`
- `ralph run --continue ...`
- 看到报错：`Cannot continue: scratchpad not found ...`

## 解决方案

1) fresh run 清理策略改为 truncate（清空内容）
- 不删除文件，只把内容写成空字符串（或用 set_len(0)）。
- 这样能同时满足：
  - “fresh run 不带旧状态”（内容清空）
  - “continue 的文件存在性要求”（文件仍存在）

2)（建议）确保父目录存在
- scratchpad 通常在 `.agent/` 下：
  - truncate 前先 `create_dir_all(parent)`，避免路径不存在导致清理失败。

3) continue/resume 的前置检查仍然保留
- continue 的语义就是“必须有 scratchpad”，所以检查存在性是合理的。
- 真正需要修的是：fresh run 不要把它删掉。

## 验证

- 跑一遍最小复现：
  - 第一次 `ralph run` 结束后，scratchpad 仍存在但内容为空。
  - 紧接着 `ralph run --continue` 不应因为“找不到 scratchpad”而直接退出。

- 跑全量背压验证：
  - `cargo test`

## 备注

- 这个坑尤其容易在“mock backend / 快速退出 backend”里出现，
  因为它们不会像真实 agent 那样写入 scratchpad/任务状态。
