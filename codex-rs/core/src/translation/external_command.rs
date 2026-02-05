use std::process::Stdio;

use tokio::io::AsyncRead;
use tokio::io::AsyncReadExt;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::config::types::AgentReasoningTranslationConfig;
use crate::translation::TranslationError;
use crate::translation::preview_bytes;

const MAX_TRANSLATION_STDOUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_TRANSLATION_STDERR_BYTES: usize = 1024 * 1024;

pub(crate) struct CommandOutput {
    pub(crate) stdout: Vec<u8>,
}

async fn read_to_end_limited<R: AsyncRead + Unpin>(
    mut reader: R,
    stream: &'static str,
    limit_bytes: usize,
) -> Result<Vec<u8>, TranslationError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader
            .read(&mut chunk)
            .await
            .map_err(TranslationError::ReadOutput)?;
        if n == 0 {
            break;
        }

        if buf.len().saturating_add(n) > limit_bytes {
            return Err(TranslationError::OutputTooLarge {
                stream,
                limit_bytes,
            });
        }
        buf.extend_from_slice(&chunk[..n]);
    }

    Ok(buf)
}

pub(crate) async fn run_translation_command(
    config: &AgentReasoningTranslationConfig,
    request_json: Vec<u8>,
) -> Result<CommandOutput, TranslationError> {
    let program = config
        .command
        .first()
        .ok_or(TranslationError::EmptyCommand)?;

    let mut command = Command::new(program);
    if config.command.len() > 1 {
        command.args(&config.command[1..]);
    }
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let mut child = command.spawn().map_err(TranslationError::Spawn)?;
    let timeout = config.timeout;
    let deadline = tokio::time::Instant::now() + timeout;

    let mut stdin = child.stdin.take().ok_or_else(|| {
        TranslationError::WriteStdin(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "stdin pipe not available",
        ))
    })?;

    match tokio::time::timeout_at(deadline, stdin.write_all(&request_json)).await {
        Ok(result) => result.map_err(TranslationError::WriteStdin)?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(TranslationError::Timeout {
                timeout_ms: timeout.as_millis(),
            });
        }
    }
    match tokio::time::timeout_at(deadline, stdin.write_all(b"\n")).await {
        Ok(result) => result.map_err(TranslationError::WriteStdin)?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            return Err(TranslationError::Timeout {
                timeout_ms: timeout.as_millis(),
            });
        }
    }
    drop(stdin);

    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| TranslationError::ReadOutput(std::io::ErrorKind::BrokenPipe.into()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| TranslationError::ReadOutput(std::io::ErrorKind::BrokenPipe.into()))?;

    let stdout_task = tokio::spawn(read_to_end_limited(
        stdout,
        "stdout",
        MAX_TRANSLATION_STDOUT_BYTES,
    ));
    let stderr_task = tokio::spawn(read_to_end_limited(
        stderr,
        "stderr",
        MAX_TRANSLATION_STDERR_BYTES,
    ));

    let status = match tokio::time::timeout_at(deadline, child.wait()).await {
        Ok(status) => status.map_err(TranslationError::ReadOutput)?,
        Err(_) => {
            let _ = child.kill().await;
            let _ = child.wait().await;
            stdout_task.abort();
            stderr_task.abort();
            return Err(TranslationError::Timeout {
                timeout_ms: timeout.as_millis(),
            });
        }
    };

    let stdout = match tokio::time::timeout_at(deadline, stdout_task).await {
        Ok(joined) => match joined {
            Ok(result) => match result {
                Ok(stdout) => stdout,
                Err(err) => {
                    stderr_task.abort();
                    return Err(err);
                }
            },
            Err(join_err) => {
                stderr_task.abort();
                return Err(TranslationError::ReadOutput(std::io::Error::other(
                    join_err.to_string(),
                )));
            }
        },
        Err(_) => {
            stderr_task.abort();
            return Err(TranslationError::Timeout {
                timeout_ms: timeout.as_millis(),
            });
        }
    };

    let stderr = match tokio::time::timeout_at(deadline, stderr_task).await {
        Ok(joined) => match joined {
            Ok(result) => result?,
            Err(join_err) => {
                return Err(TranslationError::ReadOutput(std::io::Error::other(
                    join_err.to_string(),
                )));
            }
        },
        Err(_) => {
            return Err(TranslationError::Timeout {
                timeout_ms: timeout.as_millis(),
            });
        }
    };

    if !status.success() {
        return Err(TranslationError::NonZeroExit {
            code: status.code(),
            stderr_preview: preview_bytes(&stderr),
            stdout_preview: preview_bytes(&stdout),
        });
    }

    Ok(CommandOutput { stdout })
}
