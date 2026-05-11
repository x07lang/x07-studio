use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Path, State};
use axum::routing::{delete, get, post};
use axum::{http::StatusCode, Json, Router};
use tokio::sync::Mutex;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use loom_core::WorkspaceKernel;
use loom_types::api::{
    AgentHandoffResponse, BindingDescriptor, CallMcpToolRequest, ConnectMcpRequest,
    ConnectMcpResponse, CreateSessionRequest, DispatchEventRequest, HealthResponse,
    McpCallResponse, ProbeProviderRequest, ProviderProbeResponse, RunBindingRequest,
    SaveAgentProfileRequest, SaveProviderProfileRequest,
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
        .route("/v1/bindings", get(bindings))
        .route("/v1/sessions", get(list_sessions).post(create_session))
        .route("/v1/sessions/{session_id}", get(get_session))
        .route("/v1/sessions/{session_id}/events", post(dispatch_event))
        .route("/v1/sessions/{session_id}/bindings/run", post(run_binding))
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
    axum::serve(listener, router(state)).await?;
    Ok(())
}

pub fn default_state(root: impl Into<camino::Utf8PathBuf>) -> anyhow::Result<ApiState> {
    Ok(ApiState {
        kernel: Arc::new(Mutex::new(WorkspaceKernel::open(root)?)),
    })
}

async fn health(State(state): State<ApiState>) -> Json<HealthResponse> {
    let kernel = state.kernel.lock().await;
    Json(HealthResponse {
        ok: true,
        workspace_root: kernel.workspace_root().to_string(),
    })
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
