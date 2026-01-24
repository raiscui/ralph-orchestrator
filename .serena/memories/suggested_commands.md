# 常用命令（建议）

## 构建与测试
- `cargo build`
- `cargo test`

## 运行 CLI
- `cargo run --bin ralph -- --help`
- `cargo run --bin ralph -- run --help`
- `cargo run --bin ralph -- events --help`

## 只跑指定 crate 测试
- `cargo test -p ralph-cli`
- `cargo test -p ralph-core`

## Smoke tests（回放式，快速、确定性）
- `cargo test -p ralph-core smoke_runner`
- `cargo test -p ralph-core kiro`

## E2E（真实后端验证，发布前/大改后）
- `cargo run -p ralph-e2e -- --list`
- `cargo run -p ralph-e2e -- claude`
- `cargo run -p ralph-e2e -- all`

## Git hooks（提交前自动 fmt/clippy）
- `./scripts/setup-hooks.sh`