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
use beautiful_mermaid_rs::{AsciiRenderOptions, render_mermaid_ascii};
use clap::{Parser, Subcommand, ValueEnum};
use ralph_core::{HatRegistry, RalphConfig};
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
        Some(HatsCommands::Graph { format }) => graph_hats(&mut stdout, &config, &registry, format),
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

enum CheckResult {
    Ok,
    Warn,
    Error,
}

fn print_check<W: Write>(
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
) -> Result<()> {
    match format {
        GraphFormat::Mermaid => {
            writeln!(writer, "```mermaid")?;
            write!(writer, "{}", generate_mermaid_string(config, registry))?;
            writeln!(writer, "```")?;
        }
        GraphFormat::Unicode | GraphFormat::Ascii | GraphFormat::Compact => {
            let rendered = render_hat_dag_via_mermaid(config, registry, format)?;
            write!(writer, "{rendered}")?;
        }
    }
    Ok(())
}

fn render_hat_dag_via_mermaid(
    config: &RalphConfig,
    registry: &HatRegistry,
    format: GraphFormat,
) -> Result<String> {
    if registry.is_empty() {
        return Ok("No hats configured.\n".to_string());
    }

    let diagram = generate_mermaid_string(config, registry);
    let options = match format {
        GraphFormat::Unicode => AsciiRenderOptions {
            use_ascii: Some(false),
            ..Default::default()
        },
        GraphFormat::Ascii => AsciiRenderOptions {
            use_ascii: Some(true),
            ..Default::default()
        },
        GraphFormat::Compact => AsciiRenderOptions {
            use_ascii: Some(false),
            padding_x: Some(0),
            padding_y: Some(0),
            box_border_padding: Some(0),
        },
        GraphFormat::Mermaid => AsciiRenderOptions::default(),
    };

    let mut rendered = render_mermaid_ascii(&diagram, &options)
        .with_context(|| "Failed to render Mermaid topology as ASCII/Unicode".to_string())?;

    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    Ok(rendered)
}

/// 为 TUI 的右上角 Radar 面板渲染 hats graph（Mermaid 文本）。
///
/// 返回：
/// - `(compact, full)`
///
/// 说明：
/// - **这里不做 Mermaid→ASCII 盒子图渲染**，只输出 Mermaid 源码文本。
///   原因：`beautiful-mermaid-rs`（QuickJS+大 bundle）在 debug/release 下都可能是“十几秒级”开销，
///   会把 `ralph run --tui` 的启动体验拖成“长时间黑屏”。
/// - small/zoom 的差异通过 “TUI panel 尺寸 + 文本裁剪” 达成：
///   - compact：仅输出关键连线（更像雷达概览）
///   - full：输出完整 Mermaid（含节点 label），便于复制/外部渲染
pub(crate) fn render_hat_graph_radar_text(
    config: &RalphConfig,
    registry: &HatRegistry,
) -> Result<(String, String)> {
    if registry.is_empty() {
        let empty = "No hats configured.\n".to_string();
        return Ok((empty.clone(), empty));
    }

    let mut compact = generate_mermaid_radar_compact(config, registry);
    if !compact.ends_with('\n') {
        compact.push('\n');
    }

    let mut full = generate_mermaid_string(config, registry);
    if !full.ends_with('\n') {
        full.push('\n');
    }

    Ok((compact, full))
}

/// Radar 小窗：只输出“关键连线”，尽量让用户在小面板里就能看懂拓扑。
fn generate_mermaid_radar_compact(config: &RalphConfig, registry: &HatRegistry) -> String {
    // 逻辑视图：保持与 `generate_mermaid_string(...)` 一致的推导规则，但去掉 hat 节点声明行。
    //
    // 备注：
    // - 这里输出的是 Mermaid 文本（不是 ASCII 盒子图），因此性能几乎只与 hats 数量线性相关；
    // - 仍然保持排序+去重，确保输出稳定可预测（便于截图对比/调试）。
    let mut hats: Vec<_> = registry.all().collect();
    hats.sort_by(|a, b| a.id.as_str().cmp(b.id.as_str()));

    let mut output = String::new();
    output.push_str("flowchart LR\n");

    // 入口边：starting_event（可选）
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

        output.push_str("    Start[task.start]\n");
        for target_id in targets {
            output.push_str(&format!(
                "    Start -->|{}| {}\n",
                starting_event, target_id
            ));
        }
    }

    // 结束节点：complete_publishes（可选）
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

/// Generate Mermaid flowchart syntax for the hat topology.
fn generate_mermaid_string(config: &RalphConfig, registry: &HatRegistry) -> String {
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
    output.push_str("flowchart LR\n");

    // 先声明节点（含 label），避免后续边只引用 ID 时渲染器把 label 丢成裸 ID。
    for hat in &hats {
        let node_id = mermaid_hat_node_id(hat.id.as_str());
        let label_source = if hat.name.trim().is_empty() {
            hat.id.as_str()
        } else {
            &hat.name
        };
        let label = format_mermaid_node_label(label_source);
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

fn format_mermaid_node_label(label: &str) -> String {
    // Mermaid 的 `Node[label]` 语法无法直接包含 `]` 与换行。
    // 遇到这类情况就回退到带引号的 label，并做必要转义。
    //
    // 备注：
    // - 我们默认优先不用引号，是因为当前 `beautiful-mermaid-rs` 会把引号当成 label 内容输出，
    //   造成终端图里出现 `"Ralph"` 这种多余引号，阅读体验很差。
    if label.contains(']') || label.contains('\n') {
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

        graph_hats(&mut buf, &config, &registry, GraphFormat::Mermaid).unwrap();
        let output = String::from_utf8(buf).unwrap();

        assert!(output.contains("flowchart LR"));
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

        graph_hats(&mut buf, &config, &registry, GraphFormat::Ascii).unwrap();
        let output = String::from_utf8(buf).unwrap();

        // deterministic output should contain key node names
        assert!(output.contains("Builder"));
    }

    #[test]
    fn test_generate_mermaid_string() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let config = RalphConfig::default();
        let output = generate_mermaid_string(&config, &registry);

        assert!(output.contains("flowchart LR"));
        assert!(output.contains("Hat_A -->|mid| Hat_B"));
        assert!(!output.contains("Ralph"));
        assert!(!output.contains("-.->"));
    }

    #[test]
    fn test_generate_mermaid_string_includes_complete_publishes() {
        let mut registry = HatRegistry::new();
        registry.register(mock_hat("A", &["start"], &["mid"]));
        registry.register(mock_hat("B", &["mid"], &["end"]));

        let mut config = RalphConfig::default();
        config.event_loop.complete_publishes = Some("end".to_string());

        let output = generate_mermaid_string(&config, &registry);

        assert!(output.contains("Complete[complete]"));
        assert!(output.contains("Hat_B -->|end| Complete"));
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
        let rendered =
            render_hat_dag_via_mermaid(&config, &registry, GraphFormat::Unicode).unwrap();

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
