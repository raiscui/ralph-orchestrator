//! Tier 8: Parallel Runtime (experimental) - example coverage scenarios.
//!
//! 目标：
//! - 直接跑仓库自带的 example：`examples/parallel-experimental-dev-engine`
//! - 用 Codex 真后端验证“并行实验开发永动机”能走完整闭环：
//!   experiment.* -> review -> integration.* -> experiment.complete -> LOOP_COMPLETE
//! - 断言尽量“硬”，优先用 `.ralph/events.jsonl`（比 stdout 更稳）

use super::parallel::{
    parse_parallel_job_line, read_agents_snapshot, replace_or_append_top_level_yaml_block,
};
use super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

// 这里的实验数量与 `fill_experiment_plan` 的预填内容对齐。
//
// 说明：
// - 该 example 的目的是验证“多实验 -> 审计 -> 集成 -> 收敛”的完整闭环。
// - 我们在 E2E 里强制预填 2 个实验，避免真实后端漂移导致“只跑 1 个实验”却误判通过。
const EXPECTED_EXPERIMENTS: usize = 2;

// -------------------------------------------------------------------------
// E2E workspace root AGENTS override
// -------------------------------------------------------------------------
// 说明:
// - 这个 example 是验证 `examples/parallel-experimental-dev-engine` 的 workflow 闭环,
//   不是验证仓库根目录里给 Codex 开发本仓库时用的重型 AGENTS 规范。
// - 若直接克隆整个仓库并原样运行,worker 会继承根 `AGENTS.md` 里的六文件/续档/
//   持续学习等流程,把一个本该很短的 experiment/integration 任务拖成超长开发任务。
// - 因此 E2E setup 需要在隔离 workspace 根目录覆盖一份极简 AGENTS,只保留:
//   - 跟随当前 incoming event
//   - 尊重当前 hat instructions
//   - 用最小动作完成当前 workflow 并尽快发出结构化事件
// -------------------------------------------------------------------------
const E2E_WORKSPACE_AGENTS_OVERRIDE: &str = r#"# AGENTS.md

## E2E Workspace

- 这是隔离的 E2E workspace,目标是验证 `examples/parallel-experimental-dev-engine` 的 workflow 是否闭环。
- 这里的首要真相源是:
  - 当前 hat instructions
  - 当前 incoming event payload
  - 当前 example 配置与工作区文件
- 只执行完成当前 event 必需的最小步骤。不要把任务扩展成仓库开发流程。

## Do

- 优先使用当前 hat instructions 明确要求的命令、验证和事件格式。
- 完成 verification 后,直接输出当前 workflow 需要的 `<event ...>...</event>` 正文事件。
- 如果当前任务只需要读写少量文件,就只做这些最小文件操作。

## Do Not

- 不要读取或维护仓库级 `task_plan.md`、`notes.md`、`WORKLOG.md`、`ERRORFIX.md`、`EPIPHANY_LOG.md`、`LATER_PLANS.md`,除非当前 incoming event 明确要求。
- 不要执行 OpenSpec、持续学习、归档、文档同步、项目级 review 流程。
- 不要为了“流程完整”创建额外计划、日志、经验文件。
- 不要把事件写进 shell transcript、文件内容或 stderr 后再口头说明“已经上报”。
- 不要输出与当前 event 无关的大段背景分析。
"#;

