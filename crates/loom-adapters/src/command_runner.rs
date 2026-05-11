use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use tokio::process::Command;

#[derive(Debug, Clone)]
pub struct CommandExecution {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Utf8PathBuf,
    pub started_at: String,
    pub finished_at: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub stdout_json: Option<Value>,
    pub stderr_json: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct CommandRunner;

impl CommandRunner {
    pub async fn run(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_timeout(cwd, program, args, envs, None).await
    }

    pub async fn run_with_timeout(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<CommandExecution> {
        let started_at = now_string();
        let mut command = Command::new(program);
        command.current_dir(cwd.as_std_path());
        command.args(args);
        command.stdin(Stdio::null());
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);
        for (key, value) in envs {
            command.env(key, value);
        }

        let output = if let Some(seconds) = timeout_seconds {
            match tokio::time::timeout(Duration::from_secs(seconds.max(1)), command.output()).await
            {
                Ok(output) => output.with_context(|| format!("failed to spawn `{program}`"))?,
                Err(_) => {
                    let stderr = format!("command timed out after {} seconds", seconds.max(1));
                    return Ok(CommandExecution {
                        program: program.to_string(),
                        args: args.to_vec(),
                        cwd: cwd.to_owned(),
                        started_at,
                        finished_at: now_string(),
                        exit_code: None,
                        stdout_json: None,
                        stderr_json: parse_json(&stderr),
                        stdout: String::new(),
                        stderr,
                    });
                }
            }
        } else {
            command
                .output()
                .await
                .with_context(|| format!("failed to spawn `{program}`"))?
        };

        let finished_at = now_string();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandExecution {
            program: program.to_string(),
            args: args.to_vec(),
            cwd: cwd.to_owned(),
            started_at,
            finished_at,
            exit_code: output.status.code(),
            stdout_json: parse_json(&stdout),
            stderr_json: parse_json(&stderr),
            stdout,
            stderr,
        })
    }
}

pub fn now_string() -> String {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .to_string()
}

fn parse_json(input: &str) -> Option<Value> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    serde_json::from_str(trimmed).ok()
}
