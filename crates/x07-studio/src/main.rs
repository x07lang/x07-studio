use std::collections::BTreeMap;
use std::fs;
use std::net::{SocketAddr, TcpListener};
use std::path::{Path as StdPath, PathBuf};

use clap::Parser;
use eframe::egui;
use serde_json::Value;
use tokio::runtime::Runtime;
use tokio::task::JoinHandle;
use uuid::Uuid;

use loom_client::DaemonClient;
use loom_types::api::{
    ConnectMcpRequest, CreateSessionRequest, DispatchEventRequest, HealthResponse,
    RunBindingRequest, RuntimeComponentState, RuntimeComponentStatus,
};
use loom_types::artifacts::{IntentPacket, ProviderProfile, TaskType};
use loom_types::mcp::{McpEndpoint, McpHttpEndpoint};
use loom_types::ops::SessionEvent;
use loom_types::session::{SessionPhase, SessionSnapshot};

const DEFAULT_DAEMON_URL: &str = "http://127.0.0.1:7719";
const COMPONENT_ENV_KEYS: &[&str] = &[
    "X07_STUDIO_X07_EXE",
    "X07_STUDIO_X07_WASM_EXE",
    "X07_STUDIO_X07LP_EXE",
];
const ONBOARDING_BOOTSTRAP_COMMAND: &str =
    "python3 scripts/bootstrap_components.py --install-missing --write-env .x07/studio/defaults.env";

#[derive(Debug, Parser)]
#[command(name = "x07-studio")]
#[command(version)]
#[command(about = "GUI shell for x07 Studio.")]
struct Cli {
    #[arg(long)]
    daemon_url: Option<String>,

    #[arg(long)]
    root: Option<String>,

    #[arg(long, help = "Use the daemon URL without starting an embedded daemon")]
    external_daemon: bool,

    #[arg(long, help = "Load x07 Studio component defaults from this env file")]
    defaults_env: Option<PathBuf>,
}

fn main() -> eframe::Result<()> {
    let cli = Cli::parse();
    if let Err(error) = load_first_defaults_env(cli.defaults_env.as_deref()) {
        eprintln!("could not load x07 Studio defaults: {error}");
    }
    let options = eframe::NativeOptions::default();
    let launch = StudioLaunch {
        daemon_url: cli
            .daemon_url
            .clone()
            .or_else(|| std::env::var("X07_STUDIO_DAEMON_URL").ok())
            .unwrap_or_else(|| DEFAULT_DAEMON_URL.to_string()),
        root: cli
            .root
            .clone()
            .or_else(|| std::env::var("X07_STUDIO_WORKSPACE_ROOT").ok())
            .map(expand_user_path)
            .unwrap_or_else(|| ".".to_string()),
        external_daemon: cli.external_daemon,
    };

    eframe::run_native(
        "x07 Studio",
        options,
        Box::new(move |_| Ok(Box::new(StudioApp::new(launch.clone())?))),
    )
}

#[derive(Clone)]
struct StudioLaunch {
    daemon_url: String,
    root: String,
    external_daemon: bool,
}

struct ManagedDaemon {
    url: String,
    task: JoinHandle<()>,
}

fn start_managed_daemon(rt: &Runtime, root: &str) -> anyhow::Result<ManagedDaemon> {
    let std_listener = TcpListener::bind("127.0.0.1:0")?;
    let addr: SocketAddr = std_listener.local_addr()?;
    set_managed_daemon_env(addr);
    std_listener.set_nonblocking(true)?;
    let _guard = rt.enter();
    let listener = tokio::net::TcpListener::from_std(std_listener)?;
    let state = loom_daemon::default_state(camino::Utf8PathBuf::from(root))?;
    let task = rt.spawn(async move {
        if let Err(error) = loom_daemon::serve_listener(listener, state).await {
            eprintln!("managed loom daemon stopped: {error}");
        }
    });
    Ok(ManagedDaemon {
        url: format!("http://{addr}"),
        task,
    })
}

