use anyhow::{anyhow, Context};
use serde_json::Value;
use uuid::Uuid;

use loom_types::api::{
    BindingDescriptor, CallMcpToolRequest, ConnectMcpRequest, ConnectMcpResponse,
    CreateSessionRequest, DispatchEventRequest, HealthResponse, McpCallResponse,
    ProbeProviderRequest, ProviderProbeResponse, RunBindingRequest, SaveProviderProfileRequest,
};
use loom_types::artifacts::ProviderProfile;
use loom_types::mcp::McpToolDescriptor;
use loom_types::session::SessionSnapshot;

#[derive(Debug, Clone)]
pub struct DaemonClient {
    base_url: String,
    client: reqwest::Client,
}

impl DaemonClient {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: base_url.into().trim_end_matches('/').to_string(),
            client: reqwest::Client::builder()
                .user_agent("x07-studio-shell/0.1")
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }

    pub async fn health(&self) -> anyhow::Result<HealthResponse> {
        self.get("/v1/health").await
    }

    pub async fn list_bindings(&self) -> anyhow::Result<Vec<BindingDescriptor>> {
        self.get("/v1/bindings").await
    }

    pub async fn list_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>> {
        self.get("/v1/sessions").await
    }

    pub async fn create_session(
        &self,
        request: &CreateSessionRequest,
    ) -> anyhow::Result<SessionSnapshot> {
        self.post("/v1/sessions", request).await
    }

    pub async fn get_session(&self, session_id: Uuid) -> anyhow::Result<SessionSnapshot> {
        self.get(&format!("/v1/sessions/{session_id}")).await
    }

    pub async fn dispatch_event(
        &self,
        session_id: Uuid,
        request: &DispatchEventRequest,
    ) -> anyhow::Result<SessionSnapshot> {
        self.post(&format!("/v1/sessions/{session_id}/events"), request)
            .await
    }

    pub async fn run_binding(
        &self,
        session_id: Uuid,
        request: &RunBindingRequest,
    ) -> anyhow::Result<SessionSnapshot> {
        self.post(&format!("/v1/sessions/{session_id}/bindings/run"), request)
            .await
    }

    pub async fn list_providers(&self) -> anyhow::Result<Vec<ProviderProfile>> {
        self.get("/v1/providers").await
    }

    pub async fn save_provider(
        &self,
        profile: &ProviderProfile,
    ) -> anyhow::Result<ProviderProfile> {
        let request = SaveProviderProfileRequest {
            profile: profile.clone(),
        };
        self.post("/v1/providers", &request).await
    }

    pub async fn probe_provider(
        &self,
        profile: &ProviderProfile,
    ) -> anyhow::Result<ProviderProbeResponse> {
        let request = ProbeProviderRequest {
            profile: profile.clone(),
        };
        self.post("/v1/providers/probe", &request).await
    }

    pub async fn connect_mcp(
        &self,
        request: &ConnectMcpRequest,
    ) -> anyhow::Result<ConnectMcpResponse> {
        self.post("/v1/mcp/connect", request).await
    }

    pub async fn list_mcp_tools(
        &self,
        connection_id: &str,
    ) -> anyhow::Result<Vec<McpToolDescriptor>> {
        self.get(&format!("/v1/mcp/{connection_id}/tools")).await
    }

    pub async fn call_mcp_tool(
        &self,
        connection_id: &str,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpCallResponse> {
        self.post(
            &format!("/v1/mcp/{connection_id}/call"),
            &CallMcpToolRequest {
                name: name.to_string(),
                arguments,
            },
        )
        .await
    }

    pub async fn close_mcp(&self, connection_id: &str) -> anyhow::Result<()> {
        let response = self
            .client
            .delete(format!("{}/v1/mcp/{}", self.base_url, connection_id))
            .send()
            .await?;
        if response.status().is_success() {
            Ok(())
        } else {
            Err(anyhow!("daemon returned HTTP {}", response.status()))
        }
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> anyhow::Result<T> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await?;
        parse_response(response).await
    }

    async fn post<B: serde::Serialize, T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> anyhow::Result<T> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await?;
        parse_response(response).await
    }
}

async fn parse_response<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> anyhow::Result<T> {
    let status = response.status();
    let text = response
        .text()
        .await
        .context("failed to read daemon response body")?;
    if !status.is_success() {
        return Err(anyhow!("daemon returned HTTP {status}: {text}"));
    }
    serde_json::from_str::<T>(&text).with_context(|| format!("invalid daemon JSON: {text}"))
}
