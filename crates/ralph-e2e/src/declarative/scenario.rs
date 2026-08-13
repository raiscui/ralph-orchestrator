//! 声明式场景: YAML 定义"测什么", runner 负责"怎么跑"。
//!
//! 试点范围: setup(config/prompt)+ 内置断言子集。
//! 复杂断言(注入时序/自定义检查)暂由现有命令式场景承担(逃生舱)。

use super::super::scenarios::Assertions;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use crate::{Backend, ScenarioError, TestScenario};
use async_trait::async_trait;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// 声明式场景定义(YAML 反序列化目标)。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeScenario {
    pub id: String,
    pub description: String,
    #[serde(default)]
    pub tier: String,
    /// 支持的 backend 名列表(空 = 全部)。
    #[serde(default)]
    pub backends: Vec<String>,
    pub setup: DeclarativeSetup,
    pub expect: DeclarativeExpect,
}

/// setup: 如何装配一次 run。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeSetup {
    /// ralph.yml 模板(支持 `{backend}` 占位符, 按 backend 名展开)。
    #[serde(default)]
    pub config: String,
    /// 内联 prompt。
    #[serde(default)]
    pub prompt: Option<String>,
    /// prompt 文件路径(相对场景目录)。
    #[serde(default)]
    pub prompt_file: Option<String>,
    #[serde(default)]
    pub max_iterations: Option<u32>,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
    /// 额外 CLI 参数(如 --no-tui / --idle-start)。
    #[serde(default)]
    pub extra_args: Vec<String>,
    /// prompt 来源: inline(默认)或 config(使用 event_loop.ralph_prompt)。
    #[serde(default)]
    pub prompt_source: Option<String>,
    /// 注入时序(parallel 场景: 在 ralph 运行期间并发执行)。
    #[serde(default)]
    pub inject: Vec<DeclarativeInjectStep>,
    /// 额外环境变量(透传给 ralph 进程)。
    #[serde(default)]
    pub env: std::collections::HashMap<String, String>,
    /// 将指定内容写入 workspace 的 PROMPT.md(prompt_source: config 且 ralph 读 prompt_file 时使用)。
    #[serde(default)]
    pub write_prompt_to: Option<String>,
    /// 附加文件写入(相对 workspace; 如 fake codex shim)。
    #[serde(default)]
    pub write_files: Vec<DeclarativeWriteFile>,
    /// 从仓库 examples 目录引用 example 场景(自动 patch cli 段)。
    #[serde(default)]
    pub example: Option<String>,
    /// 注入到 PATH 前部的 workspace 相对目录(如 .e2e/bin 放 fake codex shim)。
    #[serde(default)]
    pub path_prefix: Vec<String>,
}

/// 写入 workspace 的附加文件。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeWriteFile {
    /// 相对 workspace 的路径(如 .e2e/bin/codex)。
    pub path: String,
    /// 文件内容。
    pub content: String,
    /// 是否设置可执行位(unix)。
    #[serde(default)]
    pub executable: bool,
}

/// expect: 期望(映射到内置断言)。
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DeclarativeExpect {
    #[serde(default)]
    pub response_received: bool,
    #[serde(default)]
    pub exit_code_success_or_limit: bool,
    #[serde(default)]
    pub no_timeout: bool,
    /// iterations_within 上限。
    #[serde(default)]
    pub max_iterations: Option<u32>,
    /// 精确迭代数。
    #[serde(default)]
    pub exact_iterations: Option<u32>,
    /// 事件总数下限(跨 topic)。
    #[serde(default)]
    pub min_total_events: usize,
    #[serde(default)]
    pub scratchpad_updated: bool,
    /// 终止原因(LOOP_COMPLETE / MAX_ITERATIONS / ...)。
    #[serde(default)]
    pub termination: Option<String>,
    /// 事件 topic 最小出现次数。
    #[serde(default)]
    pub events: Vec<DeclarativeEventExpect>,
    /// 事件 payload 必须包含的子串。
    #[serde(default)]
    pub event_payload_contains: Vec<DeclarativePayloadContains>,
    /// 事件 payload 必须命中至少一个关键字。
    #[serde(default)]
    pub event_payload_keywords: Vec<DeclarativePayloadKeywords>,
    /// 按 hat 聚合的最小 job 运行次数(parallel)。
    #[serde(default)]
    pub hat_run_counts: std::collections::HashMap<String, usize>,
    /// 按实例的最小 job 运行次数(parallel)。
    #[serde(default)]
    pub instance_run_counts: std::collections::HashMap<String, usize>,
    /// agents.json 快照断言(parallel)。
    #[serde(default)]
    pub agents_snapshot: Option<DeclarativeAgentsSnapshot>,
    /// 实例 last_input.topic 断言(parallel, 读 agents.json)。
    #[serde(default)]
    pub instance_last_input: Vec<DeclarativeLastInput>,
    /// 必须存在的产物文件(相对 workspace)。
    #[serde(default)]
    pub artifacts: Vec<String>,
    /// 输出必须包含的文本。
    #[serde(default)]
    pub output_contains: Vec<String>,
    /// 输出必须命中至少一个文本(任一命中即通过, 如 [writer#1:out:job= / [writer#1:state])。
    #[serde(default)]
    pub output_contains_any: Vec<Vec<String>>,
    /// 失败语义:`exit_code != Some(0) || !stderr.is_empty()`。
    /// 覆盖 `BackendUnavailableScenario::execution_failed` 与
    /// `AuthFailureScenario::execution_failed_with_error` 的 OR 语义;
    /// 拆成 `exit_code_nonzero` + `stderr_nonempty` 会把命令式的 OR 退化成
    /// AND(runner 的 `assertions.iter().all(|a| a.passed)` 是 AND),失真。
    #[serde(default)]
    pub failed: bool,
    /// stderr 必须包含的文本(逐条独立断言, 类似 `output_contains` 但查 stderr)。
    #[serde(default)]
    pub stderr_contains: Vec<String>,
    /// stderr 必须命中至少一个文本(任一命中即通过, 类似 `output_contains_any` 但查 stderr)。
    /// 覆盖 `BackendUnavailableScenario::error_mentions_backend`(把 stderr 改成
    /// 含任一关键词)与 `AuthFailureScenario::error_message_helpful`(stderr + stdout
    /// 改成 stderr,接受 stderr-only 检查)。
    #[serde(default)]
    pub stderr_contains_any: Vec<Vec<String>>,
    /// failure 时间预算: `result.duration < Duration::from_secs(N)`(secs)。
    /// 覆盖 `BackendUnavailableScenario::failed_fast`(duration < 20s)。
    #[serde(default)]
    pub failed_within_secs: Option<u64>,
    /// 第一个 workflow entry 事件 topic(starting_event 未配置时的推测断言)。
    #[serde(default)]
    pub first_entry: Option<String>,
    /// 事件流中不允许出现的 topic(如 distractor hat 事件)。
    #[serde(default)]
    pub event_absent: Vec<String>,
    /// 事件流中不允许出现的 topic 前缀(如 gate.*)。
    #[serde(default)]
    pub event_absent_prefixes: Vec<String>,
    /// LOOP_COMPLETE 之后 stdout 中不得出现新的 job(并行收敛不变量)。
    #[serde(default)]
    pub no_jobs_after_loop_complete: bool,
    /// 事件出现顺序约束(按序检查首个出现位置)。
    #[serde(default)]
    pub event_order: Vec<String>,
}

/// 事件计数断言。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeEventExpect {
    pub topic: String,
    #[serde(default)]
    pub min_count: usize,
}

/// 注入时序步骤(type 字段区分: wait / sleep / assert / emit)。
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DeclarativeInjectStep {
    /// 等待实例达到状态。
    Wait {
        instance: String,
        /// idle / running / running_then_idle。
        state: String,
        #[serde(default = "default_wait_timeout")]
        timeout_secs: u64,
    },
    /// 等待固定时长。
    Sleep {
        secs: u64,
    },
    /// 断言实例处于某状态(不等待)。
    Assert {
        instance: String,
        state: String,
    },
    /// 执行 `ralph emit`。
    Emit {
        topic: String,
        payload: String,
        #[serde(default)]
        target_instance: Option<String>,
        #[serde(default)]
        session_strategy: Option<String>,
        #[serde(default)]
        turn_action: Option<String>,
        /// 是否用 `--json` 传 payload(如 approval.granted)。
        #[serde(default)]
        json_payload: bool,
    },
    /// 等待事件流中出现指定 topic(轮询 events.jsonl)。
    WaitEvent {
        topic: String,
        #[serde(default = "default_wait_timeout")]
        timeout_secs: u64,
    },
}

fn default_wait_timeout() -> u64 {
    30
}

/// 事件 payload 子串断言。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativePayloadContains {
    pub topic: String,
    pub contains: String,
}

/// 事件 payload 关键字断言(任一命中)。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativePayloadKeywords {
    pub topic: String,
    pub keywords: Vec<String>,
}

/// agents.json 快照断言。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeAgentsSnapshot {
    #[serde(default)]
    pub min_instances: usize,
    #[serde(default)]
    pub hat_ids: Vec<String>,
    /// 是否要求存在动态实例(is_dynamic=true, 含 completed)。
    #[serde(default)]
    pub has_dynamic_instance: bool,
}