struct StudioApp {
    rt: Runtime,
    client: DaemonClient,
    managed_daemon: Option<JoinHandle<()>>,
    health: Option<loom_types::api::HealthResponse>,
    daemon_url: String,
    sessions: Vec<SessionSnapshot>,
    selected_session: Option<Uuid>,
    new_session_title: String,
    binding_id: String,
    binding_vars_text: String,
    provider_profile: ProviderProfile,
    last_provider_report: Option<Value>,
    mcp_endpoint: McpHttpEndpoint,
    mcp_connection_id: Option<String>,
    mcp_tools: Vec<String>,
    mcp_tool_name: String,
    mcp_tool_args_text: String,
    mcp_last_result: Option<Value>,
    error: Option<String>,
}

impl StudioApp {
    fn new(launch: StudioLaunch) -> anyhow::Result<Self> {
        let rt = Runtime::new()?;
        let managed = if launch.external_daemon {
            None
        } else {
            Some(start_managed_daemon(&rt, &launch.root)?)
        };
        let daemon_url = managed
            .as_ref()
            .map(|daemon| daemon.url.clone())
            .unwrap_or(launch.daemon_url);
        let client = DaemonClient::new(daemon_url.clone());
        let mut app = Self {
            rt,
            client,
            managed_daemon: managed.map(|daemon| daemon.task),
            health: None,
            daemon_url,
            sessions: Vec::new(),
            selected_session: None,
            new_session_title: "New session".to_string(),
            binding_id: "xtal.verify".to_string(),
            binding_vars_text: "{}".to_string(),
            provider_profile: ProviderProfile::local_ollama(),
            last_provider_report: None,
            mcp_endpoint: McpHttpEndpoint::default(),
            mcp_connection_id: None,
            mcp_tools: Vec::new(),
            mcp_tool_name: "x07.search_v1".to_string(),
            mcp_tool_args_text: r#"{"query":"xtal verify"}"#.to_string(),
            mcp_last_result: None,
            error: None,
        };
        app.refresh();
        Ok(app)
    }

    fn set_error<E: ToString>(&mut self, error: E) {
        self.error = Some(error.to_string());
    }

    fn refresh(&mut self) {
        match self.rt.block_on(self.client.health()) {
            Ok(health) => self.health = Some(health),
            Err(error) => self.set_error(error),
        }
        match self.rt.block_on(self.client.list_sessions()) {
            Ok(sessions) => {
                self.sessions = sessions;
                if self.selected_session.is_none() {
                    self.selected_session = self.sessions.first().map(|session| session.session_id);
                }
            }
            Err(error) => self.set_error(error),
        }
    }

    fn selected_session(&self) -> Option<&SessionSnapshot> {
        let id = self.selected_session?;
        self.sessions
            .iter()
            .find(|session| session.session_id == id)
    }

    fn mutate_selected_with<F>(&mut self, f: F)
    where
        F: FnOnce(Uuid, &Runtime, &DaemonClient) -> anyhow::Result<SessionSnapshot>,
    {
        if let Some(session_id) = self.selected_session {
            match f(session_id, &self.rt, &self.client) {
                Ok(snapshot) => {
                    if let Some(slot) = self
                        .sessions
                        .iter_mut()
                        .find(|session| session.session_id == snapshot.session_id)
                    {
                        *slot = snapshot;
                    } else {
                        self.sessions.push(snapshot);
                    }
                }
                Err(error) => self.set_error(error),
            }
        } else {
            self.set_error("no session selected");
        }
    }

    fn dispatch_selected(&mut self, event: SessionEvent) {
        self.mutate_selected_with(|session_id, rt, client| {
            rt.block_on(client.dispatch_event(session_id, &DispatchEventRequest { event }))
        });
    }

    fn run_selected_binding(&mut self) {
        let vars = match parse_vars_map(&self.binding_vars_text) {
            Ok(vars) => vars,
            Err(error) => {
                self.set_error(error);
                return;
            }
        };

        let binding_id = self.binding_id.clone();
        self.mutate_selected_with(move |session_id, rt, client| {
            rt.block_on(client.run_binding(session_id, &RunBindingRequest { binding_id, vars }))
        });
    }

