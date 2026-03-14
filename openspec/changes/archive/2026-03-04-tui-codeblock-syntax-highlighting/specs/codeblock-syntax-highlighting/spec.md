## ADDED Requirements

### Requirement: 支持 fenced code block 的语法高亮（TUI + stdout pretty）
当输出渲染处于 Markdown 渲染模式（非 `--plain`）时，系统 MUST 对 Markdown fenced code block（```lang ... ```）进行语法高亮，并且该行为 MUST 同时作用于：

- TUI 输出（`ratatui` 视图）
- stdout 的 pretty 输出（TTY 环境）

语法高亮首期 MUST 支持以下语言与别名：

- Rust：`rust`
- Shell：`sh`、`bash`
- JSON：`json`
- YAML：`yaml`、`yml`
- TOML：`toml`
- Python：`python`、`py`
- JavaScript / TypeScript：`js`、`javascript`、`ts`、`typescript`

#### Scenario: 渲染支持语言的 code block
- **WHEN** 输出包含一个已闭合的 fenced code block，且语言为上述支持集合之一（例如 `rust`）
- **THEN** stdout pretty 输出 MUST 包含 ANSI 样式序列以体现语法高亮（而不是纯文本单色输出）
- **THEN** TUI 输出 MUST 以带样式的 spans 渲染 code block（至少对部分 token 使用非默认样式）

---

### Requirement: 未闭合 code block 不进行语法高亮（流式稳定性）
在流式输出过程中，对于尚未闭合（尚未出现结束 ```）的 fenced code block，系统 MUST NOT 执行语法高亮；系统 MUST 以统一的 code 样式显示该 code block 的当前内容，直到 code block 闭合为止。

#### Scenario: code block 未闭合时不触发高亮
- **WHEN** 输出流中出现 opening fence（例如 ```rust），但尚未出现对应的 closing fence（```）
- **THEN** 渲染器 MUST NOT 为该 code block 产生语法高亮的多色输出
- **THEN** 渲染器 MUST 继续显示 code block 的内容，且不丢失文本、不 panic

---

### Requirement: 未知语言安全降级
当 fenced code block 的语言标签不在支持集合中时，系统 MUST 安全降级为统一 code 样式显示（不做语法高亮），并且 MUST 保持内容完整。

#### Scenario: 未知语言降级为统一 code 样式
- **WHEN** 输出包含一个已闭合的 fenced code block，但语言标签不在支持集合中（例如 ```haskell）
- **THEN** 渲染器 MUST 不进行语法高亮
- **THEN** 渲染器 MUST 仍然显示完整的 code block 内容

---

### Requirement: 已闭合 code block 的渲染结果必须冻结（避免重复高亮）
对于已经闭合的 code block，系统 MUST 冻结其渲染结果；后续流式 chunk 到来时，系统 MUST NOT 因为新增文本而重复对历史已闭合 code block 进行语法高亮或重渲染。

#### Scenario: 新增输出不影响已闭合 code block
- **WHEN** 一个 code block 已闭合并完成渲染
- **WHEN** 随后又有新的文本 chunk 追加到输出流
- **THEN** 该已闭合 code block 的渲染结果 MUST 保持不变（内容与样式稳定）

---

### Requirement: `--plain` 模式禁用 code block 语法高亮
当启用 `--plain`（或等价的“Plain 渲染模式”）时，系统 MUST 禁用 code block 的语法高亮，并且 MUST 按原始文本展示 Markdown 控制符（包括 ``` fences）。

#### Scenario: plain 模式下 fences 原样可见且无高亮
- **WHEN** 启用 `--plain` 并输出包含 fenced code block 的文本
- **THEN** 输出 MUST 原样包含 ``` fences
- **THEN** 输出 MUST 不包含因 code block 语法高亮而新增的 ANSI 样式序列
