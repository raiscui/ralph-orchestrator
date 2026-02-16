use super::super::{AssertionBuilder, Assertions, ScenarioError, TestScenario};
use super::{JobRunCounts, parse_parallel_job_line};
use crate::Backend;
use crate::executor::{ExecutionResult, PromptSource, RalphExecutor, ScenarioConfig};
use crate::models::TestResult;
use async_trait::async_trait;
use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

// =============================================================================
// ParallelHatInstancesScenario - Validate parallel hat instances end-to-end
// =============================================================================

/// 验证并行 HatInstance（headless）在真实后端上能跑通最小闭环。
///
/// 关注点（E2E 维度）：
/// - `parallel.enabled=true` 时，Supervisor 能启动多个实例（含同一 hat 的多实例）
/// - 不写 `topic_contracts` 时，默认按 `hats.*.triggers` 路由（topic → hats fanout）
/// - 输出归因可观测（stdout 带 `[writer#1:out]` 这类前缀）
/// - `<event ...>` 能被解析并写入 `.ralph/events*.jsonl`
/// - 目标校验失败会触发 `routing.escalate`（可观测信号）
///
/// 注意：
/// - E2E 环境无法交互回答 `gate.request`，因此该场景不启用 worktree/hooks 权限 gate。
pub struct ParallelHatInstancesScenario {
    id: String,
    description: String,
    tier: String,
    locale: ParallelScenarioLocale,
}

#[derive(Clone, Copy, Debug)]
enum ParallelScenarioLocale {
    English,
    Chinese,
}

impl ParallelHatInstancesScenario {
    pub fn new() -> Self {
        Self {
            id: "parallel-hat-instances".to_string(),
            // 说明：
            // - 这个场景最初用于验证 parallel hat instances。
            // - 现在也覆盖 `parallel-trigger-routing`：不写 topic_contracts 时的 triggers 默认路由、
            //   strict target 校验、以及 completion 收敛闭环。
            description:
                "Validates parallel-trigger-routing in parallel runtime (triggers fanout + strict target + completion)"
                    .to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
            locale: ParallelScenarioLocale::English,
        }
    }

    /// 中文版本：用于覆盖“中文提示词 + 同一套并行语义约束”的稳定性回归。
    pub fn new_zh() -> Self {
        Self {
            id: "parallel-hat-instances-zh".to_string(),
            description: "验证并行 runtime 在中文提示词下的路由/严格投递/收敛稳定性".to_string(),
            tier: "Tier 8: Parallel Runtime".to_string(),
            locale: ParallelScenarioLocale::Chinese,
        }
    }

    fn cli_config_yaml(backend: Backend) -> String {
        // ---------------------------------------------------------------------
        // 说明：
        // - 该 helper 只用于 E2E 生成的 `ralph.yml`，不影响仓库默认配置。
        // - 目的：为 Codex 后端注入降噪/提速参数，避免真实 Codex 下 stderr 充满长篇思考输出，
        //   进而拖慢并行场景的收敛速度、并使排障信息被淹没。
        //
        // 技术点：
        // - 命名后端 `cli.backend: codex` 目前无法追加任意 `codex exec` 参数。
        // - 因此这里用 `backend: custom + command: codex + args: [...]` 精确控制执行参数。
        // ---------------------------------------------------------------------
        match backend {
            Backend::Codex => r#"  backend: custom
  command: codex
  args:
    - exec
    - --full-auto
    # 降低推理强度/总结输出（避免 E2E 噪音与延迟）。
    - -c
    - 'model_reasoning_effort="low"'
    - -c
    - 'model_reasoning_summary="none"'
    # E2E 场景不需要 MCP client 能力，关闭可减少启动噪音。
    - -c
    - 'rmcp_client=false'
"#
            .to_string(),
            _ => format!("  backend: {}\n", backend.as_config_str()),
        }
    }

    fn parallel_mode_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;
        let visible = stdout.contains("[supervisor] instances");
        let builder = AssertionBuilder::new("Parallel mode visible")
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

