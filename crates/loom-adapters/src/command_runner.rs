use std::collections::BTreeMap;
use std::process::Stdio;
use std::time::Duration;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use camino::{Utf8Path, Utf8PathBuf};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::mpsc;

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

#[derive(Debug, Clone)]
pub struct CommandStreamUpdate {
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Default)]
pub struct CommandRunner;

#[derive(Debug)]
struct RunOptions<'a> {
    timeout_seconds: Option<u64>,
    stdin: Option<&'a [u8]>,
    updates: Option<mpsc::UnboundedSender<CommandStreamUpdate>>,
}

impl CommandRunner {
    pub async fn run(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_options(
            cwd,
            program,
            args,
            envs,
            RunOptions {
                timeout_seconds: None,
                stdin: None,
                updates: None,
            },
        )
        .await
    }

    pub async fn run_with_stdin(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        stdin: &str,
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_options(
            cwd,
            program,
            args,
            envs,
            RunOptions {
                timeout_seconds: None,
                stdin: Some(stdin.as_bytes()),
                updates: None,
            },
        )
        .await
    }

    pub async fn run_with_stdin_bytes(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        stdin: &[u8],
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_options(
            cwd,
            program,
            args,
            envs,
            RunOptions {
                timeout_seconds: None,
                stdin: Some(stdin),
                updates: None,
            },
        )
        .await
    }

    pub async fn run_with_timeout(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        timeout_seconds: Option<u64>,
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_options(
            cwd,
            program,
            args,
            envs,
            RunOptions {
                timeout_seconds,
                stdin: None,
                updates: None,
            },
        )
        .await
    }

    pub async fn run_with_timeout_streaming(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        timeout_seconds: Option<u64>,
        updates: mpsc::UnboundedSender<CommandStreamUpdate>,
    ) -> anyhow::Result<CommandExecution> {
        self.run_with_options(
            cwd,
            program,
            args,
            envs,
            RunOptions {
                timeout_seconds,
                stdin: None,
                updates: Some(updates),
            },
        )
        .await
    }

    async fn run_with_options(
        &self,
        cwd: &Utf8Path,
        program: &str,
        args: &[String],
        envs: &BTreeMap<String, String>,
        options: RunOptions<'_>,
    ) -> anyhow::Result<CommandExecution> {
        let started_at = now_string();
        let mut command = Command::new(program);
        command.current_dir(cwd.as_std_path());
        command.args(args);
        command.stdin(if options.stdin.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        });
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        command.kill_on_drop(true);
        for (key, value) in envs {
            command.env(key, value);
        }

        let timeout_seconds = options.timeout_seconds;
        let mut child = command
            .spawn()
            .with_context(|| format!("failed to spawn `{program}`"))?;
        if let Some(input) = options.stdin {
            if let Some(mut child_stdin) = child.stdin.take() {
                child_stdin
                    .write_all(input)
                    .await
                    .with_context(|| format!("failed to write stdin for `{program}`"))?;
            }
        }

        if let Some(updates) = options.updates {
            let wait = wait_with_streaming_output(
                child,
                program.to_string(),
                args.to_vec(),
                cwd.to_owned(),
                started_at.clone(),
                updates,
            );
            return if let Some(seconds) = timeout_seconds {
                match tokio::time::timeout(Duration::from_secs(seconds.max(1)), wait).await {
                    Ok(result) => result,
                    Err(_) => {
                        let stderr = format!("command timed out after {} seconds", seconds.max(1));
                        Ok(CommandExecution {
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
                        })
                    }
                }
            } else {
                wait.await
            };
        }

