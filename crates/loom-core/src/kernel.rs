use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::fs;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context};
use base64::Engine;
use camino::{Utf8Path, Utf8PathBuf};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::event_bus::SessionEventBus;
use crate::genpack::GenpackHandoffContext;
use loom_types::api::SessionStreamEvent;

use loom_adapters::command_runner::{
    now_string, CommandExecution, CommandRunner, CommandStreamUpdate,
};
use loom_adapters::mcp::{boxed_client, McpClient};
use loom_adapters::providers::{ProviderIntentPolishRequest, ProviderProber};
use loom_adapters::x07_cli::{validate_xtal_verify_vars, CliAdapter, ExecutedBinding, InputSpec};
use loom_store::FsStore;
use loom_types::api::{AgentRunMode, ApprovalDecision, IntentInputMode};
use loom_types::api::{
    AnswerCitation, ArtifactPreviewResponse, AskAnswer, AskRequest, CassetteEntry, DocPreviewEntry,
    DocPreviewResponse, LadderState, PatchsetPreview, PatchsetTargetPreview, ProofCitation,
    QuorumAgent, QuorumDiff, QuorumRound, SessionTurn, StudioMemory, SyncCode, TryItInputKind,
    TryItRequest, TryItResult, TurnQuestion, VisualResponse, WorkspacePathState,
    WorkspaceRadarResponse,
};
use loom_types::artifacts::{
    AgentHandoff, AgentProfile, AgentStatus, IntentPacket, IntentSource, IntentTarget, OpRecord,
    OperationStatus, ProviderProbeReport, ProviderProfile, TaskType, Witness, WitnessKind,
};
use loom_types::mcp::{McpConnectionInfo, McpEndpoint, McpToolCallResult, McpToolDescriptor};
use loom_types::ops::SessionEvent;
use loom_types::session::{Room, SessionPhase, SessionSnapshot};
use tokio::sync::mpsc;

use crate::workspace::WorkspaceModel;

pub struct WorkspaceKernel {
    root: Utf8PathBuf,
    model: WorkspaceModel,
    store: FsStore,
    cli: CliAdapter,
    providers: ProviderProber,
    mcp_connections: HashMap<String, Box<dyn McpClient>>,
    sync_codes: HashMap<String, (Uuid, String)>,
    event_bus: Arc<SessionEventBus>,
}

#[derive(Debug, Clone)]
pub struct PreparedAgentRun {
    pub handoff: AgentHandoff,
    pub op: OpRecord,
    pub session: SessionSnapshot,
    pub command: Option<AgentCommandPlan>,
    pub clarify_round: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AgentCommandPlan {
    pub session_id: Uuid,
    pub op_id: Uuid,
    pub agent: AgentProfile,
    pub handoff: AgentHandoff,
    pub prompt_path: Utf8PathBuf,
    pub cwd: Utf8PathBuf,
    pub program: String,
    pub args: Vec<String>,
    pub timeout_seconds: u64,
    /// `run` for a regular supervised handoff, `clarify` for an intent
    /// clarification round. Feeds the OpRecord op name so the browser can
    /// distinguish lanes without parsing prompt content.
    pub op_kind: String,
}

impl WorkspaceKernel {
    pub fn open(root: impl Into<Utf8PathBuf>) -> anyhow::Result<Self> {
        let root = root.into();
        let store = FsStore::new(root.as_path());
        store.init()?;
        let sessions = store.load_sessions()?;
        let mut model = WorkspaceModel::from_sessions(root.to_string(), sessions);
        if model.sessions.is_empty() {
            let session_id = model.create_session("New session", TaskType::NewBehavior);
            let snapshot = model
                .get_session(session_id)
                .cloned()
                .context("new session should exist")?;
            store.save_session(&snapshot)?;
        }

        let cli = CliAdapter::new(root.as_path(), store.reports_dir());
        let sync_codes = store
            .load_sync_codes()?
            .into_iter()
            .filter(|code| !sync_code_is_expired(&code.expires_at))
            .map(|code| {
                (
                    code.code.to_ascii_uppercase(),
                    (code.session_id, code.expires_at),
                )
            })
            .collect();
        Ok(Self {
            root,
            model,
            store,
            cli,
            providers: ProviderProber::default(),
            mcp_connections: HashMap::new(),
            sync_codes,
            event_bus: Arc::new(SessionEventBus::new()),
        })
    }

    /// Shared handle to the per-session broadcast hub. The daemon's SSE
    /// handler clones this Arc and subscribes per request without locking the
    /// kernel for the lifetime of the stream.
    pub fn event_bus(&self) -> Arc<SessionEventBus> {
        self.event_bus.clone()
    }

    /// Wraps `self.model.dispatch` so the event bus stays in sync with every
    /// state transition. AppendOp / UpdateOp are published as granular `Op`
    /// events (browser dedupes by op.id); everything else publishes a full
    /// `Snapshot` so phase / room / intent / contract changes are visible.
    fn dispatch_with_publish(
        &mut self,
        session_id: Uuid,
        event: SessionEvent,
    ) -> anyhow::Result<SessionSnapshot> {
        let op_to_publish = match &event {
            SessionEvent::AppendOp(op) | SessionEvent::UpdateOp(op) => Some(op.as_ref().clone()),
            _ => None,
        };
        let snapshot = self
            .model
            .dispatch(session_id, event)
            .map_err(|error| anyhow!(error.to_string()))?;
        let stream_event = match op_to_publish {
            Some(op) => SessionStreamEvent::Op { op: Box::new(op) },
            None => SessionStreamEvent::Snapshot {
                session: Box::new(snapshot.clone()),
            },
        };
        self.event_bus.publish(session_id, stream_event);
        Ok(snapshot)
    }

    pub fn workspace_root(&self) -> &Utf8Path {
        self.root.as_path()
    }

    pub fn workspace_radar(&self) -> WorkspaceRadarResponse {
        WorkspaceRadarResponse {
            schema_version: "x07.studio.workspace_radar@0.1.0".to_string(),
            workspace_root: self.root.to_string(),
            xtal_manifest: workspace_path_state(self.root.as_path(), "arch/xtal/xtal.json"),
            spec_count: count_files_matching(self.root.join("spec").as_path(), |path| {
                path.extension() == Some("json") && path.as_str().contains(".x07spec")
            }),
            generated_tests: workspace_path_state(self.root.as_path(), "gen/xtal/tests.json"),
            latest_verify: newest_workspace_file(self.root.as_path(), "target/xtal/verify"),
            latest_certify: newest_workspace_file(self.root.as_path(), "target/xtal/cert"),
            incident_count: self
                .model
                .session_list()
                .into_iter()
                .filter(|session| session.task_type == TaskType::IncidentRepair)
                .count()
                + count_files_matching(self.root.join("target/xtal/violations").as_path(), |_| {
                    true
                })
                + count_files_matching(self.root.join("target/xtal/ingest").as_path(), |_| true),
        }
    }

    pub fn list_bindings(&self) -> Vec<loom_types::api::BindingDescriptor> {
        loom_adapters::x07_cli::CliAdapter::list_bindings()
            .into_iter()
            .map(|item| loom_types::api::BindingDescriptor {
                id: item.id.to_string(),
                category: item.category.to_string(),
                program: item.program.to_string(),
                notes: item.notes.to_string(),
            })
            .collect()
    }

    pub fn list_sessions(&self) -> Vec<SessionSnapshot> {
        self.model.session_list()
    }

    pub fn get_session(&self, session_id: Uuid) -> Option<SessionSnapshot> {
        self.model.get_session(session_id).cloned()
    }

    pub fn session_turns(&self, session_id: Uuid) -> anyhow::Result<Vec<SessionTurn>> {
        let session = self
            .model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        Ok(crate::timeline::project_session_turns(session))
    }

    pub async fn invoke_artifact(
        &mut self,
        session_id: Uuid,
        req: TryItRequest,
    ) -> anyhow::Result<TryItResult> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        // The artifact only has a verified entrypoint once the session has
        // reached TrustReview / Certified. Calling `x07 run` before that
        // produces an empty 200 that the UI can't render usefully. Surface
        // a structured "not yet" payload so the Try-It panel can prompt the
        // user to finish the build first.
        if !matches!(
            session.phase,
            SessionPhase::TrustReview | SessionPhase::CertifyRunning | SessionPhase::Certified
        ) {
            let phase_label = format!("{:?}", session.phase);
            let stats = serde_json::json!({
                "phase": phase_label,
                "blocked_on": "verified",
                "message": "Try-It runs the verified artifact. Approve the spec and finish the build first.",
            });
            return Ok(TryItResult {
                output_kind: "not_verified".to_string(),
                output_text: Some(
                    "I can't run this yet — the build hasn't reached verified.\n\n\
                     Approve the spec and click Build to produce a verified artifact, \
                     then try this input again."
                        .to_string(),
                ),
                output_json: None,
                stats,
                proof_citations: Vec::new(),
                op_id: Uuid::nil(),
            });
        }
        // Verified, but the on-disk implementation is still a stub. Running
        // `x07 run` would succeed but produce no useful output. Refuse with
        // a clear hint pointing the user at the realize CTA.
        let stub_paths = crate::summarize::scan_stub_modules(self.root.as_path());
        if !stub_paths.is_empty() {
            let stats = serde_json::json!({
                "phase": format!("{:?}", session.phase),
                "blocked_on": "implementation",
                "stub_paths": stub_paths,
                "message": "The build only scaffolded an empty implementation. Ask Claude Code to fill it in before running.",
            });
            return Ok(TryItResult {
                output_kind: "stub_impl".to_string(),
                output_text: Some(format!(
                    "This project is scaffolded but the implementation is still a stub.\n\
                     {} module{} need real code:\n  {}\n\nClick \"Implement with Claude Code\" \
                     on the Verified turn, then come back and try again.",
                    stub_paths.len(),
                    if stub_paths.len() == 1 { "" } else { "s" },
                    stub_paths.join("\n  ")
                )),
                output_json: None,
                stats,
                proof_citations: Vec::new(),
                op_id: Uuid::nil(),
            });
        }
        let input = match req.input_kind {
            TryItInputKind::Text => InputSpec::Text(req.input_text.unwrap_or_default()),
            TryItInputKind::B64 => {
                let raw = req.input_b64.unwrap_or_default();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(raw.as_bytes())
                    .context("try-it input_b64 is not valid base64")?;
                InputSpec::Bytes(bytes)
            }
            TryItInputKind::File => {
                let path = req
                    .input_path
                    .ok_or_else(|| anyhow!("file Try-It request requires input_path"))?;
                validate_relative_runtime_path(&path, "try-it input_path")?;
                InputSpec::File(Utf8PathBuf::from(path))
            }
            TryItInputKind::Argv => InputSpec::Argv(req.argv),
        };
        let executed = self
            .cli
            .run_invoke(Utf8Path::new("x07.json"), req.profile.as_deref(), input)
            .await?;
        let op = op_record_from_binding(session_id, "run.invoke", executed);
        let op_id = op.id;
        let output_json = op.stdout_json.clone().or_else(|| op.report_json.clone());
        let output_text = op.stdout.clone();
        let stats = serde_json::json!({
            "exit_code": op.exit_code,
            "status": op.status,
            "report_path": op.report_path,
        });
        let citations = proof_citations_for_session(
            self.model
                .get_session(session_id)
                .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?,
        );
        self.append_op(session_id, op)?;
        Ok(TryItResult {
            output_kind: if output_json.is_some() {
                "json".to_string()
            } else {
                "text".to_string()
            },
            output_text,
            output_json,
            stats,
            proof_citations: citations,
            op_id,
        })
    }

    pub fn ladder_state(&self, session_id: Uuid) -> anyhow::Result<LadderState> {
        let session = self
            .model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        Ok(crate::ladder::ladder_state(self.root.as_path(), session))
    }

    pub async fn climb_rung(
        &mut self,
        session_id: Uuid,
        to_rung: &str,
    ) -> anyhow::Result<SessionSnapshot> {
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let profile = crate::ladder::rung_profile_path(to_rung).unwrap_or("sandbox");
        self.run_binding(
            session_id,
            if to_rung == "local_preview" {
                "trust.report.sandbox"
            } else {
                "trust.certify.profile"
            },
            &BTreeMap::from([("profile".to_string(), profile.to_string())]),
        )
        .await
    }

    pub fn ingest_incidents(&mut self, session_id: Uuid) -> anyhow::Result<Vec<OpRecord>> {
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let mut recorded = Vec::new();
        let existing = self
            .model
            .get_session(session_id)
            .map(|session| {
                session
                    .op_log
                    .iter()
                    .filter_map(|op| {
                        op.report_json
                            .as_ref()
                            .and_then(|value| value.get("incident_id"))
                            .and_then(serde_json::Value::as_str)
                            .map(ToOwned::to_owned)
                    })
                    .collect::<BTreeSet<_>>()
            })
            .unwrap_or_default();
        for incident in crate::incidents::scan_workspace_incidents(self.root.as_path()) {
            if existing.contains(&incident.id) {
                continue;
            }
            let op = incident_detected_op(session_id, &incident);
            self.append_op(session_id, op.clone())?;
            recorded.push(op);
        }
        Ok(recorded)
    }

    pub async fn repair_incident(
        &mut self,
        session_id: Uuid,
        incident_id: &str,
    ) -> anyhow::Result<SessionSnapshot> {
        let incident = crate::incidents::scan_workspace_incidents(self.root.as_path())
            .into_iter()
            .find(|candidate| candidate.id == incident_id)
            .ok_or_else(|| anyhow!("unknown incident `{incident_id}`"))?;
        let mut vars = BTreeMap::new();
        vars.insert("input".to_string(), incident.root_path);
        self.run_binding(session_id, "xtal.improve", &vars).await
    }

    pub fn prepare_intent_quorum_with_genpack(
        &mut self,
        session_id: Uuid,
        agent_ids: &[String],
        timeout_seconds: Option<u64>,
        genpack: Option<&GenpackHandoffContext>,
    ) -> anyhow::Result<Vec<PreparedAgentRun>> {
        let mut prepared = Vec::new();
        for agent_id in agent_ids
            .iter()
            .map(|id| id.trim())
            .filter(|id| !id.is_empty())
            .take(3)
        {
            prepared.push(self.start_intent_clarify_with_genpack(
                session_id,
                agent_id,
                timeout_seconds,
                genpack,
            )?);
        }
        if prepared.is_empty() {
            bail!("intent quorum requires at least one agent");
        }
        Ok(prepared)
    }

    pub fn ingest_clarify_questions_at_round(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        run_op_id: Uuid,
        round: u32,
    ) -> anyhow::Result<SessionSnapshot> {
        self.ingest_clarify_questions_inner(session_id, agent_id, run_op_id, Some(round))
    }