    fn phase_buttons(&mut self, ui: &mut egui::Ui, phase: SessionPhase) {
        match phase {
            SessionPhase::IntentDrafting | SessionPhase::IntentReady => {
                if ui.button("Formalize sample intent").clicked() {
                    if let Some(session_id) = self.selected_session {
                        self.dispatch_selected(SessionEvent::FormalizeIntent(Box::new(
                            IntentPacket::demo(
                                session_id,
                                self.selected_session()
                                    .map(|session| session.root.clone())
                                    .unwrap_or_else(|| ".".to_string()),
                            ),
                        )));
                    }
                }
                if ui.button("Draft spec").clicked() {
                    self.dispatch_selected(SessionEvent::DraftSpec);
                }
            }
            SessionPhase::SpecDraft | SessionPhase::SpecReview => {
                if ui.button("Approve spec").clicked() {
                    self.dispatch_selected(SessionEvent::ApproveSpec);
                }
            }
            SessionPhase::SpecApproved => {
                if ui.button("Propose realization").clicked() {
                    self.dispatch_selected(SessionEvent::ProposeRealization);
                }
            }
            SessionPhase::RealizationProposed => {
                if ui.button("Accept realization").clicked() {
                    self.dispatch_selected(SessionEvent::AcceptRealization);
                }
            }
            SessionPhase::VerifyRunning => {
                if ui.button("Mark verify passed").clicked() {
                    self.dispatch_selected(SessionEvent::VerificationPassed);
                }
                if ui.button("Mark verify failed").clicked() {
                    self.dispatch_selected(SessionEvent::VerificationFailed);
                }
            }
            SessionPhase::RepairEligible => {
                if ui.button("Repair spec-preserving").clicked() {
                    self.dispatch_selected(SessionEvent::RepairSpecPreserving);
                }
                if ui.button("Repair spec-changing").clicked() {
                    self.dispatch_selected(SessionEvent::RepairSpecChanging);
                }
            }
            SessionPhase::TrustReview => {
                if ui.button("Approve trust").clicked() {
                    self.dispatch_selected(SessionEvent::ApproveTrust);
                }
            }
            SessionPhase::CertifyRunning => {
                if ui.button("Mark certification passed").clicked() {
                    self.dispatch_selected(SessionEvent::CertificationPassed);
                }
            }
            SessionPhase::Certified => {
                if ui.button("Ingest incident").clicked() {
                    self.dispatch_selected(SessionEvent::IngestIncident);
                }
            }
            SessionPhase::IncidentIngesting | SessionPhase::HumanInterventionRequired => {}
        }
    }

    fn probe_provider(&mut self) {
        match self
            .rt
            .block_on(self.client.probe_provider(&self.provider_profile))
        {
            Ok(response) => {
                self.last_provider_report = serde_json::to_value(response.report).ok();
            }
            Err(error) => self.set_error(error),
        }
    }

    fn connect_mcp(&mut self) {
        let request = ConnectMcpRequest {
            endpoint: McpEndpoint::Http(self.mcp_endpoint.clone()),
            alias: None,
        };
        match self.rt.block_on(self.client.connect_mcp(&request)) {
            Ok(response) => {
                self.mcp_connection_id = Some(response.connection.connection_id.clone());
                self.mcp_tools = response.tools.into_iter().map(|tool| tool.name).collect();
                if let Some(first) = self.mcp_tools.first() {
                    self.mcp_tool_name = first.clone();
                }
            }
            Err(error) => self.set_error(error),
        }
    }

    fn call_mcp_tool(&mut self) {
        let Some(connection_id) = self.mcp_connection_id.clone() else {
            self.set_error("connect MCP first");
            return;
        };
        let arguments = match serde_json::from_str::<Value>(&self.mcp_tool_args_text) {
            Ok(value) => value,
            Err(error) => {
                self.set_error(format!("invalid tool args JSON: {error}"));
                return;
            }
        };

        match self.rt.block_on(self.client.call_mcp_tool(
            &connection_id,
            &self.mcp_tool_name,
            arguments,
        )) {
            Ok(response) => self.mcp_last_result = serde_json::to_value(response.result).ok(),
            Err(error) => self.set_error(error),
        }
    }
}

