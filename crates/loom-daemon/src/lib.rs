use std::env;
use std::ffi::OsString;
use std::net::SocketAddr;
use std::path::Path as StdPath;
use std::sync::Arc;

use axum::extract::{Multipart, Path, State};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::IntoResponse;
use axum::routing::{delete, get, post};
use axum::{http::StatusCode, Json, Router};
use camino::{Utf8Path, Utf8PathBuf};
use futures::stream::{Stream, StreamExt};
use serde_json::Value;
use std::convert::Infallible;
use std::pin::Pin;
use tokio::sync::Mutex;
use tokio_stream::wrappers::BroadcastStream;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use loom_core::WorkspaceKernel;
use loom_types::api::{
    AgentApprovalRequest, AgentApprovalResponse, AgentHandoffResponse, AgentRunRequest,
    AgentRunResponse, ArtifactPreviewRequest, ArtifactPreviewResponse, AutopilotPolicy,
    AutopilotResponse, AutopilotStartRequest, BindingDescriptor, CallMcpToolRequest,
    CassetteBranchRequest, CassetteEntry, ClimbRungRequest, ConnectMcpRequest, ConnectMcpResponse,
    CreateSessionRequest, DispatchEventRequest, DocPreviewRequest, DocPreviewResponse,
    FormalizeIntentRequest, FormalizeIntentResponse, HealthResponse, IntentAnswerRequest,
    IntentAnswerResponse, IntentClarifyRequest, IntentClarifyResponse, IntentImageUploadResponse,
    IntentInputMode, LadderState, LiveDiff, McpCallResponse, PickRealizeProposalRequest,
    PickRealizeProposalResponse, ProbeProviderRequest, ProviderProbeResponse, QuorumRequest,
    QuorumRound, RealizeQuorumRequest, RealizeQuorumRound, RealizeRequest, RealizeResponse,
    ReleaseRequest, ReleaseStatus, ReplayExportResponse, ReplayImportRequest,
    RequestIntentRevisionRequest, RequestIntentRevisionResponse, ResolveApprovalRequest,
    RunBindingRequest, RunBuildRequest, RunXtalWorkflowRequest, RuntimeComponentState,
    RuntimeComponentStatus, SaveAgentProfileRequest, SaveProviderProfileRequest, SessionTurn,
    StudioDefaults, StudioMemory, SyncClaimResponse, SyncCode, SyncStateRequest, TryItRequest,
    TryItResult, VisualEmitRequest, VisualParseRequest, VisualResponse, WorkspaceRadarResponse,
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
        .route("/v1/sessions/{session_id}/stream", get(stream_session))
        .route(
            "/v1/sessions/{session_id}/diffs/live",
            get(stream_live_diffs),
        )
        .route("/v1/sessions/{session_id}/turns", get(list_turns))
        .route("/v1/sessions/{session_id}/events", post(dispatch_event))
        .route(
            "/v1/sessions/{session_id}/intent/formalize",
            post(formalize_intent),
        )
        .route(
            "/v1/sessions/{session_id}/intent/voice",
            post(formalize_voice_intent),
        )
        .route(
            "/v1/sessions/{session_id}/intent/revision",
            post(request_intent_revision),
        )
        .route(
            "/v1/sessions/{session_id}/intent/clarify",
            post(run_intent_clarify),
        )
        .route(
            "/v1/sessions/{session_id}/intent/answer",
            post(apply_intent_answer),
        )
        .route(
            "/v1/sessions/{session_id}/intent/quorum",
            post(run_intent_quorum),
        )
        .route(
            "/v1/sessions/{session_id}/intent/image",
            post(upload_intent_image),
        )
        .route("/v1/sessions/{session_id}/bindings/run", post(run_binding))
        .route("/v1/sessions/{session_id}/invoke", post(invoke_artifact))
        .route(
            "/v1/sessions/{session_id}/realize",
            post(realize_with_agent),
        )
        .route(
            "/v1/sessions/{session_id}/realize/quorum",
            post(realize_quorum),
        )
        .route(
            "/v1/sessions/{session_id}/realize/pick",
            post(pick_realize_proposal),
        )
        .route(
            "/v1/sessions/{session_id}/autopilot/start",
            post(start_autopilot),
        )
        .route(
            "/v1/sessions/{session_id}/autopilot/pause",
            post(pause_autopilot),
        )
        .route("/v1/sessions/{session_id}/ladder", get(ladder_state))
        .route("/v1/sessions/{session_id}/ladder/climb", post(climb_ladder))
        .route(
            "/v1/sessions/{session_id}/ladder/release",
            post(submit_release),
        )
        .route(
            "/v1/sessions/{session_id}/ladder/release/{release_id}",
            get(get_release_status),
        )
        .route("/v1/sessions/{session_id}/cassette", get(cassette_entries))
        .route(
            "/v1/sessions/{session_id}/cassette/branch",
            post(branch_cassette),
        )
        .route("/v1/sessions/{session_id}/ask", post(ask_project))
        .route(
            "/v1/sessions/{session_id}/incidents/scan",
            post(scan_incidents),
        )
        .route(
            "/v1/sessions/{session_id}/incidents/watch",
            post(watch_incidents),
        )
        .route(
            "/v1/sessions/{session_id}/incidents/{incident_id}/repair",
            post(repair_incident),
        )
        .route(
            "/v1/sessions/{session_id}/visual/streampipe/parse",
            post(visual_streampipe_parse),
        )
        .route(
            "/v1/sessions/{session_id}/visual/streampipe/emit",
            post(visual_streampipe_emit),
        )
        .route(
            "/v1/sessions/{session_id}/visual/statemachine/parse",
            post(visual_statemachine_parse),
        )
        .route(
            "/v1/sessions/{session_id}/visual/statemachine/emit",
            post(visual_statemachine_emit),
        )
        .route(
            "/v1/sessions/{session_id}/visual/tasks/parse",
            post(visual_tasks_parse),
        )
        .route(
            "/v1/sessions/{session_id}/visual/tasks/emit",
            post(visual_tasks_emit),
        )
        .route(
            "/v1/sessions/{session_id}/artifacts/preview",
            post(preview_artifact),
        )
        .route("/v1/sessions/{session_id}/docs/preview", post(preview_doc))
        .route(
            "/v1/sessions/{session_id}/xtal/run",
            post(run_xtal_workflow),
        )
        .route("/v1/sessions/{session_id}/build", post(run_build_pipeline))
        .route("/v1/providers", get(list_providers).post(save_provider))
        .route("/v1/sync/codes", get(mint_sync_code))
        .route("/v1/sync/{code}/claim", post(claim_sync_code))
        .route("/v1/sync/sessions/{code}/state", post(save_sync_state))
        .route(
            "/v1/sessions/{session_id}/replay/export",
            post(export_replay),
        )
        .route("/v1/replay/import", post(import_replay))
        .route("/v1/memory", get(load_memory).post(save_memory))
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
            &[
                "components/x07",
                "x07/target/release/x07",
                "x07/target/debug/x07",
            ],
            "Install the x07 toolchain, use a bundle with components/x07, build the sibling x07 repo, or set X07_STUDIO_X07_EXE.",
        ),
        component_status(
            root,
            "x07-wasm",
            "x07-wasm",
            "x07-wasm",
            Some("X07_STUDIO_X07_WASM_EXE"),
            true,
            &[
                "components/x07-wasm",
                "x07-wasm-backend/target/release/x07-wasm",
                "x07-wasm-backend/target/debug/x07-wasm",
            ],
            "Install x07-wasm, use a bundle with components/x07-wasm, build the sibling x07-wasm-backend repo, or set X07_STUDIO_X07_WASM_EXE.",
        ),
        component_status(
            root,
            "x07lp",
            "x07 platform",
            "x07lp",
            Some("X07_STUDIO_X07LP_EXE"),
            true,
            &[
                "components/x07lp",
                "components/x07lp-driver",
                "x07-platform/scripts/x07lp-driver",
            ],
            "Install x07lp, use a bundle with components/x07lp, place x07-platform beside Studio, or set X07_STUDIO_X07LP_EXE.",
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
                if let Some(path) = executable_path_variant(&path) {
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

fn executable_path_variant(path: &Utf8Path) -> Option<Utf8PathBuf> {
    if executable_path_exists(path.as_std_path()) {
        return Some(path.to_owned());
    }
    if cfg!(windows) && path.extension() != Some("exe") {
        let mut with_exe = path.to_owned();
        with_exe.set_extension("exe");
        if executable_path_exists(with_exe.as_std_path()) {
            return Some(with_exe);
        }
    }
    None
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let kernel = state.kernel.lock().await;
    let workspace_root = kernel.workspace_root().to_string();
    Json(HealthResponse {
        ok: true,
        workspace_root,
        defaults: studio_defaults(),
        components: runtime_components(kernel.workspace_root()),
    })
}

fn studio_defaults() -> StudioDefaults {
    StudioDefaults {
        daemon_addr: env_setting("X07_STUDIO_DAEMON_ADDR", "127.0.0.1:7719"),
        provider_profile_id: env_setting("X07_STUDIO_PROVIDER_PROFILE_ID", "ollama-local"),
        platform_state_dir: env_setting("X07_STUDIO_PLATFORM_STATE_DIR", ".x07/platform"),
    }
}

fn env_setting(key: &str, default: &str) -> String {
    env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default.to_string())
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

async fn list_turns(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<SessionTurn>>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let turns = kernel.session_turns(session_id).map_err(conflict_error)?;
    Ok(Json(turns))
}

async fn stream_session(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let (bus, initial) = {
        let kernel = state.kernel.lock().await;
        let initial = kernel.get_session(session_id);
        (kernel.event_bus(), initial)
    };
    let initial = initial.ok_or_else(not_found)?;
    let initial_event = Event::default()
        .json_data(loom_types::api::SessionStreamEvent::Snapshot {
            session: Box::new(initial),
        })
        .map_err(internal_error)?;
    let receiver = bus.subscribe(session_id);
    let live = BroadcastStream::new(receiver).filter_map(|event| async move {
        let event = event.ok()?;
        Event::default().json_data(event).ok().map(Ok)
    });
    let stream =
        futures::stream::once(async move { Ok::<_, Infallible>(initial_event) }).chain(live);
    let boxed: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(stream);
    Ok(Sse::new(boxed)
        .keep_alive(KeepAlive::default())
        .into_response())
}

async fn stream_live_diffs(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<axum::response::Response, (StatusCode, String)> {
    let (bus, existing) = {
        let kernel = state.kernel.lock().await;
        let session = kernel.get_session(session_id).ok_or_else(not_found)?;
        let existing = session
            .op_log
            .iter()
            .filter_map(live_diff_from_op)
            .collect::<Vec<_>>();
        (kernel.event_bus(), existing)
    };
    let initial = futures::stream::iter(
        existing
            .into_iter()
            .filter_map(|diff| Event::default().json_data(diff).ok().map(Ok)),
    );
    let receiver = bus.subscribe(session_id);
    let live = BroadcastStream::new(receiver).filter_map(|event| async move {
        let event = event.ok()?;
        match event {
            loom_types::api::SessionStreamEvent::Op { op } => live_diff_from_op(&op)
                .and_then(|diff| Event::default().json_data(diff).ok().map(Ok)),
            _ => None,
        }
    });
    let stream = initial.chain(live);
    let boxed: Pin<Box<dyn Stream<Item = Result<Event, Infallible>> + Send>> = Box::pin(stream);
    Ok(Sse::new(boxed)
        .keep_alive(KeepAlive::default())
        .into_response())
}

fn live_diff_from_op(op: &loom_types::artifacts::OpRecord) -> Option<LiveDiff> {
    let value = op
        .report_json
        .as_ref()?
        .get("event")?
        .get("input")?
        .get("live_diff")?
        .clone();
    serde_json::from_value(value).ok()
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
    formalize_intent_request(session_id, state, request).await
}

async fn formalize_voice_intent(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(mut request): Json<FormalizeIntentRequest>,
) -> Result<Json<FormalizeIntentResponse>, (StatusCode, String)> {
    request.input_mode = IntentInputMode::Voice;
    if request.raw.trim().is_empty() {
        if let Some(transcript) = &request.voice_transcript {
            request.raw = transcript.text.clone();
        }
    }
    formalize_intent_request(session_id, state, request).await
}

async fn formalize_intent_request(
    session_id: Uuid,
    state: ApiState,
    request: FormalizeIntentRequest,
) -> Result<Json<FormalizeIntentResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (intent, op, session) = kernel
        .formalize_intent_with_provider(
            session_id,
            &request.raw,
            request.input_mode,
            &request.revision_notes,
            request.provider_profile_id.as_deref(),
            request.voice_transcript.as_ref(),
        )
        .await
        .map_err(conflict_error)?;
    Ok(Json(FormalizeIntentResponse {
        intent,
        op,
        session,
    }))
}

async fn request_intent_revision(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<RequestIntentRevisionRequest>,
) -> Result<Json<RequestIntentRevisionResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (op, session) = kernel
        .request_intent_revision(session_id, &request.note)
        .map_err(conflict_error)?;
    Ok(Json(RequestIntentRevisionResponse { op, session }))
}

async fn run_intent_clarify(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<IntentClarifyRequest>,
) -> Result<Json<IntentClarifyResponse>, (StatusCode, String)> {
    let genpack_seed = {
        let kernel = state.kernel.lock().await;
        kernel
            .genpack_context_seed(session_id)
            .map_err(conflict_error)?
    };
    let genpack = WorkspaceKernel::resolve_genpack_context(genpack_seed).await;
    let prepared = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .start_intent_clarify_with_genpack(
                session_id,
                &request.agent_id,
                request.timeout_seconds,
                genpack.as_ref(),
            )
            .map_err(conflict_error)?
    };
    let handoff = prepared.handoff.clone();
    let run_op_id = prepared.op.id;
    let (op, _intermediate_session) = if let Some(command) = prepared.command {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let execution = loom_core::WorkspaceKernel::execute_agent_command_streaming(command, tx);
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
            .iter()
            .find(|op| op.id == run_op_id)
            .cloned()
            .unwrap_or_else(|| prepared.op.clone());
        (op, session)
    } else {
        (prepared.op.clone(), prepared.session.clone())
    };
    let session = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .ingest_clarify_questions(session_id, &request.agent_id, run_op_id)
            .map_err(internal_error)?
    };
    Ok(Json(IntentClarifyResponse {
        handoff,
        op,
        session,
    }))
}