    pub fn complete_intent_quorum(
        &mut self,
        session_id: Uuid,
        round: u32,
        agent_ids: &[String],
    ) -> anyhow::Result<QuorumRound> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let intent = session
            .intent
            .clone()
            .ok_or_else(|| anyhow!("session `{session_id}` has no intent"))?;
        let requested = agent_ids
            .iter()
            .map(|id| id.trim().to_string())
            .filter(|id| !id.is_empty())
            .take(3)
            .collect::<Vec<_>>();
        let agents = requested
            .iter()
            .map(|agent_id| {
                let questions = intent
                    .clarification_history
                    .iter()
                    .filter(|turn| turn.round == round && turn.agent_id == *agent_id)
                    .map(|turn| TurnQuestion {
                        id: turn.question_id.clone(),
                        text: turn.question_text.clone(),
                        witness_kind: turn.witness_kind.clone(),
                        options: turn.options.clone(),
                        answer: turn.answer_text.clone(),
                    })
                    .collect::<Vec<_>>();
                QuorumAgent {
                    agent_id: agent_id.clone(),
                    questions,
                }
            })
            .collect::<Vec<_>>();
        let total_questions = agents
            .iter()
            .map(|agent| agent.questions.len())
            .sum::<usize>();
        let diff = vec![QuorumDiff {
            label: "Live agent coverage".to_string(),
            detail: format!(
                "{} agent{} returned {} question{} in quorum round {round}.",
                agents.len(),
                if agents.len() == 1 { "" } else { "s" },
                total_questions,
                if total_questions == 1 { "" } else { "s" },
            ),
        }];
        let op = quorum_op(session_id, round, &agents);
        self.append_op(session_id, op)?;
        Ok(QuorumRound {
            round,
            agents,
            diff,
        })
    }

    pub fn cassette_entries(&self, session_id: Uuid) -> anyhow::Result<Vec<CassetteEntry>> {
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        Ok(cassette_entries_from_workspace(self.root.as_path()))
    }

    pub fn branch_from_cassette(
        &mut self,
        session_id: Uuid,
        from_entry: u32,
        title: &str,
    ) -> anyhow::Result<Uuid> {
        let source = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let entries = cassette_entries_from_workspace(self.root.as_path());
        let selected = entries
            .iter()
            .find(|entry| entry.idx == from_entry)
            .ok_or_else(|| anyhow!("unknown cassette entry `{from_entry}`"))?;
        let replayed = entries
            .iter()
            .filter(|entry| entry.idx <= from_entry)
            .cloned()
            .collect::<Vec<_>>();
        let truncated = entries
            .iter()
            .filter(|entry| entry.idx > from_entry)
            .cloned()
            .collect::<Vec<_>>();
        let branch_session_id = Uuid::new_v4();
        let replay_manifest = materialize_cassette_branch(
            self.root.as_path(),
            branch_session_id,
            session_id,
            from_entry,
            &replayed,
            &truncated,
        )?;
        let mut branch = source.clone();
        branch.session_id = branch_session_id;
        branch.title = if title.trim().is_empty() {
            format!("Cassette branch {from_entry}")
        } else {
            title.to_string()
        };
        branch.task_type = TaskType::BehaviorChange;
        branch.root = self.root.to_string();
        branch.op_log = truncate_ops_for_cassette(&source.op_log, selected);
        let op = cassette_branch_op(
            branch_session_id,
            session_id,
            from_entry,
            &replayed,
            &truncated,
            replay_manifest,
        );
        branch.op_log.push(op);
        self.model.load_session(branch.clone());
        self.store.save_session(&branch)?;
        Ok(branch_session_id)
    }

    pub fn ask_project(&self, session_id: Uuid, req: AskRequest) -> anyhow::Result<AskAnswer> {
        let session = self
            .model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        Ok(answer_project_question(self.root.as_path(), session, &req))
    }

    pub fn mint_sync_code(&mut self, session_id: Uuid) -> anyhow::Result<SyncCode> {
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let code = Uuid::new_v4()
            .simple()
            .to_string()
            .chars()
            .take(8)
            .collect::<String>()
            .to_ascii_uppercase();
        let expires_at = sync_expires_at();
        self.sync_codes
            .insert(code.clone(), (session_id, expires_at.clone()));
        self.save_sync_codes_state()?;
        Ok(SyncCode {
            code,
            expires_at,
            session_id,
        })
    }

    pub fn claim_sync_code(&mut self, code: &str) -> anyhow::Result<SessionSnapshot> {
        self.prune_expired_sync_codes()?;
        let normalized = code.trim().to_ascii_uppercase();
        let (session_id, _) = self
            .sync_codes
            .get(&normalized)
            .ok_or_else(|| anyhow!("unknown sync code `{code}`"))?;
        self.model
            .get_session(*session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))
    }

    fn prune_expired_sync_codes(&mut self) -> anyhow::Result<()> {
        let before = self.sync_codes.len();
        self.sync_codes
            .retain(|_, (_, expires_at)| !sync_code_is_expired(expires_at));
        if self.sync_codes.len() != before {
            self.save_sync_codes_state()?;
        }
        Ok(())
    }

    fn save_sync_codes_state(&self) -> anyhow::Result<()> {
        let mut codes = self
            .sync_codes
            .iter()
            .map(|(code, (session_id, expires_at))| SyncCode {
                code: code.clone(),
                expires_at: expires_at.clone(),
                session_id: *session_id,
            })
            .collect::<Vec<_>>();
        codes.sort_by(|left, right| left.code.cmp(&right.code));
        self.store.save_sync_codes(&codes)
    }

    pub fn load_memory(&self) -> anyhow::Result<StudioMemory> {
        self.store.load_memory()
    }

    pub fn save_memory(&self, memory: &StudioMemory) -> anyhow::Result<StudioMemory> {
        self.store.save_memory(memory)?;
        Ok(memory.clone())
    }

    pub fn save_intent_image(
        &self,
        session_id: Uuid,
        mime: &str,
        bytes: &[u8],
    ) -> anyhow::Result<String> {
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        self.store.save_intent_image(session_id, mime, bytes)
    }

    pub fn visual_parse(
        &self,
        kind: &str,
        source: serde_json::Value,
    ) -> anyhow::Result<VisualResponse> {
        Ok(VisualResponse {
            schema_version: "x07.studio.visual@0.1.0".to_string(),
            kind: kind.to_string(),
            value: visual_parse_value(kind, source),
        })
    }

    pub fn visual_emit(
        &self,
        kind: &str,
        graph: serde_json::Value,
    ) -> anyhow::Result<VisualResponse> {
        Ok(VisualResponse {
            schema_version: "x07.studio.visual@0.1.0".to_string(),
            kind: kind.to_string(),
            value: visual_emit_value(kind, graph),
        })
    }

    pub fn preview_artifact(
        &self,
        session_id: Uuid,
        artifact: &str,
    ) -> anyhow::Result<ArtifactPreviewResponse> {
        const MAX_PREVIEW_BYTES: u64 = 128 * 1024;
        let session = self
            .model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if !session_artifact_recorded(session, artifact) {
            return Err(anyhow!(
                "artifact `{artifact}` is not recorded on session `{session_id}`"
            ));
        }
        let artifact_path = safe_artifact_path(self.root.as_path(), artifact)?;
        if !artifact_path.is_file() {
            return Err(anyhow!("artifact `{artifact}` is not a readable file"));
        }
        let size = fs::metadata(&artifact_path)
            .with_context(|| format!("metadata: {artifact_path}"))?
            .len();
        let mut file =
            fs::File::open(&artifact_path).with_context(|| format!("open: {artifact_path}"))?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_PREVIEW_BYTES + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read: {artifact_path}"))?;
        let truncated = size > MAX_PREVIEW_BYTES || bytes.len() as u64 > MAX_PREVIEW_BYTES;
        if truncated {
            bytes.truncate(MAX_PREVIEW_BYTES as usize);
        }
        let text = String::from_utf8(bytes).ok();
        let json = text
            .as_deref()
            .and_then(|body| serde_json::from_str::<serde_json::Value>(body).ok());
        let patchset_preview = json
            .as_ref()
            .and_then(|value| self.preview_patchset_targets(value));
        let media_kind = if json.is_some() {
            "json"
        } else if text.is_some() {
            "text"
        } else {
            "binary"
        };
        Ok(ArtifactPreviewResponse {
            schema_version: "x07.studio.artifact_preview@0.1.0".to_string(),
            artifact: artifact.to_string(),
            media_kind: media_kind.to_string(),
            bytes_read: size.min(MAX_PREVIEW_BYTES),
            truncated,
            text,
            json,
            patchset_preview,
        })
    }

    pub fn preview_doc(
        &self,
        session_id: Uuid,
        doc_ref: &str,
    ) -> anyhow::Result<DocPreviewResponse> {
        const MAX_DOC_PREVIEW_BYTES: u64 = 16 * 1024;
        const MAX_DOC_ENTRIES: usize = 12;
        self.model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let normalized = normalize_doc_ref(doc_ref)?;
        let docs_root = find_docs_root(self.root.as_path())
            .ok_or_else(|| anyhow!("x07 docs root was not found"))?;
        let rel = normalized
            .strip_prefix("x07/docs/")
            .ok_or_else(|| anyhow!("doc ref `{normalized}` must start with `x07/docs/`"))?;
        let target = safe_doc_ref_path(docs_root.as_path(), rel)?;

        if target.is_dir() {
            let (entries, truncated) =
                doc_directory_entries(target.as_path(), &normalized, MAX_DOC_ENTRIES)?;
            let snippet = if entries.is_empty() {
                "No previewable docs found in this directory.".to_string()
            } else {
                entries
                    .iter()
                    .map(|entry| format!("{} ({})", entry.path, entry.kind))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            return Ok(DocPreviewResponse {
                schema_version: "x07.studio.doc_preview@0.1.0".to_string(),
                doc_ref: normalized,
                resolved_path: target.to_string(),
                title: doc_title_from_ref(&target, doc_ref),
                media_kind: "directory".to_string(),
                bytes_read: 0,
                truncated,
                snippet,
                entries,
            });
        }

        if !target.is_file() {
            return Err(anyhow!(
                "doc ref `{normalized}` is not a readable file or directory"
            ));
        }

        let size = fs::metadata(&target)
            .with_context(|| format!("metadata: {target}"))?
            .len();
        let mut file = fs::File::open(&target).with_context(|| format!("open: {target}"))?;
        let mut bytes = Vec::new();
        file.by_ref()
            .take(MAX_DOC_PREVIEW_BYTES + 1)
            .read_to_end(&mut bytes)
            .with_context(|| format!("read: {target}"))?;
        let truncated = size > MAX_DOC_PREVIEW_BYTES || bytes.len() as u64 > MAX_DOC_PREVIEW_BYTES;
        if truncated {
            bytes.truncate(MAX_DOC_PREVIEW_BYTES as usize);
        }
        let text = String::from_utf8(bytes)
            .with_context(|| format!("doc ref `{normalized}` is not UTF-8 text"))?;
        let title = markdown_title(&text).unwrap_or_else(|| doc_title_from_ref(&target, doc_ref));
        Ok(DocPreviewResponse {
            schema_version: "x07.studio.doc_preview@0.1.0".to_string(),
            doc_ref: normalized,
            resolved_path: target.to_string(),
            title,
            media_kind: doc_media_kind(target.as_path()).to_string(),
            bytes_read: size.min(MAX_DOC_PREVIEW_BYTES),
            truncated,
            snippet: doc_snippet(&text),
            entries: Vec::new(),
        })
    }

    fn preview_patchset_targets(&self, patchset: &serde_json::Value) -> Option<PatchsetPreview> {
        let schema = patchset
            .get("schema_version")
            .and_then(|value| value.as_str())?;
        if schema != "x07.patchset@0.1.0" && schema != "x07.arch.patchset@0.1.0" {
            return None;
        }
        let targets = patchset
            .get("patches")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(8)
            .map(|target| self.preview_patchset_target(target))
            .collect();
        Some(PatchsetPreview {
            schema_version: "x07.studio.patchset_preview@0.1.0".to_string(),
            targets,
        })
    }

    fn preview_patchset_target(&self, target: serde_json::Value) -> PatchsetTargetPreview {
        const MAX_TARGET_PREVIEW_BYTES: u64 = 128 * 1024;
        let path = target
            .get("path")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let note = target
            .get("note")
            .and_then(|value| value.as_str())
            .map(ToString::to_string);
        let patch = target
            .get("patch")
            .and_then(|value| value.as_array())
            .cloned()
            .unwrap_or_default();
        let mut output = PatchsetTargetPreview {
            path: path.clone(),
            note,
            operations: patch.len(),
            before_json: None,
            after_json: None,
            apply_error: None,
            truncated: false,
        };
        let target_path = match safe_artifact_path(self.root.as_path(), &path) {
            Ok(path) => path,
            Err(error) => {
                output.apply_error = Some(error.to_string());
                return output;
            }
        };
        if !is_reviewable_patchset_target(&path) {
            output.apply_error =
                Some("target preview path is outside reviewable project surfaces".to_string());
            return output;
        }
        let metadata = match fs::metadata(&target_path) {
            Ok(metadata) => metadata,
            Err(error) => {
                output.apply_error = Some(format!("target unavailable: {error}"));
                return output;
            }
        };
        if !metadata.is_file() {
            output.apply_error = Some("target is not a readable file".to_string());
            return output;
        }
        if metadata.len() > MAX_TARGET_PREVIEW_BYTES {
            output.truncated = true;
            output.apply_error = Some(format!(
                "target is larger than {} bytes",
                MAX_TARGET_PREVIEW_BYTES
            ));
            return output;
        }
        let before = match fs::read_to_string(&target_path) {
            Ok(text) => text,
            Err(error) => {
                output.apply_error = Some(format!("read target failed: {error}"));
                return output;
            }
        };
        let before_json = match serde_json::from_str::<serde_json::Value>(&before) {
            Ok(value) => value,
            Err(error) => {
                output.apply_error = Some(format!("target is not JSON: {error}"));
                return output;
            }
        };
        let mut after_json = before_json.clone();
        match apply_json_patch_preview(&mut after_json, &patch) {
            Ok(()) => output.after_json = Some(after_json),
            Err(error) => output.apply_error = Some(error.to_string()),
        }
        output.before_json = Some(before_json);
        output
    }

    pub fn create_session(
        &mut self,
        title: impl Into<String>,
        task_type: TaskType,
    ) -> anyhow::Result<SessionSnapshot> {
        let session_id = self.model.create_session(title, task_type);
        let snapshot = self
            .model
            .get_session(session_id)
            .cloned()
            .context("new session should exist")?;
        self.store.save_session(&snapshot)?;
        // Auto-track this project in cross-project memory so the UI can
        // surface "applied from your history" without an explicit POST.
        let _ = self.track_project_in_memory(&snapshot);
        Ok(snapshot)
    }

    /// Append a `MemoryProject` entry for this session if none exists for the
    /// current workspace root. Idempotent: each subsequent session under the
    /// same root just updates `last_session_id`.
    fn track_project_in_memory(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()> {
        let mut memory = self.store.load_memory()?;
        let root = self.root.to_string();
        if let Some(existing) = memory
            .recent_projects
            .iter_mut()
            .find(|project| project.root == root)
        {
            existing.last_session_id = Some(snapshot.session_id);
            if existing.label.trim().is_empty() {
                existing.label = snapshot.title.clone();
            }
        } else {
            memory.recent_projects.push(loom_types::api::MemoryProject {
                root,
                last_session_id: Some(snapshot.session_id),
                label: snapshot.title.clone(),
            });
        }
        // Cap to a reasonable recent-window so the JSONL doesn't grow forever.
        const MAX_RECENT: usize = 20;
        if memory.recent_projects.len() > MAX_RECENT {
            let drop = memory.recent_projects.len() - MAX_RECENT;
            memory.recent_projects.drain(..drop);
        }
        self.store.save_memory(&memory)?;
        Ok(())
    }

    pub fn dispatch_event(
        &mut self,
        session_id: Uuid,
        event: SessionEvent,
    ) -> anyhow::Result<SessionSnapshot> {
        let snapshot = self.dispatch_with_publish(session_id, event)?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    pub fn formalize_intent(
        &mut self,
        session_id: Uuid,
        raw: &str,
        input_mode: IntentInputMode,
        revision_notes: &[String],
    ) -> anyhow::Result<(IntentPacket, OpRecord, SessionSnapshot)> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let intent = intent_packet_from_raw(&session, raw, input_mode.clone(), revision_notes);
        if input_mode == IntentInputMode::Incident {
            persist_manual_incident_bundle(self.root.as_path(), &intent)?;
        }
        self.dispatch_with_publish(
            session_id,
            SessionEvent::FormalizeIntent(Box::new(intent.clone())),
        )?;
        if let Some(session) = self.model.get_session_mut(session_id) {
            session.revision_notes = revision_notes.to_vec();
        }
        let op = intent_formalize_op(session_id, &intent, input_mode, revision_notes, None);
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op.clone())))?;
        self.store.save_session(&snapshot)?;
        Ok((intent, op, snapshot))
    }

    pub fn request_intent_revision(
        &mut self,
        session_id: Uuid,
        note: &str,
    ) -> anyhow::Result<(OpRecord, SessionSnapshot)> {
        let note = note.trim();
        if note.is_empty() {
            bail!("revision note cannot be empty");
        }
        let revision_index = {
            let session = self
                .model
                .get_session_mut(session_id)
                .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
            session.revision_notes.push(note.to_string());
            session.room = Room::Intent;
            session.revision_notes.len()
        };
        let op = intent_revision_request_op(session_id, note, revision_index);
        let snapshot = self.append_op(session_id, op.clone())?;
        Ok((op, snapshot))
    }

    pub async fn formalize_intent_with_provider(
        &mut self,
        session_id: Uuid,
        raw: &str,
        input_mode: IntentInputMode,
        revision_notes: &[String],
        provider_profile_id: Option<&str>,
    ) -> anyhow::Result<(IntentPacket, OpRecord, SessionSnapshot)> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let mut intent = intent_packet_from_raw(&session, raw, input_mode.clone(), revision_notes);
        let provider_report = match provider_profile_id.and_then(|id| non_empty(id)) {
            Some(profile_id) => {
                let profile = self.provider_profile_by_id(profile_id);
                let providers = self.providers.clone();
                Some(
                    apply_provider_intent_polish(
                        &providers,
                        profile,
                        &mut intent,
                        raw,
                        &input_mode,
                        revision_notes,
                        profile_id,
                    )
                    .await,
                )
            }
            None => None,
        };
        if input_mode == IntentInputMode::Incident {
            persist_manual_incident_bundle(self.root.as_path(), &intent)?;
        }
        self.dispatch_with_publish(
            session_id,
            SessionEvent::FormalizeIntent(Box::new(intent.clone())),
        )?;
        if let Some(session) = self.model.get_session_mut(session_id) {
            session.revision_notes = revision_notes.to_vec();
        }
        let op = intent_formalize_op(
            session_id,
            &intent,
            input_mode,
            revision_notes,
            provider_report,
        );
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op.clone())))?;
        self.store.save_session(&snapshot)?;
        Ok((intent, op, snapshot))
    }

    fn provider_profile_by_id(&self, provider_profile_id: &str) -> Option<ProviderProfile> {
        self.store
            .load_provider_profiles()
            .ok()?
            .into_iter()
            .find(|profile| profile.id == provider_profile_id)
            .or_else(|| {
                let profile = ProviderProfile::local_ollama();
                (profile.id == provider_profile_id).then_some(profile)
            })
    }

    pub async fn run_binding(
        &mut self,
        session_id: Uuid,
        binding_id: &str,
        vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<SessionSnapshot> {
        let executed = self.cli.execute(binding_id, vars).await?;
        let op = op_record_from_binding(session_id, binding_id, executed);

        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op)))?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    pub async fn run_xtal_workflow(&mut self, session_id: Uuid) -> anyhow::Result<SessionSnapshot> {
        self.run_xtal_workflow_with_vars(session_id, &BTreeMap::new())
            .await
    }

    /// Simple-mode orchestrator: runs the full XTAL chain to "verified" with
    /// plain-English stage markers and bounded auto-repair on verify failure.
    /// Stops at TrustReview (verified) so certification stays an explicit
    /// Expert-mode action.
    pub async fn run_build_pipeline(
        &mut self,
        session_id: Uuid,
        run_vars: &BTreeMap<String, String>,
        max_repair_rounds: u32,
    ) -> anyhow::Result<SessionSnapshot> {
        let max_repair_rounds = max_repair_rounds.clamp(0, 5);
        self.append_op(session_id, build_stage_op(session_id, "start", 0))?;
        // Simple-mode `/build` is the "make it work" path: pre-existing
        // workspaces commonly have `default_profile = "os"`, which makes
        // `xtal.verify` fail `EXTAL_VERIFY_WORLD_UNSAFE` unless we opt into
        // OS worlds explicitly. Default to allow_os_world=true when the
        // caller has not pinned it. Expert mode and the canonical
        // `/xtal/run` keep strict defaults.
        let mut build_vars = run_vars.clone();
        build_vars
            .entry("allow_os_world".to_string())
            .or_insert_with(|| "true".to_string());
        let snapshot = self
            .run_xtal_workflow_with_vars(session_id, &build_vars)
            .await?;
        let mut current = snapshot;
        let mut round: u32 = 0;
        let mut ensured_manifest = false;
        while current.phase == SessionPhase::RepairEligible && round < max_repair_rounds {
            round += 1;
            self.append_op(session_id, build_stage_op(session_id, "repair", round))?;
            // `xtal.repair --write` requires `arch/xtal/xtal.json`. xtal-pure
            // init doesn't create it, so the first repair-write would fail
            // EXTAL_REPAIR_WRITE_REQUIRES_MANIFEST. Materialize a minimal
            // manifest the first time we enter the repair loop.
            if !ensured_manifest {
                self.ensure_xtal_manifest_for_build(session_id)?;
                ensured_manifest = true;
            }
            let mut repair_vars = build_vars.clone();
            repair_vars.insert("repair_strategy".to_string(), "semantic_only".to_string());
            repair_vars.insert("repair_write".to_string(), "true".to_string());
            let after_repair = self
                .run_binding(session_id, "xtal.repair", &repair_vars)
                .await?;
            if last_op_failed(&after_repair) {
                current = after_repair;
                break;
            }
            self.dispatch_event(session_id, SessionEvent::RepairSpecPreserving)?;
            let after_verify = self
                .run_binding(session_id, "xtal.verify", &build_vars)
                .await?;
            let event = if last_op_failed(&after_verify) {
                SessionEvent::VerificationFailed
            } else {
                SessionEvent::VerificationPassed
            };
            current = self.dispatch_event(session_id, event)?;
        }
        let final_stage = match current.phase {
            SessionPhase::TrustReview | SessionPhase::CertifyRunning | SessionPhase::Certified => {
                "done"
            }
            _ => "needs_help",
        };
        self.append_op(session_id, build_stage_op(session_id, final_stage, round))?;
        if final_stage == "done" {
            let summary_op =
                match build_plain_english_summary_with_root(&current, Some(self.root.as_path())) {
                    Some(op) => op,
                    None => return Ok(current),
                };
            current = self.append_op(session_id, summary_op)?;
        }
        Ok(current)
    }

    pub async fn run_xtal_workflow_with_vars(
        &mut self,
        session_id: Uuid,
        run_vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<SessionSnapshot> {
        let verify_vars = checked_xtal_verify_run_vars(run_vars)?;
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if !matches!(
            session.phase,
            SessionPhase::SpecApproved | SessionPhase::RealizationProposed
        ) {
            return Err(anyhow!(
                "session `{session_id}` must have an approved spec before running XTAL workflow"
            ));
        }
        let intent = session
            .intent
            .as_ref()
            .ok_or_else(|| anyhow!("session `{session_id}` has no approved intent packet"))?;
        let template = workflow_template_from_intent(intent);
        let mut vars = xtal_workflow_vars_from_intent(intent);
        vars.extend(verify_vars);

        if matches!(intent.source, IntentSource::Incident { .. }) {
            return self
                .run_incident_improve_workflow(session_id, intent, &vars)
                .await;
        }

        if !self.root.join("x07.json").exists() {
            let snapshot = if let Some(example_path) = template.example_path() {
                self.seed_example_project(session_id, template, example_path)?
            } else {
                self.run_binding(session_id, "project.init.xtal-pure", &BTreeMap::new())
                    .await?
            };
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        }

        if template != WorkflowTemplate::XtalPure {
            return self
                .run_seeded_template_workflow(session_id, template, &vars)
                .await;
        }

        if should_scaffold_spec(self.root.as_path(), &vars) {
            let snapshot = self.run_binding(session_id, "spec.scaffold", &vars).await?;
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        } else {
            let input = vars
                .get("input")
                .cloned()
                .unwrap_or_else(|| "spec/app.main.x07spec.json".to_string());
            self.append_op(session_id, existing_spec_op(session_id, &input))?;
        }

        for binding_id in ["spec.check", "tests.gen.write"] {
            let snapshot = self.run_binding(session_id, binding_id, &vars).await?;
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        }

        let current = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if current.phase == SessionPhase::SpecApproved {
            self.dispatch_event(session_id, SessionEvent::ProposeRealization)?;
        }

        for binding_id in ["impl.sync.write", "impl.check"] {
            let snapshot = self.run_binding(session_id, binding_id, &vars).await?;
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        }

        let current = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if current.phase == SessionPhase::RealizationProposed {
            self.dispatch_event(session_id, SessionEvent::AcceptRealization)?;
        }

        let verified = self.run_binding(session_id, "xtal.verify", &vars).await?;
        let event = if last_op_failed(&verified) {
            SessionEvent::VerificationFailed
        } else {
            SessionEvent::VerificationPassed
        };
        self.dispatch_event(session_id, event)
    }

    async fn run_seeded_template_workflow(
        &mut self,
        session_id: Uuid,
        template: WorkflowTemplate,
        vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<SessionSnapshot> {
        self.ensure_verify_phase(session_id)?;
        for step in template.workflow_steps() {
            let binding_id = *step;
            if let Some(directory) = template.directory_for_step(binding_id) {
                let snapshot = self.ensure_workflow_directory(session_id, directory)?;
                if last_op_failed(&snapshot) {
                    return self.finish_verification(session_id, false);
                }
            }

            let mut step_vars = vars.clone();
            if let Some(stdin) = template.stdin_for_step(binding_id) {
                step_vars.insert("stdin".to_string(), stdin.to_string());
            }
            let snapshot = self.run_binding(session_id, binding_id, &step_vars).await?;
            if last_op_failed(&snapshot) {
                return self.finish_verification(session_id, false);
            }
        }
        if template == WorkflowTemplate::X07Atlas {
            return self.run_atlas_platform_delivery(session_id).await;
        }
        self.finish_verification(session_id, true)
    }

    async fn run_atlas_platform_delivery(
        &mut self,
        session_id: Uuid,
    ) -> anyhow::Result<SessionSnapshot> {
        let accept_vars = atlas_platform_delivery_vars(self.root.as_path(), None);
        let accepted = self
            .run_binding(session_id, "lp.deploy.accept.local", &accept_vars)
            .await?;
        if last_op_failed(&accepted) {
            return self.finish_verification(session_id, false);
        }

        let Some(deployment_id) = platform_deployment_id_from_snapshot(&accepted) else {
            let snapshot = self.append_op(session_id, platform_delivery_decode_op(session_id))?;
            if last_op_failed(&snapshot) {
                return self.finish_verification(session_id, false);
            }
            return self.finish_verification(session_id, false);
        };

        let delivery_vars =
            atlas_platform_delivery_vars(self.root.as_path(), Some(deployment_id.as_str()));
        for binding_id in WorkflowTemplate::X07Atlas
            .platform_delivery_steps()
            .iter()
            .copied()
            .filter(|step| *step != "lp.deploy.accept.local")
        {
            let snapshot = self
                .run_binding(session_id, binding_id, &delivery_vars)
                .await?;
            if last_op_failed(&snapshot) {
                return self.finish_verification(session_id, false);
            }
        }
        self.finish_verification(session_id, true)
    }

    async fn run_incident_improve_workflow(
        &mut self,
        session_id: Uuid,
        intent: &IntentPacket,
        vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<SessionSnapshot> {
        let incident_input = incident_input_path(intent)
            .ok_or_else(|| anyhow!("incident workflow has no incident input artifact"))?;

        if !self.root.join("x07.json").exists() {
            let snapshot = self
                .run_binding(session_id, "project.init.xtal-pure", &BTreeMap::new())
                .await?;
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        }

        let snapshot = self.ensure_incident_xtal_manifest(session_id, intent)?;
        if last_op_failed(&snapshot) {
            return Ok(snapshot);
        }

        self.dispatch_event(session_id, SessionEvent::IngestIncident)?;
        let mut incident_vars = vars.clone();
        incident_vars.insert("input".to_string(), incident_input.to_string());
        for binding_id in ["xtal.ingest", "xtal.improve"] {
            let current = self
                .run_binding(session_id, binding_id, &incident_vars)
                .await?;
            if last_op_failed(&current) {
                return self.dispatch_event(session_id, SessionEvent::MoveRoom(Room::Repair));
            }
        }
        self.dispatch_event(session_id, SessionEvent::MoveRoom(Room::Trust))
    }

    fn ensure_incident_xtal_manifest(
        &mut self,
        session_id: Uuid,
        intent: &IntentPacket,
    ) -> anyhow::Result<SessionSnapshot> {
        let path = self.root.join("arch/xtal/xtal.json");
        let op = if path.exists() {
            xtal_manifest_ensure_op(session_id, false, None)
        } else {
            match write_incident_xtal_manifest(path.as_path(), intent) {
                Ok(()) => xtal_manifest_ensure_op(session_id, true, None),
                Err(error) => xtal_manifest_ensure_op(session_id, true, Some(error)),
            }
        };
        self.append_op(session_id, op)
    }

    /// Build-pipeline counterpart to [`Self::ensure_incident_xtal_manifest`].
    /// The xtal-pure init does not create `arch/xtal/xtal.json`, but
    /// `xtal.repair --write` refuses to run without one. Materialize a
    /// minimal manifest scoped to the approved intent the first time we
    /// need to repair.
    fn ensure_xtal_manifest_for_build(
        &mut self,
        session_id: Uuid,
    ) -> anyhow::Result<SessionSnapshot> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let intent = session
            .intent
            .as_ref()
            .ok_or_else(|| anyhow!("session `{session_id}` has no approved intent packet"))?;
        let path = self.root.join("arch/xtal/xtal.json");
        let op = if path.exists() {
            xtal_manifest_ensure_op(session_id, false, None)
        } else {
            match write_build_xtal_manifest(path.as_path(), intent) {
                Ok(()) => xtal_manifest_ensure_op(session_id, true, None),
                Err(error) => xtal_manifest_ensure_op(session_id, true, Some(error)),
            }
        };
        self.append_op(session_id, op)
    }

    fn ensure_verify_phase(&mut self, session_id: Uuid) -> anyhow::Result<()> {
        let current = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if current.phase == SessionPhase::SpecApproved {
            self.dispatch_event(session_id, SessionEvent::ProposeRealization)?;
        }
        let current = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        if current.phase == SessionPhase::RealizationProposed {
            self.dispatch_event(session_id, SessionEvent::AcceptRealization)?;
        }
        Ok(())
    }

    fn finish_verification(
        &mut self,
        session_id: Uuid,
        passed: bool,
    ) -> anyhow::Result<SessionSnapshot> {
        let event = if passed {
            SessionEvent::VerificationPassed
        } else {
            SessionEvent::VerificationFailed
        };
        self.dispatch_event(session_id, event)
    }

    fn seed_example_project(
        &mut self,
        session_id: Uuid,
        template: WorkflowTemplate,
        example_path: &str,
    ) -> anyhow::Result<SessionSnapshot> {
        let op = match find_examples_root()
            .map(|root| root.join(example_path))
            .filter(|path| template.source_exists(path.as_path()))
        {
            Some(source) => match copy_example_tree(source.as_path(), self.root.as_path()) {
                Ok(()) => seeded_example_op(session_id, template, source.as_path()),
                Err(error) => failed_seed_op(session_id, template, Some(source.as_path()), error),
            },
            None => failed_seed_op(
                session_id,
                template,
                None,
                anyhow!("x07 docs example `{example_path}` was not found"),
            ),
        };
        self.append_op(session_id, op)
    }

    fn ensure_workflow_directory(
        &mut self,
        session_id: Uuid,
        directory: &'static str,
    ) -> anyhow::Result<SessionSnapshot> {
        let target = self.root.join(directory.trim_end_matches('/'));
        let op = match fs::create_dir_all(target.as_path()) {
            Ok(()) => prepared_directory_op(session_id, directory),
            Err(error) => failed_directory_op(session_id, directory, error.into()),
        };
        self.append_op(session_id, op)
    }

    pub fn list_provider_profiles(&self) -> anyhow::Result<Vec<ProviderProfile>> {
        self.store.load_provider_profiles()
    }

    pub fn list_agent_profiles(&self) -> anyhow::Result<Vec<AgentProfile>> {
        let mut profiles = default_agent_profiles();
        for saved in self.store.load_agent_profiles()? {
            if let Some(existing) = profiles.iter_mut().find(|profile| profile.id == saved.id) {
                *existing = saved;
            } else {
                profiles.push(saved);
            }
        }
        Ok(profiles)
    }

    pub fn save_agent_profile(&self, profile: &AgentProfile) -> anyhow::Result<()> {
        self.store.save_agent_profile(profile)
    }

    pub async fn create_agent_handoff(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
    ) -> anyhow::Result<(AgentHandoff, SessionSnapshot)> {
        let seed = self.genpack_context_seed(session_id)?;
        let genpack = Self::resolve_genpack_context(seed).await;
        self.create_agent_handoff_with_genpack(session_id, agent_id, genpack.as_ref())
    }

    pub fn create_agent_handoff_with_genpack(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        genpack: Option<&GenpackHandoffContext>,
    ) -> anyhow::Result<(AgentHandoff, SessionSnapshot)> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let agent = self
            .list_agent_profiles()?
            .into_iter()
            .find(|profile| profile.id == agent_id)
            .ok_or_else(|| anyhow!("unknown agent profile `{agent_id}`"))?;
        ensure_agent_enabled(&agent, "creating a handoff")?;
        let handoff = agent_handoff_from_session(&session, &agent, genpack);
        let prompt_path = self.store.save_agent_handoff(&handoff)?;
        let now = now_string();
        let op = OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: format!("agent.handoff.{agent_id}"),
            backend: "studio".to_string(),
            command: std::iter::once(agent.command.clone())
                .chain(agent.args.clone())
                .chain(std::iter::once(handoff.prompt_path.clone()))
                .collect(),
            started_at: now.clone(),
            finished_at: Some(now),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: vec![prompt_path.to_string()],
            notes: Some(format!("Generated {} handoff prompt.", agent.label)),
            stdout: Some(handoff.prompt.clone()),
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: serde_json::to_value(&handoff).ok(),
            report_path: None,
        };
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op)))?;
        self.store.save_session(&snapshot)?;
        Ok((handoff, snapshot))
    }

    pub async fn run_agent_handoff(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        mode: AgentRunMode,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<(AgentHandoff, OpRecord, SessionSnapshot)> {
        let prepared = self
            .start_agent_handoff(session_id, agent_id, mode, timeout_seconds)
            .await?;
        let handoff = prepared.handoff.clone();
        if let Some(command) = prepared.command {
            let op = Self::execute_agent_command(command).await;
            let session = self.complete_agent_run(op.clone())?;
            Ok((handoff, op, session))
        } else {
            Ok((handoff, prepared.op, prepared.session))
        }
    }

    pub async fn start_agent_handoff(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        mode: AgentRunMode,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let seed = self.genpack_context_seed(session_id)?;
        let genpack = Self::resolve_genpack_context(seed).await;
        self.start_agent_handoff_with_genpack(
            session_id,
            agent_id,
            mode,
            timeout_seconds,
            genpack.as_ref(),
        )
    }

    pub fn start_agent_handoff_with_genpack(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        mode: AgentRunMode,
        timeout_seconds: Option<u64>,
        genpack: Option<&GenpackHandoffContext>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let agent = self
            .list_agent_profiles()?
            .into_iter()
            .find(|profile| profile.id == agent_id)
            .ok_or_else(|| anyhow!("unknown agent profile `{agent_id}`"))?;
        ensure_agent_enabled(&agent, "planning or executing a supervised run")?;
        let handoff = agent_handoff_from_session(&session, &agent, genpack);
        let prompt_path = self.store.save_agent_handoff(&handoff)?;

        let (op, command) = match mode {
            AgentRunMode::Plan => (
                agent_plan_op(session_id, &agent, &handoff, &prompt_path),
                None,
            ),
            AgentRunMode::Execute => {
                ensure_agent_command_available(&agent)?;
                if agent.approval_required && !agent_run_is_approved(&session, &agent.id) {
                    let op = agent_approval_op(
                        session_id,
                        &agent,
                        "Approve supervised execution before the command is launched.",
                    );
                    let snapshot = self.append_op(session_id, op.clone())?;
                    return Ok(PreparedAgentRun {
                        handoff,
                        op,
                        session: snapshot,
                        command: None,
                        clarify_round: None,
                    });
                }
                let op = agent_running_op(session_id, &agent, &handoff, &prompt_path);
                let command = AgentCommandPlan {
                    session_id,
                    op_id: op.id,
                    agent: agent.clone(),
                    handoff: handoff.clone(),
                    prompt_path: prompt_path.clone(),
                    cwd: self.root.clone(),
                    program: agent.command.clone(),
                    args: handoff.command.iter().skip(1).cloned().collect(),
                    timeout_seconds: timeout_seconds.unwrap_or(30).clamp(1, 300),
                    op_kind: "run".to_string(),
                };
                (op, Some(command))
            }
        };
        let snapshot = self.append_op(session_id, op.clone())?;
        Ok(PreparedAgentRun {
            handoff,
            op,
            session: snapshot,
            command,
            clarify_round: None,
        })
    }

    /// Prepares a supervised "intent clarify" round. The agent runs once,
    /// emits 1-3 plain-English clarifying questions (as structured
    /// `agent_event` JSONL with kind `clarify_question`), then exits. The
    /// browser uses the resulting `agent.event.<agent>.clarify_question`
    /// records to render question cards. No files are written.
    pub async fn start_intent_clarify(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let seed = self.genpack_context_seed(session_id)?;
        let genpack = Self::resolve_genpack_context(seed).await;
        self.start_intent_clarify_with_genpack(
            session_id,
            agent_id,
            timeout_seconds,
            genpack.as_ref(),
        )
    }

    pub fn start_intent_clarify_with_genpack(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        timeout_seconds: Option<u64>,
        genpack: Option<&GenpackHandoffContext>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let intent = session.intent.as_ref().ok_or_else(|| {
            anyhow!("session `{session_id}` must have a draft intent before clarify")
        })?;
        let round = intent
            .clarification_history
            .iter()
            .map(|turn| turn.round)
            .max()
            .unwrap_or(0)
            + 1;
        let agent = self
            .list_agent_profiles()?
            .into_iter()
            .find(|profile| profile.id == agent_id)
            .ok_or_else(|| anyhow!("unknown agent profile `{agent_id}`"))?;
        ensure_agent_enabled(&agent, "running an intent clarify round")?;
        ensure_agent_command_available(&agent)?;
        let handoff = agent_clarify_handoff_from_session(&session, &agent, round, genpack);
        let prompt_path = self
            .store
            .save_agent_handoff_with_suffix(&handoff, "clarify")?;
        let op = agent_clarify_running_op(session_id, &agent, &handoff, &prompt_path, round);
        let mut clarify_agent = agent.clone();
        clarify_agent.allowed_verbs = vec!["intent.clarify".to_string()];
        clarify_agent.write_roots = Vec::new();
        let command = AgentCommandPlan {
            session_id,
            op_id: op.id,
            agent: clarify_agent,
            handoff: handoff.clone(),
            prompt_path: prompt_path.clone(),
            cwd: self.root.clone(),
            program: agent.command.clone(),
            args: handoff.command.iter().skip(1).cloned().collect(),
            timeout_seconds: timeout_seconds.unwrap_or(90).clamp(10, 300),
            op_kind: "clarify".to_string(),
        };
        let snapshot = self.append_op(session_id, op.clone())?;
        Ok(PreparedAgentRun {
            handoff,
            op,
            session: snapshot,
            command: Some(command),
            clarify_round: Some(round),
        })
    }

    /// Prepares a supervised realize run that asks the agent to fill in the
    /// scaffolded `src/` modules. Mirrors the clarify pipeline: returns a
    /// `PreparedAgentRun` whose `command` the caller pumps through
    /// `execute_agent_command_streaming`. The realize handoff is scoped to
    /// `src/`+`tests/` write roots and the `impl.sync.write` verb.
    pub async fn start_intent_realize(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let seed = self.genpack_context_seed(session_id)?;
        let genpack = Self::resolve_genpack_context(seed).await;
        self.start_intent_realize_with_genpack(
            session_id,
            agent_id,
            timeout_seconds,
            genpack.as_ref(),
        )
    }

    pub fn start_intent_realize_with_genpack(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        timeout_seconds: Option<u64>,
        genpack: Option<&GenpackHandoffContext>,
    ) -> anyhow::Result<PreparedAgentRun> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let _intent = session.intent.as_ref().ok_or_else(|| {
            anyhow!("session `{session_id}` must have an approved intent before realize")
        })?;
        let stub_paths = crate::summarize::scan_stub_modules(self.root.as_path());
        let agent = self
            .list_agent_profiles()?
            .into_iter()
            .find(|profile| profile.id == agent_id)
            .ok_or_else(|| anyhow!("unknown agent profile `{agent_id}`"))?;
        ensure_agent_enabled(&agent, "running an implementation realize round")?;
        ensure_agent_command_available(&agent)?;
        let handoff = agent_realize_handoff_from_session(&session, &agent, &stub_paths, genpack);
        let prompt_path = self
            .store
            .save_agent_handoff_with_suffix(&handoff, "realize")?;
        let op = agent_realize_running_op(session_id, &agent, &handoff, &prompt_path);
        let mut realize_agent = agent.clone();
        realize_agent.allowed_verbs = handoff.allowed_verbs.clone();
        realize_agent.write_roots = handoff.write_roots.clone();
        let command = AgentCommandPlan {
            session_id,
            op_id: op.id,
            agent: realize_agent,
            handoff: handoff.clone(),
            prompt_path: prompt_path.clone(),
            cwd: self.root.clone(),
            program: agent.command.clone(),
            args: handoff.command.iter().skip(1).cloned().collect(),
            timeout_seconds: timeout_seconds.unwrap_or(180).clamp(20, 600),
            op_kind: "realize".to_string(),
        };
        let snapshot = self.append_op(session_id, op.clone())?;
        Ok(PreparedAgentRun {
            handoff,
            op,
            session: snapshot,
            command: Some(command),
            clarify_round: None,
        })
    }

    /// Runs after the realize agent exits. Re-runs `impl.check` and
    /// `xtal.verify` (with the same world override the build pipeline
    /// uses) so the timeline gains a fresh Verified turn with a non-stub
    /// summary. Returns the final session snapshot and the workspace-
    /// relative files the agent wrote.
    pub async fn finalize_realize(
        &mut self,
        session_id: Uuid,
        run_op_id: Uuid,
    ) -> anyhow::Result<(SessionSnapshot, bool, Vec<String>)> {
        let mut current = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let wrote_files: Vec<String> = current
            .op_log
            .iter()
            .find(|op| op.id == run_op_id)
            .and_then(|op| op.report_json.as_ref())
            .and_then(|value| value.get("write_audit"))
            .and_then(|audit| audit.get("created").or_else(|| audit.get("modified")))
            .and_then(|value| value.as_array())
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default();
        let mut vars: BTreeMap<String, String> = BTreeMap::new();
        vars.insert("allow_os_world".to_string(), "true".to_string());
        let after_check = self
            .run_binding(session_id, "impl.check", &BTreeMap::new())
            .await?;
        if last_op_failed(&after_check) {
            current = after_check;
            return Ok((current, false, wrote_files));
        }
        let after_verify = self.run_binding(session_id, "xtal.verify", &vars).await?;
        let verified_ok = !last_op_failed(&after_verify);
        // The session is usually at `TrustReview` after the initial build —
        // the lifecycle reducer rejects `verification_passed` from there.
        // Only fire a transition event when we actually came back through
        // `VerifyRunning` (e.g. after a repair). Otherwise leave the phase
        // alone and just re-emit the summary so the timeline picks up the
        // now-non-stub state.
        if matches!(after_verify.phase, SessionPhase::VerifyRunning) {
            let event = if verified_ok {
                SessionEvent::VerificationPassed
            } else {
                SessionEvent::VerificationFailed
            };
            current = self.dispatch_event(session_id, event)?;
        } else {
            current = after_verify;
        }
        if verified_ok {
            if let Some(summary_op) =
                build_plain_english_summary_with_root(&current, Some(self.root.as_path()))
            {
                current = self.append_op(session_id, summary_op)?;
            }
        }
        Ok((current, verified_ok, wrote_files))
    }

    pub fn genpack_context_seed(
        &self,
        session_id: Uuid,
    ) -> anyhow::Result<Option<(CliAdapter, IntentPacket)>> {
        let session = self
            .model
            .get_session(session_id)
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        Ok(session
            .intent
            .clone()
            .map(|intent| (self.cli.clone(), intent)))
    }

    pub async fn resolve_genpack_context(
        seed: Option<(CliAdapter, IntentPacket)>,
    ) -> Option<GenpackHandoffContext> {
        let (cli, intent) = seed?;
        crate::genpack::handoff_context(&cli, &intent).await
    }

    /// After a clarify supervised run completes, walks the new
    /// `agent.event.<agent>.clarify_question` records and merges them into
    /// `intent.clarification_history` so the UI can render Q&A cards directly
    /// off the session intent rather than parsing op_log JSON.
    pub fn ingest_clarify_questions(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        run_op_id: Uuid,
    ) -> anyhow::Result<SessionSnapshot> {
        self.ingest_clarify_questions_inner(session_id, agent_id, run_op_id, None)
    }

    fn ingest_clarify_questions_inner(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        run_op_id: Uuid,
        forced_round: Option<u32>,
    ) -> anyhow::Result<SessionSnapshot> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let mut intent = session
            .intent
            .clone()
            .ok_or_else(|| anyhow!("session `{session_id}` has no intent to clarify"))?;
        let run_op_started = session
            .op_log
            .iter()
            .find(|op| op.id == run_op_id)
            .map(|op| op.started_at.clone())
            .unwrap_or_default();
        let event_op_name = format!("agent.event.{agent_id}.clarify_question");
        let round = forced_round.unwrap_or_else(|| {
            intent
                .clarification_history
                .iter()
                .map(|turn| turn.round)
                .max()
                .unwrap_or(0)
                .saturating_add(1)
        });
        let mut appended = 0u32;
        for op in session.op_log.iter() {
            if op.op != event_op_name {
                continue;
            }
            if !run_op_started.is_empty() && op.started_at < run_op_started {
                continue;
            }
            let structured = op
                .report_json
                .as_ref()
                .and_then(|value| value.get("structured"))
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            let mut question_id = structured
                .get("id")
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| format!("q-{}", op.id));
            let duplicate = intent
                .clarification_history
                .iter()
                .find(|turn| turn.question_id == question_id);
            if duplicate
                .map(|turn| turn.agent_id == agent_id)
                .unwrap_or(false)
            {
                continue;
            }
            if duplicate.is_some() {
                question_id = format!("{question_id}-{agent_id}");
            }
            let question_text = structured
                .get("text")
                .or_else(|| structured.get("summary"))
                .and_then(serde_json::Value::as_str)
                .map(str::to_string)
                .unwrap_or_else(|| op.stdout.clone().unwrap_or_default());
            if question_text.trim().is_empty() {
                continue;
            }
            let witness_kind = structured
                .get("witness_kind")
                .and_then(serde_json::Value::as_str)
                .and_then(witness_kind_from_str)
                .unwrap_or(WitnessKind::DesiredBehavior);
            let options = structured
                .get("options")
                .and_then(serde_json::Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(|value| value.as_str().map(str::to_string))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            intent
                .clarification_history
                .push(loom_types::artifacts::ClarificationTurn {
                    question_id,
                    question_text,
                    witness_kind,
                    round,
                    agent_id: agent_id.to_string(),
                    options,
                    question_recorded_at: op.started_at.clone(),
                    answer_text: None,
                    answer_recorded_at: None,
                });
            appended += 1;
        }
        if appended == 0 {
            return Ok(session);
        }
        let snapshot = self.dispatch_with_publish(
            session_id,
            SessionEvent::FormalizeIntent(Box::new(intent.clone())),
        )?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    /// Applies user-supplied answers from the browser back into the intent
    /// packet: pairs each answer with its question, appends a matching
    /// witness, and re-emits the intent through the reducer. The session
    /// stays in `IntentReady` so a follow-up clarify round (or human
    /// approval) is legal.
    pub fn apply_intent_answers(
        &mut self,
        session_id: Uuid,
        answers: &[loom_types::api::IntentAnswer],
    ) -> anyhow::Result<(IntentPacket, OpRecord, SessionSnapshot)> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let mut intent = session
            .intent
            .clone()
            .ok_or_else(|| anyhow!("session `{session_id}` has no intent to update"))?;
        let mut applied: Vec<(String, String, WitnessKind)> = Vec::new();
        for answer in answers {
            let text = answer.text.trim();
            if text.is_empty() {
                continue;
            }
            let mut matched = false;
            for turn in intent.clarification_history.iter_mut() {
                if turn.question_id == answer.question_id {
                    turn.answer_text = Some(text.to_string());
                    turn.answer_recorded_at = Some(now_string());
                    let kind = answer
                        .witness_kind
                        .clone()
                        .unwrap_or_else(|| turn.witness_kind.clone());
                    applied.push((turn.question_id.clone(), text.to_string(), kind.clone()));
                    intent.witnesses.push(Witness {
                        kind,
                        text: format!("{}: {}", turn.question_text, text),
                    });
                    matched = true;
                    break;
                }
            }
            if !matched {
                let kind = answer
                    .witness_kind
                    .clone()
                    .unwrap_or(WitnessKind::DesiredBehavior);
                applied.push((answer.question_id.clone(), text.to_string(), kind.clone()));
                intent.witnesses.push(Witness {
                    kind,
                    text: text.to_string(),
                });
            }
        }
        if applied.is_empty() {
            bail!("no non-empty answers provided");
        }
        self.dispatch_with_publish(
            session_id,
            SessionEvent::FormalizeIntent(Box::new(intent.clone())),
        )?;
        let op = intent_clarify_answers_op(session_id, &applied);
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op.clone())))?;
        self.store.save_session(&snapshot)?;
        Ok((intent, op, snapshot))
    }

    pub fn create_agent_approval(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        reason: Option<String>,
    ) -> anyhow::Result<(OpRecord, SessionSnapshot)> {
        let agent = self
            .list_agent_profiles()?
            .into_iter()
            .find(|profile| profile.id == agent_id)
            .ok_or_else(|| anyhow!("unknown agent profile `{agent_id}`"))?;
        let op = agent_approval_op(
            session_id,
            &agent,
            reason
                .as_deref()
                .unwrap_or("Approve this agent checkpoint before continuing."),
        );
        let snapshot = self.append_op(session_id, op.clone())?;
        Ok((op, snapshot))
    }

    pub fn resolve_agent_approval(
        &mut self,
        session_id: Uuid,
        op_id: Uuid,
        decision: ApprovalDecision,
        notes: Option<String>,
    ) -> anyhow::Result<(OpRecord, SessionSnapshot)> {
        let session = self
            .model
            .get_session(session_id)
            .cloned()
            .ok_or_else(|| anyhow!("unknown session `{session_id}`"))?;
        let mut op = session
            .op_log
            .into_iter()
            .find(|candidate| candidate.id == op_id)
            .ok_or_else(|| anyhow!("unknown approval operation `{op_id}`"))?;
        if !op.op.starts_with("agent.approval.") {
            return Err(anyhow!("operation `{op_id}` is not an agent approval"));
        }
        let approved = decision == ApprovalDecision::Approve;
        op.status = if approved {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        };
        op.finished_at = Some(now_string());
        op.exit_code = Some(if approved { 0 } else { 1 });
        op.notes = Some(format!(
            "{}: {}",
            if approved { "Approved" } else { "Rejected" },
            notes.unwrap_or_else(|| "human checkpoint resolved".to_string())
        ));
        let snapshot = self.complete_agent_run(op.clone())?;
        Ok((op, snapshot))
    }

    pub async fn execute_agent_command(command: AgentCommandPlan) -> OpRecord {
        let before = snapshot_agent_workspace(command.cwd.as_path());
        let envs = agent_command_env(&command);
        match CommandRunner
            .run_with_timeout(
                command.cwd.as_path(),
                &command.program,
                &command.args,
                &envs,
                Some(command.timeout_seconds),
            )
            .await
        {
            Ok(execution) => agent_execution_op(command, execution, before),
            Err(error) => agent_spawn_error_op(command, error),
        }
    }

    pub async fn execute_agent_command_streaming(
        command: AgentCommandPlan,
        updates: mpsc::UnboundedSender<OpRecord>,
    ) -> OpRecord {
        let before = snapshot_agent_workspace(command.cwd.as_path());
        let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
        let run_command = command.clone();
        let execution = async move {
            let envs = agent_command_env(&run_command);
            CommandRunner
                .run_with_timeout_streaming(
                    run_command.cwd.as_path(),
                    &run_command.program,
                    &run_command.args,
                    &envs,
                    Some(run_command.timeout_seconds),
                    chunk_tx,
                )
                .await
        };
        tokio::pin!(execution);
        let mut semantic_events = AgentSemanticEventState::default();

        loop {
            tokio::select! {
                chunk = chunk_rx.recv() => {
                    if let Some(chunk) = chunk {
                        let _ = updates.send(agent_streaming_op(&command, chunk.clone()));
                        for event in agent_semantic_ops(&command, &chunk, &mut semantic_events) {
                            let _ = updates.send(event);
                        }
                    }
                }
                result = &mut execution => {
                    return match result {
                        Ok(execution) => agent_execution_op(command, execution, before),
                        Err(error) => agent_spawn_error_op(command, error),
                    };
                }
            }
        }
    }

    pub fn complete_agent_run(&mut self, op: OpRecord) -> anyhow::Result<SessionSnapshot> {
        let session_id = op.session_id;
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::UpdateOp(Box::new(op)))?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    pub fn save_provider_profile(&self, profile: &ProviderProfile) -> anyhow::Result<()> {
        self.store.save_provider_profile(profile)
    }

    pub async fn probe_provider(
        &mut self,
        profile: ProviderProfile,
    ) -> anyhow::Result<(ProviderProfile, ProviderProbeReport)> {
        self.store.save_provider_profile(&profile)?;
        let report = self.providers.probe(&profile).await?;
        self.store.save_provider_probe(&profile.id, &report)?;
        Ok((profile, report))
    }

    pub async fn connect_mcp(
        &mut self,
        endpoint: McpEndpoint,
        alias: Option<String>,
    ) -> anyhow::Result<(McpConnectionInfo, Vec<McpToolDescriptor>)> {
        let connection_id = alias.unwrap_or_else(|| Uuid::new_v4().to_string());
        let mut client = boxed_client(endpoint, connection_id.clone());
        let info = client.initialize().await?;
        let tools = client.list_tools().await.unwrap_or_default();
        self.mcp_connections.insert(connection_id, client);
        Ok((info, tools))
    }

    pub async fn list_mcp_tools(
        &mut self,
        connection_id: &str,
    ) -> anyhow::Result<Vec<McpToolDescriptor>> {
        let client = self
            .mcp_connections
            .get_mut(connection_id)
            .ok_or_else(|| anyhow!("unknown MCP connection `{connection_id}`"))?;
        client.list_tools().await
    }

    pub async fn call_mcp_tool(
        &mut self,
        connection_id: &str,
        name: &str,
        arguments: serde_json::Value,
    ) -> anyhow::Result<McpToolCallResult> {
        let client = self
            .mcp_connections
            .get_mut(connection_id)
            .ok_or_else(|| anyhow!("unknown MCP connection `{connection_id}`"))?;
        client.call_tool(name, arguments).await
    }

    pub async fn close_mcp_connection(&mut self, connection_id: &str) -> anyhow::Result<()> {
        if let Some(mut client) = self.mcp_connections.remove(connection_id) {
            client.close().await?;
        }
        Ok(())
    }

    fn append_op(&mut self, session_id: Uuid, op: OpRecord) -> anyhow::Result<SessionSnapshot> {
        let snapshot =
            self.dispatch_with_publish(session_id, SessionEvent::AppendOp(Box::new(op)))?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }
}

