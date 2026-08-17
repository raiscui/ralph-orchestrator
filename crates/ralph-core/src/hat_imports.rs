//! Hat imports in preflight — local file-based hat field merging.
//!
//! 说明：
//! - Phase 1 of origin issue #209: allow a `hats:` entry to declare an
//!   `imports:` key pointing to a local YAML file whose top-level
//!   `hats:` entries are merged (field-level) into the importing source.
//! - `HatConfig` itself is unchanged; this lives entirely in the
//!   `serde_yaml::Mapping` space before `serde_yaml::from_str` parses
//!   into `RalphConfig`.
//! - Only local file sources may use imports. Builtin/remote sources
//!   have `imports:` rejected at load time.

use serde_yaml::{Mapping, Value};
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Reason a non-local source cannot use `imports:`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnsupportedImportSource {
    /// Builtin hat collections (compiled into the binary).
    Builtin,
    /// Remote (URL-fetched) hat configurations.
    Remote,
}

impl UnsupportedImportSource {
    fn label(self) -> &'static str {
        match self {
            UnsupportedImportSource::Builtin => "builtin",
            UnsupportedImportSource::Remote => "remote",
        }
    }
}

/// Errors produced by hat import resolution.
///
/// 说明：使用单一 `message: String` 字段避免 thiserror v2 对多 String 字段的
/// `as_dyn_error` 兼容性陷阱。
#[derive(Debug, Error)]
pub enum HatImportError {
    /// Hat-specific error. The inner string contains the full formatted message
    /// including hat name, source label, and reason.
    #[error("{message}")]
    Hat { message: String },

    /// Source-policy error (non-local source declares `imports:`).
    #[error("{message}")]
    UnsupportedSource { message: String },
}

impl HatImportError {
    /// Build a hat-level error from its components.
    pub fn hat(hat: &str, source: &str, reason: impl AsRef<str>) -> Self {
        HatImportError::Hat {
            message: format!(
                "hat '{}' in '{}': {}",
                hat,
                source,
                reason.as_ref()
            ),
        }
    }

    /// Build a source-policy error.
    pub fn unsupported_source(
        reason: UnsupportedImportSource,
        source: &str,
        hat: &str,
    ) -> Self {
        HatImportError::UnsupportedSource {
            message: format!(
                "hat imports not allowed from {} source '{}': hat '{}' declares 'imports:'",
                reason.label(),
                source,
                hat
            ),
        }
    }
}

/// Convenience alias for the module's public result type.
pub type Result<T> = std::result::Result<T, HatImportError>;

