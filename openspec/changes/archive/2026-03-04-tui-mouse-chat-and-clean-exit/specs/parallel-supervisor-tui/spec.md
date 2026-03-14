## ADDED Requirements

### Requirement: 鼠标点选实例
Supervisor TUI MUST 支持通过鼠标点击实例列表项来切换当前选中实例，并立即同步到输出视图。

#### Scenario: 点击实例列表切换选中项
- **WHEN** 用户用鼠标点击 instances 列表中的某一行
- **THEN** 该行对应的 `HatInstanceId` 成为当前选中实例
- **THEN** 输出面板标题与内容切换为该实例（并沿用该实例的当前 job 选择规则）

---

### Requirement: 输出视图支持文本选择与框选
Supervisor TUI MUST 在 Output 视图提供“文本选择”能力，允许用户选择跨多行的文本，并在界面中可视化高亮该选择区域。

#### Scenario: 鼠标拖拽框选多行文本
- **WHEN** 用户在 Output 视图按下鼠标并拖拽形成选择区域
- **THEN** TUI 用高亮样式标记被选择的文本（至少覆盖拖拽起止之间的多行）

#### Scenario: 键盘 Shift+方向键扩展选择
- **WHEN** 用户在 Output 视图通过键盘进入选择并使用 Shift+方向键扩展范围
- **THEN** 选择范围随光标移动而扩大或缩小，并在 UI 中持续可见

---

### Requirement: 选择状态与滚动/搜索可组合
Supervisor TUI MUST 允许在存在选择范围时继续使用滚动与搜索功能，并保证选择状态不会导致崩溃或输出错乱。

#### Scenario: 选择后滚动仍可用
- **WHEN** 用户已经在 Output 视图选中了一段文本
- **THEN** 用户继续滚动视图时，TUI 仍正常渲染并且不会 panic