pub fn xtal_workflow_vars_from_intent(intent: &IntentPacket) -> BTreeMap<String, String> {
    let target = intent.targets.first();
    let module_id = target
        .map(|item| item.module_id.as_str())
        .filter(|module_id| !module_id.trim().is_empty())
        .unwrap_or("app.main");
    let op = target
        .and_then(|item| item.entry.as_deref())
        .map(sanitize_op_name)
        .filter(|entry| !entry.is_empty())
        .unwrap_or_else(|| "run_v1".to_string());
    let result = if matches!(&intent.source, IntentSource::Incident { .. }) {
        "bytes"
    } else if op.contains("makespan") || op.contains("count") || op.contains("len") {
        "i32"
    } else {
        "bytes"
    };
    let input = format!("spec/{module_id}.x07spec.json");

    BTreeMap::from([
        ("module_id".to_string(), module_id.to_string()),
        ("op".to_string(), op),
        ("param".to_string(), "payload:bytes".to_string()),
        ("result".to_string(), result.to_string()),
        ("input".to_string(), input),
        (
            "patchset_out".to_string(),
            "target/xtal/impl-sync.patchset.json".to_string(),
        ),
    ])
}

fn checked_xtal_verify_run_vars(
    run_vars: &BTreeMap<String, String>,
) -> anyhow::Result<BTreeMap<String, String>> {
    let allowed = [
        "proof_policy",
        "allow_os_world",
        "unwind",
        "max_bytes_len",
        "input_len_bytes",
    ];
    let mut verify_vars = BTreeMap::new();
    for (key, value) in run_vars {
        if allowed.contains(&key.as_str()) {
            verify_vars.insert(key.clone(), value.clone());
        } else {
            bail!("unsupported xtal workflow var `{key}`");
        }
    }
    validate_xtal_verify_vars(&verify_vars)?;
    Ok(verify_vars)
}

fn validate_relative_runtime_path(value: &str, context: &str) -> anyhow::Result<()> {
    let path = Utf8Path::new(value);
    if value.contains('\0')
        || path.is_absolute()
        || path.components().any(|part| part.as_str() == "..")
    {
        bail!("{context} must be a relative path inside the workspace");
    }
    Ok(())
}

/// Pattern-match the user's question against the session's intent witnesses
/// and (when present) the latest verify evidence. The answer is always
/// grounded — it cites the witness it derived its text from or, failing that,
/// the spec module / verify summary path. No LLM call: the goal is a
/// trustworthy deterministic answer.
fn answer_project_question(
    root: &Utf8Path,
    session: &SessionSnapshot,
    req: &AskRequest,
) -> AskAnswer {
    let intent = match session.intent.as_ref() {
        Some(intent) => intent,
        None => {
            return AskAnswer {
                text: "There's no approved intent on this session yet — \
                       describe what you want, then I can answer questions \
                       about the project's behavior."
                    .to_string(),
                citations: Vec::new(),
            };
        }
    };
    let question = req.question.trim();
    if question.is_empty() {
        return AskAnswer {
            text: "Ask me about what the project does, what it refuses, or what \
                   the latest verify report covers."
                .to_string(),
            citations: Vec::new(),
        };
    }
    let lowered = question.to_ascii_lowercase();
    let keywords: Vec<&str> = lowered
        .split(|c: char| !c.is_alphanumeric())
        .filter(|word| word.len() >= 3)
        .collect();
    let target = intent
        .targets
        .first()
        .map(|item| {
            format!(
                "{}.{}",
                item.module_id,
                item.entry.as_deref().unwrap_or("run_v1")
            )
        })
        .unwrap_or_else(|| "this project".to_string());
    let spec_path = intent
        .targets
        .first()
        .map(|item| format!("spec/{}.x07spec.json", item.module_id))
        .unwrap_or_else(|| "x07.json".to_string());

    // Score witnesses by keyword overlap. The doctrine filter from the
    // summarizer keeps boilerplate witnesses out of the response.
    let score_text = |text: &str| -> usize {
        let lower = text.to_ascii_lowercase();
        keywords.iter().filter(|kw| lower.contains(*kw)).count()
    };
    let best_witness = intent
        .witnesses
        .iter()
        .filter(|w| !crate::summarize::is_doctrine(&w.text))
        .map(|w| (score_text(&w.text), w))
        .filter(|(score, _)| *score > 0)
        .max_by_key(|(score, _)| *score)
        .map(|(_, w)| w);

    let mut citations = Vec::new();
    let text = if let Some(witness) = best_witness {
        citations.push(AnswerCitation {
            kind: "spec".to_string(),
            path: spec_path.clone(),
            locator: format!("/witnesses/{:?}", witness.kind),
        });
        let prefix = match witness.kind {
            WitnessKind::DesiredBehavior => "It will",
            WitnessKind::ForbiddenBehavior => "It will not",
            WitnessKind::PolicyRequirement => "Policy",
            WitnessKind::IncidentReport => "Incident on record",
        };
        format!("{prefix}: {}", witness.text.trim())
    } else if !intent.examples.is_empty() {
        citations.push(AnswerCitation {
            kind: "spec".to_string(),
            path: spec_path.clone(),
            locator: "/examples".to_string(),
        });
        let example = intent.examples.first().cloned().unwrap_or_default();
        format!(
            "I don't have a witness that addresses that exactly, but here's a \
             representative example from {target}: {example}"
        )
    } else {
        format!(
            "I haven't recorded a specific witness for `{question}` on \
             `{target}` yet. The latest verify evidence is the best place to \
             check whether the behavior is covered."
        )
    };

    let verify_summary = root.join("target/xtal/verify/summary.json");
    if verify_summary.exists() {
        citations.push(AnswerCitation {
            kind: "verify".to_string(),
            path: "target/xtal/verify/summary.json".to_string(),
            locator: "/entries".to_string(),
        });
    }

    AskAnswer { text, citations }
}

fn proof_citations_for_session(session: &SessionSnapshot) -> Vec<ProofCitation> {
    let target_clause = session
        .intent
        .as_ref()
        .and_then(|intent| intent.targets.first())
        .map(|target| {
            format!(
                "{}.{}",
                target.module_id,
                target.entry.as_deref().unwrap_or("run_v1")
            )
        })
        .unwrap_or_else(|| "studio.session".to_string());
    let proof_report = session
        .op_log
        .iter()
        .rev()
        .find(|op| op.op == "xtal.verify")
        .and_then(|op| {
            op.artifacts
                .iter()
                .find(|artifact| artifact.contains("verify") && artifact.ends_with(".json"))
        })
        .cloned();
    vec![ProofCitation {
        clause_id: target_clause,
        proof_report,
        summary: "Latest verify evidence backs this Try-It run.".to_string(),
    }]
}

fn incident_detected_op(session_id: Uuid, incident: &crate::incidents::IncidentBundle) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "agent.event.incident.detected".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "incidents".to_string(),
            "scan".to_string(),
            incident.root_path.clone(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![incident.root_path.clone()],
        notes: Some(incident.summary.clone()),
        stdout: Some(incident.summary.clone()),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.incident_detected@0.1.0",
            "incident_id": incident.id,
            "root_path": incident.root_path,
            "kind": incident.kind,
            "summary": incident.summary,
            "at": incident.at,
        })),
        report_path: None,
    }
}

fn quorum_op(session_id: Uuid, round: u32, agents: &[QuorumAgent]) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "intent.quorum".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "intent".to_string(),
            "quorum".to_string(),
            round.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![format!(".x07/studio/sessions/{session_id}.json")],
        notes: Some(format!(
            "Completed live parallel quorum clarify round {round}."
        )),
        stdout: Some(format!(
            "{} agents completed supervised clarify runs.",
            agents.len()
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.quorum_round@0.1.0",
            "execution": "live_parallel",
            "round": round,
            "agents": agents,
        })),
        report_path: None,
    }
}

fn cassette_entries_from_workspace(root: &Utf8Path) -> Vec<CassetteEntry> {
    let mut entries = Vec::<CassetteEntry>::new();
    visit_workspace_files(root.join(".x07_rr").as_path(), &mut |path| {
        if !path.is_file() {
            return;
        }
        let size_bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
        let key = path
            .strip_prefix(root)
            .map(|relative| relative.to_string())
            .unwrap_or_else(|_| path.to_string());
        entries.push(CassetteEntry {
            idx: 0,
            kind: path
                .extension()
                .map(str::to_string)
                .unwrap_or_else(|| "entry".to_string()),
            key,
            ts: modified_unix_ms(path)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "0".to_string()),
            size_bytes,
        });
    });
    entries.sort_by(|left, right| {
        left.ts
            .cmp(&right.ts)
            .then_with(|| left.key.cmp(&right.key))
    });
    for (idx, entry) in entries.iter_mut().enumerate() {
        entry.idx = u32::try_from(idx).unwrap_or(u32::MAX);
    }
    entries
}

fn materialize_cassette_branch(
    root: &Utf8Path,
    branch_session_id: Uuid,
    source_session_id: Uuid,
    from_entry: u32,
    replayed: &[CassetteEntry],
    truncated: &[CassetteEntry],
) -> anyhow::Result<String> {
    let relative_manifest =
        format!(".x07/studio/cassette_branches/{branch_session_id}/replay.json");
    let branch_dir = root.join(format!(".x07/studio/cassette_branches/{branch_session_id}"));
    let replay_dir = branch_dir.join("replay");
    fs::create_dir_all(replay_dir.as_path())?;
    for entry in replayed {
        let source = root.join(&entry.key);
        if !source.is_file() {
            continue;
        }
        let replay_relative = entry
            .key
            .strip_prefix(".x07_rr/")
            .unwrap_or(entry.key.as_str());
        let dest = replay_dir.join(replay_relative);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, dest)?;
    }
    let manifest = serde_json::json!({
        "schema_version": "x07.studio.cassette_replay@0.1.0",
        "source_session_id": source_session_id,
        "branch_session_id": branch_session_id,
        "from_entry": from_entry,
        "replay_root": format!(".x07/studio/cassette_branches/{branch_session_id}/replay"),
        "replayed_entries": replayed,
        "truncated_entries": truncated,
    });
    let manifest_path = root.join(&relative_manifest);
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        manifest_path,
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    )?;
    Ok(relative_manifest)
}

fn truncate_ops_for_cassette(ops: &[OpRecord], selected: &CassetteEntry) -> Vec<OpRecord> {
    let cutoff = selected.ts.parse::<u64>().ok().map(|value| {
        if value > 10_000_000_000 {
            value / 1000
        } else {
            value
        }
    });
    match cutoff {
        Some(cutoff) => ops
            .iter()
            .filter(|op| {
                op.started_at
                    .parse::<u64>()
                    .map(|started| started <= cutoff)
                    .unwrap_or(true)
            })
            .cloned()
            .collect(),
        None => ops.to_vec(),
    }
}

fn cassette_branch_op(
    new_session_id: Uuid,
    source_session_id: Uuid,
    from_entry: u32,
    replayed: &[CassetteEntry],
    truncated: &[CassetteEntry],
    replay_manifest: String,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id: new_session_id,
        op: "cassette.branch".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "cassette".to_string(),
            "branch".to_string(),
            from_entry.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![replay_manifest.clone()],
        notes: Some("Created a sibling session from cassette history.".to_string()),
        stdout: Some(format!(
            "Branched from session {source_session_id} at cassette entry {from_entry}; replayed {} entries and truncated {} entries.",
            replayed.len(),
            truncated.len()
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.cassette_branch@0.1.0",
            "source_session_id": source_session_id,
            "from_entry": from_entry,
            "replay_manifest": replay_manifest,
            "replayed_entries": replayed,
            "truncated_entries": truncated,
        })),
        report_path: None,
    }
}

fn sync_expires_at() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() + 600)
        .unwrap_or(600);
    format!("{seconds}")
}

fn sync_code_is_expired(expires_at: &str) -> bool {
    let Ok(expires_at) = expires_at.parse::<u64>() else {
        return true;
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    expires_at <= now
}

fn visual_parse_value(kind: &str, source: serde_json::Value) -> serde_json::Value {
    match kind {
        "streampipe" => serde_json::json!({
            "nodes": source
                .as_str()
                .unwrap_or("")
                .split('|')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .enumerate()
                .map(|(idx, label)| serde_json::json!({"id": idx, "label": label}))
                .collect::<Vec<_>>(),
            "edges": [],
        }),
        "statemachine" | "tasks" => source,
        _ => serde_json::json!({ "source": source }),
    }
}

fn visual_emit_value(kind: &str, graph: serde_json::Value) -> serde_json::Value {
    match kind {
        "streampipe" => {
            let text = graph
                .get("nodes")
                .and_then(serde_json::Value::as_array)
                .map(|nodes| {
                    nodes
                        .iter()
                        .filter_map(|node| node.get("label").and_then(serde_json::Value::as_str))
                        .collect::<Vec<_>>()
                        .join(" | ")
                })
                .unwrap_or_default();
            serde_json::Value::String(text)
        }
        "statemachine" | "tasks" => graph,
        _ => serde_json::json!({ "graph": graph }),
    }
}

fn atlas_platform_delivery_vars(
    root: &Utf8Path,
    deployment_id: Option<&str>,
) -> BTreeMap<String, String> {
    let pack_dir = "dist/showcase_fullstack/pack.atlas_release";
    let pack_manifest = "dist/showcase_fullstack/pack.atlas_release/app.pack.json";
    let deploy_plan = "dist/showcase_fullstack/deploy.atlas_release/deploy.plan.json";
    let metrics_dir = "tests/fixtures/metrics";
    let state_dir = ".x07/platform";

    let mut vars = BTreeMap::from([
        ("pack_dir".to_string(), pack_dir.to_string()),
        ("pack_dir_arg".to_string(), workspace_arg(root, pack_dir)),
        ("pack_manifest".to_string(), pack_manifest.to_string()),
        (
            "pack_manifest_arg".to_string(),
            workspace_arg(root, pack_manifest),
        ),
        ("plan".to_string(), deploy_plan.to_string()),
        ("plan_arg".to_string(), workspace_arg(root, deploy_plan)),
        ("metrics_dir".to_string(), metrics_dir.to_string()),
        (
            "metrics_dir_arg".to_string(),
            workspace_arg(root, metrics_dir),
        ),
        ("state_dir".to_string(), state_dir.to_string()),
        ("state_dir_arg".to_string(), workspace_arg(root, state_dir)),
    ]);
    if let Some(deployment_id) = deployment_id {
        vars.insert("deployment_id".to_string(), deployment_id.to_string());
    }
    vars
}

fn workspace_arg(root: &Utf8Path, relative: &str) -> String {
    root.join(relative.trim_end_matches('/')).to_string()
}

fn platform_deployment_id_from_snapshot(snapshot: &SessionSnapshot) -> Option<String> {
    snapshot
        .op_log
        .last()
        .and_then(|op| op.report_json.as_ref())
        .and_then(platform_deployment_id_from_report)
}

fn platform_deployment_id_from_report(report: &serde_json::Value) -> Option<String> {
    json_string_at(report, &["exec_id"])
        .or_else(|| json_string_at(report, &["deployment_id"]))
        .or_else(|| json_string_at(report, &["result", "exec_id"]))
        .or_else(|| json_string_at(report, &["result", "deployment_id"]))
        .filter(|value| !value.trim().is_empty())
}

fn json_string_at(value: &serde_json::Value, path: &[&str]) -> Option<String> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str().map(ToOwned::to_owned)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkflowTemplate {
    XtalPure,
    WorkflowGraph,
    StateMachineArch,
    ApiGateway,
    X07Crawl,
    DbGuard,
    X07Atlas,
}

impl WorkflowTemplate {
    fn id(self) -> &'static str {
        match self {
            Self::XtalPure => "xtal-pure",
            Self::WorkflowGraph => "workflow-graph",
            Self::StateMachineArch => "state-machine-arch",
            Self::ApiGateway => "x07-api-gateway",
            Self::X07Crawl => "x07crawl",
            Self::DbGuard => "x07dbguard",
            Self::X07Atlas => "x07_atlas",
        }
    }

    fn example_path(self) -> Option<&'static str> {
        match self {
            Self::XtalPure => None,
            Self::WorkflowGraph => Some("agent-gate/xtal/workflow-graph"),
            Self::StateMachineArch => Some("readiness-checks/x07-sm-arch-contracts-smoke"),
            Self::ApiGateway => Some("apps/x07-api-gateway"),
            Self::X07Crawl => Some("apps/x07crawl"),
            Self::DbGuard => Some("apps/x07dbguard"),
            Self::X07Atlas => Some("wasm_showcases/x07_atlas"),
        }
    }

    fn source_exists(self, source: &Utf8Path) -> bool {
        match self {
            Self::X07Atlas => {
                source.join("arch/app/index.x07app.json").exists()
                    && source.join("frontend/x07.json").exists()
                    && source.join("backend/x07.json").exists()
            }
            _ => source.join("x07.json").exists(),
        }
    }

    fn workflow_steps(self) -> &'static [&'static str] {
        self.workflow_steps_for_environment(sandbox_vm_guest_bundle_declared())
    }

    fn platform_delivery_steps(self) -> &'static [&'static str] {
        match self {
            Self::X07Atlas => &[
                "lp.deploy.accept.local",
                "lp.deploy.run.local.metrics",
                "lp.deploy.query.local",
                "lp.deploy.status.local",
            ],
            _ => &[],
        }
    }

    fn workflow_steps_for_environment(self, has_vm_guest_bundle: bool) -> &'static [&'static str] {
        match self {
            Self::XtalPure => &[],
            Self::WorkflowGraph => &[
                "tests.gen.write",
                "gen.verify",
                "test.xtal.generated.all",
                "impl.check",
                "xtal.dev",
                "xtal.verify",
                "test.manifest",
            ],
            Self::StateMachineArch => &[
                "sm.gen.write",
                "test.sm.generated",
                "run.stdin",
                "arch.check.write_lock",
                "test.manifest",
            ],
            Self::ApiGateway if has_vm_guest_bundle => &[
                "arch.check.write_lock",
                "test.manifest",
                "run.sandbox",
                "bundle.api_gateway.sandbox",
            ],
            Self::ApiGateway => &[
                "arch.check.write_lock",
                "test.manifest",
                "run.sandbox.os",
                "bundle.api_gateway.sandbox.os",
            ],
            Self::X07Crawl if has_vm_guest_bundle => &[
                "pkg.lock",
                "test.manifest",
                "run.x07crawl.sandbox",
                "bundle.x07crawl.sandbox",
            ],
            Self::X07Crawl => &[
                "pkg.lock",
                "test.manifest",
                "run.x07crawl.sandbox.os",
                "bundle.x07crawl.sandbox.os",
            ],
            Self::DbGuard if has_vm_guest_bundle => &[
                "pkg.lock",
                "arch.check.write_lock",
                "test.manifest",
                "run.stdin",
                "run.sandbox.stdin",
                "bundle.dbguard.sandbox",
            ],
            Self::DbGuard => &[
                "pkg.lock",
                "arch.check.write_lock",
                "test.manifest",
                "run.stdin",
                "run.sandbox.stdin.os",
                "bundle.dbguard.sandbox.os",
            ],
            Self::X07Atlas => &[
                "pkg.lock.atlas.frontend",
                "wasm.app.profile.validate.atlas_dev",
                "wasm.web_ui.contracts.validate",
                "wasm.http.contracts.validate",
                "wasm.caps.validate.atlas_release",
                "wasm.ops.validate",
                "wasm.slo.validate.atlas",
                "wasm.app.build.atlas_dev",
                "wasm.app.serve.smoke.atlas_dev",
                "wasm.app.test.happy_path",
                "wasm.app.test.validation_error",
                "wasm.app.test.regress.atlas_incident",
                "wasm.app.build.atlas_release",
                "wasm.app.pack.atlas_release",
                "wasm.app.verify.atlas_release",
                "wasm.provenance.attest.atlas_release",
                "wasm.provenance.verify.atlas_release",
                "wasm.deploy.plan.atlas_release",
                "wasm.slo.eval.atlas_canary_ok",
            ],
        }
    }

    fn stdin_for_step(self, step: &str) -> Option<&'static str> {
        match (self, step) {
            (Self::StateMachineArch, "run.stdin") => Some("start\ntick\nfinish\n"),
            (Self::DbGuard, "run.stdin") => Some("verify"),
            (Self::DbGuard, "run.sandbox.stdin") => Some("apply out/dbguard.sqlite"),
            (Self::DbGuard, "run.sandbox.stdin.os") => Some("apply out/dbguard.sqlite"),
            _ => None,
        }
    }

    fn directory_for_step(self, step: &str) -> Option<&'static str> {
        match (self, step) {
            (Self::X07Crawl, "run.x07crawl.sandbox") => Some("out/"),
            (Self::X07Crawl, "run.x07crawl.sandbox.os") => Some("out/"),
            (Self::DbGuard, "run.sandbox.stdin") => Some("out/"),
            (Self::DbGuard, "run.sandbox.stdin.os") => Some("out/"),
            (Self::X07Atlas, "wasm.app.pack.atlas_release") => {
                Some("dist/showcase_fullstack/pack.atlas_release/")
            }
            (Self::X07Atlas, "wasm.deploy.plan.atlas_release") => {
                Some("dist/showcase_fullstack/deploy.atlas_release/")
            }
            _ => None,
        }
    }
}

