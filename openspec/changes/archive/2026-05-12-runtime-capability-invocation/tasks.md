## 1. Capability Metadata 与 Catalog 接线

- [x] 1.1 定义 workflow capability / hat capability 的 metadata schema,至少覆盖 summary、goal、when_to_use、input_contract、output_contract、invocation_mode
- [x] 1.2 让 startup resource catalog 能暴露 capability 摘要,但不要求启动时加载完整 workflow / hat 指令
- [x] 1.3 明确 YAML 注释只做人类说明,运行时 capability 选择与调用不依赖注释

## 2. Runtime Invocation Protocol

- [x] 2.1 定义 `capability.invoke`、`capability.result`、`capability.failed` 的控制面协议与 artifact
- [x] 2.2 让 `ralph#1` 能基于 capability metadata 做 v1 规则驱动选择
- [x] 2.3 记录 capability 选择依据、输入 contract 和结果摘要,用于 replay / doctor / debug

## 3. Isolated Execution Model

- [x] 3.1 实现 v1 的 workflow capability isolated child run
- [x] 3.2 实现 v1 的 hat capability isolated micro-run
- [x] 3.3 补回归验证,确保 runtime capability invocation 不会热改当前 active topology

## 4. 路线与文档

- [x] 4.1 在文档中明确记录 v1 / v2 路线:
  - v1: 规则驱动 chooser + 单 capability 隔离调用
  - v2: 规则优先 + LLM fallback chooser + 多 capability 组合计划
- [x] 4.2 更新 capability authoring / doctor / getting-started 文档,解释 workflow capability、hat capability、structured metadata 与注释边界
