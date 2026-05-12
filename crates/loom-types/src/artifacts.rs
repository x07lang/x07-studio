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
    Spec { raw: String },
    Incident { path: String },
    Sketch { path: String },
    Image { path: String, mime: String },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClarificationTurn {
    pub question_id: String,
    pub question_text: String,
    pub witness_kind: WitnessKind,
    pub round: u32,
    pub agent_id: String,
    pub options: Vec<String>,
    pub question_recorded_at: String,
    pub answer_text: Option<String>,
    pub answer_recorded_at: Option<String>,
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
    #[serde(default)]
    pub clarification_history: Vec<ClarificationTurn>,
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
            clarification_history: Vec::new(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentStatus {
    Available,
    NeedsInstall,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentProfile {
    pub schema_version: String,
    pub id: String,
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub allowed_verbs: Vec<String>,
    pub mcp_tools: Vec<String>,
    pub write_roots: Vec<String>,
    pub approval_required: bool,
    pub status: AgentStatus,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentHandoff {
    pub schema_version: String,
    pub session_id: Uuid,
    pub agent_id: String,
    pub agent_label: String,
    pub command: Vec<String>,
    pub prompt_path: String,
    pub prompt: String,
    pub allowed_verbs: Vec<String>,
    pub mcp_tools: Vec<String>,
    pub write_roots: Vec<String>,
    pub approval_required: bool,
    pub artifacts: Vec<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlainEnglishSummary {
    pub schema_version: String,
    pub headline: String,
    pub behavior_promises: Vec<String>,
    pub boundaries: Vec<String>,
    pub evidence: Vec<String>,
    pub run_invocation: Option<String>,
    pub followups: Vec<String>,
}

impl AgentProfile {
    pub fn codex() -> Self {
        Self {
            schema_version: "x07.studio.agent_profile@0.1.0".to_string(),
            id: "openai-codex".to_string(),
            label: "OpenAI Codex".to_string(),
            command: "codex".to_string(),
            args: Vec::new(),
            allowed_verbs: vec![
                "intent.formalize".to_string(),
                "intent.clarify".to_string(),
                "spec.check".to_string(),
                "impl.sync.write".to_string(),
                "xtal.verify".to_string(),
                "xtal.repair".to_string(),
            ],
            mcp_tools: vec![
                "x07.search_v1".to_string(),
                "x07.context_pack_v1".to_string(),
                "x07.exec_v1".to_string(),
            ],
            write_roots: vec![
                "spec/".to_string(),
                "src/".to_string(),
                "tests/".to_string(),
            ],
            approval_required: true,
            status: AgentStatus::NeedsInstall,
            notes: "Remote coding-agent runner gated by x07 session contract.".to_string(),
        }
    }

    pub fn claude_code() -> Self {
        Self {
            schema_version: "x07.studio.agent_profile@0.1.0".to_string(),
            id: "claude-code".to_string(),
            label: "Claude Code".to_string(),
            command: "claude".to_string(),
            args: Vec::new(),
            allowed_verbs: vec![
                "intent.clarify".to_string(),
                "impl.sync.patchset".to_string(),
                "impl.check".to_string(),
                "xtal.certify".to_string(),
            ],
            mcp_tools: vec![
                "x07.search_v1".to_string(),
                "x07.context_pack_v1".to_string(),
            ],
            write_roots: vec!["src/".to_string(), "tests/".to_string()],
            approval_required: true,
            status: AgentStatus::NeedsInstall,
            notes: "Alternate coding-agent runner for implementation and review lanes.".to_string(),
        }
    }
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
