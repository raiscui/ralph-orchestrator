## 1. 许可证切换（GPLv3）

- [x] 1.1 确认再许可可行性：检查贡献者/第三方代码/子模块，确保可以将仓库许可证切换为 `GPL-3.0-or-later`
- [x] 1.2 更新许可证与元数据：修改根目录 `LICENSE`、`Cargo.toml`（workspace.package.license）与相关文档，使其一致声明为 `GPL-3.0-or-later`
- [x] 1.3 在 README/发布说明中标注 **BREAKING**：许可证从 MIT 切换到 GPLv3 的影响范围与注意事项

## 2. 依赖接入（mdfrier）

- [x] 2.1 引入 `mdfrier` 依赖（启用 `ratatui` feature），并完成最小集成验证（可编译）
- [x] 2.2 评估并移除 `termimad`：当 Markdown 渲染路径完全迁移后，删除 `termimad` 依赖与相关代码，避免双引擎分叉

## 3. 渲染管线替换（对齐 specs）

- [x] 3.1 实现/接入 `mdfrier` 的主题：先用 `mdfrier::ratatui::DefaultTheme` 跑通，再补齐 `RalphMarkdownTheme` 对齐现有 TUI 配色
- [x] 3.2 替换 Markdown 渲染分支：在“无 ANSI 且 Rendered 模式”下使用 `mdfrier`；保持“检测到 ANSI 时跳过 Markdown 渲染并保留 ANSI 样式”的优先级策略
- [x] 3.3 宽度策略对齐：TUI 渲染使用输出面板可用宽度；non-TUI pretty 输出使用终端宽度（不可用则 fallback 到明确常量，例如 80）
- [x] 3.4 保持 `--plain` 语义：禁用 Markdown 渲染但 Markdown 控制符原样可见；同时继续做 ANSI 解析以避免 ESC 控制符外露

## 4. 回归测试与 fixtures

- [x] 4.1 新增单测：不完整/解析失败的 Markdown 输入必须安全降级（不 panic、不丢内容）
- [x] 4.2 新增单测：`--plain` 下 Markdown 控制符原样可见（满足 specs 的 `--plain` 要求）
- [x] 4.3 新增单测：包含 ANSI 的输出不做 Markdown 渲染且颜色/样式被保留（满足 ANSI 优先级要求）
- [x] 4.4 如输出文本布局变化影响 replay smoke tests，更新 fixtures/断言口径（以 OpenSpec specs 为准）

## 5. 验证与收尾

- [x] 5.1 运行 `cargo fmt --check` 与 `cargo clippy --all-targets --all-features -- -D warnings`
- [x] 5.2 运行 `cargo test`（包含 replay smoke tests），确保全量通过
