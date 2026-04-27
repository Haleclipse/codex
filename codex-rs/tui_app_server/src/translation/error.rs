//! Translation error types.

use std::fmt;

/// Translation error.
#[derive(Debug)]
pub enum TranslationError {
    /// API key not found or not configured.
    ApiKeyNotFound(String),

    /// Network error during API call.
    Network(reqwest::Error),

    /// API returned an error response.
    Api { status: u16, message: String },

    /// Failed to parse API response.
    Parse(String),

    /// Translation request timed out.
    Timeout,

    /// Provider not supported.
    #[allow(dead_code)]
    UnsupportedProvider(String),

    /// Invalid configuration.
    #[allow(dead_code)]
    InvalidConfig(String),
}

impl fmt::Display for TranslationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ApiKeyNotFound(provider) => {
                write!(f, "API key not configured for {provider}")
            }
            Self::Network(e) => write!(f, "Network error: {e}"),
            Self::Api { status, message } => {
                write!(f, "API error ({status}): {message}")
            }
            Self::Parse(msg) => write!(f, "Parse error: {msg}"),
            Self::Timeout => write!(f, "Translation timeout"),
            Self::UnsupportedProvider(provider) => {
                write!(f, "Unsupported provider: {provider}")
            }
            Self::InvalidConfig(msg) => write!(f, "Invalid configuration: {msg}"),
        }
    }
}

impl std::error::Error for TranslationError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Network(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for TranslationError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            Self::Timeout
        } else {
            Self::Network(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = TranslationError::ApiKeyNotFound("DeepSeek".to_string());
        assert!(err.to_string().contains("DeepSeek"));

        let err = TranslationError::Timeout;
        assert!(err.to_string().contains("timeout"));

        let err = TranslationError::Api {
            status: 401,
            message: "Unauthorized".to_string(),
        };
        assert!(err.to_string().contains("401"));
        assert!(err.to_string().contains("Unauthorized"));
    }
}
