//! Tier 8: Parallel Runtime (experimental) - example coverage scenarios.
//!
//! 目标：
//! - 直接跑仓库自带的 example：`examples/parallel-experimental-dev-engine`
//! - 用 Codex 真后端验证“并行实验开发永动机”能走完整闭环：
//!   experiment.* -> review -> integration.* -> experiment.complete -> LOOP_COMPLETE
//! - 断言尽量“硬”，优先用 `.ralph/events.jsonl`（比 stdout 更稳）

use super::parallel::{
    parse_parallel_job_line, read_agents_snapshot, replace_top_level_yaml_block,
};
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

// 这里的实验数量与 `fill_experiment_plan` 的预填内容对齐。
//
// 说明：
// - 该 example 的目的是验证“多实验 -> 审计 -> 集成 -> 收敛”的完整闭环。
// - 我们在 E2E 里强制预填 2 个实验，避免真实后端漂移导致“只跑 1 个实验”却误判通过。
const EXPECTED_EXPERIMENTS: usize = 2;

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

    fn patch_example_config_for_e2e(
        config_content: &str,
        backend: Backend,
    ) -> Result<String, ScenarioError> {
        // ---------------------------------------------------------------------
        // 说明：
        // - 我们不修改仓库内的 example 文件。
        // - 但 E2E 运行时会受到本机 `~/.codex/config.toml` 的推理强度/总结输出影响，导致噪音与耗时抖动。
        // - 因此这里仅在 E2E workspace 覆写 `cli` 段，注入 `-c ...` 降噪/提速参数。
        // ---------------------------------------------------------------------
        if backend != Backend::Codex {
            return Ok(config_content.to_string());
        }

        // 注意：该 example 需要更高权限来跑 git/文件写入，因此保留 `--sandbox danger-full-access`。
        let cli_block = r#"cli:
  # E2E: 覆写 Codex 参数,降噪/提速(不影响仓库 example 原文件).
  backend: "custom"
  command: "codex"
  prompt_mode: "arg"
  args:
    - "exec"
    - "--sandbox"
    - "danger-full-access"
    - "-c"
    - 'model_reasoning_effort="low"'
    - "-c"
    - 'model_reasoning_summary="none"'
    - "-c"
    - 'rmcp_client=false'

"#;

        replace_top_level_yaml_block(config_content, "cli:", cli_block).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to patch example ralph.yml cli block for e2e: {e}"
            ))
        })
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

    fn agents_snapshot_written(&self, executor: &RalphExecutor) -> crate::models::Assertion {
        // -----------------------------------------------------------------
        // 说明:
        // - `.ralph/agents.json` 是并行 Supervisor 的运行态快照,用于 `ralph agents` 命令。
        // - 该 example 的“最近新增能力”之一就是把并行实例的状态持续落盘,便于另一个终端观测。
        // - 这里不强约束实例数量(避免 autoscale/动态实例引入 flaky),
        //   只要求包含关键 hat: experiment_runner/auditor/integrator。
        // -----------------------------------------------------------------
        let snapshot = match read_agents_snapshot(executor.workspace()) {
            Ok(s) => s,
            Err(e) => {
                return AssertionBuilder::new("Agents snapshot written (example)")
                    .expected(".ralph/agents.json exists and is valid JSON")
                    .actual(e)
                    .failed()
                    .build();
            }
        };

        let instance_count = snapshot.instances.len();
        let has_runner = snapshot
            .instances
            .iter()
            .any(|i| i.hat_id == "experiment_runner");
        let has_auditor = snapshot
            .instances
            .iter()
            .any(|i| i.hat_id == "experiment_auditor");
        let has_integrator = snapshot
            .instances
            .iter()
            .any(|i| i.hat_id == "experiment_integrator");

        let ok = instance_count >= 3 && has_runner && has_auditor && has_integrator;
        let builder = AssertionBuilder::new("Agents snapshot written (example)")
            .expected("agents.json contains runner + auditor + integrator (and instance_count>=3)")
            .actual(format!(
                "instance_count={instance_count}, runner={has_runner}, auditor={has_auditor}, integrator={has_integrator}"
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn required_topic_chain_observed(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let required = [
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
        let ok = result_with_commit >= EXPECTED_EXPERIMENTS;
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
            //
            // 同时我们只认可 **协调者** 输出的 completion promise：
            // - 避免 prompt/instructions 中出现的 “不要输出 LOOP_COMPLETE” 误触发断言。
            if !completion_seen
                && line.trim_end().ends_with(completion_promise)
                && line.trim_start().starts_with("[ralph#")
                && line.contains(":out:job=")
            {
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

## 约束（Constraints）

- 你必须派发并跑完本计划列出的所有实验条目(至少 exp-001 和 exp-002).
- 在 exp-001 和 exp-002 都完成 `experiment.reviewed` 且 `evidence_ok=true` 之前,你不得进入 integration.
- 你必须在第一批窗口里一次性发布两条 `experiment.task`(exp-001 与 exp-002),不要只发布一条.

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

        let patched_config = Self::patch_example_config_for_e2e(&config_content, backend)?;
        std::fs::write(dest_dir.join("ralph.yml"), patched_config).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace example ralph.yml: {e}"))
        })?;

        let prompt_filled = Self::fill_experiment_plan(&prompt_content)?;
        std::fs::write(dest_dir.join("PROMPT.md"), prompt_filled.as_str()).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace example PROMPT.md: {e}"))
        })?;

        // 说明：
        // - 当前 `event_loop.prompt_file` 的路径解析是相对“进程当前工作目录”(workspace root)，
        //   而不是相对 config 文件所在目录。
        // - 该 example 的 `prompt_file: "PROMPT.md"` 因此会去找 `${workspace}/PROMPT.md`。
        // - 为了保持 example 配置不变,这里在 E2E workspace root 再落一份同名 PROMPT.md 作为入口。
        std::fs::write(workspace.join("PROMPT.md"), prompt_filled.as_str()).map_err(|e| {
            ScenarioError::SetupError(format!("failed to write workspace root PROMPT.md: {e}"))
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
            self.agents_snapshot_written(executor),
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