fn workflow_template_from_intent(intent: &IntentPacket) -> WorkflowTemplate {
    let target = intent.targets.first();
    let module_id = target
        .map(|item| item.module_id.as_str())
        .unwrap_or_default();
    let entry = target
        .and_then(|item| item.entry.as_deref())
        .unwrap_or_default();
    let raw_source = match &intent.source {
        IntentSource::Text { raw } => raw.as_str(),
        IntentSource::Voice { transcript } => transcript.as_str(),
        IntentSource::Spec { raw } => raw.as_str(),
        IntentSource::Incident { path } => path.as_str(),
        IntentSource::Sketch { path } => path.as_str(),
        IntentSource::Image { path, .. } => path.as_str(),
    };
    let haystack = format!("{module_id} {entry} {raw_source}").to_ascii_lowercase();
    if haystack.contains("x07_atlas") || haystack.contains("x07 atlas") || module_id == "atlas.app"
    {
        WorkflowTemplate::X07Atlas
    } else if haystack.contains("x07dbguard") || module_id == "db.guard" {
        WorkflowTemplate::DbGuard
    } else if haystack.contains("x07-api-gateway") || module_id == "gateway.core" {
        WorkflowTemplate::ApiGateway
    } else if haystack.contains("x07crawl") || module_id == "crawl.plan" {
        WorkflowTemplate::X07Crawl
    } else if haystack.contains("x07-sm-arch-contracts-smoke") || module_id == "workflow.lifecycle"
    {
        WorkflowTemplate::StateMachineArch
    } else if haystack.contains("workflow-graph") || module_id == "workflow.graph" {
        WorkflowTemplate::WorkflowGraph
    } else {
        WorkflowTemplate::XtalPure
    }
}

fn find_examples_root() -> Option<Utf8PathBuf> {
    if let Ok(value) = std::env::var("X07_STUDIO_X07_EXAMPLES_ROOT") {
        let path = Utf8PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }
    let cwd = std::env::current_dir().ok()?;
    let cwd = Utf8PathBuf::from_path_buf(cwd).ok()?;
    [
        cwd.join("x07/docs/examples"),
        cwd.join("../x07/docs/examples"),
        cwd.join("../../x07/docs/examples"),
    ]
    .into_iter()
    .find(|candidate| candidate.exists())
}

fn sandbox_vm_guest_bundle_declared() -> bool {
    std::env::var_os("X07_VM_VZ_GUEST_BUNDLE").is_some()
}

fn copy_example_tree(source: &Utf8Path, destination: &Utf8Path) -> anyhow::Result<()> {
    fs::create_dir_all(destination)?;
    for entry in fs::read_dir(source.as_std_path())
        .with_context(|| format!("failed to read example directory `{source}`"))?
    {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if should_skip_seed_path(&name) {
            continue;
        }
        let source_path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|path| anyhow!("example path is not UTF-8: {}", path.display()))?;
        let destination_path = destination.join(name.as_ref());
        if source_path.is_dir() {
            copy_example_tree(source_path.as_path(), destination_path.as_path())?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::copy(source_path.as_std_path(), destination_path.as_std_path()).with_context(
                || format!("failed to copy `{source_path}` to `{destination_path}`"),
            )?;
        }
    }
    Ok(())
}

fn should_skip_seed_path(name: &str) -> bool {
    matches!(name, ".git" | ".x07" | "target" | "dist" | "node_modules")
}

fn intent_packet_from_raw(
    session: &SessionSnapshot,
    raw: &str,
    input_mode: IntentInputMode,
    revision_notes: &[String],
) -> IntentPacket {
    let normalized = raw.trim();
    let normalized = if normalized.is_empty() {
        "Build a certifiable workflow graph optimizer. A human gives task durations and dependency edges. The project must compute a deterministic makespan, reject cycles, prove the pure core, and keep all agent actions visible before implementation."
    } else {
        normalized
    };
    let lowered = normalized.to_ascii_lowercase();
    let has_any = |needles: &[&str]| needles.iter().any(|needle| lowered.contains(needle));
    let is_sorter = has_any(&["sort"]);
    let is_incident = has_any(&["incident", "repair"]);
    let is_state_machine = has_any(&["state machine", "x07 sm"]);
    let is_gateway = has_any(&["api gateway", "x07-api-gateway"]);
    let is_crawler = has_any(&["crawler", "x07crawl"]);
    let is_db_guard = has_any(&["db migration", "x07dbguard", "drift guard"]);
    let is_atlas = has_any(&["x07_atlas", "x07 atlas", "wasm_showcases/x07_atlas"]);
    let is_workflow_graph = has_any(&[
        "workflow graph",
        "workflow-graph",
        "makespan",
        "task durations",
        "dependency edges",
    ]);
    let is_greeter = has_any(&["greet", "hello", "salut"]);
    let is_calculator = has_any(&["calculator", " calc ", "add two numbers", "arithmetic"]);
    let is_parser = has_any(&["parser", "parse json", "tokenize", "lex "]);
    let is_validator = has_any(&["validator", "validate ", "schema check"]);
    let is_cli_tool = has_any(&["cli tool", "command line tool", "command-line tool"]);
    let is_service = has_any(
        [
            "http service",
            "web service",
            "api service",
            "service that handles",
        ]
        .as_ref(),
    );
    let spec_target = if input_mode == IntentInputMode::Spec {
        spec_target_from_raw(normalized)
    } else {
        None
    };
    let (module_id, entry) = if let Some((module_id, entry)) = spec_target {
        (module_id, entry)
    } else if is_sorter {
        ("toy.sorter".to_string(), "sort_u8_asc".to_string())
    } else if is_atlas {
        ("atlas.app".to_string(), "atlas_dev".to_string())
    } else if is_db_guard {
        ("db.guard".to_string(), "verify_drift".to_string())
    } else if is_gateway {
        ("gateway.core".to_string(), "route_request_v1".to_string())
    } else if is_crawler {
        ("crawl.plan".to_string(), "plan_crawl_v1".to_string())
    } else if is_state_machine {
        ("workflow.lifecycle".to_string(), "step_v1".to_string())
    } else if is_incident {
        (
            "ops.incident_repair".to_string(),
            "classify_and_repair".to_string(),
        )
    } else if is_workflow_graph {
        ("workflow.graph".to_string(), "makespan_u32".to_string())
    } else if is_greeter {
        ("app.greeter".to_string(), "greet_v1".to_string())
    } else if is_calculator {
        ("app.calculator".to_string(), "compute_v1".to_string())
    } else if is_parser {
        ("app.parser".to_string(), "parse_v1".to_string())
    } else if is_validator {
        ("app.validator".to_string(), "validate_v1".to_string())
    } else if is_service {
        ("app.service".to_string(), "handle_v1".to_string())
    } else if is_cli_tool {
        ("app.cli".to_string(), "run_v1".to_string())
    } else {
        // Friendly default for anything we don't recognize. Previously we
        // fell through to workflow.graph/makespan_u32 which surprised users
        // who asked for unrelated tooling.
        ("app.main".to_string(), "run_v1".to_string())
    };

    let mut witnesses = vec![
        Witness {
            kind: WitnessKind::DesiredBehavior,
            text: normalized.to_string(),
        },
        Witness {
            kind: WitnessKind::PolicyRequirement,
            text: "All agent work must flow through canonical x07/XTAL bindings.".to_string(),
        },
        Witness {
            kind: WitnessKind::ForbiddenBehavior,
            text: "Do not turn the prompt directly into unchecked source code.".to_string(),
        },
    ];
    if input_mode == IntentInputMode::Incident {
        witnesses.push(Witness {
            kind: WitnessKind::IncidentReport,
            text: normalized.to_string(),
        });
    } else if input_mode == IntentInputMode::Spec {
        witnesses.push(Witness {
            kind: WitnessKind::PolicyRequirement,
            text: "Use the provided x07 spec as the canonical behavioral source.".to_string(),
        });
    }

    let mut policy_implications =
        vec!["OS worlds, network, budget, and trust widening require explicit review.".to_string()];
    if is_gateway || is_crawler || is_db_guard {
        policy_implications.push(
            "RR fixtures, sandbox policy, and OS/network/db capability widening require explicit review."
                .to_string(),
        );
    } else if is_state_machine {
        policy_implications.push(
            "Generated outputs, arch contracts, and budget profiles require drift evidence before certify."
                .to_string(),
        );
    }

    let mut constraints = vec![
        "Use spec-first XTAL flow.".to_string(),
        "Keep solve worlds deterministic by default.".to_string(),
        "Route spec-changing repairs back to human approval.".to_string(),
    ];
    if input_mode == IntentInputMode::Spec {
        constraints
            .push("Treat the provided spec as already-authored behavioral intent.".to_string());
    }
    constraints.extend(
        revision_notes
            .iter()
            .filter(|note| !note.trim().is_empty())
            .map(|note| format!("Revision request: {}", note.trim())),
    );

    IntentPacket {
        schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
        session_id: session.session_id,
        workspace_root: session.root.clone(),
        task_type: session.task_type.clone(),
        targets: vec![IntentTarget {
            module_id: module_id.clone(),
            entry: Some(entry.clone()),
        }],
        examples: vec![
            "Input examples become spec examples before implementation.".to_string(),
            "Generated tests must be reviewable before verify.".to_string(),
        ],
        constraints,
        policy_implications,
        ambiguities: vec![
            "Acceptance examples need final human approval.".to_string(),
            "Proof strictness should be selected before certify.".to_string(),
        ],
        assumptions: vec![
            "Agent may edit implementation paths after spec approval.".to_string(),
            "Agent may not widen specs or architecture policy without approval.".to_string(),
        ],
        witnesses,
        source: match input_mode {
            IntentInputMode::Text => IntentSource::Text {
                raw: normalized.to_string(),
            },
            IntentInputMode::Voice => IntentSource::Voice {
                transcript: normalized.to_string(),
            },
            IntentInputMode::Spec => IntentSource::Spec {
                raw: normalized.to_string(),
            },
            IntentInputMode::Incident => IntentSource::Incident {
                path: manual_incident_bundle_path(
                    session.session_id,
                    normalized,
                    &module_id,
                    &entry,
                ),
            },
        },
        clarification_history: session
            .intent
            .as_ref()
            .map(|intent| intent.clarification_history.clone())
            .unwrap_or_default(),
    }
}

fn non_empty(value: &str) -> Option<&str> {
    let trimmed = value.trim();
    (!trimmed.is_empty()).then_some(trimmed)
}

async fn apply_provider_intent_polish(
    providers: &ProviderProber,
    profile: Option<ProviderProfile>,
    intent: &mut IntentPacket,
    raw: &str,
    input_mode: &IntentInputMode,
    revision_notes: &[String],
    provider_profile_id: &str,
) -> serde_json::Value {
    let Some(profile) = profile else {
        return serde_json::json!({
            "schema_version": "x07.studio.intent_polish_report@0.1.0",
            "profile_id": provider_profile_id,
            "ok": false,
            "skipped": true,
            "notes": ["provider profile was not configured; deterministic intent was used"]
        });
    };
    let request = ProviderIntentPolishRequest {
        raw: raw.to_string(),
        input_mode: format!("{input_mode:?}"),
        revision_notes: revision_notes.to_vec(),
        deterministic_intent: serde_json::to_value(&*intent).unwrap_or_default(),
    };
    match providers.polish_intent(&profile, &request).await {
        Ok(report) => {
            if let Some(json) = &report.json {
                merge_provider_intent_polish(intent, json);
            }
            serde_json::to_value(report).unwrap_or_else(|error| {
                serde_json::json!({
                    "schema_version": "x07.studio.intent_polish_report@0.1.0",
                    "profile_id": provider_profile_id,
                    "ok": false,
                    "notes": [format!("provider report serialization failed: {error}")]
                })
            })
        }
        Err(error) => serde_json::json!({
            "schema_version": "x07.studio.intent_polish_report@0.1.0",
            "profile_id": provider_profile_id,
            "ok": false,
            "notes": [format!("provider polish failed: {error}; deterministic intent was used")]
        }),
    }
}

fn merge_provider_intent_polish(intent: &mut IntentPacket, polish: &serde_json::Value) {
    append_string_array(&mut intent.examples, polish.get("examples"));
    append_string_array(&mut intent.constraints, polish.get("constraints"));
    append_string_array(
        &mut intent.policy_implications,
        polish.get("policy_implications"),
    );
    append_string_array(&mut intent.ambiguities, polish.get("ambiguities"));
    append_string_array(&mut intent.assumptions, polish.get("assumptions"));
    append_witnesses(&mut intent.witnesses, polish.get("witnesses"));
}

fn append_string_array(target: &mut Vec<String>, value: Option<&serde_json::Value>) {
    let Some(items) = value.and_then(serde_json::Value::as_array) else {
        return;
    };
    for item in items.iter().filter_map(serde_json::Value::as_str).take(16) {
        append_unique(target, item);
    }
}

fn append_unique(target: &mut Vec<String>, item: &str) {
    let trimmed = item.trim();
    if trimmed.is_empty() {
        return;
    }
    let bounded = trimmed.chars().take(512).collect::<String>();
    if !target.iter().any(|existing| existing == &bounded) {
        target.push(bounded);
    }
}

fn append_witnesses(target: &mut Vec<Witness>, value: Option<&serde_json::Value>) {
    let Some(items) = value.and_then(serde_json::Value::as_array) else {
        return;
    };
    for item in items.iter().take(16) {
        let Some(kind) = item
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .and_then(witness_kind_from_str)
        else {
            continue;
        };
        let Some(text) = item.get("text").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() {
            continue;
        }
        let bounded = trimmed.chars().take(512).collect::<String>();
        if !target
            .iter()
            .any(|existing| existing.kind == kind && existing.text == bounded)
        {
            target.push(Witness {
                kind,
                text: bounded,
            });
        }
    }
}

fn witness_kind_from_str(value: &str) -> Option<WitnessKind> {
    match value.trim().to_ascii_lowercase().as_str() {
        "desired_behavior" | "desired" => Some(WitnessKind::DesiredBehavior),
        "forbidden_behavior" | "forbidden" => Some(WitnessKind::ForbiddenBehavior),
        "policy_requirement" | "policy" => Some(WitnessKind::PolicyRequirement),
        "incident_report" | "incident" => Some(WitnessKind::IncidentReport),
        _ => None,
    }
}

fn spec_target_from_raw(raw: &str) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;
    let module_id = value.get("module_id")?.as_str()?.trim();
    if module_id.is_empty() {
        return None;
    }
    let operation = value.get("operations")?.as_array()?.first()?;
    let operation_name = operation
        .get("name")
        .and_then(|item| item.as_str())
        .or_else(|| operation.get("id").and_then(|item| item.as_str()))
        .unwrap_or("run_v1");
    Some((
        module_id.to_string(),
        entry_from_spec_operation(module_id, operation_name),
    ))
}

fn entry_from_spec_operation(module_id: &str, operation_name: &str) -> String {
    let mut entry = operation_name.trim();
    if let Some(stripped) = entry.strip_prefix("op.") {
        entry = stripped;
    }
    if let Some(stripped) = entry
        .strip_prefix(module_id)
        .and_then(|item| item.strip_prefix('.'))
    {
        entry = stripped;
    }
    if let Some(stripped) = entry.strip_suffix(".v1") {
        entry = stripped;
    }
    sanitize_op_name(entry)
}

fn incident_input_path(intent: &IntentPacket) -> Option<&str> {
    match &intent.source {
        IntentSource::Incident { path } => Some(path.as_str()),
        _ => None,
    }
}

fn manual_incident_bundle_path(
    session_id: Uuid,
    note: &str,
    module_id: &str,
    entry: &str,
) -> String {
    let id = manual_incident_id(session_id, note, module_id, entry);
    format!(".x07/studio/incidents/{id}")
}

fn manual_incident_id(session_id: Uuid, note: &str, module_id: &str, entry: &str) -> String {
    let repro = manual_incident_repro(session_id, note, module_id, entry);
    let bytes =
        serde_json::to_vec_pretty(&repro).expect("manual incident repro JSON should serialize");
    sha256_hex(&bytes)
}

fn persist_manual_incident_bundle(root: &Utf8Path, intent: &IntentPacket) -> anyhow::Result<()> {
    let Some(input_path) = incident_input_path(intent) else {
        return Ok(());
    };
    let target = intent
        .targets
        .first()
        .ok_or_else(|| anyhow!("incident intent has no target"))?;
    let entry = target.entry.as_deref().unwrap_or("classify_and_repair");
    let note = incident_note_text(intent);
    let repro = manual_incident_repro(intent.session_id, &note, &target.module_id, entry);
    let repro_bytes = serde_json::to_vec_pretty(&repro)?;
    let incident_id = sha256_hex(&repro_bytes);
    let expected_path = format!(".x07/studio/incidents/{incident_id}");
    if input_path != expected_path {
        return Err(anyhow!(
            "incident input path `{input_path}` does not match repro digest `{expected_path}`"
        ));
    }

    let bundle_dir = safe_artifact_path(root, input_path)?;
    fs::create_dir_all(bundle_dir.as_path())?;
    let repro_path = bundle_dir.join("repro.json");
    fs::write(repro_path.as_path(), &repro_bytes)?;

    let violation = serde_json::json!({
        "schema_version": "x07.xtal.violation@0.1.0",
        "kind": "contract_violation",
        "id": incident_id,
        "clause_id": "studio_manual_incident",
        "world": "solve-pure",
        "source": {
            "mode": "studio",
            "test_entry": format!("{}.{}", target.module_id, entry),
        },
        "repro": {
            "path": "repro.json",
            "sha256": incident_id,
            "bytes_len": repro_bytes.len(),
        },
        "original_repro_path": format!("{input_path}/repro.json"),
        "generated_at": "2000-01-01T00:00:00Z",
    });
    fs::write(
        bundle_dir.join("violation.json").as_path(),
        serde_json::to_vec_pretty(&violation)?,
    )?;
    Ok(())
}

fn manual_incident_repro(
    session_id: Uuid,
    note: &str,
    module_id: &str,
    entry: &str,
) -> serde_json::Value {
    serde_json::json!({
        "schema_version": "x07.contract.repro@0.1.0",
        "tool": {
            "x07_version": "studio",
            "x07c_version": "studio",
        },
        "source": {
            "mode": "x07run",
            "target_kind": "project",
            "target_path": "x07.json",
        },
        "world": "solve-pure",
        "runner": {
            "solve_fuel": 0,
            "max_memory_bytes": 0,
            "max_output_bytes": 0,
            "cpu_time_limit_seconds": 0,
            "debug_borrow_checks": false,
        },
        "input_bytes_b64": "",
        "contract": {
            "contract_kind": "requires",
            "fn": format!("{module_id}.{entry}"),
            "clause_id": "studio_manual_incident",
            "clause_index": 0,
            "clause_ptr": "/studio/manual_incident",
            "witness": [{
                "ty": "text",
                "note": note,
                "studio_session_id": session_id.to_string(),
            }],
        },
    })
}

fn incident_note_text(intent: &IntentPacket) -> String {
    intent
        .witnesses
        .iter()
        .find(|witness| witness.kind == WitnessKind::IncidentReport)
        .or_else(|| intent.witnesses.first())
        .map(|witness| witness.text.clone())
        .unwrap_or_else(|| "Manual incident note captured by Studio.".to_string())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn write_incident_xtal_manifest(path: &Utf8Path, intent: &IntentPacket) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = intent.targets.first();
    let module_id = target
        .map(|item| item.module_id.as_str())
        .unwrap_or("ops.incident_repair");
    let entry = target
        .and_then(|item| item.entry.as_deref())
        .unwrap_or("classify_and_repair");
    let manifest = serde_json::json!({
        "schema_version": "x07.xtal.manifest@0.1.0",
        "xtal_version": "1.0",
        "spec_roots": ["spec/"],
        "impl_roots": ["src/"],
        "entrypoints": [{
            "name": format!("{module_id}.{entry}"),
            "kind": "defn",
        }],
        "profiles": {
            "dev_world": "solve-pure",
            "ci_world": "solve-pure",
            "prod_world": "solve-pure",
        },
        "trust": {
            "review_gates": ["incident_improve"],
            "cert_profile": "arch/trust/profiles/studio.json",
        },
        "autonomy": {
            "agent_write_paths": ["src/", "tests/"],
            "agent_write_specs": false,
            "agent_write_arch": false,
            "max_repair_iters": 1,
        },
    });
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

fn write_build_xtal_manifest(path: &Utf8Path, intent: &IntentPacket) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let target = intent.targets.first();
    let module_id = target
        .map(|item| item.module_id.as_str())
        .unwrap_or("app.main");
    let entry = target
        .and_then(|item| item.entry.as_deref())
        .unwrap_or("run_v1");
    let manifest = serde_json::json!({
        "schema_version": "x07.xtal.manifest@0.1.0",
        "xtal_version": "1.0",
        "spec_roots": ["spec/"],
        "impl_roots": ["src/"],
        "entrypoints": [{
            "name": format!("{module_id}.{entry}"),
            "kind": "defn",
        }],
        "profiles": {
            "dev_world": "solve-pure",
            "ci_world": "solve-pure",
            "prod_world": "solve-pure",
        },
        "trust": {
            "review_gates": ["build_repair"],
            "cert_profile": "arch/trust/profiles/studio.json",
        },
        "autonomy": {
            "agent_write_paths": ["src/", "tests/"],
            "agent_write_specs": false,
            "agent_write_arch": false,
            "max_repair_iters": 3,
        },
    });
    fs::write(path, serde_json::to_vec_pretty(&manifest)?)?;
    Ok(())
}

fn xtal_manifest_ensure_op(
    session_id: Uuid,
    wrote_manifest: bool,
    error: Option<anyhow::Error>,
) -> OpRecord {
    let now = now_string();
    let failed = error.is_some();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "xtal.manifest.ensure".to_string(),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "xtal".to_string(),
            "manifest".to_string(),
            "ensure".to_string(),
            "arch/xtal/xtal.json".to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: if failed {
            OperationStatus::Failed
        } else {
            OperationStatus::Succeeded
        },
        exit_code: Some(if failed { 1 } else { 0 }),
        artifacts: if failed {
            Vec::new()
        } else {
            vec!["arch/xtal/xtal.json".to_string()]
        },
        notes: Some(if failed {
            "Failed to prepare XTAL manifest for incident improvement.".to_string()
        } else if wrote_manifest {
            "Prepared XTAL manifest so incident ingest can resolve recovery evidence.".to_string()
        } else {
            "Existing XTAL manifest kept for incident improvement.".to_string()
        }),
        stdout: if failed {
            None
        } else if wrote_manifest {
            Some("Wrote arch/xtal/xtal.json for incident improvement.".to_string())
        } else {
            Some("Using existing arch/xtal/xtal.json.".to_string())
        },
        stderr: error.as_ref().map(ToString::to_string),
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.xtal_manifest_ensure@0.1.0",
            "ok": !failed,
            "path": "arch/xtal/xtal.json",
            "wrote": wrote_manifest && !failed,
            "error": error.as_ref().map(ToString::to_string),
        })),
        report_path: None,
    }
}

fn intent_formalize_op(
    session_id: Uuid,
    intent: &IntentPacket,
    input_mode: IntentInputMode,
    revision_notes: &[String],
    provider_polish: Option<serde_json::Value>,
) -> OpRecord {
    let now = now_string();
    let source = match input_mode {
        IntentInputMode::Text => "text",
        IntentInputMode::Voice => "voice",
        IntentInputMode::Spec => "spec",
        IntentInputMode::Incident => "incident",
    };
    let mut artifacts = vec![format!(".x07/studio/sessions/{session_id}.json")];
    if let Some(path) = incident_input_path(intent) {
        artifacts.push(path.to_string());
        artifacts.push(format!("{path}/violation.json"));
        artifacts.push(format!("{path}/repro.json"));
    }
    let provider_used = provider_polish
        .as_ref()
        .and_then(|value| value.get("ok"))
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let notes = if provider_polish.is_some() {
        "Formalized human input into a reviewable XTAL intent packet with provider polish evidence."
            .to_string()
    } else {
        "Formalized human input into a reviewable XTAL intent packet.".to_string()
    };
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "intent.formalize".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "intent".to_string(),
            "formalize".to_string(),
            source.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts,
        notes: Some(notes),
        stdout: Some(format!(
            "Intent formalized from {source}; {} witnesses, {} constraints, {} revision notes, provider polish: {}.",
            intent.witnesses.len(),
            intent.constraints.len(),
            revision_notes.len(),
            if provider_used { "applied" } else if provider_polish.is_some() { "recorded" } else { "not requested" }
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.intent_formalize_report@0.1.0",
            "input_mode": source,
            "target": intent.targets.first(),
            "revision_notes": revision_notes,
            "intent": intent,
            "provider_polish": provider_polish,
        })),
        report_path: None,
    }
}

