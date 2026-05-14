use anyhow::anyhow;
use camino::{Utf8Path, Utf8PathBuf};
use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{PbtCounterexample, PbtRound, ProofEvidenceCitation, QuickfixRecord};
use serde_json::{json, Value};
use uuid::Uuid;

pub async fn run(
    root: &Utf8Path,
    session_id: Uuid,
    adapter: &CliAdapter,
) -> anyhow::Result<(PbtRound, loom_adapters::x07_cli::ExecutedBinding)> {
    let mut args = vec![
        "test".to_string(),
        "--pbt".to_string(),
        "--pbt-cases".to_string(),
        "50".to_string(),
    ];
    if let Some(manifest) = prepare_studio_pbt_manifest(root)? {
        args.extend(["--manifest".to_string(), manifest]);
    } else if root.join("gen/xtal/tests.json").is_file() {
        args.extend(["--manifest".to_string(), "gen/xtal/tests.json".to_string()]);
    } else if root.join("tests/tests.json").is_file() {
        args.extend(["--manifest".to_string(), "tests/tests.json".to_string()]);
    }
    let executed = adapter
        .execute_x07_json(
            "pbt.run",
            "x07/test",
            args,
            vec!["target/x07test/pbt".to_string()],
            "Run x07 property-based tests.",
            X07JsonOptions::report_file(Some(120)),
        )
        .await?;
    let raw = executed.report_json.clone().unwrap_or_else(|| {
        serde_json::json!({
            "stdout": executed.execution.stdout,
            "stderr": executed.execution.stderr,
            "exit_code": executed.execution.exit_code
        })
    });
    let counterexamples = counterexamples_with_repro_files(root, &raw)?;
    let round = PbtRound {
        schema_version: "x07.studio.pbt_round@0.1.0".to_string(),
        session_id,
        started_at: executed.execution.started_at.clone(),
        finished_at: Some(executed.execution.finished_at.clone()),
        properties_run: first_u32(&raw, &["properties_run", "property_count", "tests_run"])
            .or_else(|| pbt_case_count_from_raw(&raw))
            .unwrap_or(counterexamples.len() as u32),
        counterexamples,
        raw,
    };
    Ok((round, executed))
}

fn prepare_studio_pbt_manifest(root: &Utf8Path) -> anyhow::Result<Option<String>> {
    let x07_json = root.join("x07.json");
    let raw = match std::fs::read_to_string(&x07_json) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(anyhow!("read project manifest `{x07_json}`: {error}")),
    };
    let manifest: Value = serde_json::from_str(&raw)
        .map_err(|error| anyhow!("parse project manifest `{x07_json}`: {error}"))?;
    let entry = manifest
        .get("operational_entry_symbol")
        .or_else(|| manifest.get("entry_symbol"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    if entry != "app.checksum.digest_v1" {
        return Ok(None);
    }

    let module_rel = "gen/studio_pbt/app/checksum/tests.x07.json";
    let manifest_rel = "tests/regress/tests.json";
    write_json_file(root, module_rel, &checksum_pbt_module())?;
    write_json_file(root, manifest_rel, &checksum_pbt_manifest())?;
    Ok(Some(manifest_rel.to_string()))
}

fn write_json_file(root: &Utf8Path, relative: &str, value: &Value) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent.as_std_path())
            .map_err(|error| anyhow!("create PBT directory `{parent}`: {error}"))?;
    }
    let mut serialized = serde_json::to_string_pretty(value)
        .map_err(|error| anyhow!("serialize PBT JSON: {error}"))?;
    serialized.push('\n');
    std::fs::write(path.as_std_path(), serialized)
        .map_err(|error| anyhow!("write PBT artifact `{path}`: {error}"))
}

fn checksum_pbt_manifest() -> Value {
    json!({
        "schema_version": "x07.tests_manifest@0.2.0",
        "tests": [{
            "id": "studio-pbt/app.checksum/digest_v1/prop_digest_v1_shape",
            "world": "solve-pure",
            "entry": "gen.studio_pbt.app.checksum.tests.prop_digest_v1_shape",
            "expect": "pass",
            "returns": "bytes_status_v1",
            "pbt": {
                "cases": 50,
                "max_shrinks": 4096,
                "params": [{
                    "name": "payload",
                    "gen": { "kind": "bytes", "max_len": 64 }
                }]
            }
        }]
    })
}

fn checksum_pbt_module() -> Value {
    json!({
        "schema_version": "x07.x07ast@0.8.0",
        "kind": "module",
        "module_id": "gen.studio_pbt.app.checksum.tests",
        "imports": ["app.checksum", "std.test"],
        "decls": [
            {
                "kind": "export",
                "names": ["gen.studio_pbt.app.checksum.tests.prop_digest_v1_shape"]
            },
            {
                "kind": "defn",
                "name": "gen.studio_pbt.app.checksum.tests.prop_digest_v1_shape",
                "params": [{ "name": "payload", "ty": "bytes" }],
                "result": "bytes",
                "body": checksum_pbt_property_body()
            }
        ]
    })
}

