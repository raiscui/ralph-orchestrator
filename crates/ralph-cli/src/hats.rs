//! CLI commands for the `ralph hats` namespace.
//!
//! 用途（中文说明）：
//! - 在运行 orchestration loop 之前，先检查 hats 配置是否“可启动、可收敛、可理解”
//! - 以纯文本/mermaid 的形式输出拓扑图，方便后续在 TUI 中直接渲染或展示
//!
//! 子命令：
//! - `list`: 列出所有 hats（名称 + 简要描述）
//! - `show`: 显示某个 hat 的详细配置（订阅/发布/指令）
//! - `validate`: 基础拓扑校验（starting_event 是否有订阅者、孤儿事件、dead-end 等）
//! - `graph`: 输出拓扑图（mermaid / ASCII / Unicode，确定性渲染）

use crate::ConfigSource;
use crate::display::colors;
use crate::presets;
use anyhow::{Context, Result};
use beautiful_mermaid_rs::{
    AsciiRenderOptions, render_mermaid_ascii, render_mermaid_ascii_with_meta,
};
use clap::{Parser, Subcommand, ValueEnum};
use ralph_core::{HatRegistry, RalphConfig};
use ralph_tui::state::{
    HatGraphRadar, HatGraphRadarEdgeMeta, HatGraphRadarMeta, HatGraphRadarNodeMeta,
    HatGraphRadarPoint, HatGraphRadarRect,
};
use std::collections::HashSet;
use std::io::Write;
use tracing::warn;

/// Manage configured hats.
#[derive(Parser, Debug)]
pub struct HatsArgs {
    #[command(subcommand)]
    pub command: Option<HatsCommands>,
}

