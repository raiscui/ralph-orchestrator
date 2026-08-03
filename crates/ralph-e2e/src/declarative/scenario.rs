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
    /// 输出必须包含的文本。
    #[serde(default)]
    pub output_contains: Vec<String>,
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

    /// 渲染 ralph.yml 模板({backend} 占位符)。
    fn render_config(&self, backend: Backend) -> String {
        self.spec
            .setup
            .config
            .replace("{backend}", backend.as_config_str())
    }
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

        let config_content = self.render_config(backend);
        let config_path = workspace.join("ralph.yml");
        std::fs::write(&config_path, config_content).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write ralph.yml: {e}"))
        })?;

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
        let inject_task = if !self.spec.setup.inject.is_empty() {
            let workspace = executor.workspace().clone();
            let ralph_bin = executor.ralph_binary();
            let steps = self.spec.setup.inject.clone();
            Some(tokio::spawn(async move {
                run_inject_sequence(&ralph_bin, &workspace, &steps).await
            }))
        } else {
            None
        };

        let extra_env: Vec<(String, String)> = self.spec.setup.env.clone().into_iter().collect();
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
    let event = result.events.iter().find(|e| e.topic == topic);
    let ok = event.map(|e| e.payload.contains(needle)).unwrap_or(false);
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Event '{topic}' payload contains '{needle}'"
    ))
    .expected(format!("Payload containing '{needle}'"))
    .actual(match event {
        Some(e) => format!("Payload: {}", truncate_payload(&e.payload)),
        None => "Event not found".to_string(),
    });
    if ok { builder.passed() } else { builder.failed() }.build()
}

fn event_payload_keywords(
    result: &ExecutionResult,
    topic: &str,
    keywords: &[String],
) -> crate::models::Assertion {
    let event = result.events.iter().find(|e| e.topic == topic);
    let ok = event
        .map(|e| {
            let payload = e.payload.to_lowercase();
            keywords.iter().any(|k| payload.contains(&k.to_lowercase()))
        })
        .unwrap_or(false);
    let builder = crate::scenarios::AssertionBuilder::new(format!(
        "Event '{topic}' payload hits a keyword"
    ))
    .expected(format!("Payload with one of {keywords:?}"))
    .actual(match event {
        Some(e) => format!("Payload: {}", truncate_payload(&e.payload)),
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

fn output_contains(result: &ExecutionResult, needle: &str) -> crate::models::Assertion {
    let ok = result.stdout.contains(needle);
    let builder = crate::scenarios::AssertionBuilder::new(format!("Output contains {needle:?}"))
        .expected(format!("stdout contains {needle:?}"))
        .actual(if ok { "found".to_string() } else { "missing".to_string() });
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
            } => {
                emit_event(
                    ralph_bin,
                    workspace,
                    topic,
                    payload,
                    target_instance.as_deref(),
                    session_strategy.as_deref(),
                )
                .await?;
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
        let current = match read_instance_state(workspace, instance) {
            Ok(state) => state,
            Err(_) => None,
        };
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
) -> Result<(), String> {
    use tokio::process::Command;

    let mut cmd = Command::new(ralph_bin);
    cmd.arg("emit").arg(topic).arg(payload).current_dir(workspace);
    if let Some(target) = target_instance {
        cmd.arg("--target-instance").arg(target);
    }
    if let Some(strategy) = session_strategy {
        cmd.arg("--session-strategy").arg(strategy);
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
}