/// 实例 last_input 断言。
#[derive(Debug, Clone, Deserialize)]
pub struct DeclarativeLastInput {
    pub instance: String,
    pub topic: String,
}

/// 声明式场景 runner: 实现 TestScenario, 让现有 harness 直接支持。
pub struct DeclarativeScenarioRunner {
    spec: DeclarativeScenario,
    /// 场景目录(prompt_file 相对解析)。
    base_dir: PathBuf,
}

impl DeclarativeScenarioRunner {
    pub fn new(spec: DeclarativeScenario, base_dir: PathBuf) -> Self {
        Self { spec, base_dir }
    }

    /// 渲染 ralph.yml 模板({backend} / {model} / {profile_args} 占位符)。
    ///
    /// 说明:
    /// - `{backend}`: 按 e2e 后端名展开(与命令式场景一致)。
    /// - `{model}`: 按 `codex_e2e_model()` 展开(env `RALPH_E2E_CODEX_MODEL` 优先, 否则默认),
    ///   避免声明式 YAML 硬编码模型导致与命令式行为不一致。
    /// - `{profile_args}`: 按 `codex_e2e_profile()` 展开为 `- -p\n        - <profile>`
    ///   (未配置 profile 时展开为空, 不注入 -p 参数)。
    fn render_config(&self, backend: Backend) -> String {
        let profile_args = render_profile_args(crate::scenarios::parallel::codex_e2e_profile());
        self.spec
            .setup
            .config
            .replace("{backend}", backend.as_config_str())
            .replace(
                "{model}",
                &crate::scenarios::parallel::codex_e2e_model(),
            )
            .replace("{profile_args}", &profile_args)
    }
}

/// 渲染 `{profile_args}` 占位符: 有 profile 时注入两行 `- -p` / `- <profile>`。
fn render_profile_args(profile: Option<String>) -> String {
    profile
        // 注意: config 块经 serde_yaml 解析后, 公共缩进(4 空格)已被剥离,
        // 占位符所在行自带 4 空格前缀(与 `- exec` 同级), 因此第一行不能再带缩进;
        // 第二行起需要显式 4 空格对齐。
        .map(|profile| format!("- -p\n    - {profile}"))
        .unwrap_or_default()
}

#[async_trait]
impl TestScenario for DeclarativeScenarioRunner {
    fn id(&self) -> &str {
        &self.spec.id
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn tier(&self) -> &str {
        &self.spec.tier
    }

    fn supported_backends(&self) -> Vec<Backend> {
        if self.spec.backends.is_empty() {
            vec![Backend::Claude, Backend::Kiro, Backend::Codex, Backend::OpenCode]
        } else {
            self.spec
                .backends
                .iter()
                .filter_map(|name| {
                    match name.as_str() {
                        "claude" => Some(Backend::Claude),
                        "kiro" => Some(Backend::Kiro),
                        "codex" => Some(Backend::Codex),
                        "opencode" => Some(Backend::OpenCode),
                        _ => None,
                    }
                })
                .collect()
        }
    }

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 与命令式场景一致: 创建 .agent 目录。
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // example 引用: 从仓库 examples/<name>/ 读取 ralph.yml + PROMPT.md, patch cli 后写入 workspace。
        if let Some(example_name) = &self.spec.setup.example {
            return crate::scenarios::parallel::setup_prompt_file_example_workspace(
                workspace,
                backend,
                example_name,
                self.spec.setup.max_iterations.unwrap_or(10),
            );
        }

        let config_content = self.render_config(backend);
        let config_path = workspace.join("ralph.yml");
        std::fs::write(&config_path, config_content).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write ralph.yml: {e}"))
        })?;

        // 部分场景依赖 ralph 从 PROMPT.md 读取入口 prompt(prompt_source: config)。
        if let Some(content) = &self.spec.setup.write_prompt_to {
            std::fs::write(workspace.join("PROMPT.md"), content).map_err(|e| {
                ScenarioError::SetupError(format!("failed to write PROMPT.md: {e}"))
            })?;
        }

        // 附加文件写入(如 fake codex shim)。
        for file in &self.spec.setup.write_files {
            let path = workspace.join(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| {
                    ScenarioError::SetupError(format!(
                        "failed to create dir for {}: {e}",
                        file.path
                    ))
                })?;
            }
            std::fs::write(&path, &file.content).map_err(|e| {
                ScenarioError::SetupError(format!("failed to write {}: {e}", file.path))
            })?;
            if file.executable {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    let mut perms = std::fs::metadata(&path)
                        .map_err(|e| {
                            ScenarioError::SetupError(format!(
                                "failed to stat {}: {e}",
                                file.path
                            ))
                        })?
                        .permissions();
                    perms.set_mode(0o755);
                    std::fs::set_permissions(&path, perms).map_err(|e| {
                        ScenarioError::SetupError(format!(
                            "failed to chmod +x {}: {e}",
                            file.path
                        ))
                    })?;
                }
            }
        }

        // prompt: 内联 / 文件 / config(ralph_prompt)。
        let prompt = match self.spec.setup.prompt_source.as_deref() {
            Some("config") => PromptSource::Config,
            _ => {
                if let Some(text) = &self.spec.setup.prompt {
                    PromptSource::Inline(text.clone())
                } else if let Some(file) = &self.spec.setup.prompt_file {
                    let content = std::fs::read_to_string(self.base_dir.join(file)).map_err(|e| {
                        ScenarioError::SetupError(format!("failed to read prompt file {file}: {e}"))
                    })?;
                    PromptSource::Inline(content)
                } else {
                    return Err(ScenarioError::SetupError(
                        "declarative scenario requires setup.prompt, setup.prompt_file, or prompt_source: config".to_string(),
                    ));
                }
            }
        };

        let timeout = self
            .spec
            .setup
            .timeout_secs
            .map(Duration::from_secs)
            .unwrap_or_else(|| backend.default_timeout());

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt,
            max_iterations: self.spec.setup.max_iterations.unwrap_or(10),
            timeout,
            extra_args: self.spec.setup.extra_args.clone(),
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let start = std::time::Instant::now();

        // 注入时序: 在 ralph 运行期间并发执行(wait/sleep/assert/emit)。
        let inject_task = (!self.spec.setup.inject.is_empty()).then(|| {
            let workspace = executor.workspace().clone();
            let ralph_bin = executor.ralph_binary();
            let steps = self.spec.setup.inject.clone();
            tokio::spawn(async move {
                run_inject_sequence(&ralph_bin, &workspace, &steps).await
            })
        });

        // PATH 前缀注入: 把 workspace 相对目录(如 .e2e/bin)放到 PATH 最前面,
        // 让 ralph 优先找到 fake codex shim; 与命令式 fake shim 场景一致。
        let mut extra_env: Vec<(String, String)> = self.spec.setup.env.clone().into_iter().collect();
        if !self.spec.setup.path_prefix.is_empty() {
            let old_path = std::env::var("PATH").unwrap_or_default();
            let prefix = self
                .spec
                .setup
                .path_prefix
                .iter()
                .map(|dir| executor.workspace().join(dir).display().to_string())
                .collect::<Vec<_>>()
                .join(":");
            extra_env.push(("PATH".to_string(), format!("{prefix}:{old_path}")));
        }
        let execution = executor
            .run_with_extra_env(config, &extra_env)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;
        let duration = start.elapsed();

        // 注入任务收尾: 失败即场景失败(注入是场景契约的一部分)。
        if let Some(task) = inject_task {
            task.await
                .map_err(|e| ScenarioError::ExecutionError(format!("inject task panicked: {e}")))?
                .map_err(|e| ScenarioError::ExecutionError(format!("inject failed: {e}")))?;
        }

        let mut assertions = Vec::new();
        let expect = &self.spec.expect;

        if expect.response_received {
            assertions.push(Assertions::response_received(&execution));
        }
        if expect.exit_code_success_or_limit {
            assertions.push(Assertions::exit_code_success_or_limit(&execution));
        }
        if expect.no_timeout {
            assertions.push(Assertions::no_timeout(&execution));
        }
        if let Some(max) = expect.max_iterations {
            assertions.push(Assertions::iterations_within(&execution, max));
        }
        if let Some(exact) = expect.exact_iterations {
            assertions.push(iterations_exact(&execution, exact));
        }
        if expect.min_total_events > 0 {
            assertions.push(total_events_at_least(&execution, expect.min_total_events));
        }
        if expect.scratchpad_updated {
            assertions.push(scratchpad_updated(&execution));
        }
        if let Some(reason) = &expect.termination {
            assertions.push(termination_matches(&execution, reason));
        }
        for event_expect in &expect.events {
            assertions.push(event_count_at_least(
                &execution,
                &event_expect.topic,
                event_expect.min_count,
            ));
        }
        for needle in &expect.output_contains {
            assertions.push(output_contains(&execution, needle));
        }
        for needles in &expect.output_contains_any {
            assertions.push(output_contains_any(&execution, needles));
        }
        // 新增失败族断言(为 backend-unavailable / auth-failure 等场景补全 schema 覆盖):
        // failed/stderr_contains/stderr_contains_any/failed_within_secs。
        if expect.failed {
            assertions.push(failed(&execution));
        }
        for needle in &expect.stderr_contains {
            assertions.push(stderr_contains(&execution, needle));
        }
        for needles in &expect.stderr_contains_any {
            assertions.push(stderr_contains_any(&execution, needles));
        }
        if let Some(secs) = expect.failed_within_secs {
            assertions.push(failed_within(&execution, secs));
        }
        for payload_expect in &expect.event_payload_contains {
            assertions.push(event_payload_contains(
                &execution,
                &payload_expect.topic,
                &payload_expect.contains,
            ));
        }
        for keyword_expect in &expect.event_payload_keywords {
            assertions.push(event_payload_keywords(
                &execution,
                &keyword_expect.topic,
                &keyword_expect.keywords,
            ));
        }
        for (hat, min_runs) in &expect.hat_run_counts {
            assertions.push(hat_run_count_at_least(&execution, hat, *min_runs));
        }
        for (instance, min_runs) in &expect.instance_run_counts {
            assertions.push(instance_run_count_at_least(&execution, instance, *min_runs));
        }
        if let Some(snapshot_expect) = &expect.agents_snapshot {
            assertions.push(agents_snapshot_matches(
                executor.workspace(),
                snapshot_expect,
            ));
        }
        for last_input_expect in &expect.instance_last_input {
            assertions.push(instance_last_input_matches(
                executor.workspace(),
                &last_input_expect.instance,
                &last_input_expect.topic,
            ));
        }
        for artifact in &expect.artifacts {
            assertions.push(artifact_exists(executor.workspace(), artifact));
        }
        if let Some(topic) = &expect.first_entry {
            assertions.push(first_entry_matches(&execution, topic));
        }
        for absent in &expect.event_absent {
            assertions.push(event_absent(&execution, absent));
        }
        for prefix in &expect.event_absent_prefixes {
            assertions.push(event_absent_prefix(&execution, prefix));
        }
        if expect.no_jobs_after_loop_complete {
            assertions.push(no_jobs_after_loop_complete(&execution));
        }
        if !expect.event_order.is_empty() {
            assertions.push(event_order_matches(&execution, &expect.event_order));
        }

        let all_passed = assertions.iter().all(|a| a.passed);

        Ok(TestResult {
            scenario_id: self.spec.id.clone(),
            scenario_description: self.spec.description.clone(),
            backend: String::new(),
            tier: self.spec.tier.clone(),
            passed: all_passed,
            assertions,
            duration,
        })
    }
}