        let output = if let Some(seconds) = timeout_seconds {
            match tokio::time::timeout(
                Duration::from_secs(seconds.max(1)),
                child.wait_with_output(),
            )
            .await
            {
                Ok(output) => output.with_context(|| format!("failed to wait for `{program}`"))?,
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
            child
                .wait_with_output()
                .await
                .with_context(|| format!("failed to wait for `{program}`"))?
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

enum StreamChunk {
    Stdout(Vec<u8>),
    Stderr(Vec<u8>),
}

async fn wait_with_streaming_output(
    mut child: tokio::process::Child,
    program: String,
    args: Vec<String>,
    cwd: Utf8PathBuf,
    started_at: String,
    updates: mpsc::UnboundedSender<CommandStreamUpdate>,
) -> anyhow::Result<CommandExecution> {
    let stdout = child.stdout.take().context("missing child stdout")?;
    let stderr = child.stderr.take().context("missing child stderr")?;
    let (chunk_tx, mut chunk_rx) = mpsc::unbounded_channel();
    let stdout_task = tokio::spawn(read_stream(stdout, true, chunk_tx.clone()));
    let stderr_task = tokio::spawn(read_stream(stderr, false, chunk_tx.clone()));
    drop(chunk_tx);

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let status = loop {
        tokio::select! {
            chunk = chunk_rx.recv() => {
                if let Some(chunk) = chunk {
                    match chunk {
                        StreamChunk::Stdout(bytes) => stdout.extend(bytes),
                        StreamChunk::Stderr(bytes) => stderr.extend(bytes),
                    }
                    let _ = updates.send(CommandStreamUpdate {
                        stdout: String::from_utf8_lossy(&stdout).to_string(),
                        stderr: String::from_utf8_lossy(&stderr).to_string(),
                    });
                }
            }
            status = child.wait() => {
                break status.with_context(|| format!("failed to wait for `{program}`"))?;
            }
        }
    };

    let _ = stdout_task.await;
    let _ = stderr_task.await;
    while let Ok(chunk) = chunk_rx.try_recv() {
        match chunk {
            StreamChunk::Stdout(bytes) => stdout.extend(bytes),
            StreamChunk::Stderr(bytes) => stderr.extend(bytes),
        }
    }
    let stdout = String::from_utf8_lossy(&stdout).to_string();
    let stderr = String::from_utf8_lossy(&stderr).to_string();

    Ok(CommandExecution {
        program,
        args,
        cwd,
        started_at,
        finished_at: now_string(),
        exit_code: status.code(),
        stdout_json: parse_json(&stdout),
        stderr_json: parse_json(&stderr),
        stdout,
        stderr,
    })
}

async fn read_stream<R>(
    mut reader: R,
    stdout: bool,
    tx: mpsc::UnboundedSender<StreamChunk>,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin,
{
    let mut buffer = [0_u8; 4096];
    loop {
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        let chunk = buffer[..read].to_vec();
        let _ = if stdout {
            tx.send(StreamChunk::Stdout(chunk))
        } else {
            tx.send(StreamChunk::Stderr(chunk))
        };
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::CommandRunner;

    #[tokio::test]
    async fn runner_can_supply_stdin() {
        let cwd = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("utf8 temp");
        let execution = CommandRunner
            .run_with_stdin(
                cwd.as_path(),
                "/bin/sh",
                &["-c".to_string(), "cat".to_string()],
                &BTreeMap::new(),
                "verify",
            )
            .await
            .expect("run stdin command");

        assert_eq!(execution.exit_code, Some(0));
        assert_eq!(execution.stdout, "verify");
    }

    #[tokio::test]
    async fn runner_streams_stdout_updates() {
        let cwd = camino::Utf8PathBuf::from_path_buf(std::env::temp_dir()).expect("utf8 temp");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let execution = CommandRunner
            .run_with_timeout_streaming(
                cwd.as_path(),
                "/bin/sh",
                &[
                    "-c".to_string(),
                    "printf first; sleep 0.05; printf second".to_string(),
                ],
                &BTreeMap::new(),
                Some(2),
                tx,
            )
            .await
            .expect("run streaming command");

        let mut updates = Vec::new();
        while let Ok(update) = rx.try_recv() {
            updates.push(update);
        }

        assert_eq!(execution.exit_code, Some(0));
        assert_eq!(execution.stdout, "firstsecond");
        assert!(updates.iter().any(|update| update.stdout.contains("first")));
    }
}