async fn apply_intent_answer(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<IntentAnswerRequest>,
) -> Result<Json<IntentAnswerResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (intent, op, session) = kernel
        .apply_intent_answers(session_id, &request.answers)
        .map_err(conflict_error)?;
    Ok(Json(IntentAnswerResponse {
        intent,
        op,
        session,
    }))
}

async fn run_intent_quorum(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<QuorumRequest>,
) -> Result<Json<QuorumRound>, (StatusCode, String)> {
    let genpack_seed = {
        let kernel = state.kernel.lock().await;
        kernel
            .genpack_context_seed(session_id)
            .map_err(conflict_error)?
    };
    let genpack = WorkspaceKernel::resolve_genpack_context(genpack_seed).await;
    let prepared = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .prepare_intent_quorum_with_genpack(
                session_id,
                &request.agent_ids,
                request.timeout_seconds,
                genpack.as_ref(),
            )
            .map_err(conflict_error)?
    };
    let round = prepared
        .iter()
        .filter_map(|item| item.clarify_round)
        .min()
        .ok_or_else(|| internal_error(anyhow::anyhow!("quorum prepared no clarify runs")))?;
    let runs = prepared
        .iter()
        .map(|item| (item.handoff.agent_id.clone(), item.op.id))
        .collect::<Vec<_>>();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let mut join_set = tokio::task::JoinSet::new();
    for item in prepared {
        if let Some(command) = item.command {
            let updates = tx.clone();
            join_set.spawn(async move {
                WorkspaceKernel::execute_agent_command_streaming(command, updates).await
            });
        }
    }
    drop(tx);
    let mut running = join_set.len();
    let mut updates_open = true;
    while running > 0 || updates_open {
        tokio::select! {
            update = rx.recv(), if updates_open => {
                if let Some(update) = update {
                    let mut kernel = state.kernel.lock().await;
                    kernel
                        .complete_agent_run(update)
                        .map_err(internal_error)?;
                } else {
                    updates_open = false;
                }
            }
            joined = join_set.join_next(), if running > 0 => {
                running -= 1;
                let final_op = joined
                    .ok_or_else(|| internal_error(anyhow::anyhow!("quorum task ended without result")))?
                    .map_err(|error| internal_error(anyhow::anyhow!("quorum task join failed: {error}")))?;
                let mut kernel = state.kernel.lock().await;
                kernel
                    .complete_agent_run(final_op)
                    .map_err(internal_error)?;
            }
        }
    }
    {
        let mut kernel = state.kernel.lock().await;
        for (agent_id, run_op_id) in &runs {
            kernel
                .ingest_clarify_questions_at_round(session_id, agent_id, *run_op_id, round)
                .map_err(internal_error)?;
        }
    }
    let round = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .complete_intent_quorum(session_id, round, &request.agent_ids)
            .map_err(conflict_error)?
    };
    Ok(Json(round))
}

