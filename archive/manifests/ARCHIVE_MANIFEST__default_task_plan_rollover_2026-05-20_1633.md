# Archive Manifest: default task_plan rollover 2026-05-20 16:33

## 触发条件

- 用户选择继续第 2 项: 单独处理 parent-visible spawn dogfood worker `MaxRuntime`。
- 当前 `task_plan.md` 已 991 行,继续追加新调试任务会越过 1000 行。
- 前一轮已经完成 `$continuous-learning`,并把默认组和相关支线组总结、沉淀、归档。

## 归档对象

- `task_plan.md` -> `archive/default_history/task_plan_2026-05-20_1633_pre_dogfood_worker_maxruntime.md`

## 新入口

- 新 `task_plan.md` 从 `dogfood worker MaxRuntime` 调试任务开始。

## 验证

- 续档后应运行 `git diff --check`。
