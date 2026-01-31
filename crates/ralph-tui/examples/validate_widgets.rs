//! Outputs header and footer widgets to files for TUI validation.
//!
//! Run with: cargo run -p ralph-tui --example validate_widgets
//!
//! 建议用法（视觉回归）：
//! - 生成输出：`cargo run -p ralph-tui --example validate_widgets`
//! - 输出位置：`target/tui-validation/*.txt`（避免污染 git 工作区）
//! - 然后用项目内的 `/tui-validate` 技能校验：
//!   - `file:target/tui-validation/header.txt criteria:ralph-header`
//!   - `file:target/tui-validation/footer_active.txt criteria:ralph-footer`
//!   - `file:target/tui-validation/parallel_full_layout.txt criteria:ralph-full`

use ralph_core::{HatJobOutputChunk, OutputStream};
use ralph_proto::{Event, HatId};
use ralph_tui::TuiState;
use ralph_tui::theme::{TuiTheme, panel_block};
use ralph_tui::widgets::{instances, parallel_output::ParallelOutputPane};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::text::{Line, Span};
use std::fs;
use std::time::Duration;

fn render_to_string(terminal: &Terminal<TestBackend>) -> String {
    let buffer = terminal.backend().buffer();
    let mut lines = Vec::new();
    for y in 0..buffer.area.height {
        let mut line = String::new();
        for x in 0..buffer.area.width {
            let cell = buffer.cell((x, y)).unwrap();
            line.push_str(cell.symbol());
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

fn main() {
    // 输出写到 `target/`，避免在仓库根目录产生未跟踪文件。
    let output_dir = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("tui-validation");
    fs::create_dir_all(&output_dir).unwrap();

    // 统一使用 TUI 默认主题（Catppuccin Mocha），确保输出可作为视觉回归基线。
    let theme = TuiTheme::default();

    // Create a fully populated state for validation
    let mut state = TuiState::new();
    let event = Event::new("task.start", "");
    state.update(&event);

    state.iteration = 2;
    state.max_iterations = Some(10);
    state.loop_started = Some(
        std::time::Instant::now()
            .checked_sub(Duration::from_secs(272))
            .unwrap(),
    );
    state.pending_hat = Some((HatId::new("builder"), "🔨Builder".to_string()));
    state.last_event = Some("build.task".to_string());
    state.last_event_at = Some(std::time::Instant::now()); // Active

    // Render header (1-line borderless design)
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::header::render(&state, theme, 80);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let header_output = render_to_string(&terminal);
    fs::write(output_dir.join("header.txt"), &header_output).unwrap();
    println!("Header output written to target/tui-validation/header.txt");
    println!("{}", header_output);
    println!();

    // Render header with scroll mode (1-line borderless design)
    state.in_scroll_mode = true;
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::header::render(&state, theme, 80);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let header_scroll_output = render_to_string(&terminal);
    fs::write(output_dir.join("header_scroll.txt"), &header_scroll_output).unwrap();
    println!("Header (scroll mode) output written to target/tui-validation/header_scroll.txt");
    println!("{}", header_scroll_output);
    println!();
    state.in_scroll_mode = false;

    // Render header with idle countdown (1-line borderless design)
    state.idle_timeout_remaining = Some(Duration::from_secs(25));
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::header::render(&state, theme, 80);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let header_idle_output = render_to_string(&terminal);
    fs::write(output_dir.join("header_idle.txt"), &header_idle_output).unwrap();
    println!("Header (idle countdown) output written to target/tui-validation/header_idle.txt");
    println!("{}", header_idle_output);
    println!();
    state.idle_timeout_remaining = None;

    // Render footer (default) - 1-line borderless design
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::footer::render(&state, theme);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let footer_output = render_to_string(&terminal);
    fs::write(output_dir.join("footer_active.txt"), &footer_output).unwrap();
    println!("Footer (active) output written to target/tui-validation/footer_active.txt");
    println!("{}", footer_output);
    println!();

    // Render footer (idle state) - 1-line borderless design
    state.last_event_at = Some(
        std::time::Instant::now()
            .checked_sub(Duration::from_secs(10))
            .unwrap(),
    );
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::footer::render(&state, theme);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let footer_idle_output = render_to_string(&terminal);
    fs::write(output_dir.join("footer_idle.txt"), &footer_idle_output).unwrap();
    println!("Footer (idle) output written to target/tui-validation/footer_idle.txt");
    println!("{}", footer_idle_output);
    println!();

    // Render footer (done state) - 1-line borderless design
    state.pending_hat = None;
    let backend = TestBackend::new(80, 1);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let widget = ralph_tui::footer::render(&state, theme);
            f.render_widget(widget, f.area());
        })
        .unwrap();
    let footer_done_output = render_to_string(&terminal);
    fs::write(output_dir.join("footer_done.txt"), &footer_done_output).unwrap();
    println!("Footer (done) output written to target/tui-validation/footer_done.txt");
    println!("{}", footer_done_output);
    println!();

    // Render full layout simulation (1-line header/footer, maximizes terminal pane)
    state.pending_hat = Some((HatId::new("builder"), "🔨Builder".to_string()));
    state.last_event_at = Some(std::time::Instant::now());
    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1), // Header (1 line, borderless)
                    Constraint::Min(0),    // Terminal pane (flex)
                    Constraint::Length(1), // Footer (1 line, borderless)
                ])
                .split(f.area());

            f.render_widget(
                ralph_tui::header::render(&state, theme, chunks[0].width),
                chunks[0],
            );
            // Middle content area (just empty for this test)
            f.render_widget(panel_block("Terminal Output", false, &theme), chunks[1]);
            f.render_widget(ralph_tui::footer::render(&state, theme), chunks[2]);
        })
        .unwrap();
    let full_output = render_to_string(&terminal);
    fs::write(output_dir.join("full_layout.txt"), &full_output).unwrap();
    println!("Full layout output written to target/tui-validation/full_layout.txt");
    println!("{}", full_output);

    // ------------------------------------------------------------------------
    // Parallel Supervisor TUI (instances/output/chat+gates)
    // ------------------------------------------------------------------------
    //
    // 说明：
    // - 这是一个“可快照/可回归”的并行布局渲染，目标是给 `/tui-validate` 提供输入。
    // - 为了让输出尽量稳定，这里避免依赖实时倒计时（gate.timeout_seconds=None）。
    let mut parallel_state = TuiState::new_parallel();

    // 注入一个实例与一行输出（模拟 writer#1）
    let instance_id = ralph_proto::HatInstanceId::from("writer#1");
    parallel_state
        .parallel
        .register_instance(instance_id.clone(), ralph_proto::HatInstanceState::Running);
    parallel_state.parallel.append_output(&HatJobOutputChunk {
        job_id: 1,
        instance_id: instance_id.clone(),
        stream: OutputStream::Stdout,
        line: "hello from writer".to_string(),
    });

    // 快照稳定性：避免 Instances 列表显示随时间变化的 "0s/1s"。
    if let Some(inst) = parallel_state.parallel.instances.get_mut(&instance_id) {
        inst.last_output_at = None;
    }

    // 注入一个 gate.request（timeout_seconds=None → 状态为 open，不依赖时间）
    let request = ralph_proto::GateRequest {
        gate_id: "g1".to_string(),
        thread_id: None,
        requested_by: instance_id.clone(),
        kind: ralph_proto::GateKind::Consult,
        timeout_seconds: None,
        prompt: "need decision".to_string(),
        proposed_default: None,
    };
    let payload = serde_json::to_string(&request).unwrap();
    parallel_state
        .parallel
        .apply_event(&ralph_proto::Event::new(
            ralph_proto::TOPIC_GATE_REQUEST,
            payload,
        ));

    // 渲染完整布局（与 tests/common/mod.rs 的 render_full 尽量一致）
    let backend = TestBackend::new(100, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal
        .draw(|f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(2), // Header: content + bottom border
                    Constraint::Min(0),    // Content
                    Constraint::Length(2), // Footer: top border + content
                ])
                .split(f.area());

            f.render_widget(
                ralph_tui::header::render(&parallel_state, theme, chunks[0].width),
                chunks[0],
            );

            let content_area = chunks[1];
            let vertical = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(0),    // main
                    Constraint::Length(7), // bottom panel
                ])
                .split(content_area);

            let main_area = vertical[0];
            let bottom_area = vertical[1];

            let horizontal = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Length(30), // instances
                    Constraint::Length(1),  // gap（避免边框贴合）
                    Constraint::Min(0),     // output
                ])
                .split(main_area);

            // 左：实例列表
            f.render_widget(
                instances::render(&parallel_state.parallel, theme),
                horizontal[0],
            );

            // 右：输出（选中实例的当前 job）
            let output_area = horizontal[2];
            let block = panel_block("Output", false, &theme);
            let inner = block.inner(output_area);
            f.render_widget(block, output_area);

            if let Some(instance) = parallel_state.parallel.selected_instance()
                && let Some(buffer) = instance.current_job_buffer()
            {
                let content_widget = ParallelOutputPane::new(buffer);
                f.render_widget(content_widget, inner);
            }

            // 下：chat/gates
            let bottom_block = panel_block("Chat / Gates", false, &theme);
            let bottom_inner = bottom_block.inner(bottom_area);
            f.render_widget(bottom_block, bottom_area);

            if bottom_inner.width > 0 && bottom_inner.height > 0 {
                let inner_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([
                        Constraint::Length(1), // input
                        Constraint::Length(1), // status
                        Constraint::Min(0),    // gates
                    ])
                    .split(bottom_inner);

                let input_area = inner_chunks[0];
                let status_area = inner_chunks[1];
                let gates_area = inner_chunks[2];

                // 输入行（示例：不渲染颜色，只验证文字/布局）
                let input_line = Line::from(vec![
                    Span::raw(" "),
                    Span::raw("> "),
                    Span::raw("Type: @instance msg | !approve/!deny/!resolve ..."),
                ]);
                f.render_widget(ratatui::widgets::Paragraph::new(input_line), input_area);

                // 状态行
                let status_line = Line::from(vec![Span::raw(" "), Span::raw("ready")]);
                f.render_widget(ratatui::widgets::Paragraph::new(status_line), status_area);

                // gate 列表（最多渲染可见高度）
                let mut gate_lines: Vec<Line> = Vec::new();
                let max_lines = gates_area.height as usize;
                for gate_id in parallel_state.parallel.gate_order.iter().rev() {
                    if gate_lines.len() >= max_lines {
                        break;
                    }
                    let Some(g) = parallel_state.parallel.gates.get(gate_id) else {
                        continue;
                    };

                    let kind = match g.request.kind {
                        ralph_proto::GateKind::Consult => "consult",
                        ralph_proto::GateKind::Approval => "approval",
                    };

                    let status = match g.status_at(std::time::Instant::now()) {
                        ralph_tui::state::GateStatus::Resolved => "resolved".to_string(),
                        ralph_tui::state::GateStatus::Timeout => "timeout".to_string(),
                        ralph_tui::state::GateStatus::Waiting { remaining_seconds } => {
                            format!("T-{remaining_seconds}s")
                        }
                        ralph_tui::state::GateStatus::Open => "open".to_string(),
                    };

                    gate_lines.push(Line::from(vec![
                        Span::raw(" "),
                        Span::raw(format!("[{kind}] ")),
                        Span::raw(gate_id),
                        Span::raw(" "),
                        Span::raw(status),
                        Span::raw(" "),
                        Span::raw(g.request.requested_by.to_string()),
                    ]));
                }
                if gate_lines.is_empty() {
                    gate_lines.push(Line::from(vec![Span::raw(" "), Span::raw("No gates")]));
                }

                f.render_widget(ratatui::widgets::Paragraph::new(gate_lines), gates_area);
            }

            f.render_widget(ralph_tui::footer::render(&parallel_state, theme), chunks[2]);
        })
        .unwrap();

    let parallel_output = render_to_string(&terminal);
    fs::write(
        output_dir.join("parallel_full_layout.txt"),
        &parallel_output,
    )
    .unwrap();
    println!("Parallel full layout written to target/tui-validation/parallel_full_layout.txt");
    println!("{}", parallel_output);

    println!("\n=== All validation outputs written to target/tui-validation/ ===");
}
