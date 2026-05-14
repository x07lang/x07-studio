use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use loom_adapters::command_runner::{now_string, CommandExecution};
use loom_adapters::x07_cli::{CliAdapter, ExecutedBinding, RenderedCommand, X07JsonOptions};
use loom_types::api::{LintDiagnostic, LintReport, ProofEvidenceCitation, QuickfixRecord};
use serde_json::Value;
use uuid::Uuid;

pub async fn run(
    root: &Utf8Path,
    session_id: Uuid,
    adapter: &CliAdapter,
) -> anyhow::Result<(LintReport, ExecutedBinding)> {
    let inputs = lint_inputs(root);
    let mut executions = Vec::new();
    for input in &inputs {
        executions.push(
            adapter
                .execute_x07_json(
                    "lint.report",
                    "x07/lint",
                    vec!["lint".to_string(), "--input".to_string(), input.clone()],
                    vec![".x07/studio/reports".to_string()],
                    "Run x07 lint and project x07diag diagnostics.",
                    X07JsonOptions::report_file(Some(60)),
                )
                .await?,
        );
    }
    let mut diagnostics = Vec::new();
    for executed in &executions {
        let raw = executed.report_json.clone().unwrap_or_else(|| {
            serde_json::json!({
                "stdout": executed.execution.stdout,
                "stderr": executed.execution.stderr,
                "exit_code": executed.execution.exit_code
            })
        });
        diagnostics.extend(diagnostics_from_raw(&raw));
    }
    let raw = aggregate_raw(&inputs, &executions, &diagnostics);
    let executed = aggregate_execution(root, &inputs, executions, &raw);
    let report = LintReport {
        schema_version: "x07.studio.lint_report@0.1.0".to_string(),
        session_id,
        generated_at: executed.execution.finished_at.clone(),
        diagnostics,
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
    let (lint_report, _) = run(root, session_id, adapter).await?;
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

fn lint_inputs(root: &Utf8Path) -> Vec<String> {
    let mut inputs = Vec::new();
    for dir in ["src", "tests"] {
        collect_lint_inputs(root, root.join(dir).as_path(), &mut inputs);
    }
    inputs.sort();
    inputs.dedup();
    inputs.truncate(512);
    inputs
}

fn collect_lint_inputs(root: &Utf8Path, dir: &Utf8Path, out: &mut Vec<String>) {
    let Ok(entries) = std::fs::read_dir(dir.as_std_path()) else {
        return;
    };
    for entry in entries.flatten() {
        let path = Utf8PathBuf::from_path_buf(entry.path()).ok();
        let Some(path) = path else {
            continue;
        };
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_dir() {
            collect_lint_inputs(root, path.as_path(), out);
        } else if file_type.is_file()
            && path
                .file_name()
                .map(|name| name.ends_with(".x07.json"))
                .unwrap_or(false)
        {
            if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_string());
            }
        }
    }
}

fn aggregate_raw(
    inputs: &[String],
    executions: &[ExecutedBinding],
    diagnostics: &[LintDiagnostic],
) -> Value {
    serde_json::json!({
        "schema_version": "x07.studio.lint_batch@0.1.0",
        "ok": executions.iter().all(|execution| execution.execution.exit_code == Some(0)),
        "inputs": inputs,
        "diagnostic_count": diagnostics.len(),
        "reports": executions
            .iter()
            .map(|execution| serde_json::json!({
                "input": execution
                    .rendered
                    .args
                    .windows(2)
                    .find(|items| items[0] == "--input")
                    .map(|items| items[1].clone()),
                "exit_code": execution.execution.exit_code,
                "report_path": execution.report_path.as_ref().map(|path| path.to_string()),
            }))
            .collect::<Vec<_>>(),
    })
}

fn aggregate_execution(
    root: &Utf8Path,
    inputs: &[String],
    executions: Vec<ExecutedBinding>,
    raw: &Value,
) -> ExecutedBinding {
    let mut args = vec!["lint".to_string()];
    for input in inputs {
        args.push("--input".to_string());
        args.push(input.clone());
    }
    let now = now_string();
    let started_at = executions
        .first()
        .map(|execution| execution.execution.started_at.clone())
        .unwrap_or_else(|| now.clone());
    let finished_at = executions
        .last()
        .map(|execution| execution.execution.finished_at.clone())
        .unwrap_or(now);
    let exit_code = executions
        .iter()
        .find_map(|execution| {
            (execution.execution.exit_code != Some(0)).then_some(execution.execution.exit_code)
        })
        .unwrap_or(Some(0));
    let stdout = executions
        .iter()
        .filter(|execution| !execution.execution.stdout.trim().is_empty())
        .map(|execution| execution.execution.stdout.clone())
        .collect::<Vec<_>>()
        .join("\n");
    let stderr = executions
        .iter()
        .filter(|execution| !execution.execution.stderr.trim().is_empty())
        .map(|execution| execution.execution.stderr.clone())
        .collect::<Vec<_>>()
        .join("\n");
    ExecutedBinding {
        rendered: RenderedCommand {
            id: "lint.report".to_string(),
            category: "x07/lint".to_string(),
            program: "x07".to_string(),
            args: args.clone(),
            artifacts: vec![".x07/studio/reports".to_string()],
            notes: "Run x07 lint and project x07diag diagnostics.".to_string(),
        },
        execution: CommandExecution {
            program: "x07".to_string(),
            args,
            cwd: root.to_owned(),
            started_at,
            finished_at,
            exit_code,
            stdout,
            stderr,
            stdout_json: None,
            stderr_json: None,
        },
        report_json: Some(raw.clone()),
        report_path: None,
    }
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
    use uuid::Uuid;

    use super::{diagnostics_from_raw, lint_inputs};

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

    #[test]
    fn lint_inputs_find_source_ast_files_deterministically() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-lint-inputs-{}", Uuid::new_v4()));
        std::fs::create_dir_all(root.join("src/b")).expect("mkdir src");
        std::fs::create_dir_all(root.join("tests/a")).expect("mkdir tests");
        std::fs::write(root.join("src/z.x07.json"), "{}").expect("src z");
        std::fs::write(root.join("src/b/a.x07.json"), "{}").expect("src b");
        std::fs::write(root.join("tests/a/test.x07.json"), "{}").expect("test ast");
        std::fs::write(root.join("src/ignore.json"), "{}").expect("ignore");

        let inputs = lint_inputs(root.as_path());

        assert_eq!(
            inputs,
            vec![
                "src/b/a.x07.json".to_string(),
                "src/z.x07.json".to_string(),
                "tests/a/test.x07.json".to_string(),
            ]
        );
        std::fs::remove_dir_all(root).ok();
    }
}
