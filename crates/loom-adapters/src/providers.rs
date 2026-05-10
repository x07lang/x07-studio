use std::env;

use reqwest::{header, Client};
use serde_json::{json, Value};

use loom_types::artifacts::{
    ProbeStatus, ProviderCapabilities, ProviderProbeMode, ProviderProbeReport, ProviderProfile,
};

use crate::command_runner::now_string;

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

    use super::ProviderProber;

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
}