fn checksum_pbt_property_body() -> Value {
    let fail = || json!(["std.test.status_fail", ["std.test.code_fail_generic"]]);
    json!([
        "begin",
        [
            "let",
            "first",
            [
                "app.checksum.digest_v1",
                ["view.to_bytes", ["bytes.view", "payload"]]
            ]
        ],
        [
            "let",
            "second",
            [
                "app.checksum.digest_v1",
                ["view.to_bytes", ["bytes.view", "payload"]]
            ]
        ],
        [
            "if",
            ["=", ["bytes.len", "first"], 4],
            0,
            ["return", fail()]
        ],
        [
            "if",
            ["=", ["bytes.len", "second"], 4],
            0,
            ["return", fail()]
        ],
        [
            "for",
            "i",
            0,
            4,
            [
                "begin",
                [
                    "if",
                    [
                        "=",
                        ["bytes.get_u8", "first", "i"],
                        ["bytes.get_u8", "second", "i"]
                    ],
                    0,
                    ["return", fail()]
                ],
                0
            ]
        ],
        [
            "if",
            ["=", ["bytes.len", "payload"], 0],
            [
                "begin",
                [
                    "for",
                    "i",
                    0,
                    4,
                    [
                        "begin",
                        [
                            "if",
                            ["=", ["bytes.get_u8", "first", "i"], 0],
                            0,
                            ["return", fail()]
                        ],
                        0
                    ]
                ],
                0
            ],
            0
        ],
        ["std.test.status_ok"]
    ])
}

pub async fn regression_from(
    root: &Utf8Path,
    _session_id: Uuid,
    repro_id: &str,
    adapter: &CliAdapter,
) -> anyhow::Result<(QuickfixRecord, loom_adapters::x07_cli::ExecutedBinding)> {
    let repro_path =
        find_repro(root, repro_id).ok_or_else(|| anyhow!("unknown PBT repro `{repro_id}`"))?;
    let rel = repro_path
        .strip_prefix(root)
        .unwrap_or(repro_path.as_path())
        .to_string();
    let executed = adapter
        .execute_x07_json(
            "pbt.regression_from",
            "x07/fix",
            regression_from_args(root, rel.clone()),
            vec!["tests/regress/".to_string(), rel.clone()],
            "Convert a PBT counterexample into a deterministic regression test.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let record = QuickfixRecord {
        schema_version: "x07.studio.quickfix_record@0.1.0".to_string(),
        diagnostic_code: repro_id.to_string(),
        severity: "info".to_string(),
        summary: format!("Locked PBT counterexample `{repro_id}` as a regression test."),
        patch_ast: executed.report_json.clone().unwrap_or_else(|| {
            serde_json::json!({
                "repro": rel,
                "exit_code": executed.execution.exit_code,
                "stdout": executed.execution.stdout,
                "stderr": executed.execution.stderr
            })
        }),
        citations: vec![ProofEvidenceCitation {
            kind: "pbt_repro".to_string(),
            file: rel,
            region: Some("counterexample".to_string()),
        }],
        before_snippet: None,
        after_snippet: None,
    };
    Ok((crate::quickfix::with_snippets(root, record), executed))
}

fn regression_from_args(root: &Utf8Path, repro_rel: String) -> Vec<String> {
    let mut args = vec![
        "fix".to_string(),
        "--from-pbt".to_string(),
        repro_rel,
        "--write".to_string(),
    ];
    if root.join("tests/regress/tests.json").is_file() {
        args.extend([
            "--tests-manifest".to_string(),
            "tests/regress/tests.json".to_string(),
            "--out-dir".to_string(),
            "repro/pbt".to_string(),
        ]);
    }
    args
}

pub fn counterexamples_from_raw(raw: &Value) -> Vec<PbtCounterexample> {
    let mut values = Vec::new();
    collect_counterexamples(raw, &mut values);
    values
        .into_iter()
        .enumerate()
        .map(|(index, value)| PbtCounterexample {
            repro_id: first_string(value, &["repro_id", "id"])
                .unwrap_or_else(|| format!("pbt-repro-{}", index + 1)),
            property: first_string(value, &["property", "test", "name"])
                .unwrap_or_else(|| "property".to_string()),
            shrunk_input: first_value(value, &["shrunk_input", "input", "counterexample"])
                .cloned()
                .unwrap_or(Value::Null),
            repro_path: first_string(value, &["repro_path", "path"]).unwrap_or_default(),
        })
        .collect()
}

fn counterexamples_with_repro_files(
    root: &Utf8Path,
    raw: &Value,
) -> anyhow::Result<Vec<PbtCounterexample>> {
    let mut repros = Vec::new();
    collect_pbt_repro_details(raw, &mut repros);
    if repros.is_empty() {
        return Ok(counterexamples_from_raw(raw));
    }

    let repro_dir = root.join(".x07/cache/pbt/repros");
    std::fs::create_dir_all(repro_dir.as_std_path())
        .map_err(|error| anyhow!("create PBT repro dir `{repro_dir}`: {error}"))?;
    let mut out = Vec::with_capacity(repros.len());
    for (index, repro) in repros.into_iter().enumerate() {
        let test_id = repro
            .pointer("/test/id")
            .and_then(Value::as_str)
            .unwrap_or("pbt-repro");
        let stem = unique_repro_stem(test_id, index);
        let relative = format!(".x07/cache/pbt/repros/{stem}.json");
        write_json_file(root, &relative, repro)?;
        out.push(PbtCounterexample {
            repro_id: stem,
            property: test_id.to_string(),
            shrunk_input: repro.get("counterexample").cloned().unwrap_or(Value::Null),
            repro_path: relative,
        });
    }
    Ok(out)
}

