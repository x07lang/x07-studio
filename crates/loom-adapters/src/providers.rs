use std::env;

use reqwest::{header, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use loom_types::artifacts::{
    ProbeStatus, ProviderCapabilities, ProviderProbeMode, ProviderProbeReport, ProviderProfile,
};

use crate::command_runner::now_string;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderIntentPolishRequest {
    pub raw: String,
    pub input_mode: String,
    pub revision_notes: Vec<String>,
    pub deterministic_intent: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderIntentPolishReport {
    pub schema_version: String,
    pub profile_id: String,
    pub model: String,
    pub endpoint: String,
    pub ok: bool,
    pub notes: Vec<String>,
    pub text: String,
    pub json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct ProviderProber {
    client: Client,
}

impl Default for ProviderProber {
    fn default() -> Self {
        let client = Client::builder()
            .user_agent("x07-studio/0.1")
            .timeout(std::time::Duration::from_secs(8))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { client }
    }
}

impl ProviderProber {
    pub async fn polish_intent(
        &self,
        profile: &ProviderProfile,
        request: &ProviderIntentPolishRequest,
    ) -> anyhow::Result<ProviderIntentPolishReport> {
        if profile.disabled {
            anyhow::bail!("provider profile `{}` is disabled", profile.id);
        }
        let model = profile
            .model
            .clone()
            .ok_or_else(|| anyhow::anyhow!("provider profile `{}` has no model", profile.id))?;
        let base = profile.base_url.trim_end_matches('/').to_string();
        let endpoint = format!("{base}/chat/completions");
        let auth_header = resolve_auth_header(profile);
        let prompt = build_intent_polish_prompt(request);
        let mut req = self.client.post(&endpoint).json(&json!({
            "model": model,
            "messages": [
                {
                    "role": "system",
                    "content": "You polish x07 Studio XTAL intent packets. Return JSON only. Do not generate source code. Do not approve the spec. Do not change the target module or entrypoint."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ],
            "temperature": 0.1,
            "max_tokens": 900,
            "stream": false
        }));
        if let Some(value) = &auth_header {
            req = req.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &profile.default_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        let response = req.send().await?;
        let status = response.status();
        let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
        if !status.is_success() {
            anyhow::bail!(
                "intent polish provider `{}` returned HTTP {}",
                profile.id,
                status.as_u16()
            );
        }
        let text = extract_chat_text(&body)
            .ok_or_else(|| anyhow::anyhow!("intent polish provider returned no text"))?;
        let parsed = parse_json_object_from_text(&text);
        let mut notes = vec!["provider returned intent polish suggestions".to_string()];
        if parsed.is_none() {
            notes.push(
                "provider text did not parse as JSON; suggestions were recorded only".to_string(),
            );
        }
        Ok(ProviderIntentPolishReport {
            schema_version: "x07.studio.intent_polish_report@0.1.0".to_string(),
            profile_id: profile.id.clone(),
            model,
            endpoint,
            ok: true,
            notes,
            text,
            json: parsed,
        })
    }

    pub async fn probe(&self, profile: &ProviderProfile) -> anyhow::Result<ProviderProbeReport> {
        let base = profile.base_url.trim_end_matches('/').to_string();
        let auth_header = resolve_auth_header(profile);
        let mut report = ProviderProbeReport {
            schema_version: "x07.studio.provider_probe_report@0.1.0".to_string(),
            profile_id: profile.id.clone(),
            base_url: base.clone(),
            observed_at: now_string(),
            ok: false,
            http_status: None,
            models: Vec::new(),
            capabilities: ProviderCapabilities::default(),
            notes: Vec::new(),
            raw: None,
        };

        let models_url = format!("{base}/models");
        let mut models_req = self.client.get(&models_url);
        if let Some(value) = &auth_header {
            models_req = models_req.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &profile.default_headers {
            models_req = models_req.header(key.as_str(), value.as_str());
        }

        match models_req.send().await {
            Ok(response) => {
                report.http_status = Some(response.status().as_u16());
                let status = response.status();
                let body: Value = response.json().await.unwrap_or_else(|_| json!({}));
                report.raw = Some(body.clone());
                if status.is_success() {
                    report.capabilities.models_endpoint = ProbeStatus::Supported;
                    if let Some(items) = body.get("data").and_then(|value| value.as_array()) {
                        for item in items {
                            if let Some(id) = item.get("id").and_then(|value| value.as_str()) {
                                report.models.push(id.to_string());
                            }
                        }
                    }
                    report.ok = true;
                } else {
                    report.capabilities.models_endpoint = ProbeStatus::Error;
                    report
                        .notes
                        .push(format!("GET /models returned HTTP {}", status.as_u16()));
                }
            }
            Err(error) => {
                report.capabilities.models_endpoint = ProbeStatus::Error;
                report.notes.push(format!("GET /models failed: {error}"));
            }
        }

        if matches!(profile.probe_mode, ProviderProbeMode::Deep) {
            let model = profile
                .model
                .clone()
                .or_else(|| report.models.first().cloned());
            if let Some(model) = model {
                let responses_status = self
                    .deep_probe_responses(profile, &model, auth_header.as_deref())
                    .await;
                report.capabilities.responses = responses_status.0;
                report.notes.extend(responses_status.1);

                let chat_status = self
                    .deep_probe_chat(profile, &model, auth_header.as_deref())
                    .await;
                report.capabilities.chat_completions = chat_status.0;
                report.notes.extend(chat_status.1);

                let tools_status = self
                    .deep_probe_tools(profile, &model, auth_header.as_deref())
                    .await;
                report.capabilities.tools = tools_status.0;
                report.notes.extend(tools_status.1);
            } else {
                report.notes.push(
                    "deep probe skipped because no model was configured and /models returned none"
                        .to_string(),
                );
            }
        } else {
            report
                .notes
                .push("shallow probe ran only GET /models".to_string());
        }

        Ok(report)
    }

    async fn deep_probe_responses(
        &self,
        profile: &ProviderProfile,
        model: &str,
        auth_header: Option<&str>,
    ) -> (ProbeStatus, Vec<String>) {
        let mut req = self
            .client
            .post(format!(
                "{}/responses",
                profile.base_url.trim_end_matches('/')
            ))
            .json(&json!({
                "model": model,
                "input": "ping",
                "max_output_tokens": 1,
                "stream": false
            }));
        if let Some(value) = auth_header {
            req = req.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &profile.default_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        match req.send().await {
            Ok(response) if response.status().is_success() => (
                ProbeStatus::Supported,
                vec!["/responses succeeded".to_string()],
            ),
            Ok(response)
                if response.status().as_u16() == 404 || response.status().as_u16() == 405 =>
            {
                (
                    ProbeStatus::Unsupported,
                    vec![format!(
                        "/responses unsupported: HTTP {}",
                        response.status().as_u16()
                    )],
                )
            }
            Ok(response) => (
                ProbeStatus::Error,
                vec![format!(
                    "/responses returned HTTP {}",
                    response.status().as_u16()
                )],
            ),
            Err(error) => (
                ProbeStatus::Error,
                vec![format!("/responses probe failed: {error}")],
            ),
        }
    }

    async fn deep_probe_chat(
        &self,
        profile: &ProviderProfile,
        model: &str,
        auth_header: Option<&str>,
    ) -> (ProbeStatus, Vec<String>) {
        let mut req = self
            .client
            .post(format!(
                "{}/chat/completions",
                profile.base_url.trim_end_matches('/')
            ))
            .json(&json!({
                "model": model,
                "messages": [{"role":"user","content":"ping"}],
                "max_tokens": 1,
                "stream": false
            }));
        if let Some(value) = auth_header {
            req = req.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &profile.default_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        match req.send().await {
            Ok(response) if response.status().is_success() => (
                ProbeStatus::Supported,
                vec!["/chat/completions succeeded".to_string()],
            ),
            Ok(response)
                if response.status().as_u16() == 404 || response.status().as_u16() == 405 =>
            {
                (
                    ProbeStatus::Unsupported,
                    vec![format!(
                        "/chat/completions unsupported: HTTP {}",
                        response.status().as_u16()
                    )],
                )
            }
            Ok(response) => (
                ProbeStatus::Error,
                vec![format!(
                    "/chat/completions returned HTTP {}",
                    response.status().as_u16()
                )],
            ),
            Err(error) => (
                ProbeStatus::Error,
                vec![format!("/chat/completions probe failed: {error}")],
            ),
        }
    }

    async fn deep_probe_tools(
        &self,
        profile: &ProviderProfile,
        model: &str,
        auth_header: Option<&str>,
    ) -> (ProbeStatus, Vec<String>) {
        let mut req = self
            .client
            .post(format!(
                "{}/chat/completions",
                profile.base_url.trim_end_matches('/')
            ))
            .json(&json!({
                "model": model,
                "messages": [{"role":"user","content":"ping"}],
                "tools": [{
                    "type": "function",
                    "function": {
                        "name": "noop",
                        "description": "no-op tool probe",
                        "parameters": {
                            "type": "object",
                            "properties": {}
                        }
                    }
                }],
                "tool_choice": "none",
                "max_tokens": 1,
                "stream": false
            }));
        if let Some(value) = auth_header {
            req = req.header(header::AUTHORIZATION, value);
        }
        for (key, value) in &profile.default_headers {
            req = req.header(key.as_str(), value.as_str());
        }

        match req.send().await {
            Ok(response) if response.status().is_success() => (
                ProbeStatus::Supported,
                vec!["tool field accepted by /chat/completions".to_string()],
            ),
            Ok(response)
                if response.status().as_u16() == 400 || response.status().as_u16() == 422 =>
            {
                (
                    ProbeStatus::Unsupported,
                    vec![format!(
                        "tool field rejected by /chat/completions: HTTP {}",
                        response.status().as_u16()
                    )],
                )
            }
            Ok(response)
                if response.status().as_u16() == 404 || response.status().as_u16() == 405 =>
            {
                (
                    ProbeStatus::Unsupported,
                    vec!["tool probe skipped because /chat/completions is unavailable".to_string()],
                )
            }
            Ok(response) => (
                ProbeStatus::Error,
                vec![format!(
                    "tool probe returned HTTP {}",
                    response.status().as_u16()
                )],
            ),
            Err(error) => (
                ProbeStatus::Error,
                vec![format!("tool probe failed: {error}")],
            ),
        }
    }
}

fn build_intent_polish_prompt(request: &ProviderIntentPolishRequest) -> String {
    format!(
        r#"Polish this human request into reviewable XTAL intent metadata.

Return one JSON object with optional keys:
- "examples": array of short example strings
- "constraints": array of short constraint strings
- "policy_implications": array of short policy or capability-review strings
- "ambiguities": array of questions or uncertainty strings
- "assumptions": array of explicit assumption strings
- "witnesses": array of {{"kind":"desired_behavior|forbidden_behavior|policy_requirement|incident_report","text":"..."}}

Rules:
- Do not write source code.
- Do not approve anything.
- Do not change target module, entrypoint, task type, or source.
- Keep every item concise and reviewable by a human.

Input mode: {}
Raw request:
{}

Revision notes:
{}

Deterministic baseline intent:
{}"#,
        request.input_mode,
        request.raw,
        serde_json::to_string_pretty(&request.revision_notes).unwrap_or_else(|_| "[]".to_string()),
        serde_json::to_string_pretty(&request.deterministic_intent)
            .unwrap_or_else(|_| "{}".to_string())
    )
}

fn extract_chat_text(body: &Value) -> Option<String> {
    body.get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .and_then(|message| message.get("content"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn parse_json_object_from_text(text: &str) -> Option<Value> {
    let trimmed = text.trim();
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        if value.is_object() {
            return Some(value);
        }
    }
    let unfenced = trimmed
        .strip_prefix("```json")
        .or_else(|| trimmed.strip_prefix("```"))
        .and_then(|value| value.strip_suffix("```"))
        .map(str::trim)
        .unwrap_or(trimmed);
    if let Ok(value) = serde_json::from_str::<Value>(unfenced) {
        if value.is_object() {
            return Some(value);
        }
    }
    let start = trimmed.find('{')?;
    let end = trimmed.rfind('}')?;
    if end <= start {
        return None;
    }
    serde_json::from_str::<Value>(&trimmed[start..=end])
        .ok()
        .filter(Value::is_object)
}

fn resolve_auth_header(profile: &ProviderProfile) -> Option<String> {
    if let Some(raw) = &profile.api_key {
        if !raw.trim().is_empty() {
            return Some(format!("Bearer {}", raw.trim()));
        }
    }
    if let Some(env_name) = &profile.api_key_env {
        if let Ok(value) = env::var(env_name) {
            if !value.trim().is_empty() {
                return Some(format!("Bearer {}", value.trim()));
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    use loom_types::artifacts::{ProbeStatus, ProviderProbeMode, ProviderProfile};

    use super::{ProviderIntentPolishRequest, ProviderProber};

    #[tokio::test]
    async fn shallow_probe_reads_openai_compatible_models() {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind test server");
        let addr = listener.local_addr().expect("test server address");
        let server = tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.expect("accept request");
            let mut request = [0_u8; 1024];
            let read = stream.read(&mut request).await.expect("read request");
            let request = String::from_utf8_lossy(&request[..read]);
            assert!(request.starts_with("GET /v1/models "));

            let body = r#"{"data":[{"id":"local-fast"},{"id":"local-auditor"}]}"#;
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

        let mut profile = ProviderProfile::local_ollama();
        profile.base_url = format!("http://{addr}/v1");
        profile.probe_mode = ProviderProbeMode::Shallow;

        let report = ProviderProber::default()
            .probe(&profile)
            .await
            .expect("probe");

        server.await.expect("server task");
        assert!(report.ok);
        assert_eq!(report.models, vec!["local-fast", "local-auditor"]);
        assert_eq!(report.capabilities.models_endpoint, ProbeStatus::Supported);
        assert_eq!(report.capabilities.responses, ProbeStatus::Unknown);
        assert!(report
            .notes
            .iter()
            .any(|note| note == "shallow probe ran only GET /models"));
    }

    #[tokio::test]
    async fn intent_polish_posts_chat_request_and_parses_json_suggestions() {
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
            assert!(request.contains("Polish this human request"));

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

        let mut profile = ProviderProfile::local_ollama();
        profile.id = "test-polisher".to_string();
        profile.base_url = format!("http://{addr}/v1");
        profile.model = Some("local-polisher".to_string());
        let request = ProviderIntentPolishRequest {
            raw: "Create a stable sorter.".to_string(),
            input_mode: "text".to_string(),
            revision_notes: vec!["Keep reviewable.".to_string()],
            deterministic_intent: serde_json::json!({
                "schema_version": "x07.studio.intent_packet@0.1.0"
            }),
        };

        let report = ProviderProber::default()
            .polish_intent(&profile, &request)
            .await
            .expect("polish intent");

        server.await.expect("server task");
        assert!(report.ok);
        assert_eq!(report.profile_id, "test-polisher");
        assert_eq!(report.model, "local-polisher");
        assert_eq!(
            report.endpoint,
            format!("http://{addr}/v1/chat/completions")
        );
        assert_eq!(
            report.json.as_ref().and_then(|value| {
                value
                    .get("examples")
                    .and_then(serde_json::Value::as_array)?
                    .first()?
                    .as_str()
            }),
            Some("Provider example: [1] -> [1]")
        );
    }
}
