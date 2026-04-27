//! Translation configuration.
//!
//! Configuration is stored at `~/.codex/translation.toml`.

use serde::Deserialize;
use serde::Serialize;
use std::fs;
use std::path::PathBuf;

use super::provider::ProviderDef;
use super::provider::ProviderId;

/// Default timeout for translation requests (in milliseconds).
#[allow(dead_code)]
const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// Translation configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationConfig {
    /// Whether translation is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Target language code (e.g., "zh-CN").
    #[serde(default = "default_target_language")]
    pub target_language: String,

    /// Provider identifier (e.g., "deepseek", "openai").
    #[serde(default = "default_provider")]
    pub provider: String,

    /// API key for the provider (stored in config file).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    /// Model name (overrides provider default).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,

    /// Custom base URL (for proxies or self-hosted).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,

    /// Timeout in milliseconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

fn default_target_language() -> String {
    "zh-CN".to_string()
}

fn default_provider() -> String {
    ProviderId::default().as_str().to_string()
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            target_language: default_target_language(),
            provider: default_provider(),
            api_key: None,
            model: None,
            base_url: None,
            timeout_ms: None,
        }
    }
}

impl TranslationConfig {
    /// Get the configuration file path.
    pub fn config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|home| home.join(".codex").join("translation.toml"))
    }

    /// Load configuration from file, or return default if not found.
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            return Self::default();
        };

        if !path.exists() {
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<TranslationConfig>(&content) {
                Ok(config) => config,
                Err(e) => {
                    tracing::warn!("Failed to parse translation config: {}, using default", e);
                    Self::default()
                }
            },
            Err(e) => {
                tracing::warn!("Failed to read translation config: {}, using default", e);
                Self::default()
            }
        }
    }

    /// Save configuration to file.
    pub fn save(&self) -> std::io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Cannot determine config file path",
            ));
        };

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = toml::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        fs::write(&path, &content)?;

        // Set restrictive permissions on Unix (600 - owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let permissions = std::fs::Permissions::from_mode(0o600);
            let _ = fs::set_permissions(&path, permissions);
        }

        Ok(())
    }

    /// Check if translation is enabled.
    #[allow(dead_code)]
    pub fn should_translate(&self) -> bool {
        self.enabled
    }

    /// Get the effective provider ID.
    pub fn effective_provider(&self) -> ProviderId {
        ProviderId::from_str(&self.provider).unwrap_or_default()
    }

    /// Get the effective API key.
    pub fn effective_api_key(&self) -> Option<&str> {
        self.api_key.as_deref().filter(|k| !k.is_empty())
    }

    /// Get the effective base URL.
    pub fn effective_base_url(&self, provider: &ProviderDef) -> &str {
        self.base_url
            .as_deref()
            .filter(|u| !u.is_empty())
            .unwrap_or(provider.default_base_url)
    }

    /// Get the effective model name.
    pub fn effective_model(&self, provider: &ProviderDef) -> &str {
        self.model
            .as_deref()
            .filter(|m| !m.is_empty())
            .unwrap_or(provider.default_model)
    }

    /// Get the effective timeout in milliseconds.
    #[allow(dead_code)]
    pub fn effective_timeout_ms(&self) -> u64 {
        self.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS)
    }

    /// Check if API key is configured.
    #[allow(dead_code)]
    pub fn has_api_key(&self) -> bool {
        self.effective_api_key().is_some()
    }

    /// Check if the configuration is valid for translation.
    #[allow(dead_code)]
    pub fn is_valid(&self) -> bool {
        let provider = self.effective_provider();
        let def = provider.definition();
        !def.requires_api_key || self.has_api_key()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn translation_config_should_translate() {
        let config = TranslationConfig {
            enabled: true,
            target_language: "zh-CN".to_string(),
            ..Default::default()
        };

        assert!(config.should_translate());

        let disabled = TranslationConfig {
            enabled: false,
            ..config
        };
        assert!(!disabled.should_translate());
    }

    #[test]
    fn translation_config_serialization() {
        let config = TranslationConfig {
            enabled: true,
            target_language: "ja".to_string(),
            provider: "deepseek".to_string(),
            api_key: Some("sk-test123".to_string()),
            model: Some("deepseek-chat".to_string()),
            base_url: None,
            timeout_ms: Some(15000),
        };

        let toml_str = toml::to_string(&config).unwrap();
        let parsed: TranslationConfig = toml::from_str(&toml_str).unwrap();

        assert_eq!(parsed.enabled, config.enabled);
        assert_eq!(parsed.target_language, config.target_language);
        assert_eq!(parsed.provider, config.provider);
        assert_eq!(parsed.api_key, config.api_key);
        assert_eq!(parsed.model, config.model);
        assert_eq!(parsed.timeout_ms, config.timeout_ms);
    }

    #[test]
    fn translation_config_effective_values() {
        let config = TranslationConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-xxx".to_string()),
            model: None,
            base_url: None,
            ..Default::default()
        };

        assert_eq!(config.effective_provider(), ProviderId::OpenAI);
        assert_eq!(config.effective_api_key(), Some("sk-xxx"));

        let provider_def = config.effective_provider().definition();
        assert_eq!(config.effective_model(provider_def), "gpt-4o-mini");
        assert_eq!(
            config.effective_base_url(provider_def),
            "https://api.openai.com/v1"
        );
    }

    #[test]
    fn translation_config_is_valid() {
        // Config with API key for provider that requires it
        let valid_config = TranslationConfig {
            provider: "openai".to_string(),
            api_key: Some("sk-xxx".to_string()),
            ..Default::default()
        };
        assert!(valid_config.is_valid());

        // Config without API key for provider that requires it
        let invalid_config = TranslationConfig {
            provider: "openai".to_string(),
            api_key: None,
            ..Default::default()
        };
        assert!(!invalid_config.is_valid());

        // Ollama doesn't require API key
        let ollama_config = TranslationConfig {
            provider: "ollama".to_string(),
            api_key: None,
            ..Default::default()
        };
        assert!(ollama_config.is_valid());
    }
}
