//! Tier 8: Parallel Runtime (experimental) - example coverage scenarios.
//!
//! 目标：
//! - 直接跑仓库自带的 example：`examples/parallel-experimental-dev-engine`
//! - 用 Codex 真后端验证“并行实验开发永动机”能走完整闭环：
//!   experiment.* -> review -> integration.* -> experiment.complete -> LOOP_COMPLETE
//! - 断言尽量“硬”，优先用 `.ralph/events.jsonl`（比 stdout 更稳）

use super::parallel::parse_parallel_job_line;
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

/// 直接覆盖 `examples/parallel-experimental-dev-engine` 的端到端（E2E）场景。
///
/// 关注点（偏硬断言）：
/// - **必须**出现关键 topic 链路（experiment -> review -> integration -> complete）
/// - **必须**出现 `commit`（可搬运、可审计的最小产物）
/// - **必须**收敛到 `LOOP_COMPLETE`
///
/// 说明：
/// - 该 example 的设计就是“用户先填 EXPERIMENT_PLAN 再运行”。
///   因此这里在 E2E workspace 里会把 plan 预填成一组轻量、确定能成功的实验（只写入小文件 + rg 验证）。
pub struct ParallelExperimentalDevEngineExampleScenario {
    id: String,
    description: String,
    tier: String,
}

impl ParallelExperimentalDevEngineExampleScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-experimental-dev-engine-example".to_string(),
            description: "Directly runs examples/parallel-experimental-dev-engine (Codex) and asserts the experiment→audit→integration→complete chain"
                .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;
        let visible = stdout.contains("[supervisor] instances");

        let builder = AssertionBuilder::new("Parallel mode visible (example)")
            .expected("stdout contains '[supervisor] instances' banner")
            .actual(if visible {
                "Found supervisor instance banner".to_string()
            } else {
                "Missing supervisor instance banner".to_string()
            });

        if visible {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 这里的实验数量与我们在 E2E setup 里预填的 EXPERIMENT_PLAN 对齐。
        // 目标：避免真实后端波动造成“偶尔只跑了一个实验”却误判通过。
        const EXPECTED_EXPERIMENTS: usize = 2;

        let required = [
            "experiment.start",
            "experiment.task",
            "experiment.result",
            "experiment.reviewed",
            "integration.task",
            "integration.applied",
            "experiment.complete",
        ];

        let mut missing = Vec::new();
        let mut first_index = Vec::new();
        for topic in required {
            let idx = result.events.iter().position(|e| e.topic == topic);
            if idx.is_none() {
                missing.push(topic);
            }
            first_index.push((topic, idx));
        }

        let task_count = result
            .events
            .iter()
            .filter(|e| e.topic == "experiment.task")
            .count();
        let result_count = result
            .events
            .iter()
            .filter(|e| e.topic == "experiment.result")
            .count();
        let reviewed_count = result
            .events
            .iter()
            .filter(|e| e.topic == "experiment.reviewed")
            .count();
        let evidence_ok_count = result
            .events
            .iter()
            .filter(|e| {
                e.topic == "experiment.reviewed"
                    && (e.payload.contains("evidence_ok: true")
                        || e.payload.contains("\"evidence_ok\":true"))
            })
            .count();

        // 关键链路（硬门槛）：
        // - topic 全部出现
        // - experiment.task/result/reviewed 的数量至少等于预填实验数
        // - reviewed 必须明确 evidence_ok=true（否则属于“证据不足也收敛”的回归）
        let ok = missing.is_empty()
            && task_count >= EXPECTED_EXPERIMENTS
            && result_count >= EXPECTED_EXPERIMENTS
            && reviewed_count >= EXPECTED_EXPERIMENTS
            && evidence_ok_count >= EXPECTED_EXPERIMENTS;

        let builder = AssertionBuilder::new("Required topic chain observed (example)")
            .expected(format!(
                "must observe full chain + counts >= {EXPECTED_EXPERIMENTS} (evidence_ok=true required)"
            ))
            .actual(format!(
                "missing={missing:?}; counts: task={task_count}, result={result_count}, reviewed={reviewed_count}, evidence_ok={evidence_ok_count}; first_index={first_index:?}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn commit_artifact_present(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 该 example 约定 runner 必须产出可搬运产物 `commit`（git hash），而不是在事件里嵌入超长 patch 文本。
        let result_with_commit = result
            .events
            .iter()
            .filter(|e| e.topic == "experiment.result" && e.payload.contains("commit"))
            .count();

        // 与 `fill_experiment_plan` 里的预填实验数量对齐：至少每个 experiment 都应该产出一个 commit。
        let ok = result_with_commit >= 2;
        let builder = AssertionBuilder::new("Commit artifact present (example)")
            .expected("experiment.result payload includes commit (git hash) for each experiment")
            .actual(format!(
                "experiment.result with 'commit'={result_with_commit}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_unexpected_gates_or_routing_escalations(
        &self,
        result: &ExecutionResult,
    ) -> crate::models::Assertion {
        // 该 example 默认 permissions=allow，正常情况下不应该出现 gate.*。
        // 同时也不应该出现 routing.escalate（这通常意味着 target/instance 校验失败或路由异常）。
        let bad_topics = [
            "gate.request",
            "gate.resolve",
            "gate.timeout",
            "routing.escalate",
        ];

        let found = result
            .events
            .iter()
            .filter(|e| bad_topics.contains(&e.topic.as_str()))
            .map(|e| e.topic.clone())
            .collect::<Vec<_>>();

        let ok = found.is_empty();
        let builder = AssertionBuilder::new("No unexpected gate/routing escalation (example)")
            .expected("no gate.* and no routing.escalate events")
            .actual(format!("found={found:?}"));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn no_new_jobs_started_after_loop_complete(
        &self,
        result: &ExecutionResult,
    ) -> crate::models::Assertion {
        let completion_promise = "LOOP_COMPLETE";
        let mut completion_seen = false;

        let mut jobs_before: HashSet<(String, u64)> = HashSet::new();
        let mut new_jobs_after: HashSet<(String, u64)> = HashSet::new();

        for line in result.stdout.lines() {
            if let Some((instance_id, job_id)) = parse_parallel_job_line(line) {
                let key = (instance_id, job_id);
                if completion_seen {
                    if !jobs_before.contains(&key) {
                        new_jobs_after.insert(key);
                    }
                } else {
                    jobs_before.insert(key);
                }
            }

            // 注意：必须在解析 job_id 之后再判断 completion，
            // 这样 `[ralph#1:out:job=...] LOOP_COMPLETE` 会被算作 completion 之前的 job。
            if !completion_seen && line.trim_end().ends_with(completion_promise) {
                completion_seen = true;
            }
        }

        let mut new_list = new_jobs_after.into_iter().collect::<Vec<_>>();
        new_list.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));

        let ok = completion_seen && new_list.is_empty();
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE (example)")
            .expected("After LOOP_COMPLETE, no new job_id should appear in stdout")
            .actual(format!(
                "completion_seen={}, new_jobs_after={:?}",
                completion_seen, new_list
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn fill_experiment_plan(prompt_content: &str) -> Result<String, ScenarioError> {
        // 说明：
        // - example 的 PROMPT.md 是一份“Markdown 的 EXPERIMENT_PLAN 模板”（给 ralph#1 的 top-level prompt）。
        // - E2E 为了确定性，会直接覆写 workspace 里的 PROMPT.md 为一份预填计划（同样是 Markdown）。
        // - 这样能把“真后端”的不确定性压到最低：只做轻量文件改动 + rg 验证。
        let _ = prompt_content; // 保留参数，便于未来在此处做结构校验（但当前不依赖 marker/模板内容）。

        // 预填的计划必须“简单、确定、可跑通”：
        // - 每个实验只写一个小文件，并用 rg 验证内容
        // - final_verification 也只做轻量检查，避免 E2E 被编译/网络拖慢
        let plan = r#"# 实验计划（E2E 预填）

## Run ID

- e2e

## 目标（Objective）

- e2e: parallel experimental dev engine

## 选择标准（Selection Criteria）

- 优先采纳：改动更小、且验证证据更完整的 commit
- 如果两个实验结果都等价：优先选择 `exp-001`（减少不确定性）

## 最终验收（Final Verification / 主工作区）

- `rg -n "exp-00[12]" e2e_marker.txt`

## 实验列表（Experiments）

### exp-001：create marker file

#### 实现（Implementation）

1. 创建文件 `e2e_marker.txt`，内容必须包含字符串：`exp-001`
2. 将改动提交为 1 个 commit（用命令级 git 身份，避免环境缺失导致 commit 失败）：
   - `git add -A`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-001: e2e marker file"`
3. 不要修改其他文件

#### 验证（Verification）

- `rg -n "exp-001" e2e_marker.txt`
- `git show --name-only --oneline HEAD`

#### 备注（Notes，可选）

- 产物要求：`experiment.result` 必须包含 `commit`（git hash），不要在 payload 里嵌入 patch 文本。

### exp-002：alternative marker file

#### 实现（Implementation）

1. 创建文件 `e2e_marker.txt`，内容必须包含字符串：`exp-002`
2. 将改动提交为 1 个 commit（用命令级 git 身份，避免环境缺失导致 commit 失败）：
   - `git add -A`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-002: e2e marker file"`
3. 不要修改其他文件

#### 验证（Verification）

- `rg -n "exp-002" e2e_marker.txt`
- `git show --name-only --oneline HEAD`

"#;
        Ok(plan.to_string())
    }
}

impl Default for ParallelExperimentalDevEngineExampleScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelExperimentalDevEngineExampleScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn tier(&self) -> &str {
        &self.tier
    }

    fn supported_backends(&self) -> Vec<Backend> {
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        let root = crate::executor::find_workspace_root().ok_or_else(|| {
            ScenarioError::SetupError("failed to find workspace root (Cargo.toml)".to_string())
        })?;

        let example_dir = root.join("examples/parallel-experimental-dev-engine");
        let example_config_path = example_dir.join("ralph.yml");
        let config_content = std::fs::read_to_string(&example_config_path).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to read example config {}: {e}",
                example_config_path.display()
            ))
        })?;

        let example_prompt_path = example_dir.join("PROMPT.md");
        let prompt_content = std::fs::read_to_string(&example_prompt_path).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to read example prompt {}: {e}",
                example_prompt_path.display()
            ))
        })?;

        // 原样拷贝示例目录，但会在 E2E workspace 里预填 EXPERIMENT_PLAN（否则示例仍是 TODO 模板）。
        let dest_dir = workspace.join("examples/parallel-experimental-dev-engine");
        std::fs::create_dir_all(&dest_dir).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to create example directory in workspace {}: {e}",
                dest_dir.display()
            ))
        })?;

        std::fs::write(dest_dir.join("ralph.yml"), config_content).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace example ralph.yml: {e}"))
        })?;

        let prompt_filled = Self::fill_experiment_plan(&prompt_content)?;
        std::fs::write(dest_dir.join("PROMPT.md"), prompt_filled).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace example PROMPT.md: {e}"))
        })?;

        Ok(ScenarioConfig {
            config_file: "examples/parallel-experimental-dev-engine/ralph.yml".into(),
            // 直接使用 example 配置里的 prompt_file（含我们的 plan 预填），避免 E2E runner 的提示词污染示例语义。
            prompt: PromptSource::Config,
            // 与示例保持一致（当前为 40），避免 E2E 放宽迭代上限掩盖失控行为。
            max_iterations: 40,
            timeout: std::cmp::min(backend.default_timeout(), Duration::from_secs(600)),
            extra_args: vec!["--no-tui".to_string()],
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

        let assertions = vec![
            Assertions::response_received(&execution),
            // 这个 example 目标就是“必须收敛”，因此这里用更硬的 exit_code=0（而不是 0/2）。
            Assertions::exit_code(&execution, 0),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.required_topic_chain_observed(&execution),
            self.commit_artifact_present(&execution),
            self.no_unexpected_gates_or_routing_escalations(&execution),
            self.no_new_jobs_started_after_loop_complete(&execution),
        ];

        let all_passed = assertions.iter().all(|a| a.passed);

        Ok(TestResult {
            scenario_id: self.id.clone(),
            scenario_description: self.description.clone(),
            backend: String::new(), // runner 会填充
            tier: self.tier.clone(),
            passed: all_passed,
            assertions,
            duration,
        })
    }
}
