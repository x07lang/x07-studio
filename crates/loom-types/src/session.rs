use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::artifacts::{IntentPacket, OpRecord, TaskType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Room {
    Intent,
    Spec,
    Realization,
    Verify,
    Repair,
    Trust,
    Ops,
    Providers,
    Mcp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionPhase {
    IntentDrafting,
    IntentReady,
    SpecDraft,
    SpecReview,
    SpecApproved,
    RealizationProposed,
    VerifyRunning,
    RepairEligible,
    TrustReview,
    CertifyRunning,
    Certified,
    IncidentIngesting,
    HumanInterventionRequired,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AllowedVerb {
    IntentFormalize,
    IntentReview,
    SpecEdit,
    SpecCheck,
    SpecApprove,
    ImplSync,
    ImplReview,
    VerifyRun,
    RepairRun,
    RepairSuggestSpecPatch,
    TrustReview,
    CertifyRun,
    IncidentIngest,
    ImproveRun,
    ProviderProbe,
    McpConnect,
    McpCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalDoctrine {
    pub mcp_tools: Vec<String>,
    pub doc_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WritePolicy {
    pub agent_write_specs: bool,
    pub agent_write_arch: bool,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDoctrine {
    pub xtal_manifest: String,
    pub agent_md: String,
    pub write_policy: WritePolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskDoctrine {
    pub intent_ref: Option<String>,
    pub focus_paths: Vec<String>,
    pub baseline_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionContract {
    pub schema_version: String,
    pub session_id: Uuid,
    pub workspace_root: String,
    pub global_doctrine: GlobalDoctrine,
    pub project_doctrine: ProjectDoctrine,
    pub task_doctrine: TaskDoctrine,
    pub allowed_verbs: Vec<AllowedVerb>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSnapshot {
    pub schema_version: String,
    pub session_id: Uuid,
    pub title: String,
    pub root: String,
    pub task_type: TaskType,
    pub room: Room,
    pub phase: SessionPhase,
    pub intent: Option<IntentPacket>,
    pub contract: Option<SessionContract>,
    pub allowed_verbs: Vec<AllowedVerb>,
    pub op_log: Vec<OpRecord>,
}

impl SessionSnapshot {
    pub fn new(
        session_id: Uuid,
        title: impl Into<String>,
        root: impl Into<String>,
        task_type: TaskType,
    ) -> Self {
        Self {
            schema_version: "x07.studio.session_snapshot@0.1.0".to_string(),
            session_id,
            title: title.into(),
            root: root.into(),
            task_type,
            room: Room::Intent,
            phase: SessionPhase::IntentDrafting,
            intent: None,
            contract: None,
            allowed_verbs: vec![AllowedVerb::IntentFormalize],
            op_log: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceSummary {
    pub workspace_root: String,
    pub sessions: Vec<SessionSnapshot>,
}
