use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::artifacts::{
    AgentHandoff, AgentProfile, IntentPacket, OpRecord, ProviderProbeReport, ProviderProfile,
    TaskType, WitnessKind,
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