    fn attributed_outputs_visible(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let stdout = &result.stdout;

        // 这里不依赖模型在 payload 里回显 instance_id，而是依赖 runner 的“日志归因前缀”。
        //
        // 注意：并行日志模式的输出行形如 `[writer#1:out:job=12] ...`，因此这里用 `:out:job=` 匹配。
        let has_writer_1 = stdout.contains("[writer#1:out:job=")
            || stdout.contains("[writer#1:err:job=")
            || stdout.contains("[writer#1:state]");
        let has_writer_2 = stdout.contains("[writer#2:out:job=")
            || stdout.contains("[writer#2:err:job=")
            || stdout.contains("[writer#2:state]");
        let has_tester_1 = stdout.contains("[tester#1:out:job=")
            || stdout.contains("[tester#1:err:job=")
            || stdout.contains("[tester#1:state]");
        let ok = has_writer_1 && has_writer_2 && has_tester_1;

        let builder = AssertionBuilder::new("Attributed instance output")
            .expected("stdout shows writer#1 + writer#2 + tester#1 output/state prefixes")
            .actual(format!(
                "writer#1: {}, writer#2: {}, tester#1: {}",
                has_writer_1, has_writer_2, has_tester_1
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn expected_events_recorded(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let build_task_count = result
            .events
            .iter()
            .filter(|e| e.topic == "build.task")
            .count();
        let build_done_count = result
            .events
            .iter()
            .filter(|e| e.topic == "build.done")
            .count();
        let test_done_count = result
            .events
            .iter()
            .filter(|e| e.topic == "test.done")
            .count();

        // 说明：
        // - 本场景会触发两次 build.task：
        //   1) fanout -> writer#1 + tester#1
        //   2) target=writer -> 触发 autoscale，期望出现 writer#2
        let ok = build_task_count >= 2 && build_done_count >= 2 && test_done_count >= 1;
        let builder = AssertionBuilder::new("Parallel events recorded")
            .expected("events.jsonl contains >=2 build.task, >=2 build.done, >=1 test.done")
            .actual(format!(
                "build.task: {}, build.done: {}, test.done: {}",
                build_task_count, build_done_count, test_done_count
            ));

        if ok {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn routing_escalate_recorded(&self, result: &ExecutionResult) -> crate::models::Assertion {
        let has_escalate = result.events.iter().any(|e| e.topic == "routing.escalate");

        let builder = AssertionBuilder::new("routing.escalate recorded")
            .expected("events.jsonl contains routing.escalate (invalid target must be rejected)")
            .actual(if has_escalate {
                "Found routing.escalate".to_string()
            } else {
                "Missing routing.escalate".to_string()
            });

        if has_escalate {
            builder.passed().build()
        } else {
            builder.failed().build()
        }
    }

    fn hat_run_counts_expected(&self, result: &ExecutionResult) -> crate::models::Assertion {
        // 说明：
        // - 这里统计的是“job 次数”，而不是事件次数。
        // - 一个 job 可能输出很多行，因此必须按 job_id 去重（见 JobRunCounts）。
        let counts = JobRunCounts::from_stdout(&result.stdout);
        let ralph_runs = counts.runs_for_hat("ralph");
        let writer_runs = counts.runs_for_hat("writer");
        let tester_runs = counts.runs_for_hat("tester");
        let collector_runs = counts.runs_for_hat("collector");
        let writer_2_runs = counts.runs_for_instance("writer#2");

        // 语义断言（稳定优先）：
        // - ralph 至少被调用两次（启动 + 收敛）；
        // - writer 总计至少两次，且必须出现 writer#2（验证定向第二任务）；
        // - tester 至少一次；
        // - collector 至少两次（处理 test.done/build.done 链路）。
        let ok = ralph_runs >= 2
            && writer_runs >= 2
            && tester_runs >= 1
            && collector_runs >= 2
            && writer_2_runs >= 1;
        let builder = AssertionBuilder::new("Hat run counts")
            .expected("ralph>=2, writer(total)>=2 with writer#2>=1, tester>=1, collector(total)>=2")
            .actual(format!(
                "instances: {}; hats: {}; writer#2={}",
                counts.summary(),
                counts.hat_summary(),
                writer_2_runs
            ));

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
        // 说明：
        // - `LOOP_COMPLETE` 出现后，允许“已在跑的 job”继续输出（同一个 job_id 会重复出现）。
        // - 但不允许再出现“新的 job_id”（这意味着 completion 之后仍在派生新 job）。
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
        let builder = AssertionBuilder::new("No new jobs after LOOP_COMPLETE")
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
}

impl Default for ParallelHatInstancesScenario {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl TestScenario for ParallelHatInstancesScenario {
    fn id(&self) -> &str {
        &self.id
    }

    fn description(&self) -> &str {
        &self.description
    }

    fn tier(&self) -> &str {
        &self.tier
    }

    /// 目前先限制在 Codex：
    /// - 并行场景对 headless/并发稳定性更敏感
    /// - 先把 Codex 跑稳，后续再扩展到更多后端
    fn supported_backends(&self) -> Vec<Backend> {
        vec![Backend::Codex]
    }

    fn setup(&self, workspace: &Path, backend: Backend) -> Result<ScenarioConfig, ScenarioError> {
        // 创建 `.agent/`（某些代码路径会假设其存在）
        let agent_dir = workspace.join(".agent");
        std::fs::create_dir_all(&agent_dir).map_err(|e| {
            ScenarioError::SetupError(format!("failed to create .agent directory: {e}"))
        })?;

        // 配置目标（对齐 parallel-workflow-semantics）：
        // - 启用 parallel runtime（不写 topic_contracts，完全依赖 triggers 默认路由）
        // - 用 `event_loop.starting_event` 固化工作流入口：由 ralph#1 在 task.start 后发布
        // - 用 `event_loop.complete_publishes` 固化工作流收敛：当 ralph#1 观察到该 topic 时输出 LOOP_COMPLETE
        //
        // 行为设计（E2E 稳定性优先）：
        // - starting_event=build.task：触发 writer + tester 并发；
        // - complete_publishes=routing.escalate：把“严格投递失败”作为 completion candidate；
        // - writer 固定 2 个实例，tester 直接把第二个任务定向到 writer#2（避免模型漂移导致实例选择抖动）；
        // - collector 在收到 build.done(task_id=2) 时仅发 invalid target 事件，由 Supervisor 生成 routing.escalate。
        let cli_config = Self::cli_config_yaml(backend);
        let config_content = match self.locale {
            ParallelScenarioLocale::English => format!(
                r#"# Parallel hat instances E2E config for {backend}
cli:
{cli_config}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  # workflow entry/exit（官方并行语义锚点）
  starting_event: "build.task"
  complete_publishes: "routing.escalate"
  max_iterations: 12
  # 并行模式下如果模型不输出 completion promise，必须有硬退出护栏，避免 E2E 卡死
  max_runtime_seconds: 240

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 4
    dynamic_idle_ttl_secs: 30

  # E2E 场景中不启用 ask gate，避免“等待 human”导致卡住
  permissions:
    worktree: allow
    hooks: allow

hats:
  writer:
    name: "Writer"
    description: "Writes a short output and emits build.done."
    instances: 2
    triggers: ["build.task"]
    publishes: ["build.done"]
    instructions: |
      You are Writer.

      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Print exactly ONE short stdout line per job.
      - Emit exactly ONE build.done event per job.
      - Keep output concise. Do NOT print long loops.

      Task ID rule:
      - If payload contains "task_id: 2", emit `task_id: 2`.
      - Otherwise emit `task_id: 1`.

      writer ok
      <event topic="build.done">
      task_id: 1
      status: ok
      </event>

      Do NOT output LOOP_COMPLETE.

  tester:
    name: "Tester"
    description: "Emits test.done and dispatches the second task to writer#2."
    instances: 1
    triggers: ["build.task"]
    publishes: ["test.done", "build.task"]
    instructions: |
      You are Tester.

      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Print exactly ONE short stdout line per job so the harness can count job runs.

      Rule:
      - Only when payload contains "task_id: 1":
        1) Print one short line that includes the word "tester"
        2) Emit ONE test.done event:

        <event topic="test.done">
        status: ok
        </event>

        3) Emit ONE build.task event targeting writer#2 (instance-direct):

        <event topic="build.task" target_instance="writer#2">
        task_id: 2
        Task: Second task must run on writer#2
        </event>

      - For any other payload, emit NO events.

      Do NOT output LOOP_COMPLETE.

  collector:
    name: "Collector"
    description: "Consumes build/test events and closes workflow via strict target failure."
    instances: 1
    triggers: ["build.done", "test.done"]
    publishes: ["build.task", "routing.escalate"]
    instructions: |
      You are Collector.

      IMPORTANT (E2E harness):
      - Do NOT run tests, do NOT run shell commands/tools, do NOT edit files.
      - Print exactly ONE short stdout line per job.
      - Do NOT wrap `<event ...>` blocks in markdown fences.

      Rules:
      - Your single stdout line MUST include:
        - the word "collector"
        - and the topics you received in this job (comma-separated, e.g. "build.done,test.done")
      - If the "Incoming Events" section contains BOTH substrings:
        - "topic=build.done"
        - "task_id: 2"
        then you MUST emit EXACTLY ONE invalid build.task event targeting ghost_hat (hat id, not instance id).
        Copy-paste this block exactly (no extra attributes, no code fences):

        <event topic="build.task" target="ghost_hat">
        Task: This must be rejected and should trigger routing.escalate
        </event>

      - Do NOT emit routing.escalate manually.
      - Otherwise emit NO events.
"#,
                backend = backend,
                cli_config = cli_config,
            ),
            ParallelScenarioLocale::Chinese => format!(
                r#"# Parallel hat instances E2E config for {backend}
cli:
{cli_config}

event_loop:
  completion_promise: "LOOP_COMPLETE"
  # workflow entry/exit（官方并行语义锚点）
  starting_event: "build.task"
  complete_publishes: "routing.escalate"
  max_iterations: 12
  # 并行模式下如果模型不输出 completion promise，必须有硬退出护栏，避免 E2E 卡死
  max_runtime_seconds: 240

parallel:
  enabled: true
  autoscale:
    max_running_jobs: 4
    dynamic_idle_ttl_secs: 30

  # E2E 场景中不启用 ask gate，避免“等待 human”导致卡住
  permissions:
    worktree: allow
    hooks: allow

hats:
  writer:
    name: "写作员"
    description: "输出一段文本，并发出 build.done。"
    instances: 2
    triggers: ["build.task"]
    publishes: ["build.done"]
    instructions: |
      你是 Writer（写作员）。

      重要（E2E harness 约束）：
      - 不要运行测试，不要运行任何 shell 命令/工具，不要编辑文件。
      - 每次 job 只输出 1 行短 stdout。
      - 每次 job 只发出 1 个 build.done 事件。
      - 输出要精简，不要打印长循环。

      task_id 规则：
      - 如果 payload 包含 "task_id: 2"，输出 `task_id: 2`。
      - 否则输出 `task_id: 1`。

      writer ok
      <event topic="build.done">
      task_id: 1
      status: ok
      </event>

      不要输出 LOOP_COMPLETE。

  tester:
    name: "测试员"
    description: "发出 test.done，并把第二个任务定向给 writer#2。"
    instances: 1
    triggers: ["build.task"]
    publishes: ["test.done", "build.task"]
    instructions: |
      你是 Tester（测试员）。

      重要（E2E harness 约束）：
      - 不要运行测试，不要运行任何 shell 命令/工具，不要编辑文件。
      - 每次 job 只输出 1 行 stdout（让 harness 能统计 job 运行次数）。

      规则：
      - 仅当 payload 包含 "task_id: 1" 时：
        1) 打印 1 行短文本，包含单词 "tester"
        2) 发出 1 个 test.done 事件：

        <event topic="test.done">
        status: ok
        </event>

        3) 发出 1 个 build.task 事件，定向到 writer#2（实例直达）：

        <event topic="build.task" target_instance="writer#2">
        task_id: 2
        Task: Second task must run on writer#2
        </event>

      - 其他 payload 不发事件。

      不要输出 LOOP_COMPLETE。

  collector:
    name: "收集员"
    description: "消费 build/test 事件，并通过严格投递失败触发工作流收敛。"
    instances: 1
    triggers: ["build.done", "test.done"]
    publishes: ["build.task", "routing.escalate"]
    instructions: |
      你是 Collector（收集员）。

      重要（E2E harness 约束）：
      - 不要运行测试,不要运行任何 shell 命令/工具,不要编辑文件。
      - 每次 job 只输出 1 行短 stdout。
      - 不要用 markdown code fence 包裹 `<event ...>`。

      规则：
      - 你这一行 stdout 必须同时包含：
        - 单词 "collector"
        - 本次 job 收到的 topic 列表(用逗号分隔,例如 "build.done,test.done")
      - 如果 "Incoming Events" 区域里同时出现以下两个子串：
        - "topic=build.done"
        - "task_id: 2"
        那你必须输出且只输出 1 个无效 build.task 事件(target=ghost_hat,是 hat id,不是 instance id)。
        请原样复制下面这段(不要加额外属性,不要加 code fence)：

        <event topic="build.task" target="ghost_hat">
        Task: This must be rejected and should trigger routing.escalate
        </event>

      - 不要手工发 routing.escalate。
      - 否则不发事件。
"#,
                backend = backend,
                cli_config = cli_config,
            ),
        };
        std::fs::write(workspace.join("ralph.yml"), config_content)
            .map_err(|e| ScenarioError::SetupError(format!("failed to write ralph.yml: {e}")))?;

        // 说明：
        // - 并行模式下，Coordinator（ralph#1）会持续多轮被调用。
        // - 这里支持用环境变量切换 prompt 变体，以覆盖“prompt 自身包含 <event> 示例文本”时的误解析风险。
        //
        // 用法：
        // - RALPH_E2E_PARALLEL_PROMPT_VARIANT=baseline|variant1|variant2
        // - 这只影响 prompt 文本，不影响 ralph.yml 配置与断言逻辑。
        // - 目的：覆盖“prompt 自身包含 `<event ...>` 文本”时，模型是否会错误回显导致误解析。
        let prompt_variant = std::env::var("RALPH_E2E_PARALLEL_PROMPT_VARIANT")
            .unwrap_or_else(|_| "baseline".to_string());

        let prompt = match (self.locale, prompt_variant.as_str()) {
            (ParallelScenarioLocale::English, "variant1") => {
                r#"You are running an E2E test for Ralph's PARALLEL runtime.

NOTE:
- This prompt includes an EXAMPLE event block below.
- The EXAMPLE block is NOT a real event. Do NOT emit it.

EXAMPLE ONLY (do not emit, do not reprint):
<event topic="fake.echo">
status: this must NOT be recorded as a real event
</event>

Goal:
- Follow the configured workflow (starting_event + complete_publishes).
- When you emit the workflow entry event (starting_event: build.task), the payload MUST include the line `task_id: 1`.
- Do NOT implement code, do NOT run tools/shell commands, do NOT edit files.
- In this E2E test, `routing.escalate` is EXPECTED (invalid target is deliberate).
- You MUST output `LOOP_COMPLETE` on its own line and stop when you observe the completion candidate event `routing.escalate`.
- Do NOT retry or emit follow-up events when you see `routing.escalate`.

IMPORTANT:
- Do NOT echo or reprint the EXAMPLE ONLY block above.
"#
            }
            (ParallelScenarioLocale::English, "variant2") => {
                r#"You are running an E2E test for Ralph's PARALLEL runtime.

NOTE:
- This prompt includes a fenced code example below.
- The fenced EXAMPLE is NOT a real event. Do NOT emit it.

```xml
<event topic="fake.fenced">
status: this must NOT be recorded as a real event
</event>
```

Goal:
- Follow the configured workflow (starting_event + complete_publishes).
- When you emit the workflow entry event (starting_event: build.task), the payload MUST include the line `task_id: 1`.
- Do NOT implement code, do NOT run tools/shell commands, do NOT edit files.
- In this E2E test, `routing.escalate` is EXPECTED (invalid target is deliberate).
- You MUST output `LOOP_COMPLETE` on its own line and stop when you observe the completion candidate event `routing.escalate`.
- Do NOT retry or emit follow-up events when you see `routing.escalate`.

IMPORTANT:
- Do NOT echo or reprint the fenced EXAMPLE block above.
"#
            }
            (ParallelScenarioLocale::English, _) => {
                r"You are running an E2E test for Ralph's PARALLEL runtime.

Goal:
- Follow the configured workflow (starting_event + complete_publishes).
- When you emit the workflow entry event (starting_event: build.task), the payload MUST include the line `task_id: 1`.
- Do NOT implement code, do NOT run tools/shell commands, do NOT edit files.
- In this E2E test, `routing.escalate` is EXPECTED (invalid target is deliberate).
- You MUST output `LOOP_COMPLETE` on its own line and stop when you observe the completion candidate event `routing.escalate`.
- Do NOT retry or emit follow-up events when you see `routing.escalate`.
"
            }
            (ParallelScenarioLocale::Chinese, "variant1") => {
                r#"你正在运行 Ralph 的并行 runtime 的端到端（E2E）测试。

注意：
- 这个提示词里包含一个“示例事件块（EXAMPLE）”。
- 这个示例块不是实际事件，禁止输出它、禁止复述它。

仅示例（不要输出、不要复述）：
<event topic="fake.echo">
status: 这段绝不能被记录为真实事件
</event>

目标：
- 遵循配置中的工作流（starting_event + complete_publishes）。
- 当你发出工作流入口事件（starting_event: build.task）时，payload 必须包含一行 `task_id: 1`。
- 不实现代码，不运行任何工具/命令，不编辑文件。
- 在这个 E2E 测试里，`routing.escalate` 是预期事件（无效 target 是故意的）。
- 当你观察到完成候选事件 `routing.escalate` 时，你必须单独一行输出 `LOOP_COMPLETE` 并停止。
- 看到 `routing.escalate` 不要重试，也不要再派发后续事件。

重要：
- 不要回显或复述上面的“仅示例”事件块。
"#
            }
            (ParallelScenarioLocale::Chinese, "variant2") => {
                r#"你正在运行 Ralph 的并行 runtime 的端到端（E2E）测试。

注意：
- 这个提示词里包含一个 fenced code block 示例。
- fenced 示例不是实际事件，禁止输出它、禁止复述它。

```xml
<event topic="fake.fenced">
status: 这段绝不能被记录为真实事件
</event>
```

目标：
- 遵循配置中的工作流（starting_event + complete_publishes）。
- 当你发出工作流入口事件（starting_event: build.task）时，payload 必须包含一行 `task_id: 1`。
- 不实现代码，不运行任何工具/命令，不编辑文件。
- 在这个 E2E 测试里，`routing.escalate` 是预期事件（无效 target 是故意的）。
- 当你观察到完成候选事件 `routing.escalate` 时，你必须单独一行输出 `LOOP_COMPLETE` 并停止。
- 看到 `routing.escalate` 不要重试，也不要再派发后续事件。

重要：
- 不要回显或复述上面的 fenced 示例。
"#
            }
            (ParallelScenarioLocale::Chinese, _) => {
                r"你正在运行 Ralph 的并行 runtime 的端到端（E2E）测试。

目标：
- 遵循配置中的工作流（starting_event + complete_publishes）。
- 当你发出工作流入口事件（starting_event: build.task）时，payload 必须包含一行 `task_id: 1`。
- 不实现代码，不运行任何工具/命令，不编辑文件。
- 在这个 E2E 测试里，`routing.escalate` 是预期事件（无效 target 是故意的）。
- 当你观察到完成候选事件 `routing.escalate` 时，你必须单独一行输出 `LOOP_COMPLETE` 并停止。
- 看到 `routing.escalate` 不要重试，也不要再派发后续事件。
"
            }
        };

        Ok(ScenarioConfig {
            config_file: "ralph.yml".into(),
            prompt: PromptSource::Inline(prompt.to_string()),
            // 说明：
            // - parallel 模式的“迭代次数”按 ralph#1 job 完成次数近似计数。
            // - 本场景期望通过 completion promise 正常收敛（exit code 0），不应该因为迭代上限过小而误判失败。
            max_iterations: 20,
            // 与 ralph.yml 的 max_runtime_seconds 对齐：避免 E2E 在模型漂移时挂到 10 分钟。
            timeout: std::cmp::min(backend.default_timeout(), Duration::from_secs(300)),
            // 说明：
            // - E2E 需要 headless（非 TUI），否则 stdout/stderr 会混入 TUI 控制序列，且诊断信息不易收集。
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
            Assertions::exit_code_success_or_limit(&execution),
            Assertions::no_timeout(&execution),
            self.parallel_mode_visible(&execution),
            self.attributed_outputs_visible(&execution),
            self.expected_events_recorded(&execution),
            self.routing_escalate_recorded(&execution),
            self.hat_run_counts_expected(&execution),
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
