# 笔记：mdfried 风格渲染差异与技术要点

## 现象（你截图里看到的差别）
- 你看到的内容大量来自 `stderr`（每行都有 `"[stderr]"` 前缀）。
- Ralph 并行 TUI 对 `stderr` 默认走 Plain 模式，不做 Markdown 渲染，并且还会把行整体弱化成灰色。
- 所以看起来像“纯文本”，标题/列表/引用不会被渲染成结构化样式。

## 关键本质：mdfried ≠ mdfrier
- `mdfried` 是完整的 Markdown viewer：
  - 大标题（Big Headers™）
  - 图片内联（多图形协议）
- `mdfrier` 是一个“解析 + 语义换行 + 输出 span/style 信息”的库：
  - 它本身不负责把标题变大、也不负责图片协议。
- 结论：要达到 `mdfried` 的视觉效果，必须引入 **图片/图形协议渲染层**（例如 `ratatui-image`）。

## 技术组件候选（方案 A）
- `ratatui-image`
  - 负责探测终端协议（Kitty / iTerm2 / Sixel）与字体像素尺寸
  - 提供 Image/StatefulImage widgets
  - fallback：halfblocks /（可选）Chafa
- Header 放大策略（可能二选一或混合）：
  - Kitty text sizing protocol（若可用）：直接缩放文字
  - 否则：把标题渲染成图片（需要字体栅格化）并用图形协议绘制

## 设计警告（对 Ralph 的影响）
- 这会把“输出视图”从纯文本渲染，升级成“文本 + 图片块”的混合渲染。
- 会影响：
  - 软换行/滚动（图片块高度不是 1 行）
  - 框选复制（图片不可复制，需要定义复制语义）
  - 性能（StatefulImage 的 resize/encode 可能阻塞，需要后台线程/缓存）

## 2026-01-30 03:40 +0800 进展总结（本次实现后的结论）
- 现在并行 Supervisor 的 Output 面板已经支持“Text + Image”的统一滚动模型。
- `stderr` 不再被强制当作 Plain，也不再把流标识拼到正文里。
  - 流标识改为 UI 前缀列渲染，因此 Markdown 行首语义（`#`/`>`/`-`）能正常工作。
- Big Headers 的实现路径与 `mdfried` 一致：
  - 用 `cosmic-text` 把 H1/H2/H3 栅格化为 RGBA，再用 `ratatui-image` 编码成协议图像。
  - 同宽度/同文本会命中缓存，避免重复 encode。
- 还没做的部分：`![]()` 图片内联（已留好开关与数据结构，等下一轮实现）。

## 2026-01-30 12:47 +0800｜决策：取消 mdfried/mdfrier，回退 termimad

- 你决定：不再使用 `mdfrier`（参考 `mdfried`）来渲染 Markdown。
- 我做的回退（最小回退，优先恢复原本渲染器）：
  - stdout（`PrettyStreamHandler`）：`termimad` 直接把 Markdown 渲染成 ANSI 字符串并写入 stdout。
  - TUI（`render_text_to_lines`）：`termimad` 先输出 ANSI，再用 `ansi-to-tui` 解析回 `ratatui::Line`。
- 依赖变化：
  - workspace：移除 `mdfrier`，新增 `termimad = 0.34.1`（其内部依赖 `minimad`）。
- 影响提醒：
  - 既然不再依赖 `mdfrier`，仓库是否还需要保持 `GPL-3.0-or-later` 可以另开任务再决定。

## 2026-01-30 13:34 +0800｜执行：彻底回退 Big Headers/图片渲染 + 许可证回退 + 移除左侧红色 E

- 你选择了“彻底回退”（方案 A）：
  - 移除了 Big Headers / 图片块等 `mdfried` 相关渲染特性
  - 并行 Output 面板不再渲染左侧红色 `E` 前缀列（stderr 仅用灰色弱化区分）
- 许可证结论：
  - 已把仓库许可证从 `GPL-3.0-or-later` 回退到 `MIT`
