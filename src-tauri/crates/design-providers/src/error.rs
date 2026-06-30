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
    #[error("provider returned HTTP status {status}")]
    Http { status: u16 },
    #[error("provider transport failed: {message}")]
    Transport { message: String },
    #[error("provider returned an invalid response: {message}")]
    InvalidResponse { message: String },
}

impl ProviderError {
    pub(crate) fn from_status(status: reqwest::StatusCode) -> Self {
        match status.as_u16() {
            401 => Self::Authentication,
            429 => Self::RateLimited,
            status => Self::Http { status },
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
