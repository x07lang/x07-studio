use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    NewBehavior,
    BugFix,
    BehaviorChange,
    IncidentRepair,
    Explanation,
    BrownfieldExtract,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WitnessKind {
    DesiredBehavior,
    ForbiddenBehavior,
    PolicyRequirement,
    IncidentReport,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Witness {
    pub kind: WitnessKind,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentTarget {
    pub module_id: String,
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum IntentSource {
    Text { raw: String },
    Voice { transcript: String },
    Incident { path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentPacket {
    pub schema_version: String,
    pub session_id: Uuid,
    pub workspace_root: String,
    pub task_type: TaskType,
    pub targets: Vec<IntentTarget>,
    pub examples: Vec<String>,
    pub constraints: Vec<String>,
    pub policy_implications: Vec<String>,
    pub ambiguities: Vec<String>,
    pub assumptions: Vec<String>,
    pub witnesses: Vec<Witness>,
    pub source: IntentSource,
}

impl IntentPacket {
    pub fn demo(session_id: Uuid, workspace_root: impl Into<String>) -> Self {
        Self {
            schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
            session_id,
            workspace_root: workspace_root.into(),
            task_type: TaskType::NewBehavior,
            targets: vec![IntentTarget {
                module_id: "app.sorter".to_string(),
                entry: Some("sort_ascending".to_string()),
            }],
            examples: vec![
                "[3,1,2] -> [1,2,3]".to_string(),
                "[2,2,1] -> [1,2,2]".to_string(),
            ],
            constraints: vec!["reject empty input".to_string()],
            policy_implications: vec![],
            ambiguities: vec!["sorting stability not yet formalized as a property".to_string()],
            assumptions: vec!["byte array sort is ascending, unsigned".to_string()],
            witnesses: vec![
                Witness {
                    kind: WitnessKind::DesiredBehavior,
                    text: "Keep equal items in order.".to_string(),
                },
                Witness {
                    kind: WitnessKind::ForbiddenBehavior,
                    text: "Reject empty input.".to_string(),
                },
            ],
            source: IntentSource::Text {
                raw: "Create a stable sorter for byte arrays. Equal items must keep their original order. Reject empty input.".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpRecord {
    pub schema_version: String,
    pub id: Uuid,
    pub session_id: Uuid,
    pub op: String,
    pub backend: String,
    pub command: Vec<String>,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub status: OperationStatus,
    pub exit_code: Option<i32>,
    pub artifacts: Vec<String>,
    pub notes: Option<String>,
    pub stdout: Option<String>,
    pub stderr: Option<String>,
    pub stdout_json: Option<Value>,
    pub stderr_json: Option<Value>,
    pub report_json: Option<Value>,
    pub report_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApiKind {
    OpenaiCompatible,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    LocalTrusted,
    RemoteUntrusted,
    RemoteTrusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderProbeMode {
    Shallow,
    Deep,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfile {
    pub schema_version: String,
    pub id: String,
    pub label: String,
    pub base_url: String,
    pub api_key_env: Option<String>,
    pub api_key: Option<String>,
    pub api_kind: ApiKind,
    pub model: Option<String>,
    pub default_headers: BTreeMap<String, String>,
    pub local: bool,
    pub trust_tier: TrustTier,
    pub probe_mode: ProviderProbeMode,
    pub disabled: bool,
}

impl ProviderProfile {
    pub fn local_ollama() -> Self {
        Self {
            schema_version: "x07.studio.provider_profile@0.1.0".to_string(),
            id: "ollama-local".to_string(),
            label: "Ollama local".to_string(),
            base_url: "http://127.0.0.1:11434/v1".to_string(),
            api_key_env: None,
            api_key: None,
            api_kind: ApiKind::OpenaiCompatible,
            model: None,
            default_headers: BTreeMap::new(),
            local: true,
            trust_tier: TrustTier::LocalTrusted,
            probe_mode: ProviderProbeMode::Deep,
            disabled: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProbeStatus {
    Supported,
    Unsupported,
    Unknown,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub models_endpoint: ProbeStatus,
    pub responses: ProbeStatus,
    pub chat_completions: ProbeStatus,
    pub tools: ProbeStatus,
    pub json_schema: ProbeStatus,
    pub streaming: ProbeStatus,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            models_endpoint: ProbeStatus::Unknown,
            responses: ProbeStatus::Unknown,
            chat_completions: ProbeStatus::Unknown,
            tools: ProbeStatus::Unknown,
            json_schema: ProbeStatus::Unknown,
            streaming: ProbeStatus::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbeReport {
    pub schema_version: String,
    pub profile_id: String,
    pub base_url: String,
    pub observed_at: String,
    pub ok: bool,
    pub http_status: Option<u16>,
    pub models: Vec<String>,
    pub capabilities: ProviderCapabilities,
    pub notes: Vec<String>,
    pub raw: Option<Value>,
}
