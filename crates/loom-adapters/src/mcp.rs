use std::env;

use anyhow::{anyhow, Context};
use async_trait::async_trait;
use reqwest::header;
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};

use loom_types::mcp::{
    McpConnectionInfo, McpEndpoint, McpHttpEndpoint, McpStdioEndpoint, McpToolCallResult,
    McpToolDescriptor,
};

const MCP_PROTOCOL_VERSION: &str = "2025-11-25";

#[async_trait]
pub trait McpClient: Send {
    async fn initialize(&mut self) -> anyhow::Result<McpConnectionInfo>;
    async fn list_tools(&mut self) -> anyhow::Result<Vec<McpToolDescriptor>>;
    async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpToolCallResult>;
    async fn close(&mut self) -> anyhow::Result<()>;
}

pub fn boxed_client(endpoint: McpEndpoint, connection_id: String) -> Box<dyn McpClient> {
    match endpoint {
        McpEndpoint::Http(config) => Box::new(HttpMcpClient::new(connection_id, config)),
        McpEndpoint::Stdio(config) => Box::new(StdioMcpClient::new(connection_id, config)),
    }
}

pub struct HttpMcpClient {
    connection_id: String,
    config: McpHttpEndpoint,
    client: reqwest::Client,
    next_id: u64,
    initialized: bool,
    session_id: Option<String>,
    server_name: Option<String>,
    server_version: Option<String>,
}

impl HttpMcpClient {
    pub fn new(connection_id: String, config: McpHttpEndpoint) -> Self {
        Self {
            connection_id,
            config,
            client: reqwest::Client::builder()
                .user_agent("x07-studio/0.1")
                .timeout(std::time::Duration::from_secs(15))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            next_id: 1,
            initialized: false,
            session_id: None,
            server_name: None,
            server_version: None,
        }
    }

    fn endpoint_url(&self) -> String {
        format!(
            "{}{}",
            self.config.base_url.trim_end_matches('/'),
            if self.config.mcp_path.starts_with('/') {
                self.config.mcp_path.clone()
            } else {
                format!("/{}", self.config.mcp_path)
            }
        )
    }

    fn auth_header(&self) -> Option<String> {
        if let Some(token) = &self.config.bearer_token {
            if !token.trim().is_empty() {
                return Some(format!("Bearer {}", token.trim()));
            }
        }
        if let Some(env_name) = &self.config.bearer_env {
            if let Ok(value) = env::var(env_name) {
                if !value.trim().is_empty() {
                    return Some(format!("Bearer {}", value.trim()));
                }
            }
        }
        None
    }