// -------------------------------------------------------------------------
// E2E light all-hat overlay
// -------------------------------------------------------------------------
// 说明:
// - `config/all_hat.md` 对真实仓库开发很有用,但对这个 example/E2E 来说偏重。
// - 这里给隔离 workspace 显式切一份“只保留协议必需语义”的轻量 overlay。
// - 目标不是改变默认产品行为,而是减少 worker 被开发型元指令拉长尾巴。
// -------------------------------------------------------------------------
const E2E_LIGHT_ALL_HAT_PROMPT: &str = r#"你是 Ralph workflow 中的一个 agent。
- 优先遵守当前 hat instructions 与 incoming event payload。
- 只执行完成当前 event 必需的最小步骤,不要把任务扩展成仓库开发流程。
- 需要回流 workflow 结果时,直接在最终 assistant 回复中输出原始 `<event ...>...</event>`。
- 不要通过 shell、文件、stderr、代码块或工具 transcript 间接打印 `<event ...>`。
- 如果当前 job 不是在回复人类,不要输出 `reply.human.message`。"#;

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
        let mut patched = config_content.to_string();

        if backend == Backend::Codex {
            // 注意：该 example 需要更高权限来跑 git/文件写入，因此保留 `--sandbox danger-full-access`。
            let cli_block = r#"cli:
  # E2E: 覆写 Codex 参数,降噪/提速(不影响仓库 example 原文件).
  backend: "custom"
  command: "codex"
  prompt_mode: "arg"
  args:
    - "exec"
    - "-m"
    - "gpt-5-codex"
    - "--sandbox"
    - "danger-full-access"
    - "-c"
    - 'model_reasoning_effort="low"'
    - "-c"
    - 'model_reasoning_summary="none"'
    - "-c"
    - 'rmcp_client=false'

"#;

            patched = replace_or_append_top_level_yaml_block(&patched, "cli:", cli_block).map_err(
                |e| {
                    ScenarioError::SetupError(format!(
                        "failed to patch example ralph.yml cli block for e2e: {e}"
                    ))
                },
            )?;
        }

        let core_block = build_inline_all_hat_prompt_block(E2E_LIGHT_ALL_HAT_PROMPT);
        replace_or_append_top_level_yaml_block(&patched, "core:", &core_block).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to patch example ralph.yml core all-hat overlay block for e2e: {e}"
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
                e.topic == "experiment.reviewed" && payload_field_is_true(&e.payload, "evidence_ok")
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
- 如果某个候选缺少明确的顶层 `commit` 字段,它不能直接进入 integration。

## 最终验收（Final Verification / 主工作区）

- `integration.task.final_verification` 必须根据被采纳的 `experiment_id` 选择对应命令:
  - 如果采纳 `exp-001`: `rg -n "exp-001" e2e_marker_exp_001.txt`
  - 如果采纳 `exp-002`: `rg -n "exp-002" e2e_marker_exp_002.txt`
- 不要要求主工作区同时存在 `exp-001` 与 `exp-002` 两个 marker。
- 两个实验都已跑完并通过审计,由 `experiment.reviewed` 链保证,不要求 integrator 把两个实验都 cherry-pick 到主工作区。

## 实验列表（Experiments）

### exp-001：create marker file

#### 实现（Implementation）

1. 创建文件 `e2e_marker_exp_001.txt`，内容必须包含字符串：`exp-001`
2. 将改动提交为 1 个 commit（用命令级 git 身份，避免环境缺失导致 commit 失败）：
   - `git add -A`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-001: e2e marker file"`
3. 不要修改其他文件

#### 验证（Verification）

- `rg -n "exp-001" e2e_marker_exp_001.txt`
- `git show --name-only --oneline HEAD`
- `git rev-parse HEAD`

#### 备注（Notes，可选）

- 产物要求：`experiment.result` 必须包含独立顶层 `commit:` 字段,并且放在 `verification_evidence` 之前。
- `commit` 必须来自真实执行的 `git rev-parse HEAD`。
- `experiment.reviewed` 在 `approved` 时也必须回写同一个顶层 `commit:` 字段。

### exp-002：alternative marker file

#### 实现（Implementation）

1. 创建文件 `e2e_marker_exp_002.txt`，内容必须包含字符串：`exp-002`
2. 将改动提交为 1 个 commit（用命令级 git 身份，避免环境缺失导致 commit 失败）：
   - `git add -A`
   - `git -c user.name="ralph" -c user.email="ralph@local" commit -m "exp-002: e2e marker file"`
3. 不要修改其他文件

#### 验证（Verification）

- `rg -n "exp-002" e2e_marker_exp_002.txt`
- `git show --name-only --oneline HEAD`
- `git rev-parse HEAD`

#### 备注（Notes，可选）

- 产物要求：`experiment.result` 必须包含独立顶层 `commit:` 字段,并且放在 `verification_evidence` 之前。
- `commit` 必须来自真实执行的 `git rev-parse HEAD`。
- `experiment.reviewed` 在 `approved` 时也必须回写同一个顶层 `commit:` 字段。