async fn upload_intent_image(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    mut multipart: Multipart,
) -> Result<Json<IntentImageUploadResponse>, (StatusCode, String)> {
    const MAX_IMAGE_BYTES: usize = 8 * 1024 * 1024;

    let mut file_mime = None;
    let mut declared_mime = None;
    let mut bytes = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?
    {
        match field.name().unwrap_or_default() {
            "file" | "image" => {
                if let Some(content_type) = field.content_type() {
                    file_mime = Some(content_type.to_string());
                }
                let data = field
                    .bytes()
                    .await
                    .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
                if data.len() > MAX_IMAGE_BYTES {
                    return Err((
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "image upload exceeds 8 MiB".to_string(),
                    ));
                }
                bytes = Some(data.to_vec());
            }
            "mime" => {
                declared_mime = Some(
                    field
                        .text()
                        .await
                        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?,
                );
            }
            _ => {}
        }
    }

    let bytes = bytes.ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            "multipart upload must include a file field".to_string(),
        )
    })?;
    let mime = declared_mime
        .or(file_mime)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    if !mime.starts_with("image/") {
        return Err((
            StatusCode::BAD_REQUEST,
            "multipart file must use an image/* content type".to_string(),
        ));
    }

    let kernel = state.kernel.lock().await;
    let path = kernel
        .save_intent_image(session_id, &mime, &bytes)
        .map_err(conflict_error)?;
    Ok(Json(IntentImageUploadResponse { path }))
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

