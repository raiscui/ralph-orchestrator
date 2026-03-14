//! # Autopilot
//!
//! 目标:
//! - 在一个已初始化的 Git 仓库目录内,无人值守地运行 `ralph run --record-session ...`.
//! - 以 record-session JSONL 为主证据源,做硬断言判定,并生成 report.json/report.md.
//!
//! 说明:
//! - 这里的 autopilot 是一个薄包装层,不改变 `ralph run` 语义.
//! - 真实工作流仍由子进程 `ralph run` 执行,autopilot 负责"前置自检 + 证据落盘 + 判定".

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use ralph_core::{CliConfig, EventParser, RalphConfig};
use ralph_proto::Event;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tracing::{info, warn};

/// Headless automation for in-place Git repo runs.
///
/// 约束:
/// - 必须在已初始化的 Git 仓库目录内运行(或显式指定 --repo-dir).
/// - 判定以 `--record-session` 生成的 JSONL 为主证据源.
#[derive(Parser, Debug)]
#[command(about)]
pub(crate) struct AutopilotArgs {
    #[command(subcommand)]
    pub(crate) command: AutopilotCommand,
}

#[derive(Subcommand, Debug)]
pub(crate) enum AutopilotCommand {
    /// 在指定 Git 仓库目录内运行一次真实 `ralph run --no-tui --record-session ...`,然后分析并输出 verdict
    Run(AutopilotRunArgs),

    /// 不重新运行,只分析一份已存在的 record-session JSONL,生成报告并以稳定退出码表达 verdict
    Analyze(AutopilotAnalyzeArgs),
}

/// Shared analysis-related flags (run/analyze).
#[derive(Parser, Debug, Clone)]
pub(crate) struct AutopilotAnalysisArgs {
    /// 输出目录(用于落盘 report/analysis 产物).
    ///
    /// 说明:
    /// - 若不提供,程序会在 repo 下生成一个带时间戳的默认目录.
    #[arg(long, value_name = "DIR")]
    pub(crate) out_dir: Option<PathBuf>,

    /// 跳过 agent 分析步骤(仅做硬断言判定).
    ///
    /// 说明:
    /// - 默认会在 hard verdict 通过后执行 agent 分析.
    /// - 只有显式传 `--skip-agent-analysis` 才会关闭该步骤.
    #[arg(long)]
    pub(crate) skip_agent_analysis: bool,

    /// agent 分析步骤使用的 backend(可选覆盖;不传则默认跟随配置或自动探测).
    #[arg(long, value_name = "BACKEND")]
    pub(crate) analysis_backend: Option<String>,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct AutopilotRunArgs {
    /// 目标 Git 仓库目录(必须是已初始化的 Git repo).
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub(crate) repo_dir: PathBuf,

    /// record-session JSONL 输出路径(将强制传给子进程 `ralph run --record-session`).
    #[arg(long, value_name = "FILE")]
    pub(crate) record_session: PathBuf,

    /// (autopilot/tests 用) 覆盖并行模式全局并发上限,仅作用于 autopilot 子进程的 `ralph run`.
    ///
    /// 说明:
    /// - 这不会改变用户直接执行 `ralph run` 的默认并发语义.
    /// - autopilot 会在 out_dir 内生成派生 config(默认文件名: child_ralph.yml),
    ///   并用该文件启动子进程,从而做到对用户零影响的强隔离.
    #[arg(long, value_name = "N")]
    pub(crate) child_parallel_max_running_jobs: Option<usize>,

    #[command(flatten)]
    pub(crate) analysis: AutopilotAnalysisArgs,
}

#[derive(Parser, Debug, Clone)]
pub(crate) struct AutopilotAnalyzeArgs {
    /// repo 目录(用于解析相对路径;不要求一定是 Git repo,但推荐).
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub(crate) repo_dir: PathBuf,

    /// 待分析的 record-session JSONL 文件路径.
    #[arg(long, value_name = "FILE")]
    pub(crate) record_session: PathBuf,

    #[command(flatten)]
    pub(crate) analysis: AutopilotAnalysisArgs,
}

/// Autopilot 的退出码语义(按 spec 固化).
///
/// 说明:
/// - 这里用 `i32` 是为了与 `std::process::exit` 直接对接.
/// - 不要在核心逻辑里直接 `exit`,方便单测覆盖退出码映射与报告生成.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(i32)]
pub(crate) enum AutopilotExitCode {
    /// 硬断言 PASS 且 agent verdict PASS(或显式跳过 agent 分析).
    Pass = 0,
    /// 硬断言 FAIL.
    HardFail = 1,
    /// 硬断言 PASS 但 agent verdict FAIL 或 quality_score=suboptimal.
    AgentFail = 2,
    /// 需要 agent 分析但分析运行/解析失败.
    AnalysisError = 3,
}

/// 单条硬断言的结构化结果(供 report.json 消费).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HardAssertion {
    /// 断言名称(稳定标识,便于脚本消费).
    pub(crate) name: String,
    /// 是否通过.
    pub(crate) passed: bool,
    /// 期望值(结构化).
    pub(crate) expected: serde_json::Value,
    /// 实际观察到的值(结构化).
    pub(crate) actual: serde_json::Value,
    /// 证据引用(指向 record-session JSONL 的 record index).
    #[serde(default)]
    pub(crate) evidence_refs: Vec<EvidenceRef>,
}

/// 证据引用: 只用 record index 做“可定位”引用,避免把大 payload 塞进 report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct EvidenceRef {
    pub(crate) record_index: usize,
    pub(crate) record_event: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) topic: Option<String>,
}

/// 硬判定总览.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct HardVerdict {
    pub(crate) passed: bool,
    pub(crate) assertions: Vec<HardAssertion>,
}