fn intent_revision_request_op(session_id: Uuid, note: &str, revision_index: usize) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "intent.revision.request".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "intent".to_string(),
            "request-changes".to_string(),
            revision_index.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![format!(".x07/studio/sessions/{session_id}.json")],
        notes: Some("Human requested changes before spec approval.".to_string()),
        stdout: Some(format!(
            "Revision request {revision_index} recorded; approval remains blocked until intent repolish."
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.intent_revision_request@0.1.0",
            "revision_index": revision_index,
            "note": note,
            "approval_state": "changes",
        })),
        report_path: None,
    }
}

fn build_stage_op(session_id: Uuid, stage: &str, round: u32) -> OpRecord {
    let now = now_string();
    let (notes, stdout) = match stage {
        "start" => (
            "Build pipeline started.".to_string(),
            "Understanding what you want.".to_string(),
        ),
        "repair" => (
            format!("Repair round {round}."),
            format!("Fixing an issue I found (round {round})."),
        ),
        "done" => (
            "Build pipeline finished successfully.".to_string(),
            "Built and verified.".to_string(),
        ),
        "needs_help" => (
            "Build pipeline paused.".to_string(),
            "I need a human to help me get unblocked.".to_string(),
        ),
        other => (format!("Build stage `{other}`."), other.to_string()),
    };
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("build.stage.{stage}"),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "build".to_string(),
            "stage".to_string(),
            stage.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: if stage == "needs_help" {
            OperationStatus::Failed
        } else {
            OperationStatus::Succeeded
        },
        exit_code: Some(if stage == "needs_help" { 1 } else { 0 }),
        artifacts: vec![format!(".x07/studio/sessions/{session_id}.json")],
        notes: Some(notes),
        stdout: Some(stdout),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.build_stage@0.1.0",
            "stage": stage,
            "round": round,
        })),
        report_path: None,
    }
}

#[allow(dead_code)]
fn build_plain_english_summary(session: &SessionSnapshot) -> Option<OpRecord> {
    build_plain_english_summary_with_root(session, None)
}

fn build_plain_english_summary_with_root(
    session: &SessionSnapshot,
    root: Option<&Utf8Path>,
) -> Option<OpRecord> {
    let summary = crate::summarize::plain_english_summary_with_root(session, root)?;
    let now = now_string();
    let session_id = session.session_id;
    let stdout = std::iter::once(summary.headline.clone())
        .chain(
            summary
                .behavior_promises
                .iter()
                .map(|item| format!("- {item}")),
        )
        .collect::<Vec<_>>()
        .join("\n");
    Some(OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "summary.plain_english".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "summary".to_string(),
            "plain-english".to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![format!(".x07/studio/sessions/{session_id}.json")],
        notes: Some("Plain-English summary of what was built.".to_string()),
        stdout: Some(stdout),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::to_value(&summary).unwrap_or_default()),
        report_path: None,
    })
}

fn intent_clarify_answers_op(
    session_id: Uuid,
    applied: &[(String, String, WitnessKind)],
) -> OpRecord {
    let now = now_string();
    let summary = applied
        .iter()
        .map(|(qid, text, _)| format!("{qid}: {text}"))
        .collect::<Vec<_>>()
        .join("; ");
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "intent.clarify.answers".to_string(),
        backend: "studio-kernel".to_string(),
        command: vec![
            "studio".to_string(),
            "intent".to_string(),
            "clarify-answers".to_string(),
            applied.len().to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![format!(".x07/studio/sessions/{session_id}.json")],
        notes: Some(format!(
            "Applied {} user-supplied answer(s) to the intent.",
            applied.len()
        )),
        stdout: Some(summary),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.intent_clarify_answers@0.1.0",
            "applied": applied
                .iter()
                .map(|(qid, text, kind)| serde_json::json!({
                    "question_id": qid,
                    "answer_text": text,
                    "witness_kind": kind,
                }))
                .collect::<Vec<_>>(),
        })),
        report_path: None,
    }
}

fn seeded_example_op(session_id: Uuid, template: WorkflowTemplate, source: &Utf8Path) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("project.seed.{}", template.id()),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "seed-example".to_string(),
            source.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![
            "x07.json".to_string(),
            "src/".to_string(),
            "tests/".to_string(),
        ],
        notes: Some(format!(
            "Seeded docs example `{}` into the workspace.",
            template.id()
        )),
        stdout: Some(format!("Seeded example from `{source}`.")),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.project_seed@0.1.0",
            "ok": true,
            "template": template.id(),
            "source": source.to_string(),
        })),
        report_path: None,
    }
}

fn failed_seed_op(
    session_id: Uuid,
    template: WorkflowTemplate,
    source: Option<&Utf8Path>,
    error: anyhow::Error,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("project.seed.{}", template.id()),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "seed-example".to_string(),
            source.map(Utf8Path::to_string).unwrap_or_default(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Failed,
        exit_code: Some(1),
        artifacts: Vec::new(),
        notes: Some(format!("Failed to seed docs example `{}`.", template.id())),
        stdout: None,
        stderr: Some(error.to_string()),
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.project_seed@0.1.0",
            "ok": false,
            "template": template.id(),
            "source": source.map(|path| path.to_string()),
            "error": error.to_string(),
        })),
        report_path: None,
    }
}

fn prepared_directory_op(session_id: Uuid, directory: &str) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!(
            "project.prepare.{}",
            directory.trim_end_matches('/').replace('/', ".")
        ),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "mkdir".to_string(),
            directory.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![directory.to_string()],
        notes: Some(format!(
            "Prepared `{directory}` for the documented workflow."
        )),
        stdout: Some(format!("Prepared directory `{directory}`.")),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.directory_prepare@0.1.0",
            "ok": true,
            "directory": directory,
        })),
        report_path: None,
    }
}

fn failed_directory_op(session_id: Uuid, directory: &str, error: anyhow::Error) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!(
            "project.prepare.{}",
            directory.trim_end_matches('/').replace('/', ".")
        ),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "mkdir".to_string(),
            directory.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Failed,
        exit_code: Some(1),
        artifacts: Vec::new(),
        notes: Some(format!("Failed to prepare `{directory}`.")),
        stdout: None,
        stderr: Some(error.to_string()),
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.directory_prepare@0.1.0",
            "ok": false,
            "directory": directory,
            "error": error.to_string(),
        })),
        report_path: None,
    }
}

fn platform_delivery_decode_op(session_id: Uuid) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "lp.deploy.accept.decode".to_string(),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "platform".to_string(),
            "deployment-id".to_string(),
            "decode".to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Failed,
        exit_code: Some(1),
        artifacts: Vec::new(),
        notes: Some("Failed to read the platform deployment id from accept output.".to_string()),
        stdout: None,
        stderr: Some(
            "x07lp accept did not return result.exec_id or result.deployment_id".to_string(),
        ),
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.platform_delivery_decode@0.1.0",
            "ok": false,
            "expected": ["exec_id", "deployment_id", "result.exec_id", "result.deployment_id"],
        })),
        report_path: None,
    }
}

fn sanitize_op_name(value: &str) -> String {
    let mut output = String::new();
    let mut last_was_underscore = false;
    for ch in value.chars() {
        let next = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if next == '_' {
            if !last_was_underscore && !output.is_empty() {
                output.push(next);
            }
            last_was_underscore = true;
        } else {
            output.push(next);
            last_was_underscore = false;
        }
    }
    output.trim_matches('_').to_string()
}

fn last_op_failed(snapshot: &SessionSnapshot) -> bool {
    snapshot
        .op_log
        .last()
        .is_some_and(|op| op.status == OperationStatus::Failed)
}

fn session_artifact_recorded(session: &SessionSnapshot, artifact: &str) -> bool {
    session.op_log.iter().any(|op| {
        op.artifacts.iter().any(|item| item == artifact)
            || op.report_path.as_deref() == Some(artifact)
    })
}

fn safe_artifact_path(root: &Utf8Path, artifact: &str) -> anyhow::Result<Utf8PathBuf> {
    if artifact.trim().is_empty() || artifact.contains('\0') {
        return Err(anyhow!("artifact path is empty or invalid"));
    }
    let rel = Utf8Path::new(artifact);
    if rel.is_absolute() {
        return Err(anyhow!("artifact path must be relative"));
    }
    if rel.components().any(|component| {
        matches!(
            component,
            camino::Utf8Component::ParentDir | camino::Utf8Component::Prefix(_)
        )
    }) {
        return Err(anyhow!("artifact path must stay inside the workspace"));
    }
    Ok(root.join(rel))
}

fn normalize_doc_ref(doc_ref: &str) -> anyhow::Result<String> {
    let normalized = doc_ref.trim().trim_end_matches('/').to_string();
    if normalized.is_empty() || normalized.contains('\0') {
        return Err(anyhow!("doc ref is empty or invalid"));
    }
    if !normalized.starts_with("x07/docs/") {
        return Err(anyhow!(
            "doc ref `{normalized}` must start with `x07/docs/`"
        ));
    }
    let rel = Utf8Path::new(
        normalized
            .strip_prefix("x07/docs/")
            .expect("prefix checked above"),
    );
    if rel.is_absolute() {
        return Err(anyhow!("doc ref must be relative"));
    }
    if rel.components().any(|component| {
        matches!(
            component,
            camino::Utf8Component::ParentDir | camino::Utf8Component::Prefix(_)
        )
    }) {
        return Err(anyhow!("doc ref must stay inside x07 docs"));
    }
    Ok(normalized)
}

fn safe_doc_ref_path(docs_root: &Utf8Path, rel: &str) -> anyhow::Result<Utf8PathBuf> {
    let root = canonical_utf8_path(docs_root)?;
    let candidate = root.join(rel);
    let target = canonical_utf8_path(candidate.as_path())?;
    if !target.starts_with(root.as_path()) {
        return Err(anyhow!("doc ref must stay inside x07 docs"));
    }
    Ok(target)
}

fn canonical_utf8_path(path: &Utf8Path) -> anyhow::Result<Utf8PathBuf> {
    Utf8PathBuf::from_path_buf(
        fs::canonicalize(path).with_context(|| format!("canonicalize: {path}"))?,
    )
    .map_err(|path| anyhow!("path is not UTF-8: {}", path.display()))
}

fn find_docs_root(root: &Utf8Path) -> Option<Utf8PathBuf> {
    if let Ok(value) = std::env::var("X07_STUDIO_X07_DOCS_ROOT") {
        let path = Utf8PathBuf::from(value);
        if path.exists() {
            return Some(path);
        }
    }
    let mut candidates = vec![
        root.join("x07/docs"),
        root.join("../x07/docs"),
        root.join("../../x07/docs"),
    ];
    if let Ok(cwd) = std::env::current_dir().and_then(|path| {
        Utf8PathBuf::from_path_buf(path).map_err(|path| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("current dir is not UTF-8: {}", path.display()),
            )
        })
    }) {
        candidates.extend([
            cwd.join("x07/docs"),
            cwd.join("../x07/docs"),
            cwd.join("../../x07/docs"),
        ]);
    }
    candidates.into_iter().find(|candidate| candidate.exists())
}

fn doc_directory_entries(
    directory: &Utf8Path,
    doc_ref: &str,
    limit: usize,
) -> anyhow::Result<(Vec<DocPreviewEntry>, bool)> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(directory.as_std_path())
        .with_context(|| format!("read docs directory: {directory}"))?
    {
        let entry = entry?;
        let path = Utf8PathBuf::from_path_buf(entry.path())
            .map_err(|path| anyhow!("doc path is not UTF-8: {}", path.display()))?;
        let Some(name) = path.file_name() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let kind = if path.is_dir() {
            "directory"
        } else if is_previewable_doc_file(path.as_path()) {
            "file"
        } else {
            continue;
        };
        entries.push(DocPreviewEntry {
            path: format!("{doc_ref}/{name}"),
            title: doc_title_from_ref(path.as_path(), name),
            kind: kind.to_string(),
        });
    }
    entries.sort_by(|left, right| {
        let left_rank = if left.kind == "directory" { 0 } else { 1 };
        let right_rank = if right.kind == "directory" { 0 } else { 1 };
        left_rank
            .cmp(&right_rank)
            .then_with(|| left.path.cmp(&right.path))
    });
    let truncated = entries.len() > limit;
    entries.truncate(limit);
    Ok((entries, truncated))
}

fn is_previewable_doc_file(path: &Utf8Path) -> bool {
    matches!(
        path.extension(),
        Some("md" | "json" | "toml" | "yaml" | "yml")
    )
}

fn doc_title_from_ref(path: &Utf8Path, fallback: &str) -> String {
    path.file_stem()
        .or_else(|| path.file_name())
        .unwrap_or(fallback)
        .replace(['-', '_'], " ")
}

fn doc_media_kind(path: &Utf8Path) -> &'static str {
    match path.extension() {
        Some("md") => "markdown",
        Some("json") => "json",
        _ => "text",
    }
}

fn markdown_title(text: &str) -> Option<String> {
    text.lines()
        .find_map(|line| line.trim().strip_prefix("# "))
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToString::to_string)
}

fn doc_snippet(text: &str) -> String {
    let mut lines = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("# ") {
            continue;
        }
        lines.push(trimmed.to_string());
        if lines.len() == 8 {
            break;
        }
    }
    if lines.is_empty() {
        text.lines()
            .take(8)
            .map(str::trim)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        lines.join("\n")
    }
}

fn is_reviewable_patchset_target(path: &str) -> bool {
    let rel = Utf8Path::new(path);
    if rel.components().any(|component| match component {
        camino::Utf8Component::Normal(name) => name.starts_with('.'),
        _ => false,
    }) {
        return false;
    }
    if matches!(path, "x07.json" | "x07.lock.json") {
        return true;
    }
    [
        "src/",
        "tests/",
        "spec/",
        "arch/",
        "gen/",
        "wit/",
        "policy/",
        "policies/",
    ]
    .iter()
    .any(|prefix| path.starts_with(prefix))
}

fn apply_json_patch_preview(
    document: &mut serde_json::Value,
    patch: &[serde_json::Value],
) -> anyhow::Result<()> {
    for (index, op_value) in patch.iter().enumerate() {
        let op = op_value
            .as_object()
            .ok_or_else(|| anyhow!("patch operation {index} is not an object"))?;
        let op_name = op
            .get("op")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("patch operation {index} is missing `op`"))?;
        let path = op
            .get("path")
            .and_then(|value| value.as_str())
            .ok_or_else(|| anyhow!("patch operation {index} is missing `path`"))?;
        let tokens = decode_json_pointer(path)
            .with_context(|| format!("patch operation {index} has invalid path `{path}`"))?;
        match op_name {
            "add" => {
                let value = op
                    .get("value")
                    .cloned()
                    .ok_or_else(|| anyhow!("patch add operation {index} is missing `value`"))?;
                add_json_value(document, &tokens, value)
                    .with_context(|| format!("patch add operation {index} failed"))?;
            }
            "replace" => {
                let value = op
                    .get("value")
                    .cloned()
                    .ok_or_else(|| anyhow!("patch replace operation {index} is missing `value`"))?;
                replace_json_value(document, &tokens, value)
                    .with_context(|| format!("patch replace operation {index} failed"))?;
            }
            "remove" => {
                remove_json_value(document, &tokens)
                    .with_context(|| format!("patch remove operation {index} failed"))?;
            }
            "test" => {
                let expected = op
                    .get("value")
                    .ok_or_else(|| anyhow!("patch test operation {index} is missing `value`"))?;
                let actual = json_value_at(document, &tokens)
                    .with_context(|| format!("patch test operation {index} failed"))?;
                if actual != expected {
                    return Err(anyhow!("patch test operation {index} did not match"));
                }
            }
            other => return Err(anyhow!("unsupported JSON Patch operation `{other}`")),
        }
    }
    Ok(())
}

fn decode_json_pointer(path: &str) -> anyhow::Result<Vec<String>> {
    if path.is_empty() {
        return Ok(Vec::new());
    }
    if !path.starts_with('/') {
        return Err(anyhow!("JSON pointer must start with `/`"));
    }
    path.split('/')
        .skip(1)
        .map(decode_json_pointer_segment)
        .collect()
}

fn decode_json_pointer_segment(segment: &str) -> anyhow::Result<String> {
    let mut decoded = String::new();
    let mut chars = segment.chars();
    while let Some(ch) = chars.next() {
        if ch != '~' {
            decoded.push(ch);
            continue;
        }
        match chars.next() {
            Some('0') => decoded.push('~'),
            Some('1') => decoded.push('/'),
            Some(other) => return Err(anyhow!("unsupported JSON pointer escape `~{other}`")),
            None => return Err(anyhow!("unterminated JSON pointer escape")),
        }
    }
    Ok(decoded)
}

fn add_json_value(
    document: &mut serde_json::Value,
    path: &[String],
    value: serde_json::Value,
) -> anyhow::Result<()> {
    if path.is_empty() {
        *document = value;
        return Ok(());
    }
    let (parent_path, leaf) = path.split_at(path.len() - 1);
    let leaf = &leaf[0];
    let parent = json_value_at_mut(document, parent_path)?;
    match parent {
        serde_json::Value::Object(map) => {
            map.insert(leaf.clone(), value);
            Ok(())
        }
        serde_json::Value::Array(items) => {
            if leaf == "-" {
                items.push(value);
                return Ok(());
            }
            let index = parse_json_array_index(leaf)?;
            if index > items.len() {
                return Err(anyhow!("array index {index} is past the end"));
            }
            items.insert(index, value);
            Ok(())
        }
        other => Err(anyhow!("cannot add into {}", json_type_name(other))),
    }
}

fn replace_json_value(
    document: &mut serde_json::Value,
    path: &[String],
    value: serde_json::Value,
) -> anyhow::Result<()> {
    if path.is_empty() {
        *document = value;
        return Ok(());
    }
    let target = json_value_at_mut(document, path)?;
    *target = value;
    Ok(())
}

fn remove_json_value(document: &mut serde_json::Value, path: &[String]) -> anyhow::Result<()> {
    if path.is_empty() {
        *document = serde_json::Value::Null;
        return Ok(());
    }
    let (parent_path, leaf) = path.split_at(path.len() - 1);
    let leaf = &leaf[0];
    let parent = json_value_at_mut(document, parent_path)?;
    match parent {
        serde_json::Value::Object(map) => map
            .remove(leaf)
            .map(|_| ())
            .ok_or_else(|| anyhow!("object key `{leaf}` does not exist")),
        serde_json::Value::Array(items) => {
            let index = parse_json_array_index(leaf)?;
            if index >= items.len() {
                return Err(anyhow!("array index {index} is out of bounds"));
            }
            items.remove(index);
            Ok(())
        }
        other => Err(anyhow!("cannot remove from {}", json_type_name(other))),
    }
}

fn json_value_at<'a>(
    value: &'a serde_json::Value,
    path: &[String],
) -> anyhow::Result<&'a serde_json::Value> {
    let mut current = value;
    for token in path {
        current = match current {
            serde_json::Value::Object(map) => map
                .get(token)
                .ok_or_else(|| anyhow!("object key `{token}` does not exist"))?,
            serde_json::Value::Array(items) => {
                let index = parse_json_array_index(token)?;
                items
                    .get(index)
                    .ok_or_else(|| anyhow!("array index {index} is out of bounds"))?
            }
            other => return Err(anyhow!("cannot descend into {}", json_type_name(other))),
        };
    }
    Ok(current)
}

fn json_value_at_mut<'a>(
    value: &'a mut serde_json::Value,
    path: &[String],
) -> anyhow::Result<&'a mut serde_json::Value> {
    let mut current = value;
    for token in path {
        current = match current {
            serde_json::Value::Object(map) => map
                .get_mut(token)
                .ok_or_else(|| anyhow!("object key `{token}` does not exist"))?,
            serde_json::Value::Array(items) => {
                let index = parse_json_array_index(token)?;
                items
                    .get_mut(index)
                    .ok_or_else(|| anyhow!("array index {index} is out of bounds"))?
            }
            other => return Err(anyhow!("cannot descend into {}", json_type_name(other))),
        };
    }
    Ok(current)
}

fn parse_json_array_index(token: &str) -> anyhow::Result<usize> {
    if token.is_empty() {
        return Err(anyhow!("array index is empty"));
    }
    token
        .parse::<usize>()
        .with_context(|| format!("array index `{token}` is invalid"))
}

fn json_type_name(value: &serde_json::Value) -> &'static str {
    match value {
        serde_json::Value::Null => "null",
        serde_json::Value::Bool(_) => "boolean",
        serde_json::Value::Number(_) => "number",
        serde_json::Value::String(_) => "string",
        serde_json::Value::Array(_) => "array",
        serde_json::Value::Object(_) => "object",
    }
}

fn should_scaffold_spec(root: &Utf8Path, vars: &BTreeMap<String, String>) -> bool {
    let Some(input) = vars.get("input") else {
        return true;
    };
    !root.join(input).exists()
}

fn existing_spec_op(session_id: Uuid, input: &str) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: "spec.scaffold".to_string(),
        backend: "studio".to_string(),
        command: vec![
            "studio".to_string(),
            "skip".to_string(),
            "spec.scaffold".to_string(),
            input.to_string(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![input.to_string()],
        notes: Some(
            "Existing spec detected; scaffold skipped to preserve template alignment.".to_string(),
        ),
        stdout: Some(format!("Using existing spec `{input}`.")),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.scaffold_skip@0.1.0",
            "ok": true,
            "skipped": true,
            "reason": "existing_spec",
            "input": input,
        })),
        report_path: None,
    }
}

fn default_agent_profiles() -> Vec<AgentProfile> {
    let mut codex = AgentProfile::codex();
    codex.status = status_for_command(&codex.command);
    let mut claude = AgentProfile::claude_code();
    claude.status = status_for_command(&claude.command);
    vec![codex, claude]
}

fn status_for_command(command: &str) -> AgentStatus {
    if command_available(command) {
        AgentStatus::Available
    } else {
        AgentStatus::NeedsInstall
    }
}

fn ensure_agent_enabled(agent: &AgentProfile, action: &str) -> anyhow::Result<()> {
    if agent.status == AgentStatus::Disabled {
        return Err(anyhow!(
            "agent profile `{}` is disabled; enable it before {action}",
            agent.id
        ));
    }
    Ok(())
}

fn ensure_agent_command_available(agent: &AgentProfile) -> anyhow::Result<()> {
    if command_available(&agent.command) {
        return Ok(());
    }
    Err(anyhow!(
        "agent command `{}` for profile `{}` is not available; install it or update the profile command before supervised execution",
        agent.command,
        agent.id
    ))
}

fn command_available(command: &str) -> bool {
    let path = Path::new(command);
    if path.is_absolute() || command.contains('/') || command.contains('\\') {
        return path.is_file();
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(command).is_file())
}

fn agent_handoff_from_session(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    genpack: Option<&GenpackHandoffContext>,
) -> AgentHandoff {
    let prompt_path = format!(
        ".x07/studio/handoffs/{}-{}.md",
        session.session_id, agent.id
    );
    let command = std::iter::once(agent.command.clone())
        .chain(agent.args.clone())
        .chain(std::iter::once(prompt_path.clone()))
        .collect::<Vec<_>>();
    let artifacts = vec![
        prompt_path.clone(),
        format!(".x07/studio/sessions/{}.json", session.session_id),
        "x07.json".to_string(),
        "target/xtal/verify/summary.json".to_string(),
    ];
    let prompt = render_agent_handoff_prompt(session, agent, &command, genpack);
    AgentHandoff {
        schema_version: "x07.studio.agent_handoff@0.1.0".to_string(),
        session_id: session.session_id,
        agent_id: agent.id.clone(),
        agent_label: agent.label.clone(),
        command,
        prompt_path,
        prompt,
        allowed_verbs: agent.allowed_verbs.clone(),
        mcp_tools: agent.mcp_tools.clone(),
        write_roots: agent.write_roots.clone(),
        approval_required: agent.approval_required,
        artifacts,
        created_at: now_string(),
    }
}

fn render_agent_handoff_prompt(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    command: &[String],
    genpack: Option<&GenpackHandoffContext>,
) -> String {
    let mut out = String::new();
    out.push_str("# x07 Studio Agent Handoff\n\n");
    out.push_str(&format!("- Agent: {} (`{}`)\n", agent.label, agent.id));
    out.push_str(&format!(
        "- Session: {} (`{}`)\n",
        session.title, session.session_id
    ));
    out.push_str(&format!("- Workspace: `{}`\n", session.root));
    out.push_str(&format!("- Phase: `{:?}`\n", session.phase));
    out.push_str(&format!("- Command: `{}`\n", command.join(" ")));
    out.push_str("\n## Guardrails\n\n");
    out.push_str("- Stay inside the XTAL lifecycle; do not generate unchecked source directly from the prompt.\n");
    out.push_str("- Use only the allowed verbs and write roots listed below.\n");
    out.push_str("- Ask for human approval before changing specs, architecture, policy, network access, or trust boundaries.\n");
    out.push_str(
        "- Record every x07 command and artifact path so Studio can show the worklog.\n\n",
    );
    out.push_str("## Execution Boundary\n\n");
    out.push_str(
        "- Use `x07 run` as the default execution front door for runnable X07 programs.\n",
    );
    out.push_str(
        "- Keep solve-pure deterministic by default; OS, sandbox, network, release, provenance, and budget widening require approval.\n",
    );
    out.push_str(
        "- Read the `X07_STUDIO_*` environment variables for the machine-readable session contract; they mirror the allowed verbs, write roots, handoff path, and agent event schema.\n",
    );
    for boundary in handoff_execution_boundaries(session) {
        out.push_str(&format!("- {boundary}\n"));
    }
    out.push('\n');
    out.push_str("## Automation Runbook\n\n");
    for step in handoff_automation_runbook(session) {
        out.push_str(&format!("- {step}\n"));
    }
    out.push('\n');
    out.push_str("## Allowed Verbs\n\n");
    for verb in &agent.allowed_verbs {
        out.push_str(&format!("- `{verb}`\n"));
    }
    out.push_str("\n## MCP Tools\n\n");
    for tool in &agent.mcp_tools {
        out.push_str(&format!("- `{tool}`\n"));
    }
    out.push_str("\n## Write Roots\n\n");
    for root in &agent.write_roots {
        out.push_str(&format!("- `{root}`\n"));
    }
    out.push_str(
        "\nThese roots are the supervised write contract; report any required write outside them as an approval event before acting.\n",
    );
    if let Some(contract) = &session.contract {
        out.push_str("\n## Session Contract\n\n");
        out.push_str("Canonical docs:\n");
        for doc_ref in &contract.global_doctrine.doc_refs {
            out.push_str(&format!("- `{doc_ref}`\n"));
        }
        out.push_str("\nContract MCP tools:\n");
        for tool in &contract.global_doctrine.mcp_tools {
            out.push_str(&format!("- `{tool}`\n"));
        }
        out.push('\n');
        out.push_str(&format!(
            "- XTAL manifest: `{}`\n",
            contract.project_doctrine.xtal_manifest
        ));
        out.push_str(&format!(
            "- Agent instructions: `{}`\n",
            contract.project_doctrine.agent_md
        ));
        out.push_str(&format!(
            "- Focus paths: `{}`\n",
            contract.task_doctrine.focus_paths.join("`, `")
        ));
        out.push_str(&format!(
            "- Baseline refs: `{}`\n",
            contract.task_doctrine.baseline_refs.join("`, `")
        ));
    }
    if let Some(intent) = &session.intent {
        out.push_str("\n## Approved Intent\n\n");
        out.push_str("Targets:\n");
        for target in &intent.targets {
            out.push_str(&format!(
                "- `{}` / `{}`\n",
                target.module_id,
                target.entry.as_deref().unwrap_or("run_v1")
            ));
        }
        out.push_str("\nConstraints:\n");
        for constraint in &intent.constraints {
            out.push_str(&format!("- {constraint}\n"));
        }
        out.push_str("\nWitnesses:\n");
        for witness in &intent.witnesses {
            out.push_str(&format!("- `{:?}`: {}\n", witness.kind, witness.text));
        }
    }
    if let Some(genpack) = genpack {
        render_genpack_section(&mut out, genpack);
    }
    out.push_str("\n## Required Loop\n\n");
    out.push_str("1. Re-read this handoff and the session contract.\n");
    out.push_str("2. Use x07 docs/MCP tools before selecting commands.\n");
    out.push_str("3. Produce or update artifacts only inside the permitted roots.\n");
    out.push_str("4. Run the canonical XTAL checks before reporting completion.\n");
    out.push_str("\n## Agent Event Protocol\n\n");
    out.push_str("Emit one JSON object per line for machine-visible milestones:\n\n");
    out.push_str("```json\n");
    out.push_str(
        r#"{"schema_version":"x07.studio.agent_event@0.1.0","kind":"artifact","summary":"verify summary ready","artifact":"target/xtal/verify/summary.json"}"#,
    );
    out.push_str("\n```\n\n");
    out.push_str(
        "`kind` must be one of `artifact`, `diagnostic`, `write`, or `approval`. Use `approval` whenever policy, spec, architecture, world, budget, trust, or release scope would widen.\n",
    );
    out
}