async fn invoke_artifact(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<TryItRequest>,
) -> Result<Json<TryItResult>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let result = kernel
        .invoke_artifact(session_id, request)
        .await
        .map_err(internal_error)?;
    Ok(Json(result))
}

async fn realize_with_agent(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    request: Option<Json<RealizeRequest>>,
) -> Result<Json<RealizeResponse>, (StatusCode, String)> {
    let body = request.map(|Json(body)| body).unwrap_or(RealizeRequest {
        agent_id: None,
        timeout_seconds: None,
    });
    let agent_id = body
        .agent_id
        .clone()
        .unwrap_or_else(|| "claude-code".to_string());
    let prepared = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .start_intent_realize(session_id, &agent_id, body.timeout_seconds)
            .await
            .map_err(conflict_error)?
    };
    let run_op_id = prepared.op.id;
    if let Some(command) = prepared.command {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let execution = loom_core::WorkspaceKernel::execute_agent_command_streaming(command, tx);
        tokio::pin!(execution);
        loop {
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
                    kernel
                        .complete_agent_run(final_op.clone())
                        .map_err(internal_error)?;
                    break;
                }
            }
        }
    }
    let mut kernel = state.kernel.lock().await;
    let (session, ok, wrote_files) = kernel
        .finalize_realize(session_id, run_op_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(RealizeResponse {
        agent_id,
        ok,
        wrote_files,
        session,
    }))
}

