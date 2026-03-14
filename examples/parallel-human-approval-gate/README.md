# 并行人工批准闸口

这是一个"自动化准备先并行完成,最后再等人工批准"的可运行范例。
它很适合展示真实上线前常见的人工闸口。

## 它在做什么

- `deploy_checker`、`rollback_checker`、`comms_checker` 先并行完成准备检查
- `ralph#1` 收到 3 条检查结果后,发布 `approval.requested`
- 运行不会立刻结束
- 只有当外部有人发来 `approval.granted` 后,`ralph#1` 才会派发 `deployment.finalize`
- `deployment_finalizer` 再产出 `deployment.ready`

## 运行

先在一个终端里启动:

```bash
cd examples/parallel-human-approval-gate
cargo run --bin ralph -- run --no-tui
```

看到 `approval.requested` 后,在另一个终端执行:

```bash
cd examples/parallel-human-approval-gate
cargo run --bin ralph -- emit approval.granted \
  --json '{"approved_by":"release-manager","window":"2026-03-10 10:00 UTC"}' \
  --target-instance ralph#1
```

## 你应该看到什么

一次正常闭环至少包含:

- `deployment.plan.check`
- `rollback.plan.check`
- `comms.plan.check`
- `deployment.checked`
- `rollback.checked`
- `comms.checked`
- `approval.requested`
- `approval.granted`
- `deployment.finalize`
- `deployment.ready`

最后由 `ralph#1` 输出 `LOOP_COMPLETE`。

## 适合用来演示什么

- "自动化能先做,但最终动作必须等人点头"
- 外部 `ralph emit` 如何和并行流程配合
- 协调者如何在等待审批时让流程保持运行
