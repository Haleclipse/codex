//! External translation plugin (command hook).
//!
//! This module is intentionally implemented as a thin protocol wrapper that
//! executes a user-supplied external command. Codex does not embed any online
//! translation SDK to avoid privacy/compliance risk and dependency coupling.

mod external_command;

use serde::Deserialize;
use serde::Serialize;
use std::collections::HashSet;
use std::sync::Mutex;
use std::sync::OnceLock;
use toml::Value as TomlValue;

use crate::config::types::AgentReasoningTranslationConfig;
use crate::config::types::DEFAULT_AGENT_REASONING_TRANSLATION_TIMEOUT_MS;
use crate::config::types::DEFAULT_AGENT_REASONING_TRANSLATION_UI_MAX_WAIT_MS;
use crate::config::types::TranslationToml;

pub const TRANSLATION_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TranslationKind {
    AgentReasoningTitle,
    AgentReasoningBody,
}

impl TranslationKind {
    fn as_wire_value(self) -> &'static str {
        match self {
            TranslationKind::AgentReasoningTitle => "agent_reasoning_title",
            TranslationKind::AgentReasoningBody => "agent_reasoning_body",
        }
    }

    fn format(self) -> TranslationFormat {
        match self {
            TranslationKind::AgentReasoningTitle => TranslationFormat::Plain,
            TranslationKind::AgentReasoningBody => TranslationFormat::Markdown,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum TranslationFormat {
    Plain,
    Markdown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct TranslationRequest<'a> {
    schema_version: u32,
    kind: &'static str,
    format: TranslationFormat,
    source_language: &'a str,
    target_language: &'a str,
    text: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
struct TranslationResponse {
    schema_version: u32,
    text: String,
}

#[derive(Debug, thiserror::Error)]
pub enum TranslationError {
    #[error("translation command is empty")]
    EmptyCommand,

    #[error("failed to serialize translation request: {0}")]
    SerializeRequest(#[from] serde_json::Error),

    #[error("failed to spawn translation command: {0}")]
    Spawn(std::io::Error),

    #[error("failed to write translation stdin: {0}")]
    WriteStdin(std::io::Error),

    #[error("failed to read translation output: {0}")]
    ReadOutput(std::io::Error),

    #[error("translator output too large ({stream} exceeded {limit_bytes} bytes)")]
    OutputTooLarge {
        stream: &'static str,
        limit_bytes: usize,
    },

    #[error("translator timed out ({timeout_ms}ms)")]
    Timeout { timeout_ms: u128 },

    #[error(
        "translator exited non-zero (code={code:?}): stderr={stderr_preview} stdout={stdout_preview}"
    )]
    NonZeroExit {
        code: Option<i32>,
        stderr_preview: String,
        stdout_preview: String,
    },

    #[error("translator output is not valid JSON: {stdout_preview}")]
    InvalidJson { stdout_preview: String },

    #[error("translator returned schema_version mismatch: expected={expected} actual={actual}")]
    SchemaVersionMismatch { expected: u32, actual: u32 },

    #[error("translator returned an empty translation")]
    EmptyTranslation,
}

pub(crate) fn preview_bytes(bytes: &[u8]) -> String {
    const MAX_CHARS: usize = 300;
    let s = String::from_utf8_lossy(bytes);
    let trimmed = s.trim();

    let mut out = String::new();
    let mut chars = trimmed.chars();
    for _ in 0..MAX_CHARS {
        let Some(c) = chars.next() else {
            return out;
        };
        out.push(c);
    }

    if chars.next().is_some() {
        out.push('…');
    }

    out
}

pub async fn translate_text(
    config: &AgentReasoningTranslationConfig,
    kind: TranslationKind,
    text: &str,
) -> Result<String, TranslationError> {
    if config.command.is_empty() {
        return Err(TranslationError::EmptyCommand);
    }

    let request = TranslationRequest {
        schema_version: TRANSLATION_SCHEMA_VERSION,
        kind: kind.as_wire_value(),
        format: kind.format(),
        source_language: "en",
        target_language: "zh-CN",
        text,
    };

    let request_json = serde_json::to_vec(&request)?;
    let output = external_command::run_translation_command(config, request_json).await?;

    let response: TranslationResponse =
        serde_json::from_slice(&output.stdout).map_err(|_| TranslationError::InvalidJson {
            stdout_preview: preview_bytes(&output.stdout),
        })?;

    if response.schema_version != TRANSLATION_SCHEMA_VERSION {
        return Err(TranslationError::SchemaVersionMismatch {
            expected: TRANSLATION_SCHEMA_VERSION,
            actual: response.schema_version,
        });
    }

    let translated = response.text.trim().to_string();
    if translated.is_empty() {
        return Err(TranslationError::EmptyTranslation);
    }

    Ok(translated)
}

pub fn format_bilingual_title(original: &str, translated: &str) -> String {
    format!("{original}({translated})")
}

#[derive(Debug, Clone)]
struct AgentReasoningTranslationSettingsToml {
    command: Option<Vec<String>>,
    timeout_ms: Option<u64>,
    ui_max_wait_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentReasoningTranslationPluginToml {
    command: Option<Vec<String>>,
    timeout_ms: Option<u64>,
    ui_max_wait_ms: Option<u64>,
}

pub(crate) struct AgentReasoningTranslationConfigSources<'a> {
    pub active_profile_name: Option<&'a str>,
    pub global_plugins_translation: Option<&'a TomlValue>,
    pub global_legacy_translation: Option<&'a TranslationToml>,
    pub profile_plugins_translation: Option<&'a TomlValue>,
    pub profile_legacy_translation: Option<&'a TranslationToml>,
}

pub(crate) fn resolve_agent_reasoning_translation_config(
    sources: AgentReasoningTranslationConfigSources<'_>,
) -> std::io::Result<Option<AgentReasoningTranslationConfig>> {
    let global_new_present =
        plugins_translation_has_agent_reasoning(sources.global_plugins_translation);
    let global_old_present = sources
        .global_legacy_translation
        .and_then(|translation| translation.agent_reasoning.as_ref())
        .is_some();
    if global_new_present && global_old_present {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "cannot set both `[plugins.translation.agent_reasoning]` and `[translation.agent_reasoning]` in the same scope; migrate to the new path and remove the legacy section.",
        ));
    }

    let profile_name = sources.active_profile_name;
    let profile_new_present =
        plugins_translation_has_agent_reasoning(sources.profile_plugins_translation);
    let profile_old_present = sources
        .profile_legacy_translation
        .and_then(|translation| translation.agent_reasoning.as_ref())
        .is_some();
    if profile_new_present && profile_old_present {
        let Some(profile_name) = profile_name else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "cannot set both legacy and new translation configs in the same profile scope",
            ));
        };
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!(
                "cannot set both `[profiles.{profile_name}.plugins.translation.agent_reasoning]` and `[profiles.{profile_name}.translation.agent_reasoning]` in the same scope; migrate to the new path and remove the legacy section.",
            ),
        ));
    }

    let global_new = parse_agent_reasoning_translation_from_plugins_translation(
        "plugins.translation",
        sources.global_plugins_translation,
    )?;
    let global_old = sources
        .global_legacy_translation
        .and_then(|translation| translation.agent_reasoning.as_ref())
        .map(|settings| AgentReasoningTranslationSettingsToml {
            command: settings.command.clone(),
            timeout_ms: settings.timeout_ms,
            ui_max_wait_ms: settings.ui_max_wait_ms,
        });
    if global_new.is_none() && global_old.is_some() {
        warn_deprecated_translation_config_once(
            "[translation.agent_reasoning]",
            "[plugins.translation.agent_reasoning]",
        );
    }

    let profile_scope = profile_name.map(|name| format!("profiles.{name}.plugins.translation"));
    let profile_new = parse_agent_reasoning_translation_from_plugins_translation(
        profile_scope
            .as_deref()
            .unwrap_or("profiles.<unknown>.plugins.translation"),
        sources.profile_plugins_translation,
    )?;
    let profile_old = sources
        .profile_legacy_translation
        .and_then(|translation| translation.agent_reasoning.as_ref())
        .map(|settings| AgentReasoningTranslationSettingsToml {
            command: settings.command.clone(),
            timeout_ms: settings.timeout_ms,
            ui_max_wait_ms: settings.ui_max_wait_ms,
        });
    if profile_new.is_none()
        && profile_old.is_some()
        && let Some(profile_name) = profile_name
    {
        warn_deprecated_translation_config_once(
            &format!("[profiles.{profile_name}.translation.agent_reasoning]"),
            &format!("[profiles.{profile_name}.plugins.translation.agent_reasoning]"),
        );
    }

    let global = global_new.or(global_old);
    let profile = profile_new.or(profile_old);

    let command = profile
        .as_ref()
        .and_then(|settings| settings.command.clone())
        .or_else(|| {
            global
                .as_ref()
                .and_then(|settings| settings.command.clone())
        });

    let timeout_ms = profile
        .as_ref()
        .and_then(|settings| settings.timeout_ms)
        .or_else(|| global.as_ref().and_then(|settings| settings.timeout_ms))
        .unwrap_or(DEFAULT_AGENT_REASONING_TRANSLATION_TIMEOUT_MS);

    let ui_max_wait_ms = profile
        .as_ref()
        .and_then(|settings| settings.ui_max_wait_ms)
        .or_else(|| global.as_ref().and_then(|settings| settings.ui_max_wait_ms))
        .unwrap_or(DEFAULT_AGENT_REASONING_TRANSLATION_UI_MAX_WAIT_MS);

    Ok(match command {
        Some(command) if !command.is_empty() => Some(AgentReasoningTranslationConfig {
            command,
            timeout: std::time::Duration::from_millis(timeout_ms),
            ui_max_wait: std::time::Duration::from_millis(ui_max_wait_ms),
        }),
        _ => None,
    })
}

