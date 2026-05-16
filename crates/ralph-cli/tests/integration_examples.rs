use std::path::PathBuf;

fn assert_prompt_file_example_is_self_contained(example_name: &str) {
    // ---------------------------------------------------------------------
    // 说明:
    // - 仓库内带 `prompt_file` 的 runnable example 都应该支持:
    //   `cd examples/<name> && ralph run`
    // - 因此这里统一锁住:
    //   - `prompt_file` 必须是同目录的 `PROMPT.md`
    //   - 目录里必须真的存在 `PROMPT.md`
    // ---------------------------------------------------------------------
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let example_dir = manifest_dir.join(format!("../../examples/{example_name}"));

    let config_path = example_dir.join("ralph.yml");
    let config_content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "failed to read example config {}: {e}",
            config_path.display()
        );
    });

    assert!(
        config_content.contains("prompt_file: \"PROMPT.md\""),
        "example config {} should contain `prompt_file: \"PROMPT.md\"` (self-contained)",
        config_path.display()
    );

    let root_relative = format!("prompt_file: \"examples/{example_name}/PROMPT.md\"");
    assert!(
        !config_content.contains(&root_relative),
        "example config {} should not hardcode repo-root relative prompt path",
        config_path.display()
    );

    let prompt_path = example_dir.join("PROMPT.md");
    assert!(
        prompt_path.exists(),
        "example prompt file should exist: {}",
        prompt_path.display()
    );
}

#[test]
fn test_example_parallel_experimental_dev_engine_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-experimental-dev-engine");
}

#[test]
fn test_example_parallel_experimental_dev_engine_uses_builtin_event_protocol() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path =
        manifest_dir.join("../../examples/parallel-experimental-dev-engine/ralph.yml");
    let config_content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "failed to read example config {}: {e}",
            config_path.display()
        );
    });

    for stale_tutorial in [
        "## 发事件格式",
        "发事件必须使用如下格式",
        "&lt;event topic=",
        "<event topic=",
        "此处写完整",
        "...payload...",
    ] {
        assert!(
            !config_content.contains(stale_tutorial),
            "example config {} should not duplicate generic event protocol tutorial `{stale_tutorial}`",
            config_path.display()
        );
    }

    for workflow_contract in [
        "experiment.result 的 payload 必须包含",
        "integration.applied 的 payload 必须包含",
        "experiment.complete 的 payload 可审计",
        "verification_evidence",
        "commit",
    ] {
        assert!(
            config_content.contains(workflow_contract),
            "example config {} should keep workflow payload contract `{workflow_contract}`",
            config_path.display()
        );
    }
}

#[test]
fn test_example_parallel_pr_review_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-pr-review");
}

#[test]
fn test_example_parallel_release_checklist_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-release-checklist");
}

#[test]
fn test_example_parallel_human_approval_gate_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-human-approval-gate");
}

#[test]
fn test_example_parallel_security_exception_review_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-security-exception-review");
}

#[test]
fn test_example_parallel_customer_renewal_desk_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-customer-renewal-desk");
}

#[test]
fn test_example_parallel_audit_evidence_pack_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-audit-evidence-pack");
}

#[test]
fn test_example_parallel_finance_close_control_room_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-finance-close-control-room");
}

#[test]
fn test_example_parallel_hiring_debrief_panel_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-hiring-debrief-panel");
}

#[test]
fn test_example_parallel_customer_onboarding_activation_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-customer-onboarding-activation");
}

#[test]
fn test_example_parallel_support_escalation_desk_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-support-escalation-desk");
}

#[test]
fn test_example_parallel_partner_launch_coordination_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-partner-launch-coordination");
}

#[test]
fn test_example_parallel_field_enablement_rollout_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-field-enablement-rollout");
}

#[test]
fn test_example_parallel_revops_quote_desk_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-revops-quote-desk");
}

#[test]
fn test_example_parallel_executive_business_review_prep_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-executive-business-review-prep");
}

#[test]
fn test_example_parallel_customer_advisory_board_prep_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-customer-advisory-board-prep");
}

#[test]
fn test_example_parallel_regional_operating_review_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-regional-operating-review");
}

#[test]
fn test_example_parallel_renewal_risk_calibration_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-renewal-risk-calibration");
}

#[test]
fn test_example_parallel_multi_region_pipeline_sync_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-multi-region-pipeline-sync");
}

#[test]
fn test_example_parallel_incident_response_war_room_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-incident-response-war-room");
}

#[test]
fn test_example_parallel_launch_readiness_command_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-launch-readiness-command");
}

#[test]
fn test_example_parallel_migration_rehearsal_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-migration-rehearsal");
}

#[test]
fn test_example_parallel_postmortem_action_board_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-postmortem-action-board");
}

#[test]
fn test_example_parallel_proposal_assembly_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-proposal-assembly");
}

#[test]
fn test_example_parallel_vendor_security_procurement_prompt_file_self_contained() {
    assert_prompt_file_example_is_self_contained("parallel-vendor-security-procurement");
}
