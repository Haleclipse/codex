//! Translation provider definitions.
//!
//! This module defines supported LLM providers for translation,
//! including their default configurations and protocol types.

use serde::Deserialize;
use serde::Serialize;

/// Protocol type for API communication.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    /// OpenAI-compatible API (used by most providers).
    OpenAI,
    /// Anthropic's native API.
    Anthropic,
    /// Google's Gemini API.
    Gemini,
}

/// Provider identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ProviderId {
    OpenAI,
    Anthropic,
    #[default]
    DeepSeek,
    Moonshot,
    ZhipuAI,
    Qwen,
    Groq,
    Gemini,
    Mistral,
    Cohere,
    Ollama,
    OpenRouter,
    TogetherAI,
    Perplexity,
    SiliconFlow,
}

impl ProviderId {
    /// Get all provider IDs.
    pub const ALL: &'static [Self] = &[
        Self::OpenAI,
        Self::Anthropic,
        Self::DeepSeek,
        Self::Moonshot,
        Self::ZhipuAI,
        Self::Qwen,
        Self::Groq,
        Self::Gemini,
        Self::Mistral,
        Self::Cohere,
        Self::Ollama,
        Self::OpenRouter,
        Self::TogetherAI,
        Self::Perplexity,
        Self::SiliconFlow,
    ];

    /// Get the provider definition.
    pub fn definition(self) -> &'static ProviderDef {
        match self {
            Self::OpenAI => &OPENAI,
            Self::Anthropic => &ANTHROPIC,
            Self::DeepSeek => &DEEPSEEK,
            Self::Moonshot => &MOONSHOT,
            Self::ZhipuAI => &ZHIPUAI,
            Self::Qwen => &QWEN,
            Self::Groq => &GROQ,
            Self::Gemini => &GEMINI,
            Self::Mistral => &MISTRAL,
            Self::Cohere => &COHERE,
            Self::Ollama => &OLLAMA,
            Self::OpenRouter => &OPENROUTER,
            Self::TogetherAI => &TOGETHERAI,
            Self::Perplexity => &PERPLEXITY,
            Self::SiliconFlow => &SILICONFLOW,
        }
    }

    /// Get provider ID from string.
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "openai" => Some(Self::OpenAI),
            "anthropic" => Some(Self::Anthropic),
            "deepseek" => Some(Self::DeepSeek),
            "moonshot" => Some(Self::Moonshot),
            "zhipuai" | "zhipu" => Some(Self::ZhipuAI),
            "qwen" | "dashscope" => Some(Self::Qwen),
            "groq" => Some(Self::Groq),
            "gemini" | "google" => Some(Self::Gemini),
            "mistral" => Some(Self::Mistral),
            "cohere" => Some(Self::Cohere),
            "ollama" => Some(Self::Ollama),
            "openrouter" => Some(Self::OpenRouter),
            "togetherai" | "together" => Some(Self::TogetherAI),
            "perplexity" => Some(Self::Perplexity),
            "siliconflow" => Some(Self::SiliconFlow),
            _ => None,
        }
    }

    /// Convert to lowercase string identifier.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAI => "openai",
            Self::Anthropic => "anthropic",
            Self::DeepSeek => "deepseek",
            Self::Moonshot => "moonshot",
            Self::ZhipuAI => "zhipuai",
            Self::Qwen => "qwen",
            Self::Groq => "groq",
            Self::Gemini => "gemini",
            Self::Mistral => "mistral",
            Self::Cohere => "cohere",
            Self::Ollama => "ollama",
            Self::OpenRouter => "openrouter",
            Self::TogetherAI => "togetherai",
            Self::Perplexity => "perplexity",
            Self::SiliconFlow => "siliconflow",
        }
    }
}

impl std::fmt::Display for ProviderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.definition().name)
    }
}

/// Provider definition with default configuration.
#[derive(Debug)]
pub struct ProviderDef {
    /// Provider identifier.
    #[allow(dead_code)]
    pub id: ProviderId,
    /// Display name.
    pub name: &'static str,
    /// Default base URL.
    pub default_base_url: &'static str,
    /// Default model name.
    pub default_model: &'static str,
    /// API protocol type.
    pub protocol: Protocol,
    /// Whether API key is required.
    pub requires_api_key: bool,
    /// Description of the provider.
    pub description: &'static str,
}

// Provider definitions
static OPENAI: ProviderDef = ProviderDef {
    id: ProviderId::OpenAI,
    name: "OpenAI",
    default_base_url: "https://api.openai.com/v1",
    default_model: "gpt-4o-mini",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "OpenAI GPT models",
};

static ANTHROPIC: ProviderDef = ProviderDef {
    id: ProviderId::Anthropic,
    name: "Anthropic",
    default_base_url: "https://api.anthropic.com/v1",
    default_model: "claude-3-haiku-20240307",
    protocol: Protocol::Anthropic,
    requires_api_key: true,
    description: "Anthropic Claude models",
};

static DEEPSEEK: ProviderDef = ProviderDef {
    id: ProviderId::DeepSeek,
    name: "DeepSeek",
    default_base_url: "https://api.deepseek.com/v1",
    default_model: "deepseek-chat",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "DeepSeek AI models",
};