/// Recursively resolve `imports:` keys inside the `hats:` block of a parsed
/// `Mapping`. Mutates `mapping` in place.
///
/// 说明：
/// - 仅处理 `mapping["hats"]`（如果存在）。
/// - 每个 hat entry 可以包含 `imports: <relative-path>`，值必须是字符串。
/// - 解析后的基帽子会按 field-level merge 到本地覆盖之上：导入字段作为底，
///   本地字段覆盖。
/// - 仅处理一层导入：导入文件内的 hats 如果也含 `imports:` 直接报错。
pub fn resolve_hat_imports_in_mapping(
    mapping: &mut Mapping,
    base_dir: &Path,
    source_label: &str,
) -> Result<()> {
    let Some(hats_value) = mapping.get_mut(Value::String("hats".to_string())) else {
        return Ok(());
    };

    let Value::Mapping(hats_map) = hats_value else {
        return Ok(());
    };

    let mut replacements: Vec<(Value, Value)> = Vec::new();

    for (hat_key, hat_value) in hats_map.iter() {
        let Value::Mapping(hat_map) = hat_value else {
            continue;
        };

        let hat_label = hat_key_label(hat_key);

        let imports_value = hat_map
            .get(Value::String("imports".to_string()))
            .cloned();

        let Some(imports_value) = imports_value else {
            continue;
        };

        let imports_path_str = match &imports_value {
            Value::String(s) => s.clone(),
            other => {
                return Err(HatImportError::hat(
                    &hat_label,
                    source_label,
                    format!("'imports' must be a string path, found {}", value_kind(other)),
                ));
            }
        };

        let imports_path = resolve_import_path(base_dir, &imports_path_str);
        let imports_label = imports_path.display().to_string();

        let imports_content = std::fs::read_to_string(&imports_path).map_err(|e| {
            HatImportError::hat(
                &hat_label,
                source_label,
                format!("failed to read import '{}': {}", imports_label, e),
            )
        })?;

        let imports_value_mapping: Value = serde_yaml::from_str(&imports_content).map_err(|e| {
            HatImportError::hat(
                &hat_label,
                source_label,
                format!("invalid YAML in import '{}': {}", imports_label, e),
            )
        })?;

        let imports_hats_map = match &imports_value_mapping {
            Value::Mapping(m) => m,
            _ => {
                return Err(HatImportError::hat(
                    &hat_label,
                    source_label,
                    format!(
                        "import '{}' must be a YAML mapping with a top-level 'hats' block",
                        imports_label
                    ),
                ));
            }
        };

        let imports_hats_value = imports_hats_map
            .get(Value::String("hats".to_string()))
            .cloned();

        let imports_hats_mapping = match imports_hats_value {
            Some(Value::Mapping(m)) => m,
            _ => {
                return Err(HatImportError::hat(
                    &hat_label,
                    source_label,
                    format!("import '{}' has no 'hats:' mapping", imports_label),
                ));
            }
        };

        let imported_hat = imports_hats_mapping.get(hat_key).cloned();

        let Some(Value::Mapping(imported_hat)) = imported_hat else {
            return Err(HatImportError::hat(
                &hat_label,
                source_label,
                format!(
                    "import '{}' does not contain hat '{}'",
                    imports_label, hat_label
                ),
            ));
        };

        if imported_hat
            .get(Value::String("imports".to_string()))
            .is_some()
        {
            return Err(HatImportError::hat(
                &hat_label,
                source_label,
                format!(
                    "transitive imports not allowed: imported hat in '{}' also declares 'imports:'",
                    imports_label
                ),
            ));
        }

        if imported_hat
            .get(Value::String("events".to_string()))
            .is_some()
        {
            return Err(HatImportError::hat(
                &hat_label,
                source_label,
                format!("imported hat in '{}' must not contain 'events:' field", imports_label),
            ));
        }

        let merged = merge_imported_hat(imported_hat, hat_map);

        replacements.push((hat_key.clone(), Value::Mapping(merged)));
    }

    for (k, v) in replacements {
        hats_map.insert(k, v);
    }

    Ok(())
}

/// Reject any `imports:` key found in a non-local source (builtin/remote).
pub fn reject_hat_imports_in_mapping(
    mapping: &Mapping,
    source_label: &str,
    reason: UnsupportedImportSource,
) -> Result<()> {
    let Some(Value::Mapping(hats_map)) = mapping.get(Value::String("hats".to_string())) else {
        return Ok(());
    };

    for (hat_key, hat_value) in hats_map.iter() {
        let hat_label = hat_key_label(hat_key);
        let Value::Mapping(hat_map) = hat_value else {
            continue;
        };
        if hat_map.get(Value::String("imports".to_string())).is_some() {
            return Err(HatImportError::unsupported_source(
                reason,
                source_label,
                &hat_label,
            ));
        }
    }

    Ok(())
}

/// Resolve an import path string against a base directory.
fn resolve_import_path(base_dir: &Path, import_str: &str) -> PathBuf {
    let p = PathBuf::from(import_str);
    if p.is_absolute() {
        p
    } else {
        base_dir.join(p)
    }
}

/// Merge an imported hat mapping with local override mapping.
///
/// 说明：
/// - 导入字段作为底，本地字段覆盖（同 key 替换）。
/// - 删除本地 `imports:` 字段（已解析完成）。
fn merge_imported_hat(mut imported: Mapping, local_overrides: &Mapping) -> Mapping {
    for (k, v) in local_overrides.iter() {
        if k == &Value::String("imports".to_string()) {
            continue;
        }
        imported.insert(k.clone(), v.clone());
    }
    imported
}

/// Render a YAML value as a string for error messages.
fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Sequence(_) => "sequence",
        Value::Mapping(_) => "mapping",
        Value::Tagged(_) => "tagged",
    }
}

