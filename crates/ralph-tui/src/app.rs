//! Main application loop for the TUI.
//!
//! This module provides a read-only observation dashboard that displays
//! formatted output from the Ralph orchestrator, with iteration navigation,
//! scroll, and search functionality.

use crate::animation;
use crate::chat::{ChatSubmit, parse_chat_submit};
use crate::external_event_writer::ExternalEventWriter;
use crate::input::{Action, map_key};
use crate::state::{GateStatus, ParallelFocus, TuiMode, TuiState, TuiUpdate};
use crate::theme::{TuiTheme, panel_block, patch_exabind_panel_border_bg};
use crate::widgets::{
    content::{ContentPane, SelectionBounds},
    footer, header, help, instances,
    parallel_output::ParallelOutputPane,
};
use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{
        DisableMouseCapture, EnableMouseCapture, Event, EventStream, KeyCode, KeyEventKind,
        KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use futures::StreamExt;
use ralph_core::truncate_with_ellipsis;
use ralph_proto::{GateResolve, GateResolvedBy, HatInstanceId, TOPIC_GATE_RESOLVE};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};
use scopeguard::defer;
use std::io;
use std::io::IsTerminal;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tachyonfx::{Duration as FxDuration, EffectManager};
use tokio::sync::mpsc;
use tokio::sync::watch;
use tokio::time::{Duration, interval};
use tracing::info;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// 并行模式下用于鼠标 hit-test 的布局快照。
///
/// 说明：
/// - 该结构只在 `App::run` 的局部变量里保存“最近一次渲染”的 Rect。
/// - 这样输入事件可以在下一帧到来前做 hit-test，而无需把布局塞进 state（保持 reducer 纯净）。
#[derive(Debug, Clone, Copy)]
struct ParallelLayoutSnapshot {
    instances_inner: ratatui::layout::Rect,
    output_inner: ratatui::layout::Rect,
    bottom_inner: ratatui::layout::Rect,
    chat_input_area: ratatui::layout::Rect,
    chat_targets_area: ratatui::layout::Rect,
    gate_list_area: ratatui::layout::Rect,
    gate_actions_area: ratatui::layout::Rect,
}

/// gate 快捷操作（actions chips）的枚举（用于 hit-test 后预填输入框）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateActionChip {
    Approve,
    Deny,
    Resolve,
}

// =============================================================================
// Warp：保留“半透明窗口背景”效果
// =============================================================================
//
// 用户反馈（Warp）：
// - 窗口 padding（字符栅格之外）仍然发灰；
// - 且在动画时能观察到“外圈”也会随之变色。
//
// 解释：
// - ratatui 只能控制“字符栅格(cell)”的 fg/bg；
// - Warp 的“半透明/blur”通常是窗口级效果：它会作用在终端默认背景上（以及 UI 范围外的 padding）。
// - 如果我们在 cell 上大量绘制显式 bg（crust/base），内容区会变成纯色不透明，
//   与 padding 的半透明背景脱钩，视觉上就像外圈发灰一圈。
//
// 策略：
// - 在 Warp + TTY 下，启用 `TuiTheme::with_terminal_default_bg()`：
//   - app 背景用 `bg=Reset`（使用终端默认背景），让“非 pane 区域”与 padding 共享同一套半透明背景
//   - pane 内部仍保留主题底色（base），提升可读性，并避免动画出现刺眼的纯白条

fn is_warp_terminal() -> bool {
    std::env::var("TERM_PROGRAM")
        .ok()
        .map(|v| v.to_ascii_lowercase())
        .is_some_and(|v| v.contains("warp"))
}

fn contains_point(area: ratatui::layout::Rect, x: u16, y: u16) -> bool {
    x >= area.x
        && x < area.x.saturating_add(area.width)
        && y >= area.y
        && y < area.y.saturating_add(area.height)
}

/// 动画首帧 priming：强制从“初始态”起步，避免闪烁。
///
/// 背景：
/// - tachyonfx 的默认 Shader::process 流程是：先推进 timer，再 execute。
/// - 如果 TUI 首帧渲染被输入事件拖慢（或首次 tick 间隔偏大），`fx_delta` 可能很大。
/// - 若直接用大 delta 处理“刚添加的动画”，动画会从中途开始：
///   - 启动进场：观感像“先把全部面板画出来 → 再扫一遍动画”，会非常怪且闪。
///   - Output 重启：观感像“先显示新内容一帧 → 再消失 → 再入场”，会闪烁。
///
/// 策略：
/// - 在“动画刚被添加”的那一帧，把 delta 强制归零，让第一帧先渲染出纯粹的起步态。
/// - 下一帧开始再用正常的 delta 推进时间轴。
fn prime_animation_first_frame(fx_delta: &mut FxDuration, last_effect_tick: &mut Instant) {
    *fx_delta = FxDuration::from_millis(0);
    *last_effect_tick = Instant::now();
}

