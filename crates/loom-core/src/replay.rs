use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use loom_types::api::{ReplayCapsule, ReplayExportResponse};
use loom_types::session::SessionSnapshot;
use sha2::{Digest, Sha256};
use uuid::Uuid;

pub fn export_capsule(
    root: &Utf8Path,
    session: &SessionSnapshot,
) -> anyhow::Result<ReplayExportResponse> {
    let capsule = build_capsule(root, session)?;
    let artifact = format!(".x07/studio/replay/{}.json", capsule.capsule_id);
    let path = root.join(&artifact);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent.as_std_path())?;
    }
    std::fs::write(path.as_std_path(), serde_json::to_vec_pretty(&capsule)?)?;
    Ok(ReplayExportResponse {
        capsule_id: capsule.capsule_id,
        artifact,
        signature: capsule.signature,
    })
}

pub fn build_capsule(root: &Utf8Path, session: &SessionSnapshot) -> anyhow::Result<ReplayCapsule> {
    let capsule_id = Uuid::new_v4().to_string();
    let cassettes = cassette_manifest(root)?;
    let latest_summary = session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "summary.plain_english")
        .and_then(|op| op.report_json.clone());
    let manifest = serde_json::json!({
        "schema_version": "x07.studio.replay_manifest@0.1.0",
        "capsule_id": capsule_id,
        "session_id": session.session_id,
        "latest_summary": latest_summary,
        "cassettes": cassettes,
    });
    let manifest_bytes = serde_json::to_vec(&manifest)?;
    let signature = serde_json::json!({
        "schema_version": "x07.studio.replay_signature@0.1.0",
        "scheme": "sha256-local-manifest-v1",
        "sha256": sha256_hex(&manifest_bytes),
    });
    Ok(ReplayCapsule {
        schema_version: "x07.studio.replay_capsule@0.1.0".to_string(),
        capsule_id,
        session: session.clone(),
        manifest,
        signature,
    })
}

pub fn import_capsule(root: &Utf8Path, capsule: &ReplayCapsule) -> anyhow::Result<SessionSnapshot> {
    let artifact = root.join(format!(
        ".x07/studio/replay/imported-{}.json",
        capsule.capsule_id
    ));
    if let Some(parent) = artifact.parent() {
        std::fs::create_dir_all(parent.as_std_path())?;
    }
    std::fs::write(artifact.as_std_path(), serde_json::to_vec_pretty(capsule)?)?;
    Ok(capsule.session.clone())
}

fn cassette_manifest(root: &Utf8Path) -> anyhow::Result<Vec<serde_json::Value>> {
    let mut out = Vec::new();
    let rr_root = root.join(".x07_rr");
    if !rr_root.exists() {
        return Ok(out);
    }
    collect_files(root, rr_root.as_path(), &mut out)?;
    out.sort_by(|left, right| left["path"].as_str().cmp(&right["path"].as_str()));
    Ok(out)
}

fn collect_files(
    root: &Utf8Path,
    dir: &Utf8Path,
    out: &mut Vec<serde_json::Value>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir.as_std_path())? {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|path| anyhow::anyhow!("non-utf8 replay path: {path:?}"))?;
        if path.is_dir() {
            collect_files(root, path.as_path(), out)?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let bytes = std::fs::read(path.as_std_path())
            .with_context(|| format!("read replay cassette {path}"))?;
        let relative = path
            .strip_prefix(root)
            .map(|path| path.to_string())
            .unwrap_or_else(|_| path.to_string());
        out.push(serde_json::json!({
            "path": relative,
            "bytes": bytes.len(),
            "sha256": sha256_hex(&bytes),
        }));
    }
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use camino::Utf8PathBuf;
    use loom_types::artifacts::TaskType;
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    use super::{build_capsule, import_capsule};

    #[test]
    fn replay_capsule_roundtrip_preserves_session_and_cassette_digest() {
        let root = temp_root();
        std::fs::create_dir_all(root.join(".x07_rr/http").as_std_path()).expect("mkdir");
        std::fs::write(
            root.join(".x07_rr/http/001-request.json").as_std_path(),
            br#"{"ok":true}"#,
        )
        .expect("cassette");
        let session = SessionSnapshot::new(
            Uuid::new_v4(),
            "replay demo",
            root.as_str(),
            TaskType::NewBehavior,
        );

        let capsule = build_capsule(root.as_path(), &session).expect("capsule");
        let imported = import_capsule(root.as_path(), &capsule).expect("import");

        assert_eq!(imported.session_id, session.session_id);
        assert_eq!(capsule.signature["scheme"], "sha256-local-manifest-v1");
        assert_eq!(
            capsule.manifest["cassettes"][0]["path"],
            ".x07_rr/http/001-request.json"
        );
    }

    fn temp_root() -> Utf8PathBuf {
        let root = std::env::temp_dir().join(format!("x07-studio-replay-{}", Uuid::new_v4()));
        Utf8PathBuf::from_path_buf(root).expect("utf8 temp path")
    }
}
