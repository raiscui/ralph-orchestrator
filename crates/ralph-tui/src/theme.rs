//! TUI 视觉主题：Catppuccin（Mocha）调色板 + exabind 风格框体。
//!
//! 设计目标：
//! - 把颜色/边框等“风格选择”从各个 widget 中抽离，避免硬编码与漂移
//! - 提供语义化的 style roles：bg/text/muted/accent/border/selection/search

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    symbols::border::Set,
    widgets::Block,
};

// =============================================================================
// Catppuccin (Mocha) Palette
// =============================================================================
//
// 说明：
// - 这里直接内置 Mocha 的 RGB 值，避免引入额外主题依赖
// - 取值参考：Catppuccin 官方调色板（Mocha）
//
// 约定：该结构仅作为颜色 token，不直接表达 UI 语义。

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct CatppuccinMocha {
    pub rosewater: Color,
    pub flamingo: Color,
    pub pink: Color,
    pub mauve: Color,
    pub red: Color,
    pub maroon: Color,
    pub peach: Color,
    pub yellow: Color,
    pub green: Color,
    pub teal: Color,
    pub sky: Color,
    pub sapphire: Color,
    pub blue: Color,
    pub lavender: Color,
    pub text: Color,
    pub subtext1: Color,
    pub subtext0: Color,
    pub overlay2: Color,
    pub overlay1: Color,
    pub overlay0: Color,
    pub surface2: Color,
    pub surface1: Color,
    pub surface0: Color,
    pub base: Color,
    pub mantle: Color,
    pub crust: Color,
}

impl CatppuccinMocha {
    pub const fn new() -> Self {
        Self {
            rosewater: Color::from_u32(0xF5_E0_DC),
            flamingo: Color::from_u32(0xF2_CD_CD),
            pink: Color::from_u32(0xF5_C2_E7),
            mauve: Color::from_u32(0xCB_A6_F7),
            red: Color::from_u32(0xF3_8B_A8),
            maroon: Color::from_u32(0xEB_A0_AC),
            peach: Color::from_u32(0xFA_B3_87),
            yellow: Color::from_u32(0xF9_E2_AF),
            green: Color::from_u32(0xA6_E3_A1),
            teal: Color::from_u32(0x94_E2_D5),
            sky: Color::from_u32(0x89_DC_EB),
            sapphire: Color::from_u32(0x74_C7_EC),
            blue: Color::from_u32(0x89_B4_FA),
            lavender: Color::from_u32(0xB4_BE_FE),
            text: Color::from_u32(0xCD_D6_F4),
            subtext1: Color::from_u32(0xBA_C2_DE),
            subtext0: Color::from_u32(0xA6_AD_C8),
            overlay2: Color::from_u32(0x93_99_B2),
            overlay1: Color::from_u32(0x7F_84_9C),
            overlay0: Color::from_u32(0x6C_70_86),
            surface2: Color::from_u32(0x58_5B_70),
            surface1: Color::from_u32(0x45_47_5A),
            surface0: Color::from_u32(0x31_32_44),
            base: Color::from_u32(0x1E_1E_2E),
            mantle: Color::from_u32(0x18_18_25),
            crust: Color::from_u32(0x11_11_1B),
        }
    }
}

impl Default for CatppuccinMocha {
    fn default() -> Self {
        Self::new()
    }
}

pub const CATPPUCCIN_MOCHA: CatppuccinMocha = CatppuccinMocha::new();

/// 兼容常量：弱化前景色（用于历史代码里 `Style::fg(MUTED_FG)` 的调用点）。
///
/// 说明：
/// - 新主题体系里更推荐用 `theme.muted()`（Style role）。
/// - 但 state 层/少量 widget 仍会用到一个纯 Color 常量做快速弱化。
pub const MUTED_FG: Color = CATPPUCCIN_MOCHA.overlay0;

// =============================================================================
// TuiTheme (semantic roles)
// =============================================================================

/// TUI 主题（语义化 style roles）。
///
/// 注意：
/// - 这里“角色”是为了表达 UI 语义，而不是绑定某个具体组件
/// - 目前只实现默认主题（Catppuccin Mocha），后续如果要支持切换，可在这里扩展
#[derive(Debug, Clone, Copy)]
pub struct TuiTheme {
    colors: CatppuccinMocha,
    /// 背景模式：
    /// - `false`：使用主题的显式背景色（crust/base），风格一致、层次更明确
    /// - `true`：应用背景使用终端默认背景（app bg=Reset），用于 Warp 等终端保留“半透明窗口背景”效果
    ///
    /// 说明：
    /// - 终端不支持 alpha，所谓“半透明”通常是终端窗口级的透明/blur 叠加
    /// - 如果我们在整个 frame 上画了显式 bg，会把这种效果“盖掉”，导致 UI 范围外的 padding 看起来发灰
    /// - 但用户允许 panel 内部保留底色（base）：既提升可读性，也能避免动画遮罩出现刺眼的纯白条
    use_terminal_default_bg: bool,
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self::catppuccin_mocha()
    }
}

