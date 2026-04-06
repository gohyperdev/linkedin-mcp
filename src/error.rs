use thiserror::Error;

#[derive(Debug, Error)]
pub enum LinkedInError {
    #[error("Not authenticated. Run the auth flow first.")]
    NotAuthenticated,

    #[error("Token expired and refresh failed: {0}")]
    TokenRefreshFailed(String),

    #[error("OAuth callback error: {0}")]
    OAuthCallbackError(String),

    #[error("LinkedIn API error ({status}): {message}")]
    ApiError { status: u16, message: String },

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON serialization error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),

    #[error("File not found: {0}")]
    FileNotFound(String),

    #[error("{0}")]
    Other(String),
}

impl LinkedInError {
    pub fn to_mcp_error(&self) -> rmcp::ErrorData {
        rmcp::ErrorData::internal_error(self.to_string(), None)
    }
}
