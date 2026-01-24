# 代码风格与约定（项目级）

## Rust
- Workspace：`edition = "2024"`
- Lints：`unsafe_code = "forbid"`
- Clippy：启用 `clippy::pedantic`（部分常见噪音规则在 workspace 里被 allow）
- 命名：遵循 Rust 标准命名规范（类型/trait 驼峰，函数/变量 snake_case 等）

## 变更流程（高层）
- 新功能：先写 `specs/*.spec.md`，再实现。
- Bug fix：定位根因 → 修复 → 加回归测试 → `cargo test` 验证。

## 目录约定
- Specs：`specs/`
- Tasks：`tasks/`（`.code-task.md`）
- Memories：`.agent/memories.md`
- Tasks tracking：`.agent/tasks.jsonl`