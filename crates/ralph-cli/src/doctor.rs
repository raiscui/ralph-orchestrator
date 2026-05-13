//! CLI command: `ralph doctor`.
//!
//! 设计意图(中文说明):
//! - 把"跑不起来"从猜测变成确定性检查。
//! - 把"怎么修"变成可执行的修复建议(必要时提供 `--fix` 做低风险修复)。

use crate::ConfigSource;
use crate::hats::{CheckResult, print_check};
use crate::presets;
use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use ralph_adapters::{detect_backend, is_backend_available};
use ralph_core::{CanonicalWriterStore, EventLoop, HatRegistry, RalphConfig};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(ValueEnum, Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DoctorOutputFormat {
    #[default]
    Text,
    Json,
}

/// Diagnose common startup issues and provide safe fixes.
#[derive(Parser, Debug, Clone)]
pub struct DoctorArgs {
    /// Apply safe fixes (create missing directories/files).
    #[arg(long)]
    pub fix: bool,

    /// Treat warnings as errors (useful for CI).
    #[arg(long)]
    pub strict: bool,

    /// Output format (text, json).
    #[arg(long, value_enum, default_value = "text")]
    pub format: DoctorOutputFormat,

    /// Convenience alias: equivalent to `--format json`.
    #[arg(long)]
    pub json: bool,
}

pub async fn execute(config_path: PathBuf, args: DoctorArgs, use_colors: bool) -> Result<()> {
    let workspace_root = std::env::current_dir().context("Failed to get current directory")?;
    let mut stdout = std::io::stdout();
    run_doctor(&mut stdout, config_path, args, use_colors, workspace_root).await
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
struct DoctorCounts {
    warnings: usize,
    errors: usize,
}

impl DoctorCounts {
    fn warn(&mut self) {
        self.warnings += 1;
    }

    fn err(&mut self) {
        self.errors += 1;
    }
}

#[derive(Debug)]
struct LoadedConfig {
    config: RalphConfig,
    /// 是否因为默认 config 缺失而使用了 `RalphConfig::default()`。
    used_default_due_to_missing: bool,
    /// 用于输出的来源标签(便于定位配置来自哪里)。
    source_label: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoctorStatus {
    Ok,
    Warn,
    Err,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum DoctorVerdict {
    Pass,
    FailErrors,
    FailStrict,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorCheck {
    id: &'static str,
    category: &'static str,
    status: DoctorStatus,
    message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    fix: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize)]
struct DoctorJsonArgs {
    fix: bool,
    strict: bool,
    format: DoctorOutputFormat,
}

#[derive(Debug, Serialize)]
struct DoctorJsonReport {
    schema_version: u32,
    verdict: DoctorVerdict,
    counts: DoctorCounts,
    args: DoctorJsonArgs,
    checks: Vec<DoctorCheck>,
}

struct DoctorReporter<'a, W: Write> {
    writer: &'a mut W,
    use_colors: bool,
    format: DoctorOutputFormat,
    checks: Vec<DoctorCheck>,
    counts: DoctorCounts,
}

impl<'a, W: Write> DoctorReporter<'a, W> {
    fn new(writer: &'a mut W, use_colors: bool, format: DoctorOutputFormat) -> Self {
        Self {
            writer,
            use_colors,
            format,
            checks: Vec::new(),
            counts: DoctorCounts::default(),
        }
    }

    fn finish(self) -> (Vec<DoctorCheck>, DoctorCounts) {
        (self.checks, self.counts)
    }

    fn ok(
        &mut self,
        id: &'static str,
        category: &'static str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.record(DoctorStatus::Ok, id, category, message)
    }

    fn warn(
        &mut self,
        id: &'static str,
        category: &'static str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.record(DoctorStatus::Warn, id, category, message)
    }

    fn err(
        &mut self,
        id: &'static str,
        category: &'static str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.record(DoctorStatus::Err, id, category, message)
    }

    fn skipped(
        &mut self,
        id: &'static str,
        category: &'static str,
        message: impl Into<String>,
    ) -> Result<()> {
        self.record(DoctorStatus::Skipped, id, category, message)
    }

    fn record(
        &mut self,
        status: DoctorStatus,
        id: &'static str,
        category: &'static str,
        message: impl Into<String>,
    ) -> Result<()> {
        let message = message.into();
        let fix = extract_fix_suggestion(&message);

        match status {
            DoctorStatus::Warn => self.counts.warn(),
            DoctorStatus::Err => self.counts.err(),
            DoctorStatus::Ok | DoctorStatus::Skipped => {}
        }

        if self.format == DoctorOutputFormat::Text {
            let result = match status {
                DoctorStatus::Ok | DoctorStatus::Skipped => CheckResult::Ok,
                DoctorStatus::Warn => CheckResult::Warn,
                DoctorStatus::Err => CheckResult::Error,
            };
            print_check(self.writer, result, &message, self.use_colors)?;
        }

        self.checks.push(DoctorCheck {
            id,
            category,
            status,
            message,
            fix,
        });

        Ok(())
    }
}

fn extract_fix_suggestion(message: &str) -> Option<String> {
    // ------------------------------------------------------------
    // 说明:
    // - doctor 的文本输出里大量使用 "Fix: ..." 作为可执行建议.
    // - JSON 输出不想让调用方再去解析整句,因此这里做一次轻量提取.
    // - 规则保持简单: 只取第一次出现的 "Fix:" 后半段,并 trim.
    // ------------------------------------------------------------
    let (_before, after) = message.split_once("Fix:")?;
    let fix = after.trim();
    if fix.is_empty() {
        None
    } else {
        Some(fix.to_string())
    }
}

async fn run_doctor<W: Write>(
    writer: &mut W,
    config_path: PathBuf,
    args: DoctorArgs,
    use_colors: bool,
    workspace_root: PathBuf,
) -> Result<()> {
    let output_format = if args.json {
        DoctorOutputFormat::Json
    } else {
        args.format
    };

    if output_format == DoctorOutputFormat::Text {
        writeln!(writer, "Ralph doctor")?;
        writeln!(writer, "===========")?;
        writeln!(writer)?;
        writeln!(writer, "Checks:")?;
    }

    let mut reporter = DoctorReporter::new(writer, use_colors, output_format);

    // D1: 配置可加载性
    let config_source = ConfigSource::parse(config_path.to_string_lossy().as_ref());
    let loaded = match load_config_for_doctor(&config_source).await {
        Ok(mut loaded) => {
            // 对齐 run 行为: workspace_root 必须显式设为当前目录(对 tests 也很关键)。
            loaded.config.core.workspace_root = workspace_root.clone();
            loaded.config.normalize();
            Some(loaded)
        }
        Err(e) => {
            reporter.err(
                "config.load",
                "config",
                &format!(
                    "Config invalid: {e}. Fix: create/fix config (e.g. `ralph init --list-presets`)"
                ),
            )?;
            None
        }
    };

    let config = if let Some(loaded) = loaded {
        if loaded.used_default_due_to_missing {
            reporter.warn(
                "config.load",
                "config",
                &format!(
                    "Config file not found (using defaults): {}. Fix: `ralph init --list-presets` then `ralph init --preset <name>`",
                    loaded.source_label
                ),
            )?;
        } else {
            reporter.ok(
                "config.load",
                "config",
                &format!("Config loaded: {}", loaded.source_label),
            )?;
        }

        // config.validate() 属于 Ralph 的“内置 guardrail”。
        // 这里不把 warnings 当作致命错误,但会把 validate 的失败直接记为 error。
        match loaded.config.validate() {
            Ok(warnings) => {
                if warnings.is_empty() {
                    reporter.ok("config.validate", "config", "Config validated")?;
                } else {
                    reporter.warn(
                        "config.validate",
                        "config",
                        &format!(
                            "Config validated with warnings ({}). Fix: review config or run `ralph hats validate`",
                            warnings.len()
                        ),
                    )?;
                }
            }
            Err(e) => {
                reporter.err(
                    "config.validate",
                    "config",
                    &format!("Config validation failed: {e}. Fix: review config and rerun"),
                )?;
            }
        }

        Some(loaded.config)
    } else {
        None
    };

    // D2: hats 拓扑校验
    if let Some(cfg) = config.as_ref() {
        let registry = HatRegistry::from_config(cfg);
        check_hat_topology(&mut reporter, cfg, &registry)?;
    } else {
        reporter.err(
            "hats.validation.skipped",
            "hats",
            "Skipped hat validation: config invalid. Fix: repair config first",
        )?;
    }

    // D3: backend 可用性
    if let Some(cfg) = config.as_ref() {
        let backend_ok = check_backend(&mut reporter, cfg)?;

        // D3.5: context window guard(仅当 backend 可用时才做,避免重复报错)
        if backend_ok {
            check_context_window_guard(&mut reporter, cfg)?;
        } else {
            reporter.skipped(
                "context_window.skipped_backend_failed",
                "context_window",
                "Skipped context window guard: backend check failed",
            )?;
        }
    } else {
        reporter.err(
            "backend.skipped",
            "backend",
            "Skipped backend check: config invalid. Fix: repair config first",
        )?;
    }

    // D4: scratchpad/工作区可写性 + `--fix`(安全)
    if let Some(cfg) = config.as_ref() {
        check_and_fix_scratchpad(&mut reporter, cfg, args.fix)?;
    } else {
        reporter.err(
            "workspace.scratchpad.skipped",
            "workspace",
            "Skipped scratchpad check: config invalid. Fix: repair config first",
        )?;
    }

    // D4.5: scoped experience 路径与 writer 可见性
    if let Some(cfg) = config.as_ref() {
        check_scoped_experience_visibility(&mut reporter, cfg)?;
    } else {
        reporter.err(
            "experience.visibility.skipped",
            "experience",
            "Skipped scoped experience visibility: config invalid. Fix: repair config first",
        )?;
    }

    // D5: 当前 run 的 events marker 健康度(如果存在)
    check_and_fix_events_marker(&mut reporter, &workspace_root, args.fix)?;

    // D6: 编译期内嵌配置的新鲜度提示(all_hat overlay)
    check_binary_freshness(&mut reporter, &workspace_root)?;

    let (checks, counts) = reporter.finish();

    let verdict = if counts.errors > 0 {
        DoctorVerdict::FailErrors
    } else if args.strict && counts.warnings > 0 {
        DoctorVerdict::FailStrict
    } else {
        DoctorVerdict::Pass
    };

    match output_format {
        DoctorOutputFormat::Text => {
            writeln!(writer)?;
            writeln!(
                writer,
                "Result: {} errors, {} warnings",
                counts.errors, counts.warnings
            )?;
        }
        DoctorOutputFormat::Json => {
            let report = DoctorJsonReport {
                schema_version: 1,
                verdict,
                counts,
                args: DoctorJsonArgs {
                    fix: args.fix,
                    strict: args.strict,
                    format: output_format,
                },
                checks,
            };
            serde_json::to_writer_pretty(&mut *writer, &report)?;
            writeln!(writer)?;
        }
    }

    if verdict == DoctorVerdict::FailErrors {
        return Err(anyhow::anyhow!(
            "Doctor found {} errors ({} warnings)",
            counts.errors,
            counts.warnings
        ));
    }

    if verdict == DoctorVerdict::FailStrict {
        return Err(anyhow::anyhow!(
            "Doctor strict mode failed: {} warnings",
            counts.warnings
        ));
    }

    Ok(())
}

async fn load_config_for_doctor(source: &ConfigSource) -> Result<LoadedConfig> {
    match source {
        ConfigSource::File(path) => {
            if path.exists() {
                let cfg = RalphConfig::from_file(path)
                    .with_context(|| format!("Failed to load config from {path:?}"))?;
                Ok(LoadedConfig {
                    config: cfg,
                    used_default_due_to_missing: false,
                    source_label: path.display().to_string(),
                })
            } else if path.as_path() == Path::new("ralph.yml") {
                Ok(LoadedConfig {
                    config: RalphConfig::default(),
                    used_default_due_to_missing: true,
                    source_label: path.display().to_string(),
                })
            } else {
                Err(anyhow::anyhow!(
                    "Config file not found: {path:?}. Fix: pass an existing file via -c/--config",
                ))
            }
        }
        ConfigSource::Builtin(name) => {
            let preset = presets::get_preset(name).ok_or_else(|| {
                let available = presets::preset_names().join(", ");
                anyhow::anyhow!(
                    "Unknown preset '{name}'. Fix: `ralph init --list-presets` (available: {available})"
                )
            })?;
            let cfg = RalphConfig::parse_yaml(preset.content)
                .with_context(|| format!("Failed to parse builtin preset '{name}'"))?;
            Ok(LoadedConfig {
                config: cfg,
                used_default_due_to_missing: false,
                source_label: format!("builtin:{name}"),
            })
        }
        ConfigSource::Remote(url) => {
            let response = reqwest::get(url)
                .await
                .with_context(|| format!("Failed to fetch config from {url}"))?;
            if !response.status().is_success() {
                anyhow::bail!(
                    "Failed to fetch config from {url}: HTTP {}. Fix: check URL or use a local config file",
                    response.status()
                );
            }
            let content = response
                .text()
                .await
                .with_context(|| format!("Failed to read config content from {url}"))?;
            let cfg = RalphConfig::parse_yaml(&content)
                .with_context(|| format!("Failed to parse config from {url}"))?;
            Ok(LoadedConfig {
                config: cfg,
                used_default_due_to_missing: false,
                source_label: url.clone(),
            })
        }
    }
}

fn check_hat_topology<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    config: &RalphConfig,
    registry: &HatRegistry,
) -> Result<()> {
    if registry.is_empty() {
        reporter.ok("hats.solo_mode", "hats", "No hats configured (solo mode)")?;
        return Ok(());
    }

    // 1) starting_event 必须有订阅者
    if let Some(start) = &config.event_loop.starting_event {
        if registry.has_subscriber(start) {
            let hat = registry
                .get_for_topic(start)
                .expect("has_subscriber ensures presence");
            reporter.ok(
                "hats.starting_event",
                "hats",
                &format!("Starting event '{start}' has subscriber ({})", hat.name),
            )?;
        } else {
            reporter.err(
                "hats.starting_event",
                "hats",
                &format!(
                    "starting_event '{start}' has no subscribers. Fix: add it to some hat.triggers or unset event_loop.starting_event"
                ),
            )?;
        }
    }

    // 2) orphan events: published but no subscribers
    for hat in registry.all() {
        for pub_event in &hat.publishes {
            let topic = pub_event.as_str();

            if topic == config.event_loop.completion_promise {
                continue;
            }

            if !registry.has_subscriber(topic) {
                reporter.warn(
                    "hats.orphan_event",
                    "hats",
                    &format!(
                        "Event '{topic}' published by '{}' has no hat subscribers. Fix: add a subscriber (hat.triggers) or remove it from publishes",
                        hat.name
                    ),
                )?;
            }
        }
    }

    // 3) dead-end hats: publishes nothing
    let mut dead_end_hats: Vec<String> = registry
        .all()
        .filter(|h| h.publishes.is_empty())
        .map(|h| h.name.clone())
        .collect();
    dead_end_hats.sort();
    for name in dead_end_hats {
        reporter.warn(
            "hats.dead_end",
            "hats",
            &format!(
                "Hat '{name}' publishes nothing (dead end). Fix: add publishes/default_publishes to keep workflow moving"
            ),
        )?;
    }

    Ok(())
}

fn check_scoped_experience_visibility<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    config: &RalphConfig,
) -> Result<()> {
    let store = CanonicalWriterStore::new(&config.core);
    let inspection = store.inspect(config.hats.keys().cloned().collect::<Vec<_>>())?;

    reporter.ok(
        "experience.paths",
        "experience",
        format!(
            "Scoped experience paths resolved: project={}, role_root={}, writer_root={}",
            inspection.project_experience_path.display(),
            inspection.role_experience_root.display(),
            inspection.writer_root.display(),
        ),
    )?;

    let mut writer_parts = vec![format!("project={}", inspection.project_writer.owner)];

    if inspection.role_writers.is_empty() {
        writer_parts.push("roles=none".to_string());
    } else {
        let roles = inspection
            .role_writers
            .iter()
            .map(|record| match &record.scope {
                ralph_core::SharedKnowledgeScope::Role { hat_id } => {
                    format!("{hat_id}:{}", record.owner)
                }
                _ => unreachable!("inspection role_writers only contains role scopes"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        writer_parts.push(format!("roles=[{roles}]"));
    }

    if inspection.topic_writers.is_empty() {
        writer_parts.push("topics=none".to_string());
    } else {
        let topics = inspection
            .topic_writers
            .iter()
            .map(|record| match &record.scope {
                ralph_core::SharedKnowledgeScope::Topic { suffix } => {
                    format!("{suffix}:{}", record.owner)
                }
                _ => unreachable!("inspection topic_writers only contains topic scopes"),
            })
            .collect::<Vec<_>>()
            .join(", ");
        writer_parts.push(format!("topics=[{topics}]"));
    }

    reporter.ok(
        "experience.writers",
        "experience",
        format!("Scoped writer ownership: {}", writer_parts.join(" ; ")),
    )?;

    Ok(())
}

fn check_backend<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    config: &RalphConfig,
) -> Result<bool> {
    let mut backend = config.cli.backend.clone();

    if backend == "auto" {
        let priority = config.get_agent_priority();
        let detected = detect_backend(&priority, |b| config.adapter_settings(b).enabled);
        match detected {
            Ok(chosen) => {
                reporter.ok(
                    "backend.auto_detect",
                    "backend",
                    &format!("Auto-detected backend: {chosen}"),
                )?;
                backend = chosen;
            }
            Err(e) => {
                reporter.err(
                    "backend.auto_detect",
                    "backend",
                    &format!(
                        "Backend auto-detect failed: {e}. Fix: install a supported CLI or set cli.backend explicitly"
                    ),
                )?;
                return Ok(false);
            }
        }
    }

    if backend == "custom" {
        let Some(command) = config.cli.command.as_deref() else {
            reporter.err(
                "backend.custom.command_required",
                "backend",
                "Custom backend requires cli.command. Fix: set cli.command or choose a builtin backend",
            )?;
            return Ok(false);
        };

        // `--version` 不一定适用于所有 custom 命令,这里的主要目的只是验证 "命令是否存在"。
        let ok = match Command::new(command).arg("--version").output() {
            Ok(output) => {
                if output.status.success() {
                    reporter.ok(
                        "backend.custom.command_available",
                        "backend",
                        &format!("Custom backend command available: {command}"),
                    )?;
                    true
                } else {
                    reporter.warn(
                        "backend.custom.version_failed",
                        "backend",
                        &format!(
                            "Custom backend command found but `--version` failed: {command}. Fix: verify the command and args in cli.*"
                        ),
                    )?;
                    true
                }
            }
            Err(e) => {
                reporter.err(
                    "backend.custom.command_not_runnable",
                    "backend",
                    &format!(
                        "Custom backend command not runnable: {command} ({e}). Fix: ensure it is in PATH and executable"
                    ),
                )?;
                false
            }
        };
        return Ok(ok);
    }

    if is_backend_available(&backend) {
        reporter.ok(
            "backend.available",
            "backend",
            &format!("Backend available: {backend}"),
        )?;
        Ok(true)
    } else {
        reporter.err(
            "backend.available",
            "backend",
            &format!(
                "Backend not found in PATH: {backend}. Fix: install the CLI or change cli.backend"
            ),
        )?;
        Ok(false)
    }
}

fn check_context_window_guard<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    config: &RalphConfig,
) -> Result<()> {
    // ---------------------------------------------------------------------
    // 说明(为什么要做):
    // - openclaw 的启发: context window 是硬资源,不足就应该提前 warn/block。
    // - Ralph 自己无法从各个 CLI 后端稳定获取“模型上下文窗”信息,因此这里采用:
    //   - 配置驱动: 由用户在 adapters.<backend>.context_window_tokens 显式声明。
    //   - 轻量估算: 用 chars/4 粗估 tokens,用于 guardrail 提示(不是精确计费)。
    // ---------------------------------------------------------------------
    const CONTEXT_WINDOW_HARD_MIN_TOKENS: u32 = 16_000;
    const CONTEXT_WINDOW_WARN_BELOW_TOKENS: u32 = 32_000;

    // 当 prompt 占用超过该比例时,发出 warn(避免“刚好能塞下,但没法输出”的情况)。
    const PROMPT_WARN_RATIO: f64 = 0.85;

    // 1) 推导当前 run 的 backend(对齐 check_backend 的 auto/custom 语义)
    let mut backend = config.cli.backend.clone();
    let initial_backend = backend.clone();

    if backend == "auto" {
        let priority = config.get_agent_priority();
        let detected = detect_backend(&priority, |b| config.adapter_settings(b).enabled);
        match detected {
            Ok(chosen) => {
                backend = chosen;
            }
            Err(e) => {
                // 理论上这里不应发生(因为 D3 已通过),但 doctor 要保持“可诊断而非 panic”。
                reporter.warn(
                    "context_window.skipped_backend_autodetect_failed",
                    "context_window",
                    &format!(
                        "Context window guard skipped: backend auto-detect failed ({e}). Fix: set cli.backend explicitly"
                    ),
                )?;
                return Ok(());
            }
        }
    }

    let backend_label = match initial_backend.as_str() {
        "auto" => format!("auto->{backend}"),
        "custom" => {
            let command = config.cli.command.as_deref().unwrap_or("<none>");
            format!("custom(command={command})")
        }
        _ => backend.clone(),
    };

    // 2) 选择 adapter profile(用于提示用户该去改哪个 adapters.<backend>)
    //
    // 注意:
    // - 必须与 `RalphConfig::adapter_settings()` 的 custom 映射规则保持一致。
    let adapter_profile = match backend.as_str() {
        "custom" => match config.cli.command.as_deref() {
            Some("codex") => "codex",
            _ => "claude",
        },
        "claude" | "gemini" | "kiro" | "codex" | "amp" => backend.as_str(),
        _ => "claude",
    };

    let window_tokens = config.adapter_settings(&backend).context_window_tokens;

    let Some(window_tokens) = window_tokens else {
        // 默认兼容: 未配置时不产生 warn/err,只提示如何开启。
        reporter.skipped(
            "context_window.skipped_not_configured",
            "context_window",
            &format!(
                "Skipped context window guard: adapters.{adapter_profile}.context_window_tokens not set. Fix: set `adapters.{adapter_profile}.context_window_tokens: <tokens>` (e.g. 200000)",
            ),
        )?;
        return Ok(());
    };

    // 3) 仅根据 window 大小做 openclaw 风格的 warn/block
    if window_tokens < CONTEXT_WINDOW_HARD_MIN_TOKENS {
        reporter.err(
            "context_window.size",
            "context_window",
            &format!(
                "Context window too small: {window_tokens} tokens (<{CONTEXT_WINDOW_HARD_MIN_TOKENS}) for backend={backend_label}. Fix: use a larger-context model and set adapters.{adapter_profile}.context_window_tokens accordingly"
            ),
        )?;
        // 仍继续做 prompt-fit 估算,帮助用户理解“为什么会失败”。
    } else if window_tokens < CONTEXT_WINDOW_WARN_BELOW_TOKENS {
        reporter.warn(
            "context_window.size",
            "context_window",
            &format!(
                "Context window is small: {window_tokens} tokens (<{CONTEXT_WINDOW_WARN_BELOW_TOKENS}) for backend={backend_label}. Fix: consider a larger-context model (or confirm adapters.{adapter_profile}.context_window_tokens matches your model)"
            ),
        )?;
    } else {
        reporter.ok(
            "context_window.size",
            "context_window",
            &format!("Context window ok: {window_tokens} tokens for backend={backend_label}"),
        )?;
    }

    // 4) prompt-fit: 用 core 的 EventLoop 组装一次“非并行”的 ralph prompt,做粗略预算。
    //
    // 说明:
    // - 这里的 tokens 估算是 chars/4,属于近似值。
    // - 并行模式下 ralph#1 prompt 与各 hat prompt 会不同;这里只做“足够发现极端问题”的 guardrail。
    let prompt_content = match crate::loop_runner::resolve_prompt_content(&config.event_loop) {
        Ok(content) => content,
        Err(e) => {
            reporter.skipped(
                "context_window.prompt_fit.skipped",
                "context_window",
                &format!(
                    "Skipped prompt-fit estimate: prompt not readable ({e}). Fix: create/repair {prompt_file} or pass -p/-P when running",
                    prompt_file = config.event_loop.prompt_file
                ),
            )?;
            return Ok(());
        }
    };

    let prompt_chars = match build_serial_ralph_prompt_for_estimation(config, &prompt_content) {
        Some(p) => p.len(),
        None => {
            reporter.skipped(
                "context_window.prompt_fit.skipped",
                "context_window",
                "Skipped prompt-fit estimate: could not build prompt (unexpected). Fix: rerun with `-v` and inspect logs",
            )?;
            return Ok(());
        }
    };

    let prompt_tokens_est = estimate_tokens_from_chars(prompt_chars);
    let ratio = prompt_tokens_est as f64 / window_tokens as f64;
    let ratio_pct = (ratio * 100.0).round() as u32;

    if ratio >= 1.0 {
        reporter.err(
            "context_window.prompt_fit",
            "context_window",
            &format!(
                "Prompt likely exceeds context window: prompt≈{prompt_tokens_est} tok ({prompt_chars} chars), window={window_tokens} tok (backend={backend_label}). Fix: shorten PROMPT.md / reduce memories/scratchpad injection / increase context window"
            ),
        )?;
    } else if ratio >= PROMPT_WARN_RATIO {
        reporter.warn(
            "context_window.prompt_fit",
            "context_window",
            &format!(
                "Prompt is large vs context window: prompt≈{prompt_tokens_est} tok ({ratio_pct}%), window={window_tokens} tok (backend={backend_label}). Fix: shorten PROMPT.md / set memories.budget / use a larger-context model"
            ),
        )?;
    } else {
        reporter.ok(
            "context_window.prompt_fit",
            "context_window",
            &format!(
                "Prompt budget ok: prompt≈{prompt_tokens_est} tok ({ratio_pct}%), window={window_tokens} tok (backend={backend_label})"
            ),
        )?;
    }

    Ok(())
}

fn build_serial_ralph_prompt_for_estimation(
    config: &RalphConfig,
    prompt_content: &str,
) -> Option<String> {
    // ------------------------------------------------------------------
    // 说明:
    // - 通过 core 真实组装一次 prompt,避免 doctor 自己“拼字符串”导致漏算/错算。
    // - 这里不会执行任何后端 CLI,只是在本地构造 prompt 字符串。
    // ------------------------------------------------------------------
    let mut event_loop = EventLoop::new(config.clone());
    event_loop.initialize(prompt_content);

    // next_hat 在 multi-hat/solo 下都应当返回 ralph,但我们不做假设,用返回值驱动。
    let next = event_loop.next_hat()?.clone();
    event_loop.build_prompt(&next)
}

fn estimate_tokens_from_chars(chars: usize) -> u32 {
    // 经验值: English-ish 文本约 4 chars ≈ 1 token。
    // 这不是精确计费,只是用于 guardrail 预算与提示。
    ((chars + 3) / 4) as u32
}

fn check_and_fix_scratchpad<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    config: &RalphConfig,
    fix: bool,
) -> Result<()> {
    let scratchpad_path = config.core.resolve_path(&config.core.scratchpad);
    let Some(scratchpad_dir) = scratchpad_path.parent() else {
        reporter.err(
            "workspace.scratchpad_path_kind",
            "workspace",
            &format!(
                "Scratchpad path has no parent: {}. Fix: set core.scratchpad to a file path",
                scratchpad_path.display()
            ),
        )?;
        return Ok(());
    };

    if !scratchpad_dir.exists() {
        if fix {
            fs::create_dir_all(scratchpad_dir).with_context(|| {
                format!(
                    "Failed to create scratchpad dir {}",
                    scratchpad_dir.display()
                )
            })?;
            reporter.ok(
                "workspace.scratchpad_dir",
                "workspace",
                &format!("Fixed: created scratchpad dir {}", scratchpad_dir.display()),
            )?;
        } else {
            reporter.warn(
                "workspace.scratchpad_dir",
                "workspace",
                &format!(
                    "Scratchpad dir missing: {}. Fix: mkdir -p {} (or run with --fix)",
                    scratchpad_dir.display(),
                    scratchpad_dir.display()
                ),
            )?;
        }
    } else {
        reporter.ok(
            "workspace.scratchpad_dir",
            "workspace",
            &format!("Scratchpad dir exists: {}", scratchpad_dir.display()),
        )?;
    }

    if scratchpad_path.exists() {
        if scratchpad_path.is_dir() {
            reporter.err(
                "workspace.scratchpad_path_kind",
                "workspace",
                &format!(
                    "Scratchpad path is a directory: {}. Fix: set core.scratchpad to a file path",
                    scratchpad_path.display()
                ),
            )?;
            return Ok(());
        }

        // 最小写入能力探测: 尝试以 append 打开。
        if let Err(e) = fs::OpenOptions::new().append(true).open(&scratchpad_path) {
            reporter.err(
                "workspace.scratchpad_writable",
                "workspace",
                &format!(
                    "Scratchpad not writable: {} ({e}). Fix: check permissions",
                    scratchpad_path.display()
                ),
            )?;
        } else {
            reporter.ok(
                "workspace.scratchpad_writable",
                "workspace",
                &format!("Scratchpad writable: {}", scratchpad_path.display()),
            )?;
        }
    } else if fix {
        // 只创建空文件,不覆盖既有内容。
        fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(&scratchpad_path)
            .with_context(|| {
                format!("Failed to create scratchpad {}", scratchpad_path.display())
            })?;
        reporter.ok(
            "workspace.scratchpad_exists",
            "workspace",
            &format!("Fixed: created scratchpad {}", scratchpad_path.display()),
        )?;
    } else {
        reporter.warn(
            "workspace.scratchpad_exists",
            "workspace",
            &format!(
                "Scratchpad missing: {}. Fix: touch {} (or run with --fix)",
                scratchpad_path.display(),
                scratchpad_path.display()
            ),
        )?;
    }

    Ok(())
}

fn check_and_fix_events_marker<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    workspace_root: &Path,
    fix: bool,
) -> Result<()> {
    let Some(marker_path) =
        crate::find_file_in_parents_from(workspace_root, ".ralph/current-events")
    else {
        reporter.ok(
            "events_marker.missing",
            "events_marker",
            "No active run marker found (.ralph/current-events)",
        )?;
        return Ok(());
    };

    let Some(events_path) = crate::resolve_events_file_from_marker(&marker_path) else {
        reporter.err(
            "events_marker.invalid",
            "events_marker",
            &format!(
                "Active run marker is invalid/blank: {}. Fix: rewrite it or delete it",
                marker_path.display()
            ),
        )?;
        return Ok(());
    };

    let try_open = || -> std::io::Result<()> {
        let _file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&events_path)?;
        Ok(())
    };

    match try_open() {
        Ok(()) => {
            reporter.ok(
                "events_marker.events_file_writable",
                "events_marker",
                &format!("Active events file writable: {}", events_path.display()),
            )?;
        }
        Err(e) => {
            if fix {
                if let Some(parent) = events_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                match try_open() {
                    Ok(()) => {
                        reporter.ok(
                            "events_marker.events_file_writable",
                            "events_marker",
                            &format!(
                                "Fixed: created/writable events file {}",
                                events_path.display()
                            ),
                        )?;
                    }
                    Err(e2) => {
                        reporter.err(
                            "events_marker.events_file_writable",
                            "events_marker",
                            &format!(
                                "Active events file not writable: {} ({e2}). Fix: check permissions",
                                events_path.display()
                            ),
                        )?;
                    }
                }
            } else {
                reporter.warn(
                    "events_marker.events_file_writable",
                    "events_marker",
                    &format!(
                        "Active events file not writable: {} ({e}). Fix: check marker/path/permissions (or run with --fix)",
                        events_path.display()
                    ),
                )?;
            }
        }
    }

    Ok(())
}