/// Render a hat key (string) for error messages.
fn hat_key_label(key: &Value) -> String {
    match key {
        Value::String(s) => s.clone(),
        other => format!("{:?}", other),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(yaml: &str) -> Mapping {
        let v: Value = serde_yaml::from_str(yaml).expect("test yaml must parse");
        match v {
            Value::Mapping(m) => m,
            _ => panic!("expected top-level mapping"),
        }
    }

    #[test]
    fn resolve_no_hats_block_is_noop() {
        let mut m = parse("cli:\n  backend: codex\n");
        resolve_hat_imports_in_mapping(&mut m, Path::new("."), "test").unwrap();
        assert!(m.get(Value::String("hats".to_string())).is_none());
    }

    #[test]
    fn resolve_hats_without_imports_is_noop() {
        let mut m = parse(
            "hats:\n  writer:\n    description: local\n    publishes: [build.done]\n",
        );
        resolve_hat_imports_in_mapping(&mut m, Path::new("."), "test").unwrap();
        let hats = m.get(Value::String("hats".to_string())).unwrap();
        let hats_map = match hats {
            Value::Mapping(m) => m,
            _ => panic!(),
        };
        let writer = hats_map
            .get(Value::String("writer".to_string()))
            .unwrap();
        let writer_map = match writer {
            Value::Mapping(m) => m,
            _ => panic!(),
        };
        assert!(writer_map
            .get(Value::String("imports".to_string()))
            .is_none());
    }

    #[test]
    fn reject_non_string_imports_returns_error() {
        let mut m = parse(
            "hats:\n  writer:\n    imports:\n      - ../shared/base.yml\n",
        );
        let err = resolve_hat_imports_in_mapping(&mut m, Path::new("."), "test")
            .expect_err("must reject non-string imports");
        match err {
            HatImportError::Hat { message } => {
                assert!(message.contains("writer"), "missing hat name: {}", message);
                assert!(message.contains("must be a string"), "wrong reason: {}", message);
            }
            _ => panic!("expected Hat error"),
        }
    }

    #[test]
    fn reject_missing_import_file_returns_error() {
        let tmp = tempdir();
        let mut m = parse("hats:\n  writer:\n    imports: missing.yml\n");
        let err = resolve_hat_imports_in_mapping(&mut m, &tmp, "test")
            .expect_err("must reject missing file");
        match err {
            HatImportError::Hat { message } => {
                assert!(message.contains("writer"));
                assert!(message.contains("failed to read"));
            }
            _ => panic!("expected Hat error"),
        }
    }

    #[test]
    fn reject_imported_events_field_returns_error() {
        let tmp = tempdir();
        std::fs::write(
            tmp.join("base.yml"),
            "hats:\n  writer:\n    description: base\n    events:\n      build.done: ok\n",
        )
        .unwrap();
        let mut m = parse("hats:\n  writer:\n    imports: base.yml\n");
        let err = resolve_hat_imports_in_mapping(&mut m, &tmp, "test")
            .expect_err("must reject events in imported hat");
        match err {
            HatImportError::Hat { message } => {
                assert!(message.contains("events"), "got: {}", message);
            }
            _ => panic!("expected Hat error"),
        }
    }

    #[test]
    fn reject_transitive_imports_returns_error() {
        let tmp = tempdir();
        std::fs::write(
            tmp.join("a.yml"),
            "hats:\n  writer:\n    imports: b.yml\n",
        )
        .unwrap();
        std::fs::write(
            tmp.join("b.yml"),
            "hats:\n  writer:\n    description: b\n",
        )
        .unwrap();
        let mut m = parse("hats:\n  writer:\n    imports: a.yml\n");
        let err = resolve_hat_imports_in_mapping(&mut m, &tmp, "test")
            .expect_err("must reject transitive imports");
        match err {
            HatImportError::Hat { message } => {
                assert!(message.contains("transitive"), "got: {}", message);
            }
            _ => panic!("expected Hat error"),
        }
    }

    #[test]
    fn resolve_successful_merge_overrides_publishes() {
        let tmp = tempdir();
        std::fs::write(
            tmp.join("base.yml"),
            "hats:\n  writer:\n    description: base\n    publishes: [base.done]\n    triggers: [build.task]\n    instructions: base instr\n",
        )
        .unwrap();
        let mut m = parse(
            "hats:\n  writer:\n    imports: base.yml\n    publishes: [local.done]\n",
        );
        resolve_hat_imports_in_mapping(&mut m, &tmp, "test").unwrap();

        let hats = m.get(Value::String("hats".to_string())).unwrap();
        let hats_map = match hats {
            Value::Mapping(m) => m,
            _ => panic!(),
        };
        let writer = hats_map
            .get(Value::String("writer".to_string()))
            .unwrap();
        let writer_map = match writer {
            Value::Mapping(m) => m,
            _ => panic!(),
        };

        assert!(writer_map
            .get(Value::String("imports".to_string()))
            .is_none());
        let publishes = writer_map
            .get(Value::String("publishes".to_string()))
            .unwrap();
        if let Value::Sequence(seq) = publishes {
            assert_eq!(seq.len(), 1);
            assert_eq!(seq[0], Value::String("local.done".to_string()));
        } else {
            panic!("expected sequence");
        }
        let triggers = writer_map
            .get(Value::String("triggers".to_string()))
            .unwrap();
        if let Value::Sequence(seq) = triggers {
            assert_eq!(seq[0], Value::String("build.task".to_string()));
        } else {
            panic!("expected sequence");
        }
        let description = writer_map
            .get(Value::String("description".to_string()))
            .unwrap();
        assert_eq!(description, &Value::String("base".to_string()));
    }

    #[test]
    fn reject_builtin_source_with_imports_returns_error() {
        let m = parse(
            "hats:\n  writer:\n    imports: ../shared/base.yml\n    description: local\n",
        );
        let err = reject_hat_imports_in_mapping(&m, "builtin:core", UnsupportedImportSource::Builtin)
            .expect_err("must reject builtin source");
        match err {
            HatImportError::UnsupportedSource { message } => {
                assert!(message.contains("builtin"));
                assert!(message.contains("writer"));
            }
            _ => panic!("expected UnsupportedSource"),
        }
    }

    #[test]
    fn reject_remote_source_with_imports_returns_error() {
        let m = parse(
            "hats:\n  writer:\n    imports: ../shared/base.yml\n    description: local\n",
        );
        let err = reject_hat_imports_in_mapping(
            &m,
            "https://example.com/hats.yml",
            UnsupportedImportSource::Remote,
        )
        .expect_err("must reject remote source");
        match err {
            HatImportError::UnsupportedSource { message } => {
                assert!(message.contains("remote"));
            }
            _ => panic!("expected UnsupportedSource"),
        }
    }

    fn tempdir() -> PathBuf {
        let base = std::env::temp_dir().join(format!(
            "ralph-hat-imports-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&base).unwrap();
        base
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;
    use crate::config::RalphConfig;

    fn write_files() -> std::path::PathBuf {
        let base = std::env::temp_dir().join(format!(
            "ralph-hat-imports-integ-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(base.join("shared")).unwrap();
        std::fs::write(
            base.join("shared").join("base-hat.yml"),
            "hats:\n  writer:\n    name: BaseWriter\n    description: Base writer\n    triggers: [build.task]\n    publishes: [base.done]\n",
        )
        .unwrap();
        std::fs::write(
            base.join("ralph.yml"),
            "hats:\n  writer:\n    name: Writer\n    imports: shared/base-hat.yml\n    publishes: [local.done]\n",
        )
        .unwrap();
        base
    }

    #[test]
    fn from_file_resolves_imports_and_overrides_fields() {
        let dir = write_files();
        let config = RalphConfig::from_file(dir.join("ralph.yml")).expect("must load");
        let writer = config.hats.get("writer").expect("writer hat exists");
        assert_eq!(writer.description.as_deref(), Some("Base writer"));
        assert_eq!(writer.publishes, vec!["local.done".to_string()]);
        assert_eq!(writer.triggers, vec!["build.task".to_string()]);
    }

    #[test]
    fn from_file_no_imports_works() {
        let dir = write_files();
        std::fs::write(
            dir.join("standalone.yaml"),
            "hats:\n  reader:\n    name: Reader\n    description: standalone\n",
        )
        .unwrap();
        let config =
            RalphConfig::from_file(dir.join("standalone.yaml")).expect("must load");
        assert_eq!(
            config.hats.get("reader").unwrap().description.as_deref(),
            Some("standalone")
        );
    }

    #[test]
    fn from_file_missing_import_returns_error() {
        let dir = write_files();
        std::fs::write(
            dir.join("bad.yml"),
            "hats:\n  writer:\n    name: Writer\n    imports: nope.yml\n",
        )
        .unwrap();
        let err = RalphConfig::from_file(dir.join("bad.yml"))
            .expect_err("must fail on missing import");
        let msg = err.to_string();
        assert!(msg.contains("writer"), "got: {}", msg);
        assert!(msg.contains("failed to read"), "got: {}", msg);
    }
}
