//! Translation HTTP client.
//!
//! This module provides the HTTP client for making translation requests
//! to various LLM providers.

use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;
use serde::Serialize;

use super::config::TranslationConfig;
use super::error::TranslationError;
use super::provider::Protocol;
use super::provider::ProviderDef;

/// Default timeout for translation requests (in milliseconds).
const DEFAULT_TIMEOUT_MS: u64 = 30000;

/// Translation client.
pub struct TranslationClient {
    client: Client,
    provider: &'static ProviderDef,
    api_key: Option<String>,
    base_url: String,
    model: String,
    #[allow(dead_code)]
    timeout: Duration,
}

impl TranslationClient {
    /// Create a new translation client from configuration.
    pub fn from_config(config: &TranslationConfig) -> Result<Self, TranslationError> {
        let provider_id = config.effective_provider();
        let provider = provider_id.definition();

        // Check if API key is required
        let api_key = config.effective_api_key().map(String::from);
        if provider.requires_api_key && api_key.is_none() {
            return Err(TranslationError::ApiKeyNotFound(provider.name.to_string()));
        }

        let base_url = config.effective_base_url(provider).to_string();
        let model = config.effective_model(provider).to_string();
        let timeout = Duration::from_millis(config.timeout_ms.unwrap_or(DEFAULT_TIMEOUT_MS));

        let client = Client::builder()
            .timeout(timeout)
            .build()
            .map_err(TranslationError::Network)?;

        Ok(Self {
            client,
            provider,
            api_key,
            base_url,
            model,
            timeout,
        })
    }

    /// Translate text to the target language.
    pub async fn translate(
        &self,
        text: &str,
        target_lang: &str,
    ) -> Result<String, TranslationError> {
        let prompt = build_translation_prompt(text, target_lang);

        match self.provider.protocol {
            Protocol::OpenAI => self.call_openai_compatible(&prompt).await,
            Protocol::Anthropic => self.call_anthropic(&prompt).await,
            Protocol::Gemini => self.call_gemini(&prompt).await,
        }
    }

    /// Get the timeout duration.
    #[allow(dead_code)]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Call OpenAI-compatible API.
    async fn call_openai_compatible(&self, prompt: &str) -> Result<String, TranslationError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));

        let request = OpenAIRequest {
            model: &self.model,
            messages: vec![OpenAIMessage {
                role: "user",
                content: prompt,
            }],
            temperature: Some(0.3),
            max_tokens: None,
        };

        let mut req = self.client.post(&url).json(&request);

        if let Some(api_key) = &self.api_key {
            req = req.header("Authorization", format!("Bearer {api_key}"));
        }

        let response = req.send().await?;
        let status = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TranslationError::Api {
                status,
                message: error_text,
            });
        }

        let result: OpenAIResponse = response
            .json()
            .await
            .map_err(|e| TranslationError::Parse(e.to_string()))?;

        result
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| TranslationError::Parse("Empty response".to_string()))
    }

    /// Call Anthropic API.
    async fn call_anthropic(&self, prompt: &str) -> Result<String, TranslationError> {
        let url = format!("{}/messages", self.base_url.trim_end_matches('/'));

        let request = AnthropicRequest {
            model: &self.model,
            messages: vec![AnthropicMessage {
                role: "user",
                content: prompt,
            }],
            max_tokens: 4096,
        };

        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| TranslationError::ApiKeyNotFound("Anthropic".to_string()))?;

        let response = self
            .client
            .post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TranslationError::Api {
                status,
                message: error_text,
            });
        }

        let result: AnthropicResponse = response
            .json()
            .await
            .map_err(|e| TranslationError::Parse(e.to_string()))?;

        result
            .content
            .into_iter()
            .find(|c| c.content_type == "text")
            .and_then(|c| c.text)
            .ok_or_else(|| TranslationError::Parse("Empty response".to_string()))
    }

    /// Call Google Gemini API.
    async fn call_gemini(&self, prompt: &str) -> Result<String, TranslationError> {
        let api_key = self
            .api_key
            .as_ref()
            .ok_or_else(|| TranslationError::ApiKeyNotFound("Gemini".to_string()))?;

        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url.trim_end_matches('/'),
            self.model,
            api_key
        );

        let request = GeminiRequest {
            contents: vec![GeminiContent {
                parts: vec![GeminiPart { text: prompt }],
            }],
        };

        let response = self
            .client
            .post(&url)
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await?;

        let status = response.status().as_u16();

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_default();
            return Err(TranslationError::Api {
                status,
                message: error_text,
            });
        }

        let result: GeminiResponse = response
            .json()
            .await
            .map_err(|e| TranslationError::Parse(e.to_string()))?;

        result
            .candidates
            .into_iter()
            .next()
            .and_then(|c| c.content.parts.into_iter().next())
            .map(|p| p.text)
            .ok_or_else(|| TranslationError::Parse("Empty response".to_string()))
    }
}

/// Build the translation prompt.
fn build_translation_prompt(text: &str, target_lang: &str) -> String {
    format!(
        "Translate the following text to {target_lang}. \
         Keep the original formatting (markdown, code blocks, etc.). \
         Output only the translation, nothing else.\n\n{text}"
    )
}

// OpenAI API types
#[derive(Serialize)]
struct OpenAIRequest<'a> {
    model: &'a str,
    messages: Vec<OpenAIMessage<'a>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
}

#[derive(Serialize)]
struct OpenAIMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct OpenAIResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessageResponse,
}

#[derive(Deserialize)]
struct OpenAIMessageResponse {
    content: Option<String>,
}

// Anthropic API types
#[derive(Serialize)]
struct AnthropicRequest<'a> {
    model: &'a str,
    messages: Vec<AnthropicMessage<'a>>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct AnthropicMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContent>,
}

#[derive(Deserialize)]
struct AnthropicContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

// Gemini API types
#[derive(Serialize)]
struct GeminiRequest<'a> {
    contents: Vec<GeminiContent<'a>>,
}

#[derive(Serialize)]
struct GeminiContent<'a> {
    parts: Vec<GeminiPart<'a>>,
}

#[derive(Serialize, Deserialize)]
struct GeminiPart<'a> {
    text: &'a str,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiContentResponse,
}

#[derive(Deserialize)]
struct GeminiContentResponse {
    parts: Vec<GeminiPartResponse>,
}

#[derive(Deserialize)]
struct GeminiPartResponse {
    text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_prompt() {
        let prompt = build_translation_prompt("Hello, world!", "Chinese");
        assert!(prompt.contains("Chinese"));
        assert!(prompt.contains("Hello, world!"));
        assert!(prompt.contains("markdown"));
    }
}
