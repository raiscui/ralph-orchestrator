//! TUI 动画：启动打开动画（open animation）与开关策略。
//!
//! 目标：
//! - 在进入 alternate screen 的第一屏提供更“有反馈”的视觉过渡
//! - 保持可用性优先：可禁用、可在小窗口自动降级

use crate::theme::TuiTheme;
use ratatui::{
    buffer::Buffer,
    layout::{Margin, Rect},
    style::Color,
};
use std::io::IsTerminal;
use tachyonfx::default_shader_impl;
use tachyonfx::{
    CellFilter, ColorSpace, Duration, Effect, EffectTimer, FilterProcessor, Interpolation, Motion,
    Shader, fx,
};

/// Warp 透明背景模式（`bg=Reset`）下的“symbol 遮罩”渐变宽度上限。
///
/// 为什么需要上限：
/// - `fx::slide_in/out` 的 `gradient_length` 太大时，会在屏幕上形成一大片“块字符渐变带”，
///   观感会变得“糊、厚、慢”，不像之前的 sweep 白条。
/// - 我们把渐变宽度收窄到一个合理范围，让“白条扫过”更干净、更有速度感。
const SYMBOL_SWEEP_GRADIENT_MAX: u16 = 10;

fn symbol_sweep_gradient(area: Rect) -> u16 {
    // clamp：语义更清晰，也能通过 clippy 的 `manual_clamp` 检查。
    area.height.clamp(1, SYMBOL_SWEEP_GRADIENT_MAX)
}

// =============================================================================
// 边框高亮：补回“白色边框过渡”的观感（主要用于 Alacritty 等非 Warp 终端）
// =============================================================================
//
// 背景：
// - 我们为了让 exabind 的 `▟/▔` 边框更“干净”，会把边框 cell 的 bg 刷成 outside_bg（见 theme.rs）。
// - 这会让纯 `sweep_in` 的边框过渡变得不明显（缺少“亮线扫过”的反馈）。
//
// 目标：
// - 只对 pane 的 outer border（1-cell）做一条“随 sweep 下移的亮线”；
// - 起步态/结束态不改变边框颜色，避免首帧/尾帧残留“白边”。

#[derive(Clone, Debug)]
struct BorderHighlightSweep {
    highlight_fg: Color,
    /// 高亮带的背景色（可选）。
    ///
    /// 为什么需要 bg：
    /// - exabind 的 `▔` 属于“细横线”，即使 fg 很亮，线条依然会显得很细；
    /// - 给边框区域的 cell 同时刷一层更亮的 bg，能在终端里形成更“粗”的视觉带宽。
    highlight_bg: Option<Color>,
    direction: Motion,
    /// 高亮带的厚度（以行数计）。
    ///
    /// 设计取舍：
    /// - 仅 1 行时，除了顶边那一瞬间外，左右竖边每帧只亮 2 个字符，肉眼很难捕捉到“扫过”；
    /// - 适当加厚（例如 3 行）后，在左右竖边会形成更明显的“亮段”，观感更接近原 sweep 的反馈。
    band_height: u16,
    timer: EffectTimer,
    area: Option<Rect>,
    cell_filter: Option<FilterProcessor>,
    color_space: ColorSpace,
}

impl BorderHighlightSweep {
    fn new(
        direction: Motion,
        band_height: u16,
        highlight_fg: Color,
        highlight_bg: Option<Color>,
        timer: EffectTimer,
    ) -> Self {
        Self {
            highlight_fg,
            highlight_bg,
            direction,
            band_height: band_height.max(1),
            timer,
            area: None,
            cell_filter: None,
            color_space: ColorSpace::default(),
        }
    }
}

impl Shader for BorderHighlightSweep {
    default_shader_impl!(area, timer, filter, color_space, clone);

