# 完成一个任务前的检查清单

- [ ] 跑核心验证：`cargo test`
- [ ] 如果改了 core orchestration/事件/hat 路由：补跑 smoke tests：`cargo test -p ralph-core smoke_runner`
- [ ] 如果改了 TUI：做一次 TUI 视觉校验（可用 tmux + freeze + tui-validate 工作流）
- [ ] 如果准备提交：确保 `cargo fmt --check` 与 `cargo clippy` 通过（项目 hook 也会做）
- [ ] 确认没有引入不必要的复杂性/重复逻辑（改良胜过新增）