impl TuiTheme {
    pub const fn catppuccin_mocha() -> Self {
        Self {
            colors: CATPPUCCIN_MOCHA,
            use_terminal_default_bg: false,
        }
    }

    pub const fn colors(&self) -> &CatppuccinMocha {
        &self.colors
    }

    /// 启用“使用终端默认背景”的模式（bg=Reset）。
    ///
    /// 典型用途：Warp 的窗口半透明/blur 效果只对默认背景生效时，
    /// 该模式可以让内容区与 padding 共享同一套半透明背景，避免外圈发灰。
    pub fn with_terminal_default_bg(mut self) -> Self {
        self.use_terminal_default_bg = true;
        self
    }

    pub const fn app_bg_color(&self) -> Color {
        if self.use_terminal_default_bg {
            Color::Reset
        } else {
            self.colors.crust
        }
    }

    pub const fn panel_bg_color(&self) -> Color {
        // 设计取舍：
        // - Warp 透明模式下，我们只把“全局 app 背景”交给终端（bg=Reset），以保持窗口半透明统一；
        // - 但 panel 内部仍使用主题底色（base），这样：
        //   1) 内容可读性更稳定（不会受终端背景纹理/模糊影响）
        //   2) 动画（尤其是 Output 重启的 sweep）不会出现“纯白条”过亮的观感
        self.colors.base
    }

    // -------------------------------------------------------------------------
    // Base roles
    // -------------------------------------------------------------------------

    /// 应用级背景（通常用于清屏填充整个 frame）。
    pub fn app_bg(&self) -> Style {
        Style::default().bg(self.app_bg_color())
    }

    /// 面板内部背景色（比 app_bg 略亮，增强分层）。
    pub fn panel_bg(&self) -> Style {
        Style::default().bg(self.panel_bg_color())
    }

    /// 默认正文样式。
    pub fn text(&self) -> Style {
        Style::default().fg(self.colors.text)
    }

    /// 弱化文本（例如提示、时间戳、空态）。
    pub fn muted(&self) -> Style {
        Style::default().fg(self.colors.overlay0)
    }

    /// 强调色（用于 focused border / chips / 关键提示）。
    pub fn accent(&self) -> Style {
        Style::default().fg(self.colors.sapphire)
    }

    // -------------------------------------------------------------------------
    // Panel chrome
    // -------------------------------------------------------------------------

    pub fn panel_border(&self, focused: bool) -> Style {
        if focused {
            self.accent()
        } else {
            Style::default().fg(self.colors.surface0)
        }
    }

    pub fn panel_title(&self, focused: bool) -> Style {
        if focused {
            Style::default()
                .fg(self.colors.text)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
                .fg(self.colors.subtext0)
                .add_modifier(Modifier::BOLD)
        }
    }

    // -------------------------------------------------------------------------
    // Highlight roles
    // -------------------------------------------------------------------------

    /// 选择区域背景色（Output 选择 / Chat 选择等）。
    pub fn selection_bg(&self) -> Color {
        self.colors.sapphire
    }

    /// 搜索命中高亮样式（保持强对比，避免“命中看不见”）。
    pub fn search_hit(&self) -> Style {
        Style::default()
            .fg(self.colors.crust)
            .bg(self.colors.yellow)
            .add_modifier(Modifier::REVERSED)
    }

    /// Hat Graph Radar：运行中 hat 的 box 前景色。
    ///
    /// 说明：
    /// - 这是一个“语义化角色”，而不是固定复用 Catppuccin 的某个 token；
    /// - 你指定了该高亮色为 `#a9dc76`（便于与其它蓝色系强调色区分）。
    pub const fn hat_graph_running_hat_fg(&self) -> Color {
        Color::from_u32(0xA9_DC_76)
    }
}

// =============================================================================
// Exabind-style Border Set
// =============================================================================
//
// 说明：
// - 该字符集来自 exabind 的 border set 风格（更锐利/现代）
// - 这里作为默认框体样式；若某些终端字体不兼容，可在上层做 fallback

