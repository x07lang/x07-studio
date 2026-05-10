use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "transport", rename_all = "snake_case")]
pub enum McpEndpoint {
    Http(McpHttpEndpoint),
    Stdio(McpStdioEndpoint),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpHttpEndpoint {
    pub label: String,
    pub base_url: String,
    pub mcp_path: String,
    pub bearer_env: Option<String>,
    pub bearer_token: Option<String>,
    pub default_headers: BTreeMap<String, String>,
}

impl Default for McpHttpEndpoint {
    fn default() -> Self {
        Self {
            label: "x07lang-mcp-http".to_string(),
            base_url: "http://127.0.0.1:8314".to_string(),
            mcp_path: "/mcp".to_string(),
            bearer_env: None,
            bearer_token: None,
            default_headers: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpStdioEndpoint {
    pub label: String,
    pub command: String,
    pub args: Vec<String>,
    pub cwd: Option<String>,
    pub env: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpConnectionInfo {
    pub connection_id: String,
    pub label: String,
    pub transport: String,
    pub protocol_version: String,
    pub session_id: Option<String>,
    pub server_name: Option<String>,
    pub server_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolDescriptor {
    pub name: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub input_schema: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolCallResult {
    pub raw: Value,
    pub structured_content: Option<Value>,
    pub content: Option<Value>,
    pub is_error: bool,
}