async fn realize_quorum(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<RealizeQuorumRequest>,
) -> Result<Json<RealizeQuorumRound>, (StatusCode, String)> {
    let agent_ids = if request.agent_ids.is_empty() {
        vec!["claude-code".to_string(), "openai-codex".to_string()]
    } else {
        request.agent_ids
    };
    let mut kernel = state.kernel.lock().await;
    let round = kernel
        .run_realize_quorum(session_id, &agent_ids, request.timeout_seconds)
        .await
        .map_err(internal_error)?;
    Ok(Json(round))
}

async fn pick_realize_proposal(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<PickRealizeProposalRequest>,
) -> Result<Json<PickRealizeProposalResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let response = kernel
        .pick_latest_quorum_proposal(session_id, request.proposal_index)
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn start_autopilot(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    request: Option<Json<AutopilotStartRequest>>,
) -> Result<Json<AutopilotResponse>, (StatusCode, String)> {
    let policy = request
        .and_then(|Json(body)| body.policy)
        .unwrap_or_default();
    let mut kernel = state.kernel.lock().await;
    let response = kernel
        .run_autopilot(session_id, policy)
        .await
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn pause_autopilot(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<AutopilotResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let response = kernel
        .pause_autopilot(session_id, AutopilotPolicy::default())
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn ladder_state(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<LadderState>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let state = kernel.ladder_state(session_id).map_err(conflict_error)?;
    Ok(Json(state))
}

async fn climb_ladder(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<ClimbRungRequest>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .climb_rung(session_id, &request.to_rung)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn submit_release(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    request: Option<Json<ReleaseRequest>>,
) -> Result<Json<ReleaseStatus>, (StatusCode, String)> {
    let body = request.map(|Json(body)| body).unwrap_or(ReleaseRequest {
        schema_version: "x07.studio.release_request@0.1.0".to_string(),
        rung: "shareable".to_string(),
        environment: "shareable".to_string(),
        binding_refs: Vec::new(),
    });
    let mut kernel = state.kernel.lock().await;
    let status = kernel
        .release_for_rung(session_id, body)
        .await
        .map_err(internal_error)?;
    Ok(Json(status))
}

async fn get_release_status(
    Path((session_id, release_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
) -> Result<Json<ReleaseStatus>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let status = kernel
        .release_status(session_id, &release_id)
        .map_err(conflict_error)?;
    Ok(Json(status))
}

async fn scan_incidents(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<loom_types::artifacts::OpRecord>>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let ops = kernel
        .ingest_incidents(session_id)
        .map_err(conflict_error)?;
    Ok(Json(ops))
}

async fn watch_incidents(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<loom_types::artifacts::OpRecord>>, (StatusCode, String)> {
    let initial = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .ingest_incidents(session_id)
            .map_err(conflict_error)?
    };
    spawn_incident_watch(state, session_id);
    Ok(Json(initial))
}

fn spawn_incident_watch(state: ApiState, session_id: Uuid) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
        for _ in 0..240 {
            interval.tick().await;
            let mut kernel = state.kernel.lock().await;
            if kernel.get_session(session_id).is_none() {
                break;
            }
            if let Err(error) = kernel.ingest_incidents(session_id) {
                tracing::warn!(%session_id, %error, "incident watcher stopped");
                break;
            }
        }
    });
}

async fn repair_incident(
    Path((session_id, incident_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .repair_incident(session_id, &incident_id)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn cassette_entries(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<Vec<CassetteEntry>>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let entries = kernel
        .cassette_entries(session_id)
        .map_err(conflict_error)?;
    Ok(Json(entries))
}

async fn branch_cassette(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<CassetteBranchRequest>,
) -> Result<Json<Uuid>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let id = kernel
        .branch_from_cassette(session_id, request.from_entry, &request.new_title)
        .map_err(conflict_error)?;
    Ok(Json(id))
}

async fn ask_project(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<loom_types::api::AskRequest>,
) -> Result<Json<loom_types::api::AskAnswer>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let answer = kernel
        .ask_project(session_id, request)
        .map_err(conflict_error)?;
    Ok(Json(answer))
}

async fn visual_streampipe_parse(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualParseRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_parse(session_id, state, "streampipe", request).await
}

async fn visual_streampipe_emit(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualEmitRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_emit(session_id, state, "streampipe", request).await
}

async fn visual_statemachine_parse(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualParseRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_parse(session_id, state, "statemachine", request).await
}

async fn visual_statemachine_emit(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualEmitRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_emit(session_id, state, "statemachine", request).await
}

async fn visual_tasks_parse(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualParseRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_parse(session_id, state, "tasks", request).await
}

async fn visual_tasks_emit(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    Json(request): Json<VisualEmitRequest>,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    visual_emit(session_id, state, "tasks", request).await
}

async fn visual_parse(
    session_id: Uuid,
    state: ApiState,
    kind: &str,
    request: VisualParseRequest,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    kernel.get_session(session_id).ok_or_else(not_found)?;
    Ok(Json(
        kernel
            .visual_parse(kind, request.source)
            .map_err(conflict_error)?,
    ))
}

async fn visual_emit(
    session_id: Uuid,
    state: ApiState,
    kind: &str,
    request: VisualEmitRequest,
) -> Result<Json<VisualResponse>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    kernel.get_session(session_id).ok_or_else(not_found)?;
    Ok(Json(
        kernel
            .visual_emit(kind, request.graph)
            .map_err(conflict_error)?,
    ))
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
    request: Option<Json<RunXtalWorkflowRequest>>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let vars = request.map(|Json(body)| body.vars).unwrap_or_default();
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .run_xtal_workflow_with_vars(session_id, &vars)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn run_build_pipeline(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
    request: Option<Json<RunBuildRequest>>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let body = request.map(|Json(body)| body).unwrap_or(RunBuildRequest {
        vars: Default::default(),
        max_repair_rounds: None,
    });
    let max_repair_rounds = body.max_repair_rounds.unwrap_or(3);
    let mut kernel = state.kernel.lock().await;
    let snapshot = kernel
        .run_build_pipeline(session_id, &body.vars, max_repair_rounds)
        .await
        .map_err(internal_error)?;
    Ok(Json(snapshot))
}

async fn mint_sync_code(
    State(state): State<ApiState>,
) -> Result<Json<SyncCode>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let session_id = kernel
        .list_sessions()
        .first()
        .map(|session| session.session_id)
        .ok_or_else(not_found)?;
    let code = kernel.mint_sync_code(session_id).map_err(conflict_error)?;
    Ok(Json(code))
}

async fn claim_sync_code(
    Path(code): Path<String>,
    State(state): State<ApiState>,
) -> Result<Json<SyncClaimResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let (session, state_blob) = kernel.claim_sync_code(&code).map_err(conflict_error)?;
    Ok(Json(SyncClaimResponse {
        session,
        state_blob,
    }))
}

