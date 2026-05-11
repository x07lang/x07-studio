use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::Path as StdPath;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{http::StatusCode, Json, Router};
use camino::{Utf8Path, Utf8PathBuf};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use loom_core::WorkspaceKernel;
use loom_types::api::{
    AgentApprovalRequest, AgentApprovalResponse, AgentHandoffResponse, AgentRunRequest,
    AgentRunResponse, ArtifactPreviewRequest, ArtifactPreviewResponse, BindingDescriptor,
    CallMcpToolRequest, ConnectMcpRequest, ConnectMcpResponse, CreateSessionRequest,
    DispatchEventRequest, DocPreviewRequest, DocPreviewResponse, FormalizeIntentRequest,
    FormalizeIntentResponse, HealthResponse, McpCallResponse, ProbeProviderRequest,
    ProviderProbeResponse, ResolveApprovalRequest, RunBindingRequest, RuntimeComponentState,
    RuntimeComponentStatus, SaveAgentProfileRequest, SaveProviderProfileRequest, StudioDefaults,
    WorkspaceRadarResponse,
};
use loom_types::artifacts::{AgentProfile, ProviderProfile};
use loom_types::mcp::McpToolDescriptor;
use loom_types::session::SessionSnapshot;

#[derive(Clone)]
pub struct ApiState {
    pub kernel: Arc<Mutex<WorkspaceKernel>>,
}

pub fn router(state: ApiState) -> Router {
    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/workspace/radar", get(workspace_radar))
        .route("/v1/bindings", get(bindings))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{session_id}", get(get_session))
        .route("/v1/sessions/{session_id}/events", post(dispatch_event))
        .route(
            "/v1/sessions/{session_id}/intent/formalize",
            post(formalize_intent),
        )
        .route("/v1/sessions/{session_id}/bindings/run", post(run_binding))
        .route(
            "/v1/sessions/{session_id}/artifacts/preview",
            post(preview_artifact),
        )
        .route("/v1/sessions/{session_id}/docs/preview", post(preview_doc))
        .route(
            "/v1/sessions/{session_id}/xtal/run",
            post(run_xtal_workflow),
        )
        .route("/v1/providers", get(list_providers).post(save_provider))
        .route("/v1/providers/probe", post(probe_provider))
        .route("/v1/agents", get(list_agents).post(save_agent))
        .route(
            "/v1/sessions/{session_id}/agents/{agent_id}/handoff",
            post(create_agent_handoff),
        )
        .route(
            "/v1/sessions/{session_id}/agents/{agent_id}/run",
            post(run_agent_handoff),
        )
        .route(
            "/v1/sessions/{session_id}/agents/{agent_id}/approval",
            post(create_agent_approval),
        )
        .route(
            "/v1/sessions/{session_id}/approvals/{op_id}",
            post(resolve_agent_approval),
        )
        .route("/v1/mcp/connect", post(connect_mcp))
        .route("/v1/mcp/{connection_id}/tools", get(list_mcp_tools))
        .route("/v1/mcp/{connection_id}/call", post(call_mcp_tool))
        .route("/v1/mcp/{connection_id}", delete(close_mcp))
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
}

pub async fn serve(addr: SocketAddr, state: ApiState) -> anyhow::Result<()> {
    let listener = tokio::net::TcpListener::bind(addr).await?;
    serve_listener(listener, state).await
}