fn plugins_translation_has_agent_reasoning(plugins_translation: Option<&TomlValue>) -> bool {
    match plugins_translation {
        Some(TomlValue::Table(table)) => table.contains_key("agent_reasoning"),
        _ => false,
    }
}

fn parse_agent_reasoning_translation_from_plugins_translation(
    scope: &str,
    plugins_translation: Option<&TomlValue>,
) -> std::io::Result<Option<AgentReasoningTranslationSettingsToml>> {
    let Some(plugins_translation) = plugins_translation else {
        return Ok(None);
    };
    let TomlValue::Table(table) = plugins_translation else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("failed to parse `[{scope}]`: expected table"),
        ));
    };

    let Some(agent_reasoning) = table.get("agent_reasoning") else {
        return Ok(None);
    };

    let path = format!("[{scope}.agent_reasoning]");
    let parsed: AgentReasoningTranslationPluginToml =
        agent_reasoning.clone().try_into().map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("failed to parse `{path}`: {err}"),
            )
        })?;

    Ok(Some(AgentReasoningTranslationSettingsToml {
        command: parsed.command,
        timeout_ms: parsed.timeout_ms,
        ui_max_wait_ms: parsed.ui_max_wait_ms,
    }))
}

fn warn_deprecated_translation_config_once(old_path: &str, new_path: &str) {
    static WARNED: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    let warned = WARNED.get_or_init(|| Mutex::new(HashSet::new()));

    let mut warned = match warned.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            tracing::warn!(
                "deprecated translation config warning lock was poisoned; continuing with inner value"
            );
            poisoned.into_inner()
        }
    };
    if warned.insert(old_path.to_string()) {
        tracing::warn!(
            "detected deprecated translation config {old_path}; please migrate to {new_path}. The legacy and new configs cannot coexist in the same scope."
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io;
    use std::time::Duration;

    fn ok_command() -> Vec<String> {
        if cfg!(windows) {
            vec![
                "powershell".to_string(),
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "$null = $input; Write-Output '{\"schema_version\":1,\"text\":\"translated\"}'"
                    .to_string(),
            ]
        } else {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "cat >/dev/null; echo '{\"schema_version\":1,\"text\":\"translated\"}'".to_string(),
            ]
        }
    }

    fn fail_command() -> Vec<String> {
        if cfg!(windows) {
            vec![
                "powershell".to_string(),
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Write-Error 'boom'; exit 2".to_string(),
            ]
        } else {
            vec![
                "sh".to_string(),
                "-c".to_string(),
                "echo boom >&2; exit 2".to_string(),
            ]
        }
    }

    fn sleep_command() -> Vec<String> {
        if cfg!(windows) {
            vec![
                "powershell".to_string(),
                "-NoProfile".to_string(),
                "-Command".to_string(),
                "Start-Sleep -Seconds 5".to_string(),
            ]
        } else {
            vec!["sh".to_string(), "-c".to_string(), "sleep 5".to_string()]
        }
    }

    #[tokio::test]
    async fn translate_text_success() -> io::Result<()> {
        let config = AgentReasoningTranslationConfig {
            command: ok_command(),
            timeout: Duration::from_millis(2_000),
            ui_max_wait: Duration::from_millis(5_000),
        };

        let translated = translate_text(&config, TranslationKind::AgentReasoningTitle, "Thinking")
            .await
            .expect("translation should succeed");
        assert_eq!(translated, "translated");
        Ok(())
    }

    #[tokio::test]
    async fn translate_text_non_zero_exit_is_error() -> io::Result<()> {
        let config = AgentReasoningTranslationConfig {
            command: fail_command(),
            timeout: Duration::from_millis(2_000),
            ui_max_wait: Duration::from_millis(5_000),
        };

        let err = translate_text(&config, TranslationKind::AgentReasoningTitle, "Thinking")
            .await
            .expect_err("should fail");
        let msg = err.to_string();
        assert!(msg.contains("exited non-zero"));
        assert!(msg.contains("boom"));
        Ok(())
    }

    #[tokio::test]
    async fn translate_text_timeout_is_error() -> io::Result<()> {
        let config = AgentReasoningTranslationConfig {
            command: sleep_command(),
            timeout: Duration::from_millis(50),
            ui_max_wait: Duration::from_millis(5_000),
        };

        let err = translate_text(&config, TranslationKind::AgentReasoningTitle, "Thinking")
            .await
            .expect_err("should time out");
        let msg = err.to_string();
        assert!(msg.contains("timed out"));
        Ok(())
    }
}