    fn base_request(&self) -> reqwest::RequestBuilder {
        let mut request = self
            .client
            .post(self.endpoint_url())
            .header(header::ACCEPT, "application/json, text/event-stream")
            .header(header::CONTENT_TYPE, "application/json")
            .header("MCP-Protocol-Version", MCP_PROTOCOL_VERSION);

        if let Some(session_id) = &self.session_id {
            request = request.header("MCP-Session-Id", session_id);
        }
        if let Some(value) = self.auth_header() {
            request = request.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &self.config.default_headers {
            request = request.header(key.as_str(), value.as_str());
        }
        request
    }

    async fn send_request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        if !self.initialized {
            let _ = self.initialize().await?;
        }

        let id = self.next_id;
        self.next_id += 1;
        let body = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });

        let response = self.base_request().json(&body).send().await?;
        let status = response.status();
        let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
        if !status.is_success() {
            return Err(anyhow!(
                "MCP HTTP request `{method}` failed with HTTP {status}: {payload}"
            ));
        }
        if let Some(error) = payload.get("error") {
            return Err(anyhow!("MCP error response for `{method}`: {error}"));
        }
        Ok(payload)
    }

    async fn send_notification(&self, method: &str, params: Value) -> anyhow::Result<()> {
        let body = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
        });

        let response = self.base_request().json(&body).send().await?;
        let status = response.status();
        if !status.is_success() {
            let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));
            return Err(anyhow!(
                "MCP HTTP notification `{method}` failed with HTTP {status}: {payload}"
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl McpClient for HttpMcpClient {
    async fn initialize(&mut self) -> anyhow::Result<McpConnectionInfo> {
        if self.initialized {
            return Ok(McpConnectionInfo {
                connection_id: self.connection_id.clone(),
                label: self.config.label.clone(),
                transport: "http".to_string(),
                protocol_version: MCP_PROTOCOL_VERSION.to_string(),
                session_id: self.session_id.clone(),
                server_name: self.server_name.clone(),
                server_version: self.server_version.clone(),
            });
        }

        let body = json!({
            "jsonrpc": "2.0",
            "id": self.next_id,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {}
            }
        });
        self.next_id += 1;

        let response = self.base_request().json(&body).send().await?;
        let status = response.status();
        let session_id = response
            .headers()
            .get("MCP-Session-Id")
            .and_then(|value| value.to_str().ok())
            .map(str::to_string);
        let payload: Value = response.json().await.unwrap_or_else(|_| json!({}));

        if !status.is_success() {
            return Err(anyhow!(
                "MCP initialize failed with HTTP {status}: {payload}"
            ));
        }
        if let Some(error) = payload.get("error") {
            return Err(anyhow!("MCP initialize error: {error}"));
        }

        self.session_id = session_id;
        self.server_name = payload
            .pointer("/result/serverInfo/name")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.server_version = payload
            .pointer("/result/serverInfo/version")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.initialized = true;

        let _ = self
            .send_notification("notifications/initialized", json!({}))
            .await;

        Ok(McpConnectionInfo {
            connection_id: self.connection_id.clone(),
            label: self.config.label.clone(),
            transport: "http".to_string(),
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            session_id: self.session_id.clone(),
            server_name: self.server_name.clone(),
            server_version: self.server_version.clone(),
        })
    }

    async fn list_tools(&mut self) -> anyhow::Result<Vec<McpToolDescriptor>> {
        let payload = self.send_request("tools/list", json!({})).await?;
        parse_tools(
            payload
                .pointer("/result/tools")
                .cloned()
                .unwrap_or_else(|| json!([])),
        )
    }

    async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpToolCallResult> {
        let payload = self
            .send_request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        Ok(McpToolCallResult {
            structured_content: payload.pointer("/result/structuredContent").cloned(),
            content: payload.pointer("/result/content").cloned(),
            is_error: payload
                .pointer("/result/isError")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            raw: payload,
        })
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

pub struct StdioMcpClient {
    connection_id: String,
    config: McpStdioEndpoint,
    next_id: u64,
    initialized: bool,
    child: Option<Child>,
    stdin: Option<ChildStdin>,
    stdout: Option<Lines<BufReader<ChildStdout>>>,
    server_name: Option<String>,
    server_version: Option<String>,
}

impl StdioMcpClient {
    pub fn new(connection_id: String, config: McpStdioEndpoint) -> Self {
        Self {
            connection_id,
            config,
            next_id: 1,
            initialized: false,
            child: None,
            stdin: None,
            stdout: None,
            server_name: None,
            server_version: None,
        }
    }

    async fn ensure_spawned(&mut self) -> anyhow::Result<()> {
        if self.child.is_some() {
            return Ok(());
        }

        let mut command = Command::new(&self.config.command);
        command.args(&self.config.args);
        if let Some(cwd) = &self.config.cwd {
            command.current_dir(cwd);
        }
        command.stdin(std::process::Stdio::piped());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::inherit());
        for (key, value) in &self.config.env {
            command.env(key, value);
        }

        let mut child = command.spawn().with_context(|| {
            format!(
                "failed to spawn MCP stdio process `{}`",
                self.config.command
            )
        })?;
        let stdin = child.stdin.take().context("missing MCP child stdin")?;
        let stdout = child.stdout.take().context("missing MCP child stdout")?;
        self.stdin = Some(stdin);
        self.stdout = Some(BufReader::new(stdout).lines());
        self.child = Some(child);
        Ok(())
    }

    async fn send_line(&mut self, value: &Value) -> anyhow::Result<()> {
        self.ensure_spawned().await?;
        let line = serde_json::to_vec(value)?;
        let stdin = self.stdin.as_mut().context("stdio client missing stdin")?;
        stdin.write_all(&line).await?;
        stdin.write_all(b"\n").await?;
        stdin.flush().await?;
        Ok(())
    }

    async fn read_response_for_id(&mut self, id: u64) -> anyhow::Result<Value> {
        self.ensure_spawned().await?;
        let stdout = self
            .stdout
            .as_mut()
            .context("stdio client missing stdout")?;
        while let Some(line) = stdout.next_line().await? {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let payload: Value = serde_json::from_str(trimmed)
                .with_context(|| format!("invalid stdio MCP JSON line: {trimmed}"))?;
            if payload.get("id").and_then(|value| value.as_u64()) == Some(id) {
                if let Some(error) = payload.get("error") {
                    return Err(anyhow!("MCP stdio error response: {error}"));
                }
                return Ok(payload);
            }
        }
        Err(anyhow!("stdio MCP stream ended before response {id}"))
    }

    async fn request(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        if !self.initialized {
            let _ = self.initialize().await?;
        }
        let id = self.next_id;
        self.next_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        });
        self.send_line(&request).await?;
        self.read_response_for_id(id).await
    }
}

