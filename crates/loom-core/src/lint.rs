use anyhow::anyhow;
use camino::Utf8Path;
use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{LintDiagnostic, LintReport, ProofEvidenceCitation, QuickfixRecord};
use serde_json::Value;
use uuid::Uuid;

pub async fn run(
    session_id: Uuid,
    adapter: &CliAdapter,
) -> anyhow::Result<(LintReport, loom_adapters::x07_cli::ExecutedBinding)> {
    let executed = adapter
        .execute_x07_json(
            "lint.report",
            "x07/lint",
            vec![
                "lint".to_string(),
                "--project".to_string(),
                "x07.json".to_string(),
            ],
            vec![".x07/studio/reports".to_string()],
            "Run x07 lint and project x07diag diagnostics.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let raw = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr,
            "exit_code": executed.execution.exit_code
        })
    });
    let report = LintReport {
        schema_version: "x07.studio.lint_report@0.1.0".to_string(),
        session_id,
        generated_at: executed.execution.finished_at.clone(),
        diagnostics: diagnostics_from_raw(&raw),
        raw,
    };
    Ok((report, executed))
}

pub async fn apply_quickfix(
    root: &Utf8Path,
    session_id: Uuid,
    diag_id: &str,
    adapter: &CliAdapter,
) -> anyhow::Result<(QuickfixRecord, loom_adapters::x07_cli::ExecutedBinding)> {
    let (lint_report, _) = run(session_id, adapter).await?;
    let diagnostic = lint_report
        .diagnostics
        .iter()
        .find(|diagnostic| diagnostic.id == diag_id)
        .cloned()
        .ok_or_else(|| {
            anyhow!("diagnostic `{diag_id}` was not present in the latest lint report")
        })?;
    let args = if diagnostic.file.trim().is_empty() {
        vec![
            "fix".to_string(),
            "--diagnostic".to_string(),
            diag_id.to_string(),
            "--write".to_string(),
        ]
    } else {
        validate_relative_path(&diagnostic.file)?;
        vec![
            "fix".to_string(),
            "--input".to_string(),
            diagnostic.file.clone(),
            "--write".to_string(),
        ]
    };
    let executed = adapter
        .execute_x07_json(
            "lint.quickfix",
            "x07/fix",
            args,
            vec![diagnostic.file.clone()],
            "Apply an x07 lint quickfix through x07 fix.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let patch_ast = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "diagnostic": diag_id,
            "exit_code": executed.execution.exit_code,
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr
        })
    });
    let record = QuickfixRecord {
        schema_version: "x07.studio.quickfix_record@0.1.0".to_string(),
        diagnostic_code: diagnostic.id,
        severity: diagnostic.severity,
        summary: diagnostic.summary,
        patch_ast,
        citations: vec![ProofEvidenceCitation {
            kind: "lint".to_string(),
            file: diagnostic.file,
            region: Some(format!("{}:{}", diagnostic.line, diagnostic.column)),
        }],
        before_snippet: None,
        after_snippet: None,
    };
    Ok((crate::quickfix::with_snippets(root, record), executed))
}

pub fn diagnostics_from_raw(raw: &Value) -> Vec<LintDiagnostic> {
    let mut values = Vec::new();
    collect_diagnostic_values(raw, &mut values);
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| diagnostic_from_value(index, value))
        .collect()
}

fn collect_diagnostic_values<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if looks_like_diagnostic(value) {
                out.push(value);
                return;
            }
            for (key, child) in map {
                if matches!(key.as_str(), "diagnostics" | "errors" | "warnings") {
                    if let Value::Array(items) = child {
                        for item in items {
                            collect_diagnostic_values(item, out);
                        }
                        continue;
                    }
                }
                collect_diagnostic_values(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_diagnostic_values(item, out);
            }
        }
        _ => {}
    }
}

fn looks_like_diagnostic(value: &Value) -> bool {
    first_string(value, &["id", "code", "diagnostic_code"]).is_some()
        && first_string(value, &["message", "summary", "title"]).is_some()
}

fn diagnostic_from_value(index: usize, value: &Value) -> LintDiagnostic {
    let id = first_string(value, &["id", "code", "diagnostic_code"])
        .unwrap_or_else(|| format!("X07-LINT-{:04}", index + 1));
    LintDiagnostic {
        id,
        severity: first_string(value, &["severity", "level"])
            .unwrap_or_else(|| "warning".to_string()),
        file: first_string(value, &["file", "path", "source"]).unwrap_or_default(),
        line: first_u32(value, &["line", "start_line"]).unwrap_or(1),
        column: first_u32(value, &["column", "col", "start_column"]).unwrap_or(1),
        summary: first_string(value, &["summary", "message", "title"])
            .unwrap_or_else(|| "x07 lint diagnostic".to_string()),
        fixable: first_bool(value, &["fixable"]).unwrap_or(false)
            || first_value(value, &["quickfix", "quickfixes", "fix", "fixes"]).is_some()
            || value.to_string().contains("Auto-fix"),
    }
}

fn validate_relative_path(path: &str) -> anyhow::Result<()> {
    let rel = Utf8Path::new(path);
    if path.contains('\0')
        || rel.is_absolute()
        || rel.components().any(|part| part.as_str() == "..")
    {
        return Err(anyhow!(
            "lint diagnostic path must stay inside the workspace"
        ));
    }
    Ok(())
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn first_bool(value: &Value, keys: &[&str]) -> Option<bool> {
    first_value(value, keys).and_then(Value::as_bool)
}

fn first_u32(value: &Value, keys: &[&str]) -> Option<u32> {
    first_value(value, keys)
        .and_then(Value::as_u64)
        .and_then(|num| u32::try_from(num).ok())
}

fn first_value<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
    match value {
        Value::Object(map) => {
            for key in keys {
                if let Some(value) = map.get(*key) {
                    return Some(value);
                }
            }
            map.values().find_map(|value| first_value(value, keys))
        }
        Value::Array(items) => items.iter().find_map(|value| first_value(value, keys)),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::diagnostics_from_raw;

    #[test]
    fn projects_fixture_lint_json() {
        let raw = serde_json::json!({
            "diagnostics": [{
                "code": "X07-LINT-0042",
                "severity": "error",
                "file": "src/main.x07.json",
                "line": 4,
                "column": 2,
                "message": "bad shape",
                "quickfix": {"kind": "json_patch"}
            }]
        });
        let diagnostics = diagnostics_from_raw(&raw);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].id, "X07-LINT-0042");
        assert!(diagnostics[0].fixable);
    }
}