impl eframe::App for StudioApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("header").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                ui.heading("x07 Studio");
                let daemon_mode = if self.managed_daemon.is_some() {
                    "embedded daemon"
                } else {
                    "external daemon"
                };
                ui.label(format!("{daemon_mode} · {}", self.daemon_url));
                if ui.button("Refresh").clicked() {
                    self.refresh();
                }
                if ui.button("New session").clicked() {
                    match self
                        .rt
                        .block_on(self.client.create_session(&CreateSessionRequest {
                            title: self.new_session_title.clone(),
                            task_type: TaskType::NewBehavior,
                        })) {
                        Ok(snapshot) => {
                            self.selected_session = Some(snapshot.session_id);
                            self.sessions.push(snapshot);
                        }
                        Err(error) => self.set_error(error),
                    }
                }
                ui.text_edit_singleline(&mut self.new_session_title);
            });
            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
            }
            if let Some(health) = &self.health {
                component_summary(ui, health);
            }
        });

        egui::Panel::left("sessions")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Sessions");
                for session in &self.sessions {
                    let selected = self.selected_session == Some(session.session_id);
                    if ui
                        .selectable_label(
                            selected,
                            format!("{} · {:?}", session.title, session.phase),
                        )
                        .clicked()
                    {
                        self.selected_session = Some(session.session_id);
                    }
                }
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            if let Some(session) = self.selected_session().cloned() {
                ui.heading(format!("{} · {:?}", session.title, session.phase));
                ui.label(format!("Room: {:?}", session.room));
                ui.separator();

                self.phase_buttons(ui, session.phase.clone());

                ui.separator();
                ui.heading("Run canonical binding");
                ui.horizontal(|ui| {
                    ui.label("Binding");
                    ui.text_edit_singleline(&mut self.binding_id);
                    if ui.button("Run").clicked() {
                        self.run_selected_binding();
                    }
                });
                ui.label("Binding vars (JSON object)");
                ui.text_edit_multiline(&mut self.binding_vars_text);

                if let Some(intent) = &session.intent {
                    ui.separator();
                    ui.heading("Intent packet");
                    ui.monospace(serde_json::to_string_pretty(intent).unwrap_or_default());
                }

                if let Some(contract) = &session.contract {
                    ui.separator();
                    ui.heading("Session contract");
                    ui.monospace(serde_json::to_string_pretty(contract).unwrap_or_default());
                }

                ui.separator();
                ui.heading("Operation log");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for record in session.op_log.iter().rev() {
                        ui.group(|ui| {
                            ui.label(format!("{} · {:?}", record.op, record.status));
                            ui.monospace(record.command.join(" "));
                            if let Some(report_json) = &record.report_json {
                                ui.collapsing("report", |ui| {
                                    ui.monospace(
                                        serde_json::to_string_pretty(report_json)
                                            .unwrap_or_default(),
                                    );
                                });
                            }
                            if let Some(stderr) = &record.stderr {
                                if !stderr.trim().is_empty() {
                                    ui.collapsing("stderr", |ui| {
                                        ui.monospace(stderr);
                                    });
                                }
                            }
                        });
                    }
                });
            } else {
                ui.label("No session selected.");
            }
        });

        egui::Panel::right("providers_mcp")
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.heading("Provider probe");
                ui.text_edit_singleline(&mut self.provider_profile.label);
                ui.text_edit_singleline(&mut self.provider_profile.base_url);
                ui.text_edit_singleline(self.provider_profile.model.get_or_insert(String::new()));
                if ui.button("Probe provider").clicked() {
                    self.probe_provider();
                }
                if let Some(report) = &self.last_provider_report {
                    ui.collapsing("Last provider report", |ui| {
                        ui.monospace(serde_json::to_string_pretty(report).unwrap_or_default());
                    });
                }

                ui.separator();
                ui.heading("MCP");
                ui.text_edit_singleline(&mut self.mcp_endpoint.base_url);
                ui.text_edit_singleline(&mut self.mcp_endpoint.mcp_path);
                if ui.button("Connect MCP").clicked() {
                    self.connect_mcp();
                }
                if let Some(connection_id) = &self.mcp_connection_id {
                    ui.label(format!("Connection: {connection_id}"));
                    if !self.mcp_tools.is_empty() {
                        egui::ComboBox::from_label("Tool")
                            .selected_text(self.mcp_tool_name.clone())
                            .show_ui(ui, |ui| {
                                for tool in &self.mcp_tools {
                                    ui.selectable_value(
                                        &mut self.mcp_tool_name,
                                        tool.clone(),
                                        tool,
                                    );
                                }
                            });
                    }
                    ui.text_edit_multiline(&mut self.mcp_tool_args_text);
                    if ui.button("Call tool").clicked() {
                        self.call_mcp_tool();
                    }
                    if let Some(result) = &self.mcp_last_result {
                        ui.collapsing("Last MCP result", |ui| {
                            ui.monospace(serde_json::to_string_pretty(result).unwrap_or_default());
                        });
                    }
                }
            });
    }
}