#[derive(Subcommand, Debug)]
pub enum HatsCommands {
    /// Validate hat topology and report issues
    Validate,
    /// Display hat topology graph
    Graph {
        /// Output format (unicode, ascii, compact, mermaid)
        #[arg(long, default_value = "unicode")]
        format: GraphFormat,
        /// Graph view mode (logical hides coordinator, physical includes ralph#1)
        #[arg(long, default_value = "physical")]
        view: GraphView,
    },
    /// List all configured hats (default if no subcommand)
    List {
        /// Output format (table, json)
        #[arg(long, default_value = "table")]
        format: ListFormat,
    },
    /// Show detailed configuration for a specific hat
    Show(ShowArgs),
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum GraphFormat {
    /// Unicode box-drawing characters (┌─┐│└┘▶) - best appearance
    #[default]
    Unicode,
    /// Pure ASCII characters (+--| chars) - maximum compatibility
    Ascii,
    /// Compact single-glyph nodes - minimal output
    Compact,
    /// Raw Mermaid syntax - for external rendering tools
    Mermaid,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum GraphView {
    /// Logical view: Hat→Hat edges only (hides coordinator ralph#1)
    Logical,
    /// Physical view: also shows coordinator ralph#1 edges (useful for coordinator-driven workflows)
    #[default]
    Physical,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum ListFormat {
    #[default]
    Table,
    Json,
}

#[derive(Parser, Debug)]
pub struct ShowArgs {
    /// Name of the hat to show (ID or display name)
    pub name: String,
}

/// Execute a hats command.
pub fn execute(config_path: &std::path::Path, args: HatsArgs, use_colors: bool) -> Result<()> {
    let config_source = ConfigSource::parse(config_path.to_string_lossy().as_ref());
    let mut config = load_config(&config_source)?;
    config.normalize();

    let registry = HatRegistry::from_config(&config);
    let mut stdout = std::io::stdout();

    match args.command {
        None
        | Some(HatsCommands::List {
            format: ListFormat::Table,
        }) => list_hats(&mut stdout, &registry, use_colors),
        Some(HatsCommands::List {
            format: ListFormat::Json,
        }) => list_hats_json(&mut stdout, &registry),
        Some(HatsCommands::Show(show_args)) => {
            show_hat(&mut stdout, &registry, &show_args.name, use_colors)
        }
        Some(HatsCommands::Validate) => validate_hats(&mut stdout, &config, &registry, use_colors),
        Some(HatsCommands::Graph { format, view }) => {
            graph_hats(&mut stdout, &config, &registry, format, view)
        }
    }
}

/// Load configuration from a config source, with proper error handling.
///
/// Supports:
/// - File paths (local config files)
/// - Builtin presets (e.g., `builtin:confession-loop`)
///
/// Remote URLs are intentionally not supported here (the hats command is sync).
fn load_config(config_source: &ConfigSource) -> Result<RalphConfig> {
    match config_source {
        ConfigSource::File(path) => {
            if path.exists() {
                RalphConfig::from_file(path)
                    .with_context(|| format!("Failed to load config from {path:?}"))
            } else if path.as_path() == std::path::Path::new("ralph.yml") {
                // Default path doesn't exist - this is fine, use defaults
                warn!("Config file 'ralph.yml' not found, using defaults");
                Ok(RalphConfig::default())
            } else {
                // User explicitly specified a config file that doesn't exist - this is an error
                Err(anyhow::anyhow!(
                    "Config file not found: {path:?}\n\nTo use default configuration, omit the -c/--config flag.\nTo see available presets, run: ralph init --list-presets"
                ))
            }
        }
        ConfigSource::Builtin(name) => {
            let preset = presets::get_preset(name).ok_or_else(|| {
                let available = presets::preset_names().join(", ");
                anyhow::anyhow!(
                    "Unknown preset '{name}'. Run `ralph init --list-presets` to see available presets.\n\nAvailable: {available}",
                )
            })?;
            RalphConfig::parse_yaml(preset.content)
                .with_context(|| format!("Failed to parse builtin preset '{name}'"))
        }
        ConfigSource::Remote(url) => Err(anyhow::anyhow!(
            "Remote config URLs are not supported for `ralph hats`.\n\nPlease use a local config file or builtin preset instead.\nURL: {url}"
        )),
    }
}

fn list_hats_json<W: Write>(writer: &mut W, registry: &HatRegistry) -> Result<()> {
    let hats: Vec<_> = registry.all().collect();
    serde_json::to_writer_pretty(&mut *writer, &hats)?;
    writeln!(writer)?;
    Ok(())
}

fn truncate_for_table(s: &str, max_chars: usize) -> String {
    if s.chars().count() <= max_chars {
        return s.to_string();
    }

    // NOTE: 用 char 级截断，避免对 UTF-8 字节切片导致 panic。
    let mut out: String = s.chars().take(max_chars).collect();
    out.push_str("...");
    out
}

fn list_hats<W: Write>(writer: &mut W, registry: &HatRegistry, _use_colors: bool) -> Result<()> {
    if registry.is_empty() {
        writeln!(
            writer,
            "No custom hats configured (using default HatlessRalph coordination)."
        )?;
        return Ok(());
    }

    writeln!(writer, "{:<20} DESCRIPTION", "HAT")?;
    writeln!(writer, "{}", "-".repeat(80))?;

    // Sort by name for consistent output
    let mut hats: Vec<_> = registry.all().collect();
    hats.sort_by(|a, b| a.name.cmp(&b.name));

    for hat in hats {
        let desc = if hat.description.is_empty() {
            "-".to_string()
        } else {
            truncate_for_table(&hat.description, 55)
        };

        writeln!(writer, "{:<20} {}", hat.name, desc)?;
    }
    Ok(())
}

fn validate_hats<W: Write>(
    writer: &mut W,
    config: &RalphConfig,
    registry: &HatRegistry,
    use_colors: bool,
) -> Result<()> {
    writeln!(writer, "Hat Topology Validation")?;
    writeln!(writer, "=======================")?;
    writeln!(writer)?;

    if registry.is_empty() {
        writeln!(writer, "No hats configured (solo mode).")?;
        return Ok(());
    }

    writeln!(writer, "Hats: {} configured", registry.len())?;
    if let Some(start) = &config.event_loop.starting_event {
        writeln!(writer, "Entry: task.start -> {}", start)?;
    } else {
        writeln!(writer, "Entry: task.start (Ralph coordinates)")?;
    }
    writeln!(writer)?;

    writeln!(writer, "Checks:")?;

    let mut warnings = 0;
    let mut errors = 0;

    // 1. Starting event validation
    if let Some(start) = &config.event_loop.starting_event {
        if registry.has_subscriber(start) {
            let hat = registry
                .get_for_topic(start)
                .expect("has_subscriber ensures presence");
            print_check(
                writer,
                CheckResult::Ok,
                &format!("Starting event '{start}' has subscriber ({})", hat.name),
                use_colors,
            )?;
        } else {
            print_check(
                writer,
                CheckResult::Error,
                &format!("starting_event '{start}' has no subscribers"),
                use_colors,
            )?;
            errors += 1;
        }
    }

    // 2. Orphan event detection (published but no subscribers)
    for hat in registry.all() {
        for pub_event in &hat.publishes {
            let topic = pub_event.as_str();

            // Ignore loop completion promise
            if topic == config.event_loop.completion_promise {
                continue;
            }

            // Ralph conceptually subscribes to everything as fallback, but we still warn if no SPECIFIC hat handles it.
            if !registry.has_subscriber(topic) {
                print_check(
                    writer,
                    CheckResult::Warn,
                    &format!(
                        "Event '{topic}' published by '{}' has no hat subscribers",
                        hat.name
                    ),
                    use_colors,
                )?;
                warnings += 1;
            }
        }
    }

    // 3. Dead end detection
    let mut dead_end_hats: Vec<String> = Vec::new();
    for hat in registry.all() {
        if hat.publishes.is_empty() {
            dead_end_hats.push(hat.name.clone());
        }
    }

    if dead_end_hats.is_empty() {
        print_check(writer, CheckResult::Ok, "No dead-end hats", use_colors)?;
    } else {
        dead_end_hats.sort();
        for name in dead_end_hats {
            print_check(
                writer,
                CheckResult::Warn,
                &format!("Hat '{name}' publishes nothing (dead end)"),
                use_colors,
            )?;
            warnings += 1;
        }
    }

    writeln!(writer)?;
    if errors > 0 {
        writeln!(
            writer,
            "Result: Invalid ({errors} errors, {warnings} warnings)"
        )?;
        // Return error to propagate failure to main
        return Err(anyhow::anyhow!("Validation failed with {errors} errors"));
    }

    if warnings > 0 {
        writeln!(writer, "Result: Valid ({warnings} warnings)")?;
    } else {
        writeln!(writer, "Result: Valid")?;
    }
    Ok(())
}

pub(crate) enum CheckResult {
    Ok,
    Warn,
    Error,
}

pub(crate) fn print_check<W: Write>(
    writer: &mut W,
    result: CheckResult,
    msg: &str,
    use_colors: bool,
) -> Result<()> {
    if use_colors {
        match result {
            CheckResult::Ok => {
                writeln!(writer, "  [{}ok{}] {}", colors::GREEN, colors::RESET, msg)?
            }
            CheckResult::Warn => writeln!(
                writer,
                "  [{}warn{}] {}",
                colors::YELLOW,
                colors::RESET,
                msg
            )?,
            CheckResult::Error => {
                writeln!(writer, "  [{}err{}] {}", colors::RED, colors::RESET, msg)?
            }
        }
    } else {
        match result {
            CheckResult::Ok => writeln!(writer, "  [ok] {}", msg)?,
            CheckResult::Warn => writeln!(writer, "  [warn] {}", msg)?,
            CheckResult::Error => writeln!(writer, "  [err] {}", msg)?,
        }
    }
    Ok(())
}

fn graph_hats<W: Write>(
    writer: &mut W,
    config: &RalphConfig,
    registry: &HatRegistry,
    format: GraphFormat,
    view: GraphView,
) -> Result<()> {
    match format {
        GraphFormat::Mermaid => {
            writeln!(writer, "```mermaid")?;
            write!(
                writer,
                "{}",
                generate_mermaid_string(config, registry, view, MermaidLabelMode::Strict)
            )?;
            writeln!(writer, "```")?;
        }
        GraphFormat::Unicode | GraphFormat::Ascii | GraphFormat::Compact => {
            let rendered = render_hat_dag_via_mermaid(config, registry, format, view)?;
            write!(writer, "{rendered}")?;
        }
    }
    Ok(())
}

// ============================================================================
// ASCII/Unicode 渲染选项构造器
//
// 设计意图:
// - 统一 `AsciiRenderOptions` 初始化入口,避免散落的字面量初始化在字段演进时漏改;
// - 当上游新增字段时,基于 `Default::default()` 再按需覆盖字段,可自动承接默认值,降低 E0063 风险。
// ============================================================================
fn unicode_render_options() -> AsciiRenderOptions {
    let mut options = AsciiRenderOptions::default();
    options.use_ascii = Some(false);
    options
}

fn ascii_render_options() -> AsciiRenderOptions {
    let mut options = AsciiRenderOptions::default();
    options.use_ascii = Some(true);
    options
}

fn compact_unicode_render_options() -> AsciiRenderOptions {
    let mut options = AsciiRenderOptions::default();
    options.use_ascii = Some(false);
    options.padding_x = Some(0);
    // 备注:
    // - 方向已统一为 TD,physical view 常见回边(backlink).
    // - `padding_y=0` 在某些图形下会触发渲染器异常,因此这里保留最小垂直间距.
    options.padding_y = Some(1);
    options.box_border_padding = Some(0);
    options
}

fn render_hat_dag_via_mermaid(
    config: &RalphConfig,
    registry: &HatRegistry,
    format: GraphFormat,
    view: GraphView,
) -> Result<String> {
    if registry.is_empty() {
        return Ok("No hats configured.\n".to_string());
    }

    let diagram = generate_mermaid_string(config, registry, view, MermaidLabelMode::TerminalPretty);
    let options = match format {
        GraphFormat::Unicode => unicode_render_options(),
        GraphFormat::Ascii => ascii_render_options(),
        GraphFormat::Compact => compact_unicode_render_options(),
        GraphFormat::Mermaid => AsciiRenderOptions::default(),
    };

    let mut rendered = render_mermaid_ascii(&diagram, &options)
        .with_context(|| "Failed to render Mermaid topology as ASCII/Unicode".to_string())?;

    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    Ok(rendered)
}

/// 为 TUI 的右上角 Radar 面板渲染 hats graph（文本 Mermaid：默认 Unicode 线条字符）。
///
/// 返回：
/// - `(ascii_compact, ascii_full)`
///
/// 说明：
/// - 两份字符串都带末尾 `\n`，便于直接展示/拼接；
/// - compact 视图会尽量压缩 padding,更适合小窗"雷达".
/// - full 视图使用更可读的默认渲染参数。
/// - 这里“ascii”指的是“文字图（ASCII/Unicode）”的渲染模式：
///   - 对齐 `beautiful-mermaid-rs --ascii` 的默认行为：使用 Unicode box-drawing 字符（┌─┐│└┘▶）
///   - 而不是强制纯 ASCII（+--|），后者更接近 `--ascii --use-ascii`。
pub(crate) fn render_hat_graph_radar_ascii(
    config: &RalphConfig,
    registry: &HatRegistry,
) -> Result<HatGraphRadar> {
    if registry.is_empty() {
        let empty = "No hats configured.\n".to_string();
        return Ok(HatGraphRadar {
            ascii_compact: empty.clone(),
            ascii_full: empty,
            meta_compact: None,
            meta_full: None,
        });
    }

    // Radar 面板与 `ralph hats graph` 对齐：默认展示 physical view（包含 coordinator）。
    // 这样在 coordinator-driven workflow（并行 supervisor）里，Radar 不会再“断开”。
    let diagram = generate_mermaid_string(
        config,
        registry,
        GraphView::Physical,
        MermaidLabelMode::TerminalPretty,
    );

    // 重要:
    // - physical view 通常存在 "Hat -> ralph#1" 的回边(backlink).
    // - 在 TD 方向下,`padding_y=0` 容易让渲染器找不到可走的路径并触发 QuickJS exception.
    // - 构造器里保留水平紧凑(padding_x=0),并给垂直方向最小余量(padding_y=1).
    let compact_options = compact_unicode_render_options();

    // 说明:
    // - `beautiful-mermaid-rs` 的 meta 渲染在某些布局下会触发 QuickJS exception(已知不稳定点).
    // - Radar 属于 best-effort 能力,meta 失败时降级为"仅文字图"(不影响主流程).
    let (mut ascii_compact, meta_compact) =
        match render_mermaid_ascii_with_meta(&diagram, &compact_options) {
            Ok(compact) => (
                compact.text,
                Some(convert_ascii_meta_to_radar_meta(compact.meta)),
            ),
            Err(e) => {
                warn!(
                    "Hat graph radar: compact meta render failed (falling back to text-only): {e:#}"
                );

                let text = render_mermaid_ascii(&diagram, &compact_options).with_context(|| {
                    "Failed to render Mermaid topology (compact) as Unicode text diagram"
                        .to_string()
                })?;

                (text, None)
            }
        };
    if !ascii_compact.ends_with('\n') {
        ascii_compact.push('\n');
    }

    let full_options = unicode_render_options();

    let (mut ascii_full, meta_full) = match render_mermaid_ascii_with_meta(&diagram, &full_options)
    {
        Ok(full) => (full.text, Some(convert_ascii_meta_to_radar_meta(full.meta))),
        Err(e) => {
            warn!("Hat graph radar: full meta render failed (falling back to text-only): {e:#}");

            let text = render_mermaid_ascii(&diagram, &full_options).with_context(|| {
                "Failed to render Mermaid topology (full) as Unicode text diagram".to_string()
            })?;

            (text, None)
        }
    };
    if !ascii_full.ends_with('\n') {
        ascii_full.push('\n');
    }

    Ok(HatGraphRadar {
        ascii_compact,
        ascii_full,
        meta_compact,
        meta_full,
    })
}

fn i32_to_u16_saturating(value: i32) -> u16 {
    if value <= 0 {
        0
    } else {
        (value as u32).min(u32::from(u16::MAX)) as u16
    }
}

fn densify_hat_graph_radar_path(path: Vec<HatGraphRadarPoint>) -> Vec<HatGraphRadarPoint> {
    // =========================================================================
    // Hat Graph Radar：edge.path 补点（关键点 -> 连续线段）
    //
    // 背景：
    // - `beautiful-mermaid-rs` 的 edge.meta.path 语义是“关键格子”（拐点/箭头等），
    //   并不保证包含线段上的每一个 cell。
    // - TUI 如果直接按关键点上色，会出现“只亮到一半/像断线”的观感。
    //
    // 策略：
    // - 对相邻点之间的水平/垂直段做逐 cell 补齐（保留顺序）；
    // - 如果遇到非正交段（理论上不该出现），保守地只连接关键点，避免引入错误路径。
    // =========================================================================
    if path.len() <= 1 {
        return path;
    }

    let mut dense: Vec<HatGraphRadarPoint> = Vec::new();
    let mut prev = path[0];
    dense.push(prev);

    for next in path.into_iter().skip(1) {
        let dx = i32::from(next.x) - i32::from(prev.x);
        let dy = i32::from(next.y) - i32::from(prev.y);

        if dx == 0 && dy != 0 {
            let step = dy.signum();
            let mut y = i32::from(prev.y);
            while y != i32::from(next.y) {
                y += step;
                dense.push(HatGraphRadarPoint {
                    x: prev.x,
                    y: y as u16,
                });
            }
        } else if dy == 0 && dx != 0 {
            let step = dx.signum();
            let mut x = i32::from(prev.x);
            while x != i32::from(next.x) {
                x += step;
                dense.push(HatGraphRadarPoint {
                    x: x as u16,
                    y: prev.y,
                });
            }
        } else {
            dense.push(next);
        }

        prev = next;
    }

    // 防御性去重：避免 renderer 输出重复关键点导致上层 animation “卡顿”。
    dense.dedup_by(|a, b| a.x == b.x && a.y == b.y);
    dense
}

fn convert_ascii_meta_to_radar_meta(
    meta: beautiful_mermaid_rs::AsciiRenderMeta,
) -> HatGraphRadarMeta {
    let nodes = meta
        .nodes
        .into_iter()
        .map(|node| HatGraphRadarNodeMeta {
            id: node.id,
            label: node.label,
            box_rect: HatGraphRadarRect {
                x: i32_to_u16_saturating(node.box_rect.x),
                y: i32_to_u16_saturating(node.box_rect.y),
                width: i32_to_u16_saturating(node.box_rect.width),
                height: i32_to_u16_saturating(node.box_rect.height),
            },
        })
        .collect();

    let edges = meta
        .edges
        .into_iter()
        .map(|edge| {
            let path = edge
                .path
                .into_iter()
                .map(|p| HatGraphRadarPoint {
                    x: i32_to_u16_saturating(p.x),
                    y: i32_to_u16_saturating(p.y),
                })
                .collect();

            HatGraphRadarEdgeMeta {
                from: edge.from,
                to: edge.to,
                label: edge.label,
                path: densify_hat_graph_radar_path(path),
            }
        })
        .collect();

    HatGraphRadarMeta { nodes, edges }
}

/// Generate Mermaid flowchart syntax for the hat topology.
fn generate_mermaid_string(
    config: &RalphConfig,
    registry: &HatRegistry,
    view: GraphView,
    label_mode: MermaidLabelMode,
) -> String {
    match view {
        GraphView::Logical => generate_mermaid_string_logical(config, registry, label_mode),
        GraphView::Physical => generate_mermaid_string_physical(config, registry, label_mode),
    }
}

/// Generate Mermaid flowchart syntax for the hat topology (logical view).
///
/// 说明：
/// - 逻辑视图只画 Hat→Hat 的传播关系（隐藏 ralph#1/coordinator）；
/// - 入口/结束节点按 spec 规则可选输出（Start/Complete）。
fn generate_mermaid_string_logical(
    config: &RalphConfig,
    registry: &HatRegistry,
    label_mode: MermaidLabelMode,
) -> String {
    // NOTE:
    // - Mermaid 的“节点 ID”在不同渲染器里兼容性差异很大；
    //   目前我们发现：当节点 ID 使用中文/emoji 时，`beautiful-mermaid-rs` 会吞掉边/节点，
    //   导致 `ralph hats graph` 只剩下 task.start→Ralph。
    // - 因此这里把“节点 ID”和“节点展示名（label）”分离：
    //   - ID：稳定 + ASCII 安全（hat.id）
    //   - label：保留中文/emoji（hat.name）
    // - HatRegistry 内部是 HashMap，迭代顺序不稳定。为了让输出尽量确定性，这里先按 hat.id 排序。
    let mut hats: Vec<_> = registry.all().collect();
    hats.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

    let mut output = String::new();
    // Mermaid 方向:
    // - 你要求 ASCII/Unicode/Radar 与 `--format mermaid` 保持一致方向.
    // - 因此这里统一使用 TD(Top-Down),避免方向分叉带来的理解成本.
    output.push_str("flowchart TD\n");

    // 先声明节点（含 label），避免后续边只引用 ID 时渲染器把 label 丢成裸 ID。
    for hat in &hats {
        let node_id = mermaid_hat_node_id(hat.id.as_str());
        let label_source = if hat.name.trim().is_empty() {
            hat.id.as_str()
        } else {
            &hat.name
        };
        let label = format_mermaid_node_label(label_source, label_mode);
        output.push_str(&format!("    {node_id}[{label}]\n"));
    }

    // （逻辑视图）入口边：当显式设置 starting_event 时，展示 task.start -> starting_event -> hats
    if let Some(starting_event) = config.event_loop.starting_event.as_deref() {
        let mut targets: Vec<String> = hats
            .iter()
            .filter(|hat| {
                hat.subscriptions
                    .iter()
                    .any(|s| s.as_str() == starting_event)
            })
            .map(|hat| mermaid_hat_node_id(hat.id.as_str()))
            .collect();

        targets.sort();
        targets.dedup();

        // 如果配置了 starting_event，但没有任何订阅者，这本身就应在 `ralph hats validate` 里报错。
        // 这里仍然把 Start 节点输出出来，便于用户一眼发现“入口没有接到任何 hat”。
        output.push_str("    Start[task.start]\n");
        for target_id in targets {
            output.push_str(&format!(
                "    Start -->|{}| {}\n",
                starting_event, target_id
            ));
        }
    }

    // （逻辑视图）结束节点：当显式设置 complete_publishes 时，把它画成一个固定终点节点。
    //
    // 备注：
    // - complete_publishes 是“工作流完成候选事件”，可能没有任何 hat 订阅。
    //   如果我们只靠 Hat→Hat 的订阅关系推导，它会在图上直接消失。
    if let Some(complete_topic) = config.event_loop.complete_publishes.as_deref() {
        output.push_str("    Complete[complete]\n");

        let mut complete_sources: Vec<String> = hats
            .iter()
            .filter(|hat| hat.publishes.iter().any(|p| p.as_str() == complete_topic))
            .map(|hat| mermaid_hat_node_id(hat.id.as_str()))
            .collect();
        complete_sources.sort();
        complete_sources.dedup();

        for source_id in complete_sources {
            output.push_str(&format!(
                "    {} -->|{}| Complete\n",
                source_id, complete_topic
            ));
        }
    }

    // Hat -> Hat（逻辑视图）：A publishes topic，且 B subscribes topic，则展示 A -->|topic| B
    //
    // 备注：
    // - 虽然运行时是通过 Ralph 调度，但图里我们只展示“对用户有意义”的逻辑连线。
    // - 这里会按 (source, topic, target) 去重 + 排序，保证输出稳定可预测。
    let mut edges: Vec<(String, String, String)> = Vec::new();
    for source in &hats {
        for pub_event in &source.publishes {
            let topic = pub_event.as_str();
            for target in &hats {
                if target.id == source.id {
                    continue;
                }
                if target.subscriptions.iter().any(|s| s.as_str() == topic) {
                    edges.push((
                        mermaid_hat_node_id(source.id.as_str()),
                        topic.to_string(),
                        mermaid_hat_node_id(target.id.as_str()),
                    ));
                }
            }
        }
    }

    edges.sort();
    edges.dedup();
    for (source_id, topic, target_id) in edges {
        output.push_str(&format!("    {} -->|{}| {}\n", source_id, topic, target_id));
    }

    output
}

/// Generate Mermaid flowchart syntax for the hat topology (physical view).
///
/// 目标：
/// - 提供一个“更贴近真实运行时路由”的视角：
///   - coordinator（ralph#1）显式出现在图中；
///   - 仅在“边界 topic”（无内部发布者/无内部订阅者）上画 Ralph↔Hat 边，
///     避免回到“全连接噪声”。
fn generate_mermaid_string_physical(
    config: &RalphConfig,
    registry: &HatRegistry,
    label_mode: MermaidLabelMode,
) -> String {
    // 重要：coordinator 节点的 Mermaid ID 必须与 TUI 的 `mermaid_hat_node_id("ralph")` 对齐。
    //
    // 原因：
    // - Radar 的因果边动画依赖 meta（from/to/label）做结构匹配；
    // - 并行/串行事件的 `event.source` 通常是 hat_id（例如 "ralph"），TUI 会把它映射成 `Hat_ralph`；
    // - 如果我们在 Mermaid 里用的是另一个 coordinator 节点 ID（例如 "Ralph"），Radar 就会匹配不到边。
    let ralph_node_id = mermaid_hat_node_id("ralph");

    let mut hats: Vec<_> = registry.all().collect();
    hats.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

    let mut output = String::new();
    // Mermaid 方向说明见 `generate_mermaid_string_logical`.
    output.push_str("flowchart TD\n");

    // =========================================================================
    // Mermaid 布局稳定性：优先声明 ralph#1（coordinator）
    //
    // 经验结论：
    // - `beautiful-mermaid-rs`（dagre/flowchart）对“节点声明顺序”比较敏感；
    // - 如果先声明所有 hats、最后才声明 `ralph#1`，在 Unicode/ASCII 渲染里 coordinator
    //   往往会被挤到图的右侧或中下方，不符合“调度员应在起点位置”的直觉；
    // - 将 coordinator 节点放到 Mermaid 文本的最前面，可以在不改变拓扑语义的前提下，
    //   让 `ralph#1` 更稳定地出现在图的左/上方（best-effort）。
    // =========================================================================
    // coordinator 节点（物理视图专有）
    let ralph_label = format_mermaid_node_label("ralph#1 (coordinator)", label_mode);
    output.push_str(&format!("    {ralph_node_id}[{ralph_label}]\n"));

    // 先声明节点（含 label），避免后续边只引用 ID 时渲染器把 label 丢成裸 ID。
    for hat in &hats {
        let node_id = mermaid_hat_node_id(hat.id.as_str());
        let label_source = if hat.name.trim().is_empty() {
            hat.id.as_str()
        } else {
            &hat.name
        };
        let label = format_mermaid_node_label(label_source, label_mode);
        output.push_str(&format!("    {node_id}[{label}]\n"));
    }

    // 控制面握手：fresh run 的起点永远是 task.start（见 event semantics spec）
    output.push_str("    Start[task.start]\n");
    output.push_str(&format!("    Start --> {ralph_node_id}\n"));

    // topic 集合（按“精确字符串”聚合），用于判定哪些 topic 是“边界 topic”。
    let mut published_exact: HashSet<String> = HashSet::new();
    let mut subscribed_exact: HashSet<String> = HashSet::new();
    for hat in &hats {
        for t in &hat.publishes {
            published_exact.insert(t.as_str().to_string());
        }
        for t in &hat.subscriptions {
            subscribed_exact.insert(t.as_str().to_string());
        }
    }

    // 物理视图边集合：
    // - 仍然保留 Hat→Hat 的逻辑边（内部 topic）；
    // - 仅对“无内部发布者/无内部订阅者”的边界 topic，补 Ralph↔Hat 边。
    let mut edges: Vec<(String, String, String)> = Vec::new();

    // Hat -> Hat（内部 topic）：与 logical view 一致
    for source in &hats {
        for pub_event in &source.publishes {
            let topic = pub_event.as_str();
            for target in &hats {
                if target.id == source.id {
                    continue;
                }
                if target.subscriptions.iter().any(|s| s.as_str() == topic) {
                    edges.push((
                        mermaid_hat_node_id(source.id.as_str()),
                        topic.to_string(),
                        mermaid_hat_node_id(target.id.as_str()),
                    ));
                }
            }
        }
    }

    // Ralph -> Hat：Hat 订阅了某个 topic，但没有任何 hat 发布该 topic
    for target in &hats {
        let target_id = mermaid_hat_node_id(target.id.as_str());
        for sub in &target.subscriptions {
            let topic = sub.as_str();
            if !published_exact.contains(topic) {
                edges.push((ralph_node_id.clone(), topic.to_string(), target_id.clone()));
            }
        }
    }

    // Hat -> Ralph：Hat 发布了某个 topic，但没有任何 hat 订阅该 topic
    for source in &hats {
        let source_id = mermaid_hat_node_id(source.id.as_str());
        for pub_event in &source.publishes {
            let topic = pub_event.as_str();
            if !subscribed_exact.contains(topic) {
                edges.push((source_id.clone(), topic.to_string(), ralph_node_id.clone()));
            }
        }
    }

    // complete_publishes：保持逻辑视图的“Complete[complete]”可视锚点；
    // 若没有任何 hat 发布该 topic，则认为它由 ralph#1 负责发布（并在下一轮观察到它再输出 LOOP_COMPLETE）。
    if let Some(complete_topic) = config.event_loop.complete_publishes.as_deref() {
        output.push_str("    Complete[complete]\n");

        let mut complete_sources: Vec<String> = hats
            .iter()
            .filter(|hat| hat.publishes.iter().any(|p| p.as_str() == complete_topic))
            .map(|hat| mermaid_hat_node_id(hat.id.as_str()))
            .collect();
        complete_sources.sort();
        complete_sources.dedup();

        if complete_sources.is_empty() {
            edges.push((
                ralph_node_id.clone(),
                complete_topic.to_string(),
                "Complete".to_string(),
            ));
        } else {
            for source_id in complete_sources {
                edges.push((
                    source_id,
                    complete_topic.to_string(),
                    "Complete".to_string(),
                ));
            }
        }
    }

    edges.sort();
    edges.dedup();

    // 物理视图边策略:
    // - 无论 Strict 还是 TerminalPretty,都保持“一 topic 一条边”。
    // - 这样可避免边标签过长导致图被横向拉宽,并与 `--format mermaid` 表现一致。
    let mut final_edges: Vec<(String, String, String)> = edges;

    final_edges.sort();
    final_edges.dedup();
    for (from, topic, to) in final_edges {
        output.push_str(&format!("    {from} -->|{topic}| {to}\n"));
    }

    output
}

fn mermaid_hat_node_id(hat_id: &str) -> String {
    // 加前缀是为了：
    // - 避免 hat.id 恰好叫 "Start"/"Ralph" 这类节点名导致冲突
    // - 避免 hat.id 以数字开头时触发 Mermaid 的标识符解析歧义
    format!("Hat_{}", sanitize_mermaid_identifier(hat_id))
}

fn sanitize_mermaid_identifier(raw: &str) -> String {
    // Mermaid 的标识符规则在不同渲染器/版本里存在差异。
    // 这里保守地只允许 ASCII [A-Za-z0-9_]，其余字符全部移除。
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '_' {
            out.push(c);
        }
    }

    if out.is_empty() {
        "hat".to_string()
    } else {
        out
    }
}

fn escape_mermaid_label(label: &str) -> String {
    // Mermaid 支持用 `["..."]` 作为 label，这里做最小必要转义：
    // - `\` 与 `"` 会影响字符串边界
    // - `\n` 统一转为 `\\n`，避免破坏语法（也更利于 ASCII 渲染器）
    label
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MermaidLabelMode {
    /// 标准 Mermaid 输出（面向 `--format mermaid`）：
    /// - 优先保证 `mermaid-cli` / 浏览器 Mermaid 可解析；
    /// - 必要时会为 label 加上双引号。
    Strict,

    /// 终端渲染优化（面向 ASCII/Unicode 图）：
    /// - 尽量不加引号，避免 `beautiful-mermaid-rs` 把引号当作内容直接画出来；
    /// - 只在“无法不加引号就会破坏语法”的情况下才加引号。
    TerminalPretty,
}

fn format_mermaid_node_label(label: &str, mode: MermaidLabelMode) -> String {
    // Mermaid 的 `Node[label]` 语法对可用字符非常敏感：
    // - `]` 与换行会直接破坏语法；
    // - `(` / `)` 在标准 Mermaid 解析器里也会触发歧义（会被当作“形状语法”的 token）。
    //
    // 这里做一个“按目标渲染器分层”的策略：
    // - Strict：确保标准 Mermaid 解析器可用（`--format mermaid` 输出必须可复制可渲染）。
    // - TerminalPretty：尽量保持终端图的可读性（少引号），但仍保证基本语法不破坏。
    let requires_quotes = label.contains(']')
        || label.contains('\n')
        || (mode == MermaidLabelMode::Strict && (label.contains('(') || label.contains(')')));

    if requires_quotes {
        format!("\"{}\"", escape_mermaid_label(label))
    } else {
        label.to_string()
    }
}

fn show_hat<W: Write>(
    writer: &mut W,
    registry: &HatRegistry,
    name: &str,
    use_colors: bool,
) -> Result<()> {
    // Try to find by ID first, then by display name
    let hat = registry
        .all()
        .find(|h| h.id.as_str() == name || h.name == name);

    let hat = hat.context(format!("Hat '{name}' not found"))?;

    if use_colors {
        writeln!(writer, "{}{}{}", colors::BOLD, hat.name, colors::RESET)?;
    } else {
        writeln!(writer, "{}", hat.name)?;
    }

    if !hat.description.is_empty() {
        writeln!(writer, "{}", hat.description)?;
    }
    writeln!(writer)?;

    writeln!(writer, "ID: {}", hat.id)?;

    writeln!(writer, "\nTriggers On:")?;
    if hat.subscriptions.is_empty() {
        writeln!(writer, "  (none)")?;
    } else {
        for trigger in &hat.subscriptions {
            writeln!(writer, "  - {}", trigger.as_str())?;
        }
    }

    writeln!(writer, "\nPublishes:")?;
    if hat.publishes.is_empty() {
        writeln!(writer, "  (none)")?;
    } else {
        for topic in &hat.publishes {
            writeln!(writer, "  - {}", topic.as_str())?;
        }
    }

    if !hat.instructions.is_empty() {
        writeln!(writer, "\nInstructions:")?;
        for line in hat.instructions.lines() {
            writeln!(writer, "  {}", line)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_proto::Hat;

    fn mock_hat(name: &str, subs: &[&str], pubs: &[&str]) -> Hat {
        let mut hat = Hat::new(sanitize_mermaid_identifier(name), name);
        hat.description = format!("Description for {name}");
        hat.subscriptions = subs.iter().map(|s| (*s).into()).collect();
        hat.publishes = pubs.iter().map(|s| (*s).into()).collect();
        hat
    }

    #[test]
    fn test_sanitize_mermaid_identifier() {
        assert_eq!(sanitize_mermaid_identifier("My Hat"), "MyHat");
        assert_eq!(sanitize_mermaid_identifier("cool-hat"), "coolhat");
        assert_eq!(sanitize_mermaid_identifier("Hat!@#"), "Hat");
        assert_eq!(sanitize_mermaid_identifier("123"), "123");
        assert_eq!(sanitize_mermaid_identifier("___"), "___");
    }

    #[test]
    fn test_ascii_render_option_builders_are_default_plus_overrides() {
        // 回归测试:
        // - 统一构造函数必须保留关键参数,并让“新增字段”走默认值承接;
        // - 用 “Default + 覆盖” 的形式,可以天然承接未来字段扩展,避免 E0063。
        let unicode = unicode_render_options();
        let mut expected_unicode = AsciiRenderOptions::default();
        expected_unicode.use_ascii = Some(false);
        assert_eq!(unicode, expected_unicode);

        let ascii = ascii_render_options();
        let mut expected_ascii = AsciiRenderOptions::default();
        expected_ascii.use_ascii = Some(true);
        assert_eq!(ascii, expected_ascii);

        let compact = compact_unicode_render_options();
        let mut expected_compact = AsciiRenderOptions::default();
        expected_compact.use_ascii = Some(false);
        expected_compact.padding_x = Some(0);
        expected_compact.padding_y = Some(1);
        expected_compact.box_border_padding = Some(0);
        assert_eq!(compact, expected_compact);
    }

    #[test]
    fn densify_hat_graph_radar_path_fills_horizontal_and_vertical_segments() {
        // 说明：
        // - edge.meta.path 可能只给“关键点”，TUI 需要可连续上色的 cell path；
        // - 这里验证：水平/垂直线段会被补齐到逐 cell 的连续序列。

        let horizontal = densify_hat_graph_radar_path(vec![
            HatGraphRadarPoint { x: 0, y: 0 },
            HatGraphRadarPoint { x: 3, y: 0 },
        ]);
        assert_eq!(
            horizontal,
            vec![
                HatGraphRadarPoint { x: 0, y: 0 },
                HatGraphRadarPoint { x: 1, y: 0 },
                HatGraphRadarPoint { x: 2, y: 0 },
                HatGraphRadarPoint { x: 3, y: 0 },
            ]
        );

        let vertical_reverse = densify_hat_graph_radar_path(vec![
            HatGraphRadarPoint { x: 2, y: 3 },
            HatGraphRadarPoint { x: 2, y: 1 },
        ]);
        assert_eq!(
            vertical_reverse,
            vec![
                HatGraphRadarPoint { x: 2, y: 3 },
                HatGraphRadarPoint { x: 2, y: 2 },
                HatGraphRadarPoint { x: 2, y: 1 },
            ]
        );

        let non_orthogonal = densify_hat_graph_radar_path(vec![
            HatGraphRadarPoint { x: 0, y: 0 },
            HatGraphRadarPoint { x: 1, y: 1 },
        ]);
        assert_eq!(
            non_orthogonal,
            vec![
                HatGraphRadarPoint { x: 0, y: 0 },
                HatGraphRadarPoint { x: 1, y: 1 },
            ]
        );
    }

    #[test]
    fn test_list_hats_empty() {
        let registry = HatRegistry::new();
        let mut buf = Vec::new();
        list_hats(&mut buf, &registry, false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("No custom hats configured"));
    }

    #[test]
    fn test_list_hats_with_entries() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));
        registry.register(mock_hat("Planner", &["plan.start"], &["build.task"]));

        let mut buf = Vec::new();
        list_hats(&mut buf, &registry, false).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("HAT                  DESCRIPTION"));
        assert!(output.contains("Builder"));
        assert!(output.contains("Planner"));
    }

    #[test]
    fn test_validate_hats_orphan() {
        let mut registry = HatRegistry::new();
        // Builder publishes build.done, but no one listens
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));