pub async fn serve_listener(
    listener: tokio::net::TcpListener,
    state: ApiState,
) -> anyhow::Result<()> {
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub fn default_state(root: impl Into<camino::Utf8PathBuf>) -> anyhow::Result<ApiState> {
    Ok(ApiState {
        kernel: Arc::new(Mutex::new(WorkspaceKernel::open(root)?)),
    })
}

fn runtime_components(root: &Utf8Path) -> Vec<RuntimeComponentStatus> {
    vec![
        component_status(
            root,
            "x07",
            "x07 CLI",
            "x07",
            Some("X07_STUDIO_X07_EXE"),
            true,
            &["x07/target/release/x07", "x07/target/debug/x07"],
            "Install the x07 toolchain, build the sibling x07 repo, or set X07_STUDIO_X07_EXE.",
        ),
        component_status(
            root,
            "x07-wasm",
            "x07-wasm",
            "x07-wasm",
            Some("X07_STUDIO_X07_WASM_EXE"),
            true,
            &[
                "x07-wasm-backend/target/release/x07-wasm",
                "x07-wasm-backend/target/debug/x07-wasm",
            ],
            "Install x07-wasm, build the sibling x07-wasm-backend repo, or set X07_STUDIO_X07_WASM_EXE.",
        ),
        component_status(
            root,
            "x07lp",
            "x07 platform",
            "x07lp",
            Some("X07_STUDIO_X07LP_EXE"),
            true,
            &["x07-platform/scripts/x07lp-driver"],
            "Install x07lp, place x07-platform beside Studio, or set X07_STUDIO_X07LP_EXE.",
        ),
        component_status(
            root,
            "codex",
            "OpenAI Codex",
            "codex",
            None,
            false,
            &[],
            "Install Codex CLI when supervised Codex handoffs should execute locally.",
        ),
        component_status(
            root,
            "claude-code",
            "Claude Code",
            "claude",
            None,
            false,
            &[],
            "Install Claude Code when supervised Claude handoffs should execute locally.",
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
fn component_status(
    root: &Utf8Path,
    id: &str,
    label: &str,
    command: &str,
    env_var: Option<&str>,
    required: bool,
    sibling_candidates: &[&str],
    install_hint: &str,
) -> RuntimeComponentStatus {
    let source = env_var
        .and_then(env_component_source)
        .or_else(|| sibling_component_source(root, sibling_candidates))
        .or_else(|| path_component_source(command));
    let status = if source.is_some() {
        RuntimeComponentState::Available
    } else {
        RuntimeComponentState::Missing
    };
    RuntimeComponentStatus {
        id: id.to_string(),
        label: label.to_string(),
        command: command.to_string(),
        required,
        status,
        source,
        install_hint: install_hint.to_string(),
    }
}

fn env_component_source(env_var: &str) -> Option<String> {
    let value = std::env::var(env_var).ok()?;
    if value.trim().is_empty() {
        return None;
    }
    if executable_path_exists(StdPath::new(&value)) {
        Some(format!("{env_var}={value}"))
    } else {
        None
    }
}

fn sibling_component_source(root: &Utf8Path, candidates: &[&str]) -> Option<String> {
    for base in component_search_bases(root) {
        for ancestor in base.ancestors().take(8) {
            for candidate in candidates {
                let path = ancestor.join(candidate);
                if executable_path_exists(path.as_std_path()) {
                    return Some(path.to_string());
                }
            }
        }
    }
    None
}

fn component_search_bases(root: &Utf8Path) -> Vec<Utf8PathBuf> {
    let mut bases = vec![root.to_owned()];
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(cwd) = Utf8PathBuf::from_path_buf(cwd) {
            bases.push(cwd);
        }
    }
    bases
}

fn path_component_source(command: &str) -> Option<String> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        for executable in executable_names(command) {
            let path = dir.join(&executable);
            if executable_path_exists(&path) {
                return Some(path.to_string_lossy().into_owned());
            }
        }
    }
    None
}

fn executable_names(command: &str) -> Vec<OsString> {
    let mut names = vec![OsString::from(command)];
    if cfg!(windows) && !command.ends_with(".exe") {
        names.push(OsString::from(format!("{command}.exe")));
    }
    names
}

fn executable_path_exists(path: &StdPath) -> bool {
    path.is_file()
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let kernel = state.kernel.lock().await;
    let workspace_root = kernel.workspace_root().to_string();
    Json(HealthResponse {
        ok: true,
        workspace_root,
        defaults: StudioDefaults {
            daemon_addr: "127.0.0.1:7719".to_string(),
            provider_profile_id: "ollama-local".to_string(),
            platform_state_dir: ".x07/platform".to_string(),
        },
        components: runtime_components(kernel.workspace_root()),
    })
}

async fn workspace_radar(State(state): State<ApiState>) -> Json<WorkspaceRadarResponse> {
    let kernel = state.kernel.lock().await;
    Json(kernel.workspace_radar())
}

async fn bindings(State(state): State<ApiState>) -> Json<Vec<BindingDescriptor>> {
    let kernel = state.kernel.lock().await;
    Json(kernel.list_bindings())
}

async fn list_sessions(State(state): State<ApiState>) -> Json<Vec<SessionSnapshot>> {
    let kernel = state.kernel.lock().await;
    Json(kernel.list_sessions())
}

async fn create_session(
    State(state): State<ApiState>,
    Json(request): Json<CreateSessionRequest>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .create_session(request.title, request.task_type)
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn get_session(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let snapshot = kernel.get_session(session_id).ok_or_else(not_found)?;
    Ok(Json(snapshot))
}

async fn dispatch_event(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<DispatchEventRequest>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .dispatch_event(session_id, request.event)
        .map_err(conflict_error)?;
    Ok(Json(snapshot))
}

async fn formalize_intent(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<FormalizeIntentRequest>,
) -> Result<Json<FormalizeIntentResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (intent, op, session) = kernel
        .formalize_intent(
            session_id,
            &request.raw,
            request.input_mode,
            &request.revision_notes,
        )
        .map_err(conflict_error)?;
    Ok(Json(FormalizeIntentResponse {
        intent,
        op,
        session,
    }))
}

async fn run_binding(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<RunBindingRequest>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .run_binding(session_id, &request.binding_id, &request.vars)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn preview_artifact(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<ArtifactPreviewRequest>,
) -> Result<Json<ArtifactPreviewResponse>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let preview = kernel
        .preview_artifact(session_id, &request.artifact)
        .map_err(conflict_error)?;
    Ok(Json(preview))
}

async fn preview_doc(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<DocPreviewRequest>,
) -> Result<Json<DocPreviewResponse>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let preview = kernel
        .preview_doc(session_id, &request.doc_ref)
        .map_err(conflict_error)?;
    Ok(Json(preview))
}