fn component_summary(ui: &mut egui::Ui, health: &HealthResponse) {
    let missing_required = health
        .components
        .iter()
        .filter(|component| {
            component.required && component.status != RuntimeComponentState::Available
        })
        .count();
    let label = if missing_required == 0 {
        "Setup ready".to_string()
    } else {
        format!("{missing_required} required component(s) missing")
    };
    ui.collapsing(label, |ui| {
        ui.label(format!(
            "Workspace: {} · platform state: {}",
            health.workspace_root, health.defaults.platform_state_dir
        ));
        for component in &health.components {
            ui.horizontal_wrapped(|ui| {
                let marker = match component.status {
                    RuntimeComponentState::Available => "ready",
                    RuntimeComponentState::Missing => "missing",
                };
                let color = match component.status {
                    RuntimeComponentState::Available => egui::Color32::from_rgb(88, 210, 150),
                    RuntimeComponentState::Missing if component.required => egui::Color32::YELLOW,
                    RuntimeComponentState::Missing => egui::Color32::GRAY,
                };
                ui.colored_label(color, marker);
                ui.strong(&component.label);
                ui.monospace(&component.command);
                ui.label(
                    component
                        .source
                        .as_deref()
                        .unwrap_or(component.install_hint.as_str()),
                );
            });
        }
        ui.separator();
        ui.strong("Setup plan");
        for step in build_onboarding_plan(health).into_iter().take(6) {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.strong(&step.label);
                    ui.colored_label(step.color(), step.state);
                });
                ui.monospace(&step.command);
                ui.label(&step.detail);
            });
        }
    });
}

struct OnboardingStep {
    label: String,
    state: &'static str,
    command: String,
    detail: String,
}

impl OnboardingStep {
    fn color(&self) -> egui::Color32 {
        match self.state {
            "required" => egui::Color32::YELLOW,
            "ready" => egui::Color32::from_rgb(88, 210, 150),
            _ => egui::Color32::GRAY,
        }
    }

    fn rank(&self) -> usize {
        match self.state {
            "required" => 0,
            "ready" => 1,
            _ => 2,
        }
    }
}

fn build_onboarding_plan(health: &HealthResponse) -> Vec<OnboardingStep> {
    let missing_required = health.components.iter().any(|component| {
        component.required && component.status != RuntimeComponentState::Available
    });
    let mut steps = Vec::with_capacity(health.components.len() + 1);
    steps.push(OnboardingStep {
        label: "First-run defaults".to_string(),
        state: if missing_required {
            "required"
        } else {
            "ready"
        },
        command: ONBOARDING_BOOTSTRAP_COMMAND.to_string(),
        detail: format!(
            "workspace {} / daemon {} / platform {}",
            health.workspace_root, health.defaults.daemon_addr, health.defaults.platform_state_dir
        ),
    });
    steps.extend(health.components.iter().map(component_onboarding_step));
    steps.sort_by_key(OnboardingStep::rank);
    steps
}

fn component_onboarding_step(component: &RuntimeComponentStatus) -> OnboardingStep {
    let ready = component.status == RuntimeComponentState::Available;
    let env_var = component_env_var(&component.id);
    OnboardingStep {
        label: if component.required {
            format!("{} runtime", component.label)
        } else {
            format!("{} agent", component.label)
        },
        state: if ready {
            "ready"
        } else if component.required {
            "required"
        } else {
            "optional"
        },
        command: if ready {
            component
                .source
                .clone()
                .unwrap_or_else(|| component.command.clone())
        } else if env_var.is_some() {
            ONBOARDING_BOOTSTRAP_COMMAND.to_string()
        } else {
            component.command.clone()
        },
        detail: if ready {
            format!("{} resolved for local runs.", component.label)
        } else if let Some(env_var) = env_var {
            format!("{} Override with {env_var}.", component.install_hint)
        } else {
            component.install_hint.clone()
        },
    }
}