async fn save_sync_state(
    Path(code): Path<String>,
    State(state): State<ApiState>,
    Json(request): Json<SyncStateRequest>,
) -> Result<Json<SyncCode>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let code = kernel
        .save_sync_state(&code, request.state_blob)
        .map_err(conflict_error)?;
    Ok(Json(code))
}

async fn export_replay(
    Path(session_id): Path<Uuid>,
    State(state): State<ApiState>,
) -> Result<Json<ReplayExportResponse>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let response = kernel
        .export_replay_capsule(session_id)
        .map_err(internal_error)?;
    Ok(Json(response))
}

async fn import_replay(
    State(state): State<ApiState>,
    Json(request): Json<ReplayImportRequest>,
) -> Result<Json<SessionSnapshot>, (StatusCode, String)> {
    let mut kernel = state.kernel.lock().await;
    let session = kernel
        .import_replay_capsule(request.capsule)
        .map_err(internal_error)?;
    Ok(Json(session))
}

async fn load_memory(
    State(state): State<ApiState>,
) -> Result<Json<StudioMemory>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let memory = kernel.load_memory().map_err(internal_error)?;
    Ok(Json(memory))
}

async fn save_memory(
    State(state): State<ApiState>,
    Json(patch): Json<Value>,
) -> Result<Json<StudioMemory>, (StatusCode, String)> {
    let kernel = state.kernel.lock().await;
    let mut value = serde_json::to_value(kernel.load_memory().map_err(internal_error)?)
        .map_err(internal_error)?;
    merge_json(&mut value, patch);
    let memory: StudioMemory = serde_json::from_value(value).map_err(|error| {
        (
            StatusCode::BAD_REQUEST,
            format!("invalid memory patch: {error}"),
        )
    })?;
    let memory = kernel.save_memory(&memory).map_err(internal_error)?;
    Ok(Json(memory))
}