/// Builds a clarify-specific handoff that asks the agent to either generate
/// 1–3 plain-English clarifying questions or signal `clarify_done`. The agent
/// is restricted to the `intent.clarify` verb and given no write roots so the
/// post-run audit will pass cleanly.
fn agent_clarify_handoff_from_session(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    round: u32,
    genpack: Option<&GenpackHandoffContext>,
) -> AgentHandoff {
    let mut clarify_agent = agent.clone();
    clarify_agent.allowed_verbs = vec!["intent.clarify".to_string()];
    clarify_agent.write_roots = Vec::new();
    let prompt_path = format!(
        ".x07/studio/handoffs/{}-{}-clarify.md",
        session.session_id, agent.id
    );
    let command = std::iter::once(agent.command.clone())
        .chain(agent.args.clone())
        .chain(std::iter::once(prompt_path.clone()))
        .collect::<Vec<_>>();
    let artifacts = vec![
        prompt_path.clone(),
        format!(".x07/studio/sessions/{}.json", session.session_id),
    ];
    let prompt = render_agent_clarify_prompt(session, &clarify_agent, &command, round, genpack);
    AgentHandoff {
        schema_version: "x07.studio.agent_handoff@0.1.0".to_string(),
        session_id: session.session_id,
        agent_id: agent.id.clone(),
        agent_label: agent.label.clone(),
        command,
        prompt_path,
        prompt,
        allowed_verbs: clarify_agent.allowed_verbs.clone(),
        mcp_tools: clarify_agent.mcp_tools.clone(),
        write_roots: clarify_agent.write_roots.clone(),
        approval_required: false,
        artifacts,
        created_at: now_string(),
    }
}

fn agent_realize_handoff_from_session(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    stub_paths: &[String],
    genpack: Option<&GenpackHandoffContext>,
) -> AgentHandoff {
    let mut realize_agent = agent.clone();
    realize_agent.allowed_verbs = vec![
        "impl.sync.write".to_string(),
        "spec.check".to_string(),
        "xtal.verify".to_string(),
    ];
    realize_agent.write_roots = vec!["src/".to_string(), "tests/".to_string()];
    let prompt_path = format!(
        ".x07/studio/handoffs/{}-{}-realize.md",
        session.session_id, agent.id
    );
    let command = std::iter::once(agent.command.clone())
        .chain(agent.args.clone())
        .chain(std::iter::once(prompt_path.clone()))
        .collect::<Vec<_>>();
    let artifacts = vec![
        prompt_path.clone(),
        format!(".x07/studio/sessions/{}.json", session.session_id),
    ];
    let prompt =
        render_agent_realize_prompt(session, &realize_agent, &command, stub_paths, genpack);
    AgentHandoff {
        schema_version: "x07.studio.agent_handoff@0.1.0".to_string(),
        session_id: session.session_id,
        agent_id: agent.id.clone(),
        agent_label: agent.label.clone(),
        command,
        prompt_path,
        prompt,
        allowed_verbs: realize_agent.allowed_verbs.clone(),
        mcp_tools: realize_agent.mcp_tools.clone(),
        write_roots: realize_agent.write_roots.clone(),
        approval_required: false,
        artifacts,
        created_at: now_string(),
    }
}

fn render_agent_realize_prompt(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    command: &[String],
    stub_paths: &[String],
    _genpack: Option<&GenpackHandoffContext>,
) -> String {
    let mut out = String::new();
    out.push_str("# x07 Studio — Realize Implementation\n\n");
    out.push_str(&format!("- Agent: {} (`{}`)\n", agent.label, agent.id));
    out.push_str(&format!(
        "- Session: {} (`{}`)\n",
        session.title, session.session_id
    ));
    out.push_str(&format!("- Workspace: `{}`\n", session.root));
    out.push_str(&format!("- Command: `{}`\n", command.join(" ")));
    out.push_str(
        "\nThe scaffold step produced **stub** function bodies under `src/`. \
Your job: replace them with **real implementations** that satisfy the approved spec, \
keep `xtal.verify` green, and stay inside the write roots below.\n\n",
    );
    out.push_str("## Stubs to replace\n\n");
    if stub_paths.is_empty() {
        out.push_str("- (Studio could not locate the stub modules. Walk `src/` and find any `defn` with an empty or trivial body — typically a single `bytes.empty` or `i32.lit` expression.)\n");
    } else {
        for path in stub_paths {
            out.push_str(&format!("- `{path}`\n"));
        }
    }
    out.push_str("\n## Write Roots\n\n");
    for root in &agent.write_roots {
        out.push_str(&format!("- `{root}`\n"));
    }
    out.push_str(
        "\nStudio's write-root audit runs after you exit. Any files written outside these roots will fail the run.\n",
    );
    out.push_str("\n## Required Loop\n\n");
    out.push_str(
        "1. Read the approved intent + spec below. Note the operation signatures, examples, and witnesses.\n\
2. Open each stub module and replace the `defn` body with a real implementation in x07AST.\n\
3. After every meaningful change, run `x07 xtal impl check --project x07.json` and \
`x07 xtal verify --project x07.json --allow-os-world` to confirm the impl matches the spec.\n\
4. Do NOT widen the spec, the architecture manifest, the trust profile, or any policy file. \
If a spec change is needed, emit an `approval` agent_event and STOP.\n\
5. When verify is clean, exit. Studio will re-run impl.check + xtal.verify and surface a fresh \
Verified turn with the summary.\n\n",
    );
    if let Some(intent) = &session.intent {
        out.push_str("## Approved Intent\n\n");
        for target in &intent.targets {
            out.push_str(&format!(
                "- Target: `{}` / `{}`\n",
                target.module_id,
                target.entry.as_deref().unwrap_or("run_v1")
            ));
        }
        if !intent.witnesses.is_empty() {
            out.push_str("\n### Witnesses\n");
            for witness in &intent.witnesses {
                if witness.text.trim().is_empty() {
                    continue;
                }
                out.push_str(&format!("- `{:?}`: {}\n", witness.kind, witness.text));
            }
        }
        if !intent.examples.is_empty() {
            out.push_str("\n### Examples\n");
            for example in &intent.examples {
                let text = example.trim();
                if text.is_empty() {
                    continue;
                }
                out.push_str(&format!("- {text}\n"));
            }
        }
        if !intent.constraints.is_empty() {
            out.push_str("\n### Constraints\n");
            for constraint in &intent.constraints {
                let text = constraint.trim();
                if text.is_empty() {
                    continue;
                }
                out.push_str(&format!("- {text}\n"));
            }
        }
    }
    out.push_str("\n## Agent Event Protocol\n\n");
    out.push_str(
        "Emit one JSON object per line for machine-visible milestones. Use `kind` = \
`write` (file you edited), `artifact` (report you produced), `diagnostic` (problem you \
saw), or `approval` (you need a spec/arch/policy widen).\n",
    );
    out
}

fn render_agent_clarify_prompt(
    session: &SessionSnapshot,
    agent: &AgentProfile,
    command: &[String],
    round: u32,
    genpack: Option<&GenpackHandoffContext>,
) -> String {
    let mut out = String::new();
    out.push_str("# x07 Studio — Intent Clarify Round\n\n");
    out.push_str(&format!("- Agent: {} (`{}`)\n", agent.label, agent.id));
    out.push_str(&format!(
        "- Session: {} (`{}`)\n",
        session.title, session.session_id
    ));
    out.push_str(&format!("- Workspace: `{}`\n", session.root));
    out.push_str(&format!("- Round: {round}\n"));
    out.push_str(&format!("- Command: `{}`\n", command.join(" ")));
    out.push_str(
        "\nYou are running a **clarify** round. You do not write any source code or files. \
Read the draft intent below and the prior clarification history. Then **either** emit \
1-3 short clarifying questions (one JSON object per line) **or** emit a single \
`clarify_done` event if the intent is already specific enough to scaffold a spec.\n\n",
    );
    out.push_str("## Output Protocol\n\n");
    out.push_str("Emit lines like this, one JSON object per line:\n\n");
    out.push_str("```json\n");
    out.push_str(
        r#"{"schema_version":"x07.studio.agent_event@0.1.0","kind":"clarify_question","id":"q1","text":"Should empty input reject with an error or return an empty result?","witness_kind":"desired_behavior","options":["Reject with error","Return empty result"]}"#,
    );
    out.push('\n');
    out.push_str(
        r#"{"schema_version":"x07.studio.agent_event@0.1.0","kind":"clarify_done","summary":"Intent is specific enough to proceed."}"#,
    );
    out.push_str("\n```\n\n");
    out.push_str(
        "Rules:\n- Each question must have `id`, `text`, and `witness_kind` (one of \
`desired_behavior`, `forbidden_behavior`, `policy_requirement`, `incident_report`).\n\
- `options` is optional but encouraged for binary or small enum choices.\n\
- Keep `text` plain English, no x07/XTAL jargon. Address the user directly.\n\
- Prefer 1 high-leverage question over 3 low-leverage ones.\n\
- Do NOT propose code, paths, schemas, or commands. This is intent only.\n\
- If you need nothing more, emit a single `clarify_done` line and stop.\n",
    );
    if let Some(intent) = &session.intent {
        out.push_str("\n## Draft Intent\n\n");
        for target in &intent.targets {
            out.push_str(&format!(
                "- Target: `{}` / `{}`\n",
                target.module_id,
                target.entry.as_deref().unwrap_or("run_v1")
            ));
        }
        match &intent.source {
            IntentSource::Text { raw } | IntentSource::Spec { raw } => {
                out.push_str(&format!("\nUser input:\n\n```\n{raw}\n```\n"));
            }
            IntentSource::Voice { transcript } => {
                out.push_str(&format!("\nVoice transcript:\n\n```\n{transcript}\n```\n"));
            }
            IntentSource::Incident { path } => {
                out.push_str(&format!("\nIncident path: `{path}`\n"));
            }
            IntentSource::Sketch { path } => {
                out.push_str(&format!("\nSketch artifact: `{path}`\n"));
            }
            IntentSource::Image { path, mime } => {
                out.push_str(&format!("\nImage artifact: `{path}` (`{mime}`)\n"));
            }
        }
        if !intent.witnesses.is_empty() {
            out.push_str("\nAccumulated witnesses:\n");
            for witness in &intent.witnesses {
                out.push_str(&format!("- `{:?}`: {}\n", witness.kind, witness.text));
            }
        }
        if !intent.constraints.is_empty() {
            out.push_str("\nConstraints:\n");
            for constraint in &intent.constraints {
                out.push_str(&format!("- {constraint}\n"));
            }
        }
        if !intent.ambiguities.is_empty() {
            out.push_str("\nOpen ambiguities:\n");
            for ambiguity in &intent.ambiguities {
                out.push_str(&format!("- {ambiguity}\n"));
            }
        }
        if !intent.clarification_history.is_empty() {
            out.push_str("\nPrevious Q&A:\n");
            for turn in &intent.clarification_history {
                out.push_str(&format!(
                    "- Q (round {}, `{}`): {}\n",
                    turn.round, turn.question_id, turn.question_text
                ));
                if let Some(answer) = &turn.answer_text {
                    out.push_str(&format!("  A: {}\n", answer));
                }
            }
        }
    }
    if let Some(genpack) = genpack {
        render_genpack_section(&mut out, genpack);
    }
    out.push_str(
        "\nWhen you are done emitting events, exit. The supervisor will read your output.\n",
    );
    out
}

fn render_genpack_section(out: &mut String, genpack: &GenpackHandoffContext) {
    out.push_str("\n## Service Genpack Context\n\n");
    out.push_str(&format!("- Detected archetype: `{}`\n", genpack.archetype));
    out.push_str(
        "- Use this archetype contract when drafting service manifests or generated service artifacts.\n",
    );
    if let Some(schema) = &genpack.schema {
        out.push_str("\nJSON Schema:\n\n```json\n");
        match serde_json::to_string(schema) {
            Ok(raw) => out.push_str(&raw),
            Err(_) => out.push_str("{}"),
        }
        out.push_str("\n```\n");
    } else {
        out.push_str("\nJSON Schema: unavailable from local `x07 service genpack schema`.\n");
    }
    if let Some(grammar) = &genpack.grammar {
        out.push_str("\nGrammar:\n\n```text\n");
        out.push_str(grammar.trim());
        out.push_str("\n```\n");
    } else {
        out.push_str("\nGrammar: unavailable from local `x07 service genpack grammar`.\n");
    }
}

fn handoff_execution_boundaries(session: &SessionSnapshot) -> Vec<String> {
    let haystack = handoff_haystack(session);
    let has = |needle: &str| haystack.contains(needle);
    let mut boundaries = vec![
        "solve-pure: default lane for spec, generated tests, implementation checks, and verification."
            .to_string(),
    ];
    if has("solve-rr") || has("/rr/") || has("cassette") || has("replay") {
        boundaries.push(
            "solve-rr: replay fixtures and cassettes must be recorded as evidence before trust review."
                .to_string(),
        );
    }
    if has("sandbox") || has("run-os") || has("dbguard") || has("migration") {
        boundaries.push(
            "sandbox/run-os: OS, filesystem, database, and network capability changes require human approval."
                .to_string(),
        );
    }
    if has("x07-wasm") || has("wasm") || has("app profile") || has("app build") {
        boundaries.push(
            "WASM app: profile validation, trace replay, pack verification, and app artifacts must stay visible."
                .to_string(),
        );
    }
    if has("release") || has("provenance") || has("deploy") || has("pack") {
        boundaries.push(
            "release/provenance: pack, provenance, deploy, and trust evidence are separate approval gates."
                .to_string(),
        );
    }
    if has("budget") || has("slo") || has("profile") {
        boundaries.push(
            "SLO/budget: budget profile and SLO evidence must be preserved before certification."
                .to_string(),
        );
    }
    boundaries
}

fn handoff_automation_runbook(session: &SessionSnapshot) -> Vec<String> {
    let haystack = handoff_haystack(session);
    let has = |needle: &str| haystack.contains(needle);
    let mut steps = vec![
        "`intent.formalize` -> `.x07/studio/sessions/intent.json` (approved intent packet)."
            .to_string(),
        "`approve_spec` -> session contract lock (human gate; agents cannot self-approve)."
            .to_string(),
        "`project.init.xtal-pure` -> `x07.json` and XTAL project scaffold.".to_string(),
    ];
    match session.task_type {
        TaskType::BrownfieldExtract => steps.push(
            "`spec.extract` -> `target/xtal/spec.extract.report.json` before implementation writes."
                .to_string(),
        ),
        TaskType::IncidentRepair => steps.extend([
            "`xtal.ingest --normalize-only` -> canonical violation/repro evidence.".to_string(),
            "`xtal.improve` -> incident-tied regression evidence before repair trust.".to_string(),
        ]),
        _ => steps.extend([
            "`spec.scaffold` / `spec.check` -> reviewed x07spec artifacts.".to_string(),
            "`tests.gen.write` -> generated XTAL tests from approved examples.".to_string(),
        ]),
    }
    steps.extend([
        "`impl.sync.write` / `impl.check` -> implementation changes inside approved write roots."
            .to_string(),
        "`xtal.verify` -> `target/xtal/verify/summary.json` before completion.".to_string(),
        "`xtal.repair` is allowed only from failed verification or incident evidence.".to_string(),
        "`xtal.certify` is evidence-only after trust review.".to_string(),
    ]);
    if has("x07-wasm") || has("wasm") || has("app profile") || has("app build") {
        steps.extend([
            "`x07-wasm app profile validate` -> app profile evidence.".to_string(),
            "`x07-wasm app build` -> app bundle artifacts.".to_string(),
            "`x07-wasm app test` -> deterministic trace replay evidence.".to_string(),
        ]);
    }
    if has("release") || has("provenance") || has("deploy") || has("pack") {
        steps.extend([
            "`x07-wasm app pack` / `app verify` -> release pack evidence.".to_string(),
            "`x07-wasm provenance attest` / `provenance verify` -> provenance evidence."
                .to_string(),
            "`x07lp deploy accept/run/query/status` -> visible local platform delivery evidence."
                .to_string(),
        ]);
    }
    if has("budget") || has("slo") || has("profile") {
        steps.push(
            "`x07-wasm slo eval` -> SLO and budget evidence before certification.".to_string(),
        );
    }
    steps
}

fn handoff_haystack(session: &SessionSnapshot) -> String {
    let mut parts = Vec::new();
    parts.push(format!("{:?}", session.task_type));
    parts.push(format!("{:?}", session.phase));
    if let Some(intent) = &session.intent {
        for target in &intent.targets {
            parts.push(target.module_id.clone());
            if let Some(entry) = &target.entry {
                parts.push(entry.clone());
            }
        }
        parts.extend(intent.examples.iter().cloned());
        parts.extend(intent.constraints.iter().cloned());
        parts.extend(intent.policy_implications.iter().cloned());
        parts.extend(intent.ambiguities.iter().cloned());
        parts.extend(intent.assumptions.iter().cloned());
        for witness in &intent.witnesses {
            parts.push(witness.text.clone());
        }
        match &intent.source {
            IntentSource::Text { raw } | IntentSource::Spec { raw } => parts.push(raw.clone()),
            IntentSource::Voice { transcript } => parts.push(transcript.clone()),
            IntentSource::Incident { path } => parts.push(path.clone()),
            IntentSource::Sketch { path } => parts.push(path.clone()),
            IntentSource::Image { path, mime } => {
                parts.push(path.clone());
                parts.push(mime.clone());
            }
        }
    }
    if let Some(contract) = &session.contract {
        parts.extend(contract.global_doctrine.doc_refs.iter().cloned());
        parts.extend(contract.global_doctrine.mcp_tools.iter().cloned());
        parts.push(contract.project_doctrine.xtal_manifest.clone());
        parts.push(contract.project_doctrine.agent_md.clone());
        parts.extend(contract.project_doctrine.write_policy.paths.iter().cloned());
        parts.extend(contract.task_doctrine.focus_paths.iter().cloned());
        parts.extend(contract.task_doctrine.baseline_refs.iter().cloned());
    }
    for op in &session.op_log {
        parts.push(op.op.clone());
        parts.extend(op.command.iter().cloned());
        parts.extend(op.artifacts.iter().cloned());
        if let Some(notes) = &op.notes {
            parts.push(notes.clone());
        }
    }
    parts.join(" ").to_ascii_lowercase()
}

fn agent_run_is_approved(session: &SessionSnapshot, agent_id: &str) -> bool {
    let approval_op = format!("agent.approval.{agent_id}");
    let handoff_op = format!("agent.handoff.{agent_id}");
    let plan_op = format!("agent.supervise.{agent_id}");
    let run_op = format!("agent.run.{agent_id}");
    session
        .op_log
        .iter()
        .rev()
        .find(|op| {
            op.op == approval_op || op.op == handoff_op || op.op == plan_op || op.op == run_op
        })
        .is_some_and(|op| op.op == approval_op && op.status == OperationStatus::Succeeded)
}

fn agent_approval_op(session_id: Uuid, agent: &AgentProfile, reason: &str) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("agent.approval.{}", agent.id),
        backend: "human-approval".to_string(),
        command: vec!["approve-agent".to_string(), agent.id.clone()],
        started_at: now,
        finished_at: None,
        status: OperationStatus::Pending,
        exit_code: None,
        artifacts: Vec::new(),
        notes: Some(reason.to_string()),
        stdout: Some(format!(
            "Approval required for {} before supervised execution.",
            agent.label
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "agent_id": &agent.id,
            "agent_label": &agent.label,
            "approval_required": agent.approval_required,
            "allowed_verbs": &agent.allowed_verbs,
            "write_roots": &agent.write_roots,
            "reason": reason,
        })),
        report_path: None,
    }
}

fn agent_plan_op(
    session_id: Uuid,
    agent: &AgentProfile,
    handoff: &AgentHandoff,
    prompt_path: &Utf8Path,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("agent.supervise.{}", agent.id),
        backend: "studio".to_string(),
        command: handoff.command.clone(),
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts: vec![prompt_path.to_string()],
        notes: Some(format!(
            "Recorded supervised launch plan for {}.",
            agent.label
        )),
        stdout: Some(format!(
            "Supervised launch prepared.\nCommand: {}\nPrompt: {}\n",
            handoff.command.join(" "),
            handoff.prompt_path
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "plan",
            "handoff": handoff,
        })),
        report_path: None,
    }
}

fn agent_running_op(
    session_id: Uuid,
    agent: &AgentProfile,
    handoff: &AgentHandoff,
    prompt_path: &Utf8Path,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("agent.run.{}", agent.id),
        backend: "agent-supervisor".to_string(),
        command: handoff.command.clone(),
        started_at: now,
        finished_at: None,
        status: OperationStatus::Running,
        exit_code: None,
        artifacts: vec![prompt_path.to_string()],
        notes: Some(format!(
            "{} is running under Studio supervision.",
            agent.label
        )),
        stdout: Some(format!(
            "Supervised command started.\nCommand: {}\nPrompt: {}\n",
            handoff.command.join(" "),
            handoff.prompt_path
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "execute",
            "handoff": handoff,
        })),
        report_path: None,
    }
}

fn agent_clarify_running_op(
    session_id: Uuid,
    agent: &AgentProfile,
    handoff: &AgentHandoff,
    prompt_path: &Utf8Path,
    round: u32,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("agent.clarify.{}", agent.id),
        backend: "agent-supervisor".to_string(),
        command: handoff.command.clone(),
        started_at: now,
        finished_at: None,
        status: OperationStatus::Running,
        exit_code: None,
        artifacts: vec![prompt_path.to_string()],
        notes: Some(format!(
            "{} is generating clarifying questions (round {round}).",
            agent.label
        )),
        stdout: Some(format!(
            "Supervised clarify round started.\nCommand: {}\nPrompt: {}\n",
            handoff.command.join(" "),
            handoff.prompt_path
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "clarify",
            "round": round,
            "handoff": handoff,
        })),
        report_path: None,
    }
}

fn agent_realize_running_op(
    session_id: Uuid,
    agent: &AgentProfile,
    handoff: &AgentHandoff,
    prompt_path: &Utf8Path,
) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: format!("agent.realize.{}", agent.id),
        backend: "agent-supervisor".to_string(),
        command: handoff.command.clone(),
        started_at: now,
        finished_at: None,
        status: OperationStatus::Running,
        exit_code: None,
        artifacts: vec![prompt_path.to_string()],
        notes: Some(format!(
            "{} is filling in the implementation under src/.",
            agent.label
        )),
        stdout: Some(format!(
            "Supervised realize started.\nCommand: {}\nPrompt: {}\n",
            handoff.command.join(" "),
            handoff.prompt_path
        )),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "realize",
            "handoff": handoff,
        })),
        report_path: None,
    }
}

fn agent_command_env(command: &AgentCommandPlan) -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "X07_STUDIO_SESSION_ID".to_string(),
            command.session_id.to_string(),
        ),
        ("X07_STUDIO_AGENT_ID".to_string(), command.agent.id.clone()),
        (
            "X07_STUDIO_AGENT_LABEL".to_string(),
            command.agent.label.clone(),
        ),
        (
            "X07_STUDIO_HANDOFF_PATH".to_string(),
            command.prompt_path.to_string(),
        ),
        (
            "X07_STUDIO_ALLOWED_VERBS".to_string(),
            command.agent.allowed_verbs.join(","),
        ),
        (
            "X07_STUDIO_MCP_TOOLS".to_string(),
            command.agent.mcp_tools.join(","),
        ),
        (
            "X07_STUDIO_WRITE_ROOTS".to_string(),
            command.agent.write_roots.join(","),
        ),
        (
            "X07_STUDIO_APPROVAL_REQUIRED".to_string(),
            command.agent.approval_required.to_string(),
        ),
        (
            "X07_STUDIO_EVENT_SCHEMA".to_string(),
            "x07.studio.agent_event@0.1.0".to_string(),
        ),
    ])
}

fn agent_streaming_op(command: &AgentCommandPlan, update: CommandStreamUpdate) -> OpRecord {
    let stdout = format!(
        "Supervised command streaming.\nCommand: {}\nPrompt: {}\n\n{}",
        command.handoff.command.join(" "),
        command.handoff.prompt_path,
        update.stdout
    );
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: command.op_id,
        session_id: command.session_id,
        op: format!("agent.{}.{}", command.op_kind, command.agent.id),
        backend: "agent-supervisor".to_string(),
        command: command.handoff.command.clone(),
        started_at: command.handoff.created_at.clone(),
        finished_at: None,
        status: OperationStatus::Running,
        exit_code: None,
        artifacts: vec![command.prompt_path.to_string()],
        notes: Some(format!(
            "Streaming {} output under Studio supervision.",
            command.agent.label
        )),
        stdout: Some(stdout),
        stderr: if update.stderr.is_empty() {
            None
        } else {
            Some(update.stderr)
        },
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "execute",
            "streaming": true,
            "handoff": command.handoff,
        })),
        report_path: None,
    }
}

#[derive(Debug, Default)]
struct AgentSemanticEventState {
    seen: BTreeSet<String>,
}

#[derive(Debug)]
struct AgentSemanticEvent {
    kind: String,
    line: String,
    artifact: Option<String>,
    structured: Option<serde_json::Value>,
}

fn agent_semantic_ops(
    command: &AgentCommandPlan,
    update: &CommandStreamUpdate,
    state: &mut AgentSemanticEventState,
) -> Vec<OpRecord> {
    update
        .stdout
        .lines()
        .chain(update.stderr.lines())
        .filter_map(classify_agent_output_line)
        .filter(|event| state.seen.insert(format!("{}:{}", event.kind, event.line)))
        .map(|event| agent_semantic_op(command, event))
        .collect()
}

fn classify_agent_output_line(line: &str) -> Option<AgentSemanticEvent> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    if let Some(event) = parse_structured_agent_event(line) {
        return Some(event);
    }
    let lower = line.to_ascii_lowercase();
    if lower.contains("approval")
        || lower.contains("approve")
        || lower.contains("requires human")
        || lower.contains("permission")
        || lower.contains("policy widening")
    {
        return Some(AgentSemanticEvent {
            kind: "approval".to_string(),
            line: line.to_string(),
            artifact: None,
            structured: None,
        });
    }
    if lower.contains("error:")
        || lower.contains("warning:")
        || lower.contains("diagnostic")
        || lower.contains("failed:")
    {
        return Some(AgentSemanticEvent {
            kind: "diagnostic".to_string(),
            line: line.to_string(),
            artifact: None,
            structured: None,
        });
    }
    if lower.starts_with("write:")
        || lower.starts_with("wrote ")
        || lower.starts_with("created ")
        || lower.starts_with("updated ")
        || lower.starts_with("patched ")
        || lower.starts_with("modified ")
    {
        return Some(AgentSemanticEvent {
            kind: "write".to_string(),
            line: line.to_string(),
            artifact: extract_artifact_path(line),
            structured: None,
        });
    }
    if let Some(artifact) = extract_artifact_path(line) {
        return Some(AgentSemanticEvent {
            kind: "artifact".to_string(),
            line: line.to_string(),
            artifact: Some(artifact),
            structured: None,
        });
    }
    None
}

fn parse_structured_agent_event(line: &str) -> Option<AgentSemanticEvent> {
    let value: serde_json::Value = serde_json::from_str(line).ok()?;
    if value.get("schema_version")?.as_str()? != "x07.studio.agent_event@0.1.0" {
        return None;
    }
    let kind = value.get("kind")?.as_str()?;
    if !matches!(
        kind,
        "artifact" | "diagnostic" | "write" | "approval" | "clarify_question" | "clarify_done"
    ) {
        return None;
    }
    let summary = value
        .get("summary")
        .and_then(serde_json::Value::as_str)
        .or_else(|| value.get("message").and_then(serde_json::Value::as_str))
        .or_else(|| value.get("text").and_then(serde_json::Value::as_str))
        .unwrap_or(kind);
    let artifact = value
        .get("artifact")
        .and_then(serde_json::Value::as_str)
        .filter(|artifact| looks_like_artifact_path(artifact))
        .map(str::to_string);
    Some(AgentSemanticEvent {
        kind: kind.to_string(),
        line: summary.to_string(),
        artifact,
        structured: Some(value),
    })
}

fn extract_artifact_path(line: &str) -> Option<String> {
    line.split_whitespace().find_map(|token| {
        let candidate = token.trim_matches(|ch: char| {
            matches!(
                ch,
                '"' | '\'' | '`' | ',' | ';' | ':' | '(' | ')' | '[' | ']'
            )
        });
        if looks_like_artifact_path(candidate) {
            Some(candidate.to_string())
        } else {
            None
        }
    })
}

fn looks_like_artifact_path(candidate: &str) -> bool {
    let has_known_prefix = [
        ".x07/", "arch/", "dist/", "gen/", "out/", "spec/", "src/", "target/", "tests/",
    ]
    .iter()
    .any(|prefix| candidate.starts_with(prefix));
    let has_known_suffix = [
        ".json",
        ".jsonl",
        ".x07.json",
        ".x07spec.json",
        ".patchset.json",
        ".md",
        ".txt",
    ]
    .iter()
    .any(|suffix| candidate.ends_with(suffix));
    has_known_prefix && has_known_suffix
}

fn agent_semantic_op(command: &AgentCommandPlan, event: AgentSemanticEvent) -> OpRecord {
    let now = now_string();
    let mut artifacts = vec![command.prompt_path.to_string()];
    if let Some(artifact) = &event.artifact {
        artifacts.push(artifact.clone());
    }
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id: command.session_id,
        op: format!("agent.event.{}.{}", command.agent.id, event.kind),
        backend: "agent-observer".to_string(),
        command: vec![
            "observe-agent".to_string(),
            command.agent.id.clone(),
            event.kind.clone(),
        ],
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Succeeded,
        exit_code: Some(0),
        artifacts,
        notes: Some(format!(
            "Observed {} event from {} output.",
            event.kind, command.agent.label
        )),
        stdout: Some(event.line.clone()),
        stderr: None,
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "schema_version": "x07.studio.agent_semantic_event@0.1.0",
            "kind": event.kind,
            "line": event.line,
            "artifact": event.artifact,
            "structured": event.structured,
            "agent_id": command.agent.id,
            "handoff": command.handoff,
        })),
        report_path: None,
    }
}

