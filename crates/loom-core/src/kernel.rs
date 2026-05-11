use std::collections::{BTreeMap, HashMap};

use anyhow::{anyhow, Context};
use camino::{Utf8Path, Utf8PathBuf};
use uuid::Uuid;

use loom_adapters::mcp::{boxed_client, McpClient};
use loom_adapters::providers::ProviderProber;
use loom_adapters::x07_cli::{CliAdapter, ExecutedBinding};
use loom_store::FsStore;
use loom_types::artifacts::{
    IntentPacket, IntentSource, OpRecord, OperationStatus, ProviderProbeReport, ProviderProfile,
    TaskType,
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

#[cfg(test)]
mod tests {
    use uuid::Uuid;

    use loom_types::artifacts::{
        IntentPacket, IntentSource, IntentTarget, TaskType, Witness, WitnessKind,
    };

    use super::xtal_workflow_vars_from_intent;

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
