//! E2E 声明式覆盖率 gate test。
//!
//! 这个 test 是 `e2e-declarative-migration-plan` change 的 CI back-pressure。
//! 它通过 `ralph_e2e::all_scenarios()`(lib surface 的单一真相源)计算
//! 声明式 / 命令式场景比例,要求覆盖率 ≥ 90 %。
//!
//! 重要设计点:
//!
//! - `ImperativeExplicitKeep` 显式从分母中扣除,确保 90 % 阈值在设计上可达
//!   (参 audit-p5-p1.md §A.5, `ParallelExperimentalDevEngineExampleScenario`)。
//! - drift log 在失败时打印每个 kind 的具体 id,便于 PR 描述直接 copy-paste。
//! - 阈值是单一数字(`THRESHOLD`),不读取任何环境变量,不接受外部 override,
//!   避免「悄悄放宽门禁」。
//!
//! 预期状态:
//!
//! - 当前(2026-08-13): 39 declarative + 21 effective imperative = 60,
//!   覆盖率 ≈ 65.0 %,gate 故意 fail,以推动 Wave 2 的 22 次迁移 commit。
//! - 迁移完成后: ≥ 19 / 21 = 90.5 %,gate 转 green。

use ralph_e2e::{ScenarioKind, all_scenarios};

/// 声明式覆盖率下限。任何低于此值的提交必须在 CI 中被拒。
const THRESHOLD: f64 = 0.90;

#[test]
fn declarative_coverage_at_or_above_threshold() {
    // 调用 lib surface 的单一真相源(不是 main.rs 的旧私有函数)。
    let scenarios = all_scenarios();

    let mut declarative_count = 0usize;
    let mut imperative_count = 0usize;
    let mut explicit_keep_count = 0usize;
    let mut declarative_ids: Vec<&'static str> = Vec::new();
    let mut imperative_ids: Vec<&'static str> = Vec::new();
    let mut explicit_keep_ids: Vec<&'static str> = Vec::new();

    for (kind, id, _scenario) in &scenarios {
        match kind {
            ScenarioKind::Declarative => {
                declarative_count += 1;
                declarative_ids.push(id);
            }
            ScenarioKind::Imperative => {
                imperative_count += 1;
                imperative_ids.push(id);
            }
            ScenarioKind::ImperativeExplicitKeep => {
                // 显式 keep 不计入分母,但仍打印以便审查。
                explicit_keep_count += 1;
                explicit_keep_ids.push(id);
            }
        }
    }

    // 有效分母 = declarative + imperative (显式 keep 已扣除)。
    let total = declarative_count + imperative_count;
    let coverage = if total == 0 {
        1.0
    } else {
        declarative_count as f64 / total as f64
    };

    // 故意先打 drift log,失败时输出更可读。
    eprintln!(
        "declarative_coverage_gate drift log:\n  \
         Declarative:            {declarative_count} ({})\n  \
         Imperative:             {imperative_count} ({})\n  \
         ImperativeExplicitKeep: {explicit_keep_count} ({})\n  \
         Effective denominator:  {total}\n  \
         Coverage:               {:.2} %\n  \
         Threshold:              {:.2} %\n  \
         Pass / Fail:            {}",
        declarative_ids.join(", "),
        imperative_ids.join(", "),
        explicit_keep_ids.join(", "),
        coverage * 100.0,
        THRESHOLD * 100.0,
        if coverage >= THRESHOLD {
            "PASS"
        } else {
            "FAIL"
        },
    );

    assert!(
        coverage >= THRESHOLD,
        "declarative coverage {coverage:.4} below threshold {THRESHOLD:.4}; \
         migrate one imperative (see tasks.md §2) before re-running",
    );
}

/// 不变量测试:`ImperativeExplicitKeep` 永远只有 `parallel-experimental-dev-engine-example` 一项。
///
/// 这条 test 防止后续 contributor 偷偷把别的 imperative 也标成 explicit-keep,
/// 从而虚增覆盖率。一旦有人新增 explicit-keep,此 test 会失败,迫使 PR 作者
/// 解释为什么这个 imperative 也必须显式保留(而不是迁移到 declarative)。
#[test]
fn explicit_keep_is_exactly_parallel_experimental_dev_engine_example() {
    let scenarios = all_scenarios();
    let explicit_keep_ids: Vec<&'static str> = scenarios
        .iter()
        .filter_map(|(kind, id, _)| {
            if *kind == ScenarioKind::ImperativeExplicitKeep {
                Some(*id)
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        explicit_keep_ids,
        vec!["parallel-experimental-dev-engine-example"],
        "only parallel-experimental-dev-engine-example may be ImperativeExplicitKeep; \
         if you need another, justify in audit-p5-p1.md §A.5",
    );
}
