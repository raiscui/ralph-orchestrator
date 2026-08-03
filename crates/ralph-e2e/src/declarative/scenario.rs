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

        // prompt: 内联或文件。
        let prompt = if let Some(text) = &self.spec.setup.prompt {
            PromptSource::Inline(text.clone())
        } else if let Some(file) = &self.spec.setup.prompt_file {
            let content = std::fs::read_to_string(self.base_dir.join(file)).map_err(|e| {
                ScenarioError::SetupError(format!("failed to read prompt file {file}: {e}"))
            })?;
            PromptSource::Inline(content)
        } else {
            return Err(ScenarioError::SetupError(
                "declarative scenario requires setup.prompt or setup.prompt_file".to_string(),
            ));
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
            extra_args: vec![],
        })
    }

    async fn run(
        &self,
        executor: &RalphExecutor,
        config: &ScenarioConfig,
    ) -> Result<TestResult, ScenarioError> {
        let start = std::time::Instant::now();
        let execution = executor
            .run(config)
            .await
            .map_err(|e| ScenarioError::ExecutionError(format!("ralph execution failed: {e}")))?;
        let duration = start.elapsed();

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

fn output_contains(result: &ExecutionResult, needle: &str) -> crate::models::Assertion {
    let ok = result.stdout.contains(needle);
    let builder = crate::scenarios::AssertionBuilder::new(format!("Output contains {needle:?}"))
        .expected(format!("stdout contains {needle:?}"))
        .actual(if ok { "found".to_string() } else { "missing".to_string() });
    if ok { builder.passed() } else { builder.failed() }.build()
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
    fn termination_matches_checks_reason() {
        let r = sample_result(1, vec![], Some("LOOP_COMPLETE"));
        assert!(termination_matches(&r, "LOOP_COMPLETE").passed);
        assert!(!termination_matches(&r, "MAX_ITERATIONS").passed);
    }
}
