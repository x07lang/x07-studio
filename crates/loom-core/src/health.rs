use anyhow::Context;
use camino::Utf8Path;
use loom_adapters::x07_cli::{CliAdapter, X07JsonOptions};
use loom_types::api::{DoctorStatus, HealthSnapshot, LockfileStatus, MigrateStatus};
use serde_json::Value;

pub async fn snapshot(root: &Utf8Path, adapter: &CliAdapter) -> anyhow::Result<HealthSnapshot> {
    let doctor_exec = adapter
        .execute_x07_json(
            "health.doctor",
            "x07/doctor",
            vec!["doctor".to_string()],
            vec![".x07/studio/reports".to_string()],
            "Check local x07 platform prerequisites.",
            X07JsonOptions::report_file(Some(20)),
        )
        .await?;
    let lock_exec = adapter
        .execute_x07_json(
            "health.pkg_lock_check",
            "x07/package",
            vec![
                "pkg".to_string(),
                "lock".to_string(),
                "--project".to_string(),
                "x07.json".to_string(),
                "--check".to_string(),
            ],
            vec!["x07.lock.json".to_string()],
            "Check project lockfile drift and package advisories.",
            X07JsonOptions::report_file(Some(30)),
        )
        .await?;
    let migrate_exec = adapter
        .execute_x07_json(
            "health.migrate_check",
            "x07/migrate",
            vec![
                "migrate".to_string(),
                "--check".to_string(),
                "--to".to_string(),
                "0.5".to_string(),
            ],
            vec!["x07.json".to_string()],
            "Check language schema migration status.",
            X07JsonOptions::report_file(Some(30)),
        )
        .await?;
    let project_migrate_exec = adapter
        .execute_x07_json(
            "health.project_migrate_check",
            "x07/project",
            vec![
                "project".to_string(),
                "migrate".to_string(),
                "--check".to_string(),
                "--project".to_string(),
                "x07.json".to_string(),
            ],
            vec!["x07.json".to_string()],
            "Check project schema migration status.",
            X07JsonOptions::report_file(Some(30)),
        )
        .await?;
    let doctor = doctor_status(
        doctor_exec.execution.exit_code == Some(0),
        doctor_exec.report_json.as_ref(),
        &doctor_exec.execution.stderr,
    );
    let lockfile = lockfile_status(
        lock_exec.execution.exit_code == Some(0),
        lock_exec.report_json.as_ref(),
        &lock_exec.execution.stderr,
    );
    let migrate = migrate_status(
        root,
        migrate_exec.execution.exit_code == Some(0),
        migrate_exec.report_json.as_ref(),
        project_migrate_exec.execution.exit_code == Some(0),
        project_migrate_exec.report_json.as_ref(),
    );
    let overall_color = if !doctor.ok || !lockfile.ok {
        "red"
    } else if !doctor.warnings.is_empty() || lockfile.stale || migrate.needs_migration {
        "amber"
    } else {
        "green"
    };
    Ok(HealthSnapshot {
        schema_version: "x07.studio.health_snapshot@0.1.0".to_string(),
        captured_at: loom_adapters::command_runner::now_string(),
        doctor,
        lockfile,
        migrate,
        subscriber_count: 0,
        active_sessions: 0,
        overall_color: overall_color.to_string(),
    })
}