"#;
        Ok(plan.to_string())
    }

    fn seed_workspace_git_clone(workspace: &Path, repo_root: &Path) -> Result<(), ScenarioError> {
        // -----------------------------------------------------------------
        // 说明：
        // - 这个 example 的 runner / integrator 都会执行 git 命令。
        // - 如果 E2E workspace 只是“仓库里的一个普通子目录”，`git rev-parse --show-toplevel`
        //   会一路向上命中主仓库，把 experiment/integration 的副作用直接落到真实工作树。
        // - 因此这里必须先在 scenario workspace 根部准备一份隔离 git clone，
        //   让 shared workspace / worktree 都只作用于这份副本。
        // - E2E 复跑常配合 `--keep-workspace`，因此 clone 前必须清掉旧目录，
        //   否则第二次 setup 会直接报“destination path already exists”。
        // -----------------------------------------------------------------
        if workspace.exists() {
            let metadata = std::fs::symlink_metadata(workspace).map_err(|e| {
                ScenarioError::SetupError(format!(
                    "failed to stat existing workspace {} before isolated clone: {e}",
                    workspace.display()
                ))
            })?;

            if metadata.is_dir() {
                std::fs::remove_dir_all(workspace).map_err(|e| {
                    ScenarioError::SetupError(format!(
                        "failed to remove existing workspace dir {} before isolated clone: {e}",
                        workspace.display()
                    ))
                })?;
            } else {
                std::fs::remove_file(workspace).map_err(|e| {
                    ScenarioError::SetupError(format!(
                        "failed to remove existing workspace file {} before isolated clone: {e}",
                        workspace.display()
                    ))
                })?;
            }
        }

        let output = Command::new("git")
            .args(["clone", "--no-hardlinks"])
            .arg(repo_root)
            .arg(workspace)
            .output()
            .map_err(|e| {
                ScenarioError::SetupError(format!(
                    "failed to clone repo into isolated example workspace {}: {e}",
                    workspace.display()
                ))
            })?;

        if !output.status.success() {
            return Err(ScenarioError::SetupError(format!(
                "git clone for isolated example workspace failed: workspace={} exit_code={:?} stdout={} stderr={}",
                workspace.display(),
                output.status.code(),
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
            )));
        }

        Ok(())
    }

    fn commit_workspace_seed_state(workspace: &Path) -> Result<(), ScenarioError> {
        // -----------------------------------------------------------------
        // 说明：
        // - 这个 example 的 runner / integrator 使用 `workspace.strategy: worktree`。
        // - `git worktree add --detach <path> HEAD` 只会基于当前 `HEAD` 提交态创建新 worktree，
        //   不会自动带上源工作树里的未提交修改。
        // - E2E setup 会在 clone 后把 `PROMPT.md` / example `ralph.yml` 改成测试专用内容。
        //   如果这里不立刻做一个隔离 snapshot commit，后续 worktree job 看到的仍然是旧版仓库输入。
        // -----------------------------------------------------------------
        let add = Command::new("git")
            .args(["add", "-A"])
            .current_dir(workspace)
            .output()
            .map_err(|e| {
                ScenarioError::SetupError(format!(
                    "failed to stage seeded workspace patches in {}: {e}",
                    workspace.display()
                ))
            })?;

        if !add.status.success() {
            return Err(ScenarioError::SetupError(format!(
                "git add -A for seeded workspace failed: workspace={} exit_code={:?} stdout={} stderr={}",
                workspace.display(),
                add.status.code(),
                String::from_utf8_lossy(&add.stdout),
                String::from_utf8_lossy(&add.stderr),
            )));
        }

        let commit = Command::new("git")
            .args([
                "-c",
                "user.name=ralph",
                "-c",
                "user.email=ralph@local",
                "commit",
                "-m",
                "e2e: seed parallel experimental dev engine workspace",
            ])
            .current_dir(workspace)
            .output()
            .map_err(|e| {
                ScenarioError::SetupError(format!(
                    "failed to create seeded workspace snapshot commit in {}: {e}",
                    workspace.display()
                ))
            })?;

        if !commit.status.success() {
            return Err(ScenarioError::SetupError(format!(
                "seeded workspace snapshot commit failed: workspace={} exit_code={:?} stdout={} stderr={}",
                workspace.display(),
                commit.status.code(),
                String::from_utf8_lossy(&commit.stdout),
                String::from_utf8_lossy(&commit.stderr),
            )));
        }

        Ok(())
    }

    fn write_workspace_root_agents_override(workspace: &Path) -> Result<(), ScenarioError> {
        // -----------------------------------------------------------------
        // 说明：
        // - 覆盖隔离 workspace 根目录的 `AGENTS.md`，避免 example worker 继承仓库开发型
        //   prompt（六文件、续档、持续学习等）。
        // - 这不是修改仓库真实文件；只发生在 E2E clone 工作区里，并会进入 snapshot HEAD，
        //   让后续 worktree job 看到同一套轻量规则。
        // -----------------------------------------------------------------
        std::fs::write(workspace.join("AGENTS.md"), E2E_WORKSPACE_AGENTS_OVERRIDE).map_err(|e| {
            ScenarioError::SetupError(format!(
                "failed to write workspace root AGENTS override {}: {e}",
                workspace.join("AGENTS.md").display()
            ))
        })
    }
}

