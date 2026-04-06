use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::error::LinkedInError;

const AUTH_URL: &str = "https://www.linkedin.com/oauth/v2/authorization";
const TOKEN_URL: &str = "https://www.linkedin.com/oauth/v2/accessToken";
const REDIRECT_URI: &str = "http://localhost:3000/callback";
const SCOPES: &str = "openid profile email w_member_social";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub linkedin_id: Option<String>,
    pub email: Option<String>,
}

impl TokenData {
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }
}

fn tokens_path() -> PathBuf {
    // Use ~/.config/linkedin-mcp/ (XDG standard) on all platforms,
    // not dirs::config_dir() which returns ~/Library/Application Support/ on macOS.
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    home.join(".config").join("linkedin-mcp").join("tokens.json")
}

fn get_client_credentials() -> Result<(String, String), LinkedInError> {
    let client_id = std::env::var("LINKEDIN_CLIENT_ID")
        .map_err(|_| LinkedInError::MissingEnvVar("LINKEDIN_CLIENT_ID".into()))?;
    let client_secret = std::env::var("LINKEDIN_CLIENT_SECRET")
        .map_err(|_| LinkedInError::MissingEnvVar("LINKEDIN_CLIENT_SECRET".into()))?;
    Ok((client_id, client_secret))
}

pub async fn load_tokens() -> Result<Option<TokenData>, LinkedInError> {
    let path = tokens_path();
    if !path.exists() {
        return Ok(None);
    }
    let data = tokio::fs::read_to_string(&path).await?;
    let tokens: TokenData = serde_json::from_str(&data)?;
    Ok(Some(tokens))
}

pub async fn save_tokens(tokens: &TokenData) -> Result<(), LinkedInError> {
    let path = tokens_path();
    if let Some(parent) = path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let data = serde_json::to_string_pretty(tokens)?;
    tokio::fs::write(&path, data).await?;
    tracing::info!("Tokens saved to {}", path.display());
    Ok(())
}

pub async fn get_valid_token() -> Result<TokenData, LinkedInError> {
    let tokens = load_tokens().await?;
    match tokens {
        Some(t) if !t.is_expired() => Ok(t),
        Some(t) => {
            tracing::info!("Token expired, attempting refresh");
            match refresh_token(&t).await {
                Ok(refreshed) => {
                    save_tokens(&refreshed).await?;
                    Ok(refreshed)
                }
                Err(_) => Err(LinkedInError::NotAuthenticated),
            }
        }
        None => Err(LinkedInError::NotAuthenticated),
    }
}

async fn refresh_token(tokens: &TokenData) -> Result<TokenData, LinkedInError> {
    let refresh = tokens
        .refresh_token
        .as_ref()
        .ok_or_else(|| LinkedInError::TokenRefreshFailed("No refresh token".into()))?;

    let (client_id, client_secret) = get_client_credentials()?;
    let client = reqwest::Client::new();

    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh.as_str()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(LinkedInError::TokenRefreshFailed(format!(
            "HTTP {status}: {body}"
        )));
    }

    let token_resp: TokenResponse = resp.json().await?;
    let new_tokens = TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token.or(tokens.refresh_token.clone()),
        expires_at: Utc::now()
            + chrono::Duration::seconds(token_resp.expires_in.unwrap_or(5184000) as i64),
        linkedin_id: tokens.linkedin_id.clone(),
        email: tokens.email.clone(),
    };

    Ok(new_tokens)
}

pub async fn run_auth_flow() -> Result<TokenData, LinkedInError> {
    let (client_id, client_secret) = get_client_credentials()?;

    let state = format!("{:x}", rand_state());
    let auth_url = format!(
        "{AUTH_URL}?response_type=code&client_id={client_id}&redirect_uri={}&scope={}&state={state}",
        urlencoded(REDIRECT_URI),
        urlencoded(SCOPES),
    );

    eprintln!();
    eprintln!("=== LinkedIn OAuth Authorization ===");
    eprintln!("Open this URL in your browser:");
    eprintln!();
    eprintln!("  {auth_url}");
    eprintln!();
    eprintln!("Waiting for callback on {REDIRECT_URI} ...");
    eprintln!();

    let code = wait_for_callback(&state).await?;

    let client = reqwest::Client::new();
    let resp = client
        .post(TOKEN_URL)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", &code),
            ("redirect_uri", &REDIRECT_URI.to_string()),
            ("client_id", &client_id),
            ("client_secret", &client_secret),
        ])
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(LinkedInError::ApiError {
            status,
            message: format!("Token exchange failed: {body}"),
        });
    }

    let token_resp: TokenResponse = resp.json().await?;
    let tokens = TokenData {
        access_token: token_resp.access_token,
        refresh_token: token_resp.refresh_token,
        expires_at: Utc::now()
            + chrono::Duration::seconds(token_resp.expires_in.unwrap_or(5184000) as i64),
        linkedin_id: None,
        email: None,
    };

    save_tokens(&tokens).await?;
    Ok(tokens)
}

async fn wait_for_callback(expected_state: &str) -> Result<String, LinkedInError> {
    let listener = TcpListener::bind("127.0.0.1:3000")
        .await
        .map_err(|e| LinkedInError::OAuthCallbackError(format!("Failed to bind port 3000: {e}")))?;

    let (mut stream, _addr) = listener.accept().await?;

    let mut buf = vec![0u8; 4096];
    let n = stream.read(&mut buf).await?;
    let request = String::from_utf8_lossy(&buf[..n]);

    let first_line = request.lines().next().unwrap_or("");
    let path = first_line.split_whitespace().nth(1).unwrap_or("");

    let query = path.split('?').nth(1).unwrap_or("");
    let params: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|p| {
            let mut parts = p.splitn(2, '=');
            Some((parts.next()?, parts.next()?))
        })
        .collect();

    let code = params
        .iter()
        .find(|(k, _)| *k == "code")
        .map(|(_, v)| v.to_string());
    let state = params
        .iter()
        .find(|(k, _)| *k == "state")
        .map(|(_, v)| v.to_string());
    let error = params
        .iter()
        .find(|(k, _)| *k == "error")
        .map(|(_, v)| v.to_string());

    let response_body = if code.is_some() {
        "<html><body><h1>Authorization successful!</h1><p>You can close this window and return to your terminal.</p></body></html>"
    } else {
        "<html><body><h1>Authorization failed.</h1><p>Check your terminal for details.</p></body></html>"
    };

    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        response_body.len(),
        response_body
    );
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;

    if let Some(err) = error {
        return Err(LinkedInError::OAuthCallbackError(err));
    }

    if let Some(s) = &state {
        if s != expected_state {
            return Err(LinkedInError::OAuthCallbackError(
                "State mismatch — possible CSRF".into(),
            ));
        }
    }

    code.ok_or_else(|| LinkedInError::OAuthCallbackError("No authorization code received".into()))
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

fn urlencoded(s: &str) -> String {
    s.replace(' ', "%20")
        .replace(':', "%3A")
        .replace('/', "%2F")
}

fn rand_state() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    duration.as_nanos() as u64 ^ duration.as_secs()
}
