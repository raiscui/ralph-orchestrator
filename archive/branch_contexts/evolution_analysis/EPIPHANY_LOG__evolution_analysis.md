## [2026-05-28 14:26:47] [Session ID: codex-20260528-135644] 主题: TUI mdfried OpenSpec tasks 不能直接当成当前实现事实

### 发现来源
- 项目演进分析中核对 `openspec/changes/tui-mdfried-viewer/tasks.md` 与 `crates/ralph-tui` 当前实现。

### 核心问题
- OpenSpec tasks 标记 `ratatui-image`、`OutputBlock::{Text, Image}` 和 Big Headers 已完成。
- 但当前 `ralph-tui` 依赖列表没有 `ratatui-image`/`image`/`cosmic-text`,并且 parallel output buffer 注释明确说不引入 Big Headers / 图片块。

### 为什么重要
- 如果后续直接从 tasks 的 5.1/5.2 继续实现图片 inline,会建立在错误的完成状态上。
- 这类 spec-code drift 会让验证报告看起来很完整,但实际功能入口并不存在。

### 未来风险
- TUI UX 线可能出现"OpenSpec 已勾选,代码已回退或从未落地"的断层。
- 后续 agent 继续该 change 时,容易把当前实现误判为已有富块渲染基础,导致补丁叠错层。

### 当前结论
- `tui-mdfried-viewer` 的下一步应先做 spec-code reconciliation。
- 只有确认当前真实实现后,才能决定恢复实现、修正 tasks 状态,或拆出 correction change。

### 后续讨论入口
- 先看 `openspec/changes/tui-mdfried-viewer/tasks.md`。
- 再看 `crates/ralph-tui/Cargo.toml` 和 `crates/ralph-tui/src/state/parallel/output.rs`。