/// agent 分析输出的结构化 JSON(从 `<event topic="analyze.complete">...</event>` 解析而来).
///
/// 注意:
/// - 这是“对外协议”.字段名尽量贴近 spec,避免后续脚本/CI 解析漂移.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentAnalysisOutput {
    pub(crate) verdict: AgentVerdict,
    pub(crate) quality_score: QualityScore,
    #[serde(default)]
    pub(crate) requirements_met: Vec<RequirementCheck>,
    #[serde(default)]
    pub(crate) risks: Vec<String>,
    #[serde(default)]
    pub(crate) suggested_fixes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentVerdict {
    Pass,
    Fail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QualityScore {
    Optimal,
    Good,
    Acceptable,
    Suboptimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct RequirementCheck {
    pub(crate) name: String,
    pub(crate) passed: bool,
    #[serde(default)]
    pub(crate) evidence_refs: Vec<String>,
}

/// report.json 的机器可读结构.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AutopilotReportJson {
    /// schema 版本,用于后续兼容脚本解析.
    pub(crate) schema_version: String,
    /// "run" 或 "analyze".
    pub(crate) mode: String,
    /// 目标 repo 目录(绝对路径).
    pub(crate) repo_dir: PathBuf,
    /// record-session JSONL 的绝对路径(或可解析路径).
    pub(crate) record_session: PathBuf,
    /// 产物输出目录.
    pub(crate) out_dir: PathBuf,
    /// 硬断言结果.
    pub(crate) hard_verdict: HardVerdict,
    /// agent 分析输出(若跳过或失败,这里仍会给出结构化占位).
    pub(crate) agent: AgentSection,
    /// 最终退出码(稳定语义: 0/1/2/3).
    pub(crate) exit_code: i32,
    /// 退出码的语义名称(便于人类阅读与快速 grep).
    pub(crate) exit_code_semantic: String,
    /// 退出原因(面向人类,但也可用于脚本诊断).
    pub(crate) exit_reason: String,
    /// 子进程 `ralph run` 的退出码(仅 run 模式存在).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) child_status: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct AgentSection {
    pub(crate) status: AgentSectionStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<AgentAnalysisOutput>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AgentSectionStatus {
    /// 已执行且解析成功.
    Ok,
    /// 显式跳过.
    Skipped,
    /// 需要执行但失败(运行/解析错误).
    Error,
    /// 不需要执行(例如硬断言失败).
    NotRun,
}

/// 解析 record-session JSONL 后得到的最小摘要(用于硬断言与证据包).
#[derive(Debug, Clone)]
struct RecordSessionSummary {
    hard_verdict: HardVerdict,
    termination_reason: Option<String>,
    commits: Vec<String>,
    banned_topics_hit: BTreeSet<String>,
    topic_counts: BTreeMap<String, usize>,
    topic_timeline: Vec<String>,
    terminal_tail: String,
    parse_error: Option<String>,
}

pub(crate) async fn execute(config_path: PathBuf, args: AutopilotArgs) -> Result<()> {
    match args.command {
        AutopilotCommand::Run(run_args) => {
            let report = autopilot_run(config_path, run_args).await?;
            write_reports(&report).await?;
            exit_if_needed(report.exit_code);
            Ok(())
        }
        AutopilotCommand::Analyze(analyze_args) => {
            let report = autopilot_analyze(config_path, analyze_args).await?;
            write_reports(&report).await?;
            exit_if_needed(report.exit_code);
            Ok(())
        }
    }
}

fn exit_if_needed(code: i32) {
    if code != 0 {
        std::process::exit(code);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CLI commands
// ─────────────────────────────────────────────────────────────────────────────

async fn autopilot_run(
    config_path: PathBuf,
    args: AutopilotRunArgs,
) -> Result<AutopilotReportJson> {
    // 说明:
    // - run 模式需要先做 repo/路径自检,再以子进程方式运行真实 `ralph run`.
    // - 子进程结束后,再走 analyze 流程生成 verdict 与报告.
    let AutopilotRunArgs {
        repo_dir,
        record_session: record_session_rel,
        child_parallel_max_running_jobs,
        analysis,
    } = args;

    let repo_dir_abs = absolute_path(&repo_dir)?;

    // 2.1 Git repo 自检必须最先做,因为后续所有相对路径都应该以 repo root 为基准.
    let git_root = ensure_git_repo(&repo_dir_abs).await?;

    let record_session = resolve_under(&git_root, &record_session_rel);
    let out_dir = resolve_out_dir(&git_root, analysis.out_dir.as_ref())?;

    tokio::fs::create_dir_all(&out_dir)
        .await
        .with_context(|| format!("Failed to create out-dir: {}", out_dir.display()))?;

    // 产物布局: 即使后续失败,也尽量保证基础文件存在,便于无人值守场景排障.
    ensure_min_output_layout(&out_dir).await?;

    // 2.1 worktree 可用性检查(必须在启动子进程前失败).
    let (config_for_check, config_arg) = load_config_for_repo(&config_path, &git_root).await?;
    if requires_git_worktree(&config_for_check) {
        ensure_git_worktree_available(&git_root).await?;
    }

    // 说明:
    // - 这是一个“强隔离”的测试开关: 只影响 autopilot 启动的子进程 `ralph run`.
    // - 用户在自己项目里直接执行 `ralph run` 时,不会读到这里生成的派生 config,因此不会被限并发.
    let mut config_arg_for_child = config_arg;
    let mut config_path_for_analyze = config_path;
    if let Some(max_jobs) = child_parallel_max_running_jobs {
        let child_cfg =
            derive_child_config_with_parallel_max_running_jobs(&config_for_check, max_jobs)?;
        let child_cfg_path = out_dir.join("child_ralph.yml");
        let yaml = serde_yaml::to_string(&child_cfg)
            .with_context(|| "派生 child config 序列化为 YAML 失败")?;
        tokio::fs::write(&child_cfg_path, yaml)
            .await
            .with_context(|| format!("Failed to write {}", child_cfg_path.display()))?;

        info!(
            "Autopilot derived child config (parallel.autoscale.max_running_jobs={}) -> {}",
            max_jobs,
            child_cfg_path.display()
        );

        config_arg_for_child = child_cfg_path.to_string_lossy().to_string();
        config_path_for_analyze = child_cfg_path;
    }

    // 2.2 record-session 目标路径可写检查(必须在启动子进程前失败).
    ensure_writable_file_path(&record_session).with_context(|| {
        format!(
            "record-session path not writable: {}",
            record_session.display()
        )
    })?;

    // 2.3/2.4 以子进程方式启动真实的 `ralph run`,并 tee stdout/stderr 到 out-dir.
    let child_status =
        run_child_ralph_run(&git_root, &config_arg_for_child, &record_session, &out_dir).await?;
    let child_status_code = child_status.code();

    // 子进程即使失败,也尝试继续 analyze,因为 JSONL 可能已落盘,可以给出更精确 verdict.
    let mut analyze_args = AutopilotAnalyzeArgs {
        repo_dir: git_root.clone(),
        record_session: record_session.clone(),
        analysis: analysis.clone(),
    };
    // 说明:
    // - run 模式下 stdout/stderr 已经落盘到 out_dir.
    // - 为了保持产物在同一目录,这里强制把 analyze 的 out_dir 指向同一个 out_dir.
    analyze_args.analysis.out_dir = Some(out_dir.clone());
    let mut report = autopilot_analyze(config_path_for_analyze, analyze_args).await?;
    report.mode = "run".to_string();
    report.repo_dir = git_root;
    report.out_dir = out_dir;
    report.child_status = child_status_code;

    // 附加信息: child run 的退出码通常对排障有帮助,但不直接决定最终 verdict.
    if child_status_code.is_some() && report.exit_code == AutopilotExitCode::Pass as i32 {
        // 这里不把 child_status 作为失败门槛,避免把 verdict 与 child 的实现细节强耦合.
        // 但如果后续发现 child_status 非 0 且 hard/agent 都 PASS,我们会给一条 warning.
        if let Some(code) = child_status_code
            && code != 0
        {
            warn!("Child `ralph run` exited with {code}, but verdict PASS (hard+agent).");
        }
    }

    Ok(report)
}

fn derive_child_config_with_parallel_max_running_jobs(
    base: &RalphConfig,
    max_running_jobs: usize,
) -> Result<RalphConfig> {
    if max_running_jobs < 1 {
        anyhow::bail!(
            "`--child-parallel-max-running-jobs` 必须 >= 1,当前为 {}",
            max_running_jobs
        );
    }
    let mut derived = base.clone();
    derived.parallel.autoscale.max_running_jobs = max_running_jobs;
    Ok(derived)
}

async fn autopilot_analyze(
    config_path: PathBuf,
    mut args: AutopilotAnalyzeArgs,
) -> Result<AutopilotReportJson> {
    // 说明:
    // - analyze 模式不重新运行目标 workflow.
    // - 它只读取 record-session JSONL,生成硬断言 verdict + 可选 agent 分析 + 报告.
    let repo_dir_abs = absolute_path(&args.repo_dir)?;

    // 若 repo_dir 在 Git 仓库内,尽量归一化到 toplevel,保证 `.ralph/*` 等相对路径一致.
    // 但 analyze 允许离线场景,因此 git 探测失败时不强制报错.
    let repo_root = match ensure_git_repo(&repo_dir_abs).await {
        Ok(root) => root,
        Err(_) => repo_dir_abs.clone(),
    };

    let record_session = resolve_under(&repo_root, &args.record_session);
    args.repo_dir = repo_root.clone();
    args.record_session = record_session.clone();

    let out_dir = resolve_out_dir(&repo_root, args.analysis.out_dir.as_ref())?;
    tokio::fs::create_dir_all(&out_dir)
        .await
        .with_context(|| format!("Failed to create out-dir: {}", out_dir.display()))?;
    ensure_min_output_layout(&out_dir).await?;

    // analyze 模式本身不要求 repo_dir 必须是 Git repo,但如果是,可以把路径写入 report.
    // 这里不做强制失败,以便支持离线分析"搬出来的 JSONL".

    // 从 JSONL 解析并输出硬断言.
    let summary = match parse_record_session(&record_session) {
        Ok(s) => s,
        Err(e) => {
            warn!(
                "Failed to parse record-session JSONL (treat as hard fail): {}",
                e
            );
            record_session_summary_from_parse_error(&e)
        }
    };

    // 证据包总是落盘,即使硬断言失败也能帮助定位缺什么.
    let analysis_input_path = out_dir.join("analysis_input.json");
    write_analysis_input_json(&analysis_input_path, &record_session, &summary).await?;

    // analysis_output.json: 无论是否执行 agent,都写入稳定占位/结果,方便脚本消费.
    let analysis_output_path = out_dir.join("analysis_output.json");

    // 若硬断言失败,agent 分析不再运行(按 spec: agent 只在 hard pass 后执行).
    let (agent_section, exit_code, exit_reason) = if !summary.hard_verdict.passed {
        write_json_pretty(
            &analysis_output_path,
            &serde_json::json!({ "status": "not_run", "reason": "hard_verdict_failed" }),
        )
        .await?;
        (
            AgentSection {
                status: AgentSectionStatus::NotRun,
                output: None,
                reason: Some("hard_verdict_failed".to_string()),
            },
            AutopilotExitCode::HardFail,
            summary.parse_error.clone().map_or_else(
                || "硬断言失败: required/banned/termination 等合同未满足".to_string(),
                |e| format!("record-session 解析失败: {e}"),
            ),
        )
    } else if args.analysis.skip_agent_analysis {
        write_json_pretty(
            &analysis_output_path,
            &serde_json::json!({ "status": "skipped", "reason": "--skip-agent-analysis" }),
        )
        .await?;
        (
            AgentSection {
                status: AgentSectionStatus::Skipped,
                output: None,
                reason: Some("--skip-agent-analysis".to_string()),
            },
            AutopilotExitCode::Pass,
            "硬断言通过,已按 --skip-agent-analysis 跳过 agent 分析".to_string(),
        )
    } else {
        // hard pass 且未跳过: 执行 agent 分析.
        match run_agent_analysis(
            &config_path,
            &args.repo_dir,
            &out_dir,
            &analysis_input_path,
            &args.analysis,
        )
        .await
        {
            Ok(output) => {
                // 解析成功: 写入 analysis_output.json,并映射退出码.
                write_json_pretty(&analysis_output_path, &output).await?;
                let (code, reason) = map_agent_output_to_exit_code(&output);
                (
                    AgentSection {
                        status: AgentSectionStatus::Ok,
                        output: Some(output),
                        reason: None,
                    },
                    code,
                    reason,
                )
            }
            Err(e) => {
                // agent 分析失败: 写入占位文件,并返回 exit code 3.
                let placeholder = serde_json::json!({
                    "status": "error",
                    "error": format!("{e:#}"),
                });
                write_json_pretty(&analysis_output_path, &placeholder).await?;
                (
                    AgentSection {
                        status: AgentSectionStatus::Error,
                        output: None,
                        reason: Some(format!("{e:#}")),
                    },
                    AutopilotExitCode::AnalysisError,
                    "硬断言通过,但 agent 分析运行/解析失败".to_string(),
                )
            }
        }
    };

    Ok(AutopilotReportJson {
        schema_version: "autopilot-report@v1".to_string(),
        mode: "analyze".to_string(),
        repo_dir: repo_root,
        record_session,
        out_dir,
        hard_verdict: summary.hard_verdict,
        agent: agent_section,
        exit_code: exit_code as i32,
        exit_code_semantic: format!("{exit_code:?}"),
        exit_reason,
        child_status: None,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Preflight checks + child run
// ─────────────────────────────────────────────────────────────────────────────

fn absolute_path(path: &Path) -> Result<PathBuf> {
    // 说明:
    // - autopilot 经常需要从“仓库外部”运行,因此把关键路径尽量转为绝对路径更稳.
    // - canonicalize 会要求路径存在,这里用 current_dir 拼接的方式更宽松.
    let p = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .with_context(|| "Failed to get current_dir")?
            .join(path)
    };
    Ok(p)
}

fn resolve_under(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn resolve_out_dir(repo_dir: &Path, out_dir: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(dir) = out_dir {
        Ok(resolve_under(repo_dir, dir))
    } else {
        // 默认输出到 repo 内的 `.ralph/autopilot/<timestamp>/`
        let ts = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
        Ok(repo_dir.join(".ralph/autopilot").join(ts))
    }
}

async fn ensure_git_repo(repo_dir: &Path) -> Result<PathBuf> {
    // 用 git rev-parse 做硬性判定,避免后续 worktree/commit 等能力在中途才炸.
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .current_dir(repo_dir)
        .output()
        .await
        .with_context(|| format!("Failed to run git rev-parse in {}", repo_dir.display()))?;

    if !output.status.success() {
        anyhow::bail!(
            "目录不是已初始化的 Git 仓库,或 git 不可用. repo_dir={}\n\n提示: autopilot 必须在 Git 仓库内就地运行(并行 worktree 依赖该前提).",
            repo_dir.display()
        );
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let top = stdout.trim();
    if top.is_empty() {
        anyhow::bail!(
            "git rev-parse returned empty toplevel for repo_dir={}",
            repo_dir.display()
        );
    }
    Ok(PathBuf::from(top))
}

async fn ensure_git_worktree_available(repo_dir: &Path) -> Result<()> {
    // 用 `git worktree list` 做可用性探测. 如果 git 太老或被裁剪,这里会直接失败.
    let output = Command::new("git")
        .args(["worktree", "list"])
        .current_dir(repo_dir)
        .output()
        .await
        .with_context(|| format!("Failed to run git worktree list in {}", repo_dir.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "检测到配置需要 workspace.strategy=worktree,但 `git worktree` 不可用.\nrepo_dir={}\n\nstderr:\n{}",
            repo_dir.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok(())
}

async fn load_config_for_repo(
    config_path: &PathBuf,
    repo_dir: &Path,
) -> Result<(RalphConfig, String)> {
    // 说明:
    // - autopilot 子进程会把 current_dir 切到 repo_dir,因此这里要把 `--config` 的相对路径解析稳.
    // - 我们同时返回两个值:
    //   - `RalphConfig`: 用于 preflight 检查(是否需要 git worktree)
    //   - `config_arg`: 传给子进程 `ralph run --config ...` 的参数(尽量用绝对路径)
    let raw = config_path.to_string_lossy().to_string();
    if raw.starts_with("builtin:") || raw.starts_with("http://") || raw.starts_with("https://") {
        let cfg = load_config_from_source(&raw).await?;
        return Ok((cfg, raw));
    }

    // File path: 相对路径优先按 repo_dir 解析(更符合“就地运行”的直觉).
    let resolved_path = resolve_under(repo_dir, config_path);
    if !resolved_path.exists() {
        anyhow::bail!(
            "Config file not found: {}\n\n提示: 你可以用 `--config <path>` 显式指定,或在 repo 根目录放置 ralph.yml.",
            resolved_path.display()
        );
    }
    let cfg = RalphConfig::from_file(&resolved_path)
        .with_context(|| format!("Failed to load config from {}", resolved_path.display()))?;
    Ok((cfg, resolved_path.to_string_lossy().to_string()))
}

async fn load_config_from_source(source: &str) -> Result<RalphConfig> {
    // 这里复用与 `ralph run` 一致的 config source 语义.
    // - builtin:<name>
    // - http(s)://...
    // - file path
    if let Some(name) = source.strip_prefix("builtin:") {
        let preset = crate::presets::get_preset(name).ok_or_else(|| {
            let available = crate::presets::preset_names().join(", ");
            anyhow::anyhow!("Unknown preset '{}'. Available: {}", name, available)
        })?;
        return RalphConfig::parse_yaml(preset.content)
            .with_context(|| format!("Failed to parse builtin preset '{name}'"));
    }

    if source.starts_with("http://") || source.starts_with("https://") {
        let response = reqwest::get(source)
            .await
            .with_context(|| format!("Failed to fetch config from {source}"))?;
        if !response.status().is_success() {
            anyhow::bail!(
                "Failed to fetch config from {source}: HTTP {}",
                response.status()
            );
        }
        let content = response
            .text()
            .await
            .with_context(|| format!("Failed to read config content from {source}"))?;
        return RalphConfig::parse_yaml(&content)
            .with_context(|| format!("Failed to parse remote config from {source}"));
    }

    RalphConfig::from_file(Path::new(source))
        .with_context(|| format!("Failed to load config from {source}"))
}

fn requires_git_worktree(config: &RalphConfig) -> bool {
    // 规则:
    // - 只有并行模式才会走 workspace 策略与 worktree 隔离.
    // - 只要有任一 hat 的默认 workspace.strategy=worktree,就认为需要 `git worktree`.
    if !config.parallel.enabled {
        return false;
    }
    config
        .hats
        .values()
        .any(|hat| hat.workspace.strategy == ralph_core::WorkspaceStrategy::Worktree)
}

fn ensure_writable_file_path(path: &Path) -> Result<()> {
    // 说明:
    // - 这里用 "create + close" 来做可写探测,避免子进程跑到一半才发现无法落盘.
    // - 文件存在时会被截断;这与 `ralph run` 内部的 File::create 行为一致.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create parent dir for file: {}", parent.display())
        })?;
    }
    let _ = std::fs::File::create(path)
        .with_context(|| format!("Failed to create file for write-check: {}", path.display()))?;
    Ok(())
}

async fn ensure_min_output_layout(out_dir: &Path) -> Result<()> {
    // 说明:
    // - spec 要求 out-dir 至少包含这些文件.
    // - 即使后续失败,也尽量提前把文件创建出来,让 CI/脚本可以稳定地 "cat" 它们.
    let stdout_path = out_dir.join("stdout.txt");
    let stderr_path = out_dir.join("stderr.txt");
    let analysis_input_path = out_dir.join("analysis_input.json");
    let analysis_output_path = out_dir.join("analysis_output.json");
    let report_json_path = out_dir.join("report.json");
    let report_md_path = out_dir.join("report.md");

    create_empty_file_if_missing(&stdout_path).await?;
    create_empty_file_if_missing(&stderr_path).await?;
    create_empty_file_if_missing(&analysis_input_path).await?;
    create_empty_file_if_missing(&analysis_output_path).await?;
    create_empty_file_if_missing(&report_json_path).await?;
    create_empty_file_if_missing(&report_md_path).await?;
    Ok(())
}

async fn create_empty_file_if_missing(path: &Path) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create parent directory: {}", parent.display()))?;
    }
    tokio::fs::write(path, &[])
        .await
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    Ok(())
}

async fn run_child_ralph_run(
    repo_root: &Path,
    config_arg: &str,
    record_session: &Path,
    out_dir: &Path,
) -> Result<std::process::ExitStatus> {
    // 说明:
    // - 按 spec: current_dir 必须为 repo_root(或 repo_dir),确保 worktree/.ralph 路径都落在目标仓库内.
    // - 强制 `--no-tui` 与 `--record-session`,保证 headless + JSONL 主证据源.
    let exe = std::env::current_exe().with_context(|| "Failed to resolve current_exe")?;
    let mut cmd = Command::new(exe);
    cmd.arg("run")
        .arg("--config")
        .arg(config_arg)
        .arg("--no-tui")
        .arg("--record-session")
        .arg(record_session)
        .current_dir(repo_root)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    info!(
        "Autopilot spawning child run: cwd={} record_session={}",
        repo_root.display(),
        record_session.display()
    );

    let mut child = cmd
        .spawn()
        .with_context(|| "Failed to spawn child `ralph run`")?;

    let stdout_path = out_dir.join("stdout.txt");
    let stderr_path = out_dir.join("stderr.txt");

    let mut stdout_file = tokio::fs::File::create(&stdout_path)
        .await
        .with_context(|| format!("Failed to create {}", stdout_path.display()))?;
    let mut stderr_file = tokio::fs::File::create(&stderr_path)
        .await
        .with_context(|| format!("Failed to create {}", stderr_path.display()))?;

    let mut child_stdout = child
        .stdout
        .take()
        .context("Child stdout missing (piped)")?;
    let mut child_stderr = child
        .stderr
        .take()
        .context("Child stderr missing (piped)")?;

    // tee: 同时写入文件与父进程 stdout/stderr.
    let stdout_task =
        tokio::spawn(async move { tee_stream(&mut child_stdout, &mut stdout_file, true).await });
    let stderr_task =
        tokio::spawn(async move { tee_stream(&mut child_stderr, &mut stderr_file, false).await });

    let status = child
        .wait()
        .await
        .with_context(|| "Failed to wait for child `ralph run`")?;

    stdout_task.await.context("stdout tee task join failed")??;
    stderr_task.await.context("stderr tee task join failed")??;

    Ok(status)
}

async fn tee_stream<R: tokio::io::AsyncRead + Unpin>(
    reader: &mut R,
    file: &mut tokio::fs::File,
    to_stdout: bool,
) -> Result<()> {
    // 说明:
    // - 这里按 chunk 复制,避免依赖换行.
    // - 同时写入文件与当前进程的 stdout/stderr,实现类似 `tee` 的效果.
    let mut buf = [0u8; 8192];
    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n]).await?;
        if to_stdout {
            tokio::io::stdout().write_all(&buf[..n]).await?;
        } else {
            tokio::io::stderr().write_all(&buf[..n]).await?;
        }
    }
    file.flush().await?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// JSONL parsing + hard verdict
// ─────────────────────────────────────────────────────────────────────────────

fn parse_record_session(path: &Path) -> Result<RecordSessionSummary> {
    // 说明:
    // - strict parse: 这份 JSONL 是 autopilot 的主证据源,必须 fail-closed.
    // - 解析/聚合口径由共享模块维护,避免 autopilot 与其他 record 工具漂移.
    let player = crate::record_session::load_session_player_strict(path)?;
    let agg = crate::record_session::aggregate_record_session(&player)
        .with_context(|| format!("Failed to aggregate record-session: {}", path.display()))?;

    let required_topics: [&str; 6] = [
        "experiment.task",
        "experiment.result",
        "experiment.reviewed",
        "integration.task",
        "integration.applied",
        "experiment.complete",
    ];
    let banned_topics: [&str; 4] = [
        "gate.request",
        "gate.resolve",
        "gate.timeout",
        "routing.escalate",
    ];

    // 记录 required topic 的命中证据(按 topic -> evidence refs).
    let mut required_hits: BTreeMap<&str, Vec<EvidenceRef>> = BTreeMap::new();
    for t in required_topics {
        required_hits.insert(t, Vec::new());
    }

    // banned topic 命中.
    let mut banned_hits: BTreeMap<&str, Vec<EvidenceRef>> = BTreeMap::new();
    for t in banned_topics {
        banned_hits.insert(t, Vec::new());
    }

    // 额外硬断言: commit 与 evidence_ok 的缺失证据.
    let mut experiment_result_missing_commit: Vec<EvidenceRef> = Vec::new();
    let mut experiment_reviewed_bad_evidence_ok: Vec<EvidenceRef> = Vec::new();
    let mut commits: Vec<String> = Vec::new();

    let mut banned_topics_hit: BTreeSet<String> = BTreeSet::new();

    // 终止原因(来自 _meta.termination.reason).
    let termination_reason = agg.termination.as_ref().and_then(|t| t.reason.clone());
    let mut termination_evidence: Vec<EvidenceRef> = Vec::new();
    if termination_reason.is_some()
        && let Some(idx) = agg.termination_record_index
    {
        termination_evidence.push(EvidenceRef {
            record_index: idx,
            record_event: "_meta.termination".to_string(),
            topic: None,
        });
    }

    for (idx, rec) in player.records().iter().enumerate() {
        let event_type = rec.record.event.as_str();

        // 1) bus.publish: topic/payload.
        if event_type == "bus.publish" {
            let evt: Event =
                serde_json::from_value(rec.record.data.clone()).with_context(|| {
                    format!("Failed to parse bus.publish data as Event at record[{idx}]")
                })?;
            let topic = evt.topic.as_str().to_string();

            // required topics: 记录命中.
            if let Some(hit_list) = required_hits.get_mut(topic.as_str()) {
                hit_list.push(EvidenceRef {
                    record_index: idx,
                    record_event: "bus.publish".to_string(),
                    topic: Some(topic.clone()),
                });
            }

            // banned topics: 命中即记录.
            if let Some(hit_list) = banned_hits.get_mut(topic.as_str()) {
                banned_topics_hit.insert(topic.clone());
                hit_list.push(EvidenceRef {
                    record_index: idx,
                    record_event: "bus.publish".to_string(),
                    topic: Some(topic.clone()),
                });
            }

            // commit: experiment.result payload 必须包含 commit.
            if topic == "experiment.result" {
                match extract_string_field_from_payload(&evt.payload, "commit") {
                    Some(commit) if !commit.trim().is_empty() => commits.push(commit),
                    _ => experiment_result_missing_commit.push(EvidenceRef {
                        record_index: idx,
                        record_event: "bus.publish".to_string(),
                        topic: Some(topic.clone()),
                    }),
                }
            }

            // evidence_ok: experiment.reviewed payload 必须包含 evidence_ok=true.
            if topic == "experiment.reviewed" {
                match extract_bool_field_from_payload(&evt.payload, "evidence_ok") {
                    Some(true) => {}
                    _ => experiment_reviewed_bad_evidence_ok.push(EvidenceRef {
                        record_index: idx,
                        record_event: "bus.publish".to_string(),
                        topic: Some(topic.clone()),
                    }),
                }
            }

            continue;
        }
    }
    let terminal_tail = agg.stdout_tail;
    let topic_counts = agg.topic_counts;
    let topic_timeline = agg.topic_timeline;

    // 硬断言组装.
    let mut assertions: Vec<HardAssertion> = Vec::new();

    // required topics: 每个 topic 都要至少出现一次.
    for topic in required_topics {
        let hits = required_hits.get(topic).cloned().unwrap_or_default();
        let passed = !hits.is_empty();
        assertions.push(HardAssertion {
            name: format!("required_topic:{topic}"),
            passed,
            expected: serde_json::json!({ "topic": topic, "count": ">=1" }),
            actual: serde_json::json!({ "count": hits.len() }),
            evidence_refs: hits,
        });
    }

    // banned topics: 任一出现即失败.
    let mut banned_hit_topics: Vec<String> = Vec::new();
    let mut banned_evidence: Vec<EvidenceRef> = Vec::new();
    for topic in banned_topics {
        let hits = banned_hits.get(topic).cloned().unwrap_or_default();
        if !hits.is_empty() {
            banned_hit_topics.push(topic.to_string());
            banned_evidence.extend(hits);
        }
    }
    assertions.push(HardAssertion {
        name: "banned_topics_absent".to_string(),
        passed: banned_hit_topics.is_empty(),
        expected: serde_json::json!({ "banned": banned_topics }),
        actual: serde_json::json!({ "hit": banned_hit_topics }),
        evidence_refs: banned_evidence,
    });

    // commit: 每条 experiment.result 都必须带 commit.
    assertions.push(HardAssertion {
        name: "experiment.result_has_commit".to_string(),
        passed: experiment_result_missing_commit.is_empty(),
        expected: serde_json::json!({ "topic": "experiment.result", "field": "commit", "required": true }),
        actual: serde_json::json!({ "missing_commit_records": experiment_result_missing_commit.len(), "commits": commits }),
        evidence_refs: experiment_result_missing_commit,
    });

    // evidence_ok: experiment.reviewed 必须 evidence_ok=true.
    assertions.push(HardAssertion {
        name: "experiment.reviewed_evidence_ok_true".to_string(),
        passed: experiment_reviewed_bad_evidence_ok.is_empty(),
        expected: serde_json::json!({ "topic": "experiment.reviewed", "field": "evidence_ok", "value": true }),
        actual: serde_json::json!({ "bad_records": experiment_reviewed_bad_evidence_ok.len() }),
        evidence_refs: experiment_reviewed_bad_evidence_ok,
    });

    // termination reason: 必须 CompletionPromise.
    let term_passed = termination_reason.as_deref() == Some("CompletionPromise");
    assertions.push(HardAssertion {
        name: "termination_reason_completion_promise".to_string(),
        passed: term_passed,
        expected: serde_json::json!({ "reason": "CompletionPromise" }),
        actual: serde_json::json!({ "reason": termination_reason }),
        evidence_refs: termination_evidence,
    });

    let passed = assertions.iter().all(|a| a.passed);
    let hard_verdict = HardVerdict { passed, assertions };

    Ok(RecordSessionSummary {
        hard_verdict,
        termination_reason,
        commits,
        banned_topics_hit,
        topic_counts,
        topic_timeline,
        terminal_tail,
        parse_error: None,
    })
}

fn record_session_summary_from_parse_error(err: &anyhow::Error) -> RecordSessionSummary {
    // 说明:
    // - 当 JSONL 无法解析/文件不存在时,我们仍希望生成 report,并用硬断言失败表达.
    // - 这比直接 panic/返回 Err 更适合无人值守调用(有稳定退出码 + 可读报告).
    let msg = format!("{err:#}");
    let hard_verdict = HardVerdict {
        passed: false,
        assertions: vec![HardAssertion {
            name: "record_session_readable".to_string(),
            passed: false,
            expected: serde_json::json!({ "readable": true }),
            actual: serde_json::json!({ "error": msg }),
            evidence_refs: Vec::new(),
        }],
    };

    RecordSessionSummary {
        hard_verdict,
        termination_reason: None,
        commits: Vec::new(),
        banned_topics_hit: BTreeSet::new(),
        topic_counts: BTreeMap::new(),
        topic_timeline: Vec::new(),
        terminal_tail: String::new(),
        parse_error: Some(format!("{err}")),
    }
}

fn extract_string_field_from_payload(payload: &str, field: &str) -> Option<String> {
    // 说明:
    // - payload 可能是 JSON 或 YAML,也可能是“半结构化文本”.
    // - 优先按 JSON/YAML 解析;失败后再做一次轻量 regex 回退,尽量减少误判.
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(s) = v.get(field).and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }

    if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(payload) {
        if let Some(s) = v.get(field).and_then(|v| v.as_str()) {
            return Some(s.to_string());
        }
    }

    // fallback: 支持 `commit: xxx` 或 `"commit": "xxx"` 的常见片段.
    let re = regex::Regex::new(&format!(
        r#"(?m)^\s*{}\s*[:=]\s*([^\s#]+)\s*$"#,
        regex::escape(field)
    ))
    .ok()?;
    re.captures(payload)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str().to_string())
}

fn extract_bool_field_from_payload(payload: &str, field: &str) -> Option<bool> {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(payload) {
        if let Some(b) = v.get(field).and_then(|v| v.as_bool()) {
            return Some(b);
        }
    }

    if let Ok(v) = serde_yaml::from_str::<serde_yaml::Value>(payload) {
        if let Some(b) = v.get(field).and_then(|v| v.as_bool()) {
            return Some(b);
        }
        // YAML 里也可能出现 "true"/"false" 字符串.
        if let Some(s) = v.get(field).and_then(|v| v.as_str()) {
            return match s.trim().to_ascii_lowercase().as_str() {
                "true" => Some(true),
                "false" => Some(false),
                _ => None,
            };
        }
    }

    // fallback: 文本里出现 evidence_ok=true.
    let re = regex::Regex::new(&format!(
        r#"(?i)\b{}\s*[:=]\s*(true|false)\b"#,
        regex::escape(field)
    ))
    .ok()?;
    re.captures(payload).and_then(|c| c.get(1)).and_then(|m| {
        match m.as_str().to_ascii_lowercase().as_str() {
            "true" => Some(true),
            "false" => Some(false),
            _ => None,
        }
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Evidence pack + agent analysis
// ─────────────────────────────────────────────────────────────────────────────

async fn write_analysis_input_json(
    path: &Path,
    record_session: &Path,
    summary: &RecordSessionSummary,
) -> Result<()> {
    // 说明:
    // - 证据包必须可审计: 把硬断言与关键摘要写入 JSON,并做体积预算.
    // - 这里尽量不塞原始大 payload,而是写计数/顺序/commit 列表/terminal tail.
    const MAX_TAIL_BYTES: usize = 64 * 1024;
    const MAX_TIMELINE: usize = 400;

    let tail = ralph_core::truncate_to_budget(&summary.terminal_tail, MAX_TAIL_BYTES);
    let timeline = summary
        .topic_timeline
        .iter()
        .take(MAX_TIMELINE)
        .cloned()
        .collect::<Vec<_>>();

    let value = serde_json::json!({
        "record_session": record_session,
        "hard_verdict": &summary.hard_verdict,
        "termination_reason": summary.termination_reason,
        "commits": summary.commits,
        "banned_topics_hit": summary.banned_topics_hit,
        "topic_counts": summary.topic_counts,
        "topic_timeline": timeline,
        "terminal_tail": tail,
        "parse_error": summary.parse_error,
    });

    write_json_pretty(path, &value).await
}

async fn run_agent_analysis(
    config_path: &PathBuf,
    repo_dir: &Path,
    out_dir: &Path,
    analysis_input_path: &Path,
    analysis_args: &AutopilotAnalysisArgs,
) -> Result<AgentAnalysisOutput> {
    // 说明:
    // - agent 分析使用子进程 `ralph run --no-tui` 执行,保证与真实 CLI 行为一致.
    // - 子进程必须运行在隔离 workspace 中,否则会把 repo 根 `.agent` 当成自己的状态目录.
    let analysis_workspace = prepare_agent_analysis_workspace(out_dir).await?;
    let analysis_config_path = out_dir.join("analysis_ralph.yml");
    let analysis_prompt_path = out_dir.join("analysis_prompt.md");

    let analysis_input = tokio::fs::read_to_string(analysis_input_path)
        .await
        .with_context(|| format!("Failed to read {}", analysis_input_path.display()))?;

    // backend 选择:
    // - 默认: 跟随主 config 的 `cli`(包含 custom backend 的 command/args).
    // - 覆盖: 若显式提供 --analysis-backend,仅覆盖 backend 字段,其余字段尽量沿用主 config.
    //
    // 关键动机:
    // - 很多配置会用 `backend=custom + command=codex + args=[...]` 来表达“同一后端但附加参数”.
    // - 若只写 backend 而丢失 command/args,会导致 agent 分析子进程在 config validate 阶段直接失败.
    let mut analysis_cli = match load_config_for_repo(config_path, repo_dir).await {
        Ok((cfg, _)) => cfg.cli,
        Err(_) => RalphConfig::default().cli,
    };

    if let Some(backend_override) = analysis_args.analysis_backend.clone() {
        analysis_cli.backend = backend_override;
    }

    // agent 分析子进程必须 headless.
    analysis_cli.default_mode = "autonomous".to_string();

    if analysis_cli.backend == "custom"
        && analysis_cli.command.as_ref().is_none_or(String::is_empty)
    {
        anyhow::bail!(
            "agent analysis backend=custom,但 cli.command 缺失.\n\n修复建议:\n- 在主配置里设置 `cli.command`(例如 command: \"codex\"),或\n- 用 `--analysis-backend <claude|codex|...>` 改用非 custom 后端."
        );
    }

    let analysis_config = build_min_analysis_config_yaml(
        &analysis_cli,
        analysis_prompt_path.to_string_lossy().as_ref(),
    );
    tokio::fs::write(&analysis_config_path, analysis_config)
        .await
        .with_context(|| format!("Failed to write {}", analysis_config_path.display()))?;

    let prompt = build_analysis_prompt_markdown(&analysis_input);
    tokio::fs::write(&analysis_prompt_path, prompt)
        .await
        .with_context(|| format!("Failed to write {}", analysis_prompt_path.display()))?;

    // 执行子进程分析.
    let exe = std::env::current_exe().with_context(|| "Failed to resolve current_exe")?;
    let invocation = build_agent_analysis_invocation(&analysis_workspace, &analysis_config_path);
    let mut cmd = Command::new(exe);
    cmd.args(&invocation.args)
        .current_dir(&invocation.cwd)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped());

    info!(
        "Autopilot spawning agent analysis: backend={} cwd={} repo_dir={}",
        analysis_cli.backend,
        invocation.cwd.display(),
        repo_dir.display()
    );

    let output = cmd
        .output()
        .await
        .with_context(|| "Failed to run agent analysis `ralph run`")?;

    // 从 stdout 中解析 `<event topic="analyze.complete">...</event>`.
    let stdout_text = String::from_utf8_lossy(&output.stdout);
    let parsed = parse_agent_analysis_output_from_stdout(&stdout_text)?;

    // 说明:
    // - agent analysis 的“核心产物”是 analyze.complete 的结构化 JSON.
    // - 子进程可能因为 max_iterations/max_runtime 等护栏退出(标准 exit code=2),
    //   但只要 analyze.complete 已成功产出并可解析,autopilot 仍可判定为分析成功.
    if !output.status.success() {
        let code = output.status.code();
        let stderr_text = String::from_utf8_lossy(&output.stderr);

        if code == Some(2) {
            warn!(
                "Agent analysis child exited with code=2 (limit), but analyze.complete parsed successfully. stderr_len={}",
                stderr_text.len()
            );
        } else {
            anyhow::bail!(
                "agent analysis process exited non-zero: {:?}\n\nstderr:\n{}",
                code,
                stderr_text
            );
        }
    }

    Ok(parsed)
}

fn parse_agent_analysis_output_from_stdout(stdout_text: &str) -> Result<AgentAnalysisOutput> {
    let json = EventParser::extract_last_payload_for_topic(stdout_text, "analyze.complete")
        .with_context(
            || "Failed to find `<event topic=\"analyze.complete\">...` in analysis stdout",
        )?;

    serde_json::from_str(&json)
        .with_context(|| format!("Invalid JSON in analyze.complete payload: {json}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentAnalysisInvocation {
    cwd: PathBuf,
    args: Vec<OsString>,
}

async fn prepare_agent_analysis_workspace(out_dir: &Path) -> Result<PathBuf> {
    // 说明:
    // - analysis 子进程不能在 repo 根运行,否则 `ralph run` 会把 repo 根当成 workspace_root。
    // - 这里每次都重建独立目录,顺便清掉旧 `.agent` / scratchpad / 其他分析残留。
    let workspace = out_dir.join("analysis-workspace");
    if workspace.exists() {
        tokio::fs::remove_dir_all(&workspace)
            .await
            .with_context(|| format!("Failed to clean {}", workspace.display()))?;
    }
    tokio::fs::create_dir_all(workspace.join(".agent"))
        .await
        .with_context(|| format!("Failed to create {}", workspace.display()))?;
    Ok(workspace)
}

fn build_agent_analysis_invocation(
    workspace: &Path,
    analysis_config_path: &Path,
) -> AgentAnalysisInvocation {
    AgentAnalysisInvocation {
        cwd: workspace.to_path_buf(),
        args: build_agent_analysis_run_args(analysis_config_path),
    }
}

fn build_agent_analysis_run_args(analysis_config_path: &Path) -> Vec<OsString> {
    // 注意: `ralph run` 的 `--no-tui` 与 `--autonomous` 是互斥参数.
    // 这里用 `--no-tui` 保证 headless,并依赖 analysis_ralph.yml 的 `cli.default_mode=autonomous`.
    vec![
        OsString::from("run"),
        OsString::from("--config"),
        analysis_config_path.as_os_str().to_os_string(),
        OsString::from("--no-tui"),
        OsString::from("--completion-promise"),
        OsString::from("ANALYSIS_COMPLETE"),
    ]
}

fn build_min_analysis_config_yaml(cli: &CliConfig, prompt_file: &str) -> String {
    // 说明:
    // - starting_event 让 ralph#1 在协调后发布 analyze.start,从而触发 analyzer hat.
    // - completion_promise 在子进程 CLI 里用 --completion-promise 覆盖,这里不强依赖.
    // - 这里的 YAML 内容尽量小,减少“提示词污染”.
    //
    // 注意:
    // - 当 backend=custom 时,必须把 command/args 一并写入,否则 config validate 会直接失败.
    let mut cli_lines = String::new();
    cli_lines.push_str(&format!(
        "  backend: \"{}\"\n",
        yaml_escape_double_quoted(&cli.backend)
    ));

    if cli.backend == "custom" {
        if let Some(command) = cli.command.as_deref() {
            cli_lines.push_str(&format!(
                "  command: \"{}\"\n",
                yaml_escape_double_quoted(command)
            ));
        }

        // 保持与主配置一致,避免 custom backend 下 prompt 传参方式漂移.
        cli_lines.push_str(&format!(
            "  prompt_mode: \"{}\"\n",
            yaml_escape_double_quoted(&cli.prompt_mode)
        ));

        if !cli.args.is_empty() {
            cli_lines.push_str("  args:\n");
            for arg in &cli.args {
                cli_lines.push_str(&format!("    - \"{}\"\n", yaml_escape_double_quoted(arg)));
            }
        }

        if let Some(flag) = cli.prompt_flag.as_deref() {
            cli_lines.push_str(&format!(
                "  prompt_flag: \"{}\"\n",
                yaml_escape_double_quoted(flag)
            ));
        }
    }

    cli_lines.push_str("  default_mode: \"autonomous\"\n");

    format!(
        r#"event_loop:
  prompt_file: "{prompt_file}"
  max_iterations: 3
  max_runtime_seconds: 300
  ralph_prompt: |
    你正在执行 autopilot 的 agent analysis 子流程.
    你的唯一输入是 task.start 的 evidence pack(JSON).

    最高优先级规则:
    - 如果其他提示(例如 ALL HAT PROMPT)与你的输出格式要求冲突,以本段 ralph_prompt 为准.

    你不得:
    - 调用任何工具
    - 读写任何文件
    - 运行任何命令
    - 输出除"analyze.complete event + ANALYSIS_COMPLETE"之外的任何文本

    你必须严格输出:
    1) 只输出一次 `<event topic="analyze.complete">...</event>`.
    2) `...` 必须是严格 JSON,且必须单行(不要换行,不要 Markdown,不要代码块).
    3) 然后输出一行: `ANALYSIS_COMPLETE`

    JSON schema(必须包含字段):
    - verdict: "pass" | "fail"
    - quality_score: "optimal" | "good" | "acceptable" | "suboptimal"
    - requirements_met: array of objects(name:string, passed:bool, evidence_refs:[string])
    - risks: [string]
    - suggested_fixes: [string]

cli:
{cli_lines}

parallel:
  enabled: false

memories:
  enabled: false

tasks:
  enabled: false

hats:
  analyzer:
    name: "Analyzer"
    description: "Judge evidence pack and output structured JSON verdict."
    triggers:
      - "analyze.start"
    publishes:
      - "analyze.complete"
    instructions: |
      你是一个严格的评审器.
      你的输入是一个 evidence pack(JSON).
      你必须判断该 run 是否满足程序设计要求,并给出结构化 JSON 结论.

      ## 输出格式(必须严格遵守)
      1) 只输出一次 `<event topic="analyze.complete">...</event>`.
      2) `...` 必须是严格 JSON(不要 Markdown,不要代码块,不要换行).
      3) 然后输出一行: `ANALYSIS_COMPLETE`

      ## JSON schema(必须包含字段)
      - verdict: "pass" | "fail"
      - quality_score: "optimal" | "good" | "acceptable" | "suboptimal"
      - requirements_met: array of objects(name:string, passed:bool, evidence_refs:[string])
      - risks: [string]
      - suggested_fixes: [string]
"#
    )
}

