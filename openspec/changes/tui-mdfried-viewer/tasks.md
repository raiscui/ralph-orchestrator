## 1. 依赖与开关（安全落地）

- [x] 1.1 引入 `ratatui-image` 依赖（先选择最小可用 features），并确保全 workspace 可编译
- [x] 1.2 增加“图片渲染开关”的配置入口（默认关闭远程图片内联），并在 docs 里说明（避免意外拉取网络资源）

## 2. 终端能力探测与渲染状态管理

- [x] 2.1 在 TUI 启动阶段做协议/字体像素尺寸探测（best-effort），失败时降级到 halfblocks
- [x] 2.2 在 `TuiState` 中保存全局的 image picker/protocol 配置，并为后续缓存预留位置

## 3. 输出视图“富块渲染”重构（Text + Image）

- [x] 3.1 引入输出块中间表示（例如 `OutputBlock::{Text, Image}`），并实现基础布局/滚动的最小闭环
- [x] 3.2 将 `ContentPane`（或替代组件）升级为“按块渲染”，避免图片区域被文本覆盖
- [x] 3.3 修复/调整框选复制语义：图片块不可复制时，明确复制策略（跳过或复制 alt 文本）

## 4. Big Headers（接近 mdfried）

- [x] 4.1 Markdown 解析阶段识别 H1/H2/H3，并生成对应的“大标题块”（非纯 Line）
- [x] 4.2 实现标题栅格化与图片渲染（参考 mdfried：cosmic-text + image），并加入缓存（同宽度不重复 encode）
- [x] 4.3 stderr 流标识与内容分离：不再把 `"[stderr]"` 拼进 Markdown 内容，且仍能在 UI 上区分 stdout/stderr

## 5. 图片内联（默认关闭，显式开启才渲染）

- [ ] 5.1 实现 `![]()` 的结构识别与文本占位（默认不下载）
- [ ] 5.2 显式开启后：实现图片下载/缓存/大小上限/超时，并在失败时稳定回退

## 6. 回归测试与验证

- [x] 6.1 新增单测：stderr 标识不影响 Markdown 行首语义（标题/列表/引用）
- [x] 6.2 新增单测：Big Headers 在“启用图片渲染”时占用多行（使用 halfblocks 协议做可测验证）
- [x] 6.3 运行 `cargo fmt --check`、`cargo clippy --all-targets --all-features -- -D warnings`、`cargo test`（含 replay smoke tests）
