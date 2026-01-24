# 项目概览：ralph-orchestrator

## 项目目的
- Ralph Orchestrator 是一个“hat(角色)驱动”的编排框架，让 Ralph 以循环迭代的方式把任务做完。
- 支持传统 loop 模式与 hat-based(多角色/事件驱动)模式。

## 技术栈
- Rust workspace（edition = 2024，workspace version 当前为 2.2.1）
- CLI：clap
- TUI：ratatui + crossterm
- 序列化：serde / serde_json / serde_yaml
- 异步：tokio + futures
- HTTP：reqwest（用于 remote presets 等）

## 代码结构（workspace crates）
- crates/ralph-cli：`ralph` 命令行入口与子命令实现（run/events/init/clean/emit/plan 等）
- crates/ralph-core：核心 orchestration loop、事件日志、workspace 管理、session 录制/回放、smoke tests fixtures
- crates/ralph-adapters：对接多种 AI CLI backends（Claude/Codex/Gemini/Copilot/OpenCode 等），包含 PTY/stream 处理
- crates/ralph-tui：终端 UI 展示
- crates/ralph-proto：内部协议/类型
- crates/ralph-e2e：端到端场景验证工具（真实后端验证）
- crates/ralph-bench：基准/性能相关

## 关键理念
- orchestrator 尽量薄：让 agent 干活；用 backpressure（测试/构建/校验）兜底。
- Fresh context：每轮迭代清空上下文，重读 spec/plan/code。