        let config = RalphConfig::default();
        let mut buf = Vec::new();

        validate_hats(&mut buf, &config, &registry, false).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // Should warn about build.done having no subscribers
        assert!(
            output.contains("Event 'build.done' published by 'Builder' has no hat subscribers")
        );
        assert!(output.contains("Result: Valid"));
    }

    #[test]
    fn test_graph_hats_mermaid() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let config = RalphConfig::default();
        let mut buf = Vec::new();

        graph_hats(
            &mut buf,
            &config,
            &registry,
            GraphFormat::Mermaid,
            GraphView::Logical,
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("flowchart TD"));
        assert!(output.contains("Hat_A -->|mid| Hat_B"));
        assert!(!output.contains("Ralph"));
        assert!(!output.contains("-.->"));
    }

    #[test]
    fn test_graph_hats_ascii() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));

        let config = RalphConfig::default();
        let mut buf = Vec::new();

        graph_hats(
            &mut buf,
            &config,
            &registry,
            GraphFormat::Ascii,
            GraphView::Logical,
        )
        .unwrap();
        let output = String::from_utf8(buf).unwrap();

        // deterministic output should contain key node names
        assert!(output.contains("Builder"));
    }

    #[test]
    fn test_render_hat_graph_radar_uses_unicode_box_drawing() {
        // 回归测试：
        // - Hat Graph Radar 的“文字图”输出应对齐 `beautiful-mermaid-rs --ascii` 默认行为；
        // - 即使用 Unicode box-drawing 字符（┌─┐│└┘▶），而不是强制纯 ASCII（+--|）。
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));

        let config = RalphConfig::default();
        let radar = render_hat_graph_radar_ascii(&config, &registry).unwrap();

        fn contains_box_drawing(s: &str) -> bool {
            s.chars()
                .any(|c| matches!(c, '┌' | '─' | '│' | '└' | '┘' | '▶'))
        }

        assert!(
            contains_box_drawing(&radar.ascii_compact),
            "expected radar compact output to contain Unicode box-drawing characters"
        );
        assert!(
            contains_box_drawing(&radar.ascii_full),
            "expected radar full output to contain Unicode box-drawing characters"
        );
    }

    #[test]
    fn test_generate_mermaid_string() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Logical,
            MermaidLabelMode::Strict,
        );

        assert!(output.contains("flowchart TD"));
        assert!(output.contains("Hat_A -->|mid| Hat_B"));
        assert!(!output.contains("Ralph"));
        assert!(!output.contains("-.->"));
    }

    #[test]
    fn test_generate_mermaid_string_terminal_pretty_uses_flowchart_td() {
        // 回归测试:
        // - ASCII/Unicode/Radar 的渲染链路使用 `TerminalPretty` 生成 Mermaid 源.
        // - 你要求方向也必须是 TD,因此这里锁死 `flowchart TD`.
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Logical,
            MermaidLabelMode::TerminalPretty,
        );

        assert!(output.contains("flowchart TD"));
        assert!(!output.contains("flowchart LR"));
    }

    #[test]
    fn test_generate_mermaid_string_strict_quotes_parentheses_in_node_labels() {
        // 回归测试（Strict Mermaid 输出）：
        // - 标准 Mermaid 解析器不接受 `Node[label (x)]` 这种“未加引号的括号”写法；
        // - 因此只要 label 里含 `(` / `)`，我们就必须输出成 `Node["label (x)"]`。
        let mut registry = HatRegistry::new();

        let hat = Hat::new("hat_a", "A (primary)");
        registry.register(hat);

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Logical,
            MermaidLabelMode::Strict,
        );

        assert!(output.contains("Hat_hat_a[\"A (primary)\"]"));
    }

    #[test]
    fn test_generate_mermaid_string_includes_complete_publishes() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let mut config = RalphConfig::default();
        config.event_loop.complete_publishes = Some("end".to_string());

        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Logical,
            MermaidLabelMode::Strict,
        );

        assert!(output.contains("Complete[complete]"));
        assert!(output.contains("Hat_B -->|end| Complete"));
    }

    #[test]
    fn test_generate_mermaid_string_physical_view_adds_ralph_boundary_edges() {
        // 回归测试（physical view）：
        // - 当某个 topic 只有订阅者（没有 hat 发布者）时，应从 coordinator（Hat_ralph）画到该 hat；
        // - 当 complete_publishes 没有 hat 发布者时，应从 coordinator（Hat_ralph）画到 Complete。
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Runner", &["work.task"], &["work.result"]));

        let mut config = RalphConfig::default();
        config.event_loop.complete_publishes = Some("work.complete".to_string());

        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Physical,
            MermaidLabelMode::Strict,
        );

        assert!(output.contains("Hat_ralph[\"ralph#1 (coordinator)\"]"));
        assert!(output.contains("Start[task.start]"));
        assert!(output.contains("Start --> Hat_ralph"));
        assert!(output.contains("Hat_ralph -->|work.task| Hat_Runner"));
        assert!(output.contains("Hat_ralph -->|work.complete| Complete"));
    }

    #[test]
    fn test_generate_mermaid_string_physical_strict_does_not_collapse_ralph_topics() {
        // 回归测试（Strict Mermaid 输出）：
        // - 目标是给标准 Mermaid 渲染器消费，可读性优先；
        // - Ralph 相关多 topic 不应折叠为一条超长 label，避免图被横向拉宽。
        let mut registry = HatRegistry::new();
        registry.register(mock_hat(
            "experiment_integrator",
            &["integration.task"],
            &[
                "experiment.complete",
                "integration.applied",
                "integration.blocked",
                "integration.rejected",
            ],
        ));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Physical,
            MermaidLabelMode::Strict,
        );

        assert!(output.contains("Hat_experiment_integrator -->|experiment.complete| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.applied| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.blocked| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.rejected| Hat_ralph"));
        assert!(!output.contains(
            "Hat_experiment_integrator -->|experiment.complete / integration.applied / integration.blocked / integration.rejected| Hat_ralph"
        ));
    }

    #[test]
    fn test_generate_mermaid_string_physical_terminal_pretty_does_not_collapse_ralph_topics() {
        // 回归测试（TerminalPretty）：
        // - 你要求默认 `ralph hats graph` 也不要合并边。
        // - 因此 TerminalPretty 下也必须保持一 topic 一条边。
        let mut registry = HatRegistry::new();
        registry.register(mock_hat(
            "experiment_integrator",
            &["integration.task"],
            &[
                "experiment.complete",
                "integration.applied",
                "integration.blocked",
                "integration.rejected",
            ],
        ));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Physical,
            MermaidLabelMode::TerminalPretty,
        );

        assert!(output.contains("Hat_experiment_integrator -->|experiment.complete| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.applied| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.blocked| Hat_ralph"));
        assert!(output.contains("Hat_experiment_integrator -->|integration.rejected| Hat_ralph"));
        assert!(!output.contains(" / +3 more"));
        assert!(!output.contains(
            "Hat_experiment_integrator -->|experiment.complete / integration.applied / integration.blocked / integration.rejected| Hat_ralph"
        ));
    }

    #[test]
    fn test_generate_mermaid_string_physical_declares_ralph_first_for_layout() {
        // 回归测试（physical view 布局）：Unicode/ASCII 渲染对“节点声明顺序”比较敏感。
        //
        // 约束：
        // - physical view 的 Mermaid 文本里，应优先声明 coordinator（Hat_ralph）节点，
        //   以便 `beautiful-mermaid-rs` 渲染时 ralph#1 更稳定地靠左/靠上（best-effort）。
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(
            &config,
            &registry,
            GraphView::Physical,
            MermaidLabelMode::Strict,
        );

        let declared_hat_nodes: Vec<&str> = output
            .lines()
            .filter(|line| {
                let trimmed = line.trim_start();
                trimmed.starts_with("Hat_")
                    && trimmed.contains('[')
                    && trimmed.ends_with(']')
                    && !trimmed.contains("-->")
            })
            .collect();

        assert!(
            !declared_hat_nodes.is_empty(),
            "expected at least one Hat_* node declaration in Mermaid output"
        );
        assert_eq!(
            declared_hat_nodes[0].trim_start(),
            "Hat_ralph[\"ralph#1 (coordinator)\"]"
        );
    }

    #[test]
    fn test_graph_hats_unicode_with_non_ascii_names() {
        // 回归测试：
        // - hat.name 含中文/emoji 时，unicode/ascii 渲染不应只剩下 task.start→Ralph
        // - 关键是 Mermaid 的“节点 ID”必须是 ASCII 安全的，label 才保留中文/emoji
        let mut registry = HatRegistry::new();

        let mut writer = Hat::new("spec_writer", "📋 规格撰写者");
        writer.subscriptions = vec!["spec.start".into(), "spec.rejected".into()];
        writer.publishes = vec!["spec.ready".into()];
        registry.register(writer);

        let mut reviewer = Hat::new("spec_reviewer", "🔎 规格审阅者");
        reviewer.subscriptions = vec!["spec.ready".into()];
        reviewer.publishes = vec!["spec.rejected".into(), "spec.approved".into()];
        registry.register(reviewer);

        let mut logger = Hat::new("spec_logger", "🧾 规格记录员");
        logger.subscriptions = vec!["spec.ready".into(), "spec.rejected".into()];
        registry.register(logger);

        let config = RalphConfig::default();
        let rendered = render_hat_dag_via_mermaid(
            &config,
            &registry,
            GraphFormat::Unicode,
            GraphView::Logical,
        )
        .unwrap();

        // 只校验关键内容存在即可，避免对 ASCII/Unicode 画法过度耦合（布局可能随渲染器升级微调）。
        assert!(rendered.contains("规格撰写者"));
        assert!(rendered.contains("规格审阅者"));
        assert!(rendered.contains("规格记录员"));
    }

    #[test]
    fn test_show_hat_found() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));

        let mut buf = Vec::new();
        show_hat(&mut buf, &registry, "Builder", false).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("Builder"));
        assert!(output.contains("Triggers On:"));
        assert!(output.contains("build.task"));
        assert!(output.contains("Publishes:"));
        assert!(output.contains("build.done"));
    }

    #[test]
    fn test_show_hat_not_found() {
        let registry = HatRegistry::new();
        let mut buf = Vec::new();
        let result = show_hat(&mut buf, &registry, "Nonexistent", false);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_validate_hats_empty_registry() {
        let registry = HatRegistry::new();
        let config = RalphConfig::default();
        let mut buf = Vec::new();

        validate_hats(&mut buf, &config, &registry, false).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("No hats configured"));
    }

    #[test]
    fn test_list_hats_json() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("Builder", &["build.task"], &["build.done"]));

        let mut buf = Vec::new();
        list_hats_json(&mut buf, &registry).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // Should be valid JSON
        let parsed: serde_json::Value = serde_json::from_str(&output).unwrap();
        assert!(parsed.is_array());
        assert_eq!(parsed.as_array().unwrap().len(), 1);
    }

    #[test]
    fn test_print_check_ok() {
        let mut buf = Vec::new();
        print_check(&mut buf, CheckResult::Ok, "Test passed", false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("[ok]"));
        assert!(output.contains("Test passed"));
    }

    #[test]
    fn test_print_check_warn() {
        let mut buf = Vec::new();
        print_check(&mut buf, CheckResult::Warn, "Warning message", false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("[warn]"));
        assert!(output.contains("Warning message"));
    }

    #[test]
    fn test_print_check_error() {
        let mut buf = Vec::new();
        print_check(&mut buf, CheckResult::Error, "Error message", false).unwrap();
        let output = String::from_utf8(buf).unwrap();
        assert!(output.contains("[err]"));
        assert!(output.contains("Error message"));
    }

    #[test]
    fn test_print_check_colored() {
        let mut buf = Vec::new();
        print_check(&mut buf, CheckResult::Ok, "Color test", true).unwrap();
        let output = String::from_utf8(buf).unwrap();
        // Should contain ANSI color codes
        assert!(output.contains("\x1b["));
    }

    #[test]
    fn test_list_hats_truncates_long_description() {
        let mut registry = HatRegistry::new();
        let mut hat = mock_hat("LongDesc", &["start"], &["end"]);
        hat.description = "A".repeat(100); // Very long description
        registry.register(hat);

        let mut buf = Vec::new();
        list_hats(&mut buf, &registry, false).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // Description should be truncated with "..."
        assert!(output.contains("..."));
    }

    #[test]
    fn test_load_config_missing_explicit_file_errors() {
        // When user explicitly specifies a non-existent config file, it should error
        let source = ConfigSource::File(std::path::PathBuf::from(
            "nonexistent-config-that-does-not-exist.yml",
        ));
        let result = load_config(&source);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Config file not found"));
    }

    #[test]
    fn test_load_config_remote_url_not_supported() {
        // Remote URLs should error with a clear message
        let source = ConfigSource::Remote("http://example.com/config.yml".to_string());
        let result = load_config(&source);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Remote config URLs are not supported"));
    }

    #[test]
    fn test_load_config_unknown_preset_errors() {
        // Unknown builtin preset should error with helpful message
        let source = ConfigSource::Builtin("nonexistent-preset-name".to_string());
        let result = load_config(&source);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Unknown preset"));
        assert!(err.contains("nonexistent-preset-name"));
    }

    #[test]
    fn test_load_config_builtin_preset_works() {
        // Builtin preset should load successfully
        let source = ConfigSource::Builtin("confession-loop".to_string());
        let result = load_config(&source);
        assert!(result.is_ok());
        let config = result.unwrap();
        // confession-loop preset has hats defined
        assert!(!config.hats.is_empty());
    }
}
