//! TUI 动画：启动打开动画（open animation）与开关策略。
//!
//! 目标：
//! - 在进入 alternate screen 的第一屏提供更“有反馈”的视觉过渡
//! - 保持可用性优先：可禁用、可在小窗口自动降级

use crate::theme::TuiTheme;
use ratatui::{layout::Rect, style::Color};
use std::io::IsTerminal;
use tachyonfx::{Effect, Interpolation, Motion, fx};

/// Warp 透明背景模式（`bg=Reset`）下的“symbol 遮罩”渐变宽度上限。
///
/// 为什么需要上限：
/// - `fx::slide_in/out` 的 `gradient_length` 太大时，会在屏幕上形成一大片“块字符渐变带”，
///   观感会变得“糊、厚、慢”，不像之前的 sweep 白条。
/// - 我们把渐变宽度收窄到一个合理范围，让“白条扫过”更干净、更有速度感。
const SYMBOL_SWEEP_GRADIENT_MAX: u16 = 10;

fn symbol_sweep_gradient(area: Rect) -> u16 {
    area.height.max(1).min(SYMBOL_SWEEP_GRADIENT_MAX)
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
pub const STARTUP_GAP_MS: u32 = 80;
pub const STARTUP_TOTAL_MS: u32 = STARTUP_PANE_MS * 4 + STARTUP_GAP_MS * 3;

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
/// 出场顺序（严格串行）：
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
    let panel_bg = theme.panel_bg_color();

    // Warp 透明背景模式（bg=Reset）下：
    // - sweep_in/fade 无法把内容“藏起来”（fg/bg=Reset 仍会显示默认前景色）
    // - paint 也无法隐藏文字（paint 不会改 symbol）
    //
    // 因此这里用“基于 symbol 的遮罩”实现：
    // - 对每个 pane area 使用 slide_in（从上到下）：
    //   - 初始是“全空白”（因为 slide_in 的 timer 是 reversed，alpha=1）
    //   - 随时间推进逐步露出真实内容
    // - 用 prolong_start 做严格串行：Instances(frame) → Instances(items) → Output → Chat/Gates
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

        let delay_instances_items = STARTUP_PANE_MS + STARTUP_GAP_MS;
        let delay_output = delay_instances_items + STARTUP_PANE_MS + STARTUP_GAP_MS;
        let delay_bottom = delay_output + STARTUP_PANE_MS + STARTUP_GAP_MS;

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

    // Stage 1: Instances 框体出场（同时把 inner 文本“涂掉”，实现先框后字）
    let instances_frame = fx::parallel(&[
        fx::sweep_in(
            Motion::UpToDown,
            instances_area.height.max(1),
            0,
            app_bg,
            pane_timer,
        )
        .with_area(instances_area),
        fx::fade_from_fg(app_bg, pane_fade).with_area(instances_area),
        // 关键技巧：
        // - 用 paint 把 inner 的 fg/bg 都涂成 panel_bg，让条目在框体动画阶段“不可见”
        // - timer 设成与框体阶段一致，这样每帧都会在渲染后覆盖一次（避免下一帧又被 widget 画出来）
        fx::paint(panel_bg, panel_bg, STARTUP_PANE_MS).with_area(instances_inner),
    ]);

    // Stage 2: Instances 条目出场（只作用于 inner，逐行 sweep-in）
    let instances_items = fx::parallel(&[
        fx::sweep_in(
            Motion::UpToDown,
            instances_inner.height.max(1),
            0,
            panel_bg,
            pane_timer,
        )
        .with_area(instances_inner),
        fx::fade_from_fg(panel_bg, pane_fade).with_area(instances_inner),
    ]);

    // Stage 3: Output 出场（右上）
    let output_pane = fx::parallel(&[
        fx::sweep_in(
            Motion::UpToDown,
            output_area.height.max(1),
            0,
            app_bg,
            pane_timer,
        )
        .with_area(output_area),
        fx::fade_from_fg(app_bg, pane_fade).with_area(output_area),
    ]);

    // Stage 4: Chat/Gates 出场（下方）
    let bottom_pane = fx::parallel(&[
        fx::sweep_in(
            Motion::UpToDown,
            bottom_area.height.max(1),
            0,
            app_bg,
            pane_timer,
        )
        .with_area(bottom_area),
        fx::fade_from_fg(app_bg, pane_fade).with_area(bottom_area),
    ]);

    // 串行编排：一个 stage 结束后才进入下一个（符合“逐个进行”的要求）。
    fx::sequence(&[
        instances_frame,
        fx::sleep(STARTUP_GAP_MS),
        instances_items,
        fx::sleep(STARTUP_GAP_MS),
        output_pane,
        fx::sleep(STARTUP_GAP_MS),
        bottom_pane,
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
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::buffer::Buffer;

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