    fn name(&self) -> &'static str {
        "border_highlight_sweep"
    }

    fn execute(&mut self, _: Duration, area: Rect, buf: &mut Buffer) {
        let area = area.intersection(*buf.area());
        if area.is_empty() {
            return;
        }

        let alpha = self.timer.alpha();

        // 起步态/结束态不做高亮：
        // - 避免 duration=0 的首帧就“露白”（会破坏空屏起步）
        // - 避免最后一帧残留白边（影响静态观感）
        if alpha <= 0.001 || alpha >= 0.999 {
            return;
        }

        // 目前只用在 UpToDown（启动/入场的 sweep 方向）。
        // 其它方向先不做，避免引入没验证过的视觉副作用。
        if self.direction != Motion::UpToDown {
            return;
        }

        // sweep 前沿：用 alpha 映射到 [top..bottom]。
        //
        // 关键点：
        // - 如果用 `(height - 1)` 做映射，bottom_row 只有在 alpha==1.0 才会命中；
        // - 但我们为了避免“尾帧残留白边”，会在 alpha≈1 时提前 return；
        // - 结果就是：下边框永远不会被高亮（用户反馈：下边框完全看不到动画）。
        //
        // 这里改为用 `height` 做映射，再 clamp 到 `height-1`：
        // - alpha 只要进入最后一段区间（例如 >= (height-1)/height），就能命中 bottom_row；
        // - 同时仍然可以在 alpha≈1 的时候提前停止绘制，避免残留。
        let max_rel = f32::from(area.height);
        // 用 floor（而不是 round）避免前沿在相邻行之间“抖动跳帧”。
        let rel = (alpha * max_rel).floor() as u16;
        let rel = rel.min(area.height.saturating_sub(1));
        let front_y = area.y.saturating_add(rel);

        let start_y = front_y.saturating_sub(self.band_height.saturating_sub(1));
        let end_y = front_y;
        let highlight_fg = self.highlight_fg;
        let highlight_bg = self.highlight_bg;

        let iter = self.cell_iter(buf, area);
        iter.for_each_cell(|pos, cell| {
            if pos.y >= start_y && pos.y <= end_y {
                cell.set_fg(highlight_fg);
                if let Some(bg) = highlight_bg {
                    cell.set_bg(bg);
                }
            }
        });
    }
}

fn border_highlight_sweep(
    area: Rect,
    highlight_fg: Color,
    highlight_bg: Option<Color>,
    timer: EffectTimer,
) -> Effect {
    // 带宽：见 `BorderHighlightSweep::band_height` 的解释。
    const BAND_HEIGHT: u16 = 2;
    Effect::new(BorderHighlightSweep::new(
        Motion::UpToDown,
        BAND_HEIGHT,
        highlight_fg,
        highlight_bg,
        timer,
    ))
    .with_area(area)
    // 只作用于真正的 border ring（1-cell），避免高亮区域“太厚”。
    .with_filter(CellFilter::Outer(Margin::new(1, 1)))
}

/// 启动动画的唯一 id（用于 `EffectManager::add_unique_effect`）。
pub const STARTUP_ANIMATION_KEY: &str = "tui.startup.open";

/// Output 重启动画的唯一 id（切换实例时触发）。
pub const OUTPUT_REOPEN_ANIMATION_KEY: &str = "tui.output.reopen";

/// 启动动画的节奏（逐块出场）。
///
/// 说明：
/// - 需求是“逐块依次出场”，因此把启动动画拆成多个 stage 串行执行
/// - 这里的总时长是一个 UX 取舍：要“看得见”，但不要拖慢进入界面太久
pub const STARTUP_PANE_MS: u32 = 420;
/// 相邻 pane 的错峰启动间隔（让入场动画重叠进行，而不是完全串行）。
///
/// 目标节奏（示意）：
/// - pane1: [----]
/// - pane2:   [----]
/// - pane3:     [----]
///
/// 即：下一个 pane 在上一个 pane “进行到一半”时就开始入场。
pub const STARTUP_GAP_MS: u32 = STARTUP_PANE_MS / 2;
/// 启动动画总时长（以最后一个 pane 完成的时间为准）。
pub const STARTUP_TOTAL_MS: u32 = STARTUP_PANE_MS + STARTUP_GAP_MS * 2;

/// Output 重新打开动画的节奏（先消失，再出场）。
pub const OUTPUT_REOPEN_IN_MS: u32 = 420;