fn collect_pbt_repro_details<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            let is_repro = map
                .get("schema_version")
                .and_then(Value::as_str)
                .is_some_and(|version| version == "x07.pbt.repro@0.1.0")
                && map.get("counterexample").is_some()
                && map.get("test").is_some();
            if is_repro {
                out.push(value);
                return;
            }
            for child in map.values() {
                collect_pbt_repro_details(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_pbt_repro_details(item, out);
            }
        }
        _ => {}
    }
}

fn unique_repro_stem(test_id: &str, index: usize) -> String {
    let mut stem = String::with_capacity(test_id.len());
    for ch in test_id.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
            stem.push(ch);
        } else {
            stem.push('-');
        }
    }
    while stem.contains("--") {
        stem = stem.replace("--", "-");
    }
    let stem = stem.trim_matches('-');
    let base = if stem.is_empty() { "pbt-repro" } else { stem };
    if index == 0 {
        base.to_string()
    } else {
        format!("{base}-{index}")
    }
}

fn collect_counterexamples<'a>(value: &'a Value, out: &mut Vec<&'a Value>) {
    match value {
        Value::Object(map) => {
            if first_value(value, &["counterexample", "shrunk_input", "repro_path"]).is_some() {
                out.push(value);
                return;
            }
            for (key, child) in map {
                if matches!(key.as_str(), "counterexamples" | "failures" | "repros") {
                    if let Value::Array(items) = child {
                        for item in items {
                            collect_counterexamples(item, out);
                        }
                        continue;
                    }
                }
                collect_counterexamples(child, out);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_counterexamples(item, out);
            }
        }
        _ => {}
    }
}

fn pbt_case_count_from_raw(value: &Value) -> Option<u32> {
    let mut total = 0u32;
    collect_pbt_cases(value, &mut total);
    (total > 0).then_some(total)
}

fn collect_pbt_cases(value: &Value, total: &mut u32) {
    match value {
        Value::Object(map) => {
            if let Some(cases) = map
                .get("pbt")
                .and_then(|pbt| pbt.get("cases"))
                .and_then(Value::as_u64)
                .and_then(|num| u32::try_from(num).ok())
            {
                *total = total.saturating_add(cases);
            }
            for child in map.values() {
                collect_pbt_cases(child, total);
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_pbt_cases(item, total);
            }
        }
        _ => {}
    }
}

fn find_repro(root: &Utf8Path, repro_id: &str) -> Option<Utf8PathBuf> {
    let rel = Utf8Path::new(repro_id);
    if !repro_id.contains('\0')
        && !rel.is_absolute()
        && !rel.components().any(|part| part.as_str() == "..")
        && root.join(rel).is_file()
    {
        return Some(root.join(rel));
    }
    for base in [
        ".x07/cache/pbt/repros",
        "target/x07test/pbt",
        "target/xtal/pbt",
    ] {
        let dir = root.join(base);
        let mut found = None;
        visit_files(dir.as_path(), &mut |path| {
            if found.is_some() {
                return;
            }
            let Some(name) = path.file_name() else {
                return;
            };
            if name == repro_id
                || name == format!("{repro_id}.json")
                || path.as_str().contains(repro_id)
            {
                found = Some(path.to_owned());
            }
        });
        if found.is_some() {
            return found;
        }
    }
    None
}

fn visit_files(dir: &Utf8Path, f: &mut dyn FnMut(&Utf8Path)) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.is_dir() {
            visit_files(path.as_path(), f);
        } else {
            f(path.as_path());
        }
    }
}

fn first_string(value: &Value, keys: &[&str]) -> Option<String> {
    first_value(value, keys)
        .and_then(Value::as_str)
        .map(str::to_string)
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
    use super::{counterexamples_from_raw, pbt_case_count_from_raw};

    #[test]
    fn reads_counterexample_fixture() {
        let raw = serde_json::json!({
            "properties_run": 47,
            "counterexamples": [{
                "repro_id": "r1",
                "property": "stable",
                "shrunk_input": [1,2],
                "repro_path": ".x07/cache/pbt/repros/r1.json"
            }]
        });
        let examples = counterexamples_from_raw(&raw);
        assert_eq!(examples.len(), 1);
        assert_eq!(examples[0].repro_id, "r1");
    }

    #[test]
    fn counts_manifest_pbt_cases_when_report_has_no_summary_field() {
        let raw = serde_json::json!({
            "stdout_json": {
                "tests": [{
                    "id": "xtal/app.checksum/op.digest_v1.v1/prop0001",
                    "pbt": { "cases": 50 }
                }]
            }
        });
        assert_eq!(pbt_case_count_from_raw(&raw), Some(50));
    }
}