pub const EXABIND_BORDER_SET: Set = Set {
    top_left: "▟",
    top_right: "▜",
    bottom_left: "▔",
    bottom_right: "▔",
    vertical_left: "▏",
    vertical_right: "▕",
    horizontal_top: "▔",
    horizontal_bottom: "▔",
};

// =============================================================================
// Panel helper
// =============================================================================

/// 构建一个“统一风格”的面板 Block。
///
/// 设计要点：
/// - 所有 panes 共享同一套 border_set + 背景色 + 标题样式
/// - focused 状态只通过参数决定，避免每个 widget 自己写一堆 if/else
pub fn panel_block(title: impl Into<String>, focused: bool, theme: &TuiTheme) -> Block<'static> {
    let title = format!(" {} ", title.into());
    Block::bordered()
        .border_set(EXABIND_BORDER_SET)
        .border_style(theme.panel_border(focused))
        .title(title)
        .title_style(theme.panel_title(focused))
        .style(theme.panel_bg())
}

// =============================================================================
// Exabind border rendering details
// =============================================================================
//
// 为什么需要“二次补丁”：
// - exabind 这套边框字符集会用到 `▟` / `▔` 这类 Unicode 块元素。
// - 这些字符在字形内部存在“空白区域”，渲染时会用 **cell 的背景色** 填充。
// - 但我们的 panel 通常是：
//   - 内部背景：`base`（略亮）
//   - 外部背景：`crust`（更暗）
// - 如果直接用 `Block::bordered().style(panel_bg)` 渲染：
//   - 左上角 `▟` 的左上空白象限会被填成 `base`，视觉上像“缺口被糊住/锯齿”；
//   - 底边 `▔` 的下方空白区域也会被填成 `base`，底边看起来不贴边。
//
// exabind 的做法是：渲染完表格后，手动把“外侧区域”的背景刷回 `crust`。

