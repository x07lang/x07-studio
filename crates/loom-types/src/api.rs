use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::artifacts::{
    AgentHandoff, AgentProfile, IntentPacket, OpRecord, ProviderProbeReport, ProviderProfile,
    TaskType,
};
use crate::mcp::{McpConnectionInfo, McpEndpoint, McpToolCallResult, McpToolDescriptor};
use crate::ops::SessionEvent;
use crate::session::SessionSnapshot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub workspace_root: String,
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
    Incident,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalizeIntentRequest {
    pub raw: String,
    pub input_mode: IntentInputMode,
    pub revision_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormalizeIntentResponse {
    pub intent: IntentPacket,
    pub op: OpRecord,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunBindingRequest {
    pub binding_id: String,
    pub vars: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactPreviewRequest {
    pub artifact: String,
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
