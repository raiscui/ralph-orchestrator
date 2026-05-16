//! Answer-return evidence inspect UX。
//!
//! 设计边界:
//! - 这里只提供 `reply.hat.message` answer-return evidence 的最小 lookup surface。
//! - 单一真相源仍然是 `.ralph/events.jsonl` 等 durable artifact。
//! - `.ralph/evidence-index.jsonl` 只负责把 request id / answer id 指回这些 artifact。

use anyhow::{Context, Result, bail};
use clap::{Parser, Subcommand};
use ralph_core::{EvidenceIndexEntry, EvidenceIndexReader, EvidenceIndexWriter, EvidenceLookup};
use serde::Serialize;
use std::path::{Path, PathBuf};

/// `ralph tools answer` 参数。
#[derive(Parser, Debug)]
pub struct AnswerArgs {
    #[command(subcommand)]
    pub command: AnswerCommands,
}

/// Answer evidence tool 子命令。
#[derive(Subcommand, Debug)]
pub enum AnswerCommands {
    /// Inspect answer-return evidence by request id or answer id.
    Inspect(AnswerInspectArgs),
}

/// Answer evidence inspect 参数。
#[derive(Parser, Debug)]
pub struct AnswerInspectArgs {
    /// Request id or answer id correlation key.
    pub correlation_id: String,

    /// Workspace root, defaults to current directory.
    #[arg(long)]
    pub workspace: Option<PathBuf>,

    /// Emit machine-readable JSON.
    #[arg(long)]
    pub json: bool,
}

/// 执行 answer tool。
pub fn execute(args: AnswerArgs, _use_colors: bool) -> Result<()> {
    match args.command {
        AnswerCommands::Inspect(args) => inspect_answer_evidence(args),
    }
}

fn inspect_answer_evidence(args: AnswerInspectArgs) -> Result<()> {
    let workspace = args
        .workspace
        .unwrap_or(std::env::current_dir().context("Failed to resolve current directory")?);
    let report = inspect_answer_evidence_report(&workspace, &args.correlation_id)?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    print_answer_inspect_report(&report);
    Ok(())
}

fn inspect_answer_evidence_report(
    workspace: &Path,
    correlation_id: &str,
) -> Result<AnswerInspectReport> {
    let index_path = workspace.join(EvidenceIndexWriter::DEFAULT_PATH);
    let lookup = EvidenceIndexReader::new(&index_path)
        .find_by_correlation(correlation_id)
        .with_context(|| format!("Failed to read evidence index at {}", index_path.display()))?;

    let status = match lookup {
        EvidenceLookup::Entries(entries) => AnswerInspectStatus::Entries { entries },
        EvidenceLookup::Missing(entries) => AnswerInspectStatus::Missing { entries },
        EvidenceLookup::NoEntry => bail!(
            "No answer evidence entries for correlation id `{}` in {}",
            correlation_id,
            index_path.display()
        ),
    };

    Ok(AnswerInspectReport {
        correlation_id: correlation_id.to_string(),
        index_path: index_path.display().to_string(),
        status,
    })
}

fn print_answer_inspect_report(report: &AnswerInspectReport) {
    println!("Correlation: {}", report.correlation_id);
    println!("Evidence index: {}", report.index_path);
    println!("Status: {}", report.status.as_str());

    for entry in report.entries() {
        let artifact_kind = serde_json::to_value(entry.artifact_kind)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{:?}", entry.artifact_kind));
        let status = serde_json::to_value(entry.status)
            .ok()
            .and_then(|value| value.as_str().map(ToOwned::to_owned))
            .unwrap_or_else(|| format!("{:?}", entry.status));

        println!("- {}", artifact_kind);
        println!("  Path: {}", entry.artifact_path);
        println!("  Producer: {}", entry.producer);
        println!("  Status: {}", status);
    }
}

/// `inspect` 的 JSON 契约。
#[derive(Debug, Clone, Serialize)]
struct AnswerInspectReport {
    correlation_id: String,
    index_path: String,
    #[serde(flatten)]
    status: AnswerInspectStatus,
}

impl AnswerInspectReport {
    fn entries(&self) -> &[EvidenceIndexEntry] {
        match &self.status {
            AnswerInspectStatus::Entries { entries } | AnswerInspectStatus::Missing { entries } => {
                entries
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
enum AnswerInspectStatus {
    Entries { entries: Vec<EvidenceIndexEntry> },
    Missing { entries: Vec<EvidenceIndexEntry> },
}

impl AnswerInspectStatus {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Entries { .. } => "entries",
            Self::Missing { .. } => "missing",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ralph_core::{EvidenceArtifactKind, EvidenceIndexWriter, EvidenceStatus};
    use tempfile::TempDir;

    #[test]
    fn inspect_report_preserves_explicit_missing_answer_markers() {
        let temp = TempDir::new().unwrap();
        let index_path = temp.path().join(EvidenceIndexWriter::DEFAULT_PATH);
        let mut writer = EvidenceIndexWriter::new(&index_path);
        writer
            .record(&EvidenceIndexEntry::missing(
                "req-missing-1",
                EvidenceArtifactKind::ReplyEvent,
                ".ralph/events.jsonl",
                "answer",
            ))
            .unwrap();

        let report = inspect_answer_evidence_report(temp.path(), "req-missing-1").unwrap();

        assert!(matches!(report.status, AnswerInspectStatus::Missing { .. }));
        assert_eq!(report.entries()[0].status, EvidenceStatus::Missing);
    }
}
