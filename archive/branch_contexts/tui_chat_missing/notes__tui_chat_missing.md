## [2026-04-29 22:44:00] [Session ID: 019dd984-e9a3-7660-8264-86f293870a2b] 笔记: TUI chat 窗口缺失调查

## 来源

### 来源1: 当前根目录 ralph.yml

- 要点:
  - 当前配置是 memory-enabled preset。
  - 文件包含 tasks.enabled 和 memories.enabled。
  - 没有 parallel.enabled=true。

### 来源2: crates/ralph-cli/src/main.rs

- 要点:
  - `config.parallel.enabled` 为 true 时进入 run_parallel_loop_impl。
  - 否则进入 loop_runner::run_loop_impl。

### 来源3: crates/ralph-tui/src/app.rs

- 要点:
  - TuiMode::Parallel 才渲染底部 Chat / Gates 面板。
  - TuiMode::Serial 只渲染普通 content pane。

### 来源4: tmux 动态捕获

- 要点:
  - 临时 parallel idle config + tmux 100x30 可以看到 Chat / Gates。
  - tmux 100x16、100x14、100x12 都可以看到 Chat / Gates。

## 综合发现

### 根因

- 这次“chat 不见了”不是渲染代码坏了。
- 当前执行路径是 serial TUI,而 chat 属于 parallel Supervisor TUI。

### 后续判断

- 如果目标是“默认根目录 ralph 也要有 chat”,需要先决定是否把根配置改成 parallel workflow。
- 如果只是想使用 chat,应运行 parallel config 或在目标 config 中启用 parallel.enabled=true。
