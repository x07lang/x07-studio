use std::fs;
use std::io::Write;

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use loom_types::artifacts::{AgentProfile, ProviderProbeReport, ProviderProfile};
use loom_types::session::SessionSnapshot;

#[derive(Debug, Clone)]
pub struct FsStore {
    root: Utf8PathBuf,
}

impl FsStore {
    pub fn new(root: &Utf8Path) -> Self {
        Self {
            root: root.join(".x07").join("studio"),
        }
    }

    pub fn init(&self) -> anyhow::Result<()> {
        fs::create_dir_all(self.sessions_dir())?;
        fs::create_dir_all(self.agents_dir())?;
        fs::create_dir_all(self.providers_dir())?;
        fs::create_dir_all(self.reports_dir())?;
        Ok(())
    }

    pub fn sessions_dir(&self) -> Utf8PathBuf {
        self.root.join("sessions")
    }

    pub fn providers_dir(&self) -> Utf8PathBuf {
        self.root.join("providers")
    }

    pub fn agents_dir(&self) -> Utf8PathBuf {
        self.root.join("agents")
    }

    pub fn reports_dir(&self) -> Utf8PathBuf {
        self.root.join("reports")
    }

    pub fn save_session(&self, session: &SessionSnapshot) -> anyhow::Result<()> {
        let path = self
            .sessions_dir()
            .join(format!("{}.json", session.session_id));
        write_json(&path, session)
    }

    pub fn load_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        let mut sessions = Vec::new();
        for path in json_files(&self.sessions_dir())? {
            let bytes = fs::read(&path)?;
            let snapshot: SessionSnapshot = serde_json::from_slice(&bytes)
                .with_context(|| format!("failed to parse session file at {}", path))?;
            sessions.push(snapshot);
        }
        sessions.sort_by(|a, b| a.title.cmp(&b.title));
        Ok(sessions)
    }

    pub fn save_agent_profile(&self, profile: &AgentProfile) -> anyhow::Result<()> {
        let path = self.agents_dir().join(format!("{}.json", profile.id));
        write_json(&path, profile)
    }

    pub fn load_agent_profiles(&self) -> anyhow::Result<Vec<AgentProfile>> {
        let mut profiles = Vec::new();
        for path in json_files(&self.agents_dir())? {
            let bytes = fs::read(&path)?;
            let profile: AgentProfile = serde_json::from_slice(&bytes)
                .with_context(|| format!("failed to parse agent profile at {}", path))?;
            profiles.push(profile);
        }
        profiles.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(profiles)
    }

    pub fn save_provider_profile(&self, profile: &ProviderProfile) -> anyhow::Result<()> {
        let path = self.providers_dir().join(format!("{}.json", profile.id));
        write_json(&path, profile)
    }

    pub fn load_provider_profiles(&self) -> anyhow::Result<Vec<ProviderProfile>> {
        let mut profiles = Vec::new();
        for path in json_files(&self.providers_dir())? {
            if path
                .file_name()
                .map(|name| name.ends_with(".probe.json"))
                .unwrap_or(false)
            {
                continue;
            }
            let bytes = fs::read(&path)?;
            let profile: ProviderProfile = serde_json::from_slice(&bytes)
                .with_context(|| format!("failed to parse provider profile at {}", path))?;
            profiles.push(profile);
        }
        profiles.sort_by(|a, b| a.label.cmp(&b.label));
        Ok(profiles)
    }

    pub fn save_provider_probe(
        &self,
        profile_id: &str,
        report: &ProviderProbeReport,
    ) -> anyhow::Result<()> {
        let path = self
            .providers_dir()
            .join(format!("{profile_id}.probe.json"));
        write_json(&path, report)
    }

    pub fn next_report_path(&self, stem: &str) -> Utf8PathBuf {
        let safe = stem
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || ch == '.' || ch == '-' {
                    ch
                } else {
                    '_'
                }
            })
            .collect::<String>();
        self.reports_dir().join(format!("{safe}.json"))
    }
}

fn json_files(dir: &Utf8Path) -> anyhow::Result<Vec<Utf8PathBuf>> {
    let mut out = Vec::new();
    if !dir.exists() {
        return Ok(out);
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|value| value.to_str()) == Some("json") {
            if let Ok(path) = Utf8PathBuf::from_path_buf(path) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn write_json<T: serde::Serialize>(path: &Utf8Path, value: &T) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let bytes = serde_json::to_vec_pretty(value)?;
    let mut file = fs::File::create(path)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8PathBuf;
    use loom_types::artifacts::{
        AgentProfile, ProbeStatus, ProviderCapabilities, ProviderProbeMode, ProviderProbeReport,
        ProviderProfile,
    };
    use loom_types::session::SessionSnapshot;
    use uuid::Uuid;

    use super::FsStore;

    #[test]
    fn sessions_roundtrip_through_studio_store() {
        let root = temp_root();
        let store = FsStore::new(root.as_path());
        store.init().expect("store init");
        let session = SessionSnapshot::new(
            Uuid::new_v4(),
            "session b",
            root.to_string(),
            loom_types::artifacts::TaskType::BugFix,
        );

        store.save_session(&session).expect("save session");
        let sessions = store.load_sessions().expect("load sessions");

        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].session_id, session.session_id);
        assert_eq!(sessions[0].title, "session b");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn provider_profiles_ignore_probe_reports_on_load() {
        let root = temp_root();
        let store = FsStore::new(root.as_path());
        store.init().expect("store init");
        let mut profile = ProviderProfile::local_ollama();
        profile.id = "local".to_string();
        profile.probe_mode = ProviderProbeMode::Shallow;
        let report = ProviderProbeReport {
            schema_version: "x07.studio.provider_probe_report@0.1.0".to_string(),
            profile_id: profile.id.clone(),
            base_url: profile.base_url.clone(),
            observed_at: "1".to_string(),
            ok: true,
            http_status: Some(200),
            models: vec!["local-fast".to_string()],
            capabilities: ProviderCapabilities {
                models_endpoint: ProbeStatus::Supported,
                ..ProviderCapabilities::default()
            },
            notes: Vec::new(),
            raw: None,
        };

        store.save_provider_profile(&profile).expect("save profile");
        store
            .save_provider_probe(&profile.id, &report)
            .expect("save probe");
        let profiles = store.load_provider_profiles().expect("load profiles");

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "local");
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn agent_profiles_roundtrip_through_studio_store() {
        let root = temp_root();
        let store = FsStore::new(root.as_path());
        store.init().expect("store init");
        let mut profile = AgentProfile::codex();
        profile.id = "codex-local".to_string();

        store.save_agent_profile(&profile).expect("save profile");
        let profiles = store.load_agent_profiles().expect("load profiles");

        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, "codex-local");
        assert_eq!(profiles[0].command, "codex");
        fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> Utf8PathBuf {
        let path = std::env::temp_dir().join(format!("x07-studio-test-{}", Uuid::new_v4()));
        Utf8PathBuf::from_path_buf(path).expect("utf8 temp path")
    }
}