fn check_binary_freshness<W: Write>(
    reporter: &mut DoctorReporter<'_, W>,
    workspace_root: &Path,
) -> Result<()> {
    let all_hat_path = workspace_root.join("config/all_hat.md");
    if !all_hat_path.exists() {
        reporter.ok(
            "binary_freshness.skipped",
            "binary",
            "Skipped binary freshness: config/all_hat.md not found",
        )?;
        return Ok(());
    }

    let exe_path = std::env::current_exe().context("Failed to resolve current executable")?;

    let all_hat_mtime = fs::metadata(&all_hat_path)
        .and_then(|m| m.modified())
        .context("Failed to read config/all_hat.md mtime")?;
    let exe_mtime = fs::metadata(&exe_path)
        .and_then(|m| m.modified())
        .context("Failed to read executable mtime")?;

    if all_hat_mtime > exe_mtime {
        reporter.warn(
            "binary_freshness.stale",
            "binary",
            "Binary may be stale (config/all_hat.md newer than executable). Fix: rebuild (e.g. `cargo build`)",
        )?;
    } else {
        reporter.ok(
            "binary_freshness.ok",
            "binary",
            "Binary freshness ok (config/all_hat.md not newer than executable)",
        )?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn doctor_returns_error_when_explicit_config_missing() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            workspace_root.join("missing.yml"),
            DoctorArgs {
                fix: false,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: false,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(res.is_err(), "explicit missing config should be an error");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("Config invalid"),
            "output should mention config invalid, got: {text}"
        );
    }

    #[tokio::test]
    async fn doctor_fix_creates_scratchpad_dir_and_file() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        // 生成一个最小配置,避免依赖真实机器上的 Claude/Codex 等 CLI。
        // custom + command=true 可以在大多数 Unix 环境稳定通过 `--version` 检测。
        let cfg_path = workspace_root.join("ralph.yml");
        fs::write(
            &cfg_path,
            r#"
event_loop:
  prompt_file: "PROMPT.md"
cli:
  backend: "custom"
  command: "true"
core:
  scratchpad: ".agent/scratchpad.md"
"#,
        )
        .expect("write config");

        let scratchpad_path = workspace_root.join(".agent/scratchpad.md");
        assert!(
            !scratchpad_path.exists(),
            "precondition: scratchpad should not exist"
        );

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            cfg_path,
            DoctorArgs {
                fix: true,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: false,
            },
            false,
            workspace_root.clone(),
        )
        .await;
        assert!(res.is_ok(), "doctor should succeed when fixes are applied");
        assert!(
            scratchpad_path.exists(),
            "scratchpad should be created by --fix"
        );
    }

    #[tokio::test]
    async fn doctor_strict_fails_on_warnings() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        // 使用默认 config(缺少 ralph.yml)会产生 warning: "using defaults"。
        // strict 模式下必须失败(退出码=1)。
        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            PathBuf::from("ralph.yml"),
            DoctorArgs {
                fix: false,
                strict: true,
                format: DoctorOutputFormat::Text,
                json: false,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(res.is_err(), "strict mode should fail when warnings exist");
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("warnings"),
            "output should contain warnings summary, got: {text}"
        );
    }

    #[tokio::test]
    async fn doctor_fails_when_context_window_below_hard_min() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        // 生成一个最小配置,并启用 context window guard:
        // - backend 用 custom+true,避免依赖真实机器上的 claude/codex 等 CLI。
        // - context_window_tokens 显式设为 15000(<16000),doctor 必须报错阻断。
        let cfg_path = workspace_root.join("ralph.yml");
        fs::write(
            &cfg_path,
            r#"
event_loop:
  prompt_file: "PROMPT.md"
cli:
  backend: "custom"
  command: "true"
core:
  scratchpad: ".agent/scratchpad.md"
adapters:
  claude:
    context_window_tokens: 15000
"#,
        )
        .expect("write config");

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            cfg_path,
            DoctorArgs {
                fix: true,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: false,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(
            res.is_err(),
            "doctor should fail when context window is too small"
        );
        let text = String::from_utf8(out).expect("utf8");
        assert!(
            text.contains("Context window too small"),
            "output should mention context window too small, got: {text}"
        );
    }

    #[tokio::test]
    async fn doctor_json_output_is_valid_json_on_error() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            workspace_root.join("missing.yml"),
            DoctorArgs {
                fix: false,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: true,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(res.is_err(), "explicit missing config should still fail");

        let value: serde_json::Value = serde_json::from_slice(&out).expect("doctor json");
        assert_eq!(value["schema_version"].as_u64(), Some(1));
        assert_eq!(value["args"]["format"].as_str(), Some("json"));
        assert!(value["counts"]["errors"].as_u64().unwrap_or(0) >= 1);

        let checks = value["checks"]
            .as_array()
            .expect("checks should be an array");
        let config_load = checks
            .iter()
            .find(|c| c["id"] == "config.load")
            .expect("should include config.load check");
        assert_eq!(config_load["category"].as_str(), Some("config"));
        assert_eq!(config_load["status"].as_str(), Some("err"));
        assert!(
            config_load["fix"]
                .as_str()
                .unwrap_or_default()
                .contains("ralph init"),
            "config.load should include extracted Fix suggestion"
        );
    }

    #[tokio::test]
    async fn doctor_json_output_is_valid_json_on_success() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        let cfg_path = workspace_root.join("ralph.yml");
        fs::write(
            &cfg_path,
            r#"
event_loop:
  prompt_file: "PROMPT.md"
cli:
  backend: "custom"
  command: "true"
core:
  scratchpad: ".agent/scratchpad.md"
"#,
        )
        .expect("write config");

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            cfg_path,
            DoctorArgs {
                fix: true,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: true,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(res.is_ok(), "doctor should succeed in json mode too");

        let value: serde_json::Value = serde_json::from_slice(&out).expect("doctor json");
        assert_eq!(value["schema_version"].as_u64(), Some(1));
        assert_eq!(value["verdict"].as_str(), Some("pass"));
        assert_eq!(value["counts"]["errors"].as_u64(), Some(0));
    }

    #[tokio::test]
    async fn doctor_json_reports_scoped_experience_paths_and_writers() {
        let temp_dir = tempfile::tempdir().expect("tempdir");
        let workspace_root = temp_dir.path().to_path_buf();

        let cfg_path = workspace_root.join("ralph.yml");
        fs::write(
            &cfg_path,
            r#"
event_loop:
  prompt_file: "PROMPT.md"
cli:
  backend: "custom"
  command: "true"
core:
  scratchpad: ".agent/scratchpad.md"
hats:
  spec_reviewer:
    name: "Spec Reviewer"
    description: "Reviews specs"
    triggers: ["review.spec"]
"#,
        )
        .expect("write config");
        fs::write(
            workspace_root.join("task_plan__alpha.md"),
            "## Active topic\n",
        )
        .unwrap();

        let mut out = Vec::<u8>::new();
        let res = run_doctor(
            &mut out,
            cfg_path,
            DoctorArgs {
                fix: true,
                strict: false,
                format: DoctorOutputFormat::Text,
                json: true,
            },
            false,
            workspace_root,
        )
        .await;

        assert!(res.is_ok(), "doctor should succeed in json mode");

        let value: serde_json::Value = serde_json::from_slice(&out).expect("doctor json");
        let checks = value["checks"]
            .as_array()
            .expect("checks should be an array");

        let paths = checks
            .iter()
            .find(|c| c["id"] == "experience.paths")
            .expect("should include experience.paths");
        assert!(
            paths["message"]
                .as_str()
                .unwrap_or_default()
                .contains("experience.md"),
            "paths message should mention experience.md"
        );

        let writers = checks
            .iter()
            .find(|c| c["id"] == "experience.writers")
            .expect("should include experience.writers");
        let writers_message = writers["message"].as_str().unwrap_or_default();
        assert!(writers_message.contains("project=ralph#1"));
        assert!(writers_message.contains("spec_reviewer:ralph#1"));
        assert!(writers_message.contains("alpha:ralph#1"));
    }
}