/// 修正 exabind 风格边框在“内外背景不同”时的细节渲染。
///
/// 目前按“外圈整圈”处理（都属于“边框 cell”）：
/// - 顶边整行：让 `▔` 的空白区域使用外侧背景（crust）
/// - 底边整行：让 `▔` 的空白区域使用外侧背景（crust）
/// - 左右竖边：让 `▏` / `▕` 的空白区域使用外侧背景（crust），避免看起来发灰
pub fn patch_exabind_panel_border_bg(buf: &mut Buffer, area: Rect, theme: &TuiTheme) {
    let area = area.intersection(*buf.area());
    if area.is_empty() {
        return;
    }

    // 外圈背景色：显式主题背景 or 终端默认背景（Warp 半透明模式）。
    let outside_bg = theme.app_bg_color();

    // 1) 顶边整行：让 `▔` 的空白区域用外侧背景填充。
    let top_row = area.rows().next().unwrap_or_default();
    for pos in top_row.positions() {
        if let Some(cell) = buf.cell_mut(pos) {
            let style = cell.style();
            cell.set_style(style.bg(outside_bg));
        }
    }

    // 2) 底边整行：让 `▔` 的空白区域用外侧背景填充。
    let bottom_row = area.rows().next_back().unwrap_or_default();
    for pos in bottom_row.positions() {
        if let Some(cell) = buf.cell_mut(pos) {
            let style = cell.style();
            cell.set_style(style.bg(outside_bg));
        }
    }

    // 3) 左右竖边：让 `▏` / `▕` 的空白区域用外侧背景填充。
    //
    // 说明：
    // - `▏` / `▕` 是“细竖条”，字形只占 cell 的一小部分，其余区域会用 bg 填充。
    // - 如果边框 cell 的 bg 跟 panel 内部一样是 `base`，视觉上会出现一条“偏灰”的竖边。
    // - 把左右边框列刷回 `crust` 后，会与外侧背景融合，看起来更像 exabind demo。
    let left_col = area.columns().next().unwrap_or_default();
    for pos in left_col.positions() {
        if let Some(cell) = buf.cell_mut(pos) {
            let style = cell.style();
            cell.set_style(style.bg(outside_bg));
        }
    }

    let right_col = area.columns().next_back().unwrap_or_default();
    for pos in right_col.positions() {
        if let Some(cell) = buf.cell_mut(pos) {
            let style = cell.style();
            cell.set_style(style.bg(outside_bg));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use tachyonfx::{Duration as FxDuration, Interpolation, Motion, fx};

    #[test]
    fn hat_graph_running_hat_fg_is_custom_hex() {
        let theme = TuiTheme::default();
        assert_eq!(
            theme.hat_graph_running_hat_fg(),
            Color::from_u32(0xA9_DC_76),
            "Running hat box 的高亮色应为 #a9dc76"
        );
    }

    #[test]
    fn patch_exabind_panel_border_bg_sets_border_cells_bg_to_crust() {
        let theme = TuiTheme::default();
        let area = Rect::new(0, 0, 16, 6);
        let mut buf = Buffer::empty(area);

        // 先渲染一个标准 panel：背景是 base（比外侧 crust 更亮）
        let block = panel_block("Test", false, &theme);
        block.render(area, &mut buf);

        // 预期：未打补丁前，边框 cell 背景仍是 base（会导致部分块元素边框看起来“发灰/糊住”）。
        assert_eq!(buf[(0, 0)].style().bg, Some(theme.colors().base));
        assert_eq!(buf[(0, 1)].style().bg, Some(theme.colors().base));
        assert_eq!(
            buf[(area.width - 1, 1)].style().bg,
            Some(theme.colors().base)
        );

        patch_exabind_panel_border_bg(&mut buf, area, &theme);

        // 打补丁后：顶边整行背景被刷成 crust（让 “外圈” 保持一致深色）。
        for x in 0..area.width {
            assert_eq!(buf[(x, 0)].style().bg, Some(theme.colors().crust));
        }

        // 打补丁后：底边整行背景被刷成 crust（避免 `▔` 下方空白区仍是 base）。
        let bottom_y = area.height.saturating_sub(1);
        for x in 0..area.width {
            assert_eq!(buf[(x, bottom_y)].style().bg, Some(theme.colors().crust));
        }

        // 打补丁后：左右边框列背景被刷成 crust（避免竖边看起来“发灰”）。
        for y in 0..area.height {
            assert_eq!(buf[(0, y)].style().bg, Some(theme.colors().crust));
            assert_eq!(
                buf[(area.width - 1, y)].style().bg,
                Some(theme.colors().crust)
            );
        }

        // 内部区域不应被影响（仍然是 base）。
        assert_eq!(buf[(1, 1)].style().bg, Some(theme.colors().base));
    }

    #[test]
    fn patch_exabind_panel_border_bg_restores_border_after_bg_mutating_effect() {
        // 这个测试锁定一个关键顺序约束：
        // - exabind 边框的“外侧背景”修正必须在会改 bg 的动画（tachyonfx sweep 等）之后执行；
        // - 否则动画会把边框 cell 的 `bg=Reset` 覆盖成 panel 底色，用户会感知为“最外圈被染色”。
        let theme = TuiTheme::default().with_terminal_default_bg();

        let frame = Rect::new(0, 0, 30, 10);
        let panel = Rect::new(4, 2, 22, 6);
        let mut buf = Buffer::empty(frame);

        let block = panel_block("Test", false, &theme);
        block.render(panel, &mut buf);
        patch_exabind_panel_border_bg(&mut buf, panel, &theme);

        // 模拟 Output 的 sweep 动画：它会改 fg/bg，并且在插值阶段会把 Reset 当作 Black 参与计算。
        // 我们故意让 effect 作用到整个 panel（包含边框），复现“边框 bg 被覆盖”的风险。
        let mut effect = fx::sweep_in(
            Motion::UpToDown,
            panel.height.max(1),
            0,
            theme.panel_bg_color(),
            (120, Interpolation::Linear),
        )
        .with_area(panel);

        // 关键点：用 0ms 处理一次，让 effect 落在“起步态”（progress=0）。
        // sweep_in 在起步态会把区域整体刷成 faded_color，从而覆盖我们之前刷回去的 `bg=Reset`。
        effect.process(FxDuration::from_millis(0), &mut buf, frame);

        // 在 re-patch 之前，边框 cell 的 bg 很可能已被动画改写（不再是 Reset）。
        assert_ne!(buf[(panel.x, panel.y)].bg, Color::Reset);

        // re-patch：把边框 cell 的 bg 重新刷回 app_bg（Warp 模式下就是 Reset）。
        patch_exabind_panel_border_bg(&mut buf, panel, &theme);

        // 顶边/底边/左右竖边都应恢复为 Reset。
        for x in panel.x..panel.right() {
            assert_eq!(buf[(x, panel.y)].bg, Color::Reset);
            assert_eq!(buf[(x, panel.bottom() - 1)].bg, Color::Reset);
        }
        for y in panel.y..panel.bottom() {
            assert_eq!(buf[(panel.x, y)].bg, Color::Reset);
            assert_eq!(buf[(panel.right() - 1, y)].bg, Color::Reset);
        }
    }
}