async fn run_xtal_workflow(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .run_xtal_workflow(session_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn list_providers(
    State(state): State<ApiState>,
) -> Result<Json<Vec<ProviderProfile>>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let profiles = kernel.list_provider_profiles().map_err(internal_error)?;
    Ok(Json(profiles))
}

async fn save_provider(
    State(state): State<ApiState>,
    Json(request): Json<SaveProviderProfileRequest>,
) -> Result<Json<ProviderProfile>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    kernel
        .save_provider_profile(&request.profile)
        .map_err(internal_error)?;
    Ok(Json(request.profile))
}

async fn list_agents(
    State(state): State<ApiState>,
) -> Result<Json<Vec<AgentProfile>>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let profiles = kernel.list_agent_profiles().map_err(internal_error)?;
    Ok(Json(profiles))
}

async fn save_agent(
    State(state): State<ApiState>,
    Json(request): Json<SaveAgentProfileRequest>,
) -> Result<Json<AgentProfile>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    kernel
        .save_agent_profile(&request.profile)
        .map_err(internal_error)?;
    Ok(Json(request.profile))
}

async fn create_agent_handoff(
    Path((session_id, agent_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
) -> Result<Json<AgentHandoffResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (handoff, session) = kernel
        .create_agent_handoff(session_id, &agent_id)
        .map_err(internal_error)?;
    Ok(Json(AgentHandoffResponse { handoff, session }))
}

async fn run_agent_handoff(
    Path((session_id, agent_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
    Json(request): Json<AgentRunRequest>,
) -> Result<Json<AgentRunResponse>, (StatusCode, String)> {
    let prepared = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .start_agent_handoff(session_id, &agent_id, request.mode, request.timeout_seconds)
            .map_err(internal_error)?
    };
    let (op, session) = if let Some(command) = prepared.command {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let execution = WorkspaceKernel::execute_agent_command_streaming(command, tx);
        tokio::pin!(execution);
        let session = loop {
            tokio::select! {
                update = rx.recv() => {
                    if let Some(update) = update {
                        let mut kernel = state.kernel.lock().await;
                        kernel
                            .complete_agent_run(update)
                            .map_err(internal_error)?;
                    }
                }
                final_op = &mut execution => {
                    let mut kernel = state.kernel.lock().await;
                    let session = kernel
                        .complete_agent_run(final_op.clone())
                        .map_err(internal_error)?;
                    break session;
                }
            }
        };
        let op = session
            .op_log
            .last()
            .cloned()
            .unwrap_or_else(|| prepared.op.clone());
        (op, session)
    } else {
        (prepared.op.clone(), prepared.session.clone())
    };
    Ok(Json(AgentRunResponse {
        handoff: prepared.handoff,
        op,
        session,
    }))
}

async fn create_agent_approval(
    Path((session_id, agent_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
    Json(request): Json<AgentApprovalRequest>,
) -> Result<Json<AgentApprovalResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (op, session) = kernel
        .create_agent_approval(session_id, &agent_id, request.reason)
        .map_err(internal_error)?;
    Ok(Json(AgentApprovalResponse { op, session }))
}

async fn resolve_agent_approval(
    Path((session_id, op_id)): Path<(Uuid, Uuid)>,
    State(state): State<ApiState>,
    Json(request): Json<ResolveApprovalRequest>,
) -> Result<Json<AgentApprovalResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (op, session) = kernel
        .resolve_agent_approval(session_id, op_id, request.decision, request.notes)
        .map_err(internal_error)?;
    Ok(Json(AgentApprovalResponse { op, session }))
}

async fn probe_provider(
    State(state): State<ApiState>,
    Json(request): Json<ProbeProviderRequest>,
) -> Result<Json<ProviderProbeResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (profile, report) = kernel
        .probe_provider(request.profile)
        .await
        .map_err(internal_error)?;
    Ok(Json(ProviderProbeResponse { profile, report }))
}

async fn connect_mcp(
    State(state): State<ApiState>,
    Json(request): Json<ConnectMcpRequest>,
) -> Result<Json<ConnectMcpResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (connection, tools) = kernel
        .connect_mcp(request.endpoint, request.alias)
        .await
        .map_err(internal_error)?;
    Ok(Json(ConnectMcpResponse { connection, tools }))
}