// ---------------------------------------------------------------------------
// 内置断言(声明式场景专用; 与命令式场景共享 Assertions 基座)。
// ---------------------------------------------------------------------------

fn scratchpad_updated(result: &ExecutionResult) -> crate::models::Assertion {
    let updated = result
        .scratchpad
        .as_ref()
        .is_some_and(|s| !s.trim().is_empty());
    let builder = crate::scenarios::AssertionBuilder::new("Scratchpad updated")
        .expected("Scratchpad contains content")
        .actual(if updated {
            "Scratchpad has content".to_string()
        } else {
            "Scratchpad empty or missing".to_string()
        });
    if updated { builder.passed() } else { builder.failed() }.build()
}

fn termination_matches(
    result: &ExecutionResult,
    expected: &str,
) -> crate::models::Assertion {
    let actual = result.termination_reason.clone().unwrap_or_default();
    let ok = actual == expected;
    let builder = crate::scenarios::AssertionBuilder::new(format!("Termination reason is {expected}"))
        .expected(expected)
        .actual(if actual.is_empty() {
            "<none>".to_string()
        } else {
            actual
        });
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn event_count_at_least(
    result: &ExecutionResult,
    topic: &str,
    min_count: usize,
) -> crate::models::Assertion {
    let count = result
        .events
        .iter()
        .filter(|e| e.topic == topic)
        .count();
    let ok = count >= min_count;
    let builder = crate::scenarios::AssertionBuilder::new(format!("Event {topic} count >= {min_count}"))
        .expected(format!("at least {min_count} events"))
        .actual(format!("count={count}"));
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn iterations_exact(result: &ExecutionResult, expected: u32) -> crate::models::Assertion {
    let ok = result.iterations == expected;
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Completed in {expected} iterations"
    ))
    .expected(format!("{expected} iterations"))
    .actual(format!("{} iterations", result.iterations));
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn total_events_at_least(result: &ExecutionResult, min: usize) -> crate::models::Assertion {
    let count = result.events.len();
    let ok = count >= min;
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "At least {min} events emitted"
    ))
    .expected(format!("at least {min} events"))
    .actual(format!("{count} events"));
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn event_payload_contains(
    result: &ExecutionResult,
    topic: &str,
    needle: &str,
) -> crate::models::Assertion {
    // 优先用 stdout 解析的完整 payload:
    // - events.jsonl 会截断 >500 字符的 payload, 截断后的 JSON 不可解析;
    // - 并行 stdout 会带 `[instance:out:job=N]` 前缀与 err 通道交错,
    //   必须先归一化(:out:job 行剥前缀拼接)再提取, 否则 payload 不是纯 JSON。
    let full_payload = crate::scenarios::parallel::extract_last_parallel_out_payload_for_topic(
        &result.stdout,
        topic,
    );
    let event_payload = full_payload
        .as_deref()
        .or_else(|| result.events.iter().find(|e| e.topic == topic).map(|e| e.payload.as_str()));
    let ok = event_payload
        .map(|payload| payload_matches_needle(payload, needle))
        .unwrap_or(false);
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Event '{topic}' payload contains '{needle}'"
    ))
    .expected(format!("Payload containing '{needle}'"))
    .actual(match event_payload {
        Some(payload) => format!("Payload: {}", truncate_payload(payload)),
        None => "Event not found".to_string(),
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 判断 payload 是否匹配期望子串。
///
/// 说明:
/// - 事件 payload 可能是 JSON(如 `{"audit_status":"READY_FOR_AUDITOR", ...}`)
///   或 line 格式(如 `audit_status: READY_FOR_AUDITOR`), 两者都要能匹配。
/// - 期望写成 `key: value` 时, 优先做 JSON 字段语义匹配:
///   - JSON: 解析后检查 `key` 字段值 == value(大小写不敏感)。
///   - line: 保持子串匹配(与命令式 example 场景的 payload matcher 口径一致)。
/// - 期望不含 `: ` 时退化为纯子串匹配。
fn payload_matches_needle(payload: &str, needle: &str) -> bool {
    // 先尝试纯子串匹配(JSON 与 line 都可能有)。
    if payload.contains(needle) {
        return true;
    }

    // 期望形如 `key: value` 时, 尝试 JSON 字段语义匹配。
    let Some((key, expected)) = needle.split_once(": ") else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(payload) else {
        return false;
    };
    let Some(actual) = value.get(key).and_then(|v| v.as_str()) else {
        return false;
    };
    actual.eq_ignore_ascii_case(expected)
}

fn event_payload_keywords(
    result: &ExecutionResult,
    topic: &str,
    keywords: &[String],
) -> crate::models::Assertion {
    // 与 event_payload_contains 一致: 优先用 stdout 完整 payload(避免截断)。
    let full_payload = crate::scenarios::parallel::extract_last_parallel_out_payload_for_topic(
        &result.stdout,
        topic,
    );
    let event_payload = full_payload
        .as_deref()
        .or_else(|| result.events.iter().find(|e| e.topic == topic).map(|e| e.payload.as_str()));
    let ok = event_payload
        .map(|payload| {
            let payload = payload.to_lowercase();
            keywords.iter().any(|k| payload.contains(&k.to_lowercase()))
        })
        .unwrap_or(false);
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Event '{topic}' payload hits a keyword"
    ))
    .expected(format!("Payload with one of {keywords:?}"))
    .actual(match event_payload {
        Some(payload) => format!("Payload: {}", truncate_payload(payload)),
        None => "Event not found".to_string(),
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn truncate_payload(payload: &str) -> String {
    let max = 50;
    if payload.chars().count() <= max {
        payload.to_string()
    } else {
        payload.chars().take(max).collect::<String>() + "..."
    }
}

fn hat_run_count_at_least(
    result: &ExecutionResult,
    hat: &str,
    min_runs: usize,
) -> crate::models::Assertion {
    let counts = crate::scenarios::parallel::job_run_counts::JobRunCounts::from_stdout(&result.stdout);
    let runs = counts.runs_for_hat(hat);
    let ok = runs >= min_runs;
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Hat {hat} job runs >= {min_runs}"
    ))
    .expected(format!("at least {min_runs} runs"))
    .actual(format!("{runs} runs"));
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn instance_run_count_at_least(
    result: &ExecutionResult,
    instance: &str,
    min_runs: usize,
) -> crate::models::Assertion {
    let counts = crate::scenarios::parallel::job_run_counts::JobRunCounts::from_stdout(&result.stdout);
    let runs = counts.runs_for_instance(instance);
    let ok = runs >= min_runs;
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Instance {instance} job runs >= {min_runs}"
    ))
    .expected(format!("at least {min_runs} runs"))
    .actual(format!("{runs} runs"));
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn agents_snapshot_matches(
    workspace: &std::path::Path,
    expect: &DeclarativeAgentsSnapshot,
) -> crate::models::Assertion {
    let path = workspace.join(".ralph").join("agents.json");
    let content = std::fs::read_to_string(&path);
    let (ok, actual) = match content {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => {
                let instances = value
                    .get("instances")
                    .and_then(|v| v.as_array())
                    .cloned()
                    .unwrap_or_default();
                let count = instances.len();
                let hat_ids = expect.hat_ids.clone();
                let missing: Vec<String> = hat_ids
                    .iter()
                    .filter(|hid| {
                        !instances.iter().any(|i| {
                            i.get("hat_id").and_then(|v| v.as_str()) == Some(hid.as_str())
                        })
                    })
                    .cloned()
                    .collect();
                let has_dynamic = expect.has_dynamic_instance && {
                    let dynamic = instances.iter().any(|i| {
                        i.get("is_dynamic").and_then(|v| v.as_bool()) == Some(true)
                    });
                    // 动态实例可能完成后退场(completed_dynamic_instances)
                    let completed_dynamic = value
                        .get("completed_dynamic_instances")
                        .and_then(|v| v.as_array())
                        .map(|list| !list.is_empty())
                        .unwrap_or(false);
                    dynamic || completed_dynamic
                };
                let ok = count >= expect.min_instances
                    && missing.is_empty()
                    && (!expect.has_dynamic_instance || has_dynamic);
                let actual = if missing.is_empty() {
                    format!(
                        "instance_count={count}, has_dynamic={has_dynamic}"
                    )
                } else {
                    format!("instance_count={count}, missing={missing:?}")
                };
                (ok, actual)
            }
            Err(e) => (false, format!("invalid JSON: {e}")),
        },
        Err(e) => (false, format!("missing: {e}")),
    };
    let builder = crate::scenarios::AssertionBuilder::new("Agents snapshot written")
        .expected(format!(
            "agents.json contains >= {} instances and hats {:?}",
            expect.min_instances, expect.hat_ids
        ))
        .actual(actual);
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn instance_last_input_matches(
    workspace: &std::path::Path,
    instance: &str,
    expected_topic: &str,
) -> crate::models::Assertion {
    let path = workspace.join(".ralph").join("agents.json");
    let (ok, actual) = match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(value) => {
                let topic = value
                    .get("instances")
                    .and_then(|v| v.as_array())
                    .and_then(|list| {
                        list.iter().find(|i| {
                            i.get("instance_id").and_then(|v| v.as_str())
                                == Some(instance)
                        })
                    })
                    .and_then(|i| i.get("last_input"))
                    .and_then(|li| li.get("topic"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                match topic {
                    Some(t) if t == expected_topic => (true, format!("topic={t}")),
                    Some(t) => (false, format!("topic={t} (expected {expected_topic})")),
                    None => (false, "no last_input".to_string()),
                }
            }
            Err(e) => (false, format!("invalid JSON: {e}")),
        },
        Err(e) => (false, format!("missing: {e}")),
    };
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "{instance} last_input.topic == {expected_topic}"
    ))
    .expected(expected_topic)
    .actual(actual);
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn artifact_exists(workspace: &std::path::Path, artifact: &str) -> crate::models::Assertion {
    let ok = workspace.join(artifact).exists();
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Artifact exists: {artifact}"
    ))
    .expected("file exists")
    .actual(if ok { "found".to_string() } else { "missing".to_string() });
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn output_contains(result: &ExecutionResult, needle: &str) -> crate::models::Assertion {
    let ok = result.stdout.contains(needle);
    let builder = crate::scenarios::AssertionBuilder::new(format!("Output contains {needle:?}"))
        .expected(format!("stdout contains {needle:?}"))
        .actual(if ok { "found".to_string() } else { "missing".to_string() });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: stdout 命中任一 needle(如实例的 out:job 行或 state 行)。
///
/// 说明:
/// - 并行日志中, 实例被创建但未实际跑 job 时只有 `[writer#1:state]` 行,
///   没有 `[writer#1:out:job=]` 行; 命令式 attributed_outputs_visible 用"任一命中"口径。
fn output_contains_any(result: &ExecutionResult, needles: &[String]) -> crate::models::Assertion {
    let hits: Vec<&str> = needles
        .iter()
        .filter(|needle| result.stdout.contains(needle.as_str()))
        .map(|needle| needle.as_str())
        .collect();
    let ok = !hits.is_empty();
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Output contains any of {needles:?}"
    ))
    .expected(format!("stdout contains at least one of {needles:?}"))
    .actual(if hits.is_empty() {
        "none matched".to_string()
    } else {
        format!("matched {hits:?}")
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: 失败语义, 即 `exit_code != Some(0) || !stderr.is_empty()`。
///
/// 设计要点:
/// - 命令式 `BackendUnavailableScenario::execution_failed` 与
///   `AuthFailureScenario::execution_failed_with_error` 都是这个 OR 语义。
/// - 拆成两个独立字段会让 runner 的 AND 语义(`assertions.iter().all`)压扁
///   命令式的 OR,失真;本函数保留 OR。
/// - exit_code = None(被信号 kill)在该不等式下也判定为 "failed"("不是干净的 0"),
///   与命令式 `!= Some(0)` 行为一致。
fn failed(result: &ExecutionResult) -> crate::models::Assertion {
    let ok = result.exit_code != Some(0) || !result.stderr.is_empty();
    let builder = crate::scenarios::AssertionBuilder::new("Execution failed")
        .expected("non-zero exit code or non-empty stderr")
        .actual(format!(
            "exit_code={:?} stderr_len={}",
            result.exit_code,
            result.stderr.len()
        ));
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: stderr 命中单 needle(类似 `output_contains`, 但查 stderr 通道)。
///
/// 用法: `expect.stderr_contains: ["command not found"]` 等价于
/// 命令式 `result.stderr.to_lowercase().contains("command not found")`。
fn stderr_contains(result: &ExecutionResult, needle: &str) -> crate::models::Assertion {
    let ok = result.stderr.contains(needle);
    let builder = crate::scenarios::AssertionBuilder::new(format!("stderr contains {needle:?}"))
        .expected(format!("stderr contains {needle:?}"))
        .actual(if ok { "found".to_string() } else { "missing".to_string() });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: stderr 命中任一 needle(类似 `output_contains_any`, 但查 stderr 通道)。
///
/// 用法: 覆盖 `BackendUnavailableScenario::error_mentions_backend` 与
/// `AuthFailureScenario::error_message_helpful`(原本同时查 stderr + stdout,
/// 这里改为只查 stderr;auth-failure 关键字应出现在 ralph 的错误输出流)。
fn stderr_contains_any(result: &ExecutionResult, needles: &[String]) -> crate::models::Assertion {
    let hits: Vec<&str> = needles
        .iter()
        .filter(|needle| result.stderr.contains(needle.as_str()))
        .map(|needle| needle.as_str())
        .collect();
    let ok = !hits.is_empty();
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "stderr contains any of {needles:?}"
    ))
    .expected(format!("stderr contains at least one of {needles:?}"))
    .actual(if hits.is_empty() {
        "none matched".to_string()
    } else {
        format!("matched {hits:?}")
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: `result.duration < Duration::from_secs(secs)`。
///
/// 用法: 覆盖 `BackendUnavailableScenario::failed_fast`(duration < 20s)。
/// 选择 "硬性 <" 而不是 "<=" 以兼容 `Duration::from_secs(20)` 边界:
/// 命令式代码 `result.duration < Duration::from_secs(20)` 用 "<"。
fn failed_within(result: &ExecutionResult, secs: u64) -> crate::models::Assertion {
    let budget = std::time::Duration::from_secs(secs);
    let ok = result.duration < budget;
    let builder = crate::scenarios::AssertionBuilder::new(format!("Failed within {secs}s"))
        .expected(format!("duration < {secs}s"))
        .actual(format!("duration={:?}", result.duration));
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: ralph#1 在 task.start/task.resume 之后发布的第一个事件 topic。
///
/// 说明:
/// - 用于 `event_loop.starting_event` 未配置时,验证 coordinator 从拓扑推测入口事件。
/// - 与命令式 starting_event_inference 的 workflow_entry_inferred 口径一致。
fn first_entry_matches(result: &ExecutionResult, expected_topic: &str) -> crate::models::Assertion {
    let mut first: Option<&str> = None;
    for e in &result.events {
        if e.source_instance.as_deref() != Some("ralph#1") {
            continue;
        }
        if matches!(e.topic.as_str(), "task.start" | "task.resume") {
            continue;
        }
        first = Some(e.topic.as_str());
        break;
    }
    let ok = first == Some(expected_topic);
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "First ralph#1 workflow entry event is {expected_topic:?}"
    ))
    .expected(expected_topic)
    .actual(match first {
        Some(topic) => format!("first_entry={topic}"),
        None => "first_entry=<none>".to_string(),
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: 事件流中不存在指定 topic(distractor hat 不应被触发)。
fn event_absent(result: &ExecutionResult, topic: &str) -> crate::models::Assertion {
    let present = result.events.iter().any(|e| e.topic == topic);
    let builder = crate::scenarios::AssertionBuilder::new(format!("Event absent: {topic:?}"))
        .expected(format!("events.jsonl does NOT contain {topic:?}"))
        .actual(if present { "present".to_string() } else { "absent".to_string() });
    if present { builder.failed() } else { builder.passed() }.build()
}

/// 断言: 事件流中不存在以指定前缀开头的 topic(如 gate.*)。
fn event_absent_prefix(result: &ExecutionResult, prefix: &str) -> crate::models::Assertion {
    let present: Vec<String> = result
        .events
        .iter()
        .filter(|e| e.topic.starts_with(prefix))
        .map(|e| e.topic.clone())
        .collect();
    let builder =
        crate::scenarios::AssertionBuilder::new(format!("Event absent prefix: {prefix:?}"))
            .expected(format!("events.jsonl does NOT contain any topic starting with {prefix:?}"))
            .actual(if present.is_empty() {
                "absent".to_string()
            } else {
                format!("present={present:?}")
            });
    if present.is_empty() { builder.passed() } else { builder.failed() }.build()
}

/// 断言: LOOP_COMPLETE 之后 stdout 中不得出现新的并行 job。
///
/// 说明:
/// - 这是并行收敛的强不变量: 协调者宣布完成后,任何新 job 都意味着"幽灵工作"。
/// - 与命令式 example 场景的 no_new_jobs_started_after_loop_complete 口径一致。
fn no_jobs_after_loop_complete(result: &ExecutionResult) -> crate::models::Assertion {
    let completion_promise = "LOOP_COMPLETE";
    let mut completion_seen = false;
    let mut jobs_before: std::collections::HashSet<(String, u64)> =
        std::collections::HashSet::new();
    let mut new_jobs_after: Vec<(String, u64)> = Vec::new();

    for line in result.stdout.lines() {
        if let Some((instance_id, job_id)) =
            crate::scenarios::parallel::job_run_counts::parse_parallel_job_line(line)
        {
            if completion_seen {
                if !jobs_before.contains(&(instance_id.clone(), job_id)) {
                    new_jobs_after.push((instance_id, job_id));
                }
            } else {
                jobs_before.insert((instance_id, job_id));
            }
        }
        // 注意: 必须精确匹配 payload 本身 == LOOP_COMPLETE,
        // 不能用 ends_with —— ralph#2 等实例的文本里可能以 "…与 LOOP_COMPLETE" 结尾,
        // 误判会把真正完成后的新 job 当幽灵(与命令式 no_new_jobs 口径一致)。
        if !completion_seen
            && line.trim_start().starts_with("[ralph#")
            && line.contains(":out:job=")
            && let Some((_prefix, payload)) = line.split_once("] ")
            && payload.trim() == completion_promise
        {
            completion_seen = true;
        }
    }

    let ok = completion_seen && new_jobs_after.is_empty();
    let builder = crate::scenarios::AssertionBuilder::new(
        "No new jobs after LOOP_COMPLETE",
    )
    .expected("After LOOP_COMPLETE, no new job_id should appear in stdout")
    .actual(format!(
        "completion_seen={completion_seen}, new_jobs_after={new_jobs_after:?}"
    ));
    if ok { builder.passed() } else { builder.failed() }.build()
}

/// 断言: 事件按给定顺序出现(每个 topic 取首个出现位置, 必须严格递增)。
fn event_order_matches(result: &ExecutionResult, expected_order: &[String]) -> crate::models::Assertion {
    let mut positions = Vec::new();
    for topic in expected_order {
        let pos = result
            .events
            .iter()
            .position(|e| e.topic == *topic)
            .map(|i| i as i64)
            .unwrap_or(-1);
        positions.push((topic.as_str(), pos));
    }
    // 严格递增且全部出现。
    let ok = positions
        .iter()
        .all(|(_, pos)| *pos >= 0)
        && positions.windows(2).all(|w| w[0].1 < w[1].1);
    let actual = positions
        .iter()
        .map(|(t, p)| format!("{t}={p}"))
        .collect::<Vec<_>>()
        .join(", ");
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Event order: {}",
        expected_order.join(" < ")
    ))
    .expected(expected_order.join(" < "))
    .actual(actual);
    if ok { builder.passed() } else { builder.failed() }.build()
}

// ---------------------------------------------------------------------------
// 注入时序执行器(wait/sleep/assert/emit)。
// ---------------------------------------------------------------------------

async fn run_inject_sequence(
    ralph_bin: &std::path::Path,
    workspace: &std::path::Path,
    steps: &[DeclarativeInjectStep],
) -> Result<(), String> {
    let mut seen_running: bool = false;
    for step in steps {
        match step {
            DeclarativeInjectStep::Wait {
                instance,
                state,
                timeout_secs,
            } => {
                wait_instance(workspace, instance, state, &mut seen_running, *timeout_secs).await?;
            }
            DeclarativeInjectStep::Sleep { secs } => {
                tokio::time::sleep(std::time::Duration::from_secs(*secs)).await;
            }
            DeclarativeInjectStep::Assert { instance, state } => {
                let current = read_instance_state(workspace, instance)?;
                let ok = match state.as_str() {
                    "idle" => current.as_deref() == Some("idle"),
                    "running" => current.as_deref() == Some("running"),
                    other => {
                        return Err(format!("unsupported assert state: {other}"));
                    }
                };
                if !ok {
                    return Err(format!(
                        "assert failed: {} expected {} got {:?}",
                        instance, state, current
                    ));
                }
            }
            DeclarativeInjectStep::Emit {
                topic,
                payload,
                target_instance,
                session_strategy,
                turn_action,
                json_payload,
            } => {
                emit_event(
                    ralph_bin,
                    workspace,
                    topic,
                    payload,
                    target_instance.as_deref(),
                    session_strategy.as_deref(),
                    turn_action.as_deref(),
                    *json_payload,
                )
                .await?;
            }
            DeclarativeInjectStep::WaitEvent { topic, timeout_secs } => {
                wait_for_event(workspace, topic, *timeout_secs).await?;
            }
        }
    }
    Ok(())
}

/// 读取实例状态(agents.json 轮询)。
async fn wait_instance(
    workspace: &std::path::Path,
    instance: &str,
    state: &str,
    seen_running: &mut bool,
    timeout_secs: u64,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        // agents.json 可能尚未生成(ralph 刚启动): 读取失败视为"未知状态", 继续等。
        let current = read_instance_state(workspace, instance).unwrap_or_default();
        if let Some(ref current) = current {
            if current == "running" {
                *seen_running = true;
            }
            let ok = match state {
                "idle" => current == "idle",
                "running" => current == "running",
                "running_then_idle" => *seen_running && current == "idle",
                other => return Err(format!("unsupported wait state: {other}")),
            };
            if ok {
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!(
                "timeout waiting for {instance} state {state} (last={current:?})"
            ));
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
}

/// 从 .ralph/agents.json 读取实例状态。
fn read_instance_state(
    workspace: &std::path::Path,
    instance: &str,
) -> Result<Option<String>, String> {
    let path = workspace.join(".ralph").join("agents.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read agents.json: {e}"))?;
    let value: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("failed to parse agents.json: {e}"))?;
    let state = value
        .get("instances")
        .and_then(|instances| instances.as_array())
        .and_then(|list| {
            list.iter().find(|i| {
                i.get("instance_id")
                    .and_then(|v| v.as_str())
                    .is_some_and(|id| id == instance)
            })
        })
        .and_then(|i| i.get("state").and_then(|v| v.as_str()))
        .map(|s| s.to_string());
    Ok(state)
}

/// 执行 `ralph emit`。
async fn emit_event(
    ralph_bin: &std::path::Path,
    workspace: &std::path::Path,
    topic: &str,
    payload: &str,
    target_instance: Option<&str>,
    session_strategy: Option<&str>,
    turn_action: Option<&str>,
    json_payload: bool,
) -> Result<(), String> {
    use tokio::process::Command;

    let mut cmd = Command::new(ralph_bin);
    cmd.arg("emit").arg(topic).current_dir(workspace);
    if json_payload {
        // 结构化 payload(如 approval.granted --json): 保持 JSON 原样传递。
        cmd.arg("--json").arg(payload);
    } else {
        cmd.arg(payload);
    }
    if let Some(target) = target_instance {
        cmd.arg("--target-instance").arg(target);
    }
    if let Some(strategy) = session_strategy {
        cmd.arg("--session-strategy").arg(strategy);
    }
    if let Some(action) = turn_action {
        cmd.arg("--turn-action").arg(action);
    }
    let output = cmd
        .output()
        .await
        .map_err(|e| format!("failed to run ralph emit: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "ralph emit failed: status={:?}, stderr={}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

/// 等待事件流(.ralph/events.jsonl)中出现指定 topic。
///
/// 说明:
/// - 用于"先看到某事件, 再注入后续事件"的时序场景(如 approval.requested → approval.granted)。
/// - 与命令式 human_approval_gate 的 wait_for_topic 口径一致。
async fn wait_for_event(
    workspace: &std::path::Path,
    topic: &str,
    timeout_secs: u64,
) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    loop {
        // events.jsonl 可能尚未生成: 读取失败视为"尚未出现", 继续等。
        if let Ok(content) = std::fs::read_to_string(workspace.join(".ralph").join("events.jsonl")) {
            let found = content.lines().any(|line| {
                serde_json::from_str::<serde_json::Value>(line)
                    .ok()
                    .and_then(|v| v.get("topic").and_then(|t| t.as_str()).map(str::to_string))
                    .as_deref()
                    == Some(topic)
            });
            if found {
                return Ok(());
            }
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!("timeout waiting for event topic {topic:?}"));
        }
        tokio::time::sleep(std::time::Duration::from_millis(250)).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionResult;
    use std::time::Duration;

    fn sample_result(iterations: u32, events: Vec<&str>, termination: Option<&str>) -> ExecutionResult {
        ExecutionResult {
            exit_code: Some(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
            duration: Duration::from_secs(1),
            scratchpad: Some("content".to_string()),
            events: events.into_iter().map(|t| crate::executor::EventRecord { topic: t.to_string(), payload: String::new(), source_instance: None }).collect(),
            iterations,
            termination_reason: termination.map(|s| s.to_string()),
            timed_out: false,
        }
    }

    #[test]
    fn iterations_exact_matches() {
        let ok = iterations_exact(&sample_result(3, vec![], None), 3);
        assert!(ok.passed);
        let bad = iterations_exact(&sample_result(2, vec![], None), 3);
        assert!(!bad.passed);
    }

    #[test]
    fn total_events_at_least_counts_all_topics() {
        let r = sample_result(1, vec!["a", "b", "c"], None);
        assert!(total_events_at_least(&r, 3).passed);
        assert!(!total_events_at_least(&r, 4).passed);
    }

    #[test]
    fn event_payload_contains_matches_substring() {
        let r = sample_result(1, vec!["test.event"], None);
        // 需要带 payload 的事件: 手工构造
        let mut r2 = r;
        r2.events[0].payload = "Test payload data".to_string();
        assert!(event_payload_contains(&r2, "test.event", "Test payload").passed);
        assert!(!event_payload_contains(&r2, "test.event", "missing").passed);
        assert!(!event_payload_contains(&r2, "other.event", "Test").passed);
    }

    #[test]
    fn payload_matches_needle_handles_json_and_line_formats() {
        // line 格式: 子串直接命中。
        let line = "review_status: READY_FOR_REGION_WEEKLY\nregion_code: APAC_ENTERPRISE";
        assert!(payload_matches_needle(line, "review_status: READY_FOR_REGION_WEEKLY"));
        // JSON 格式: 字段语义匹配(key: value 拆分后查 JSON 字段)。
        let json = r#"{"review_id":"x","review_status":"READY_FOR_REGION_WEEKLY","region_code":"APAC_ENTERPRISE"}"#;
        assert!(payload_matches_needle(json, "review_status: READY_FOR_REGION_WEEKLY"));
        assert!(payload_matches_needle(json, "region_code: APAC_ENTERPRISE"));
        // JSON 但期望值不匹配。
        assert!(!payload_matches_needle(json, "review_status: SOMETHING_ELSE"));
        // JSON 且 key 不存在。
        assert!(!payload_matches_needle(json, "missing_key: VALUE"));
        // 非 JSON payload + key: value 期望 → 子串失败则整体失败。
        assert!(!payload_matches_needle("plain text", "key: value"));
        // 纯值子串(无冒号)在 JSON 里命中。
        assert!(payload_matches_needle(json, "READY_FOR_REGION_WEEKLY"));
    }

    #[test]
    fn output_contains_any_matches_at_least_one_needle() {
        let mut r = sample_result(1, vec![], None);
        r.stdout = "[writer#1:state] created\n[writer#2:out:job=1] work\n".to_string();
        // writer#1 只有 state 行(未实际跑 job), 任一命中应通过。
        let needles = vec![
            "[writer#1:out:job=".to_string(),
            "[writer#1:err:job=".to_string(),
            "[writer#1:state]".to_string(),
        ];
        assert!(output_contains_any(&r, &needles).passed);
        // 全部缺失则失败。
        let r2 = sample_result(1, vec![], None);
        let missing = vec!["[ghost:out:job=".to_string()];
        assert!(!output_contains_any(&r2, &missing).passed);
    }

    #[test]
    fn failed_passes_on_non_zero_exit() {
        // 非零 exit_code 即视为 failed, 与命令式 `!= Some(0)` 一致(stderr 为空也通过)。
        let r = sample_result(1, vec![], None);
        let mut r = r;
        r.exit_code = Some(1);
        r.stderr = String::new();
        assert!(failed(&r).passed);
    }

    #[test]
    fn failed_passes_on_stderr_presence_even_when_exit_zero() {
        // exit_code == 0 但 stderr 非空 → 仍视为 failed(OR 语义)。
        // 模拟 ralph 写出警告/诊断但仍正常退出的场景。
        let mut r = sample_result(1, vec![], None);
        r.exit_code = Some(0);
        r.stderr = "warning: deprecated config key\n".to_string();
        assert!(failed(&r).passed);
    }

    #[test]
    fn failed_fails_when_exit_zero_and_stderr_empty() {
        // exit_code == 0 且 stderr 为空 → 干净成功, 不算 failed。
        let mut r = sample_result(1, vec![], None);
        r.exit_code = Some(0);
        r.stderr = String::new();
        assert!(!failed(&r).passed);
    }

    #[test]
    fn failed_treats_signal_kill_as_failed() {
        // exit_code = None(被信号 kill) 在 `!= Some(0)` 下也为 true,
        // 与命令式 `execution_failed` 行为一致。
        let mut r = sample_result(1, vec![], None);
        r.exit_code = None;
        r.stderr = String::new();
        assert!(failed(&r).passed);
    }

    #[test]
    fn stderr_contains_matches_needle() {
        let mut r = sample_result(1, vec![], None);
        r.stderr = "Error: command not found in PATH\n".to_string();
        assert!(stderr_contains(&r, "command not found").passed);
        assert!(!stderr_contains(&r, "unauthorized").passed);
    }

    #[test]
    fn stderr_contains_any_matches_at_least_one_needle() {
        let mut r = sample_result(1, vec![], None);
        r.stderr = "auth failed: invalid API key\n".to_string();
        let hits = vec![
            "unauthorized".to_string(),
            "invalid".to_string(),
            "credential".to_string(),
        ];
        assert!(stderr_contains_any(&r, &hits).passed);
        let miss = vec!["backend".to_string(), "cli".to_string()];
        assert!(!stderr_contains_any(&r, &miss).passed);
    }

    #[test]
    fn failed_within_passes_when_under_budget() {
        let mut r = sample_result(1, vec![], None);
        r.duration = std::time::Duration::from_secs(5);
        assert!(failed_within(&r, 20).passed);
    }

    #[test]
    fn failed_within_fails_at_or_over_budget() {
        // 命令式 `duration < Duration::from_secs(20)` 用严格 <;
        // 我们保留相同语义, 所以 20s 边界失败。
        let mut r = sample_result(1, vec![], None);
        r.duration = std::time::Duration::from_secs(20);
        assert!(!failed_within(&r, 20).passed);
        r.duration = std::time::Duration::from_secs(25);
        assert!(!failed_within(&r, 20).passed);
    }

    #[test]
    fn event_payload_contains_prefers_full_stdout_payload_over_truncated_jsonl() {
        // events.jsonl 的 payload 会被截断(>500 字符), 截断后的 JSON 不可解析;
        // 断言应优先用 stdout 解析的完整 payload(含并行 :out:job 前缀归一化)。
        let mut r = sample_result(1, vec!["regional.review.ready"], None);
        r.events[0].payload = "{\"review_status\":\"REA... [truncated, 800 chars total]".to_string();
        r.stdout = "[regional_operating_lead#1:out:job=1] <event topic=\"regional.review.ready\">\n[regional_operating_lead#1:out:job=1] {\"review_status\":\"READY_FOR_REGION_WEEKLY\",\n[regional_operating_lead#1:out:job=1] \"region_code\":\"APAC_ENTERPRISE\",\n[regional_operating_lead#1:out:job=1] \"operating_owner\":\"regional-chief-of-staff\"}\n[regional_operating_lead#1:out:job=1] </event>"
            .to_string();
        assert!(
            event_payload_contains(&r, "regional.review.ready", "review_status: READY_FOR_REGION_WEEKLY")
                .passed
        );
        assert!(
            event_payload_contains(&r, "regional.review.ready", "region_code: APAC_ENTERPRISE")
                .passed
        );
        // stdout 也缺失时, 退回 events.jsonl 的(可能截断)payload。
        let mut r2 = sample_result(1, vec!["build.done"], None);
        r2.events[0].payload = "status: ok".to_string();
        assert!(event_payload_contains(&r2, "build.done", "status: ok").passed);
    }

    #[test]
    fn event_payload_keywords_hits_any() {
        let mut r = sample_result(1, vec!["build.done"], None);
        r.events[0].payload = "tests: pass".to_string();
        let ok = event_payload_keywords(
            &r,
            "build.done",
            &["pass".to_string(), "lint".to_string()],
        );
        assert!(ok.passed);
        let mut r2 = sample_result(1, vec!["build.done"], None);
        r2.events[0].payload = "nothing here".to_string();
        let bad = event_payload_keywords(
            &r2,
            "build.done",
            &["pass".to_string(), "lint".to_string()],
        );
        assert!(!bad.passed);
    }

    #[test]
    fn read_instance_state_parses_agents_json() {
        let dir = tempfile::tempdir().unwrap();
        let ralph = dir.path().join(".ralph");
        std::fs::create_dir_all(&ralph).unwrap();
        std::fs::write(
            ralph.join("agents.json"),
            r#"{"generated_at":"t","instances":[{"instance_id":"ralph#1","state":"idle"}]}"#,
        )
        .unwrap();
        assert_eq!(
            read_instance_state(dir.path(), "ralph#1").unwrap(),
            Some("idle".to_string())
        );
        assert_eq!(read_instance_state(dir.path(), "ghost").unwrap(), None);
        // 文件缺失 → 错误(由 wait 容错转为"未知状态")
        let empty = tempfile::tempdir().unwrap();
        assert!(read_instance_state(empty.path(), "ralph#1").is_err());
    }

    #[test]
    fn wait_instance_tolerates_missing_agents_json() {
        let dir = tempfile::tempdir().unwrap();
        let ralph = dir.path().join(".ralph");
        std::fs::create_dir_all(&ralph).unwrap();
        // 先写 idle, 再写 running, 再写 idle(running_then_idle 语义)
        std::fs::write(
            ralph.join("agents.json"),
            r#"{"instances":[{"instance_id":"ralph#1","state":"idle"}]}"#,
        )
        .unwrap();
        let mut seen = false;
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(wait_instance(
            dir.path(),
            "ralph#1",
            "running_then_idle",
            &mut seen,
            2,
        ));
        assert!(result.is_err(), "idle-only history cannot satisfy running_then_idle");
    }

    #[test]
    fn termination_matches_checks_reason() {
        let r = sample_result(1, vec![], Some("LOOP_COMPLETE"));
        assert!(termination_matches(&r, "LOOP_COMPLETE").passed);
        assert!(!termination_matches(&r, "MAX_ITERATIONS").passed);
    }

    #[test]
    fn first_entry_matches_skips_task_start_and_resume() {
        // 构造事件: task.start(task.start) → build.done → spec.start
        let mut r = sample_result(2, vec!["task.start", "build.done", "spec.start"], None);
        r.events[0].source_instance = Some("ralph#1".to_string());
        r.events[1].source_instance = Some("ralph#1".to_string());
        r.events[2].source_instance = Some("worker#1".to_string());
        // 第一个 ralph#1 非 task.start/resume 事件是 build.done
        assert!(first_entry_matches(&r, "build.done").passed);
        assert!(!first_entry_matches(&r, "spec.start").passed);
        // 忽略其它实例的事件
        let mut r2 = sample_result(1, vec!["spec.start"], None);
        r2.events[0].source_instance = Some("worker#1".to_string());
        assert!(!first_entry_matches(&r2, "spec.start").passed);
        // 无 ralph#1 事件
        let r3 = sample_result(1, vec![], None);
        assert!(!first_entry_matches(&r3, "spec.start").passed);
    }

    #[test]
    fn event_absent_detects_presence() {
        let r = sample_result(1, vec!["docs.start", "build.done"], None);
        assert!(event_absent(&r, "docs.done").passed);
        assert!(!event_absent(&r, "docs.start").passed);
        assert!(!event_absent(&r, "build.done").passed);
    }

    #[test]
    fn event_absent_prefix_matches_gate_style() {
        let r = sample_result(1, vec!["gate.approval", "build.done"], None);
        assert!(!event_absent_prefix(&r, "gate.").passed);
        let r2 = sample_result(1, vec!["build.done"], None);
        assert!(event_absent_prefix(&r2, "gate.").passed);
    }

    #[test]
    fn no_jobs_after_loop_complete_detects_ghost_jobs() {
        // 正常: LOOP_COMPLETE 后没有新 job。
        let mut r = sample_result(1, vec![], Some("LOOP_COMPLETE"));
        r.stdout = "[ralph#1:out:job=1] LOOP_COMPLETE\n".to_string();
        assert!(no_jobs_after_loop_complete(&r).passed);
        // 异常: LOOP_COMPLETE 后又出现新 job。
        r.stdout = "[ralph#1:out:job=1] LOOP_COMPLETE\n[writer#1:out:job=7] ghost\n"
            .to_string();
        assert!(!no_jobs_after_loop_complete(&r).passed);
        // 无 LOOP_COMPLETE 本身即失败。
        let mut r2 = sample_result(1, vec![], None);
        r2.stdout = "[writer#1:out:job=1] work\n".to_string();
        assert!(!no_jobs_after_loop_complete(&r2).passed);
    }

    #[test]
    fn no_jobs_after_loop_complete_requires_exact_payload_match() {
        // 回归: ralph#2 等实例的文本以 "…与 LOOP_COMPLETE" 结尾,
        // 不能误判为完成信号(否则真正完成后的新 job 会被当作幽灵)。
        let mut r = sample_result(1, vec![], None);
        r.stdout = "[ralph#2:out:job=1] next_action: 等待 deployment.ready 后输出最终上线摘要与 LOOP_COMPLETE\n[ralph#1:out:job=5] LOOP_COMPLETE\n"
            .to_string();
        assert!(no_jobs_after_loop_complete(&r).passed);
        // 而 payload 恰好为 LOOP_COMPLETE 时才算完成。
        let mut r2 = sample_result(1, vec![], None);
        r2.stdout = "[ralph#1:out:job=1] LOOP_COMPLETE\n[writer#1:out:job=7] ghost\n"
            .to_string();
        assert!(!no_jobs_after_loop_complete(&r2).passed);
    }

    #[test]
    fn event_order_matches_checks_strict_increasing_positions() {
        let r = sample_result(1, vec!["a", "b", "c"], None);
        assert!(event_order_matches(&r, &["a".into(), "b".into(), "c".into()]).passed);
        assert!(!event_order_matches(&r, &["c".into(), "b".into(), "a".into()]).passed);
        assert!(!event_order_matches(&r, &["a".into(), "missing".into()]).passed);
    }

    #[test]
    fn emit_event_uses_json_flag_for_json_payload() {
        // 轻量验证: 无法真实 spawn ralph, 这里只验证参数构造逻辑的可见部分
        // (真正的 --json 路径在 wait/emit 集成测试覆盖)。
        // 直接验证 wait_for_event 的读取逻辑。
        let dir = tempfile::tempdir().unwrap();
        let ralph = dir.path().join(".ralph");
        std::fs::create_dir_all(&ralph).unwrap();
        let rt = tokio::runtime::Runtime::new().unwrap();
        // 文件不存在 → 超时错误
        let err = rt.block_on(wait_for_event(dir.path(), "approval.requested", 1));
        assert!(err.is_err());
        // 写入事件 → 命中
        std::fs::write(
            ralph.join("events.jsonl"),
            "{\"topic\":\"approval.requested\"}\n",
        )
        .unwrap();
        let ok = rt.block_on(wait_for_event(dir.path(), "approval.requested", 1));
        assert!(ok.is_ok());
    }

    #[test]
    fn render_config_expands_backend_and_model_placeholders() {
        // {model} 占位符: 与命令式 codex_e2e_model() 一致(env 优先, 否则默认)。
        let spec = DeclarativeScenario {
            id: "placeholder-test".to_string(),
            description: String::new(),
            tier: String::new(),
            backends: vec!["codex".to_string()],
            setup: DeclarativeSetup {
                config: "backend={backend}\nmodel={model}".to_string(),
                prompt: None,
                prompt_file: None,
                max_iterations: None,
                timeout_secs: None,
                extra_args: vec![],
                prompt_source: None,
                inject: vec![],
                env: Default::default(),
                write_prompt_to: None,
                write_files: vec![],
                example: None,
                path_prefix: vec![],
            },
            expect: DeclarativeExpect::default(),
        };
        let runner = DeclarativeScenarioRunner::new(spec, PathBuf::from("."));
        let rendered = runner.render_config(Backend::Codex);
        assert!(rendered.contains("backend=codex"));
        assert!(
            rendered.contains(&format!("model={}", crate::scenarios::parallel::codex_e2e_model())),
            "rendered config must use codex_e2e_model(): {rendered}"
        );
    }

    #[test]
    fn render_config_expands_profile_args_when_profile_configured() {
        // 纯函数验证: profile 存在时注入 -p 两行, 否则为空(不依赖进程级 env)。
        let with_profile = render_profile_args(Some("minimax".to_string()));
        // config 块经 serde_yaml 解析后公共缩进已剥离, 注入内容与 `- exec` 同级(4 空格)。
        assert_eq!(with_profile, "- -p\n    - minimax");
        assert!(render_profile_args(None).is_empty());
        // 端到端: 占位符替换后 config 形态正确。
        let spec = DeclarativeScenario {
            id: "profile-test".to_string(),
            description: String::new(),
            tier: String::new(),
            backends: vec!["codex".to_string()],
            setup: DeclarativeSetup {
                // 模拟 serde_yaml 解析后的形态: 公共缩进(4 空格)已被剥离。
                // 占位符所在行保留 4 空格前缀(与 `- exec` 同级)。
                config: "args:\n    - exec\n    {profile_args}\n    - -m\n    - {model}"
                    .to_string(),
                prompt: None,
                prompt_file: None,
                max_iterations: None,
                timeout_secs: None,
                extra_args: vec![],
                prompt_source: None,
                inject: vec![],
                env: Default::default(),
                write_prompt_to: None,
                write_files: vec![],
                example: None,
                path_prefix: vec![],
            },
            expect: DeclarativeExpect::default(),
        };
        let runner = DeclarativeScenarioRunner::new(spec, PathBuf::from("."));
        let rendered = runner
            .spec
            .setup
            .config
            .replace("{profile_args}", &render_profile_args(Some("minimax".to_string())));
        // 占位符行自带 4 空格前缀, 替换后第一行与 `- exec` 对齐。
        assert!(rendered.contains("    - -p\n    - minimax"));
        assert!(
            rendered.contains("args:\n    - exec\n    - -p\n    - minimax\n    - -m"),
            "rendered cli args must align: {rendered}"
        );
    }

    #[test]
    fn emit_spawn_yaml_renders_full_config() {
        // 回归保护: emit-spawn YAML 的 config 块必须完整渲染,
        // 不能因为 ralph_prompt 块缩进错误而截断(event_loop 之后的内容丢失)。
        let yaml = include_str!("../../scenarios/emit-spawn-instance.yaml");
        let runner = super::super::from_yaml("parallel-emit-spawn-instance", yaml);
        let dir = tempfile::tempdir().unwrap();
        let _config = runner
            .setup(dir.path(), Backend::Codex)
            .expect("setup should succeed");
        let rendered = std::fs::read_to_string(dir.path().join("ralph.yml"))
            .expect("ralph.yml should be written");
        for needle in [
            "ralph_prompt: |",
            "E2E_SPAWN_MARKER_42",
            "[E2E_CMD] ralph emit spawn.task",
            "parallel:",
            "hats:",
            "worker:",
            "spawn.done",
        ] {
            assert!(
                rendered.contains(needle),
                "rendered ralph.yml must contain {needle:?}, got:\n{rendered}"
            );
        }
    }
}

#[cfg(test)]
mod yaml_parse_tests {
    use super::DeclarativeScenario;

    // 回归保护: 所有编译期内嵌的声明式场景 YAML 必须可解析。
    // 新增/修改场景时如果 YAML 语法错误, 会在 --list 或 run 时 panic, 这里提前拦截。
    #[test]
    fn all_scenario_yamls_parse() {
        let cases: &[(&str, &str)] = &[
            ("connectivity", include_str!("../../scenarios/connectivity.yaml")),
            ("single-iter", include_str!("../../scenarios/single-iter.yaml")),
            ("multi-iter", include_str!("../../scenarios/multi-iter.yaml")),
            ("completion", include_str!("../../scenarios/completion.yaml")),
            ("events", include_str!("../../scenarios/events.yaml")),
            ("backpressure", include_str!("../../scenarios/backpressure.yaml")),
            ("hat-instances", include_str!("../../scenarios/hat-instances.yaml")),
            ("hat-instances-zh", include_str!("../../scenarios/hat-instances-zh.yaml")),
            ("app-server-idle-start-live", include_str!("../../scenarios/app-server-idle-start-live.yaml")),
            ("steer-multi-turn-live", include_str!("../../scenarios/steer-multi-turn-live.yaml")),
            ("steer-live-reply-multi-turn", include_str!("../../scenarios/steer-live-reply-multi-turn.yaml")),
            ("emit-spawn-instance", include_str!("../../scenarios/emit-spawn-instance.yaml")),
            ("starting-event-inference", include_str!("../../scenarios/starting-event-inference.yaml")),
            ("starting-event-inference-multi-candidate", include_str!("../../scenarios/starting-event-inference-multi-candidate.yaml")),
            ("trigger-routing-example", include_str!("../../scenarios/parallel-trigger-routing-example.yaml")),
            ("pr-review-example", include_str!("../../scenarios/pr-review-example.yaml")),
            ("release-checklist-example", include_str!("../../scenarios/release-checklist-example.yaml")),
            ("audit-evidence-pack-example", include_str!("../../scenarios/audit-evidence-pack-example.yaml")),
            ("customer-advisory-board-prep-example", include_str!("../../scenarios/customer-advisory-board-prep-example.yaml")),
            ("customer-onboarding-activation-example", include_str!("../../scenarios/customer-onboarding-activation-example.yaml")),
            ("customer-renewal-desk-example", include_str!("../../scenarios/customer-renewal-desk-example.yaml")),
            ("executive-business-review-prep-example", include_str!("../../scenarios/executive-business-review-prep-example.yaml")),
            ("field-enablement-rollout-example", include_str!("../../scenarios/field-enablement-rollout-example.yaml")),
            ("finance-close-control-room-example", include_str!("../../scenarios/finance-close-control-room-example.yaml")),
            ("hiring-debrief-panel-example", include_str!("../../scenarios/hiring-debrief-panel-example.yaml")),
            ("incident-response-war-room-example", include_str!("../../scenarios/incident-response-war-room-example.yaml")),
            ("launch-readiness-command-example", include_str!("../../scenarios/launch-readiness-command-example.yaml")),
            ("migration-rehearsal-example", include_str!("../../scenarios/migration-rehearsal-example.yaml")),
            ("multi-region-pipeline-sync-example", include_str!("../../scenarios/multi-region-pipeline-sync-example.yaml")),
            ("partner-launch-coordination-example", include_str!("../../scenarios/partner-launch-coordination-example.yaml")),
            ("postmortem-action-board-example", include_str!("../../scenarios/postmortem-action-board-example.yaml")),
            ("proposal-assembly-example", include_str!("../../scenarios/proposal-assembly-example.yaml")),
            ("regional-operating-review-example", include_str!("../../scenarios/regional-operating-review-example.yaml")),
            ("renewal-risk-calibration-example", include_str!("../../scenarios/renewal-risk-calibration-example.yaml")),
            ("revops-quote-desk-example", include_str!("../../scenarios/revops-quote-desk-example.yaml")),
            ("security-exception-review-example", include_str!("../../scenarios/security-exception-review-example.yaml")),
            ("support-escalation-desk-example", include_str!("../../scenarios/support-escalation-desk-example.yaml")),
            ("vendor-security-procurement-example", include_str!("../../scenarios/vendor-security-procurement-example.yaml")),
            ("human-approval-gate-example", include_str!("../../scenarios/human-approval-gate-example.yaml")),
        ];
        for (id, yaml) in cases {
            let spec: DeclarativeScenario = serde_yaml::from_str(yaml)
                .unwrap_or_else(|e| panic!("invalid YAML for {id}: {e}"));
            assert!(!spec.id.is_empty(), "scenario {id} must have id");
        }
    }
}

#[cfg(test)]
mod profile_render_integration_tests {
    use super::*;

    /// 端到端回归: 内嵌 config 的 5 个场景 YAML, 在注入 profile 后生成的 ralph.yml
    /// cli 段缩进必须正确(与 `- exec` 同级 4 空格), 否则 `-p` 不生效。
    ///
    /// 实现: 不依赖进程级 env, 直接把 {profile_args} 占位符替换为
    /// render_profile_args 的输出(与 render_config 行为一致), 再走 setup 落盘。
    #[test]
    fn inline_config_scenarios_render_valid_profile_args() {
        let cases: &[(&str, &str)] = &[
            ("emit-spawn", include_str!("../../scenarios/emit-spawn-instance.yaml")),
            ("hat-instances", include_str!("../../scenarios/hat-instances.yaml")),
            ("hat-instances-zh", include_str!("../../scenarios/hat-instances-zh.yaml")),
            ("starting-event-inference", include_str!("../../scenarios/starting-event-inference.yaml")),
            ("starting-event-inference-multi-candidate", include_str!("../../scenarios/starting-event-inference-multi-candidate.yaml")),
        ];
        for (id, yaml) in cases {
            let mut spec: DeclarativeScenario = serde_yaml::from_str(yaml)
                .unwrap_or_else(|e| panic!("invalid YAML for {id}: {e}"));
            // 模拟 render_config 的占位符替换(profile 注入)。
            spec.setup.config = spec
                .setup
                .config
                .replace("{profile_args}", &render_profile_args(Some("minimax".to_string())));
            let runner = DeclarativeScenarioRunner::new(spec, PathBuf::from("."));
            let dir = tempfile::tempdir().unwrap();
            runner
                .setup(dir.path(), Backend::Codex)
                .unwrap_or_else(|e| panic!("setup failed for {id}: {e}"));
            let rendered = std::fs::read_to_string(dir.path().join("ralph.yml"))
                .unwrap_or_else(|e| panic!("read ralph.yml failed for {id}: {e}"));
            // 关键断言: `- -p` 必须与 `- exec` 同级(4 空格缩进), 且紧接着是 `- <profile>`。
            let cli_section = rendered
                .split("event_loop:")
                .next()
                .unwrap_or_default();
            let exec_line = cli_section.lines().find(|l| l.trim() == "- exec").expect("missing - exec");
            let exec_indent = exec_line.len() - exec_line.trim_start().len();
            let p_line = cli_section
                .lines()
                .find(|l| l.trim() == "- -p")
                .expect("missing - -p after profile injection");
            let p_indent = p_line.len() - p_line.trim_start().len();
            assert_eq!(
                exec_indent, p_indent,
                "{id}: - -p must align with - exec (both 4 spaces), got exec={exec_indent} p={p_indent}\n{cli_section}"
            );
            assert!(
                cli_section.lines().any(|l| l.trim() == "- minimax"),
                "{id}: missing - minimax after - -p"
            );
            // 生成的 ralph.yml 必须能被解析为合法 YAML。
            let _: serde_yaml::Value = serde_yaml::from_str(&rendered)
                .unwrap_or_else(|e| panic!("{id}: rendered ralph.yml invalid YAML: {e}"));
        }
    }
}