#[async_trait]
impl McpClient for StdioMcpClient {
    async fn initialize(&mut self) -> anyhow::Result<McpConnectionInfo> {
        if self.initialized {
            return Ok(McpConnectionInfo {
                connection_id: self.connection_id.clone(),
                label: self.config.label.clone(),
                transport: "stdio".to_string(),
                protocol_version: MCP_PROTOCOL_VERSION.to_string(),
                session_id: None,
                server_name: self.server_name.clone(),
                server_version: self.server_version.clone(),
            });
        }

        let id = self.next_id;
        self.next_id += 1;
        self.send_line(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": {
                "protocolVersion": MCP_PROTOCOL_VERSION,
                "capabilities": {}
            }
        }))
        .await?;
        let payload = self.read_response_for_id(id).await?;
        self.server_name = payload
            .pointer("/result/serverInfo/name")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.server_version = payload
            .pointer("/result/serverInfo/version")
            .and_then(|value| value.as_str())
            .map(str::to_string);
        self.initialized = true;

        self.send_line(&json!({
            "jsonrpc": "2.0",
            "method": "notifications/initialized",
            "params": {}
        }))
        .await?;

        Ok(McpConnectionInfo {
            connection_id: self.connection_id.clone(),
            label: self.config.label.clone(),
            transport: "stdio".to_string(),
            protocol_version: MCP_PROTOCOL_VERSION.to_string(),
            session_id: None,
            server_name: self.server_name.clone(),
            server_version: self.server_version.clone(),
        })
    }

    async fn list_tools(&mut self) -> anyhow::Result<Vec<McpToolDescriptor>> {
        let payload = self.request("tools/list", json!({})).await?;
        parse_tools(
            payload
                .pointer("/result/tools")
                .cloned()
                .unwrap_or_else(|| json!([])),
        )
    }

    async fn call_tool(
        &mut self,
        name: &str,
        arguments: Value,
    ) -> anyhow::Result<McpToolCallResult> {
        let payload = self
            .request(
                "tools/call",
                json!({ "name": name, "arguments": arguments }),
            )
            .await?;
        Ok(McpToolCallResult {
            structured_content: payload.pointer("/result/structuredContent").cloned(),
            content: payload.pointer("/result/content").cloned(),
            is_error: payload
                .pointer("/result/isError")
                .and_then(|value| value.as_bool())
                .unwrap_or(false),
            raw: payload,
        })
    }

    async fn close(&mut self) -> anyhow::Result<()> {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill().await;
        }
        Ok(())
    }
}

fn parse_tools(value: Value) -> anyhow::Result<Vec<McpToolDescriptor>> {
    let mut out = Vec::new();
    let array = value
        .as_array()
        .ok_or_else(|| anyhow!("MCP tools/list result did not contain an array"))?;
    for item in array {
        out.push(McpToolDescriptor {
            name: item
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or_default()
                .to_string(),
            title: item
                .get("title")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            description: item
                .get("description")
                .and_then(|value| value.as_str())
                .map(str::to_string),
            input_schema: item.get("inputSchema").cloned(),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::parse_tools;

    #[test]
    fn parse_tools_preserves_names_and_input_schema() {
        let tools = parse_tools(json!([
            {
                "name": "x07.search_v1",
                "title": "Search",
                "description": "Search x07 docs",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "query": { "type": "string" }
                    }
                }
            }
        ]))
        .expect("tools parse");

        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "x07.search_v1");
        assert_eq!(tools[0].title.as_deref(), Some("Search"));
        assert_eq!(
            tools[0]
                .input_schema
                .as_ref()
                .and_then(|schema| schema.pointer("/properties/query/type")),
            Some(&json!("string"))
        );
    }

    #[test]
    fn parse_tools_rejects_non_array_payload() {
        let error = parse_tools(json!({ "tools": [] })).expect_err("non-array tools must fail");

        assert!(error
            .to_string()
            .contains("MCP tools/list result did not contain an array"));
    }
}