async fn list_mcp_tools(
    Path(connection_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<McpToolDescriptor>>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let tools = kernel
        .list_mcp_tools(&connection_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(tools))
}

async fn call_mcp_tool(
    Path(connection_id): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<CallMcpToolRequest>,
) -> Result<Json<McpCallResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let result = kernel
        .call_mcp_tool(&connection_id, &request.name, request.arguments)
        .await
        .map_err(internal_error)?;
    Ok(Json(McpCallResponse { result }))
}

async fn close_mcp(
    Path(connection_id): Path<String>,
    State(state): State<ApiState>,
) -> Result<StatusCode, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    kernel
        .close_mcp_connection(&connection_id)
        .await
        .map_err(internal_error)?;
    Ok(StatusCode::NO_CONTENT)
}

fn internal_error(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn conflict_error(error: impl ToString) -> (StatusCode, String) {
    (StatusCode::CONFLICT, error.to_string())
}

fn not_found() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "not found".to_string())
}

#[cfg(test)]
mod tests {
    use super::{runtime_components, sibling_component_source};

    #[test]
    fn runtime_components_include_required_x07_wasm_and_platform_tools() {
        let root = temp_root();
        let components = runtime_components(root.as_path());
        let required = components
            .iter()
            .filter(|component| component.required)
            .map(|component| component.id.as_str())
            .collect::<Vec<_>>();

        assert!(required.contains(&"x07"));
        assert!(required.contains(&"x07-wasm"));
        assert!(required.contains(&"x07lp"));
        assert!(components
            .iter()
            .any(|component| component.id == "codex" && !component.required));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn sibling_component_source_searches_workspace_ancestors() {
        let root = temp_root();
        let project = root.join("workspace/x07-studio/test-project");
        let driver = root.join("workspace/x07-platform/scripts/x07lp-driver");
        std::fs::create_dir_all(driver.parent().expect("driver parent"))
            .expect("create driver dir");
        std::fs::create_dir_all(&project).expect("create project dir");
        std::fs::write(&driver, "#!/usr/bin/env bash\n").expect("write driver");

        assert_eq!(
            sibling_component_source(project.as_path(), &["x07-platform/scripts/x07lp-driver"])
                .as_deref(),
            Some(driver.as_str())
        );

        std::fs::remove_dir_all(root).ok();
    }

    fn temp_root() -> camino::Utf8PathBuf {
        camino::Utf8PathBuf::from_path_buf(std::env::temp_dir())
            .expect("utf8 temp")
            .join(format!(
                "x07-studio-daemon-test-{}-{}",
                std::process::id(),
                uuid::Uuid::new_v4()
            ))
    }
}