/// 终端过小会导致动画观感差、甚至出现闪烁/错位，因此直接降级为“无动画”。
///
/// 说明：
/// - 这不是功能性门槛，只是体验/稳定性的保守阈值
/// - 如果你想更激进，可以降低阈值；但不建议在 40x10 以下启用动画
pub const MIN_STARTUP_ANIM_WIDTH: u16 = 60;
pub const MIN_STARTUP_ANIM_HEIGHT: u16 = 12;

/// 动画总开关：是否启用动态效果。
///
/// 规则：
/// - `RALPH_TUI_REDUCED_MOTION=1|true|yes` → 关闭动画（无障碍 / CI / 录屏 / 回放）
/// - 默认开启（但仍可能因为窗口过小而自动降级）
pub fn animations_enabled() -> bool {
    // 非交互环境（stdout 不是 TTY）下，直接禁用动画：
    // - 避免在重定向/日志采集场景引入额外控制序列与闪烁
    // - 符合“可用性优先”的降级原则
    if !std::io::stdout().is_terminal() {
        return false;
    }

    let reduced = std::env::var("RALPH_TUI_REDUCED_MOTION")
        .ok()
        .map(|v| v.to_lowercase())
        .is_some_and(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"));

    !reduced
}

pub fn should_run_startup_animation(area: Rect) -> bool {
    area.width >= MIN_STARTUP_ANIM_WIDTH && area.height >= MIN_STARTUP_ANIM_HEIGHT
}

/// 构建启动打开动画 Effect（全屏版本）。
///
/// 设计：
/// - sweep-in（由上到下）逐步“揭开”已渲染的 UI
/// - 以 app 的背景色作为 faded_color，让动画与主题融合
pub fn startup_open_effect(theme: TuiTheme, area: Rect) -> Effect {
    let bg = theme.app_bg_color();
    let timer = (STARTUP_PANE_MS, Interpolation::QuadOut);

    // Warp 透明背景模式（bg=Reset）下，不能用 sweep/fade 来“藏住”内容：
    // - sweep_in 只改 fg/bg，不改 symbol；而 fg/bg=Reset 仍会显示默认前景色 → 仍可见
    // - 同时 Reset 参与插值时还可能被当作黑色，带来“背景变动”的副作用
    //
    // 这里改用 slide_in：
    // - 它会通过逐步把 cell 的 symbol 改为空格来“遮罩”区域（遮罩背景仍是 Reset）
    // - 因此在动画开始前，区域会真实变成“空白屏”
    if bg == Color::Reset {
        return fx::slide_in(Motion::UpToDown, symbol_sweep_gradient(area), 0, bg, timer)
            .with_area(area);
    }

    // 说明：
    // - gradient_length 用 area.height，让过渡更平滑（类似 exabind 的风格）
    // - randomness=0 保持稳定、可重复（避免录屏/回放中出现随机闪烁）
    fx::parallel(&[
        fx::sweep_in(Motion::UpToDown, area.height.max(1), 0, bg, timer),
        fx::fade_from_fg(bg, (STARTUP_PANE_MS, Interpolation::SineOut)),
    ])
}

/// 构建启动打开动画 Effect（并行 Supervisor：逐块出场）。
///
/// 出场顺序（错峰重叠）：
/// 1) Instances 框体（左上）
/// 2) Instances 条目（必须在框体完成后）
/// 3) Output（右上）
/// 4) Chat/Gates（下方）
pub fn startup_open_effect_parallel(
    theme: TuiTheme,
    header_area: Rect,
    instances_area: Rect,
    instances_inner: Rect,
    output_area: Rect,
    bottom_area: Rect,
    footer_area: Rect,
) -> Effect {
    let app_bg = theme.app_bg_color();

    // Warp 透明背景模式（bg=Reset）下：
    // - sweep_in/fade 无法把内容“藏起来”（fg/bg=Reset 仍会显示默认前景色）
    // - paint 也无法隐藏文字（paint 不会改 symbol）
    //
    // 因此这里用“基于 symbol 的遮罩”实现：
    // - 对每个 pane area 使用 slide_in（从上到下）：
    //   - 初始是“全空白”（因为 slide_in 的 timer 是 reversed，alpha=1）
    //   - 随时间推进逐步露出真实内容
    // - 用 prolong_start 做“错峰重叠”的编排：Instances(frame) 先出场，其它 pane 在半程/完成点开始入场
    if app_bg == Color::Reset {
        let pane_timer = (STARTUP_PANE_MS, Interpolation::QuadOut);

        let reveal = |area: Rect| {
            fx::slide_in(
                Motion::UpToDown,
                symbol_sweep_gradient(area),
                0,
                app_bg,
                pane_timer,
            )
            .with_area(area)
        };

        // 入场编排：错峰重叠（而非完全串行）。
        //
        // 约束：
        // - Instances 条目必须晚于框体（所以 items 仍然等 frame 完成后才开始）。
        let delay_output = STARTUP_GAP_MS; // Instances 进行到一半时，Output 开始
        let delay_instances_items = STARTUP_PANE_MS; // Instances 框体完成后，条目开始
        let delay_bottom = STARTUP_GAP_MS * 2; // Output 进行到一半时，Bottom 开始

        return fx::parallel(&[
            // Stage 1: Header/Instances 框体出场
            //
            // 需求：启动时先“真正空屏”，再开始逐块出场。
            // - Header/Footer 也是视觉上的“块”，如果它们首帧可见，用户仍会感知到“先全显示一帧”。
            // - 因此这里把 Header/Footer 也纳入启动遮罩，并与 Stage 1 同步出场。
            reveal(header_area),
            reveal(footer_area),
            reveal(instances_area),
            // Stage 2: Instances 条目出场（延迟启动，确保“先框后字”）
            fx::prolong_start(delay_instances_items, reveal(instances_inner)),
            // Stage 3: Output 出场
            fx::prolong_start(delay_output, reveal(output_area)),
            // Stage 4: Chat/Gates 出场
            fx::prolong_start(delay_bottom, reveal(bottom_area)),
        ]);
    }

    let pane_timer = (STARTUP_PANE_MS, Interpolation::QuadOut);
    let pane_fade = (STARTUP_PANE_MS, Interpolation::SineOut);

    // ---------------------------------------------------------------------
    // 非 Warp（显式背景色）并行启动动画：必须“首帧空屏”
    // ---------------------------------------------------------------------
    //
    // 用户反馈（Alacritty 等终端）：
    // - 之前用 `fx::sequence` 串行 stage：只有第一个 stage 在跑时会“遮住”区域；
    //   其它 pane 在轮到自己之前会保持可见，导致“先全显示一帧 → 再开始逐块动画”的闪烁。
    //
    // 解决策略：
    // - 改为 `fx::parallel + fx::prolong_start`：
    //   - 每个 pane 的 effect 从 0ms 起就“常驻”在自己的初始隐藏态；
    //   - 到达 delay 后才开始推进 timer，从隐藏态逐步 reveal。
    //
    // 关键点（非常重要）：
    // - prolong_start 在 delay 期间也会执行 inner.process(0ms)，因此：
    //   - “初始态”必须是不可见态；
    //   - 对非 Warp 来说，最安全的不可见态是 `faded_color=app_bg`（与整屏背景一致）。
    let sweep_fade = |area: Rect, faded: Color| {
        fx::parallel(&[
            fx::sweep_in(Motion::UpToDown, area.height.max(1), 0, faded, pane_timer)
                .with_area(area),
            fx::fade_from_fg(faded, pane_fade).with_area(area),
        ])
    };

    let sweep_fade_panel = |area: Rect, faded: Color| {
        fx::parallel(&[
            fx::sweep_in(Motion::UpToDown, area.height.max(1), 0, faded, pane_timer)
                .with_area(area),
            fx::fade_from_fg(faded, pane_fade).with_area(area),
            // 只给边框加一条“亮线扫过”的反馈：
            // - 不改变 pane 的底色策略
            // - 也不影响“首帧空屏”（我们的 shader 在 alpha≈0 时不做任何事）
            border_highlight_sweep(
                area,
                Color::White,
                Some(theme.colors().surface1),
                EffectTimer::from_ms(STARTUP_PANE_MS, Interpolation::QuadOut),
            ),
        ])
    };

    // 入场编排：错峰重叠（而非完全串行）。
    //
    // 约束：
    // - Instances 条目必须晚于框体（所以 items 仍然等 frame 完成后才开始）。
    let delay_output = STARTUP_GAP_MS; // Instances 进行到一半时，Output 开始
    let delay_instances_items = STARTUP_PANE_MS; // Instances 框体完成后，条目开始
    let delay_bottom = STARTUP_GAP_MS * 2; // Output 进行到一半时，Bottom 开始

    fx::parallel(&[
        // Stage 1: Header/Footer + Instances 框体出场
        sweep_fade(header_area, app_bg),
        sweep_fade(footer_area, app_bg),
        sweep_fade_panel(instances_area, app_bg),
        // Stage 2: Instances 条目出场（注意：faded_color 仍用 app_bg，保证 delay 期间首帧不可见）
        fx::prolong_start(delay_instances_items, sweep_fade(instances_inner, app_bg)),
        // Stage 3: Output 出场
        fx::prolong_start(delay_output, sweep_fade_panel(output_area, app_bg)),
        // Stage 4: Chat/Gates 出场
        fx::prolong_start(delay_bottom, sweep_fade_panel(bottom_area, app_bg)),
    ])
}

/// Output 重启动画：切换实例时，让 Output 像“重新打开”一样。
///
/// 行为：
/// - 首帧强制从“隐藏态”起步（避免切换实例时出现“先显示一帧再消失”的闪烁）
/// - 然后 sweep-in 回真实内容（看起来像“打开/出现”）
pub fn output_reopen_effect(theme: TuiTheme, output_area: Rect) -> Effect {
    let in_timer = (OUTPUT_REOPEN_IN_MS, Interpolation::QuadOut);

    // faded_color 的选择：
    // - Warp（app bg=Reset）：为了避免 Reset→Black 的插值副作用，使用 pane 底色（base）。
    // - 非 Warp（app bg=crust）：沿用 app 的背景色，让输出“融进外侧背景”。
    let faded = if theme.app_bg_color() == Color::Reset {
        theme.panel_bg_color()
    } else {
        theme.app_bg_color()
    };

    // Warp（bg=Reset）下不做边框高亮：
    // - 边框 cell 的 bg 我们会刷回 Reset（保持半透明），参与插值更容易触发终端差异；
    // - 用户也明确表示 Warp 的外圈染色“先不管”，这里先稳住最小影响面。
    if theme.app_bg_color() == Color::Reset {
        return fx::parallel(&[
            fx::sweep_in(
                Motion::UpToDown,
                output_area.height.max(1),
                0,
                faded,
                in_timer,
            )
            .with_area(output_area),
            fx::fade_from_fg(faded, (OUTPUT_REOPEN_IN_MS, Interpolation::SineOut))
                .with_area(output_area),
        ]);
    }

    fx::parallel(&[
        fx::sweep_in(
            Motion::UpToDown,
            output_area.height.max(1),
            0,
            faded,
            in_timer,
        )
        .with_area(output_area),
        fx::fade_from_fg(faded, (OUTPUT_REOPEN_IN_MS, Interpolation::SineOut))
            .with_area(output_area),
        border_highlight_sweep(
            output_area,
            Color::White,
            Some(theme.colors().surface1),
            EffectTimer::from_ms(OUTPUT_REOPEN_IN_MS, Interpolation::QuadOut),
        ),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

    #[test]
    fn border_highlight_sweep_highlights_outer_border_with_band() {
        // 这个测试锁定“边框高亮 sweep”确实会对 outer border 生效，
        // 且带宽>1 时，会在竖边形成可见的“亮段”（而不是每帧只有 2 个点太难察觉）。
        let frame = Rect::new(0, 0, 20, 10);
        let area = Rect::new(2, 2, 10, 6);
        let mut buf = Buffer::empty(frame);

        // 初始化：整屏先填一个底色，避免出现“未初始化 cell”导致的偶然性。
        for y in 0..frame.height {
            for x in 0..frame.width {
                let cell = &mut buf[(x, y)];
                cell.set_char('x');
                cell.fg = Color::Blue;
                cell.bg = Color::Black;
            }
        }

        // 先把“边框 ring”与“内容区”设成不同 fg，便于断言过滤范围正确。
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                // 这里的“边框区域”按 `Outer(Margin::new(1,1))` 的语义来理解：
                // - outer=1 意味着只有最外侧一圈属于“边框区域”（避免高亮太厚）。
                let core = area.inner(Margin::new(1, 1));
                let in_core = x >= core.x
                    && x < core.x.saturating_add(core.width)
                    && y >= core.y
                    && y < core.y.saturating_add(core.height);
                let is_border = !in_core;
                let cell = &mut buf[(x, y)];
                cell.fg = if is_border { Color::Red } else { Color::Green };
            }
        }

        let mut effect = border_highlight_sweep(
            area,
            Color::White,
            Some(Color::DarkGray),
            EffectTimer::from_ms(100, Interpolation::Linear),
        );
        // 取一半进度：front_y 落在中间附近，带宽=3 时应覆盖 3 行的边框。
        effect.process(std::time::Duration::from_millis(50).into(), &mut buf, frame);

        // 断言：核心内容区不应被影响（仍是 Green）。
        let core = area.inner(Margin::new(1, 1));
        assert_eq!(buf[(core.x, core.y)].fg, Color::Green);

        // 断言：至少存在一个竖边 cell 被刷成 White（高亮）。
        //
        // 我们不写死具体 y（避免未来调参导致测试脆弱），只检查“边框里确实有白点”即可。
        let mut any_white_on_border = false;
        for y in area.y..area.y.saturating_add(area.height) {
            for x in area.x..area.x.saturating_add(area.width) {
                let core = area.inner(Margin::new(1, 1));
                let in_core = x >= core.x
                    && x < core.x.saturating_add(core.width)
                    && y >= core.y
                    && y < core.y.saturating_add(core.height);
                let is_border = !in_core;
                if is_border && buf[(x, y)].fg == Color::White {
                    any_white_on_border = true;
                }
            }
        }
        assert!(any_white_on_border);
    }

    #[test]
    fn border_highlight_sweep_can_reach_bottom_row_before_end() {
        // 这个测试锁定一个真实踩坑：
        // - 如果 y 前沿用 `(height - 1)` 映射，再加上“尾帧提前 return”，
        //   bottom_row 会永远命中不到 → 下边框看不到动画。
        //
        // 这里用接近结束（但还没到最后一帧）的进度，确保能命中 bottom_row。
        let frame = Rect::new(0, 0, 20, 10);
        let area = Rect::new(2, 2, 10, 6);
        let mut buf = Buffer::empty(frame);

        for y in 0..frame.height {
            for x in 0..frame.width {
                let cell = &mut buf[(x, y)];
                cell.set_char('x');
                cell.fg = Color::Red;
                cell.bg = Color::Black;
            }
        }

        let mut effect = border_highlight_sweep(
            area,
            Color::White,
            None,
            EffectTimer::from_ms(100, Interpolation::Linear),
        );

        // 99% 进度：仍应当处于“可绘制阶段”（避开 alpha>=0.999 的尾帧禁绘）。
        effect.process(std::time::Duration::from_millis(99).into(), &mut buf, frame);

        let bottom_y = area.y.saturating_add(area.height.saturating_sub(1));
        let mut any_white_on_bottom = false;
        for x in area.x..area.x.saturating_add(area.width) {
            if buf[(x, bottom_y)].fg == Color::White {
                any_white_on_bottom = true;
                break;
            }
        }
        assert!(any_white_on_bottom);
    }

    #[test]
    fn startup_open_effect_parallel_terminal_default_bg_starts_from_blank_screen() {
        let theme = TuiTheme::default().with_terminal_default_bg();
        let frame = Rect::new(0, 0, 80, 24);

        // 模拟真实布局：
        // - header/footer 各 2 行
        // - content 中 main=8 行、bottom=12 行（与 App 的固定高度接近）
        let header = Rect::new(0, 0, 80, 2);
        let footer = Rect::new(0, 22, 80, 2);
        let instances = Rect::new(0, 2, 30, 8);
        let instances_inner = Rect::new(1, 3, 28, 6);
        let output = Rect::new(30, 2, 50, 8);
        let bottom = Rect::new(0, 10, 80, 12);

        let mut buf = Buffer::empty(frame);

        // 先填满“可见内容”，确保如果动画没有把首帧遮住，测试能立刻发现。
        for y in 0..frame.height {
            for x in 0..frame.width {
                let cell = &mut buf[(x, y)];
                cell.set_char('x');
                cell.fg = theme.colors().text;
                cell.bg = Color::Reset;
            }
        }

        let mut effect = startup_open_effect_parallel(
            theme,
            header,
            instances,
            instances_inner,
            output,
            bottom,
            footer,
        );
        effect.process(std::time::Duration::from_millis(0).into(), &mut buf, frame);

        // 关键断言：首帧必须是“空屏”（全部是空格），
        // 避免出现“先全显示一帧，再开始逐块动画”的闪烁观感。
        for y in 0..frame.height {
            for x in 0..frame.width {
                assert_eq!(buf[(x, y)].symbol(), " ");
            }
        }
    }

    #[test]
    fn startup_open_effect_parallel_non_terminal_default_bg_starts_from_blank_screen() {
        // 非 Warp（显式背景色）下也必须做到“首帧空屏”：
        // - 不能出现“先全显示一帧，再开始逐块动画”的闪烁。
        let theme = TuiTheme::default();
        let frame = Rect::new(0, 0, 80, 24);

        let header = Rect::new(0, 0, 80, 2);
        let footer = Rect::new(0, 22, 80, 2);
        let instances = Rect::new(0, 2, 30, 8);
        let instances_inner = Rect::new(1, 3, 28, 6);
        let output = Rect::new(30, 2, 50, 8);
        let bottom = Rect::new(0, 10, 80, 12);

        let mut buf = Buffer::empty(frame);

        // 先填满“可见内容”，确保如果动画没有把首帧遮住，测试能立刻发现。
        for y in 0..frame.height {
            for x in 0..frame.width {
                let cell = &mut buf[(x, y)];
                cell.set_char('x');
                cell.fg = theme.colors().text;
                cell.bg = theme.colors().base;
            }
        }

        let mut effect = startup_open_effect_parallel(
            theme,
            header,
            instances,
            instances_inner,
            output,
            bottom,
            footer,
        );
        effect.process(std::time::Duration::from_millis(0).into(), &mut buf, frame);

        // 非 Warp 分支用“颜色遮罩”隐藏：
        // - 符号不一定是空格（它仍可能是 'x'），
        // - 但只要 fg/bg 都被刷成 app_bg（crust），就应当是“肉眼不可见”的空屏。
        let app_bg = theme.app_bg_color();
        for y in 0..frame.height {
            for x in 0..frame.width {
                assert_eq!(buf[(x, y)].fg, app_bg);
                assert_eq!(buf[(x, y)].bg, app_bg);
            }
        }
    }

    #[test]
    fn output_reopen_effect_terminal_default_bg_does_not_paint_black_background() {
        let theme = TuiTheme::default().with_terminal_default_bg();
        let area = Rect::new(0, 0, 24, 6);
        let mut buf = Buffer::empty(area);

        // 先填充一些“有内容”的 cell，模拟 Output 区域已经渲染出文本。
        // 关键点：Warp 模式下我们保留 panel 底色（base），
        // 这样 sweep 渐变既好看，也不会触发 Reset→Black 的插值分支。
        for y in 0..area.height {
            for x in 0..area.width {
                let cell = &mut buf[(x, y)];
                cell.set_char('x');
                cell.fg = theme.colors().text;
                cell.bg = theme.panel_bg_color();
            }
        }

        let mut effect = output_reopen_effect(theme, area);
        effect.process(
            std::time::Duration::from_millis(u64::from(OUTPUT_REOPEN_IN_MS) / 2).into(),
            &mut buf,
            area,
        );

        // 断言：动画中不应出现“黑底”（这会导致整屏背景被感知为变暗/闪烁）。
        // 这能捕捉到 tachyonfx 在插值阶段把 `cell.bg==Reset` 临时当作 Black 的问题。
        for y in 0..area.height {
            for x in 0..area.width {
                assert_ne!(buf[(x, y)].bg, Color::Black);
            }
        }
    }
}
