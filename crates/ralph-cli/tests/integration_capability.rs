use anyhow::Result;
use ralph_core::{EvidenceArtifactKind, EvidenceIndexReader, EvidenceLookup, EvidenceStatus};
use serde_json::Value;
use std::fs;
use std::process::Command;
use tempfile::TempDir;

#[test]
fn tools_capability_list_exposes_lightweight_metadata() -> Result<()> {
    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args(["tools", "capability", "list", "--json"])
        .output()?;

    assert!(
        output.status.success(),
        "capability list should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let values: Vec<Value> = serde_json::from_slice(&output.stdout)?;
    assert!(
        values
            .iter()
            .any(|value| value["kind"] == "workflow_capability")
    );
    assert!(values.iter().any(|value| value["kind"] == "hat_capability"));
    assert!(values.iter().all(|value| value.get("summary").is_some()));
    assert!(
        values
            .iter()
            .all(|value| value.get("input_contract").is_some())
    );

    Ok(())
}

#[test]
fn tools_capability_invoke_writes_isolated_artifacts_and_events() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let workspace = temp_dir.path();
    fs::write(workspace.join("ralph.yml"), "parent topology sentinel")?;

    let output = Command::new(env!("CARGO_BIN_EXE_ralph"))
        .args([
            "tools",
            "capability",
            "invoke",
            "--id",
            "hat:focused-reviewer",
            "--input",
            "review this patch",
            "--json",
        ])
        .current_dir(workspace)
        .output()?;

    assert!(
        output.status.success(),
        "capability invoke should succeed.\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let report: Value = serde_json::from_slice(&output.stdout)?;
    let invocation_id = report["invocation"]["invocation_id"]
        .as_str()
        .expect("invocation id");
    assert_eq!(
        report["invocation"]["capability"]["invocation_mode"],
        "isolated_micro_run"
    );
    assert_eq!(report["invocation"]["parent_topology_unchanged"], true);
    assert_eq!(report["result"]["parent_topology_unchanged"], true);

    let invocation_dir = workspace
        .join(".ralph/capability-invocations")
        .join(invocation_id);
    assert!(invocation_dir.join("invoke.json").exists());
    assert!(invocation_dir.join("result.json").exists());
    assert!(invocation_dir.join("resolved-config.yml").exists());

    let events = fs::read_to_string(workspace.join(".ralph/events.jsonl"))?;
    assert!(events.contains("capability.invoke"));
    assert!(events.contains("capability.result"));

    // Phase 3: capability invocation 不能只留下离散文件。
    // evidence index 必须能用 invocation id 把 child/micro-run 证据串起来。
    let evidence_path = workspace.join(".ralph/evidence-index.jsonl");
    let evidence_lookup = EvidenceIndexReader::new(&evidence_path)
        .find_by_correlation(invocation_id)
        .map_err(anyhow::Error::from)?;
    assert!(matches!(evidence_lookup, EvidenceLookup::Entries(_)));
    let evidence_entries = evidence_lookup.entries();

    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityInvokeJson
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("invoke.json")
        }),
        "invocation id should find invoke.json evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::CapabilityResultJson
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("result.json")
        }),
        "invocation id should find result.json evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::ResolvedConfig
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with("resolved-config.yml")
        }),
        "invocation id should find resolved config evidence: {evidence_entries:#?}"
    );
    assert!(
        evidence_entries.iter().any(|entry| {
            entry.artifact_kind == EvidenceArtifactKind::EventLogJsonl
                && entry.status == EvidenceStatus::Success
                && entry.artifact_path.ends_with(".ralph/events.jsonl")
        }),
        "invocation id should find event log evidence: {evidence_entries:#?}"
    );
    assert_eq!(
        fs::read_to_string(workspace.join("ralph.yml"))?,
        "parent topology sentinel"
    );

    Ok(())
}
