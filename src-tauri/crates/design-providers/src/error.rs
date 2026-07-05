use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ProviderError {
    #[error("provider authentication failed")]
    Authentication,
    #[error("provider rate limit was reached")]
    RateLimited,
    #[error("provider request timed out")]
    Timeout,
    #[error("configured provider does not support the requested capability")]
    CapabilityMismatch,
    #[error("model is required")]
    MissingModel,
    #[error("provider returned HTTP status {status}{}", detail.as_deref().map(|detail| format!(": {detail}")).unwrap_or_default())]
    Http { status: u16, detail: Option<String> },
    #[error("provider transport failed: {message}")]
    Transport { message: String },
    #[error("provider returned an invalid response: {message}")]
    InvalidResponse { message: String },
}

const ERROR_DETAIL_MAX_CHARS: usize = 300;

impl ProviderError {
    pub(crate) fn from_status_and_body(status: reqwest::StatusCode, body: &str) -> Self {
        match status.as_u16() {
            401 | 403 => Self::Authentication,
            429 => Self::RateLimited,
            status => Self::Http {
                status,
                detail: extract_error_message(body),
            },
        }
    }

    pub(crate) fn from_reqwest(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            Self::Timeout
        } else {
            Self::Transport {
                message: error.to_string(),
            }
        }
    }
}

/// Pulls the human-readable message out of a provider error body
/// (`{"error":{"message":"..."}}` or `{"error":"..."}`). The raw body is
/// never included verbatim so unexpected payloads cannot leak into logs
/// or the UI.
fn extract_error_message(body: &str) -> Option<String> {
    let envelope: Value = serde_json::from_str(body).ok()?;
    let error = envelope.get("error")?;
    let message = match error {
        Value::String(message) => message.as_str(),
        Value::Object(_) => error.get("message")?.as_str()?,
        _ => return None,
    };
    let message = message.trim();
    if message.is_empty() {
        return None;
    }
    Some(message.chars().take(ERROR_DETAIL_MAX_CHARS).collect())
}
