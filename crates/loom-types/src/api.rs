use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::artifacts::{
    AgentHandoff, AgentProfile, IntentPacket, OpRecord, OperationStatus, PlainEnglishSummary,
    ProviderProbeReport, ProviderProfile, TaskType, WitnessKind,
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
    /// A supervised agent run that filled in the implementation under
    /// `src/` after a scaffolded build. Carries the agent id, the write
    /// audit summary, and the follow-up impl.check / xtal.verify op ids
    /// so the timeline can render proof that the realize step actually
    /// changed something and stayed within the approved write roots.
    AgentRealize {
        id: Uuid,
        at: String,
        agent_id: String,
        ok: bool,
        wrote_files: Vec<String>,
        op_ids: Vec<Uuid>,
    },
    AgentStream {
        id: Uuid,
        at: String,
        agent_id: String,
        event: AgentStreamEvent,
        op_id: Uuid,
    },
    QuorumRealize {
        id: Uuid,
        at: String,
        round: RealizeQuorumRound,
        op_ids: Vec<Uuid>,
    },
    TrustPostureChanged {
        id: Uuid,
        at: String,
        posture: TrustPosture,
    },
    McpCall {
        id: Uuid,
        at: String,
        call: AgentStreamEvent,
        op_id: Uuid,
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
    #[serde(alias = "intent_text")]
    pub title: String,
    #[serde(default = "default_task_type", alias = "mode")]
    pub task_type: TaskType,
}

fn default_task_type() -> TaskType {
    TaskType::NewBehavior
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
    #[serde(default)]
    pub voice_transcript: Option<VoiceTranscript>,
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
pub struct RealizeRequest {
    /// Agent to delegate the realization to (defaults to `claude-code`
    /// when omitted). Must have `impl.sync.write` in its allowed_verbs
    /// and `src/` in its write_roots.
    #[serde(default)]
    pub agent_id: Option<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeResponse {
    pub agent_id: String,
    pub ok: bool,
    pub wrote_files: Vec<String>,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentStreamEvent {
    Reasoning {
        id: Uuid,
        at: String,
        text: String,
    },
    ToolUse {
        id: Uuid,
        at: String,
        agent_id: String,
        tool: String,
        input: Value,
    },
    ToolResult {
        id: Uuid,
        at: String,
        agent_id: String,
        tool: String,
        success: bool,
        snippet: Option<String>,
    },
    AgentMessage {
        id: Uuid,
        at: String,
        agent_id: String,
        text: String,
    },
    Done {
        id: Uuid,
        at: String,
        agent_id: String,
        exit_code: i32,
    },
    McpCall {
        id: Uuid,
        at: String,
        agent_id: String,
        tool: String,
        server: String,
        input: Value,
        output: Option<Value>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LiveDiff {
    pub schema_version: String,
    pub path: String,
    pub before: Option<String>,
    pub after: Option<String>,
    pub unified_diff: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeQuorumRound {
    pub schema_version: String,
    pub session_id: Uuid,
    pub started_at: String,
    pub finished_at: Option<String>,
    pub proposals: Vec<RealizeProposal>,
    pub agreed: bool,
    pub judge: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeProposal {
    pub schema_version: String,
    pub agent_id: String,
    pub path: String,
    pub body: Value,
    pub digest: String,
    pub stdout_excerpt: String,
    pub stderr_excerpt: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RealizeQuorumRequest {
    #[serde(default = "default_realize_quorum_request_schema_version")]
    pub schema_version: String,
    pub agent_ids: Vec<String>,
    #[serde(default)]
    pub timeout_seconds: Option<u64>,
}

fn default_realize_quorum_request_schema_version() -> String {
    "x07.studio.realize_quorum_request@0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickRealizeProposalRequest {
    pub proposal_index: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PickRealizeProposalResponse {
    pub round: RealizeQuorumRound,
    pub session: SessionSnapshot,
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
    #[serde(default)]
    pub gates: Vec<RungGate>,
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
pub struct ReleaseRequest {
    #[serde(default = "default_release_request_schema_version")]
    pub schema_version: String,
    pub rung: String,
    pub environment: String,
    #[serde(default)]
    pub binding_refs: Vec<String>,
}

fn default_release_request_schema_version() -> String {
    "x07.studio.release_request@0.1.0".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RungGate {
    pub id: String,
    pub label: String,
    pub description: String,
    pub currently_satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPosture {
    pub schema_version: String,
    pub session_id: Uuid,
    pub captured_at: String,
    pub trust_profile: String,
    pub worlds: Vec<String>,
    pub capabilities: Vec<Capability>,
    pub budgets: BudgetSummary,
    pub proof_coverage: ProofCoverage,
    pub deltas: Vec<PostureDelta>,
    pub posture_color: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String,
    pub source: String,
    pub justification: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BudgetSummary {
    pub local_cap_ms: Option<u64>,
    pub arch_profile: Option<String>,
    pub prover_seconds_used: u64,
    pub prover_seconds_cap: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProofCoverage {
    pub support_pct: f32,
    pub proved_pct: f32,
    pub proof_count: u32,
    pub assumptions_open: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostureDelta {
    pub at: String,
    pub kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDiffRequest {
    #[serde(default = "default_semantic_diff_request_schema_version")]
    pub schema_version: String,
    pub from: DiffRef,
    pub to: DiffRef,
    #[serde(default = "default_semantic_diff_mode")]
    pub mode: String,
}

fn default_semantic_diff_request_schema_version() -> String {
    "x07.studio.semantic_diff_request@0.1.0".to_string()
}

fn default_semantic_diff_mode() -> String {
    "project".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DiffRef {
    OpId { op_id: Uuid },
    TurnId { turn_id: Uuid },
    Hash { hash: String },
    Current,
    QuorumProposal { round: Uuid, agent_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SemanticDiff {
    pub schema_version: String,
    pub from: DiffRef,
    pub to: DiffRef,
    pub headline: String,
    pub trust_delta_color: String,
    pub raw: Value,
    pub world_changes: Vec<String>,
    pub capability_changes: Vec<String>,
    pub budget_changes: Vec<String>,
    pub proof_changes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidence {
    pub schema_version: String,
    pub session_id: Uuid,
    pub behavior_id: String,
    pub status: String,
    pub citations: Vec<ProofEvidenceCitation>,
    pub obligations: Vec<ProofObligation>,
    pub z3_ms: Option<u64>,
    pub assumptions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofEvidenceCitation {
    pub kind: String,
    pub file: String,
    pub region: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofObligation {
    pub id: String,
    pub goal: String,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuickfixRecord {
    pub schema_version: String,
    pub diagnostic_code: String,
    pub severity: String,
    pub summary: String,
    pub patch_ast: Value,
    pub citations: Vec<ProofEvidenceCitation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CassetteRibbon {
    pub schema_version: String,
    pub boundaries: Vec<BoundaryEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoundaryEntry {
    pub at: String,
    pub kind: String,
    pub policy: String,
    pub summary: String,
    pub cassette_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CertificateSummary {
    pub schema_version: String,
    pub session_id: Uuid,
    pub profile: String,
    pub operational_entry: String,
    pub issued_at: String,
    pub expires_at: Option<String>,
    pub proof_summary: Value,
    pub trust_report: Value,
    pub html_summary_path: String,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Recipe {
    pub schema_version: String,
    pub id: String,
    pub title: String,
    pub one_liner: String,
    pub intent_text: String,
    pub task_type: TaskType,
    pub preview_posture: TrustPosture,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReleaseStatus {
    pub schema_version: String,
    pub release_id: String,
    pub rung: String,
    pub environment: String,
    pub status: OperationStatus,
    pub op_ids: Vec<Uuid>,
    pub message: String,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncCode {
    pub code: String,
    pub expires_at: String,
    pub session_id: Uuid,
    #[serde(default)]
    pub state_blob: Option<Value>,
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
    #[serde(default)]
    pub state_blob: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncStateRequest {
    pub state_blob: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncSession {
    pub schema_version: String,
    pub code: String,
    pub expires_at: String,
    pub session_id: Uuid,
    pub state_blob: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceTranscript {
    pub schema_version: String,
    pub text: String,
    pub confidence: f32,
    pub language: String,
    pub recorded_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotState {
    pub schema_version: String,
    pub session_id: Uuid,
    pub engaged: bool,
    pub policy: AutopilotPolicy,
    pub last_decision: Option<AutopilotDecision>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AutopilotPolicy {
    pub auto_answer_min_confidence: f32,
    pub max_clarify_rounds: u32,
    pub auto_climb_to: Option<String>,
    pub allow_repair_iters: u32,
    pub allow_quorum: bool,
}

impl Default for AutopilotPolicy {
    fn default() -> Self {
        Self {
            auto_answer_min_confidence: 0.7,
            max_clarify_rounds: 3,
            auto_climb_to: None,
            allow_repair_iters: 3,
            allow_quorum: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotDecision {
    pub at: String,
    pub stage: String,
    pub action: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AutopilotStartRequest {
    #[serde(default)]
    pub policy: Option<AutopilotPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutopilotResponse {
    pub state: AutopilotState,
    pub session: SessionSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayCapsule {
    pub schema_version: String,
    pub capsule_id: String,
    pub session: SessionSnapshot,
    pub manifest: Value,
    pub signature: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayExportResponse {
    pub capsule_id: String,
    pub artifact: String,
    pub signature: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayImportRequest {
    pub capsule: ReplayCapsule,
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

#[cfg(test)]
mod tests {
    use super::{CreateSessionRequest, RealizeQuorumRequest, ReleaseRequest};
    use crate::artifacts::TaskType;

    #[test]
    fn create_session_accepts_intent_text_and_default_task_type() {
        let request: CreateSessionRequest =
            serde_json::from_str(r#"{"intent_text":"Build a sorter"}"#).expect("request");

        assert_eq!(request.title, "Build a sorter");
        assert_eq!(request.task_type, TaskType::NewBehavior);
    }

    #[test]
    fn create_session_accepts_mode_alias() {
        let request: CreateSessionRequest =
            serde_json::from_str(r#"{"title":"Fix","mode":"bug_fix"}"#).expect("request");

        assert_eq!(request.title, "Fix");
        assert_eq!(request.task_type, TaskType::BugFix);
    }

    #[test]
    fn realize_quorum_request_accepts_minimal_body() {
        let request: RealizeQuorumRequest =
            serde_json::from_str(r#"{"agent_ids":["claude-code"]}"#).expect("request");

        assert_eq!(
            request.schema_version,
            "x07.studio.realize_quorum_request@0.1.0"
        );
        assert_eq!(request.agent_ids, ["claude-code"]);
        assert_eq!(request.timeout_seconds, None);
    }

    #[test]
    fn release_request_accepts_minimal_body() {
        let request: ReleaseRequest =
            serde_json::from_str(r#"{"rung":"shareable","environment":"shareable"}"#)
                .expect("request");

        assert_eq!(request.schema_version, "x07.studio.release_request@0.1.0");
        assert_eq!(request.rung, "shareable");
        assert_eq!(request.environment, "shareable");
        assert!(request.binding_refs.is_empty());
    }
}