fn build_inline_all_hat_prompt_block(text: &str) -> String {
    let mut block = String::from("core:\n  all_hat_prompt:\n    mode: inline\n    text: |\n");
    for line in text.lines() {
        block.push_str("      ");
        block.push_str(line);
        block.push('\n');
    }
    block
}

fn payload_field_is_true(payload: &str, field: &str) -> bool {
    // -----------------------------------------------------------------
    // 说明:
    // - `experiment.reviewed` 既可能是 YAML 风格 payload,也可能是 JSON。
    // - 真后端里 JSON 有时会写成 `"evidence_ok": true`，有时是 `"evidence_ok":true`。
    // - 用结构化解析比字符串猜测更稳,只有解析失败时才回退到兼容字符串匹配。
    // -----------------------------------------------------------------
    if let Ok(value) = serde_yaml::from_str::<serde_yaml::Value>(payload) {
        return value.get(field).and_then(serde_yaml::Value::as_bool) == Some(true);
    }

    payload.contains(&format!("{field}: true"))
        || payload.contains(&format!("\"{field}\":true"))
        || payload.contains(&format!("\"{field}\": true"))
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
        let root = crate::executor::find_workspace_root().ok_or_else(|| {
            ScenarioError::SetupError("failed to find workspace root (Cargo.toml)".to_string())
        })?;

        Self::seed_workspace_git_clone(workspace, &root)?;

        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
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

        Self::write_workspace_root_agents_override(workspace)?;

        // -----------------------------------------------------------------
        // 说明：
        // - 上面这些 E2E 专用 patch 只写进了 workspace 工作树。
        // - 但该 example 的 runner / integrator 会从 `HEAD` 切出新 worktree。
        // - 因此必须先把 patch 固化成一次隔离 snapshot commit，保证后续 worktree job
        //   看到的输入世界与 `ralph#1` 当前工作树一致。
        // -----------------------------------------------------------------
        Self::commit_workspace_seed_state(workspace)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn init_git_repo(root: &Path) {
        std::fs::create_dir_all(root.join("examples/parallel-experimental-dev-engine"))
            .expect("create example dir");
        std::fs::write(
            root.join("examples/parallel-experimental-dev-engine/ralph.yml"),
            "event_loop:\n  prompt_file: \"PROMPT.md\"\n",
        )
        .expect("write example config");
        std::fs::write(
            root.join("examples/parallel-experimental-dev-engine/PROMPT.md"),
            "# plan\n",
        )
        .expect("write example prompt");
        std::fs::write(root.join("README.md"), "repo seed\n").expect("write repo seed");

        let init = Command::new("git")
            .args(["init", "-b", "main"])
            .current_dir(root)
            .output()
            .expect("git init");
        assert!(init.status.success(), "git init should succeed");

        let add = Command::new("git")
            .args(["add", "."])
            .current_dir(root)
            .output()
            .expect("git add");
        assert!(add.status.success(), "git add should succeed");

        let commit = Command::new("git")
            .args([
                "-c",
                "user.name=ralph",
                "-c",
                "user.email=ralph@local",
                "commit",
                "-m",
                "seed",
            ])
            .current_dir(root)
            .output()
            .expect("git commit");
        assert!(commit.status.success(), "git commit should succeed");
    }

    #[test]
    fn seed_workspace_git_clone_creates_isolated_git_repo() {
        let repo_root = TempDir::new().expect("repo tempdir");
        init_git_repo(repo_root.path());

        let workspace_parent = TempDir::new().expect("workspace parent");
        let workspace = workspace_parent.path().join("scenario-workspace");

        ParallelExperimentalDevEngineExampleScenario::seed_workspace_git_clone(
            &workspace,
            repo_root.path(),
        )
        .expect("seed isolated clone");

        assert!(
            workspace.join(".git").exists(),
            "workspace should become a git repo clone"
        );
        assert!(
            workspace
                .join("examples/parallel-experimental-dev-engine/ralph.yml")
                .exists(),
            "cloned workspace should contain example config"
        );

        let rev_parse = Command::new("git")
            .args(["rev-parse", "--show-toplevel"])
            .current_dir(&workspace)
            .output()
            .expect("git rev-parse");
        assert!(
            rev_parse.status.success(),
            "isolated workspace clone should answer git rev-parse"
        );
        let actual = std::fs::canonicalize(String::from_utf8_lossy(&rev_parse.stdout).trim())
            .expect("canonicalize git toplevel");
        let expected = std::fs::canonicalize(&workspace).expect("canonicalize workspace");
        assert_eq!(
            actual, expected,
            "git top-level should be the isolated workspace, not an ancestor repo"
        );
    }

    #[test]
    fn seed_workspace_git_clone_replaces_existing_workspace_dir() {
        let repo_root = TempDir::new().expect("repo tempdir");
        init_git_repo(repo_root.path());

        let workspace_parent = TempDir::new().expect("workspace parent");
        let workspace = workspace_parent.path().join("scenario-workspace");
        std::fs::create_dir_all(&workspace).expect("create stale workspace dir");
        std::fs::write(workspace.join("stale.txt"), "stale").expect("write stale file");

        ParallelExperimentalDevEngineExampleScenario::seed_workspace_git_clone(
            &workspace,
            repo_root.path(),
        )
        .expect("replace workspace with isolated clone");

        assert!(
            workspace.join(".git").exists(),
            "workspace should be replaced by a git repo clone"
        );
        assert!(
            !workspace.join("stale.txt").exists(),
            "stale workspace contents should be removed before clone"
        );
    }

    #[test]
    fn seeded_workspace_snapshot_commit_makes_patched_prompt_visible_to_worktree() {
        let repo_root = TempDir::new().expect("repo tempdir");
        init_git_repo(repo_root.path());

        let workspace_parent = TempDir::new().expect("workspace parent");
        let workspace = workspace_parent.path().join("scenario-workspace");
        ParallelExperimentalDevEngineExampleScenario::seed_workspace_git_clone(
            &workspace,
            repo_root.path(),
        )
        .expect("seed isolated clone");

        std::fs::write(
            workspace.join("PROMPT.md"),
            "# 实验计划（E2E 预填）\n\n- snapshot root prompt\n",
        )
        .expect("write workspace root prompt");
        std::fs::write(
            workspace.join("examples/parallel-experimental-dev-engine/PROMPT.md"),
            "# 实验计划（E2E 预填）\n\n- snapshot example prompt\n",
        )
        .expect("write workspace example prompt");
        std::fs::write(
            workspace.join("examples/parallel-experimental-dev-engine/ralph.yml"),
            "event_loop:\n  prompt_file: \"PROMPT.md\"\nparallel:\n  workspace:\n    worktree_backend: worktree\n",
        )
        .expect("write workspace example config");
        ParallelExperimentalDevEngineExampleScenario::write_workspace_root_agents_override(
            &workspace,
        )
        .expect("write workspace root agents override");

        ParallelExperimentalDevEngineExampleScenario::commit_workspace_seed_state(&workspace)
            .expect("commit seeded workspace state");

        let status = Command::new("git")
            .args(["status", "--short"])
            .current_dir(&workspace)
            .output()
            .expect("git status");
        assert!(
            status.status.success(),
            "git status should succeed after seeded workspace commit"
        );
        assert_eq!(
            String::from_utf8_lossy(&status.stdout).trim(),
            "",
            "seeded workspace should be clean after snapshot commit"
        );

        let worktree = workspace.join(".tmp-worktree");
        let add = Command::new("git")
            .args(["worktree", "add", "--detach"])
            .arg(&worktree)
            .arg("HEAD")
            .current_dir(&workspace)
            .output()
            .expect("git worktree add");
        assert!(
            add.status.success(),
            "git worktree add should succeed: stdout={} stderr={}",
            String::from_utf8_lossy(&add.stdout),
            String::from_utf8_lossy(&add.stderr),
        );

        let root_prompt =
            std::fs::read_to_string(worktree.join("PROMPT.md")).expect("read worktree root prompt");
        assert!(
            root_prompt.contains("snapshot root prompt"),
            "worktree should see the committed E2E root prompt, got: {root_prompt}"
        );

        let example_prompt = std::fs::read_to_string(
            worktree.join("examples/parallel-experimental-dev-engine/PROMPT.md"),
        )
        .expect("read worktree example prompt");
        assert!(
            example_prompt.contains("snapshot example prompt"),
            "worktree should see the committed E2E example prompt, got: {example_prompt}"
        );

        let root_agents =
            std::fs::read_to_string(worktree.join("AGENTS.md")).expect("read worktree root AGENTS");
        assert!(
            root_agents.contains("这是隔离的 E2E workspace")
                && root_agents.contains("不要读取或维护仓库级 `task_plan.md`"),
            "worktree should see the committed E2E root AGENTS override, got: {root_agents}"
        );
    }

    #[test]
    fn example_config_does_not_embed_raw_event_blocks() {
        let config =
            include_str!("../../../../examples/parallel-experimental-dev-engine/ralph.yml");

        assert!(
            !config.contains("<event") && !config.contains("</event>"),
            "example config must not contain raw event tags; use escaped display text instead"
        );
    }

    #[test]
    fn example_config_does_not_embed_placeholder_payload_templates() {
        let config =
            include_str!("../../../../examples/parallel-experimental-dev-engine/ralph.yml");

        assert!(
            !config.contains("...payload...") && !config.contains("### exp-001: ..."),
            "example config must not teach placeholder payloads that the model can echo back"
        );
    }

    #[test]
    fn example_config_requires_structured_commit_fields_for_review_and_integration() {
        let config =
            include_str!("../../../../examples/parallel-experimental-dev-engine/ralph.yml");

        assert!(
            config.contains("顶层 `commit` 字段")
                && config.contains("experiment.reviewed.commit")
                && config.contains("git rev-parse HEAD"),
            "example config must require a top-level commit field that survives truncation and can flow into integration"
        );
    }

    #[test]
    fn filled_experiment_plan_requires_rev_parse_and_structured_commit_contract() {
        let plan = ParallelExperimentalDevEngineExampleScenario::fill_experiment_plan("# template")
            .expect("fill experiment plan");

        assert!(
            plan.matches("git rev-parse HEAD").count() >= 2,
            "filled experiment plan should require rev-parse for both experiments"
        );
        assert!(
            plan.contains("独立顶层 `commit:` 字段")
                && plan.contains("experiment.reviewed")
                && plan.contains("verification_evidence"),
            "filled experiment plan should reinforce the structured commit contract"
        );
    }

    #[test]
    fn patch_example_config_for_e2e_adds_lightweight_all_hat_overlay() {
        let config = "event_loop:\n  prompt_file: \"PROMPT.md\"\n";
        let patched = ParallelExperimentalDevEngineExampleScenario::patch_example_config_for_e2e(
            config,
            Backend::Codex,
        )
        .expect("patch example config");

        assert!(
            patched.contains("core:\n  all_hat_prompt:\n    mode: inline\n    text: |"),
            "patched config should add a runtime all-hat overlay block"
        );
        assert!(
            patched.contains("只执行完成当前 event 必需的最小步骤"),
            "patched config should carry the lightweight E2E overlay"
        );
    }

    #[test]
    fn payload_field_is_true_accepts_yaml_and_both_json_spacing_styles() {
        assert!(payload_field_is_true(
            "evidence_ok: true\nverdict: approved\n",
            "evidence_ok"
        ));
        assert!(payload_field_is_true(
            "{\"evidence_ok\":true,\"verdict\":\"approved\"}",
            "evidence_ok"
        ));
        assert!(payload_field_is_true(
            "{\n  \"evidence_ok\": true,\n  \"verdict\": \"approved\"\n}",
            "evidence_ok"
        ));
        assert!(
            !payload_field_is_true("{\"evidence_ok\":false}", "evidence_ok"),
            "false should not be treated as true"
        );
    }
}