fn component_env_var(component_id: &str) -> Option<&'static str> {
    match component_id {
        "x07" => Some("X07_STUDIO_X07_EXE"),
        "x07-wasm" => Some("X07_STUDIO_X07_WASM_EXE"),
        "x07lp" => Some("X07_STUDIO_X07LP_EXE"),
        _ => None,
    }
}

fn set_managed_daemon_env(addr: SocketAddr) {
    let daemon_addr = addr.to_string();
    std::env::set_var("X07_STUDIO_DAEMON_ADDR", &daemon_addr);
    std::env::set_var("X07_STUDIO_DAEMON_URL", format!("http://{daemon_addr}"));
}

fn load_first_defaults_env(explicit: Option<&StdPath>) -> anyhow::Result<Option<PathBuf>> {
    let mut candidates = Vec::new();
    if let Some(path) = explicit {
        candidates.push(path.to_path_buf());
    } else {
        if let Ok(current_exe) = std::env::current_exe() {
            if let Some(bundle_root) = current_exe.parent().and_then(StdPath::parent) {
                candidates.push(bundle_root.join("defaults.env"));
            }
        }
        candidates.push(PathBuf::from("defaults.env"));
    }

    for candidate in candidates {
        if candidate.exists() {
            load_defaults_env(&candidate)?;
            return Ok(Some(candidate));
        }
    }

    if let Some(path) = explicit {
        anyhow::bail!("defaults env file not found: {}", path.display());
    }
    Ok(None)
}

fn load_defaults_env(path: &StdPath) -> anyhow::Result<()> {
    let base = path.parent().unwrap_or_else(|| StdPath::new("."));
    for line in fs::read_to_string(path)?.lines() {
        if let Some((key, value)) = parse_defaults_line(line, base) {
            std::env::set_var(key, value);
        }
    }
    Ok(())
}

fn parse_defaults_line(line: &str, base: &StdPath) -> Option<(String, String)> {
    let stripped = line.trim();
    if stripped.is_empty() || stripped.starts_with('#') {
        return None;
    }
    let (key, value) = stripped.split_once('=')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    let mut value = value.trim().trim_matches('"').to_string();
    if COMPONENT_ENV_KEYS.contains(&key) && !value.is_empty() {
        let path = StdPath::new(&value);
        if path.is_relative() {
            value = base.join(path).to_string_lossy().into_owned();
        }
    }
    Some((key.to_string(), value))
}

fn expand_user_path(value: String) -> String {
    let Some(rest) = value.strip_prefix("~/") else {
        return value;
    };
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(|home| {
            StdPath::new(&home)
                .join(rest)
                .to_string_lossy()
                .into_owned()
        })
        .unwrap_or(value)
}

