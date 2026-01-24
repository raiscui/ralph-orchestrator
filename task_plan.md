# 任务计划: 修复 `ralph events` 在 payload 含非 ASCII 时可能 panic

> 创建时间: 2026-01-23 16:02 (CST)

## 目标
`ralph events` 在输出表格(`--format table`)时，即使事件 payload/hat/topic 含有多字节 UTF-8 字符(中文/emoji)，也不应 panic。

## 阶段
- [x] 阶段1: 计划和设置
- [x] 阶段2: 研究/收集信息
- [x] 阶段3: 执行/构建
- [x] 阶段4: 审查和交付

## 方案方向(至少两个)
### 方案A: 不惜代价，最佳方案
- 修复 `ralph-cli` 中所有可能发生 UTF-8 非字符边界切片的点(不仅仅是 `truncate`)。
- 追加 Unicode 回归测试，覆盖中文/emoji 在截断边界附近的场景。
- 顺手把 `ralph-e2e` 里同类的 `truncate` 也做成“永不 panic”，避免将来测试用例带 Unicode 时炸。

### 方案B: 先能用，后面再优雅
- 仅修复 `crates/ralph-cli/src/display.rs` 的 `truncate` 和 `payload_preview` 逻辑，确保 `ralph events` 不 panic。
- 只补最小必要测试。

## 做出的决定
- [决定] 先落地“方案B”的最小修复，但实现方式会尽量可复用；如果改动很小，我会顺手把 `ralph-e2e` 的同类截断也一起修掉(接近方案A的收益，但不做大范围重构)。

## 关键问题
1. panic 的根因是否是对 `&str` 使用按字节索引切片(`&s[..N]`)导致的 “not a char boundary”？
2. 修复后是否会影响表格对齐/展示效果？(允许轻微变化，但不能再 panic)
3. 是否已有类似的 UTF-8 截断修复模式可复用？(例如项目里其他 `truncate` 实现)

## 遇到的错误
- 暂无

## 状态
**目前状态：已完成**  
- 修复已完成：`ralph events` 对中文/emoji 等多字节 UTF-8 payload 不再因为截断而 panic。
- 验证已完成：`cargo test -p ralph-cli` 与 `cargo test` 均通过。

---

# 追加任务计划: 修复 `ralph-e2e` 里同类 UTF-8 截断 panic 风险

> 追加时间: 2026-01-23 16:10 (CST)

## 追加目标
`crates/ralph-e2e` 的多个 `truncate()` 目前也用 `&s[..N]` 按字节切片；当输出包含中文/emoji 时同样可能 panic。  
这会让 e2e 报告在“打印错误信息”时崩溃，影响排障效率。

## 追加阶段
- [x] 阶段5: 扫描并修复 e2e 的 truncate
- [x] 阶段6: 为 e2e 补回归测试
- [x] 阶段7: 重新验证并交付

## 追加状态
**目前状态：追加任务已完成**  
- 我已修复 `crates/ralph-e2e/src/scenarios/*` 里所有 `truncate()` 的 UTF-8 边界问题，并补了回归测试。
- 验证已完成：`cargo test -p ralph-e2e`、`cargo fmt --check`、`cargo clippy -p ralph-e2e` 均通过。

## 追加说明（进一步稳健性）
- 我用 ast-grep 扫描到 `crates/ralph-cli/src/display.rs` 在解析 `ts` 的时间字段时仍存在 `&time_str[..N]` 这种按字节切片。
- 虽然正常情况下 `ts` 是 ISO 8601 ASCII 字符串，但 agent 写入的事件可能带“异常 ts 文本”。
- 我已把这里也改成 UTF-8 安全切片，并补了回归测试，确保异常 `ts` 也不会让 `ralph events` 崩溃。
- 验证已完成：`cargo test -p ralph-cli`、`cargo fmt --check`、`cargo clippy -p ralph-cli` 均通过。
