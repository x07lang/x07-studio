use camino::Utf8Path;
use loom_types::api::{ProofEvidenceCitation, QuickfixRecord};
use serde_json::Value;

pub fn for_incident(root: &Utf8Path, incident_id: &str) -> QuickfixRecord {
    let incident = incident_json(root, incident_id).unwrap_or(Value::Null);
    let diagnostic_code = first_string(&incident, &["diagnostic_code", "code", "kind"])
        .unwrap_or_else(|| "X07-INCIDENT".to_string());
    let severity = first_string(&incident, &["severity"]).unwrap_or_else(|| "warning".to_string());
    let summary = first_string(&incident, &["summary", "message", "error"]).unwrap_or_else(|| {
        format!("Incident `{incident_id}` can be converted into a bounded x07 quickfix review.")
    });
    let patch_ast = first_value(&incident, &["patch", "patch_ast", "patchset"]).cloned()
        .or_else(|| latest_patchset(root))
        .unwrap_or_else(|| {
            serde_json::json!({
                "schema_version": "x07.patchset@0.1.0",
                "operations": [],
                "note": "No deterministic patch was emitted yet; run incident repair to materialize one."
            })
        });
    QuickfixRecord {
        schema_version: "x07.studio.quickfix_record@0.1.0".to_string(),
        diagnostic_code,
        severity,
        summary,
        patch_ast,
        citations: vec![ProofEvidenceCitation {
            kind: "incident".to_string(),
            file: format!(".x07-wasm/incidents/{incident_id}"),
            region: Some("run.report.json".to_string()),
        }],
        before_snippet: None,
        after_snippet: None,
    }
    .pipe(|record| with_snippets(root, record))
}

pub fn with_snippets(root: &Utf8Path, mut record: QuickfixRecord) -> QuickfixRecord {
    let target = patch_target(&record.patch_ast).or_else(|| {
        record
            .citations
            .first()
            .map(|citation| citation.file.clone())
    });
    let Some(target) = target else {
        return record;
    };
    let Ok(target_path) = safe_relative_path(root, &target) else {
        return record;
    };
    let Ok(before) = std::fs::read_to_string(&target_path) else {
        return record;
    };
    record.before_snippet = Some(clip(&before, 12_000));
    if let Some(after) = after_from_patch(&before, &record.patch_ast, &target) {
        record.after_snippet = Some(clip(&after, 12_000));
    }
    record
}

fn incident_json(root: &Utf8Path, incident_id: &str) -> Option<Value> {
    let candidates = [
        root.join(format!(".x07-wasm/incidents/{incident_id}/run.report.json")),
        root.join(format!(
            "target/xtal/violations/{incident_id}/run.report.json"
        )),
        root.join(format!("target/xtal/ingest/{incident_id}.json")),
    ];
    candidates
        .iter()
        .find_map(|path| std::fs::read_to_string(path).ok())
        .and_then(|raw| serde_json::from_str(&raw).ok())
}

fn latest_patchset(root: &Utf8Path) -> Option<Value> {
    let mut latest: Option<(std::time::SystemTime, Value)> = None;
    visit_files(root.join("target/xtal/repair").as_std_path(), &mut |path| {
        if path.file_name().and_then(|name| name.to_str()) != Some("patchset.json") {
            return;
        }
        let Ok(metadata) = std::fs::metadata(path) else {
            return;
        };
        let Ok(modified) = metadata.modified() else {
            return;
        };
        let Some(value) = std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
        else {
            return;
        };
        if latest
            .as_ref()
            .map(|(old, _)| modified > *old)
            .unwrap_or(true)
        {
            latest = Some((modified, value));
        }
    });
    latest.map(|(_, value)| value)
}

fn visit_files(dir: &std::path::Path, f: &mut impl FnMut(&std::path::Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_files(&path, f);
        } else {
            f(&path);
        }
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
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

fn patch_target(patch_ast: &Value) -> Option<String> {
    patch_ast
        .get("patches")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("path"))
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| {
            patch_ast
                .get("path")
                .and_then(Value::as_str)
                .map(str::to_string)
        })
}

fn patch_ops<'a>(patch_ast: &'a Value, target: &str) -> Option<&'a Vec<Value>> {
    if let Some(ops) = patch_ast.get("patch").and_then(Value::as_array) {
        return Some(ops);
    }
    patch_ast
        .get("patches")
        .and_then(Value::as_array)?
        .iter()
        .find(|item| item.get("path").and_then(Value::as_str) == Some(target))
        .and_then(|item| item.get("patch"))
        .and_then(Value::as_array)
}

fn after_from_patch(before: &str, patch_ast: &Value, target: &str) -> Option<String> {
    if let Some(after) = first_string(patch_ast, &["after_snippet", "after"]) {
        return Some(after);
    }
    let ops = patch_ops(patch_ast, target)?;
    let mut doc = serde_json::from_str::<Value>(before).ok()?;
    for op in ops {
        apply_op(&mut doc, op).ok()?;
    }
    serde_json::to_string_pretty(&doc).ok()
}