pub async fn apply_migrate(
    root: &Utf8Path,
    adapter: &CliAdapter,
    target: &str,
) -> anyhow::Result<MigrateStatus> {
    let target = if target.trim().is_empty() {
        "0.5"
    } else {
        target.trim()
    };
    let backup = root.join(format!(
        ".x07/studio/migrate-backup-{}",
        loom_adapters::command_runner::now_string().replace([':', '.'], "-")
    ));
    std::fs::create_dir_all(&backup).with_context(|| format!("create {backup}"))?;
    for path in ["x07.json", "x07.lock.json"] {
        let source = root.join(path);
        if source.is_file() {
            std::fs::copy(&source, backup.join(path))
                .with_context(|| format!("backup {source}"))?;
        }
    }
    let migrate_exec = adapter
        .execute_x07_json(
            "health.migrate_write",
            "x07/migrate",
            vec![
                "migrate".to_string(),
                "--write".to_string(),
                "--to".to_string(),
                target.to_string(),
            ],
            vec!["x07.json".to_string(), backup.to_string()],
            "Apply x07 schema migrations after taking a Studio backup.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    let project_exec = adapter
        .execute_x07_json(
            "health.project_migrate_write",
            "x07/project",
            vec![
                "project".to_string(),
                "migrate".to_string(),
                "--write".to_string(),
                "--project".to_string(),
                "x07.json".to_string(),
            ],
            vec!["x07.json".to_string(), backup.to_string()],
            "Apply x07 project schema migrations after taking a Studio backup.",
            X07JsonOptions::report_file(Some(60)),
        )
        .await?;
    Ok(migrate_status(
        root,
        migrate_exec.execution.exit_code == Some(0),
        migrate_exec.report_json.as_ref(),
        project_exec.execution.exit_code == Some(0),
        project_exec.report_json.as_ref(),
    ))
}

pub fn doctor_status(ok: bool, raw: Option<&Value>, stderr: &str) -> DoctorStatus {
    let blockers = string_array(raw, &["blockers", "errors"])
        .into_iter()
        .chain((!ok).then(|| stderr.trim().to_string()))
        .filter(|item| !item.is_empty())
        .collect();
    DoctorStatus {
        ok: ok && blockers_is_empty(raw),
        blockers,
        warnings: string_array(raw, &["warnings"]),
    }
}

pub fn lockfile_status(ok: bool, raw: Option<&Value>, stderr: &str) -> LockfileStatus {
    let yanked = string_array(raw, &["yanked"]);
    let advisories = string_array(raw, &["advisories"]);
    let stale = bool_value(raw, &["stale", "out_of_date", "changed"]).unwrap_or(!ok);
    LockfileStatus {
        ok: ok && yanked.is_empty() && advisories.is_empty(),
        stale,
        yanked: yanked
            .into_iter()
            .chain((!ok && stderr.contains("yanked")).then(|| stderr.trim().to_string()))
            .collect(),
        advisories,
    }
}

pub fn migrate_status(
    root: &Utf8Path,
    migrate_ok: bool,
    migrate_raw: Option<&Value>,
    project_ok: bool,
    project_raw: Option<&Value>,
) -> MigrateStatus {
    let from_schema = project_schema_version(root)
        .or_else(|| first_string(migrate_raw, &["from_schema"]))
        .or_else(|| first_string(project_raw, &["from_schema"]));
    let project_schema_legacy = from_schema
        .as_deref()
        .map(|schema| {
            schema.contains("@0.2.") || schema.contains("@0.3.") || schema.contains("@0.4.")
        })
        .unwrap_or(false);
    MigrateStatus {
        needs_migration: !migrate_ok
            || !project_ok
            || bool_value(migrate_raw, &["needs_migration", "required"]).unwrap_or(false)
            || bool_value(project_raw, &["needs_migration", "required"]).unwrap_or(false)
            || project_schema_legacy,
        from_schema,
        to_schema: Some("0.5".to_string()),
        project_schema_legacy,
    }
}

fn blockers_is_empty(raw: Option<&Value>) -> bool {
    string_array(raw, &["blockers", "errors"]).is_empty()
}

fn project_schema_version(root: &Utf8Path) -> Option<String> {
    let raw = std::fs::read_to_string(root.join("x07.json")).ok()?;
    let json = serde_json::from_str::<Value>(&raw).ok()?;
    json.get("schema_version")
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn string_array(raw: Option<&Value>, keys: &[&str]) -> Vec<String> {
    let Some(value) = raw.and_then(|raw| first_value(raw, keys)) else {
        return Vec::new();
    };
    match value {
        Value::Array(items) => items
            .iter()
            .filter_map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .or_else(|| Some(item.to_string()))
            })
            .collect(),
        Value::String(text) => vec![text.clone()],
        _ => Vec::new(),
    }
}

fn bool_value(raw: Option<&Value>, keys: &[&str]) -> Option<bool> {
    raw.and_then(|raw| first_value(raw, keys))
        .and_then(Value::as_bool)
}

fn first_string(raw: Option<&Value>, keys: &[&str]) -> Option<String> {
    raw.and_then(|raw| first_value(raw, keys))
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

#[cfg(test)]
mod tests {
    use super::{doctor_status, migrate_status};

    #[test]
    fn doctor_blocks_on_error_arrays() {
        let raw = serde_json::json!({"blockers":["cc not found"],"warnings":["slow"]});
        let status = doctor_status(true, Some(&raw), "");
        assert!(!status.ok);
        assert_eq!(status.blockers, ["cc not found"]);
    }

    #[test]
    fn legacy_project_schema_needs_migration() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-health-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("mkdir");
        std::fs::write(
            root.join("x07.json"),
            r#"{"schema_version":"x07.project@0.4.0"}"#,
        )
        .expect("write");
        let status = migrate_status(root.as_path(), true, None, true, None);
        assert!(status.needs_migration);
        assert!(status.project_schema_legacy);
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn migrate_status_ignores_wrapper_schema_versions() {
        let root = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8")
            .join(format!("x07-studio-health-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).expect("mkdir");
        let raw = serde_json::json!({
            "schema_version": "x07.connected_e2e.report@0.1.0",
            "result": { "schema_version": "x07.migrate.report@0.1.0" }
        });
        let status = migrate_status(root.as_path(), true, Some(&raw), true, Some(&raw));
        assert_eq!(status.from_schema, None);
        assert!(!status.needs_migration);
        std::fs::remove_dir_all(root).ok();
    }
}