fn parse_vars_map(input: &str) -> anyhow::Result<BTreeMap<String, String>> {
    if input.trim().is_empty() {
        return Ok(BTreeMap::new());
    }
    let value: Value = serde_json::from_str(input)?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("binding vars must be a JSON object"))?;
    let mut out = BTreeMap::new();
    for (key, value) in object {
        out.insert(
            key.clone(),
            value
                .as_str()
                .map(str::to_string)
                .unwrap_or_else(|| value.to_string()),
        );
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{
        build_onboarding_plan, expand_user_path, parse_defaults_line, set_managed_daemon_env,
        ONBOARDING_BOOTSTRAP_COMMAND,
    };
    use loom_types::api::{
        HealthResponse, RuntimeComponentState, RuntimeComponentStatus, StudioDefaults,
    };
    use std::ffi::OsString;
    use std::net::SocketAddr;
    use std::path::Path;

    #[test]
    fn defaults_env_resolves_relative_component_paths() {
        let parsed = parse_defaults_line(
            "X07_STUDIO_X07_WASM_EXE=\"components/x07-wasm\"",
            Path::new("/tmp/x07-studio-bundle"),
        )
        .expect("parsed env line");

        assert_eq!(parsed.0, "X07_STUDIO_X07_WASM_EXE");
        assert_eq!(parsed.1, "/tmp/x07-studio-bundle/components/x07-wasm");
    }

    #[test]
    fn defaults_env_ignores_comments() {
        assert!(parse_defaults_line("# X07_STUDIO_X07_EXE=\"/tmp/x07\"", Path::new(".")).is_none());
    }

    #[test]
    fn defaults_env_keeps_non_component_settings_literal() {
        let parsed = parse_defaults_line(
            "X07_STUDIO_WORKSPACE_ROOT=\"~/x07-studio-workspace\"",
            Path::new("/tmp/x07-studio-bundle"),
        )
        .expect("parsed env line");

        assert_eq!(parsed.0, "X07_STUDIO_WORKSPACE_ROOT");
        assert_eq!(parsed.1, "~/x07-studio-workspace");
    }

    #[test]
    fn workspace_root_expands_home_prefix() {
        let expanded = expand_user_path("~/x07-studio-workspace".to_string());

        assert!(!expanded.starts_with("~/"));
        assert!(expanded.ends_with("x07-studio-workspace"));
    }

    #[test]
    fn onboarding_plan_prioritizes_missing_required_components() {
        let health = HealthResponse {
            ok: true,
            workspace_root: "/tmp/x07-project".to_string(),
            defaults: StudioDefaults {
                daemon_addr: "127.0.0.1:7719".to_string(),
                provider_profile_id: "ollama-local".to_string(),
                platform_state_dir: ".x07/platform".to_string(),
            },
            components: vec![
                runtime_component(
                    "x07-wasm",
                    "x07-wasm",
                    "x07-wasm",
                    true,
                    RuntimeComponentState::Missing,
                    None,
                    "Install x07-wasm.",
                ),
                runtime_component(
                    "codex",
                    "OpenAI Codex",
                    "codex",
                    false,
                    RuntimeComponentState::Missing,
                    None,
                    "Install Codex CLI.",
                ),
            ],
        };

        let plan = build_onboarding_plan(&health);
        let defaults = plan
            .iter()
            .find(|step| step.label == "First-run defaults")
            .expect("defaults step");
        let wasm = plan
            .iter()
            .find(|step| step.label == "x07-wasm runtime")
            .expect("wasm step");
        let codex = plan
            .iter()
            .find(|step| step.label == "OpenAI Codex agent")
            .expect("codex step");

        assert_eq!(defaults.state, "required");
        assert!(defaults.detail.contains("daemon 127.0.0.1:7719"));
        assert_eq!(wasm.state, "required");
        assert_eq!(wasm.command, ONBOARDING_BOOTSTRAP_COMMAND);
        assert!(wasm.detail.contains("X07_STUDIO_X07_WASM_EXE"));
        assert_eq!(codex.state, "optional");
        assert_eq!(codex.command, "codex");
    }

    #[test]
    fn managed_daemon_env_records_runtime_addr() {
        let _guard = EnvRestore::new(&["X07_STUDIO_DAEMON_ADDR", "X07_STUDIO_DAEMON_URL"]);
        let addr: SocketAddr = "127.0.0.1:7788".parse().expect("socket addr");

        set_managed_daemon_env(addr);

        assert_eq!(
            std::env::var("X07_STUDIO_DAEMON_ADDR").as_deref(),
            Ok("127.0.0.1:7788")
        );
        assert_eq!(
            std::env::var("X07_STUDIO_DAEMON_URL").as_deref(),
            Ok("http://127.0.0.1:7788")
        );
    }

    fn runtime_component(
        id: &str,
        label: &str,
        command: &str,
        required: bool,
        status: RuntimeComponentState,
        source: Option<&str>,
        install_hint: &str,
    ) -> RuntimeComponentStatus {
        RuntimeComponentStatus {
            id: id.to_string(),
            label: label.to_string(),
            command: command.to_string(),
            required,
            status,
            source: source.map(str::to_string),
            install_hint: install_hint.to_string(),
        }
    }

    struct EnvRestore {
        entries: Vec<(&'static str, Option<OsString>)>,
    }

    impl EnvRestore {
        fn new(keys: &[&'static str]) -> Self {
            Self {
                entries: keys
                    .iter()
                    .map(|key| (*key, std::env::var_os(key)))
                    .collect(),
            }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, value) in &self.entries {
                if let Some(value) = value {
                    std::env::set_var(key, value);
                } else {
                    std::env::remove_var(key);
                }
            }
        }
    }
}
