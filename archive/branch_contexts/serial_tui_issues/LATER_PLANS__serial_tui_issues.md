## [2026-04-30 09:21:02] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 后续计划: 统一 serial/parallel 输出选择代码

- 当前已经让 serial TUI 具备和 parallel 类似的输出选择能力。
- 后续若继续清理 TUI,可以把 serial/parallel 的 copy status、Mouse Up 自动复制、Shift+方向键选择扩展抽成更小的共享 helper。
- 本次没有展开这个清理,因为目标是先修复用户可见问题,并避免把 parallel chat/gate 交互一起重构扩大风险。