static MOONSHOT: ProviderDef = ProviderDef {
    id: ProviderId::Moonshot,
    name: "Moonshot",
    default_base_url: "https://api.moonshot.cn/v1",
    default_model: "moonshot-v1-8k",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Moonshot (Kimi) AI models",
};

static ZHIPUAI: ProviderDef = ProviderDef {
    id: ProviderId::ZhipuAI,
    name: "ZhipuAI",
    default_base_url: "https://open.bigmodel.cn/api/paas/v4",
    default_model: "glm-4-flash",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Zhipu GLM models",
};

static QWEN: ProviderDef = ProviderDef {
    id: ProviderId::Qwen,
    name: "Qwen",
    default_base_url: "https://dashscope.aliyuncs.com/compatible-mode/v1",
    default_model: "qwen-turbo",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Alibaba Qwen models (DashScope)",
};

static GROQ: ProviderDef = ProviderDef {
    id: ProviderId::Groq,
    name: "Groq",
    default_base_url: "https://api.groq.com/openai/v1",
    default_model: "llama-3.1-8b-instant",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Groq LPU inference",
};

static GEMINI: ProviderDef = ProviderDef {
    id: ProviderId::Gemini,
    name: "Gemini",
    default_base_url: "https://generativelanguage.googleapis.com/v1beta",
    default_model: "gemini-1.5-flash",
    protocol: Protocol::Gemini,
    requires_api_key: true,
    description: "Google Gemini models",
};

static MISTRAL: ProviderDef = ProviderDef {
    id: ProviderId::Mistral,
    name: "Mistral",
    default_base_url: "https://api.mistral.ai/v1",
    default_model: "mistral-small-latest",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Mistral AI models",
};

static COHERE: ProviderDef = ProviderDef {
    id: ProviderId::Cohere,
    name: "Cohere",
    default_base_url: "https://api.cohere.ai/v1",
    default_model: "command-r",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Cohere Command models",
};

static OLLAMA: ProviderDef = ProviderDef {
    id: ProviderId::Ollama,
    name: "Ollama",
    default_base_url: "http://localhost:11434/v1",
    default_model: "llama3",
    protocol: Protocol::OpenAI,
    requires_api_key: false,
    description: "Ollama local models",
};

static OPENROUTER: ProviderDef = ProviderDef {
    id: ProviderId::OpenRouter,
    name: "OpenRouter",
    default_base_url: "https://openrouter.ai/api/v1",
    default_model: "openai/gpt-4o-mini",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "OpenRouter unified API",
};

static TOGETHERAI: ProviderDef = ProviderDef {
    id: ProviderId::TogetherAI,
    name: "TogetherAI",
    default_base_url: "https://api.together.xyz/v1",
    default_model: "meta-llama/Llama-3-8b-chat-hf",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Together AI inference",
};

static PERPLEXITY: ProviderDef = ProviderDef {
    id: ProviderId::Perplexity,
    name: "Perplexity",
    default_base_url: "https://api.perplexity.ai",
    default_model: "llama-3.1-sonar-small-128k-online",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "Perplexity AI models",
};

static SILICONFLOW: ProviderDef = ProviderDef {
    id: ProviderId::SiliconFlow,
    name: "SiliconFlow",
    default_base_url: "https://api.siliconflow.cn/v1",
    default_model: "Qwen/Qwen2.5-7B-Instruct",
    protocol: Protocol::OpenAI,
    requires_api_key: true,
    description: "SiliconFlow inference",
};

/// Get all provider definitions.
#[allow(dead_code)]
pub static PROVIDERS: &[&ProviderDef] = &[
    &OPENAI,
    &ANTHROPIC,
    &DEEPSEEK,
    &MOONSHOT,
    &ZHIPUAI,
    &QWEN,
    &GROQ,
    &GEMINI,
    &MISTRAL,
    &COHERE,
    &OLLAMA,
    &OPENROUTER,
    &TOGETHERAI,
    &PERPLEXITY,
    &SILICONFLOW,
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_id_from_str() {
        assert_eq!(ProviderId::from_str("openai"), Some(ProviderId::OpenAI));
        assert_eq!(ProviderId::from_str("DEEPSEEK"), Some(ProviderId::DeepSeek));
        assert_eq!(ProviderId::from_str("zhipu"), Some(ProviderId::ZhipuAI));
        assert_eq!(ProviderId::from_str("unknown"), None);
    }

    #[test]
    fn provider_id_as_str() {
        assert_eq!(ProviderId::OpenAI.as_str(), "openai");
        assert_eq!(ProviderId::DeepSeek.as_str(), "deepseek");
    }

    #[test]
    fn provider_definition() {
        let def = ProviderId::DeepSeek.definition();
        assert_eq!(def.name, "DeepSeek");
        assert!(def.requires_api_key);
        assert_eq!(def.protocol, Protocol::OpenAI);
    }

    #[test]
    fn provider_count() {
        assert_eq!(ProviderId::ALL.len(), PROVIDERS.len());
    }
}
