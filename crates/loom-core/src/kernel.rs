use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use uuid::Uuid;

use loom_adapters::command_runner::{now_string, CommandExecution, CommandRunner};
use loom_adapters::mcp::{boxed_client, McpClient};
use loom_adapters::providers::ProviderProber;
use loom_adapters::x07_cli::{CliAdapter, ExecutedBinding};
use loom_store::FsStore;
use loom_types::api::{AgentRunMode, ApprovalDecision};
use loom_types::artifacts::{
    AgentHandoff, AgentProfile, AgentStatus, IntentPacket, IntentSource, OpRecord, OperationStatus,
    ProviderProbeReport, ProviderProfile, TaskType,
};
use loom_types::mcp::{McpConnectionInfo, McpEndpoint, McpToolCallResult, McpToolDescriptor};
use loom_types::ops::SessionEvent;
use loom_types::session::{SessionPhase, SessionSnapshot};

use crate::workspace::WorkspaceModel;

pub struct WorkspaceKernel {
    root: Utf8PathBuf,
    model: WorkspaceModel,
    store: FsStore,
    cli: CliAdapter,
    providers: ProviderProber,
    mcp_connections: HashMap<String, Box<dyn McpClient>>,
}

#[derive(Debug, Clone)]
pub struct PreparedAgentRun {
    pub handoff: AgentHandoff,
    pub op: OpRecord,
    pub session: SessionSnapshot,
    pub command: Option<AgentCommandPlan>,
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
        Ok(Self {
            root,
            model,
            store,
            cli,
            providers: ProviderProber::default(),
            mcp_connections: HashMap::new(),
        })
    }

    pub fn workspace_root(&self) -> &Utf8Path {
        self.root.as_path()
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
        Ok(snapshot)
    }

    pub fn dispatch_event(
        &mut self,
        session_id: Uuid,
        event: SessionEvent,
    ) -> anyhow::Result<SessionSnapshot> {
        let snapshot = self
            .model
            .dispatch(session_id, event)
            .map_err(|error| anyhow!(error.to_string()))?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    pub async fn run_binding(
        &mut self,
        session_id: Uuid,
        binding_id: &str,
        vars: &BTreeMap<String, String>,
    ) -> anyhow::Result<SessionSnapshot> {
        let executed = self.cli.execute(binding_id, vars).await?;
        let op = op_record_from_binding(session_id, binding_id, executed);

        let snapshot = self
            .model
            .dispatch(session_id, SessionEvent::AppendOp(Box::new(op)))
            .map_err(|error| anyhow!(error.to_string()))?;
        self.store.save_session(&snapshot)?;
        Ok(snapshot)
    }

    pub async fn run_xtal_workflow(&mut self, session_id: Uuid) -> anyhow::Result<SessionSnapshot> {
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
        let vars = xtal_workflow_vars_from_intent(intent);

        if !self.root.join("x07.json").exists() {
            let snapshot = self
                .run_binding(session_id, "project.init.xtal-pure", &BTreeMap::new())
                .await?;
            if last_op_failed(&snapshot) {
                return Ok(snapshot);
            }
        }

        for binding_id in ["spec.scaffold", "spec.check", "tests.gen.write"] {
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

    pub fn list_provider_profiles(&self) -> anyhow::Result<Vec<ProviderProfile>> {
        self.store.load_provider_profiles()
    }

    pub fn list_agent_profiles(&self) -> anyhow::Result<Vec<AgentProfile>> {
        let profiles = self.store.load_agent_profiles()?;
        if profiles.is_empty() {
            Ok(default_agent_profiles())
        } else {
            Ok(profiles)
        }
    }

    pub fn save_agent_profile(&self, profile: &AgentProfile) -> anyhow::Result<()> {
        self.store.save_agent_profile(profile)
    }

    pub fn create_agent_handoff(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
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
        let handoff = agent_handoff_from_session(&session, &agent);
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
        let snapshot = self
            .model
            .dispatch(session_id, SessionEvent::AppendOp(Box::new(op)))
            .map_err(|error| anyhow!(error.to_string()))?;
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
        let prepared = self.start_agent_handoff(session_id, agent_id, mode, timeout_seconds)?;
        let handoff = prepared.handoff.clone();
        if let Some(command) = prepared.command {
            let op = Self::execute_agent_command(command).await;
            let session = self.complete_agent_run(op.clone())?;
            Ok((handoff, op, session))
        } else {
            Ok((handoff, prepared.op, prepared.session))
        }
    }

    pub fn start_agent_handoff(
        &mut self,
        session_id: Uuid,
        agent_id: &str,
        mode: AgentRunMode,
        timeout_seconds: Option<u64>,
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
        let handoff = agent_handoff_from_session(&session, &agent);
        let prompt_path = self.store.save_agent_handoff(&handoff)?;

        let (op, command) = match mode {
            AgentRunMode::Plan => (
                agent_plan_op(session_id, &agent, &handoff, &prompt_path),
                None,
            ),
            AgentRunMode::Execute => {
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
        })
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
        match CommandRunner
            .run_with_timeout(
                command.cwd.as_path(),
                &command.program,
                &command.args,
                &BTreeMap::new(),
                Some(command.timeout_seconds),
            )
            .await
        {
            Ok(execution) => agent_execution_op(command, execution),
            Err(error) => agent_spawn_error_op(command, error),
        }
    }

    pub fn complete_agent_run(&mut self, op: OpRecord) -> anyhow::Result<SessionSnapshot> {
        let session_id = op.session_id;
        let snapshot = self
            .model
            .dispatch(session_id, SessionEvent::UpdateOp(Box::new(op)))
            .map_err(|error| anyhow!(error.to_string()))?;
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
        let snapshot = self
            .model
            .dispatch(session_id, SessionEvent::AppendOp(Box::new(op)))
            .map_err(|error| anyhow!(error.to_string()))?;
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

fn default_agent_profiles() -> Vec<AgentProfile> {
    let mut codex = AgentProfile::codex();
    codex.status = status_for_command(&codex.command);
    let mut claude = AgentProfile::claude_code();
    claude.status = status_for_command(&claude.command);
    vec![codex, claude]
}

fn status_for_command(command: &str) -> AgentStatus {
    if command_in_path(command) {
        AgentStatus::Available
    } else {
        AgentStatus::NeedsInstall
    }
}

fn command_in_path(command: &str) -> bool {
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&paths).any(|dir| dir.join(command).is_file())
}

fn agent_handoff_from_session(session: &SessionSnapshot, agent: &AgentProfile) -> AgentHandoff {
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
    let prompt = render_agent_handoff_prompt(session, agent, &command);
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
    if let Some(contract) = &session.contract {
        out.push_str("\n## Session Contract\n\n");
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
    out.push_str("\n## Required Loop\n\n");
    out.push_str("1. Re-read this handoff and the session contract.\n");
    out.push_str("2. Use x07 docs/MCP tools before selecting commands.\n");
    out.push_str("3. Produce or update artifacts only inside the permitted roots.\n");
    out.push_str("4. Run the canonical XTAL checks before reporting completion.\n");
    out
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

fn agent_execution_op(command: AgentCommandPlan, execution: CommandExecution) -> OpRecord {
    OpRecord {
        schema_version: "x07.studio.op_record@0.1.0".to_string(),
        id: command.op_id,
        session_id: command.session_id,
        op: format!("agent.run.{}", command.agent.id),
        backend: "agent-supervisor".to_string(),
        command: std::iter::once(execution.program.clone())
            .chain(execution.args.clone())
            .collect(),
        started_at: execution.started_at,
        finished_at: Some(execution.finished_at),
        status: if execution.exit_code == Some(0) {
            OperationStatus::Succeeded
        } else {
            OperationStatus::Failed
        },
        exit_code: execution.exit_code,
        artifacts: vec![command.prompt_path.to_string()],
        notes: Some(format!(
            "Ran {} under Studio supervision.",
            command.agent.label
        )),
        stdout: Some(execution.stdout),
        stderr: Some(execution.stderr),
        stdout_json: execution.stdout_json,
        stderr_json: execution.stderr_json,
        report_json: Some(serde_json::json!({
            "mode": "execute",
            "handoff": command.handoff,
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
        op: format!("agent.run.{}", command.agent.id),
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

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::api::{AgentRunMode, ApprovalDecision};
    use loom_types::artifacts::{
        AgentProfile, AgentStatus, IntentPacket, IntentSource, IntentTarget, OperationStatus,
        TaskType, Witness, WitnessKind,
    };

    use super::{xtal_workflow_vars_from_intent, WorkspaceKernel};

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
                path: ".x07/studio/incidents/manual-note.jsonl".to_string(),
            },
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
                "printf 'supervised:%s' \"$1\"".to_string(),
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
            .expect("start agent");
        assert_eq!(prepared.op.op, "agent.run.echo-agent");
        assert_eq!(prepared.op.status, OperationStatus::Running);
        assert!(prepared
            .session
            .op_log
            .iter()
            .any(|op| op.op == "agent.run.echo-agent" && op.status == OperationStatus::Running));

        let run_op =
            WorkspaceKernel::execute_agent_command(prepared.command.expect("agent command")).await;
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
            .expect("start blocked agent after consumed approval");
        assert_eq!(blocked_again.op.op, "agent.approval.echo-agent");
        assert_eq!(blocked_again.op.status, OperationStatus::Pending);
        assert!(blocked_again.command.is_none());

        std::fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> camino::Utf8PathBuf {
        let path = std::env::temp_dir().join(format!("x07-studio-core-test-{}", Uuid::new_v4()));
        camino::Utf8PathBuf::from_path_buf(path).expect("utf8 temp path")
    }
}