fn merge_json(target: &mut Value, patch: Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                match value {
                    Value::Null => {
                        target.remove(&key);
                    }
                    value => merge_json(target.entry(key).or_insert(Value::Null), value),
                }
            }
        }
        (target, patch) => *target = patch,
    }
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
    let genpack_seed = {
        let kernel = state.kernel.lock().await;
        kernel
            .genpack_context_seed(session_id)
            .map_err(internal_error)?
    };
    let genpack = WorkspaceKernel::resolve_genpack_context(genpack_seed).await;
    let mut kernel = state.kernel.lock().await;
    let (handoff, session) = kernel
        .create_agent_handoff_with_genpack(session_id, &agent_id, genpack.as_ref())
        .map_err(internal_error)?;
    Ok(Json(AgentHandoffResponse { handoff, session }))
}

async fn run_agent_handoff(
    Path((session_id, agent_id)): Path<(Uuid, String)>,
    State(state): State<ApiState>,
    Json(request): Json<AgentRunRequest>,
) -> Result<Json<AgentRunResponse>, (StatusCode, String)> {
    let genpack_seed = {
        let kernel = state.kernel.lock().await;
        kernel
            .genpack_context_seed(session_id)
            .map_err(internal_error)?
    };
    let genpack = WorkspaceKernel::resolve_genpack_context(genpack_seed).await;
    let prepared = {
        let mut kernel = state.kernel.lock().await;
        kernel
            .start_agent_handoff_with_genpack(
                session_id,
                &agent_id,
                request.mode,
                request.timeout_seconds,
                genpack.as_ref(),
            )
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
    use super::{env_setting, runtime_components, sibling_component_source};

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

    #[test]
    fn runtime_components_discover_bundled_components() {
        let root = temp_root();
        let component = root.join("components/x07-wasm");
        std::fs::create_dir_all(component.parent().expect("component parent"))
            .expect("create component dir");
        std::fs::write(&component, "").expect("write component");

        let components = runtime_components(root.as_path());
        let wasm = components
            .iter()
            .find(|component| component.id == "x07-wasm")
            .expect("x07-wasm component");

        assert_eq!(
            wasm.status,
            loom_types::api::RuntimeComponentState::Available
        );
        assert_eq!(wasm.source.as_deref(), Some(component.as_str()));

        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn env_setting_uses_runtime_override_and_ignores_empty_values() {
        let key = format!("X07_STUDIO_TEST_{}", uuid::Uuid::new_v4().simple());
        assert_eq!(env_setting(&key, "fallback"), "fallback");

        std::env::set_var(&key, "127.0.0.1:8123");
        assert_eq!(env_setting(&key, "fallback"), "127.0.0.1:8123");

        std::env::set_var(&key, " ");
        assert_eq!(env_setting(&key, "fallback"), "fallback");
        std::env::remove_var(key);
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
