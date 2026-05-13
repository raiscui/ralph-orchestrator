## [2026-04-29 22:44:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 任务名称: 调查 Ralph TUI chat 窗口缺失

### 任务内容

- 排查用户运行 ralph 后 TUI chat 窗口消失的原因。
- 区分 chat pane 是渲染回归、布局挤压,还是当前配置没有进入 parallel TUI。

### 完成过程

- 阅读了当前根目录 ralph.yml,确认它是 memory-enabled preset,未启用 parallel.enabled。
- 阅读了 main.rs 的 run 分支,确认 config.parallel.enabled=false 时进入 serial loop_runner。
- 阅读了 ralph-tui app 渲染路径,确认 Chat / Gates 只在 TuiMode::Parallel 分支渲染。
- 用临时 parallel idle config 在 tmux 中启动 TUI,捕获到 Chat / Gates。
- 用 100x16、100x14、100x12 三种低高度继续捕获,均能看到 Chat / Gates。

### 验证

- cargo run --bin ralph -- run --dry-run --no-tui:
  - 显示当前根配置 hats 为 builder/confession_handler/confessor。
  - 没有进入 parallel run 的证据。
- cargo run --bin ralph -- doctor --config ralph.yml --color never:
  - Config loaded 和 Config validated 通过。
  - warnings 与 chat 缺失无关。
- tmux parallel idle capture:
  - 100x30 显示 Chat / Gates。
  - 100x16、100x14、100x12 均显示 Chat / Gates。

### 总结感悟

- 这个现象是配置路径差异,不是 TUI chat 渲染本身损坏。
- 以后遇到 “TUI 某块不见了”,先确认 run mode 是 serial 还是 parallel。
- chat 是 parallel Supervisor 的控制面,不属于 serial observation TUI。
