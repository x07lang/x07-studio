use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::artifacts::{
    AgentHandoff, AgentProfile, IntentPacket, OpRecord, PlainEnglishSummary, ProviderProbeReport,
    ProviderProfile, TaskType, WitnessKind,
};
use crate::mcp::{McpConnectionInfo, McpEndpoint, McpToolCallResult, McpToolDescriptor};
use crate::ops::SessionEvent;
use crate::session::SessionSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub workspace_root: String,
    pub defaults: StudioDefaults,
    pub components: Vec<RuntimeComponentStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudioDefaults {
    pub daemon_addr: String,
    pub provider_profile_id: String,
    pub platform_state_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeComponentState {
    Available,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeComponentStatus {
    pub id: String,
    pub label: String,
    pub command: String,
    pub required: bool,
    pub status: RuntimeComponentState,
    pub source: Option<String>,
    pub install_hint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRadarResponse {
    pub schema_version: String,
    pub workspace_root: String,
    pub xtal_manifest: WorkspacePathState,
    pub spec_count: usize,
    pub generated_tests: WorkspacePathState,
    pub latest_verify: Option<WorkspacePathState>,
    pub latest_certify: Option<WorkspacePathState>,
    pub incident_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspacePathState {
    pub path: String,
    pub exists: bool,
    pub modified_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionTurn {
    UserIntent {
        id: Uuid,
        at: String,
        raw: String,
        source_kind: String,
    },
    AgentClarify {
        id: Uuid,
        at: String,
        agent_id: String,
        questions: Vec<TurnQuestion>,
    },
    UserAnswer {
        id: Uuid,
        at: String,
        question_id: String,
        text: String,
    },
    AgentDraft {
        id: Uuid,
        at: String,
        agent_id: String,
        summary: String,
        evidence: Vec<TurnEvidence>,
    },
    UserApproved {
        id: Uuid,
        at: String,
        by: String,
    },
    BuildStage {
        id: Uuid,
        at: String,
        stage: String,
        op_ids: Vec<Uuid>,
    },
    Verified {
        id: Uuid,
        at: String,
        summary: PlainEnglishSummary,
        op_ids: Vec<Uuid>,
    },
    Incident {
        id: Uuid,
        at: String,
        incident_id: String,
        summary: String,
        repair_available: bool,
    },
    Repair {
        id: Uuid,
        at: String,
        incident_id: String,
        op_ids: Vec<Uuid>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnQuestion {
    pub id: String,
    pub text: String,
    pub witness_kind: WitnessKind,
    pub options: Vec<String>,
    pub answer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TurnEvidence {
    pub label: String,
    pub op_id: Option<Uuid>,
    pub artifact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateSessionRequest {
    pub title: String,
    pub task_type: TaskType,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchEventRequest {
    pub event: SessionEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentInputMode {
    Text,
    Voice,
    Spec,
    Incident,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalizeIntentRequest {
    pub raw: String,
    pub input_mode: IntentInputMode,
    pub revision_notes: Vec<String>,
    pub provider_profile_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalizeIntentResponse {
    pub intent: IntentPacket,
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestIntentRevisionRequest {
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestIntentRevisionResponse {
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClarifyRequest {
    pub agent_id: String,
    #[serde(default)]
    pub round_max: Option<u32>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentClarifyResponse {
    pub handoff: AgentHandoff,
    pub op: OpRecord,
    pub session: crate::session::SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnswer {
    pub question_id: String,
    pub text: String,
    #[serde(default)]
    pub witness_kind: Option<WitnessKind>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnswerRequest {
    pub answers: Vec<IntentAnswer>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentAnswerResponse {
    pub intent: IntentPacket,
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBuildRequest {
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
    #[serde(default)]
    pub max_repair_rounds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBindingRequest {
    pub binding_id: String,
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TryItInputKind {
    Text,
    File,
    B64,
    Argv,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryItRequest {
    pub input_kind: TryItInputKind,
    pub input_text: Option<String>,
    pub input_b64: Option<String>,
    pub input_path: Option<String>,
    #[serde(default)]
    pub argv: Vec<String>,
    pub profile: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TryItResult {
    pub output_kind: String,
    pub output_text: Option<String>,
    pub output_json: Option<Value>,
    pub stats: Value,
    pub proof_citations: Vec<ProofCitation>,
    pub op_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofCitation {
    pub clause_id: String,
    pub proof_report: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderRung {
    pub id: String,
    pub label: String,
    pub profile_path: Option<String>,
    pub satisfied: bool,
    pub missing: Vec<String>,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LadderState {
    pub current_rung: String,
    pub rungs: Vec<LadderRung>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClimbRungRequest {
    pub to_rung: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumRound {
    pub round: u32,
    pub agents: Vec<QuorumAgent>,
    pub diff: Vec<QuorumDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumAgent {
    pub agent_id: String,
    pub questions: Vec<TurnQuestion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumDiff {
    pub label: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumRequest {
    pub agent_ids: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CassetteEntry {
    pub idx: u32,
    pub kind: String,
    pub key: String,
    pub ts: String,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CassetteBranchRequest {
    pub from_entry: u32,
    pub new_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskRequest {
    pub question: String,
    pub agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskAnswer {
    pub text: String,
    pub citations: Vec<AnswerCitation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnswerCitation {
    pub kind: String,
    pub path: String,
    pub locator: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SyncCode {
    pub code: String,
    pub expires_at: String,
    pub session_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StudioMemory {
    pub preferences: MemoryPreferences,
    pub recent_projects: Vec<MemoryProject>,
    pub reusable_specs: Vec<MemoryReusableSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryPreferences {
    pub default_agent: Option<String>,
    pub default_trust_profile: Option<String>,
    pub naming_style: Option<String>,
    pub verbosity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryProject {
    pub root: String,
    pub last_session_id: Option<Uuid>,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryReusableSpec {
    pub module_id: String,
    pub path: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncClaimResponse {
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentImageUploadResponse {
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualParseRequest {
    pub source: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualEmitRequest {
    pub graph: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VisualResponse {
    pub schema_version: String,
    pub kind: String,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RunXtalWorkflowRequest {
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPreviewRequest {
    pub artifact: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPreviewRequest {
    pub doc_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPreviewResponse {
    pub schema_version: String,
    pub doc_ref: String,
    pub resolved_path: String,
    pub title: String,
    pub media_kind: String,
    pub bytes_read: u64,
    pub truncated: bool,
    pub snippet: String,
    pub entries: Vec<DocPreviewEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPreviewEntry {
    pub path: String,
    pub title: String,
    pub kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPreviewResponse {
    pub schema_version: String,
    pub artifact: String,
    pub media_kind: String,
    pub bytes_read: u64,
    pub truncated: bool,
    pub text: Option<String>,
    pub json: Option<Value>,
    pub patchset_preview: Option<PatchsetPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchsetPreview {
    pub schema_version: String,
    pub targets: Vec<PatchsetTargetPreview>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchsetTargetPreview {
    pub path: String,
    pub note: Option<String>,
    pub operations: usize,
    pub before_json: Option<Value>,
    pub after_json: Option<Value>,
    pub apply_error: Option<String>,
    pub truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunMode {
    Plan,
    Execute,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunRequest {
    pub mode: AgentRunMode,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApprovalRequest {
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolveApprovalRequest {
    pub decision: ApprovalDecision,
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderProfileRequest {
    pub profile: ProviderProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveAgentProfileRequest {
    pub profile: AgentProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProbeProviderRequest {
    pub profile: ProviderProfile,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectMcpRequest {
    pub endpoint: McpEndpoint,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallMcpToolRequest {
    pub name: String,
    pub arguments: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingDescriptor {
    pub id: String,
    pub category: String,
    pub program: String,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectMcpResponse {
    pub connection: McpConnectionInfo,
    pub tools: Vec<McpToolDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionOpResponse {
    pub session: SessionSnapshot,
    pub last_op_id: Option<Uuid>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProbeResponse {
    pub profile: ProviderProfile,
    pub report: ProviderProbeReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpCallResponse {
    pub result: McpToolCallResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentHandoffResponse {
    pub handoff: AgentHandoff,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResponse {
    pub handoff: AgentHandoff,
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentApprovalResponse {
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

/// Server-sent event payload emitted by `GET /v1/sessions/{id}/stream`.
/// Browser clients dedupe `Op` events by `op.id`; `Snapshot` events replace
/// the local session state for phase/room/intent/contract changes that don't
/// fit a single op delta.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionStreamEvent {
    Op {
        op: Box<OpRecord>,
    },
    Snapshot {
        session: Box<SessionSnapshot>,
    },
    /// Periodic keep-alive so proxies don't drop idle connections.
    Heartbeat {
        unix_ms: u64,
    },
}