const AGENT_WORKSPACE_SNAPSHOT_FILE_LIMIT: usize = 20_000;

#[derive(Debug, Clone, Default)]
struct AgentWorkspaceSnapshot {
    files: BTreeMap<String, String>,
    truncated: bool,
}

#[derive(Debug, Clone, Default)]
struct AgentWriteAudit {
    allowed_roots: Vec<String>,
    created: Vec<String>,
    modified: Vec<String>,
    deleted: Vec<String>,
    violations: Vec<String>,
    truncated: bool,
}

fn snapshot_agent_workspace(root: &Utf8Path) -> AgentWorkspaceSnapshot {
    let mut snapshot = AgentWorkspaceSnapshot::default();
    collect_agent_workspace_snapshot(
        root.as_std_path(),
        root.as_std_path(),
        &mut snapshot.files,
        &mut snapshot.truncated,
    );
    snapshot
}

fn collect_agent_workspace_snapshot(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, String>,
    truncated: &mut bool,
) {
    if *truncated {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if files.len() >= AGENT_WORKSPACE_SNAPSHOT_FILE_LIMIT {
            *truncated = true;
            return;
        }
        let path = entry.path();
        let Ok(file_type) = entry.file_type() else {
            continue;
        };
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
            if should_skip_agent_snapshot_dir(&path) {
                continue;
            }
            collect_agent_workspace_snapshot(root, &path, files, truncated);
            if *truncated {
                return;
            }
            continue;
        }
        if !file_type.is_file() {
            continue;
        }
        let Some(relative) = relative_workspace_path(root, &path) else {
            continue;
        };
        if let Some(fingerprint) = file_fingerprint(&path) {
            files.insert(relative, fingerprint);
        }
    }
}

fn should_skip_agent_snapshot_dir(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                ".git" | ".svelte-kit" | "build" | "node_modules" | "target"
            )
        })
}

fn relative_workspace_path(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    Some(relative.to_string_lossy().replace('\\', "/"))
}

fn file_fingerprint(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(format!("{}:{}", bytes.len(), sha256_hex(&bytes)))
}

fn agent_write_audit(
    command: &AgentCommandPlan,
    before: AgentWorkspaceSnapshot,
    after: AgentWorkspaceSnapshot,
) -> AgentWriteAudit {
    let allowed_roots = agent_allowed_write_roots(command);
    let mut audit = AgentWriteAudit {
        allowed_roots: allowed_roots.clone(),
        truncated: before.truncated || after.truncated,
        ..AgentWriteAudit::default()
    };

    for (path, after_hash) in &after.files {
        match before.files.get(path) {
            None => audit.created.push(path.clone()),
            Some(before_hash) if before_hash != after_hash => audit.modified.push(path.clone()),
            _ => {}
        }
    }
    for path in before.files.keys() {
        if !after.files.contains_key(path) {
            audit.deleted.push(path.clone());
        }
    }

    audit.created.sort();
    audit.modified.sort();
    audit.deleted.sort();
    audit.violations = audit
        .created
        .iter()
        .chain(audit.modified.iter())
        .chain(audit.deleted.iter())
        .filter(|path| !is_agent_write_allowed(path, &allowed_roots))
        .cloned()
        .collect();
    audit.violations.sort();
    audit.violations.dedup();
    audit
}

fn agent_allowed_write_roots(command: &AgentCommandPlan) -> Vec<String> {
    let mut roots = command
        .agent
        .write_roots
        .iter()
        .map(|root| normalize_agent_write_root(root))
        .collect::<Vec<_>>();
    roots.push(".x07/studio/".to_string());
    roots.sort();
    roots.dedup();
    roots
}

fn normalize_agent_write_root(root: &str) -> String {
    let normalized = root
        .replace('\\', "/")
        .trim_start_matches("./")
        .trim()
        .to_string();
    if normalized.is_empty() || normalized == "." {
        return ".".to_string();
    }
    if normalized.ends_with('/') {
        normalized
    } else {
        format!("{normalized}/")
    }
}

fn is_agent_write_allowed(path: &str, allowed_roots: &[String]) -> bool {
    allowed_roots.iter().any(|root| {
        if root == "." {
            true
        } else {
            path.starts_with(root)
        }
    })
}

fn agent_execution_op(
    command: AgentCommandPlan,
    execution: CommandExecution,
    before: AgentWorkspaceSnapshot,
) -> OpRecord {
    let audit = agent_write_audit(
        &command,
        before,
        snapshot_agent_workspace(command.cwd.as_path()),
    );
    let has_write_violations = !audit.violations.is_empty();
    let mut stderr = execution.stderr;
    if has_write_violations {
        let message = format!(
            "Studio write-root audit failed; unapproved workspace writes: {}",
            audit.violations.join(", ")
        );
        if stderr.trim().is_empty() {
            stderr = message;
        } else {
            stderr.push('\n');
            stderr.push_str(&message);
        }
    }
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: command.op_id,
        session_id: command.session_id,
        op: format!("agent.{}.{}", command.op_kind, command.agent.id),
        backend: "agent-supervisor".to_string(),
        command: std::iter::once(execution.program.clone())
            .chain(execution.args.clone())
            .collect(),
        started_at: execution.started_at,
        finished_at: Some(execution.finished_at),
        status: if execution.exit_code == Some(0) && !has_write_violations {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        },
        exit_code: execution.exit_code,
        artifacts: vec![command.prompt_path.to_string()],
        notes: Some(if has_write_violations {
            format!(
                "Ran {} under Studio supervision; write-root audit found unapproved writes.",
                command.agent.label
            )
        } else {
            format!("Ran {} under Studio supervision.", command.agent.label)
        }),
        stdout: Some(execution.stdout),
        stderr: Some(stderr),
        stdout_json: execution.stdout_json,
        stderr_json: execution.stderr_json,
        report_json: Some(serde_json::json!({
            "mode": "execute",
            "handoff": command.handoff,
            "write_audit": {
                "schema_version": "x07.studio.agent_write_audit@0.1.0",
                "allowed_roots": audit.allowed_roots,
                "created": audit.created,
                "modified": audit.modified,
                "deleted": audit.deleted,
                "violations": audit.violations,
                "truncated": audit.truncated,
            },
        })),
        report_path: None,
    }
}

fn agent_spawn_error_op(command: AgentCommandPlan, error: anyhow::Error) -> OpRecord {
    let now = now_string();
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: command.op_id,
        session_id: command.session_id,
        op: format!("agent.{}.{}", command.op_kind, command.agent.id),
        backend: "agent-supervisor".to_string(),
        command: command.handoff.command.clone(),
        started_at: now.clone(),
        finished_at: Some(now),
        status: OperationStatus::Failed,
        exit_code: None,
        artifacts: vec![command.prompt_path.to_string()],
        notes: Some(format!("Failed to launch {}.", command.agent.label)),
        stdout: None,
        stderr: Some(error.to_string()),
        stdout_json: None,
        stderr_json: None,
        report_json: Some(serde_json::json!({
            "mode": "execute",
            "handoff": command.handoff,
        })),
        report_path: None,
    }
}

fn op_record_from_binding(
    session_id: Uuid,
    binding_id: &str,
    executed: ExecutedBinding,
) -> OpRecord {
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: Uuid::new_v4(),
        session_id,
        op: binding_id.to_string(),
        backend: "cli".to_string(),
        command: std::iter::once(executed.execution.program.clone())
            .chain(executed.execution.args.clone())
            .collect(),
        started_at: executed.execution.started_at,
        finished_at: Some(executed.execution.finished_at),
        status: if executed.execution.exit_code == Some(0) {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        },
        exit_code: executed.execution.exit_code,
        artifacts: executed.rendered.artifacts,
        notes: Some(executed.rendered.notes),
        stdout: Some(executed.execution.stdout),
        stderr: Some(executed.execution.stderr),
        stdout_json: executed.execution.stdout_json,
        stderr_json: executed.execution.stderr_json,
        report_json: executed.report_json,
        report_path: executed.report_path.map(|path| path.to_string()),
    }
}

fn workspace_path_state(root: &Utf8Path, relative: &str) -> WorkspacePathState {
    let path = root.join(relative);
    WorkspacePathState {
        path: relative.to_string(),
        exists: path.exists(),
        modified_unix_ms: modified_unix_ms(path.as_path()),
    }
}

fn newest_workspace_file(root: &Utf8Path, relative_dir: &str) -> Option<WorkspacePathState> {
    let mut newest: Option<WorkspacePathState> = None;
    visit_workspace_files(root.join(relative_dir).as_path(), &mut |path| {
        let Some(relative) = path.strip_prefix(root).ok() else {
            return;
        };
        let state = WorkspacePathState {
            path: relative.to_string(),
            exists: true,
            modified_unix_ms: modified_unix_ms(path),
        };
        if state.modified_unix_ms >= newest.as_ref().and_then(|item| item.modified_unix_ms) {
            newest = Some(state);
        }
    });
    newest
}

fn count_files_matching<F>(dir: &Utf8Path, mut predicate: F) -> usize
where
    F: FnMut(&Utf8Path) -> bool,
{
    let mut count = 0;
    visit_workspace_files(dir, &mut |path| {
        if predicate(path) {
            count += 1;
        }
    });
    count
}

fn visit_workspace_files(dir: &Utf8Path, visitor: &mut dyn FnMut(&Utf8Path)) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(path) = Utf8PathBuf::from_path_buf(entry.path()) else {
            continue;
        };
        if path.is_dir() {
            visit_workspace_files(path.as_path(), visitor);
        } else if path.is_file() {
            visitor(path.as_path());
        }
    }
}