fn apply_op(doc: &mut Value, op: &Value) -> anyhow::Result<()> {
    let op_name = op
        .get("op")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("patch operation missing op"))?;
    let path = op
        .get("path")
        .and_then(Value::as_str)
        .ok_or_else(|| anyhow::anyhow!("patch operation missing path"))?;
    let tokens = decode_pointer(path)?;
    match op_name {
        "add" => add_value(
            doc,
            &tokens,
            op.get("value").cloned().unwrap_or(Value::Null),
        ),
        "replace" => replace_value(
            doc,
            &tokens,
            op.get("value").cloned().unwrap_or(Value::Null),
        ),
        "remove" => remove_value(doc, &tokens),
        "test" => Ok(()),
        other => Err(anyhow::anyhow!("unsupported patch operation `{other}`")),
    }
}

fn decode_pointer(path: &str) -> anyhow::Result<Vec<String>> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err(anyhow::anyhow!("JSON pointer must start with /"));
    }
    path.split('/')
        .skip(1)
        .map(|part| Ok(part.replace("~1", "/").replace("~0", "~")))
        .collect()
}

fn add_value(doc: &mut Value, path: &[String], value: Value) -> anyhow::Result<()> {
    if path.is_empty() {
        *doc = value;
        return Ok(());
    }
    let (parent_path, leaf) = path.split_at(path.len() - 1);
    let parent = value_at_mut(doc, parent_path)?;
    let leaf = &leaf[0];
    match parent {
        Value::Object(map) => {
            map.insert(leaf.clone(), value);
            Ok(())
        }
        Value::Array(items) => {
            if leaf == "-" {
                items.push(value);
                return Ok(());
            }
            let index = leaf.parse::<usize>()?;
            if index > items.len() {
                return Err(anyhow::anyhow!("array index out of bounds"));
            }
            items.insert(index, value);
            Ok(())
        }
        _ => Err(anyhow::anyhow!("patch parent is not a container")),
    }
}

fn replace_value(doc: &mut Value, path: &[String], value: Value) -> anyhow::Result<()> {
    let target = value_at_mut(doc, path)?;
    *target = value;
    Ok(())
}

fn remove_value(doc: &mut Value, path: &[String]) -> anyhow::Result<()> {
    if path.is_empty() {
        *doc = Value::Null;
        return Ok(());
    }
    let (parent_path, leaf) = path.split_at(path.len() - 1);
    let parent = value_at_mut(doc, parent_path)?;
    let leaf = &leaf[0];
    match parent {
        Value::Object(map) => {
            map.remove(leaf);
            Ok(())
        }
        Value::Array(items) => {
            let index = leaf.parse::<usize>()?;
            if index >= items.len() {
                return Err(anyhow::anyhow!("array index out of bounds"));
            }
            items.remove(index);
            Ok(())
        }
        _ => Err(anyhow::anyhow!("patch parent is not a container")),
    }
}

fn value_at_mut<'a>(doc: &'a mut Value, path: &[String]) -> anyhow::Result<&'a mut Value> {
    let mut current = doc;
    for token in path {
        match current {
            Value::Object(map) => {
                current = map
                    .get_mut(token)
                    .ok_or_else(|| anyhow::anyhow!("object key `{token}` not found"))?;
            }
            Value::Array(items) => {
                let index = token.parse::<usize>()?;
                current = items
                    .get_mut(index)
                    .ok_or_else(|| anyhow::anyhow!("array index `{index}` not found"))?;
            }
            _ => return Err(anyhow::anyhow!("patch path traverses non-container")),
        }
    }
    Ok(current)
}

fn safe_relative_path(root: &Utf8Path, relative: &str) -> anyhow::Result<camino::Utf8PathBuf> {
    let rel = Utf8Path::new(relative);
    if relative.contains('\0')
        || rel.is_absolute()
        || rel.components().any(|component| component.as_str() == "..")
    {
        return Err(anyhow::anyhow!(
            "quickfix path must stay inside the workspace"
        ));
    }
    Ok(root.join(rel))
}

fn clip(value: &str, max_chars: usize) -> String {
    let mut out = value.chars().take(max_chars).collect::<String>();
    if value.chars().count() > max_chars {
        out.push_str("\n...");
    }
    out
}

trait Pipe: Sized {
    fn pipe<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }
}

impl<T> Pipe for T {}

#[cfg(test)]
mod tests {
    use super::for_incident;
    use uuid::Uuid;

    #[test]
    fn quickfix_reads_incident_diagnostic_code() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-quickfix-{}", Uuid::new_v4()));
        let dir = root.join(".x07-wasm/incidents/i1");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("run.report.json"),
            r#"{"diagnostic_code":"E123","severity":"error","summary":"bad input"}"#,
        )
        .expect("incident");

        let record = for_incident(root.as_path(), "i1");

        assert_eq!(record.diagnostic_code, "E123");
        assert_eq!(record.severity, "error");
        assert!(record.summary.contains("bad input"));
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn quickfix_uses_incident_kind_when_code_is_missing() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-quickfix-kind-{}", Uuid::new_v4()));
        let dir = root.join(".x07-wasm/incidents/i2");
        std::fs::create_dir_all(&dir).expect("mkdir");
        std::fs::write(
            dir.join("run.report.json"),
            r#"{"kind":"runtime_violation"}"#,
        )
        .expect("incident");

        let record = for_incident(root.as_path(), "i2");

        assert_eq!(record.diagnostic_code, "runtime_violation");
        std::fs::remove_dir_all(root).ok();
    }
}