fn inner_block(area: ratatui::layout::Rect) -> ratatui::layout::Rect {
    // Block::inner 的等价计算（borders=ALL）。
    // 注意：width/height 可能小于 2，这里用 saturating_* 避免 underflow。
    ratatui::layout::Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

fn hat_graph_radar_area(
    content_area: ratatui::layout::Rect,
    zoomed: bool,
) -> Option<ratatui::layout::Rect> {
    // =========================================================================
    // Hat Graph Radar 的布局策略（右上角覆盖层）
    //
    // 目标：
    // - 默认小窗：像游戏右上角“雷达”，尽量不遮挡正文；
    // - 放大模式：扩大可视面积，更易读；
    // - 终端过小：宁可不显示，也不要越界/挤爆布局。
    // =========================================================================

    // 需要至少能容纳一个带边框的 panel（最小：宽 8，高 5）。
    if content_area.width < 8 || content_area.height < 5 {
        return None;
    }

    let (mut width, mut height) = if zoomed {
        // 放大：占 content 区域约 2/3，并做上限裁剪，避免超大屏占用过多视线。
        let w = content_area.width.saturating_mul(2) / 3;
        let h = content_area.height.saturating_mul(2) / 3;
        (w.min(120), h.min(40))
    } else {
        // 小窗：固定上限尺寸（更像“雷达”），但不超过 content 区域。
        (content_area.width.min(36), content_area.height.min(10))
    };

    width = width.max(8).min(content_area.width);
    height = height.max(5).min(content_area.height);
    if width < 8 || height < 5 {
        return None;
    }

    // 右上角锚定：向左/向下扩张（符合“雷达放大”的直觉）。
    let x = content_area
        .x
        .saturating_add(content_area.width.saturating_sub(width));
    let y = content_area.y;

    Some(ratatui::layout::Rect {
        x,
        y,
        width,
        height,
    })
}

fn render_hat_graph_radar_overlay(
    f: &mut ratatui::Frame,
    content_area: ratatui::layout::Rect,
    state: &TuiState,
    theme: &TuiTheme,
) -> Option<ratatui::layout::Rect> {
    let radar = state.hat_graph_radar.as_ref()?;
    let zoomed = state.hat_graph_zoomed;

    let area = hat_graph_radar_area(content_area, zoomed)?;
    let inner = inner_block(area);
    if inner.width == 0 || inner.height == 0 {
        return None;
    }

    let title = if zoomed {
        "Hat Graph (p: mini)"
    } else {
        "Hat Graph (p: zoom)"
    };
    let block = panel_block(title, zoomed, theme);
    f.render_widget(block, area);
    patch_exabind_panel_border_bg(f.buffer_mut(), area, theme);

    let graph = if zoomed {
        radar.ascii_full.as_str()
    } else {
        radar.ascii_compact.as_str()
    };

    let lines: Vec<Line> = graph
        .lines()
        .take(inner.height as usize)
        .map(|l| Line::from(Span::styled(l.to_string(), theme.muted())))
        .collect();

    // 注意：这里不启用自动换行（wrap），因为换行会破坏 ASCII 图的结构。
    let paragraph = Paragraph::new(lines).style(theme.muted());
    f.render_widget(paragraph, inner);

    Some(area)
}

fn request_interrupt(tx: Option<&watch::Sender<bool>>) {
    if let Some(tx) = tx {
        let _ = tx.send(true);
    }
}

fn clamp_to_area(value: u16, start: u16, len: u16) -> u16 {
    if len == 0 {
        return start;
    }
    let end = start.saturating_add(len.saturating_sub(1));
    value.clamp(start, end)
}

fn clamp_usize(value: usize, max_exclusive: usize) -> usize {
    if max_exclusive == 0 {
        return 0;
    }
    value.min(max_exclusive.saturating_sub(1))
}

fn chat_editor_pad_top(viewport_rows: usize, total_lines: usize) -> usize {
    // =========================================================================
    // Chat 输入框垂直对齐策略（与渲染/hit-test 共用）
    //
    // 目标：
    // 1) 当内容行数不足输入框高度时，把内容“下移”，更像聊天输入区；
    // 2) 但不要“贴底”，保留 1 行底部留白，让输入内容与下方 Targets 行有呼吸间距。
    //
    // 说明：
    // - diff = viewport_rows - total_lines：可用的空白行数
    // - pad_top = diff - 1：上方 padding，底部固定留 1 行空白（diff=0/1 时自动退化）
    // =========================================================================
    viewport_rows.saturating_sub(total_lines).saturating_sub(1)
}

fn hit_test_chat_editor(
    editor: &crate::state::ChatEditorState,
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> crate::state::TextPos {
    if area.width == 0 || area.height == 0 {
        return crate::state::TextPos::default();
    }

    // 约定：prompt 占 3 个 cell（" " + ">" + " "）
    let prefix_cells: u16 = 3;
    let content_width = area.width.saturating_sub(prefix_cells);

    let viewport_rows = area.height as usize;
    let total_lines = editor.lines.len().max(1);
    let cursor_row = editor.cursor.row.min(total_lines.saturating_sub(1));
    // 与渲染逻辑保持一致：
    // - 当总行数不足输入框高度时，渲染会做“底部对齐”（上方 padding）。
    // - 当总行数超过输入框高度时，渲染会做“跟随光标”的垂直滚动。
    let start_row = if total_lines <= viewport_rows {
        0
    } else {
        cursor_row.saturating_sub(viewport_rows.saturating_sub(1))
    };
    let pad_top = chat_editor_pad_top(viewport_rows, total_lines);

    let rel_y = y.saturating_sub(area.y) as usize;
    let rel_y = rel_y.saturating_sub(pad_top);
    let mut row = start_row.saturating_add(rel_y);
    row = row.min(total_lines.saturating_sub(1));

    let rel_x = x.saturating_sub(area.x);
    let content_x = rel_x.saturating_sub(prefix_cells);

    let line_text = editor.lines.get(row).map(|s| s.as_str()).unwrap_or("");
    let graphemes: Vec<&str> = UnicodeSegmentation::graphemes(line_text, true).collect();
    let widths: Vec<u16> = graphemes
        .iter()
        .map(|g| UnicodeWidthStr::width(*g) as u16)
        .collect();

    let line_len = graphemes.len();

    // 仅对“当前光标行”应用水平滚动（对齐渲染逻辑）
    let scroll_cell = if row == cursor_row && content_width > 0 {
        let cursor_col = editor.cursor.col.min(line_len);
        let cursor_cell = widths.iter().take(cursor_col).copied().sum::<u16>();
        if cursor_cell >= content_width {
            cursor_cell.saturating_sub(content_width.saturating_sub(1))
        } else {
            0
        }
    } else {
        0
    };

    // 找到可视起点（按 grapheme 边界）
    let mut start_idx = 0usize;
    let mut cell_acc = 0u16;
    for (idx, w) in widths.iter().enumerate() {
        if cell_acc.saturating_add(*w) > scroll_cell {
            start_idx = idx;
            break;
        }
        cell_acc = cell_acc.saturating_add(*w);
        start_idx = idx.saturating_add(1);
    }

    // 将 content_x（cell）映射到 grapheme col
    let mut col = start_idx.min(line_len);
    let mut cell = 0u16;
    for idx in start_idx..line_len {
        let w = widths.get(idx).copied().unwrap_or(0);
        if cell.saturating_add(w) > content_x {
            col = idx;
            break;
        }
        cell = cell.saturating_add(w);
        col = idx.saturating_add(1);
    }

    crate::state::TextPos { row, col }
}

fn hit_test_targets_chip(
    instance_order: &[ralph_proto::HatInstanceId],
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> Option<usize> {
    if !contains_point(area, x, y) || area.width == 0 || area.height == 0 {
        return None;
    }

    // 说明：Targets 行渲染格式固定：
    // " Targets: @writer#1 @writer#2 ..."
    let rel_x = x.saturating_sub(area.x);
    let mut cursor_x: u16 = 0;

    // 前缀：" " + "Targets:" + " "
    cursor_x = cursor_x.saturating_add(1);
    cursor_x = cursor_x.saturating_add(UnicodeWidthStr::width("Targets:") as u16);
    cursor_x = cursor_x.saturating_add(1);

    for (idx, id) in instance_order.iter().enumerate() {
        let label = format!("@{id}");
        let w = UnicodeWidthStr::width(label.as_str()) as u16;
        let start = cursor_x;
        let end = cursor_x.saturating_add(w);

        if rel_x >= start && rel_x < end {
            return Some(idx);
        }

        cursor_x = end.saturating_add(1);
        if cursor_x >= area.width {
            break;
        }
    }

    None
}

fn hit_test_gate_action_chip(
    area: ratatui::layout::Rect,
    x: u16,
    y: u16,
) -> Option<GateActionChip> {
    if !contains_point(area, x, y) || area.width == 0 || area.height == 0 {
        return None;
    }

    // 说明：Actions 行渲染格式固定：
    // " Actions: !approve !deny !resolve"
    let rel_x = x.saturating_sub(area.x);
    let mut cursor_x: u16 = 0;

    // 前缀：" " + "Actions:" + " "
    cursor_x = cursor_x.saturating_add(1);
    cursor_x = cursor_x.saturating_add(UnicodeWidthStr::width("Actions:") as u16);
    cursor_x = cursor_x.saturating_add(1);

    let items = [
        (GateActionChip::Approve, "!approve"),
        (GateActionChip::Deny, "!deny"),
        (GateActionChip::Resolve, "!resolve"),
    ];

    for (action, label) in items {
        let w = UnicodeWidthStr::width(label) as u16;
        let start = cursor_x;
        let end = cursor_x.saturating_add(w);
        if rel_x >= start && rel_x < end {
            return Some(action);
        }

        cursor_x = end.saturating_add(1);
        if cursor_x >= area.width {
            break;
        }
    }

    None
}

fn resolve_human_message_target_instance(
    explicit: Option<String>,
    selected_instance_id: Option<&ralph_proto::HatInstanceId>,
) -> Option<String> {
    // 规则：
    // - 若用户显式写了 @instance，则以显式 target 为准
    // - 否则默认定向到当前 selected_instance（避免意外 broadcast）
    explicit.or_else(|| selected_instance_id.map(|id| id.to_string()))
}

// =============================================================================
// Clipboard（复制/粘贴）支持
// =============================================================================

/// Clipboard 写入方式（用于 UI status 提示）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClipboardCopyMethod {
    /// macOS: pbcopy（写入系统剪贴板）
    Pbcopy,
    /// OSC52（写入终端剪贴板，适合远程/跨平台）
    Osc52,
}

/// Clipboard 写入结果（best-effort）。
#[derive(Debug, Clone, Copy)]
struct ClipboardCopyOutcome {
    method: ClipboardCopyMethod,
    truncated: bool,
}

fn truncate_utf8_to_max_bytes(s: &str, max_bytes: usize) -> (&str, bool) {
    if s.len() <= max_bytes {
        return (s, false);
    }

    // 说明：避免切在 UTF-8 字节序列中间导致 panic。
    let mut end = max_bytes.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end = end.saturating_sub(1);
    }

    (&s[..end], true)
}

#[cfg(target_os = "macos")]
fn copy_to_pbcopy(text: &str) -> Result<()> {
    use std::io::Write as _;
    use std::process::Stdio;

    let mut child = std::process::Command::new("pbcopy")
        .stdin(Stdio::piped())
        .spawn()
        .map_err(|e| anyhow::anyhow!("spawn pbcopy failed: {e}"))?;

    let mut stdin = child
        .stdin
        .take()
        .ok_or_else(|| anyhow::anyhow!("pbcopy stdin is not piped"))?;
    stdin
        .write_all(text.as_bytes())
        .map_err(|e| anyhow::anyhow!("write to pbcopy failed: {e}"))?;
    drop(stdin);

    let status = child
        .wait()
        .map_err(|e| anyhow::anyhow!("wait pbcopy failed: {e}"))?;
    if !status.success() {
        return Err(anyhow::anyhow!("pbcopy exited with status: {status}"));
    }

    Ok(())
}

fn copy_to_osc52(text: &str) -> Result<bool> {
    use base64::Engine;
    use std::io::Write as _;

    // 经验值：OSC52 在不同终端/代理链路里可能有长度限制。
    // 这里做一个保守上限，避免一次性写入过大导致不可预期行为。
    const MAX_OSC52_BYTES: usize = 100_000;
    let (text, truncated) = truncate_utf8_to_max_bytes(text, MAX_OSC52_BYTES);

    let encoded = base64::engine::general_purpose::STANDARD.encode(text.as_bytes());
    // OSC52：设置剪贴板（c = clipboard）
    // 参考形式：ESC ] 52 ; c ; <base64> BEL
    let seq = format!("\x1b]52;c;{encoded}\x07");

    let mut stdout = io::stdout();
    stdout
        .write_all(seq.as_bytes())
        .map_err(|e| anyhow::anyhow!("write osc52 failed: {e}"))?;
    stdout
        .flush()
        .map_err(|e| anyhow::anyhow!("flush osc52 failed: {e}"))?;

    Ok(truncated)
}

fn copy_text_to_clipboard(text: &str) -> Result<ClipboardCopyOutcome> {
    // best-effort：优先用系统剪贴板（macOS），否则回退到 OSC52。
    #[cfg(target_os = "macos")]
    {
        if copy_to_pbcopy(text).is_ok() {
            return Ok(ClipboardCopyOutcome {
                method: ClipboardCopyMethod::Pbcopy,
                truncated: false,
            });
        }
    }

    let truncated = copy_to_osc52(text)?;
    Ok(ClipboardCopyOutcome {
        method: ClipboardCopyMethod::Osc52,
        truncated,
    })
}

fn extract_output_selection_text(
    buffer: crate::state::CurrentOutputBuffer<'_>,
    width: u16,
    height: u16,
    selection: crate::state::ScreenSelection,
    search_query: Option<&str>,
) -> String {
    if width == 0 || height == 0 {
        return String::new();
    }

    // 说明：selection 坐标是相对 output_inner（0,0 起）的屏幕坐标。
    let (min_x, max_x, min_y, max_y) = selection.bounds();
    let max_x = max_x.min(width.saturating_sub(1));
    let max_y = max_y.min(height.saturating_sub(1));
    let min_x = min_x.min(max_x);
    let min_y = min_y.min(max_y);

    let area = ratatui::layout::Rect::new(0, 0, width, height);
    let mut scratch = ratatui::buffer::Buffer::empty(area);

    // 复用实际渲染器，保证“所见即所得”（含 soft wrap / scroll offset）。
    match buffer {
        crate::state::CurrentOutputBuffer::Serial(buffer) => {
            let mut widget = ContentPane::new(buffer, TuiTheme::default());
            if let Some(q) = search_query {
                widget = widget.with_search(q);
            }
            ratatui::widgets::Widget::render(widget, area, &mut scratch);
        }
        crate::state::CurrentOutputBuffer::Parallel(buffer) => {
            let mut widget = ParallelOutputPane::new(buffer);
            if let Some(q) = search_query {
                widget = widget.with_search(q);
            }
            ratatui::widgets::Widget::render(widget, area, &mut scratch);
        }
    }

    // 提取选中矩形区域的字符（并对每行做一次右侧裁剪，减少粘贴噪音）。
    let mut lines: Vec<String> = Vec::new();
    for y in min_y..=max_y {
        let mut row = String::new();
        for x in min_x..=max_x {
            row.push_str(scratch[(x, y)].symbol());
        }
        let trimmed = row.trim_end_matches(' ').to_string();
        lines.push(trimmed);
    }

    lines.join("\n")
}

fn handle_parallel_mouse_down(
    mouse: MouseEvent,
    state: &mut TuiState,
    layout: ParallelLayoutSnapshot,
    chat_drag_anchor: &mut Option<crate::state::TextPos>,
) {
    let x = mouse.column;
    let y = mouse.row;

    // 只处理左键点击（其余按钮先忽略）。
    if !matches!(mouse.kind, MouseEventKind::Down(MouseButton::Left)) {
        return;
    }

    // 1) 点击实例列表：切换选中实例，并把焦点切回 Instances（与“列表可操作”的心智一致）
    if contains_point(layout.instances_inner, x, y) {
        let rel_y = y.saturating_sub(layout.instances_inner.y) as usize;
        let max = state.parallel.instance_order.len();
        if max > 0 {
            state.parallel.selected_instance = clamp_usize(rel_y, max);
            state.parallel.focus = ParallelFocus::Instances;
            state.parallel.clear_output_selection();
        }
        *chat_drag_anchor = None;
        return;
    }

    // 2) 点击 Chat 面板：进入输入态（后续会在输入区内支持鼠标定位光标/框选）
    if contains_point(layout.chat_input_area, x, y) {
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();

        let pos = hit_test_chat_editor(&state.parallel.chat_editor, layout.chat_input_area, x, y);
        state.parallel.chat_editor.set_cursor(pos, false);
        *chat_drag_anchor = Some(pos);
        return;
    }

    // 3) 点击 Targets chips：切换“默认目标实例”（保持在 Chat 焦点，便于继续输入）。
    if let Some(idx) = hit_test_targets_chip(
        &state.parallel.instance_order,
        layout.chat_targets_area,
        x,
        y,
    ) {
        state.parallel.selected_instance = idx;
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.clear_output_selection();
        *chat_drag_anchor = None;
        return;
    }

    // 4) 点击 Gate actions chips：预填输入框（不自动发送）。
    if let Some(action) = hit_test_gate_action_chip(layout.gate_actions_area, x, y) {
        let Some(gate_id) = state.parallel.selected_gate.clone() else {
            *chat_drag_anchor = None;
            return;
        };

        let prefill = match action {
            GateActionChip::Approve => format!("!approve {gate_id}"),
            GateActionChip::Deny => format!("!deny {gate_id}"),
            // 注意：末尾保留一个空格，方便继续输入 resolve 文本。
            GateActionChip::Resolve => format!("!resolve {gate_id} "),
        };

        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();
        state.parallel.chat_editor.clear();
        for ch in prefill.chars() {
            if ch == '\n' {
                state.parallel.chat_editor.insert_newline();
            } else {
                state.parallel.chat_editor.insert_char(ch);
            }
        }
        *chat_drag_anchor = None;
        return;
    }

    // 5) 点击 gate 列表行：选中 gate，并联动切换 selected_instance=requested_by。
    if contains_point(layout.gate_list_area, x, y) {
        let rel_y = y.saturating_sub(layout.gate_list_area.y) as usize;
        let max_lines = layout.gate_list_area.height as usize;

        let mut line_idx = 0usize;
        for gate_id in state.parallel.gate_order.iter().rev() {
            if line_idx >= max_lines {
                break;
            }
            let Some(g) = state.parallel.gates.get(gate_id) else {
                continue;
            };

            if line_idx == rel_y {
                state.parallel.selected_gate = Some(gate_id.clone());
                let requested_by = g.request.requested_by.clone();
                let _ = state.parallel.select_instance_by_id(&requested_by);

                state.parallel.focus = ParallelFocus::Chat;
                state.parallel.finish_output_selection();
                *chat_drag_anchor = None;
                return;
            }

            line_idx += 1;
        }

        *chat_drag_anchor = None;
        return;
    }

    if contains_point(layout.bottom_inner, x, y) {
        state.parallel.focus = ParallelFocus::Chat;
        state.parallel.finish_output_selection();
        *chat_drag_anchor = None;
        return;
    }

    // 3) 点击 Output 面板：切换焦点到 Output（后续会在输出区内支持拖拽框选）
    if contains_point(layout.output_inner, x, y) {
        let rel_x = x.saturating_sub(layout.output_inner.x);
        let rel_y = y.saturating_sub(layout.output_inner.y);
        state.parallel.focus = ParallelFocus::Output;
        state
            .parallel
            .start_output_selection(crate::state::ScreenPos { x: rel_x, y: rel_y });
        *chat_drag_anchor = None;
    }
}

/// Dispatches an action to the TuiState.
///
/// Returns `true` if the action signals to quit the application.
pub fn dispatch_action(action: Action, state: &mut TuiState, viewport_height: usize) -> bool {
    match action {
        Action::Quit => return true,
        Action::ScrollDown => {
            if let Some(mut buffer) = state.current_output_buffer_mut() {
                buffer.scroll_down(viewport_height);
            }
        }
        Action::ScrollUp => {
            if let Some(mut buffer) = state.current_output_buffer_mut() {
                buffer.scroll_up();
            }
        }
        Action::ScrollTop => {
            if let Some(mut buffer) = state.current_output_buffer_mut() {
                buffer.scroll_top();
            }
        }
        Action::ScrollBottom => {
            if let Some(mut buffer) = state.current_output_buffer_mut() {
                buffer.scroll_bottom(viewport_height);
            }
        }
        Action::NextIteration => match state.mode {
            TuiMode::Serial => state.navigate_next(),
            TuiMode::Parallel => {
                if state.parallel.focus == ParallelFocus::Output {
                    state.parallel.select_next_job();
                }
            }
        },
        Action::PrevIteration => match state.mode {
            TuiMode::Serial => state.navigate_prev(),
            TuiMode::Parallel => {
                if state.parallel.focus == ParallelFocus::Output {
                    state.parallel.select_prev_job();
                }
            }
        },
        Action::ShowHelp => {
            state.show_help = true;
        }
        Action::DismissHelp => {
            state.show_help = false;
            state.search_state.search_mode = false;
            state.search_query.clear();
            state.clear_search();
        }
        Action::StartSearch => {
            state.search_state.search_mode = true;
            state.search_query.clear();
        }
        Action::SearchNext => {
            state.next_match();
        }
        Action::SearchPrev => {
            state.prev_match();
        }
        Action::ToggleHatGraphZoom => {
            // 右上角 Hat Graph Radar：只改变 UI 尺寸，不影响任何 orchestration 行为。
            if state.hat_graph_radar.is_some() {
                state.hat_graph_zoomed = !state.hat_graph_zoomed;
            }
        }
        Action::None => {}
    }
    false
}

/// Main TUI application for read-only observation.
pub struct App {
    state: Arc<Mutex<TuiState>>,
    /// Receives notification when the underlying process terminates.
    /// This is the ONLY exit path for the TUI event loop (besides Action::Quit).
    terminated_rx: watch::Receiver<bool>,
    /// Channel to signal main loop on Ctrl+C.
    /// In raw terminal mode, SIGINT is not generated, so TUI must signal
    /// the main orchestration loop through this channel.
    interrupt_tx: Option<watch::Sender<bool>>,

    /// 并行模式：UI 更新通道（observer → channel → reducer）。
    update_rx: Option<mpsc::UnboundedReceiver<TuiUpdate>>,
}

impl App {
    /// Creates a new App with shared state, termination signal, and optional interrupt channel.
    pub fn new(
        state: Arc<Mutex<TuiState>>,
        terminated_rx: watch::Receiver<bool>,
        interrupt_tx: Option<watch::Sender<bool>>,
        update_rx: Option<mpsc::UnboundedReceiver<TuiUpdate>>,
    ) -> Self {
        Self {
            state,
            terminated_rx,
            interrupt_tx,
            update_rx,
        }
    }

    /// Runs the TUI event loop.
    pub async fn run(mut self) -> Result<()> {
        // 默认主题（Catppuccin Mocha）。
        //
        // Warp 特殊处理：
        // - 用户希望保留 Warp 的半透明窗口背景效果（包括 UI 范围外的 padding）。
        // - 因此在 Warp + TTY 下使用终端默认背景（app bg=Reset），让“pane 之外”与 padding 共享背景。
        // - 同时 pane 内部仍使用主题底色（base），让文字更稳、更不刺眼。
        let theme = if std::io::stdout().is_terminal() && is_warp_terminal() {
            TuiTheme::default().with_terminal_default_bg()
        } else {
            TuiTheme::default()
        };

        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;

        // 为了让终端能区分 `Enter` 与 `Shift+Enter`，这里启用 kitty keyboard protocol
        // （crossterm: progressive keyboard enhancement）。
        //
        // 背景：
        // - 许多终端在默认模式下不会把 `Shift+Enter` 作为“带 SHIFT 的 Enter”上报；
        // - 结果就是应用层只能看到一次普通 Enter，无法实现 “Shift+Enter=换行” 的体验。
        //
        // 处理策略：
        // - best-effort 启用：失败也不影响 TUI 运行（例如不支持的终端/平台）。
        let _ = execute!(
            stdout,
            crossterm::event::PushKeyboardEnhancementFlags(
                crossterm::event::KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES
            )
        );

        // CRITICAL: Ensure terminal cleanup on ANY exit path (normal, abort, or panic).
        // When cleanup_tui() calls handle.abort(), the task is cancelled immediately
        // at its current await point, skipping all code after the loop. This defer!
        // guard runs on Drop, which is guaranteed even during task cancellation.
        defer! {
            let _ = disable_raw_mode();
            let _ = execute!(
                io::stdout(),
                crossterm::event::PopKeyboardEnhancementFlags,
                LeaveAlternateScreen,
                DisableMouseCapture,
                Show
            );
        }

        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        // Event-driven architecture: input polling is the primary driver
        // Render is throttled to ~60fps via interval tick
        let mut events = EventStream::new();
        let mut render_tick = interval(Duration::from_millis(16));

        // Track viewport height for scroll calculations
        let mut viewport_height: usize = 24; // Default, updated on render

        // 并行模式：底部控制面板（chat/gates）的固定高度。
        // 说明：先用固定高度满足“多行输入 + gate 列表可见”的最小需求，后续可再做自适应。
        const PARALLEL_BOTTOM_PANEL_HEIGHT: u16 = 12;
        const PARALLEL_CHAT_INPUT_HEIGHT: u16 = 3;
        // 并行模式：左侧 Instances 面板的固定宽度（保持稳定布局，避免随窗口伸缩导致跳动）。
        const PARALLEL_INSTANCES_PANEL_WIDTH: u16 = 30;
        // 并行模式：Instances 与 Output 之间留一列间隙。
        // 目的：取消“边框贴合/看起来像 collapsing borders”的观感，让两个 pane 更清晰分离。
        const PARALLEL_PANE_GAP_WIDTH: u16 = 1;

        // 并行模式的 state 更新通道（由 App 消费）
        let mut update_rx = self.update_rx.take();

        // 并行模式：保存“最近一次渲染”的布局快照，用于鼠标 hit-test
        let mut parallel_layout: Option<ParallelLayoutSnapshot> = None;
        // 并行模式：Chat 区域的鼠标拖拽选择锚点（Down→Drag→Up）。
        let mut chat_drag_anchor: Option<crate::state::TextPos> = None;

        // =========================================================================
        // 动画（启动打开动画）
        // =========================================================================
        //
        // 说明：
        // - 动画是“锦上添花”，必须保证随时可降级为无动画
        // - 使用 tachyonfx：先渲染 UI，再对 buffer 应用 effect（与 exabind 的实现范式一致）
        let animations_enabled = animation::animations_enabled();
        let mut effects = if animations_enabled {
            Some(EffectManager::<&'static str>::default())
        } else {
            None
        };
        let mut last_effect_tick = Instant::now();
        let mut startup_animation_attempted = false;
        let mut startup_animation_started_at: Option<Instant> = None;

        // 并行模式：用于检测“选中实例发生变化”，从而触发 Output 的重启动画。
        //
        // 说明：
        // - 这不是 state 的一部分（避免 reducer 污染），仅用于 UI 层的动画触发
        // - 只要 selected_instance_id 变了，就认为需要让 Output “重新打开”
        let mut last_selected_instance_id: Option<HatInstanceId> = None;

        loop {
            // Use biased select to prioritize input over render ticks
            tokio::select! {
                biased;

                // Priority 1: Handle input events immediately for responsiveness
                maybe_event = events.next() => {
                    match maybe_event {
                        Some(Ok(event)) => {
                            match event {
                                // Handle Ctrl+C: signal main loop and exit.
                                // In raw mode, SIGINT is not generated, so we must signal the
                                // main orchestration loop through interrupt_tx channel.
                                Event::Key(key) if key.kind == KeyEventKind::Press
                                    && key.code == KeyCode::Char('c')
                                    && key.modifiers.contains(KeyModifiers::CONTROL) =>
                                {
                                    info!("Ctrl+C detected, signaling main loop");
                                    request_interrupt(self.interrupt_tx.as_ref());
                                    break;
                                }
                                Event::Mouse(mouse) => {
                                    match mouse.kind {
                                        MouseEventKind::ScrollUp => {
                                            let mut state = self.state.lock().unwrap();
                                            if let Some(mut buffer) = state.current_output_buffer_mut() {
                                                for _ in 0..3 {
                                                    buffer.scroll_up();
                                                }
                                            }
                                        }
                                        MouseEventKind::ScrollDown => {
                                            let mut state = self.state.lock().unwrap();
                                            if let Some(mut buffer) = state.current_output_buffer_mut() {
                                                for _ in 0..3 {
                                                    buffer.scroll_down(viewport_height);
                                                }
                                            }
                                        }
                                        MouseEventKind::Down(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                handle_parallel_mouse_down(
                                                    mouse,
                                                    &mut state,
                                                    layout,
                                                    &mut chat_drag_anchor,
                                                );
                                            }
                                        }
                                        // 说明：Drag/Up 的选择逻辑会在后续任务里补齐（输出框选 / chat 框选）。
                                        // 这里先做结构化分发，避免未来再大改输入循环。
                                        MouseEventKind::Drag(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                let x = mouse.column;
                                                let y = mouse.row;

                                                // Output：拖拽更新选择区域（屏幕坐标）。
                                                if state.parallel.output_selecting {
                                                    let clamped_x = clamp_to_area(
                                                        x,
                                                        layout.output_inner.x,
                                                        layout.output_inner.width,
                                                    );
                                                    let clamped_y = clamp_to_area(
                                                        y,
                                                        layout.output_inner.y,
                                                        layout.output_inner.height,
                                                    );
                                                    let rel_x = clamped_x.saturating_sub(layout.output_inner.x);
                                                    let rel_y = clamped_y.saturating_sub(layout.output_inner.y);
                                                    state.parallel.focus = ParallelFocus::Output;
                                                    state.parallel.update_output_selection_cursor(crate::state::ScreenPos {
                                                        x: rel_x,
                                                        y: rel_y,
                                                    });
                                                    continue;
                                                }

                                                // Chat：拖拽更新线性选择（TextPos）。
                                                if let Some(anchor) = chat_drag_anchor {
                                                    let clamped_x = clamp_to_area(
                                                        x,
                                                        layout.chat_input_area.x,
                                                        layout.chat_input_area.width,
                                                    );
                                                    let clamped_y = clamp_to_area(
                                                        y,
                                                        layout.chat_input_area.y,
                                                        layout.chat_input_area.height,
                                                    );
                                                    let pos = hit_test_chat_editor(
                                                        &state.parallel.chat_editor,
                                                        layout.chat_input_area,
                                                        clamped_x,
                                                        clamped_y,
                                                    );
                                                    state.parallel.focus = ParallelFocus::Chat;
                                                    state.parallel.chat_editor.set_mouse_selection(anchor, pos);
                                                    continue;
                                                }

                                                // 兜底：拖拽落在哪个区域，就把焦点切过去（后续 chat 框选会复用）。
                                                if contains_point(layout.output_inner, x, y) {
                                                    state.parallel.focus = ParallelFocus::Output;
                                                } else if contains_point(layout.bottom_inner, x, y) {
                                                    state.parallel.focus = ParallelFocus::Chat;
                                                }
                                            }
                                        }
                                        MouseEventKind::Up(MouseButton::Left) => {
                                            let mut state = self.state.lock().unwrap();
                                            if matches!(state.mode, TuiMode::Parallel)
                                                && let Some(layout) = parallel_layout
                                            {
                                                // Output：结束拖拽选择。
                                                if state.parallel.output_selecting {
                                                    state.parallel.finish_output_selection();

                                                    // 鼠标框选结束后：自动复制到剪贴板（best-effort）。
                                                    // 说明：在 raw mode + mouse capture 下，终端原生选择通常不可用；
                                                    // 因此这里把“应用内选择”主动写入剪贴板，才能形成 Cmd+C/Cmd+V 闭环。
                                                    if let Some(sel) = state.parallel.output_selection
                                                        && let Some(buffer) = state.current_output_buffer()
                                                    {
                                                        let selected_text = extract_output_selection_text(
                                                            buffer,
                                                            layout.output_inner.width,
                                                            layout.output_inner.height,
                                                            sel,
                                                            state.search_state.query.as_deref(),
                                                        );

                                                        if selected_text.trim().is_empty() {
                                                            state.parallel.chat_status =
                                                                Some("copy: no text selected".to_string());
                                                        } else {
                                                            match copy_text_to_clipboard(&selected_text) {
                                                                Ok(outcome) => {
                                                                    let method = match outcome.method {
                                                                        ClipboardCopyMethod::Pbcopy => "pbcopy",
                                                                        ClipboardCopyMethod::Osc52 => "osc52",
                                                                    };
                                                                    let truncated = if outcome.truncated {
                                                                        " (truncated)"
                                                                    } else {
                                                                        ""
                                                                    };
                                                                    state.parallel.chat_status = Some(format!(
                                                                        "copied {} chars to clipboard via {method}{truncated}",
                                                                        selected_text.chars().count()
                                                                    ));
                                                                }
                                                                Err(e) => {
                                                                    state.parallel.chat_status =
                                                                        Some(format!("copy failed: {e:#}"));
                                                                }
                                                            }
                                                        }
                                                    }
                                                }

                                                // Chat：结束拖拽选择（保留 selection 结果，仅清理锚点）。
                                                chat_drag_anchor = None;
                                            }
                                        }
                                        _ => {}
                                    }
                                }
                                Event::Paste(paste) => {
                                    // 说明：
                                    // - 部分终端会使用 bracketed paste，上报为 `Event::Paste(text)`。
                                    // - 如果忽略该事件，用户会感知为“Cmd+V 没反应”。
                                    let mut state = self.state.lock().unwrap();

                                    // Paste 也视为“用户有意输入”，因此顺手关闭 help overlay，避免挡住视线。
                                    if state.show_help {
                                        state.show_help = false;
                                    }

                                    // 搜索输入模式：追加到 query（并压平换行，保持单行语义）
                                    if state.search_state.search_mode {
                                        let normalized = paste
                                            .replace('\r', "")
                                            .replace('\n', " ")
                                            .trim_end()
                                            .to_string();
                                        state.search_query.push_str(&normalized);
                                        continue;
                                    }

                                    // 并行模式：仅在 Chat 聚焦时接收粘贴内容（避免误写到其它区域）
                                    if matches!(state.mode, TuiMode::Parallel)
                                        && state.parallel.focus == ParallelFocus::Chat
                                    {
                                        for ch in paste.chars() {
                                            match ch {
                                                '\r' => {}
                                                '\n' => state.parallel.chat_editor.insert_newline(),
                                                c => state.parallel.chat_editor.insert_char(c),
                                            }
                                        }
                                    }
                                }
                                Event::Key(key) if key.kind == KeyEventKind::Press => {
                                    // Dismiss help on any key when help is showing
                                    {
                                        let mut state = self.state.lock().unwrap();
                                        if state.show_help {
                                            state.show_help = false;
                                            continue;
                                        }
                                    }

                                    // 串行/并行：按 mode 分流输入处理
                                    let mut state = self.state.lock().unwrap();

                                    // 搜索输入模式（串行/并行共用）。
                                    if state.search_state.search_mode {
                                        match key.code {
                                            KeyCode::Esc => {
                                                state.search_state.search_mode = false;
                                                state.search_query.clear();
                                                state.clear_search();
                                            }
                                            KeyCode::Backspace => {
                                                state.search_query.pop();
                                            }
                                            KeyCode::Enter => {
                                                let query = state.search_query.trim().to_string();
                                                state.search_state.search_mode = false;
                                                state.search_query.clear();

                                                if query.is_empty() {
                                                    state.clear_search();
                                                } else {
                                                    state.search(&query);
                                                }
                                            }
                                            KeyCode::Char(c) => {
                                                state.search_query.push(c);
                                            }
                                            _ => {}
                                        }
                                        continue;
                                    }

                                    match state.mode {
                                        TuiMode::Serial => {
                                            let action = map_key(key);
                                            if action == Action::Quit {
                                                // 说明：
                                                // - 对用户来说，退出 TUI 等价于“我不再需要这个 run 继续执行”。
                                                // - 在 raw mode 下，TUI 是唯一能可靠捕获用户退出意图的地方。
                                                // 因此这里复用 interrupt_tx 通道，触发主循环走统一的 shutdown 清理路径。
                                                request_interrupt(self.interrupt_tx.as_ref());
                                                break;
                                            }
                                            if dispatch_action(action, &mut state, viewport_height) {
                                                break;
                                            }
                                        }
                                        TuiMode::Parallel => {
                                            // 3.x：并行模式的输入映射（焦点/导航/滚动/搜索）。
                                            // 5.x/6.x：chat/gate 交互（写外部事件 + 展示 gate 列表）。

                                            // Focus switching first (Tab / BackTab)
                                            if key.code == KeyCode::Tab {
                                                state.parallel.focus_next();
                                                continue;
                                            }
                                            if key.code == KeyCode::BackTab {
                                                state.parallel.focus_prev();
                                                continue;
                                            }

                                            // Global keys（注意：Chat 焦点下字符应当进入输入框，不应触发 quit/help）
                                            let focus = state.parallel.focus;
                                            if focus != ParallelFocus::Chat {
                                                if key.code == KeyCode::Char('p') {
                                                    // 并行模式：在非 Chat 输入场景下，`p` 用于切换右上角 Hat Graph Radar 的放大/还原。
                                                    if state.hat_graph_radar.is_some() {
                                                        state.hat_graph_zoomed =
                                                            !state.hat_graph_zoomed;
                                                    }
                                                    continue;
                                                }
                                                if key.code == KeyCode::Char('q') {
                                                    // 并行模式：退出 TUI 时必须退出所有 worker CLI 子进程。
                                                    // 复用 interrupt_tx，让并行 runner 走 killpg(SIGTERM→SIGKILL) 的统一清理路径。
                                                    request_interrupt(self.interrupt_tx.as_ref());
                                                    break;
                                                }
                                                if key.code == KeyCode::Char('y') {
                                                    // `y`：复制当前输出选择到剪贴板（best-effort）。
                                                    // 说明：Cmd+C 是终端模拟器快捷键，通常不会被 TUI 应用接收到；
                                                    // 因此这里提供一个应用内显式复制键作为兜底。
                                                    let Some(layout) = parallel_layout else {
                                                        state.parallel.chat_status =
                                                            Some("copy failed: layout not ready".to_string());
                                                        continue;
                                                    };

                                                    let Some(sel) = state.parallel.output_selection else {
                                                        state.parallel.chat_status =
                                                            Some("copy: no selection".to_string());
                                                        continue;
                                                    };

                                                    let Some(buffer) = state.current_output_buffer() else {
                                                        state.parallel.chat_status =
                                                            Some("copy failed: no output buffer".to_string());
                                                        continue;
                                                    };

                                                    let selected_text = extract_output_selection_text(
                                                        buffer,
                                                        layout.output_inner.width,
                                                        layout.output_inner.height,
                                                        sel,
                                                        state.search_state.query.as_deref(),
                                                    );

                                                    if selected_text.trim().is_empty() {
                                                        state.parallel.chat_status =
                                                            Some("copy: no text selected".to_string());
                                                        continue;
                                                    }

                                                    match copy_text_to_clipboard(&selected_text) {
                                                        Ok(outcome) => {
                                                            let method = match outcome.method {
                                                                ClipboardCopyMethod::Pbcopy => "pbcopy",
                                                                ClipboardCopyMethod::Osc52 => "osc52",
                                                            };
                                                            let truncated = if outcome.truncated {
                                                                " (truncated)"
                                                            } else {
                                                                ""
                                                            };
                                                            state.parallel.chat_status = Some(format!(
                                                                "copied {} chars to clipboard via {method}{truncated}",
                                                                selected_text.chars().count()
                                                            ));
                                                        }
                                                        Err(e) => {
                                                            state.parallel.chat_status =
                                                                Some(format!("copy failed: {e:#}"));
                                                        }
                                                    }

                                                    continue;
                                                }
                                                if key.code == KeyCode::Char('?') {
                                                    state.show_help = true;
                                                    continue;
                                                }
                                            }

                                            match focus {
                                                ParallelFocus::Instances => match key.code {
                                                    KeyCode::Up | KeyCode::Char('k') => {
                                                        state.parallel.select_prev_instance();
                                                    }
                                                    KeyCode::Down | KeyCode::Char('j') => {
                                                        state.parallel.select_next_instance();
                                                    }
                                                    KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter => {
                                                        state.parallel.focus = ParallelFocus::Output;
                                                    }
                                                    _ => {}
                                                },
                                                ParallelFocus::Output => {
                                                    // Esc：清空输出选择（避免与 search-mode 的 Esc 冲突：
                                                    // search-mode 已在上方分支提前 continue 处理）。
                                                    if key.code == KeyCode::Esc {
                                                        state.parallel.clear_output_selection();
                                                        continue;
                                                    }

                                                    // Shift+方向键：扩展输出选择（最小可用键盘选择）。
                                                    if key.modifiers.contains(KeyModifiers::SHIFT) {
                                                        let (dx, dy) = match key.code {
                                                            KeyCode::Left => (-1, 0),
                                                            KeyCode::Right => (1, 0),
                                                            KeyCode::Up => (0, -1),
                                                            KeyCode::Down => (0, 1),
                                                            _ => (0, 0),
                                                        };

                                                        if (dx, dy) != (0, 0)
                                                            && let Some(layout) = parallel_layout
                                                        {
                                                            state.parallel.focus = ParallelFocus::Output;
                                                            state.parallel.extend_output_selection_by_delta(
                                                                dx,
                                                                dy,
                                                                layout.output_inner.width,
                                                                layout.output_inner.height,
                                                            );
                                                            continue;
                                                        }
                                                    }

                                                    let action = map_key(key);
                                                    if dispatch_action(action, &mut state, viewport_height) {
                                                        break;
                                                    }
                                                }
                                                ParallelFocus::Chat => {
                                                    match key.code {
                                                        KeyCode::Esc => {
                                                            // Esc：优先清空选择；若没有选择，则清空输入内容。
                                                            if state.parallel.chat_editor.has_selection() {
                                                                state.parallel.chat_editor.clear_selection();
                                                            } else {
                                                                state.parallel.chat_editor.clear();
                                                            }
                                                        }
                                                        KeyCode::Backspace => {
                                                            state.parallel.chat_editor.backspace();
                                                        }
                                                        KeyCode::Delete => {
                                                            state.parallel.chat_editor.delete();
                                                        }
                                                        KeyCode::Char('j')
                                                            if key.modifiers.contains(KeyModifiers::CONTROL) =>
                                                        {
                                                            // Ctrl+J：换行（在很多终端里比 Shift+Enter 更稳定可区分）。
                                                            state.parallel.chat_editor.insert_newline();
                                                        }
                                                        KeyCode::Enter => {
                                                            // 说明：
                                                            // - 理想情况：Shift+Enter 可被终端区分，用于换行。
                                                            // - 现实情况：部分终端不会把 Shift 修饰符上报给 TUI（Shift+Enter 看起来就像 Enter）。
                                                            // 因此这里保留 Shift+Enter，同时提供 Alt+Enter 作为更可靠的 fallback。
                                                            if key.modifiers.contains(KeyModifiers::SHIFT)
                                                                || key.modifiers.contains(KeyModifiers::ALT)
                                                            {
                                                                state.parallel.chat_editor.insert_newline();
                                                                continue;
                                                            }

                                                            let raw = state.parallel.chat_editor.text();
                                                            state.parallel.chat_editor.clear();
                                                            if raw.trim().is_empty() {
                                                                continue;
                                                            }

                                                            match parse_chat_submit(&raw) {
                                                                Ok(ChatSubmit::HumanMessage { target_instance, payload }) => {
                                                                    // 默认消息（不写 @...）需要定向到当前选中实例，
                                                                    // 避免 human.message 在并行模式下“意外广播”。
                                                                    let resolved_target = resolve_human_message_target_instance(
                                                                        target_instance,
                                                                        state.parallel.selected_instance_id(),
                                                                    );
                                                                    if resolved_target.is_none() {
                                                                        state.parallel.chat_status =
                                                                            Some("send failed: no instance selected".to_string());
                                                                        continue;
                                                                    }
                                                                    let writer = ExternalEventWriter::new();
                                                                    match writer.append("human.message", payload, resolved_target) {
                                                                        Ok(()) => {
                                                                            state.parallel.chat_status = Some(format!(
                                                                                "sent human.message -> {}",
                                                                                writer.path().display()
                                                                            ));
                                                                        }
                                                                        Err(e) => {
                                                                            state.parallel.chat_status = Some(format!("send failed: {e:#}"));
                                                                        }
                                                                    }
                                                                }
                                                                Ok(ChatSubmit::GateResolve { gate_id, decision }) => {
                                                                    let requested_by = state
                                                                        .parallel
                                                                        .gates
                                                                        .get(&gate_id)
                                                                        .map(|g| g.request.requested_by.clone());

                                                                    let resolve = GateResolve {
                                                                        gate_id,
                                                                        resolved_by: GateResolvedBy::Human,
                                                                        decision,
                                                                        requested_by,
                                                                    };

                                                                    match serde_json::to_string(&resolve) {
                                                                        Ok(payload) => {
                                                                            let writer = ExternalEventWriter::new();
                                                                            match writer.append(TOPIC_GATE_RESOLVE, payload, None) {
                                                                                Ok(()) => {
                                                                                    state.parallel.chat_status = Some(format!(
                                                                                        "sent gate.resolve -> {}",
                                                                                        writer.path().display()
                                                                                    ));
                                                                                }
                                                                                Err(e) => {
                                                                                    state.parallel.chat_status = Some(format!("send failed: {e:#}"));
                                                                                }
                                                                            }
                                                                        }
                                                                        Err(e) => {
                                                                            state.parallel.chat_status = Some(format!("serialize failed: {e}"));
                                                                        }
                                                                    }
                                                                }
                                                                Err(e) => {
                                                                    state.parallel.chat_status = Some(format!("parse error: {e}"));
                                                                }
                                                            }
                                                        }
                                                        KeyCode::Left => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_left(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Right => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_right(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Up => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_up(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Down => {
                                                            state
                                                                .parallel
                                                                .chat_editor
                                                                .move_down(key.modifiers.contains(KeyModifiers::SHIFT));
                                                        }
                                                        KeyCode::Char(c) => {
                                                            state.parallel.chat_editor.insert_char(c);
                                                        }
                                                        _ => {}
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                                // Ignore other events (FocusGained, FocusLost, Resize, key releases)
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            // Log error but continue - transient errors shouldn't crash TUI
                            tracing::warn!("Event stream error: {}", e);
                        }
                        None => {
                            // Stream ended unexpectedly
                            break;
                        }
                    }
                }

                    // Priority 2: Render at throttled rate (~60fps)
                    _ = render_tick.tick() => {
                        let frame_size = terminal.size()?;
                        let frame_area = ratatui::layout::Rect::new(0, 0, frame_size.width, frame_size.height);

                        // 动画 delta：只在启用动画时推进 EffectManager 的时间轴。
                        let mut fx_delta = if effects.is_some() {
                            let elapsed = last_effect_tick.elapsed();
                            last_effect_tick = Instant::now();
                            FxDuration::from_millis(elapsed.as_millis().min(u128::from(u32::MAX)) as u32)
                        } else {
                            FxDuration::from_millis(0)
                        };

                    let chunks = Layout::default()
                        .direction(Direction::Vertical)
                        .constraints([
                            Constraint::Length(2),  // Header: content + bottom border
                            Constraint::Min(0),     // Content: flexible
                            Constraint::Length(2),  // Footer: top border + content
                        ])
                        .split(frame_area);

                        let content_area = chunks[1];
                        // viewport_height 代表“当前可滚动输出视图”的高度（串行=content， 并行=output inner）
                        // 注意：并行模式下 output 还有边框与底部 chat panel，因此需要做一次保守估计。
                        viewport_height = content_area.height as usize;

                        let mut state = self.state.lock().unwrap();

                        // 启动打开动画只尝试一次：进入 alternate screen 的第一帧。
                        //
                        // 说明：
                        // - 串行模式：允许用全屏 sweep 作为轻量“打开”
                        // - 并行模式：按需求做“逐块出场”，并且 Instances 条目必须晚于框体
                        if !startup_animation_attempted {
                            startup_animation_attempted = true;
                            if let Some(manager) = effects.as_mut()
                                && animation::should_run_startup_animation(frame_area)
                            {
                                let effect = match state.mode {
                                    TuiMode::Parallel => {
                                        let vertical = Layout::default()
                                            .direction(Direction::Vertical)
                                            .constraints([
                                                Constraint::Min(0), // main
                                                Constraint::Length(PARALLEL_BOTTOM_PANEL_HEIGHT), // bottom panel
                                            ])
                                            .split(content_area);

                                        let main_area = vertical[0];
                                        let bottom_area = vertical[1];

                                        let horizontal = Layout::default()
                                            .direction(Direction::Horizontal)
                                            .constraints([
                                                Constraint::Length(PARALLEL_INSTANCES_PANEL_WIDTH), // instances
                                                Constraint::Length(PARALLEL_PANE_GAP_WIDTH),        // gap
                                                Constraint::Min(0),                                  // output
                                            ])
                                            .split(main_area);

                                        let instances_area = horizontal[0];
                                        let output_area = horizontal[2];

                                        let instances_inner =
                                            panel_block("Instances", false, &theme).inner(instances_area);

                                        animation::startup_open_effect_parallel(
                                            theme,
                                            chunks[0],
                                            instances_area,
                                            instances_inner,
                                            output_area,
                                            bottom_area,
                                            chunks[2],
                                        )
                                    }
                                    TuiMode::Serial => animation::startup_open_effect(theme, frame_area),
                                };

                                manager.add_unique_effect(animation::STARTUP_ANIMATION_KEY, effect);
                                startup_animation_started_at = Some(Instant::now());

                                // 关键修复：
                                // - tachyonfx 的 Shader 默认实现是“先推进 timer，再 execute”
                                // - 如果首帧渲染被输入事件拖慢，fx_delta 可能很大
                                //   → 启动动画会从中途开始，导致“先全显示，再扫一遍”的闪烁观感
                                // - 因此：启动动画被添加的这一帧，强制用 0 delta 先渲染一次“全隐藏起步态”
                                prime_animation_first_frame(&mut fx_delta, &mut last_effect_tick);
                            }
                        }

                        // Output 重启动画：当选中实例变化时，让 Output “先消失再打开”。
                        //
                        // 关键规则：
                        // - 启动出场期间不触发（避免和 pane-level startup 编排打架）
                        // - reduced-motion / 小窗口下不触发（可用性优先）
                        if matches!(state.mode, TuiMode::Parallel) {
                            let current_selected = state.parallel.selected_instance_id().cloned();

                            if last_selected_instance_id.is_none() {
                                // 第一次记录：避免在启动阶段或首次渲染时误触发重启动画。
                                last_selected_instance_id = current_selected;
                            } else if last_selected_instance_id != current_selected {
                                last_selected_instance_id = current_selected;

                                let startup_done = match startup_animation_started_at {
                                    None => true,
                                    Some(t) => t.elapsed()
                                        >= Duration::from_millis(u64::from(animation::STARTUP_TOTAL_MS)),
                                };

                                if startup_done
                                    && animation::should_run_startup_animation(frame_area)
                                    && let Some(manager) = effects.as_mut()
                                {
                                    let vertical = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Min(0), // main
                                            Constraint::Length(PARALLEL_BOTTOM_PANEL_HEIGHT), // bottom panel
                                        ])
                                        .split(content_area);

                                    let main_area = vertical[0];
                                    let horizontal = Layout::default()
                                        .direction(Direction::Horizontal)
                                        .constraints([
                                            Constraint::Length(PARALLEL_INSTANCES_PANEL_WIDTH), // instances
                                            Constraint::Length(PARALLEL_PANE_GAP_WIDTH),        // gap
                                            Constraint::Min(0),                                  // output
                                        ])
                                        .split(main_area);

                                    let output_area = horizontal[2];
                                    let output_inner = inner_block(output_area);
                                    // 体验取舍：
                                    // - Warp（bg=Reset）下，为了避免边框 cell 参与插值导致“外圈被带色”，
                                    //   Output 重启动画只作用于 inner。
                                    // - 非 Warp 下，允许动画包含边框，能看到更清晰的“边框过渡”。
                                    let output_effect_area = if theme.app_bg_color() == Color::Reset {
                                        output_inner
                                    } else {
                                        output_area
                                    };

                                    if output_effect_area.width > 0 && output_effect_area.height > 0 {
                                        manager.add_unique_effect(
                                            animation::OUTPUT_REOPEN_ANIMATION_KEY,
                                            animation::output_reopen_effect(theme, output_effect_area),
                                        );

                                        // 关键修复：
                                        // - Output 重启动画必须从“隐藏态”首帧起步，
                                        //   否则会出现“先显示新内容一帧再消失”的闪烁。
                                        prime_animation_first_frame(
                                            &mut fx_delta,
                                            &mut last_effect_tick,
                                        );
                                    }
                                }
                            }
                        }

                        // Autoscroll（串行/并行）：如果用户没离开底部，就跟随输出
                        let effective_viewport_height = match state.mode {
                            TuiMode::Serial => content_area.height as usize,
                        TuiMode::Parallel => {
                            // content = main + bottom_panel(PARALLEL_BOTTOM_PANEL_HEIGHT)
                            // output inner = main - borders(2)
                            let main_height = content_area
                                .height
                                .saturating_sub(PARALLEL_BOTTOM_PANEL_HEIGHT);
                            main_height.saturating_sub(2) as usize
                        }
                    };
                    if let Some(mut buffer) = state.current_output_buffer_mut()
                        && buffer.following_bottom()
                    {
                        let max_scroll = buffer
                            .row_count()
                            .saturating_sub(effective_viewport_height);
                        buffer.set_scroll_offset_clamped(max_scroll);
                    }

                    // 并行模式下：把“输出面板可用宽度”同步到 state，用于 Markdown 语义换行。
                    // 这样 blockquote/list 等结构前缀在换行后仍能保持正确展示。
                    if state.mode == TuiMode::Parallel {
                        let vertical = Layout::default()
                            .direction(Direction::Vertical)
                            .constraints([
                                Constraint::Min(0), // main
                                Constraint::Length(PARALLEL_BOTTOM_PANEL_HEIGHT), // bottom panel
                            ])
                            .split(content_area);

                        let main_area = vertical[0];
                        let horizontal = Layout::default()
                            .direction(Direction::Horizontal)
                            .constraints([
                                Constraint::Length(30), // instances
                                Constraint::Min(0),     // output
                            ])
                            .split(main_area);

                        let output_area = horizontal[1];
                        let output_inner_width = output_area.width.saturating_sub(2);
                        state.parallel.set_output_render_width(output_inner_width);
                    }

                    let state = state; // Rebind as immutable for rendering
                        terminal.draw(|f| {
                            // 说明：
                            // - exabind 风格边框需要对“边框 cell 的背景色”做二次修正（见 theme.rs）。
                            // - 但动画是后处理（effects 在最后执行），会覆盖掉我们在 widget 阶段刷回去的 bg。
                            // - 因此：这里先收集各 pane 的 Rect，便于在应用 effects 之后再修正一次。
                            let mut exabind_instances_area: Option<ratatui::layout::Rect> = None;
                            let mut exabind_output_area: Option<ratatui::layout::Rect> = None;
                            let mut exabind_bottom_area: Option<ratatui::layout::Rect> = None;
                            let mut exabind_radar_area: Option<ratatui::layout::Rect> = None;

                            // 统一背景：先铺一层 app 级背景色，避免切换视图时出现“旧帧残影”或颜色不一致。
                            f.render_widget(Block::new().style(theme.app_bg()), f.area());

                        // Render header
                        f.render_widget(header::render(&state, theme, chunks[0].width), chunks[0]);

                        match state.mode {
                            TuiMode::Serial => {
                                // Render content using ContentPane
                                if let Some(buffer) = state.current_iteration() {
                                    let mut content_widget = ContentPane::new(buffer, theme);
                                    if let Some(query) = &state.search_state.query {
                                        content_widget = content_widget.with_search(query);
                                    }
                                    f.render_widget(content_widget, content_area);
                                }
                            }
                            TuiMode::Parallel => {
                                // 布局：上（实例列表 + 输出） / 下（chat + gate）
                                let vertical = Layout::default()
                                    .direction(Direction::Vertical)
                                    .constraints([
                                        Constraint::Min(0),     // main
                                        Constraint::Length(PARALLEL_BOTTOM_PANEL_HEIGHT),  // bottom panel（后续可做自适应）
                                    ])
                                    .split(content_area);

                                let main_area = vertical[0];
                                let bottom_area = vertical[1];

                                let horizontal = Layout::default()
                                    .direction(Direction::Horizontal)
                                    .constraints([
                                        Constraint::Length(PARALLEL_INSTANCES_PANEL_WIDTH), // instances
                                        Constraint::Length(PARALLEL_PANE_GAP_WIDTH),        // gap
                                        Constraint::Min(0),                                  // output
                                    ])
                                    .split(main_area);

                                let instances_area = horizontal[0];
                                let output_area = horizontal[2];
                                exabind_instances_area = Some(instances_area);
                                exabind_output_area = Some(output_area);
                                exabind_bottom_area = Some(bottom_area);

                                // 左：实例列表
                                f.render_widget(instances::render(&state.parallel, theme), instances_area);

                                // 右：输出（选中实例的当前 job）
                                let output_focused = state.parallel.focus == crate::state::ParallelFocus::Output;

                                let title = if let Some(id) = state.parallel.selected_instance_id() {
                                    if let Some(instance) = state.parallel.selected_instance() {
                                        let state_label = instance.state.to_string();
                                        let total = instance.jobs.len();
                                        if total > 0 {
                                            let current = instance.current_job.saturating_add(1);
                                            format!("Output ({id}) [{state_label}] [job {current}/{total}]")
                                        } else {
                                            format!("Output ({id}) [{state_label}]")
                                        }
                                    } else {
                                        format!("Output ({id})")
                                    }
                                } else {
                                    "Output".to_string()
                                };
                                let block = panel_block(title, output_focused, &theme);
                                let inner = block.inner(output_area);
                                f.render_widget(block, output_area);
                                // exabind 风格边框：需要把“外侧背景”刷回 crust，才能让左上斜切角与底边贴边。
                                patch_exabind_panel_border_bg(f.buffer_mut(), output_area, &theme);

                                // 更新可滚动视图高度（给鼠标滚动/键盘滚动用）
                                viewport_height = inner.height as usize;

                                if let Some(instance) = state.parallel.selected_instance()
                                    && let Some(buffer) = instance.current_job_buffer()
                                {
                                    let mut content_widget = ParallelOutputPane::new(buffer);
                                    if let Some(query) = &state.search_state.query {
                                        content_widget = content_widget.with_search(query);
                                    }
                                    if let Some(sel) = state.parallel.output_selection {
                                        content_widget = content_widget.with_selection(SelectionBounds::from_points(
                                            sel.anchor.x,
                                            sel.anchor.y,
                                            sel.cursor.x,
                                            sel.cursor.y,
                                        ));
                                    }
                                    f.render_widget(content_widget, inner);
                                } else {
                                    let empty = Paragraph::new(Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled("No instance selected", theme.muted()),
                                    ]));
                                    f.render_widget(empty, inner);
                                }

                                // 下：chat + gate（human async chat + gate 面板）
                                let bottom_focused = state.parallel.focus == crate::state::ParallelFocus::Chat;
                                let bottom_block = panel_block("Chat / Gates", bottom_focused, &theme);
                                let bottom_inner = bottom_block.inner(bottom_area);
                                f.render_widget(bottom_block, bottom_area);
                                // exabind 风格边框：需要把“外侧背景”刷回 crust，才能让左上斜切角与底边贴边。
                                patch_exabind_panel_border_bg(f.buffer_mut(), bottom_area, &theme);

                                if bottom_inner.width > 0 && bottom_inner.height > 0 {
                                    // 上：输入框 / Targets / 状态提示 / gate（详情 + 列表）
                                    let inner_chunks = Layout::default()
                                        .direction(Direction::Vertical)
                                        .constraints([
                                            Constraint::Length(PARALLEL_CHAT_INPUT_HEIGHT), // input（多行）
                                            Constraint::Length(1), // targets
                                            Constraint::Length(1), // status
                                            Constraint::Min(0),    // gates（详情 + 列表）
                                        ])
                                        .split(bottom_inner);

                                    let input_area = inner_chunks[0];
                                    let targets_area = inner_chunks[1];
                                    let status_area = inner_chunks[2];
                                    let gates_area = inner_chunks[3];

                                    // gate 详情/快捷 actions 与 gate 列表的区域划分：
                                    // - 只有存在 selected_gate 时，才占用 gates_area 的顶部行。
                                    // - 终端高度太小的情况下，会自动降级：优先保证 gate 列表仍可显示。
                                    let mut gate_info_area = ratatui::layout::Rect::default();
                                    let mut gate_prompt_area = ratatui::layout::Rect::default();
                                    let mut gate_actions_area = ratatui::layout::Rect::default();
                                    let mut gate_list_area = gates_area;
                                    if let Some(gate_id) = state.parallel.selected_gate.as_deref()
                                        && state.parallel.gates.contains_key(gate_id)
                                        && gates_area.height >= 4
                                    {
                                        let chunks = Layout::default()
                                            .direction(Direction::Vertical)
                                            .constraints([
                                                Constraint::Length(1),
                                                Constraint::Length(1),
                                                Constraint::Length(1),
                                                Constraint::Min(0),
                                            ])
                                            .split(gates_area);
                                        gate_info_area = chunks[0];
                                        gate_prompt_area = chunks[1];
                                        gate_actions_area = chunks[2];
                                        gate_list_area = chunks[3];
                                    }

                                    // 保存布局快照，用于鼠标点击/拖拽做 hit-test
                                    parallel_layout = Some(ParallelLayoutSnapshot {
                                        instances_inner: inner_block(instances_area),
                                        output_inner: inner,
                                        bottom_inner,
                                        chat_input_area: input_area,
                                        chat_targets_area: targets_area,
                                        gate_list_area,
                                        gate_actions_area,
                                    });

                                    // 1) chat 输入行
                                    let prompt_style = if bottom_focused {
                                        theme.accent()
                                    } else {
                                        theme.muted()
                                    };

                                    let selection_style = Style::default().bg(theme.selection_bg());

                                    // 约定：prompt 占 3 个 cell（" " + ">" + " "）
                                    let prefix_cells: u16 = 3;
                                    let content_width = input_area.width.saturating_sub(prefix_cells);

                                    let editor = &state.parallel.chat_editor;
                                    let mut input_lines: Vec<Line> = Vec::new();
                                    let mut cursor_pos: Option<(u16, u16)> = None;

                                    if editor.is_empty() && !bottom_focused {
                                        // 未聚焦且为空：显示占位提示。
                                        // 这里也做与正文一致的“下移 + 底部留白”，避免点击定位与渲染产生错位感。
                                        let viewport_rows = input_area.height as usize;
                                        let pad_top = chat_editor_pad_top(viewport_rows, 1);
                                        for _ in 0..pad_top {
                                            input_lines.push(Line::from(""));
                                        }

                                        input_lines.push(Line::from(vec![
                                            Span::raw(" "),
                                            Span::styled(">", prompt_style),
                                            Span::raw(" "),
                                            Span::styled(
                                                "Type: msg (-> selected) | @instance msg | !approve/!deny/!resolve ...",
                                                theme.muted(),
                                            ),
                                        ]));
                                    } else {
                                            let total_lines = editor.lines.len().max(1);
                                            let cursor_row = editor.cursor.row.min(total_lines.saturating_sub(1));
                                            let viewport_rows = input_area.height as usize;
                                            // 视觉对齐策略：
                                            // - 当行数不足输入框高度时，把内容“下移”（底部对齐），更符合聊天输入区直觉。
                                            // - 当行数超过输入框高度时，保持光标行可见（垂直滚动）。
                                            let start_row = if total_lines <= viewport_rows {
                                                0
                                            } else {
                                                cursor_row.saturating_sub(viewport_rows.saturating_sub(1))
                                            };
                                            let pad_top = chat_editor_pad_top(viewport_rows, total_lines);

                                            for i in 0..viewport_rows {
                                                // 内容不足高度时，在上方补空行，把实际文本推到更靠下的位置。
                                                if pad_top > 0 && i < pad_top {
                                                    input_lines.push(Line::from(""));
                                                    continue;
                                                }

                                                let row =
                                                    start_row.saturating_add(i.saturating_sub(pad_top));
                                                if row >= total_lines {
                                                    input_lines.push(Line::from(""));
                                                    continue;
                                                }

                                            let prefix_symbol = if row == 0 { ">" } else { "|" };

                                            let line_text = editor.lines.get(row).map(|s| s.as_str()).unwrap_or("");
                                            let graphemes: Vec<&str> =
                                                UnicodeSegmentation::graphemes(line_text, true).collect();
                                            let widths: Vec<u16> = graphemes
                                                .iter()
                                                .map(|g| UnicodeWidthStr::width(*g) as u16)
                                                .collect();

                                            let line_len = graphemes.len();
                                            let selection_range = editor.selection_range_for_row(row);

                                            // 光标所在行：做水平滚动，保证光标可见
                                            let is_cursor_row = row == cursor_row;
                                            let cursor_col = if is_cursor_row {
                                                editor.cursor.col.min(line_len)
                                            } else {
                                                0
                                            };
                                            let cursor_cell = widths.iter().take(cursor_col).copied().sum::<u16>();

                                            let scroll_cell = if is_cursor_row && content_width > 0 {
                                                if cursor_cell >= content_width {
                                                    cursor_cell.saturating_sub(content_width.saturating_sub(1))
                                                } else {
                                                    0
                                                }
                                            } else {
                                                0
                                            };

                                            // 根据 scroll_cell 找到可视起点（按 grapheme 边界）
                                            let mut start_idx = 0usize;
                                            let mut start_cell = 0u16;
                                            for (idx, w) in widths.iter().enumerate() {
                                                if start_cell.saturating_add(*w) > scroll_cell {
                                                    start_idx = idx;
                                                    break;
                                                }
                                                start_cell = start_cell.saturating_add(*w);
                                                start_idx = idx.saturating_add(1);
                                            }

                                            // 找到可视终点
                                            let mut end_idx = start_idx;
                                            let mut used_cells = 0u16;
                                            for idx in start_idx..line_len {
                                                let w = widths.get(idx).copied().unwrap_or(0);
                                                if used_cells.saturating_add(w) > content_width {
                                                    break;
                                                }
                                                used_cells = used_cells.saturating_add(w);
                                                end_idx = idx.saturating_add(1);
                                            }

                                            let vis_start = start_idx.min(line_len);
                                            let vis_end = end_idx.min(line_len).max(vis_start);

                                            // 构造 content spans（带选择高亮）
                                            let mut content_spans: Vec<Span> = Vec::new();
                                            if let Some((sel_start, sel_end)) = selection_range {
                                                let inter_start = sel_start.max(vis_start);
                                                let inter_end = sel_end.min(vis_end);

                                                if inter_start < inter_end {
                                                    let before = graphemes[vis_start..inter_start].concat();
                                                    let selected = graphemes[inter_start..inter_end].concat();
                                                    let after = graphemes[inter_end..vis_end].concat();

                                                    if !before.is_empty() {
                                                        content_spans.push(Span::raw(before));
                                                    }
                                                    if !selected.is_empty() {
                                                        content_spans.push(Span::styled(selected, selection_style));
                                                    }
                                                    if !after.is_empty() {
                                                        content_spans.push(Span::raw(after));
                                                    }
                                                } else {
                                                    content_spans.push(Span::raw(graphemes[vis_start..vis_end].concat()));
                                                }
                                            } else {
                                                content_spans.push(Span::raw(graphemes[vis_start..vis_end].concat()));
                                            }

                                            let mut spans = vec![
                                                Span::raw(" "),
                                                Span::styled(prefix_symbol, prompt_style),
                                                Span::raw(" "),
                                            ];
                                            spans.extend(content_spans);
                                            input_lines.push(Line::from(spans));

                                            // 计算 cursor 的屏幕位置（聚焦时才显示）
                                            if bottom_focused && is_cursor_row {
                                                let cursor_x_cells = prefix_cells.saturating_add(
                                                    cursor_cell.saturating_sub(start_cell),
                                                );
                                                let cursor_x = input_area
                                                    .x
                                                    .saturating_add(cursor_x_cells.min(input_area.width.saturating_sub(1)));
                                                let cursor_y = input_area.y.saturating_add(i as u16);
                                                cursor_pos = Some((cursor_x, cursor_y));
                                            }
                                        }
                                    }

                                    // 填满剩余行，避免旧帧残影
                                    while input_lines.len() < input_area.height as usize {
                                        input_lines.push(Line::from(""));
                                    }

                                    f.render_widget(Paragraph::new(input_lines), input_area);
                                    if let Some((x, y)) = cursor_pos {
                                        f.set_cursor_position((x, y));
                                    }

                                    // 2) Targets chips（默认消息目标选择）
                                    let selected_id = state.parallel.selected_instance_id();
                                    let mut targets_spans: Vec<Span> = vec![
                                        Span::raw(" "),
                                        Span::styled("Targets:", theme.muted()),
                                        Span::raw(" "),
                                    ];
                                    if state.parallel.instance_order.is_empty() {
                                        targets_spans.push(Span::styled(
                                            "(none)",
                                            theme.muted(),
                                        ));
                                    } else {
                                        for id in &state.parallel.instance_order {
                                            let label = format!("@{id}");
                                            let is_selected = selected_id == Some(id);
                                            let chip_style = if is_selected {
                                                Style::default()
                                                    .fg(theme.colors().crust)
                                                    .bg(theme.selection_bg())
                                                    .add_modifier(ratatui::style::Modifier::BOLD)
                                            } else {
                                                Style::default().fg(theme.colors().sky)
                                            };
                                            targets_spans.push(Span::styled(label, chip_style));
                                            targets_spans.push(Span::raw(" "));
                                        }
                                    }
                                    let targets_line = Line::from(targets_spans);
                                    f.render_widget(Paragraph::new(targets_line), targets_area);

                                    // 3) 状态提示
                                    let status = state
                                        .parallel
                                        .chat_status
                                        .as_deref()
                                        .unwrap_or(if bottom_focused {
                                            "Enter=send  Shift+Enter|Alt+Enter|Ctrl+J=newline  Arrows=move  Esc=clear  Tab=switch"
                                        } else {
                                            "Tab to focus chat"
                                        });
                                    let status_line = Line::from(vec![
                                        Span::raw(" "),
                                        Span::styled(status.to_string(), theme.muted()),
                                    ]);
                                    f.render_widget(Paragraph::new(status_line), status_area);

                                    // 4) 当前 gate 详情（点击 gate 列表行后显示）
                                    if let Some(gate_id) = state.parallel.selected_gate.as_deref()
                                        && let Some(g) = state.parallel.gates.get(gate_id)
                                    {
                                        let kind = match g.request.kind {
                                            ralph_proto::GateKind::Consult => "consult",
                                            ralph_proto::GateKind::Approval => "approval",
                                        };

                                        if gate_info_area.height > 0 {
                                            let info_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Gate:", theme.muted()),
                                                Span::raw(" "),
                                                Span::styled(
                                                    gate_id.to_string(),
                                                    Style::default()
                                                        .fg(theme.colors().mauve)
                                                        .add_modifier(ratatui::style::Modifier::BOLD),
                                                ),
                                                Span::raw(" "),
                                                Span::styled(
                                                    format!("[{kind}]"),
                                                    Style::default().fg(theme.colors().mauve),
                                                ),
                                                Span::raw(" "),
                                                Span::styled("by=", theme.muted()),
                                                Span::styled(
                                                    g.request.requested_by.to_string(),
                                                    Style::default().fg(theme.colors().sky),
                                                ),
                                            ]);
                                            f.render_widget(Paragraph::new(info_line), gate_info_area);
                                        }

                                        if gate_prompt_area.height > 0 {
                                            // 尽量按可视宽度截断，避免 prompt 把一行撑爆。
                                            let prefix_cells =
                                                1 + UnicodeWidthStr::width("Prompt:") as u16 + 1;
                                            let max_chars = gate_prompt_area
                                                .width
                                                .saturating_sub(prefix_cells) as usize;
                                            let prompt = truncate_with_ellipsis(&g.request.prompt, max_chars);

                                            let prompt_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Prompt:", theme.muted()),
                                                Span::raw(" "),
                                                Span::styled(prompt, theme.text()),
                                            ]);
                                            f.render_widget(Paragraph::new(prompt_line), gate_prompt_area);
                                        }

                                        if gate_actions_area.height > 0 {
                                            let action_style = Style::default()
                                                .fg(theme.colors().sky)
                                                .add_modifier(ratatui::style::Modifier::BOLD);
                                            let actions_line = Line::from(vec![
                                                Span::raw(" "),
                                                Span::styled("Actions:", theme.muted()),
                                                Span::raw(" "),
                                                Span::styled("!approve", action_style),
                                                Span::raw(" "),
                                                Span::styled("!deny", action_style),
                                                Span::raw(" "),
                                                Span::styled("!resolve", action_style),
                                            ]);
                                            f.render_widget(Paragraph::new(actions_line), gate_actions_area);
                                        }
                                    }

                                    // 5) gate 列表（最新在上）
                                    let mut gate_lines: Vec<Line> = Vec::new();
                                    let max_lines = gate_list_area.height as usize;

                                    for gate_id in state.parallel.gate_order.iter().rev() {
                                        if gate_lines.len() >= max_lines {
                                            break;
                                        }

                                        let Some(g) = state.parallel.gates.get(gate_id) else {
                                            continue;
                                        };

                                        let kind = match g.request.kind {
                                            ralph_proto::GateKind::Consult => "consult",
                                            ralph_proto::GateKind::Approval => "approval",
                                        };

                                        let now = std::time::Instant::now();
                                        let (status_text, status_style) = match g.status_at(now) {
                                            GateStatus::Resolved => (
                                                "resolved".to_string(),
                                                Style::default().fg(theme.colors().green),
                                            ),
                                            GateStatus::Timeout => (
                                                "timeout".to_string(),
                                                Style::default().fg(theme.colors().yellow),
                                            ),
                                            GateStatus::Waiting { remaining_seconds } => (
                                                format!("T-{remaining_seconds}s"),
                                                Style::default().fg(theme.colors().sky),
                                            ),
                                            GateStatus::Open => (
                                                "open".to_string(),
                                                Style::default().fg(theme.colors().sky),
                                            ),
                                        };

                                        let prompt = truncate_with_ellipsis(&g.request.prompt, 48);
                                        let is_selected_gate =
                                            state.parallel.selected_gate.as_deref() == Some(gate_id.as_str());
                                        let marker = if is_selected_gate { ">" } else { " " };
                                        let marker_style = if is_selected_gate {
                                            theme
                                                .accent()
                                                .add_modifier(ratatui::style::Modifier::BOLD)
                                        } else {
                                            Style::default()
                                        };

                                        gate_lines.push(Line::from(vec![
                                            Span::styled(marker, marker_style),
                                            Span::styled(
                                                format!("[{kind}]"),
                                                Style::default().fg(theme.colors().mauve),
                                            ),
                                            Span::raw(" "),
                                            Span::styled(
                                                gate_id.clone(),
                                                Style::default()
                                                    .fg(theme.colors().text)
                                                    .add_modifier(ratatui::style::Modifier::BOLD),
                                            ),
                                            Span::raw(" "),
                                            Span::styled(status_text, status_style),
                                            Span::raw(" "),
                                            Span::styled(
                                                g.request.requested_by.to_string(),
                                                theme.muted(),
                                            ),
                                            Span::raw(" "),
                                            Span::styled(prompt, theme.text()),
                                        ]));
                                    }

                                    if gate_lines.is_empty() {
                                        gate_lines.push(Line::from(vec![
                                            Span::raw(" "),
                                            Span::styled("No gates", theme.muted()),
                                        ]));
                                    }

                                    f.render_widget(Paragraph::new(gate_lines), gate_list_area);
                                }
                            }
                        }

                        // 右上角覆盖层：Hat Graph Radar（ASCII Mermaid）
                        if let Some(area) =
                            render_hat_graph_radar_overlay(f, content_area, &state, &theme)
                        {
                            exabind_radar_area = Some(area);
                        }

                        // Render footer
                        f.render_widget(footer::render(&state, theme), chunks[2]);

                        // Render help overlay if active
                        if state.show_help {
                            help::render(f, f.area());
                        }

                            // Effects：在 widget 渲染完成后，对 buffer 施加后处理（shader-like）。
                            if let Some(manager) = effects.as_mut() {
                                // 先取出 area，再借用 buffer_mut，避免同一表达式里出现可变/不可变借用冲突。
                                let area = f.area();
                                manager.process_effects(fx_delta, f.buffer_mut(), area);

                                // Warp 透明背景模式（bg=Reset）下：
                                // - Output 的 sweep 动画可能会覆盖边框 cell 的 bg，
                                //   让“最外圈”看起来也被染上 panel 底色（用户感知为外圈不透明/发灰）。
                                // - 因此在 effects 之后再刷一遍 exabind 边框外侧背景，确保外圈始终是 Reset。
                                if theme.app_bg_color() == Color::Reset {
                                    if let Some(area) = exabind_instances_area {
                                        patch_exabind_panel_border_bg(f.buffer_mut(), area, &theme);
                                    }
                                    if let Some(area) = exabind_output_area {
                                        patch_exabind_panel_border_bg(f.buffer_mut(), area, &theme);
                                    }
                                    if let Some(area) = exabind_bottom_area {
                                        patch_exabind_panel_border_bg(f.buffer_mut(), area, &theme);
                                    }
                                    if let Some(area) = exabind_radar_area {
                                        patch_exabind_panel_border_bg(f.buffer_mut(), area, &theme);
                                    }
                                }
                            }
                    })?;
                }

                // Priority 2.5: Apply updates from parallel runner (observer → channel)
                maybe_update = async {
                    if let Some(rx) = update_rx.as_mut() {
                        rx.recv().await
                    } else {
                        std::future::pending::<Option<TuiUpdate>>().await
                    }
                } => {
                    if let Some(update) = maybe_update {
                        let mut state = self.state.lock().unwrap();
                        state.apply_update(update);
                    }
                }

                // Priority 3: Handle termination signal
                _ = self.terminated_rx.changed() => {
                    if *self.terminated_rx.borrow() {
                        break;
                    }
                }
            }
        }

        // NOTE: Explicit cleanup removed - now handled by defer! guard above.
        // The guard ensures cleanup happens even on task abort or panic.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::{Action, map_key};
    use crate::state::TuiState;
    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ralph_proto::{
        Event, GateKind, GateRequest, HatInstanceId, HatInstanceState, TOPIC_GATE_REQUEST,
    };
    use ratatui::buffer::Buffer;
    use ratatui::layout::Rect;
    use ratatui::prelude::Widget;
    use ratatui::text::Line;
    use tokio::sync::watch;

    // =========================================================================
    // 启动动画：首帧 priming（避免“先全显示再动画”的闪烁）
    // =========================================================================

    #[test]
    fn startup_animation_first_frame_priming_prevents_full_ui_flash() {
        let theme = TuiTheme::default();
        let area = Rect::new(0, 0, 20, 6);

        // 基础面板：我们用 border 的 fg 是否变成 crust 来判断“是否被隐藏”。
        // 注意：tachyonfx 的 sweep/fade 不会改 symbol，只会改 fg/bg，所以测试必须看 style。
        let render_panel = |buf: &mut Buffer| {
            let block = panel_block("Output", false, &theme);
            block.render(area, buf);
        };

        // Case A（未 priming）：如果首帧 delta 很大，启动动画会从中途开始，
        // 顶部区域会几乎“已经是可见态”，导致观感像“先全显示，再扫一遍”。
        let mut buf_unprimed = Buffer::empty(area);
        render_panel(&mut buf_unprimed);
        let mut effect_unprimed = animation::startup_open_effect(theme, area);
        effect_unprimed.process(FxDuration::from_millis(200), &mut buf_unprimed, area);

        assert_ne!(
            buf_unprimed[(0, 0)].style().fg,
            Some(theme.colors().crust),
            "未 priming 的首帧（大 delta）不应把左上角 border 彻底隐藏"
        );

        // Case B（priming）：把“启动动画刚添加的那一帧”的 delta 强制归零，
        // 让第一帧落在“全隐藏起步态”，下一帧再开始推进时间轴。
        let mut buf_primed = Buffer::empty(area);
        render_panel(&mut buf_primed);
        let mut effect_primed = animation::startup_open_effect(theme, area);

        let mut fx_delta = FxDuration::from_millis(200);
        let mut last_effect_tick = Instant::now()
            .checked_sub(Duration::from_millis(200))
            .unwrap();
        prime_animation_first_frame(&mut fx_delta, &mut last_effect_tick);

        effect_primed.process(fx_delta, &mut buf_primed, area);

        assert_eq!(
            buf_primed[(0, 0)].style().fg,
            Some(theme.colors().crust),
            "priming 后首帧应把左上角 border 的 fg 刷成 crust（不可见）"
        );
        assert_eq!(
            buf_primed[(0, 0)].style().bg,
            Some(theme.colors().crust),
            "priming 后首帧应把左上角 border 的 bg 刷成 crust（与外侧一致）"
        );
    }

    // =========================================================================
    // AC1: Events Reach State — TuiStreamHandler → IterationBuffer
    // =========================================================================

    #[test]
    fn dispatch_action_scroll_down_calls_scroll_down_on_current_buffer() {
        // Given TuiState with an iteration buffer containing content
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for i in 0..20 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        let initial_offset = state.current_iteration().unwrap().scroll_offset;
        assert_eq!(initial_offset, 0);

        // When dispatch_action with ScrollDown and viewport_height 10
        dispatch_action(Action::ScrollDown, &mut state, 10);

        // Then scroll_offset is incremented
        assert_eq!(
            state.current_iteration().unwrap().scroll_offset,
            1,
            "scroll_down should increment scroll_offset"
        );
    }

    #[test]
    fn request_interrupt_sets_watch_signal() {
        let (tx, rx) = watch::channel(false);
        request_interrupt(Some(&tx));
        assert!(
            *rx.borrow(),
            "request_interrupt should set the watch channel to true"
        );
    }

    // =========================================================================
    // AC2: Keyboard Triggers Actions — 'j' → scroll_down()
    // =========================================================================

    #[test]
    fn j_key_triggers_scroll_down_action() {
        // Given key press 'j'
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);

        // When map_key is called
        let action = map_key(key);

        // Then Action::ScrollDown is returned
        assert_eq!(action, Action::ScrollDown);
    }

    #[test]
    fn dispatch_action_scroll_up_calls_scroll_up_on_current_buffer() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for i in 0..20 {
            buffer.append_line(Line::from(format!("line {}", i)));
        }
        // Set initial scroll offset to 5
        state.current_iteration_mut().unwrap().scroll_offset = 5;

        dispatch_action(Action::ScrollUp, &mut state, 10);

        assert_eq!(
            state.current_iteration().unwrap().scroll_offset,
            4,
            "scroll_up should decrement scroll_offset"
        );
    }

    #[test]
    fn dispatch_action_scroll_top_jumps_to_top() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for _ in 0..20 {
            buffer.append_line(Line::from("line"));
        }
        state.current_iteration_mut().unwrap().scroll_offset = 10;

        dispatch_action(Action::ScrollTop, &mut state, 10);

        assert_eq!(state.current_iteration().unwrap().scroll_offset, 0);
    }

    #[test]
    fn dispatch_action_scroll_bottom_jumps_to_bottom() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        for _ in 0..20 {
            buffer.append_line(Line::from("line"));
        }

        dispatch_action(Action::ScrollBottom, &mut state, 10);

        // max_scroll = 20 - 10 = 10
        assert_eq!(state.current_iteration().unwrap().scroll_offset, 10);
    }

    #[test]
    fn dispatch_action_next_iteration_navigates_forward() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        state.start_new_iteration();
        state.start_new_iteration();
        state.current_view = 0;
        state.following_latest = false;

        dispatch_action(Action::NextIteration, &mut state, 10);

        assert_eq!(state.current_view, 1);
    }

    #[test]
    fn dispatch_action_prev_iteration_navigates_backward() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        state.start_new_iteration();
        state.start_new_iteration();
        state.current_view = 2;

        dispatch_action(Action::PrevIteration, &mut state, 10);

        assert_eq!(state.current_view, 1);
    }

    #[test]
    fn dispatch_action_show_help_sets_show_help() {
        let mut state = TuiState::new();
        assert!(!state.show_help);

        dispatch_action(Action::ShowHelp, &mut state, 10);

        assert!(state.show_help);
    }

    #[test]
    fn dispatch_action_dismiss_help_clears_show_help() {
        let mut state = TuiState::new();
        state.show_help = true;

        dispatch_action(Action::DismissHelp, &mut state, 10);

        assert!(!state.show_help);
    }

    #[test]
    fn dispatch_action_toggle_hat_graph_zoom_toggles_when_radar_present() {
        let mut state = TuiState::new();
        state.set_hat_graph_radar("compact".to_string(), "full".to_string());
        assert!(!state.hat_graph_zoomed);

        dispatch_action(Action::ToggleHatGraphZoom, &mut state, 10);
        assert!(state.hat_graph_zoomed);

        dispatch_action(Action::ToggleHatGraphZoom, &mut state, 10);
        assert!(!state.hat_graph_zoomed);
    }

    #[test]
    fn dispatch_action_search_next_calls_next_match() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("find me"));
        buffer.append_line(Line::from("find me again"));
        state.search("find");
        assert_eq!(state.search_state.current_match, 0);

        dispatch_action(Action::SearchNext, &mut state, 10);

        assert_eq!(state.search_state.current_match, 1);
    }

    #[test]
    fn dispatch_action_search_prev_calls_prev_match() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("find me"));
        buffer.append_line(Line::from("find me again"));
        state.search("find");
        state.search_state.current_match = 1;

        dispatch_action(Action::SearchPrev, &mut state, 10);

        assert_eq!(state.search_state.current_match, 0);
    }

    // =========================================================================
    // AC5: Quit Returns True to Exit Loop
    // =========================================================================

    #[test]
    fn dispatch_action_quit_returns_true() {
        let mut state = TuiState::new();
        let should_quit = dispatch_action(Action::Quit, &mut state, 10);
        assert!(should_quit, "Quit action should return true to signal exit");
    }

    #[test]
    fn dispatch_action_non_quit_returns_false() {
        let mut state = TuiState::new();
        state.start_new_iteration();
        let buffer = state.current_iteration_mut().unwrap();
        buffer.append_line(Line::from("line"));

        let should_quit = dispatch_action(Action::ScrollDown, &mut state, 10);
        assert!(!should_quit, "Non-quit actions should return false");
    }

    // =========================================================================
    // AC6: No PTY Code — Structural Test
    // =========================================================================

    #[test]
    fn no_pty_handle_in_app() {
        let source = include_str!("app.rs");
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];

        // Check for PTY-related imports/code
        assert!(
            !production_code.contains("PtyHandle"),
            "app.rs should not contain PtyHandle after refactor"
        );
        assert!(
            !production_code.contains("tui_term"),
            "app.rs should not contain tui_term references after refactor"
        );
        assert!(
            !production_code.contains("TerminalWidget"),
            "app.rs should not contain TerminalWidget after refactor"
        );
    }

    /// Regression test: TUI must NOT have tokio::signal::ctrl_c() handler.
    ///
    /// Raw mode prevents SIGINT, so tokio's signal handler never fires.
    /// TUI must detect Ctrl+C directly via crossterm events.
    #[test]
    fn no_tokio_signal_handler_in_app() {
        let source = include_str!("app.rs");
        let pattern = ["tokio", "::", "signal", "::", "ctrl_c", "()"].concat();
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];
        let occurrences: Vec<_> = production_code.match_indices(&pattern).collect();
        assert!(
            occurrences.is_empty(),
            "Found {} occurrence(s) of tokio::signal::ctrl_c() in production code. \
             This doesn't work in raw mode - use crossterm events instead.",
            occurrences.len()
        );
    }

    // =========================================================================
    // Parallel TUI: Shift+Enter（需要启用 kitty keyboard protocol）
    // =========================================================================

    #[test]
    fn enables_keyboard_enhancement_flags_for_shift_enter() {
        // 说明：
        // - `Shift+Enter` 是否可区分，核心取决于“终端会不会上报 Enter 的修饰键”。
        // - 我们通过 crossterm 的 progressive keyboard enhancement（kitty protocol）来提升可区分性。
        // - 这里用结构性测试确保该能力不会在重构中被意外删除。
        let source = include_str!("app.rs");
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];

        assert!(
            production_code.contains("PushKeyboardEnhancementFlags"),
            "Expected app.rs to enable keyboard enhancement flags (PushKeyboardEnhancementFlags)"
        );
        assert!(
            production_code.contains("PopKeyboardEnhancementFlags"),
            "Expected app.rs to disable keyboard enhancement flags on exit (PopKeyboardEnhancementFlags)"
        );
    }

    // =========================================================================
    // Parallel TUI: Chat 输入框（底部对齐 + hit-test）
    // =========================================================================

    #[test]
    fn hit_test_chat_editor_accounts_for_bottom_aligned_padding() {
        // 说明：
        // - 输入框的垂直对齐策略是“下移 + 底部留白 1 行”。
        // - 当高度为 4 行、内容为 2 行时：顶部会补 1 行空白，底部也会留 1 行空白。
        // - hit-test 必须扣掉顶部 padding，否则点击第一行内容会错误映射到第二行。
        let mut editor = crate::state::ChatEditorState::default();
        editor.lines = vec!["first".to_string(), "second".to_string()];
        editor.cursor = crate::state::TextPos { row: 0, col: 0 };
        editor.selection = None;

        let area = ratatui::layout::Rect::new(0, 0, 20, 4);

        // 第 0 行是 padding；第 1 行应映射到 logical row=0。
        let pos_first = hit_test_chat_editor(&editor, area, area.x, area.y + 1);
        assert_eq!(pos_first.row, 0);

        // 第 2 行应映射到 logical row=1（第 3 行是底部留白）。
        let pos_second = hit_test_chat_editor(&editor, area, area.x, area.y + 2);
        assert_eq!(pos_second.row, 1);
    }

    // =========================================================================
    // Parallel TUI: Clipboard selection text（所见即所得）
    // =========================================================================

    #[test]
    fn extract_output_selection_text_copies_single_line_region() {
        let mut buffer = crate::state::IterationBuffer::new(1);
        buffer.append_line(Line::from("hello world"));

        let sel = crate::state::ScreenSelection::new(
            crate::state::ScreenPos { x: 0, y: 0 },
            crate::state::ScreenPos { x: 4, y: 0 },
        );

        let got = extract_output_selection_text(
            crate::state::CurrentOutputBuffer::Serial(&buffer),
            40,
            1,
            sel,
            None,
        );
        assert_eq!(got, "hello");
    }

    #[test]
    fn extract_output_selection_text_copies_multi_line_region_with_newlines() {
        let mut buffer = crate::state::IterationBuffer::new(1);
        buffer.append_line(Line::from("hello"));
        buffer.append_line(Line::from("world"));

        let sel = crate::state::ScreenSelection::new(
            crate::state::ScreenPos { x: 0, y: 0 },
            crate::state::ScreenPos { x: 4, y: 1 },
        );

        let got = extract_output_selection_text(
            crate::state::CurrentOutputBuffer::Serial(&buffer),
            40,
            2,
            sel,
            None,
        );
        assert_eq!(got, "hello\nworld");
    }

    // =========================================================================
    // Parallel TUI: Targets/Gates 快捷交互（chips + 默认目标）
    // =========================================================================

    #[test]
    fn resolve_human_message_target_instance_prefers_explicit_target() {
        let selected = HatInstanceId::from("writer#1");
        let got =
            resolve_human_message_target_instance(Some("writer#2".to_string()), Some(&selected));
        assert_eq!(got, Some("writer#2".to_string()));
    }

    #[test]
    fn resolve_human_message_target_instance_defaults_to_selected_instance() {
        let selected = HatInstanceId::from("writer#2");
        let got = resolve_human_message_target_instance(None, Some(&selected));
        assert_eq!(got, Some("writer#2".to_string()));
    }

    #[test]
    fn mouse_click_targets_chip_switches_selected_instance() {
        let mut state = TuiState::new_parallel();
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#1"), HatInstanceState::Idle);
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#2"), HatInstanceState::Idle);

        // 初始选中 writer#1
        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#1"
        );

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
        };

        // 点击第二个 chip（@writer#2）。
        // 说明：Targets 行格式固定：" Targets: @writer#1 @writer#2 ..."
        let click_x = layout.chat_targets_area.x + 20;
        let click_y = layout.chat_targets_area.y;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(mouse, &mut state, layout, &mut anchor);

        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#2"
        );
        assert_eq!(state.parallel.focus, ParallelFocus::Chat);
    }

    #[test]
    fn mouse_click_gate_row_selects_gate_and_switches_selected_instance() {
        let mut state = TuiState::new_parallel();
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#1"), HatInstanceState::Idle);
        state
            .parallel
            .register_instance(HatInstanceId::from("writer#2"), HatInstanceState::Idle);

        // 两个 gate：g1(by writer#1) → g2(by writer#2)（列表渲染“最新在上”）。
        let req1 = GateRequest {
            gate_id: "g1".to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#1"),
            kind: GateKind::Consult,
            timeout_seconds: None,
            prompt: "p1".to_string(),
            proposed_default: None,
        };
        let req2 = GateRequest {
            gate_id: "g2".to_string(),
            thread_id: None,
            requested_by: HatInstanceId::from("writer#2"),
            kind: GateKind::Approval,
            timeout_seconds: None,
            prompt: "p2".to_string(),
            proposed_default: None,
        };
        let ev1 = Event::new(
            TOPIC_GATE_REQUEST,
            serde_json::to_string(&req1).unwrap().as_str(),
        );
        let ev2 = Event::new(
            TOPIC_GATE_REQUEST,
            serde_json::to_string(&req2).unwrap().as_str(),
        );
        state.parallel.apply_event(&ev1);
        state.parallel.apply_event(&ev2);

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
        };

        // 点击 gate 列表第 0 行（最新的 g2）。
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: layout.gate_list_area.x,
            row: layout.gate_list_area.y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(mouse, &mut state, layout, &mut anchor);

        assert_eq!(state.parallel.selected_gate.as_deref(), Some("g2"));
        assert_eq!(
            state.parallel.selected_instance_id().unwrap().as_str(),
            "writer#2"
        );
    }

    #[test]
    fn mouse_click_gate_action_chip_prefills_input_without_sending() {
        let mut state = TuiState::new_parallel();
        state.parallel.selected_gate = Some("g2".to_string());

        // 先塞一点旧内容，验证点击会覆盖。
        state.parallel.chat_editor.insert_char('x');
        assert_eq!(state.parallel.chat_editor.text(), "x");

        let layout = ParallelLayoutSnapshot {
            instances_inner: ratatui::layout::Rect::new(0, 0, 10, 10),
            output_inner: ratatui::layout::Rect::new(0, 0, 0, 0),
            bottom_inner: ratatui::layout::Rect::new(20, 0, 60, 10),
            chat_input_area: ratatui::layout::Rect::new(20, 0, 60, 3),
            chat_targets_area: ratatui::layout::Rect::new(20, 3, 60, 1),
            gate_actions_area: ratatui::layout::Rect::new(20, 4, 60, 1),
            gate_list_area: ratatui::layout::Rect::new(20, 5, 60, 3),
        };

        // 点击 `!resolve`（actions 第 3 个）。
        let click_x = layout.gate_actions_area.x + 25;
        let click_y = layout.gate_actions_area.y;
        let mouse = MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: click_x,
            row: click_y,
            modifiers: KeyModifiers::empty(),
        };
        let mut anchor = None;
        handle_parallel_mouse_down(mouse, &mut state, layout, &mut anchor);

        assert_eq!(state.parallel.chat_editor.text(), "!resolve g2 ");
    }

    /// Verify Ctrl+C handling exists in production code.
    ///
    /// Since raw mode prevents SIGINT, we must handle Ctrl+C via crossterm events.
    /// TUI is observation-only, so Ctrl+C breaks out of the event loop.
    #[test]
    fn ctrl_c_handling_exists_in_app() {
        let source = include_str!("app.rs");
        let test_module_start = source.find("#[cfg(test)]").unwrap_or(source.len());
        let production_code = &source[..test_module_start];

        assert!(
            production_code.contains("KeyCode::Char('c')")
                && production_code.contains("KeyModifiers::CONTROL"),
            "Production code must detect Ctrl+C via crossterm events"
        );
    }
}