fn modified_unix_ms(path: &Utf8Path) -> Option<u64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    let millis = modified.duration_since(UNIX_EPOCH).ok()?.as_millis();
    u64::try_from(millis).ok()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use uuid::Uuid;

    use loom_types::api::{AgentRunMode, ApprovalDecision, IntentInputMode};
    use loom_types::artifacts::{
        AgentProfile, AgentStatus, IntentPacket, IntentSource, IntentTarget, OpRecord,
        OperationStatus, ProviderProfile, TaskType, Witness, WitnessKind,
    };
    use loom_types::ops::SessionEvent;
    use loom_types::session::SessionSnapshot;

    use super::{
        agent_clarify_handoff_from_session, agent_handoff_from_session,
        atlas_platform_delivery_vars, checked_xtal_verify_run_vars, copy_example_tree,
        entry_from_spec_operation, intent_packet_from_raw, platform_deployment_id_from_report,
        sha256_hex, should_scaffold_spec, workflow_template_from_intent,
        xtal_workflow_vars_from_intent, GenpackHandoffContext, WorkflowTemplate, WorkspaceKernel,
    };

    #[test]
    fn xtal_workflow_vars_use_safe_payload_param() {
        let intent = IntentPacket::demo(Uuid::nil(), "/workspace");
        let vars = xtal_workflow_vars_from_intent(&intent);

        assert_eq!(
            vars.get("module_id").map(String::as_str),
            Some("app.sorter")
        );
        assert_eq!(vars.get("op").map(String::as_str), Some("sort_ascending"));
        assert_eq!(vars.get("param").map(String::as_str), Some("payload:bytes"));
        assert_eq!(
            vars.get("input").map(String::as_str),
            Some("spec/app.sorter.x07spec.json")
        );
    }

    #[test]
    fn xtal_workflow_vars_map_incidents_to_bytes_result() {
        let intent = IntentPacket {
            schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
            session_id: Uuid::nil(),
            workspace_root: "/workspace".to_string(),
            task_type: TaskType::IncidentRepair,
            targets: vec![IntentTarget {
                module_id: "ops.incident_repair".to_string(),
                entry: Some("Classify And Repair!".to_string()),
            }],
            examples: vec![],
            constraints: vec![],
            policy_implications: vec![],
            ambiguities: vec![],
            assumptions: vec![],
            witnesses: vec![Witness {
                kind: WitnessKind::IncidentReport,
                text: "failed verify".to_string(),
            }],
            source: IntentSource::Incident {
                path: ".x07/studio/incidents/aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                    .to_string(),
            },
            clarification_history: Vec::new(),
        };

        let vars = xtal_workflow_vars_from_intent(&intent);

        assert_eq!(
            vars.get("module_id").map(String::as_str),
            Some("ops.incident_repair")
        );
        assert_eq!(
            vars.get("op").map(String::as_str),
            Some("classify_and_repair")
        );
        assert_eq!(vars.get("result").map(String::as_str), Some("bytes"));
    }

    #[test]
    fn xtal_workflow_run_vars_allow_only_verify_controls() {
        let vars = checked_xtal_verify_run_vars(&BTreeMap::from([
            ("proof_policy".to_string(), "strict".to_string()),
            ("unwind".to_string(), "2".to_string()),
        ]))
        .expect("verify vars accepted");

        assert_eq!(vars.get("proof_policy").map(String::as_str), Some("strict"));
        assert_eq!(vars.get("unwind").map(String::as_str), Some("2"));

        assert!(checked_xtal_verify_run_vars(&BTreeMap::from([(
            "input".to_string(),
            "spec/other.x07spec.json".to_string()
        )]))
        .is_err());
    }

    #[test]
    fn atlas_platform_delivery_vars_keep_artifacts_relative_and_commands_absolute() {
        let root = temp_root();
        let vars = atlas_platform_delivery_vars(root.as_path(), Some("lpexec_atlas"));

        assert_eq!(
            vars.get("pack_manifest").map(String::as_str),
            Some("dist/showcase_fullstack/pack.atlas_release/app.pack.json")
        );
        assert_eq!(
            vars.get("state_dir").map(String::as_str),
            Some(".x07/platform")
        );
        assert_eq!(
            vars.get("deployment_id").map(String::as_str),
            Some("lpexec_atlas")
        );
        assert!(vars
            .get("pack_manifest_arg")
            .expect("pack manifest arg")
            .starts_with(root.as_str()));
        assert!(vars
            .get("plan_arg")
            .expect("plan arg")
            .ends_with("dist/showcase_fullstack/deploy.atlas_release/deploy.plan.json"));
        assert!(vars
            .get("state_dir_arg")
            .expect("state dir arg")
            .ends_with(".x07/platform"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn platform_deployment_id_parses_direct_and_wrapped_accept_reports() {
        let direct = serde_json::json!({
            "schema_version": "lp.deploy.accept.stage@0.1.0",
            "run_id": "lprun_demo",
            "exec_id": "lpexec_direct",
            "decision_id": "lpdec_demo"
        });
        assert_eq!(
            platform_deployment_id_from_report(&direct).as_deref(),
            Some("lpexec_direct")
        );

        let wrapped = serde_json::json!({
            "schema_version": "lp.cli.report@0.1.0",
            "command": "deploy accept",
            "ok": true,
            "result": {
                "deployment_id": "lpexec_wrapped"
            }
        });
        assert_eq!(
            platform_deployment_id_from_report(&wrapped).as_deref(),
            Some("lpexec_wrapped")
        );
    }

    #[test]
    fn formalize_intent_creates_packet_and_visible_op() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root).expect("open kernel");
        let session = kernel
            .create_session("voice workflow", TaskType::NewBehavior)
            .expect("create session");

        let (intent, op, snapshot) = kernel
            .formalize_intent(
                session.session_id,
                "Transcript: follow docs/examples/agent-gate/xtal/workflow-graph and reject cycles.",
                IntentInputMode::Voice,
                &["Make cycle rejection explicit.".to_string()],
            )
            .expect("formalize intent");

        assert_eq!(intent.targets[0].module_id, "workflow.graph");
        assert!(matches!(intent.source, IntentSource::Voice { .. }));
        assert!(intent
            .constraints
            .iter()
            .any(|item| item.contains("Make cycle rejection explicit")));
        assert_eq!(op.op, "intent.formalize");
        assert_eq!(op.status, OperationStatus::Succeeded);
        assert_eq!(
            snapshot.revision_notes,
            vec!["Make cycle rejection explicit.".to_string()]
        );
        assert!(snapshot.intent.is_some());
        assert!(snapshot
            .op_log
            .iter()
            .any(|item| item.op == "intent.formalize"));
    }

    #[test]
    fn request_intent_revision_records_visible_blocker() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root).expect("open kernel");
        let session = kernel
            .create_session("revision loop", TaskType::NewBehavior)
            .expect("create session");

        let (op, snapshot) = kernel
            .request_intent_revision(session.session_id, "Keep empty input explicit.")
            .expect("request revision");

        assert_eq!(op.op, "intent.revision.request");
        assert_eq!(op.status, OperationStatus::Succeeded);
        assert_eq!(
            snapshot.revision_notes,
            vec!["Keep empty input explicit.".to_string()]
        );
        assert!(snapshot
            .op_log
            .iter()
            .any(|item| item.op == "intent.revision.request"));
    }

    #[tokio::test]
    async fn formalize_intent_with_provider_merges_polish_as_review_evidence() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut request = Vec::new();
            let mut buffer = [0_u8; 1024];
            loop {
                let read = stream.read(&mut buffer).await.expect("read request");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
                if request.windows(4).any(|window| window == b"\r\n\r\n") {
                    break;
                }
            }
            let header_len = request
                .windows(4)
                .position(|window| window == b"\r\n\r\n")
                .map(|index| index + 4)
                .expect("request headers");
            let header_text = String::from_utf8_lossy(&request[..header_len]);
            let content_length = header_text
                .lines()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            while request.len() < header_len + content_length {
                let read = stream.read(&mut buffer).await.expect("read request body");
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&buffer[..read]);
            }
            let request = String::from_utf8_lossy(&request);
            assert!(request.starts_with("POST /v1/chat/completions "));
            assert!(request.contains("\"model\":\"local-polisher\""));

            let content = serde_json::json!({
                "examples": ["Provider example: [1] -> [1]"],
                "constraints": ["Provider constraint: keep spec reviewable"],
                "ambiguities": ["Provider ambiguity: stability examples missing"],
                "witnesses": [
                    {
                        "kind": "forbidden_behavior",
                        "text": "Provider forbidden: unchecked code generation"
                    }
                ]
            })
            .to_string();
            let body = serde_json::json!({
                "choices": [
                    {
                        "message": {
                            "content": content
                        }
                    }
                ]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let mut profile = ProviderProfile::local_ollama();
        profile.id = "test-polisher".to_string();
        profile.base_url = format!("http://{addr}/v1");
        profile.model = Some("local-polisher".to_string());
        kernel
            .save_provider_profile(&profile)
            .expect("save provider profile");
        let session = kernel
            .create_session("provider polish", TaskType::NewBehavior)
            .expect("create session");

        let (intent, op, snapshot) = kernel
            .formalize_intent_with_provider(
                session.session_id,
                "Create a stable sorter with reviewable acceptance examples.",
                IntentInputMode::Text,
                &[],
                Some("test-polisher"),
            )
            .await
            .expect("formalize intent with provider");

        server.await.expect("server task");
        assert!(intent
            .examples
            .iter()
            .any(|item| item == "Provider example: [1] -> [1]"));
        assert!(intent
            .constraints
            .iter()
            .any(|item| item == "Provider constraint: keep spec reviewable"));
        assert!(intent
            .witnesses
            .iter()
            .any(|item| item.text == "Provider forbidden: unchecked code generation"));
        assert_eq!(op.op, "intent.formalize");
        assert_eq!(
            op.report_json
                .as_ref()
                .and_then(|value| value.get("provider_polish"))
                .and_then(|value| value.get("ok"))
                .and_then(serde_json::Value::as_bool),
            Some(true)
        );
        assert!(snapshot
            .op_log
            .iter()
            .any(|item| item.op == "intent.formalize"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn formalize_incident_persists_ingestable_violation_bundle() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("incident workflow", TaskType::IncidentRepair)
            .expect("create session");

        let (intent, op, _snapshot) = kernel
            .formalize_intent(
                session.session_id,
                "Incident note: runtime verify found a cycle handling failure.",
                IntentInputMode::Incident,
                &[],
            )
            .expect("formalize incident");

        let IntentSource::Incident { path } = intent.source else {
            panic!("expected incident source");
        };
        assert!(path.starts_with(".x07/studio/incidents/"));
        assert!(op.artifacts.contains(&format!("{path}/violation.json")));
        assert!(op.artifacts.contains(&format!("{path}/repro.json")));

        let violation_path = root.join(&path).join("violation.json");
        let repro_path = root.join(&path).join("repro.json");
        assert!(violation_path.is_file(), "missing {violation_path}");
        assert!(repro_path.is_file(), "missing {repro_path}");

        let repro_bytes = std::fs::read(repro_path).expect("read repro");
        let violation: serde_json::Value =
            serde_json::from_slice(&std::fs::read(violation_path).expect("read violation"))
                .expect("parse violation");
        assert_eq!(violation["schema_version"], "x07.xtal.violation@0.1.0");
        assert_eq!(violation["id"], sha256_hex(&repro_bytes));
        assert_eq!(violation["repro"]["sha256"], sha256_hex(&repro_bytes));
        assert_eq!(violation["clause_id"], "studio_manual_incident");
        assert_eq!(violation["world"], "solve-pure");
        assert_eq!(violation["generated_at"], "2000-01-01T00:00:00Z");

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn formalize_intent_maps_atlas_prompts_to_app_pipeline() {
        let root = temp_root();
        let session = SessionSnapshot::new(
            Uuid::nil(),
            "atlas",
            root.to_string(),
            TaskType::NewBehavior,
        );

        let intent = intent_packet_from_raw(
            &session,
            "Use docs/examples/wasm_showcases/x07_atlas to build x07 Atlas.",
            IntentInputMode::Text,
            &[],
        );

        assert_eq!(intent.targets[0].module_id, "atlas.app");
        assert_eq!(intent.targets[0].entry.as_deref(), Some("atlas_dev"));
    }

    #[test]
    fn formalize_intent_accepts_existing_spec_as_source() {
        let session = SessionSnapshot::new(
            Uuid::nil(),
            "spec",
            "/workspace".to_string(),
            TaskType::NewBehavior,
        );
        let raw = r#"{
          "schema_version": "x07.x07spec@0.1.0",
          "module_id": "toy.sorter",
          "operations": [
            {"id": "op.sort_u8_asc.v1", "name": "toy.sorter.sort_u8_asc"}
          ]
        }"#;

        let intent = intent_packet_from_raw(&session, raw, IntentInputMode::Spec, &[]);

        assert_eq!(intent.targets[0].module_id, "toy.sorter");
        assert_eq!(intent.targets[0].entry.as_deref(), Some("sort_u8_asc"));
        assert!(matches!(intent.source, IntentSource::Spec { .. }));
        assert!(intent
            .constraints
            .iter()
            .any(|item| item.contains("already-authored behavioral intent")));
    }

    #[test]
    fn spec_entry_derivation_requires_exact_module_prefix() {
        assert_eq!(
            entry_from_spec_operation("toy.sort", "toy.sort.sort_u8_asc"),
            "sort_u8_asc"
        );
        assert_eq!(
            entry_from_spec_operation("toy.sort", "toy.sorter.sort_u8_asc"),
            "toy_sorter_sort_u8_asc"
        );
        assert_eq!(
            entry_from_spec_operation("toy.sort", "op.sort_u8_asc.v1"),
            "sort_u8_asc"
        );
    }

    #[tokio::test]
    async fn agent_handoff_prompt_names_world_and_budget_boundaries() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("atlas app", TaskType::NewBehavior)
            .expect("create session");
        kernel
            .formalize_intent(
                session.session_id,
                "Use docs/examples/wasm_showcases/x07_atlas with x07-wasm app profile validation, trace replay, release pack verification, provenance, deploy planning, and SLO evidence.",
                IntentInputMode::Text,
                &[],
            )
            .expect("formalize intent");
        kernel
            .dispatch_event(session.session_id, SessionEvent::DraftSpec)
            .expect("draft spec");
        kernel
            .dispatch_event(session.session_id, SessionEvent::ApproveSpec)
            .expect("approve spec");

        let (handoff, _session) = kernel
            .create_agent_handoff(session.session_id, "openai-codex")
            .await
            .expect("create handoff");

        assert!(handoff.prompt.contains("## Execution Boundary"));
        assert!(handoff.prompt.contains("## Automation Runbook"));
        assert!(handoff.prompt.contains("`x07 run`"));
        assert!(handoff.prompt.contains("`X07_STUDIO_*`"));
        assert!(handoff.prompt.contains("`approve_spec`"));
        assert!(handoff.prompt.contains("agents cannot self-approve"));
        assert!(handoff.prompt.contains("`xtal.verify`"));
        assert!(handoff.prompt.contains("`x07-wasm app build`"));
        assert!(handoff.prompt.contains("WASM app"));
        assert!(handoff.prompt.contains("release/provenance"));
        assert!(handoff.prompt.contains("SLO/budget"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn agent_handoff_prompt_embeds_genpack_context() {
        let root = temp_root();
        let mut session = SessionSnapshot::new(
            Uuid::nil(),
            "api gateway",
            root.to_string(),
            TaskType::NewBehavior,
        );
        session.intent = Some(intent_packet_from_raw(
            &session,
            "Build an API gateway service for account reads.",
            IntentInputMode::Text,
            &[],
        ));
        let agent = AgentProfile::codex();
        let genpack = GenpackHandoffContext {
            archetype: "api-cell",
            schema: Some(serde_json::json!({
                "type": "object",
                "required": ["routes"]
            })),
            grammar: Some("api-cell = route+".to_string()),
        };

        let handoff = agent_handoff_from_session(&session, &agent, Some(&genpack));
        assert!(handoff.prompt.contains("## Service Genpack Context"));
        assert!(handoff.prompt.contains("Detected archetype: `api-cell`"));
        assert!(handoff.prompt.contains(r#""required":["routes"]"#));
        assert!(handoff.prompt.contains("api-cell = route+"));

        let clarify = agent_clarify_handoff_from_session(&session, &agent, 1, Some(&genpack));
        assert!(clarify.prompt.contains("## Service Genpack Context"));
        assert!(clarify.prompt.contains("Detected archetype: `api-cell`"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn sync_codes_survive_kernel_reopen() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("sync source", TaskType::NewBehavior)
            .expect("create session");
        let code = kernel
            .mint_sync_code(session.session_id)
            .expect("mint sync code");
        drop(kernel);

        let mut reopened = WorkspaceKernel::open(root.clone()).expect("reopen kernel");
        let claimed = reopened
            .claim_sync_code(&code.code)
            .expect("claim sync code");

        assert_eq!(claimed.session_id, session.session_id);
        assert!(root.join(".x07/studio/sync_codes.json").exists());
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn cassette_branch_replays_entries_and_truncates_session_ops() {
        let root = temp_root();
        std::fs::create_dir_all(root.join(".x07_rr/http")).expect("cassette dir");
        std::fs::write(
            root.join(".x07_rr/http/001-request.json"),
            b"{\"request\":1}",
        )
        .expect("first cassette");
        std::fs::write(
            root.join(".x07_rr/http/002-response.json"),
            b"{\"response\":2}",
        )
        .expect("second cassette");
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("cassette source", TaskType::NewBehavior)
            .expect("create session");
        let old_op = OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id: session.session_id,
            op: "rr.replay.old".to_string(),
            backend: "test".to_string(),
            command: Vec::new(),
            started_at: "1".to_string(),
            finished_at: Some("1".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts: vec![".x07_rr/http/001-request.json".to_string()],
            notes: None,
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        };
        let future_op = OpRecord {
            started_at: "9999999999".to_string(),
            finished_at: Some("9999999999".to_string()),
            op: "rr.replay.future".to_string(),
            id: Uuid::new_v4(),
            artifacts: vec![".x07_rr/http/002-response.json".to_string()],
            ..old_op.clone()
        };
        kernel
            .dispatch_event(
                session.session_id,
                SessionEvent::AppendOp(Box::new(old_op.clone())),
            )
            .expect("append old");
        kernel
            .dispatch_event(
                session.session_id,
                SessionEvent::AppendOp(Box::new(future_op)),
            )
            .expect("append future");

        let branch_id = kernel
            .branch_from_cassette(session.session_id, 0, "Replay first")
            .expect("branch cassette");
        let branch = kernel.get_session(branch_id).expect("branch session");

        assert_eq!(branch.title, "Replay first");
        assert!(branch.op_log.iter().any(|op| op.op == "rr.replay.old"));
        assert!(!branch.op_log.iter().any(|op| op.op == "rr.replay.future"));
        let branch_op = branch
            .op_log
            .iter()
            .find(|op| op.op == "cassette.branch")
            .expect("branch op");
        let manifest = branch_op.artifacts.first().expect("manifest artifact");
        let manifest_text = std::fs::read_to_string(root.join(manifest)).expect("manifest");
        assert!(manifest_text.contains("001-request.json"));
        assert!(manifest_text.contains("truncated_entries"));

        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn supervised_agent_plan_and_execute_append_visible_ops() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("supervised agent", TaskType::NewBehavior)
            .expect("create session");
        let agent = AgentProfile {
            schema_version: "x07.studio.agent_profile@0.1.0".to_string(),
            id: "echo-agent".to_string(),
            label: "Echo Agent".to_string(),
            command: "/bin/sh".to_string(),
            args: vec![
                "-c".to_string(),
                "printf '%s\\nartifact: target/xtal/verify/summary.json\\napproval required: policy widening\\nsupervised:%s\\nenv:%s:%s:%s:%s' '{\"schema_version\":\"x07.studio.agent_event@0.1.0\",\"kind\":\"approval\",\"summary\":\"structured policy gate\",\"artifact\":\"arch/xtal/xtal.json\"}' \"$1\" \"$X07_STUDIO_AGENT_ID\" \"$X07_STUDIO_ALLOWED_VERBS\" \"$X07_STUDIO_WRITE_ROOTS\" \"$X07_STUDIO_EVENT_SCHEMA\"".to_string(),
                "x07-studio-agent".to_string(),
            ],
            allowed_verbs: vec!["intent.formalize".to_string()],
            mcp_tools: vec!["x07.exec_v1".to_string()],
            write_roots: vec!["src/".to_string()],
            approval_required: true,
            status: AgentStatus::Available,
            notes: "test agent".to_string(),
        };
        kernel.save_agent_profile(&agent).expect("save agent");

        let (handoff, plan_op, plan_session) = kernel
            .run_agent_handoff(session.session_id, "echo-agent", AgentRunMode::Plan, None)
            .await
            .expect("plan agent");

        assert_eq!(plan_op.op, "agent.supervise.echo-agent");
        assert_eq!(plan_op.status, OperationStatus::Succeeded);
        assert!(root.join(&handoff.prompt_path).exists());
        assert!(plan_session
            .op_log
            .iter()
            .any(|op| op.op == "agent.supervise.echo-agent"));

        let blocked = kernel
            .start_agent_handoff(
                session.session_id,
                "echo-agent",
                AgentRunMode::Execute,
                Some(5),
            )
            .await
            .expect("start blocked agent");
        assert_eq!(blocked.op.op, "agent.approval.echo-agent");
        assert_eq!(blocked.op.status, OperationStatus::Pending);
        assert!(blocked.command.is_none());

        let (approval, approval_session) = kernel
            .resolve_agent_approval(
                session.session_id,
                blocked.op.id,
                ApprovalDecision::Approve,
                Some("test approval".to_string()),
            )
            .expect("approve agent");
        assert_eq!(approval.status, OperationStatus::Succeeded);
        assert!(approval_session
            .op_log
            .iter()
            .any(|op| op.op == "agent.approval.echo-agent"
                && op.status == OperationStatus::Succeeded));

        let prepared = kernel
            .start_agent_handoff(
                session.session_id,
                "echo-agent",
                AgentRunMode::Execute,
                Some(5),
            )
            .await
            .expect("start agent");
        assert_eq!(prepared.op.op, "agent.run.echo-agent");
        assert_eq!(prepared.op.status, OperationStatus::Running);
        assert!(prepared
            .session
            .op_log
            .iter()
            .any(|op| op.op == "agent.run.echo-agent" && op.status == OperationStatus::Running));

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let run_op = WorkspaceKernel::execute_agent_command_streaming(
            prepared.command.expect("agent command"),
            tx,
        )
        .await;
        let mut stream_updates = Vec::new();
        while let Ok(update) = rx.try_recv() {
            stream_updates.push(update);
        }
        let run_session = kernel
            .complete_agent_run(run_op.clone())
            .expect("complete agent");

        assert_eq!(run_op.op, "agent.run.echo-agent");
        assert_eq!(run_op.status, OperationStatus::Succeeded);
        assert!(run_op
            .stdout
            .as_deref()
            .unwrap_or_default()
            .contains(&handoff.prompt_path));
        assert!(run_op
            .stdout
            .as_deref()
            .unwrap_or_default()
            .contains("env:echo-agent:intent.formalize:src/:x07.studio.agent_event@0.1.0"));
        assert!(stream_updates.iter().any(|op| {
            op.status == OperationStatus::Running
                && op
                    .stdout
                    .as_deref()
                    .unwrap_or_default()
                    .contains(&handoff.prompt_path)
        }));
        assert!(stream_updates.iter().any(|op| {
            op.op == "agent.event.echo-agent.artifact"
                && op
                    .artifacts
                    .iter()
                    .any(|artifact| artifact == "target/xtal/verify/summary.json")
        }));
        assert!(stream_updates
            .iter()
            .any(|op| op.op == "agent.event.echo-agent.approval"));
        let structured_approval = stream_updates
            .iter()
            .find(|op| {
                op.op == "agent.event.echo-agent.approval"
                    && op
                        .artifacts
                        .iter()
                        .any(|artifact| artifact == "arch/xtal/xtal.json")
            })
            .expect("structured approval event");
        assert_eq!(
            structured_approval
                .report_json
                .as_ref()
                .and_then(|report| report.get("structured"))
                .and_then(|structured| structured.get("schema_version"))
                .and_then(serde_json::Value::as_str),
            Some("x07.studio.agent_event@0.1.0")
        );
        assert!(run_session
            .op_log
            .iter()
            .any(|op| op.op == "agent.run.echo-agent"));

        let blocked_again = kernel
            .start_agent_handoff(
                session.session_id,
                "echo-agent",
                AgentRunMode::Execute,
                Some(5),
            )
            .await
            .expect("start blocked agent after consumed approval");
        assert_eq!(blocked_again.op.op, "agent.approval.echo-agent");
        assert_eq!(blocked_again.op.status, OperationStatus::Pending);
        assert!(blocked_again.command.is_none());

        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn supervised_agent_execute_fails_on_unapproved_workspace_write() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("write audit", TaskType::NewBehavior)
            .expect("create session");
        let agent = AgentProfile {
            schema_version: "x07.studio.agent_profile@0.1.0".to_string(),
            id: "write-agent".to_string(),
            label: "Write Agent".to_string(),
            command: "/bin/sh".to_string(),
            args: vec![
                "-c".to_string(),
                "mkdir -p src private && printf ok > src/ok.txt && printf bad > private/bad.txt"
                    .to_string(),
                "x07-studio-agent".to_string(),
            ],
            allowed_verbs: vec!["impl.sync.write".to_string()],
            mcp_tools: vec!["x07.exec_v1".to_string()],
            write_roots: vec!["src/".to_string()],
            approval_required: false,
            status: AgentStatus::Available,
            notes: "test agent".to_string(),
        };
        kernel.save_agent_profile(&agent).expect("save agent");

        let (_handoff, run_op, _session) = kernel
            .run_agent_handoff(
                session.session_id,
                "write-agent",
                AgentRunMode::Execute,
                None,
            )
            .await
            .expect("run agent");

        assert_eq!(run_op.op, "agent.run.write-agent");
        assert_eq!(run_op.status, OperationStatus::Failed);
        assert!(root.join("src/ok.txt").exists());
        assert!(root.join("private/bad.txt").exists());
        assert!(run_op
            .stderr
            .as_deref()
            .unwrap_or_default()
            .contains("write-root audit failed"));
        let write_audit = run_op
            .report_json
            .as_ref()
            .and_then(|report| report.get("write_audit"))
            .expect("write audit report");
        assert_eq!(
            write_audit
                .get("violations")
                .and_then(serde_json::Value::as_array)
                .and_then(|items| items.first())
                .and_then(serde_json::Value::as_str),
            Some("private/bad.txt")
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn custom_agent_profiles_do_not_hide_default_coding_agents() {
        let root = temp_root();
        let kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let custom = AgentProfile {
            schema_version: "x07.studio.agent_profile@0.1.0".to_string(),
            id: "write-audit-agent".to_string(),
            label: "Write Audit Agent".to_string(),
            command: "/bin/sh".to_string(),
            args: Vec::new(),
            allowed_verbs: vec!["impl.sync.write".to_string()],
            mcp_tools: vec!["x07.exec_v1".to_string()],
            write_roots: vec!["src/".to_string()],
            approval_required: false,
            status: AgentStatus::Available,
            notes: "test agent".to_string(),
        };
        kernel
            .save_agent_profile(&custom)
            .expect("save custom agent");

        let profiles = kernel.list_agent_profiles().expect("list agent profiles");
        let ids = profiles
            .iter()
            .map(|profile| profile.id.as_str())
            .collect::<Vec<_>>();

        assert!(ids.contains(&"openai-codex"));
        assert!(ids.contains(&"claude-code"));
        assert!(ids.contains(&"write-audit-agent"));

        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn disabled_agent_profile_is_rejected_by_daemon_policy() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("disabled agent", TaskType::NewBehavior)
            .expect("create session");
        let mut agent = AgentProfile::codex();
        agent.id = "disabled-agent".to_string();
        agent.label = "Disabled Agent".to_string();
        agent.command = "/bin/sh".to_string();
        agent.status = AgentStatus::Disabled;
        kernel.save_agent_profile(&agent).expect("save agent");

        let handoff_error = kernel
            .create_agent_handoff(session.session_id, "disabled-agent")
            .await
            .expect_err("disabled handoff should fail")
            .to_string();
        assert!(handoff_error.contains("disabled"));

        let run_error = kernel
            .start_agent_handoff(
                session.session_id,
                "disabled-agent",
                AgentRunMode::Plan,
                None,
            )
            .await
            .expect_err("disabled run should fail")
            .to_string();
        assert!(run_error.contains("disabled"));

        std::fs::remove_dir_all(root).ok();
    }

    #[tokio::test]
    async fn missing_agent_command_cannot_execute_from_daemon_api() {
        let root = temp_root();
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("missing agent", TaskType::NewBehavior)
            .expect("create session");
        let mut agent = AgentProfile::codex();
        agent.id = "missing-agent".to_string();
        agent.label = "Missing Agent".to_string();
        agent.command = "x07-studio-missing-agent-command".to_string();
        agent.status = AgentStatus::Available;
        agent.approval_required = false;
        kernel.save_agent_profile(&agent).expect("save agent");

        let (_handoff, plan_op, _plan_session) = kernel
            .run_agent_handoff(
                session.session_id,
                "missing-agent",
                AgentRunMode::Plan,
                None,
            )
            .await
            .expect("planning should not require launching the command");
        assert_eq!(plan_op.op, "agent.supervise.missing-agent");
        assert_eq!(plan_op.status, OperationStatus::Succeeded);

        let execute_error = kernel
            .start_agent_handoff(
                session.session_id,
                "missing-agent",
                AgentRunMode::Execute,
                Some(5),
            )
            .await
            .expect_err("missing command should fail before supervised launch")
            .to_string();
        assert!(execute_error.contains("not available"));
        assert!(execute_error.contains("missing-agent"));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_doc_resolves_bounded_file_and_directory() {
        let root = temp_root();
        let quickstart = root.join("x07/docs/getting-started/agent-quickstart.md");
        let examples = root.join("x07/docs/examples");
        std::fs::create_dir_all(quickstart.parent().expect("quickstart parent"))
            .expect("create quickstart dir");
        std::fs::create_dir_all(examples.join("apps")).expect("create examples dir");
        std::fs::write(
            &quickstart,
            "# Agent quickstart\n\nUse x07 run as the canonical execution front door.\n\nKeep evidence visible.",
        )
        .expect("write quickstart");
        std::fs::write(
            examples.join("README.md"),
            "# Examples\n\nRunnable examples.",
        )
        .expect("write examples readme");

        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("doc preview", TaskType::NewBehavior)
            .expect("create session");

        let preview = kernel
            .preview_doc(
                session.session_id,
                "x07/docs/getting-started/agent-quickstart.md",
            )
            .expect("preview doc");
        assert_eq!(preview.schema_version, "x07.studio.doc_preview@0.1.0");
        assert_eq!(preview.title, "Agent quickstart");
        assert_eq!(preview.media_kind, "markdown");
        assert!(preview.snippet.contains("x07 run"));
        assert!(preview.entries.is_empty());

        let directory = kernel
            .preview_doc(session.session_id, "x07/docs/examples")
            .expect("preview docs directory");
        assert_eq!(directory.media_kind, "directory");
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "x07/docs/examples/README.md"));
        assert!(directory
            .entries
            .iter()
            .any(|entry| entry.path == "x07/docs/examples/apps"));

        let rejected = kernel
            .preview_doc(session.session_id, "x07/docs/../Cargo.toml")
            .expect_err("parent traversal should be rejected")
            .to_string();
        assert!(rejected.contains("inside x07 docs"), "{rejected}");

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_artifact_reads_recorded_json_patchset() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("src")).expect("create src");
        std::fs::create_dir_all(root.join("target/xtal")).expect("create target/xtal");
        std::fs::write(
            root.join("src/main.x07.json"),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "x07.ast@0.1.0",
                "decls": [],
                "solve": ["bytes.lit", "todo"]
            }))
            .expect("serialize source"),
        )
        .expect("write source");
        let artifact = "target/xtal/impl-sync.patchset.json";
        std::fs::write(
            root.join(artifact),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "x07.patchset@0.1.0",
                "patches": [
                    {
                        "path": "src/main.x07.json",
                        "patch": [
                            { "op": "replace", "path": "/solve", "value": ["bytes.lit", "ok"] }
                        ],
                        "note": "Realize approved operation"
                    }
                ]
            }))
            .expect("serialize patchset"),
        )
        .expect("write patchset");

        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("artifact preview", TaskType::NewBehavior)
            .expect("create session");
        kernel
            .append_op(
                session.session_id,
                test_op(
                    session.session_id,
                    "impl.sync.patchset",
                    vec![artifact.to_string()],
                ),
            )
            .expect("append op");

        let preview = kernel
            .preview_artifact(session.session_id, artifact)
            .expect("preview artifact");

        assert_eq!(preview.schema_version, "x07.studio.artifact_preview@0.1.0");
        assert_eq!(preview.media_kind, "json");
        assert_eq!(
            preview
                .json
                .as_ref()
                .and_then(|json| json["schema_version"].as_str()),
            Some("x07.patchset@0.1.0")
        );
        let target = preview
            .patchset_preview
            .as_ref()
            .and_then(|preview| preview.targets.first())
            .expect("patchset target preview");
        assert_eq!(target.path, "src/main.x07.json");
        assert_eq!(target.operations, 1);
        assert!(target.apply_error.is_none(), "{:?}", target.apply_error);
        assert_eq!(
            target
                .before_json
                .as_ref()
                .and_then(|json| json["solve"][1].as_str()),
            Some("todo")
        );
        assert_eq!(
            target
                .after_json
                .as_ref()
                .and_then(|json| json["solve"][1].as_str()),
            Some("ok")
        );

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_artifact_reports_patchset_target_errors_per_file() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("target/xtal")).expect("create target/xtal");
        std::fs::create_dir_all(root.join(".x07/studio")).expect("create hidden studio dir");
        std::fs::write(root.join(".x07/studio/private.json"), "{}").expect("write hidden target");
        let artifact = "target/xtal/impl-sync.patchset.json";
        std::fs::write(
            root.join(artifact),
            serde_json::to_vec(&serde_json::json!({
                "schema_version": "x07.patchset@0.1.0",
                "patches": [
                    {
                        "path": "../outside.x07.json",
                        "patch": [
                            { "op": "replace", "path": "/solve", "value": ["bytes.lit", "ok"] }
                        ],
                        "note": "Invalid target path"
                    },
                    {
                        "path": ".x07/studio/private.json",
                        "patch": [
                            { "op": "replace", "path": "/secret", "value": "nope" }
                        ],
                        "note": "Hidden workspace target"
                    }
                ]
            }))
            .expect("serialize patchset"),
        )
        .expect("write patchset");

        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("artifact preview", TaskType::NewBehavior)
            .expect("create session");
        kernel
            .append_op(
                session.session_id,
                test_op(
                    session.session_id,
                    "impl.sync.patchset",
                    vec![artifact.to_string()],
                ),
            )
            .expect("append op");

        let preview = kernel
            .preview_artifact(session.session_id, artifact)
            .expect("preview artifact");
        let target = preview
            .patchset_preview
            .as_ref()
            .and_then(|preview| preview.targets.first())
            .expect("patchset target preview");
        assert_eq!(target.path, "../outside.x07.json");
        assert!(
            target
                .apply_error
                .as_deref()
                .unwrap_or_default()
                .contains("workspace"),
            "{:?}",
            target.apply_error
        );
        assert!(target.before_json.is_none());
        assert!(target.after_json.is_none());
        let hidden_target = preview
            .patchset_preview
            .as_ref()
            .and_then(|preview| preview.targets.get(1))
            .expect("hidden target preview");
        assert_eq!(hidden_target.path, ".x07/studio/private.json");
        assert!(
            hidden_target
                .apply_error
                .as_deref()
                .unwrap_or_default()
                .contains("reviewable project surfaces"),
            "{:?}",
            hidden_target.apply_error
        );
        assert!(hidden_target.before_json.is_none());
        assert!(hidden_target.after_json.is_none());

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn preview_artifact_rejects_unrecorded_and_parent_paths() {
        let root = temp_root();
        std::fs::create_dir_all(&root).expect("create root");
        std::fs::write(root.join("unrecorded.json"), "{}").expect("write unrecorded");
        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        let session = kernel
            .create_session("artifact preview", TaskType::NewBehavior)
            .expect("create session");

        let unrecorded = kernel
            .preview_artifact(session.session_id, "unrecorded.json")
            .expect_err("unrecorded artifact should be rejected")
            .to_string();
        assert!(unrecorded.contains("not recorded"), "{unrecorded}");

        kernel
            .append_op(
                session.session_id,
                test_op(
                    session.session_id,
                    "impl.sync.patchset",
                    vec!["../secret.json".to_string()],
                ),
            )
            .expect("append op");
        let traversal = kernel
            .preview_artifact(session.session_id, "../secret.json")
            .expect_err("parent traversal should be rejected")
            .to_string();
        assert!(traversal.contains("workspace"), "{traversal}");

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn xtal_workflow_skips_scaffold_when_template_spec_exists() {
        let root = temp_root();
        let spec_dir = root.join("spec");
        std::fs::create_dir_all(&spec_dir).expect("create spec dir");
        std::fs::write(spec_dir.join("toy.sorter.x07spec.json"), "{}").expect("write spec");
        let vars = std::collections::BTreeMap::from([(
            "input".to_string(),
            "spec/toy.sorter.x07spec.json".to_string(),
        )]);

        assert!(!should_scaffold_spec(root.as_path(), &vars));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn workflow_template_maps_docs_examples_to_command_lanes() {
        let workflow = IntentPacket {
            schema_version: "x07.studio.intent_packet@0.1.0".to_string(),
            session_id: Uuid::nil(),
            workspace_root: "/workspace".to_string(),
            task_type: TaskType::NewBehavior,
            targets: vec![IntentTarget {
                module_id: "workflow.graph".to_string(),
                entry: Some("makespan_u32".to_string()),
            }],
            examples: vec![],
            constraints: vec![],
            policy_implications: vec![],
            ambiguities: vec![],
            assumptions: vec![],
            witnesses: vec![],
            source: IntentSource::Text {
                raw: "Use docs/examples/agent-gate/xtal/workflow-graph".to_string(),
            },
            clarification_history: Vec::new(),
        };
        let gateway = IntentPacket {
            targets: vec![IntentTarget {
                module_id: "gateway.core".to_string(),
                entry: Some("route_request_v1".to_string()),
            }],
            source: IntentSource::Text {
                raw: "Use docs/examples/apps/x07-api-gateway".to_string(),
            },
            ..workflow.clone()
        };
        let crawler = IntentPacket {
            targets: vec![IntentTarget {
                module_id: "crawl.plan".to_string(),
                entry: Some("plan_crawl_v1".to_string()),
            }],
            source: IntentSource::Text {
                raw: "Use docs/examples/apps/x07crawl".to_string(),
            },
            ..workflow.clone()
        };
        let dbguard = IntentPacket {
            targets: vec![IntentTarget {
                module_id: "db.guard".to_string(),
                entry: Some("verify_drift".to_string()),
            }],
            source: IntentSource::Text {
                raw: "Use docs/examples/apps/x07dbguard".to_string(),
            },
            ..workflow.clone()
        };
        let atlas = IntentPacket {
            targets: vec![IntentTarget {
                module_id: "atlas.app".to_string(),
                entry: Some("atlas_dev".to_string()),
            }],
            source: IntentSource::Text {
                raw: "Use docs/examples/wasm_showcases/x07_atlas".to_string(),
            },
            ..workflow.clone()
        };

        assert_eq!(
            workflow_template_from_intent(&workflow),
            WorkflowTemplate::WorkflowGraph
        );
        assert_eq!(
            WorkflowTemplate::WorkflowGraph.workflow_steps(),
            &[
                "tests.gen.write",
                "gen.verify",
                "test.xtal.generated.all",
                "impl.check",
                "xtal.dev",
                "xtal.verify",
                "test.manifest"
            ]
        );
        assert_eq!(
            workflow_template_from_intent(&gateway),
            WorkflowTemplate::ApiGateway
        );
        assert_eq!(
            WorkflowTemplate::ApiGateway.workflow_steps_for_environment(true),
            &[
                "arch.check.write_lock",
                "test.manifest",
                "run.sandbox",
                "bundle.api_gateway.sandbox"
            ]
        );
        assert_eq!(
            WorkflowTemplate::ApiGateway.workflow_steps_for_environment(false),
            &[
                "arch.check.write_lock",
                "test.manifest",
                "run.sandbox.os",
                "bundle.api_gateway.sandbox.os"
            ]
        );
        assert_eq!(
            workflow_template_from_intent(&dbguard),
            WorkflowTemplate::DbGuard
        );
        assert_eq!(
            workflow_template_from_intent(&crawler),
            WorkflowTemplate::X07Crawl
        );
        assert_eq!(
            workflow_template_from_intent(&atlas),
            WorkflowTemplate::X07Atlas
        );
        assert_eq!(
            WorkflowTemplate::X07Crawl.workflow_steps_for_environment(false),
            &[
                "pkg.lock",
                "test.manifest",
                "run.x07crawl.sandbox.os",
                "bundle.x07crawl.sandbox.os"
            ]
        );
        assert_eq!(
            WorkflowTemplate::X07Atlas.workflow_steps(),
            &[
                "pkg.lock.atlas.frontend",
                "wasm.app.profile.validate.atlas_dev",
                "wasm.web_ui.contracts.validate",
                "wasm.http.contracts.validate",
                "wasm.caps.validate.atlas_release",
                "wasm.ops.validate",
                "wasm.slo.validate.atlas",
                "wasm.app.build.atlas_dev",
                "wasm.app.serve.smoke.atlas_dev",
                "wasm.app.test.happy_path",
                "wasm.app.test.validation_error",
                "wasm.app.test.regress.atlas_incident",
                "wasm.app.build.atlas_release",
                "wasm.app.pack.atlas_release",
                "wasm.app.verify.atlas_release",
                "wasm.provenance.attest.atlas_release",
                "wasm.provenance.verify.atlas_release",
                "wasm.deploy.plan.atlas_release",
                "wasm.slo.eval.atlas_canary_ok"
            ]
        );
        assert_eq!(
            WorkflowTemplate::X07Atlas.platform_delivery_steps(),
            &[
                "lp.deploy.accept.local",
                "lp.deploy.run.local.metrics",
                "lp.deploy.query.local",
                "lp.deploy.status.local"
            ]
        );
        assert_eq!(
            WorkflowTemplate::X07Crawl.directory_for_step("run.x07crawl.sandbox.os"),
            Some("out/")
        );
        assert_eq!(
            WorkflowTemplate::X07Atlas.directory_for_step("wasm.app.pack.atlas_release"),
            Some("dist/showcase_fullstack/pack.atlas_release/")
        );
        assert_eq!(
            WorkflowTemplate::StateMachineArch.stdin_for_step("run.stdin"),
            Some("start\ntick\nfinish\n")
        );
        assert_eq!(
            WorkflowTemplate::DbGuard.workflow_steps_for_environment(false),
            &[
                "pkg.lock",
                "arch.check.write_lock",
                "test.manifest",
                "run.stdin",
                "run.sandbox.stdin.os",
                "bundle.dbguard.sandbox.os"
            ]
        );
        assert_eq!(
            WorkflowTemplate::DbGuard.stdin_for_step("run.sandbox.stdin.os"),
            Some("apply out/dbguard.sqlite")
        );
        assert_eq!(
            WorkflowTemplate::DbGuard.directory_for_step("run.sandbox.stdin.os"),
            Some("out/")
        );
    }

    #[test]
    fn copy_example_tree_skips_generated_targets() {
        let source = temp_root();
        let destination = temp_root();
        std::fs::create_dir_all(source.join("src")).expect("create src");
        std::fs::create_dir_all(source.join(".x07/deps/stale")).expect("create deps");
        std::fs::create_dir_all(source.join("target")).expect("create target");
        std::fs::write(source.join("x07.json"), "{}").expect("write project");
        std::fs::write(source.join("src/main.x07.json"), "{}").expect("write source");
        std::fs::write(source.join(".x07/deps/stale/x07-package.json"), "{}").expect("write deps");
        std::fs::write(source.join("target/stale.json"), "{}").expect("write target");

        copy_example_tree(source.as_path(), destination.as_path()).expect("copy example");

        assert!(destination.join("x07.json").exists());
        assert!(destination.join("src/main.x07.json").exists());
        assert!(!destination
            .join(".x07/deps/stale/x07-package.json")
            .exists());
        assert!(!destination.join("target/stale.json").exists());

        std::fs::remove_dir_all(source).ok();
        std::fs::remove_dir_all(destination).ok();
    }

    #[test]
    fn atlas_seed_source_uses_multi_project_root_markers() {
        let source = temp_root();
        std::fs::create_dir_all(source.join("arch/app")).expect("create app arch");
        std::fs::create_dir_all(source.join("frontend")).expect("create frontend");
        std::fs::create_dir_all(source.join("backend")).expect("create backend");
        std::fs::write(source.join("arch/app/index.x07app.json"), "{}").expect("write app index");
        std::fs::write(source.join("frontend/x07.json"), "{}").expect("write frontend project");
        std::fs::write(source.join("backend/x07.json"), "{}").expect("write backend project");

        assert!(WorkflowTemplate::X07Atlas.source_exists(source.as_path()));
        assert!(!WorkflowTemplate::WorkflowGraph.source_exists(source.as_path()));

        std::fs::remove_dir_all(source).ok();
    }

    #[test]
    fn workspace_radar_reports_xtal_artifact_readiness() {
        let root = temp_root();
        std::fs::create_dir_all(root.join("arch/xtal")).expect("create xtal dir");
        std::fs::create_dir_all(root.join("spec")).expect("create spec dir");
        std::fs::create_dir_all(root.join("gen/xtal")).expect("create gen dir");
        std::fs::create_dir_all(root.join("target/xtal/verify/nested")).expect("create verify dir");
        std::fs::create_dir_all(root.join("target/xtal/cert")).expect("create cert dir");
        std::fs::create_dir_all(root.join("target/xtal/violations"))
            .expect("create violations dir");
        std::fs::write(root.join("arch/xtal/xtal.json"), "{}").expect("write manifest");
        std::fs::write(root.join("spec/app.x07spec.json"), "{}").expect("write spec");
        std::fs::write(root.join("spec/not-a-spec.json"), "{}").expect("write non spec");
        std::fs::write(root.join("gen/xtal/tests.json"), "{}").expect("write tests");
        std::fs::write(root.join("target/xtal/verify/nested/summary.json"), "{}")
            .expect("write verify summary");
        std::fs::write(root.join("target/xtal/cert/bundle.json"), "{}").expect("write cert bundle");
        std::fs::write(root.join("target/xtal/violations/incident.json"), "{}")
            .expect("write incident");

        let mut kernel = WorkspaceKernel::open(root.clone()).expect("open kernel");
        kernel
            .create_session("incident", TaskType::IncidentRepair)
            .expect("create incident session");

        let radar = kernel.workspace_radar();

        assert_eq!(radar.schema_version, "x07.studio.workspace_radar@0.1.0");
        assert!(radar.xtal_manifest.exists);
        assert_eq!(radar.spec_count, 1);
        assert!(radar.generated_tests.exists);
        assert_eq!(
            radar
                .latest_verify
                .as_ref()
                .map(|state| state.path.as_str()),
            Some("target/xtal/verify/nested/summary.json")
        );
        assert_eq!(
            radar
                .latest_certify
                .as_ref()
                .map(|state| state.path.as_str()),
            Some("target/xtal/cert/bundle.json")
        );
        assert_eq!(radar.incident_count, 2);

        std::fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> camino::Utf8PathBuf {
        let path = std::env::temp_dir().join(format!("x07-studio-core-test-{}", Uuid::new_v4()));
        camino::Utf8PathBuf::from_path_buf(path).expect("utf8 temp path")
    }

    fn test_op(session_id: Uuid, op: &str, artifacts: Vec<String>) -> OpRecord {
        OpRecord {
            schema_version: "x07.studio.op_record@0.1.0".to_string(),
            id: Uuid::new_v4(),
            session_id,
            op: op.to_string(),
            backend: "test".to_string(),
            command: vec!["studio-test".to_string()],
            started_at: "now".to_string(),
            finished_at: Some("now".to_string()),
            status: OperationStatus::Succeeded,
            exit_code: Some(0),
            artifacts,
            notes: Some("test operation".to_string()),
            stdout: None,
            stderr: None,
            stdout_json: None,
            stderr_json: None,
            report_json: None,
            report_path: None,
        }
    }
}