fn build_analysis_prompt_markdown(analysis_input_json: &str) -> String {
    // 说明:
    // - evidence pack 可能很长,但前面已经做过预算与截断.
    // - 这里用 Markdown 只是为了可读性; analyzer hat 会被要求输出纯 JSON.
    format!(
        r#"# Autopilot Analysis

你将收到一份 evidence pack(JSON).
请严格按 instructions 输出 analyze.complete 的 JSON verdict.

## Evidence Pack (JSON)
{analysis_input_json}
"#
    )
}

fn yaml_escape_double_quoted(value: &str) -> String {
    // 说明:
    // - 这里用于生成 YAML 的双引号字符串,避免引入额外依赖或把格式化逻辑分散到多处.
    // - 我们只做最关键的转义,确保 command/args/prompt_flag 包含特殊字符时依然可解析.
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}

fn map_agent_output_to_exit_code(output: &AgentAnalysisOutput) -> (AutopilotExitCode, String) {
    // 规则:
    // - verdict=fail => exit 2
    // - quality_score=suboptimal => exit 2
    // - 否则 exit 0
    if output.verdict == AgentVerdict::Fail {
        return (
            AutopilotExitCode::AgentFail,
            "硬断言通过,但 agent verdict=fail".to_string(),
        );
    }
    if output.quality_score == QualityScore::Suboptimal {
        return (
            AutopilotExitCode::AgentFail,
            "硬断言通过,但 quality_score=suboptimal".to_string(),
        );
    }
    (
        AutopilotExitCode::Pass,
        "硬断言通过,且 agent verdict=pass".to_string(),
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// Report writing
// ─────────────────────────────────────────────────────────────────────────────

async fn write_reports(report: &AutopilotReportJson) -> Result<()> {
    // 说明:
    // - report.json: 机器可读.
    // - report.md: 人类可读(失败时优先看的证据路径).
    let report_json_path = report.out_dir.join("report.json");
    let report_md_path = report.out_dir.join("report.md");

    write_json_pretty(&report_json_path, report).await?;
    tokio::fs::write(&report_md_path, render_report_markdown(report))
        .await
        .with_context(|| format!("Failed to write {}", report_md_path.display()))?;
    Ok(())
}

fn render_report_markdown(report: &AutopilotReportJson) -> String {
    // 说明:
    // - 这里不追求花哨排版,重点是失败时“下一步看什么证据”.
    // - Markdown 更适合在 CI artifact 或本地直接打开阅读.
    let mut md = String::new();
    md.push_str("# Autopilot Report\n\n");
    md.push_str(&format!("- Mode: `{}`\n", report.mode));
    md.push_str(&format!(
        "- Exit: `{}` (`{}`) ({})\n",
        report.exit_code, report.exit_code_semantic, report.exit_reason
    ));
    if let Some(code) = report.child_status {
        md.push_str(&format!("- Child status: `{}`\n", code));
    }
    md.push_str(&format!("- Repo: `{}`\n", report.repo_dir.display()));
    md.push_str(&format!(
        "- Record session: `{}`\n",
        report.record_session.display()
    ));
    md.push_str(&format!("- Out dir: `{}`\n\n", report.out_dir.display()));

    // 说明:
    // - 若存在 child_ralph.yml,说明 autopilot run 期间启用了“子进程配置派生”(例如限并发).
    // - 该文件仅用于本次 child `ralph run`,不会影响用户直接运行 `ralph run` 的默认语义.
    let child_cfg_path = report.out_dir.join("child_ralph.yml");
    if child_cfg_path.exists() {
        md.push_str(&format!(
            "- Child config (autopilot derived): `{}`\n\n",
            child_cfg_path.display()
        ));
    }

    md.push_str("## Hard Verdict\n\n");
    md.push_str(&format!(
        "- Passed: `{}`\n\n",
        if report.hard_verdict.passed {
            "true"
        } else {
            "false"
        }
    ));
    for a in &report.hard_verdict.assertions {
        md.push_str(&format!(
            "- `{}`: `{}`\n",
            a.name,
            if a.passed { "PASS" } else { "FAIL" }
        ));
    }

    // 说明:
    // - report.json 更偏“结构化结果”,而 report.md 更偏“打开就能看懂”.
    // - 因此我们在 report.md 里补充两类“辅助审计”的摘要:
    //   1) topic_counts: 观察重复派发/噪声是否上升(不会改变 verdict).
    //   2) 并行度指标: 从 stdout 状态机日志推断 max_concurrent_running(不会改变 verdict).
    if let Some(topic_counts) = try_load_topic_counts(&report.out_dir) {
        md.push_str("\n## Topic Counts\n\n");
        md.push_str("关键 topic 的出现次数(用于观察重复派发/噪声,不作为 verdict 门槛):\n\n");

        // 说明: 这里固定顺序,便于人工对比多次 run 的变化.
        let topics = [
            "experiment.task",
            "experiment.result",
            "experiment.reviewed",
            "integration.task",
            "integration.applied",
            "integration.rejected",
            "integration.blocked",
            "experiment.complete",
        ];
        for t in topics {
            let count = topic_counts.get(t).copied().unwrap_or(0);
            md.push_str(&format!("- `{t}`: `{count}`\n"));
        }
    }

    if let Some(metrics) =
        try_summarize_parallel_runner_concurrency(&report.out_dir.join("stdout.txt"))
    {
        md.push_str("\n## Parallelism (stdout state machine)\n\n");
        md.push_str("从 stdout 的 `[experiment_runner#*:state]` 状态机日志推断的并行度指标:\n\n");
        md.push_str(&format!(
            "- unique_runner_instances_seen: `{}`\n",
            metrics.unique_runner_instances_seen
        ));
        md.push_str(&format!(
            "- runner_instances_entered_running: `{}`\n",
            metrics.runner_instances_entered_running
        ));
        md.push_str(&format!(
            "- max_concurrent_running: `{}`\n",
            metrics.max_concurrent_running
        ));
        md.push_str(&format!(
            "- entered_running: `{:?}`\n",
            metrics.entered_running
        ));
    }

    md.push_str("\n## Agent Analysis\n\n");
    md.push_str(&format!("- Status: `{:?}`\n", report.agent.status));
    if let Some(reason) = &report.agent.reason {
        md.push_str(&format!("- Reason: `{}`\n", reason));
    }
    if let Some(output) = &report.agent.output {
        md.push_str(&format!("- Verdict: `{:?}`\n", output.verdict));
        md.push_str(&format!("- Quality: `{:?}`\n", output.quality_score));
    }

    md.push_str("\n## Evidence Files\n\n");
    md.push_str("失败时建议按顺序查看:\n\n");
    md.push_str(&format!(
        "- `{}`\n",
        report.out_dir.join("report.json").display()
    ));
    md.push_str(&format!(
        "- `{}`\n",
        report.out_dir.join("analysis_input.json").display()
    ));
    md.push_str(&format!(
        "- `{}`\n",
        report.out_dir.join("analysis_output.json").display()
    ));
    md.push_str(&format!(
        "- `{}`\n",
        report.out_dir.join("stdout.txt").display()
    ));
    md.push_str(&format!(
        "- `{}`\n",
        report.out_dir.join("stderr.txt").display()
    ));
    md
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers for report.md (best-effort; failures must not fail autopilot)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AnalysisInputPreview {
    /// 说明: analysis_input.json 的字段名是对外协议,这里保持一致.
    #[serde(default)]
    topic_counts: BTreeMap<String, usize>,
}

fn try_load_topic_counts(out_dir: &Path) -> Option<BTreeMap<String, usize>> {
    // 说明:
    // - report.md 的渲染是“尽力而为”的,任何读取失败都不应影响最终 verdict/退出码.
    // - topic_counts 来自 analysis_input.json(由 autopilot analyze 阶段生成).
    let path = out_dir.join("analysis_input.json");
    let content = std::fs::read_to_string(path).ok()?;
    let preview: AnalysisInputPreview = serde_json::from_str(&content).ok()?;
    if preview.topic_counts.is_empty() {
        return None;
    }
    Some(preview.topic_counts)
}

#[derive(Debug, Clone)]
struct ParallelRunnerMetrics {
    unique_runner_instances_seen: usize,
    runner_instances_entered_running: usize,
    max_concurrent_running: usize,
    entered_running: Vec<String>,
}

fn try_summarize_parallel_runner_concurrency(stdout_path: &Path) -> Option<ParallelRunnerMetrics> {
    // 说明:
    // - 并发度指标更适合从 stdout 的状态机日志里提取,无需重新解析 record-session JSONL.
    // - 我们只做简单匹配: "[experiment_runner#N:state] running/idle/done".
    let file = std::fs::File::open(stdout_path).ok()?;
    let reader = BufReader::new(file);

    let mut states: BTreeMap<String, String> = BTreeMap::new();
    let mut entered_running: BTreeSet<String> = BTreeSet::new();
    let mut max_running = 0usize;

    for raw in std::io::BufRead::lines(reader) {
        let line = raw.ok()?;
        let line = line.trim_end();
        let Some(rest) = line.strip_prefix('[') else {
            continue;
        };
        let Some((head, tail)) = rest.split_once(']') else {
            continue;
        };
        let Some((instance_id, tag)) = head.split_once(':') else {
            continue;
        };
        if tag != "state" || !instance_id.starts_with("experiment_runner#") {
            continue;
        }

        let state = tail.trim();
        if state.is_empty() {
            continue;
        }

        states.insert(instance_id.to_string(), state.to_string());
        if state == "running" {
            entered_running.insert(instance_id.to_string());
        }

        // 说明: 这里是 O(n) 统计,但 stdout 文件通常不大,足够稳定易懂.
        let running_now = states.values().filter(|s| s.as_str() == "running").count();
        max_running = max_running.max(running_now);
    }

    if states.is_empty() {
        return None;
    }

    Some(ParallelRunnerMetrics {
        unique_runner_instances_seen: states.len(),
        runner_instances_entered_running: entered_running.len(),
        max_concurrent_running: max_running,
        entered_running: entered_running.into_iter().collect(),
    })
}

async fn write_json_pretty(path: &Path, value: &impl Serialize) -> Result<()> {
    let content = serde_json::to_string_pretty(value)?;
    tokio::fs::write(path, content)
        .await
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(name)
    }

    #[test]
    fn derive_child_config_parallel_cap_does_not_mutate_base() -> Result<()> {
        // 说明:
        // - 这个测试覆盖“强隔离”约束: autopilot 生成派生 config 时,不得修改原始 config.
        // - 否则会导致后续逻辑在同一进程内复用 config 时出现隐式漂移,也更难审计.
        let mut base = RalphConfig::default();
        base.parallel.enabled = true;
        base.parallel.autoscale.max_running_jobs = 7;

        let derived = derive_child_config_with_parallel_max_running_jobs(&base, 2)?;
        assert_eq!(derived.parallel.autoscale.max_running_jobs, 2);
        assert_eq!(base.parallel.autoscale.max_running_jobs, 7);
        Ok(())
    }

    #[test]
    fn derive_child_config_parallel_cap_rejects_zero() {
        let base = RalphConfig::default();
        let err = derive_child_config_with_parallel_max_running_jobs(&base, 0).unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("child-parallel-max-running-jobs"),
            "error message should mention the flag for user guidance, got: {msg}"
        );
    }

    #[test]
    fn parse_record_session_pass_fixture() -> Result<()> {
        let path = fixture_path("autopilot_pass.jsonl");
        let summary = parse_record_session(&path)?;
        assert!(summary.hard_verdict.passed, "pass fixture must hard-pass");
        Ok(())
    }

    #[test]
    fn parse_record_session_fail_fixture() -> Result<()> {
        let path = fixture_path("autopilot_fail.jsonl");
        let summary = parse_record_session(&path)?;
        assert!(!summary.hard_verdict.passed, "fail fixture must hard-fail");
        Ok(())
    }

    #[test]
    fn exit_code_mapping_agent_fail() {
        let output = AgentAnalysisOutput {
            verdict: AgentVerdict::Fail,
            quality_score: QualityScore::Good,
            requirements_met: vec![],
            risks: vec![],
            suggested_fixes: vec![],
        };
        let (code, _) = map_agent_output_to_exit_code(&output);
        assert_eq!(code, AutopilotExitCode::AgentFail);
    }

    #[test]
    fn report_json_contains_key_fields() -> Result<()> {
        let report = AutopilotReportJson {
            schema_version: "autopilot-report@v1".to_string(),
            mode: "analyze".to_string(),
            repo_dir: PathBuf::from("/tmp/repo"),
            record_session: PathBuf::from("/tmp/repo/session.jsonl"),
            out_dir: PathBuf::from("/tmp/repo/out"),
            hard_verdict: HardVerdict {
                passed: true,
                assertions: vec![],
            },
            agent: AgentSection {
                status: AgentSectionStatus::Skipped,
                output: None,
                reason: Some("--skip-agent-analysis".to_string()),
            },
            exit_code: 0,
            exit_code_semantic: "Pass".to_string(),
            exit_reason: "ok".to_string(),
            child_status: None,
        };

        let v = serde_json::to_value(&report)?;
        assert!(v.get("record_session").is_some());
        assert!(v.get("hard_verdict").is_some());
        assert!(v.get("exit_code").is_some());
        Ok(())
    }

    #[test]
    fn parse_agent_analysis_output_from_stdout_uses_last_matching_event() -> Result<()> {
        let stdout = r#"
noise
<event topic="analyze.complete">
{"verdict":"pass","quality_score":"good","requirements_met":[],"risks":[],"suggested_fixes":["old"]}
</event>
more noise
<event id="latest" topic="analyze.complete">
{"verdict":"pass","quality_score":"optimal","requirements_met":[],"risks":[],"suggested_fixes":["new"]}
</event>
"#;

        let parsed = parse_agent_analysis_output_from_stdout(stdout)?;
        assert_eq!(parsed.verdict, AgentVerdict::Pass);
        assert_eq!(parsed.quality_score, QualityScore::Optimal);
        assert_eq!(parsed.suggested_fixes, vec!["new".to_string()]);
        Ok(())
    }

    #[test]
    fn parse_agent_analysis_output_from_stdout_errors_when_event_missing() {
        let err = parse_agent_analysis_output_from_stdout("no analysis event here").unwrap_err();
        let msg = format!("{err:#}");
        assert!(
            msg.contains("Failed to find `<event topic=\"analyze.complete\">"),
            "expected missing-event guidance, got: {msg}"
        );
    }

    #[test]
    fn agent_analysis_args_do_not_conflict() {
        let args = build_agent_analysis_run_args(Path::new("analysis.yml"));
        let args = args
            .into_iter()
            .map(|s| s.to_string_lossy().to_string())
            .collect::<Vec<_>>();

        assert!(
            args.iter().any(|a| a == "--no-tui"),
            "agent analysis must run headlessly via --no-tui"
        );
        assert!(
            !args.iter().any(|a| a == "--autonomous"),
            "agent analysis args must not include --autonomous (conflicts with --no-tui)"
        );
    }

    #[test]
    fn analysis_config_custom_backend_includes_command_and_args() -> Result<()> {
        // 说明:
        // - agent 分析默认会跟随主 config 的 backend.
        // - 当主 config 使用 custom backend 时,analysis_ralph.yml 必须携带 command/args,否则 validate 会失败.
        let cli = CliConfig {
            backend: "custom".to_string(),
            command: Some("codex".to_string()),
            prompt_mode: "arg".to_string(),
            default_mode: "autonomous".to_string(),
            idle_timeout_secs: 30,
            args: vec![
                "exec".to_string(),
                "--sandbox".to_string(),
                "danger-full-access".to_string(),
            ],
            prompt_flag: None,
        };

        let yaml = build_min_analysis_config_yaml(&cli, "/tmp/analysis_prompt.md");
        let cfg: RalphConfig = serde_yaml::from_str(&yaml)?;
        cfg.validate()?;

        assert_eq!(cfg.cli.backend, "custom");
        assert_eq!(cfg.cli.command.as_deref(), Some("codex"));
        assert_eq!(
            cfg.cli.args,
            vec![
                "exec".to_string(),
                "--sandbox".to_string(),
                "danger-full-access".to_string(),
            ]
        );
        Ok(())
    }

    #[test]
    fn analysis_config_disables_memories_and_tasks() -> Result<()> {
        let cli = CliConfig {
            backend: "claude".to_string(),
            command: None,
            prompt_mode: "arg".to_string(),
            default_mode: "autonomous".to_string(),
            idle_timeout_secs: 30,
            args: vec![],
            prompt_flag: None,
        };

        let yaml = build_min_analysis_config_yaml(&cli, "/tmp/analysis_prompt.md");
        let cfg: RalphConfig = serde_yaml::from_str(&yaml)?;
        cfg.validate()?;

        assert!(!cfg.memories.enabled, "analysis must disable memories");
        assert!(!cfg.tasks.enabled, "analysis must disable tasks");
        Ok(())
    }

    #[tokio::test]
    async fn agent_analysis_workspace_is_recreated_under_out_dir() -> Result<()> {
        let temp = tempfile::tempdir()?;
        let repo_dir = temp.path().join("repo");
        let out_dir = temp.path().join("out");
        std::fs::create_dir_all(repo_dir.join(".agent"))?;
        std::fs::create_dir_all(out_dir.join("analysis-workspace/.agent"))?;
        std::fs::write(repo_dir.join(".agent/memories.md"), "# repo memory")?;
        std::fs::write(
            out_dir.join("analysis-workspace/.agent/tasks.jsonl"),
            "{\"id\":\"stale\"}\n",
        )?;
        std::fs::write(out_dir.join("analysis-workspace/stale.txt"), "stale")?;

        let workspace = prepare_agent_analysis_workspace(&out_dir).await?;
        let invocation =
            build_agent_analysis_invocation(&workspace, &out_dir.join("analysis_ralph.yml"));

        assert_eq!(workspace, out_dir.join("analysis-workspace"));
        assert_eq!(invocation.cwd, workspace);
        assert_ne!(invocation.cwd, repo_dir);
        assert!(
            workspace.join(".agent").exists(),
            "isolated workspace should recreate .agent directory"
        );
        assert!(
            !workspace.join(".agent/tasks.jsonl").exists(),
            "stale task state must be removed"
        );
        assert!(
            !workspace.join("stale.txt").exists(),
            "stale analysis artifacts must be removed"
        );
        assert!(
            repo_dir.join(".agent/memories.md").exists(),
            "repo root state should remain untouched because analysis no longer runs there"
        );
        Ok(())
    }
}
