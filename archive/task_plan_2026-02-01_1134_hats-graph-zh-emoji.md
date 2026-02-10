# 任务计划：修复 `ralph hats graph` 在中文/emoji hat 名称下只显示 task.start→Ralph

## 目标

- 在 `examples/parallel-trigger-routing/ralph.yml` 这类包含中文/emoji hat 名称的配置下：
  - `ralph hats graph`（unicode/ascii/compact）必须能完整显示 hats 节点与边
  - `ralph hats graph --format mermaid` 输出的 Mermaid 文本也应保持可读、可渲染
- 输出应尽量确定性（同一份配置，多次运行图形结构一致）
- 增加回归测试，锁死该问题不再回归

## 方案（至少二选一）

### 方案 A（先能用，后面再优雅｜我将先按此执行）

- 不改 `beautiful-mermaid-rs`，只改 Ralph 生成的 Mermaid：
  - 节点 ID 使用 ASCII 安全的 `hat.id`（必要时再做 sanitize）
  - 节点显示名（label）仍用 `hat.name`（中文/emoji 放在 label 里）
- 优点：改动面最小、最稳；对外部渲染器也更标准（id/label 分离）
- 缺点：Mermaid 输出相较之前会更“规范”，节点 ID 不再直接是中文名

### 方案 B（不惜代价，最佳方案）

- 直接修 `beautiful-mermaid-rs` 让它支持 Unicode 节点 ID（或更宽松的 Mermaid 标识符规则）
- 优点：即便 Mermaid 节点 ID 是中文也能渲染
- 缺点：需要改外部仓库（目前是本机 path 依赖），成本更高、验证面更大

## 阶段

- [x] 阶段1：复现与根因定位
- [x] 阶段2：确定修复策略（A/B）
- [x] 阶段3：实现修复 + 回归测试
- [x] 阶段4：验证（fmt/clippy/test/smoke）
- [x] 阶段5：四文件记录（notes/WORKLOG/ERRORFIX）

## 关键问题

1. 这次问题到底是“没有读取配置”，还是“读取到了，但 Mermaid→ASCII 渲染吞图”？（我会用 `--format mermaid` 对比确认）
2. Mermaid 的“节点 ID”与“节点展示名”是否应该分离？（我倾向：ID 用稳定 ASCII，label 用人类可读文本）
3. 是否需要把 hats graph 的输出顺序做成确定性？（HashMap 迭代顺序会影响 Mermaid 文本与布局）

## 做出的决定

- [x] 决定：先走方案 A（在 Ralph 侧修 Mermaid 生成），因为这是最小改动且能从根因解决“Unicode 节点 ID”兼容问题。

## 遇到错误

- （暂无）

## 状态

**已完成**：`ralph hats graph` 在中文/emoji hat 名称的配置下，unicode/ascii/compact 能正确显示完整拓扑，并有回归测试锁定。

## 日志

### 2026-02-01 15:28 +0800

- [复现] 现象：在 `examples/parallel-trigger-routing/ralph.yml` 下，`--format mermaid` 输出拓扑完整，但 `--format unicode/ascii` 只剩 task.start→Ralph。
- [根因] `beautiful-mermaid-rs` 在解析 Mermaid 时，对“Unicode 节点 ID”兼容性不足，导致吞边/吞节点；而我们此前用 `hat.name` 生成节点 ID（中文/emoji）。

### 2026-02-01 15:33 +0800

- [修复] Mermaid 生成改为“ID/label 分离”：
  - 节点 ID：使用 ASCII 安全的 `hat.id`（并做最小 sanitize），并统一加 `Hat_` 前缀避免冲突/歧义
  - 节点 label：保留中文/emoji（`hat.name`）
- [确定性] 输出前先对 hats 按 `hat.id` 排序，降低 HashMap 迭代顺序导致的布局波动。
- [测试] 新增回归：中文/emoji hat 名称下 unicode 图必须包含各 hat 名称。

### 2026-02-01 15:38 +0800

- [验证] 通过：
  - `cargo fmt --check`
  - `cargo clippy --all-targets --all-features -- -D warnings`
  - `cargo test`
  - `cargo test -p ralph-core smoke_runner`
  - `cargo test -p ralph-core kiro`
