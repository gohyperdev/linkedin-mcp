use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::router::tool::ToolRouter,
    handler::server::wrapper::Parameters,
    model::*,
    schemars, tool, tool_handler, tool_router,
};

use crate::auth;
use crate::linkedin::LinkedInClient;

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateTextPostParams {
    /// The text content of the LinkedIn post
    pub text: String,
    /// Post visibility: "PUBLIC" or "CONNECTIONS" (default: "PUBLIC")
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct ShareUrlParams {
    /// The text commentary for the post
    pub text: String,
    /// The URL to share (LinkedIn will render a preview card)
    pub url: String,
    /// Custom title for the URL preview (optional — LinkedIn auto-fetches from Open Graph if omitted)
    #[serde(default)]
    pub title: Option<String>,
    /// Custom description for the URL preview (optional)
    #[serde(default)]
    pub description: Option<String>,
    /// Post visibility: "PUBLIC" or "CONNECTIONS" (default: "PUBLIC")
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateVideoPostParams {
    /// The text content of the LinkedIn post
    pub text: String,
    /// Absolute path to the video file on disk
    pub video_path: String,
    /// Title for the video (optional)
    #[serde(default)]
    pub title: Option<String>,
    /// Description for the video (optional)
    #[serde(default)]
    pub description: Option<String>,
    /// Post visibility: "PUBLIC" or "CONNECTIONS" (default: "PUBLIC")
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Debug, serde::Deserialize, schemars::JsonSchema)]
pub struct CreateImagePostParams {
    /// The text content of the LinkedIn post
    pub text: String,
    /// Absolute path to the image file on disk
    pub image_path: String,
    /// Alt text for the image (accessibility)
    #[serde(default)]
    pub image_alt: Option<String>,
    /// Post visibility: "PUBLIC" or "CONNECTIONS" (default: "PUBLIC")
    #[serde(default)]
    pub visibility: Option<String>,
}

#[derive(Clone)]
pub struct LinkedInMcpServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl LinkedInMcpServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    /// Get the authenticated LinkedIn user's profile information.
    #[tool(description = "Get LinkedIn profile info (name, email, ID, picture URL)")]
    async fn get_profile(&self) -> Result<CallToolResult, McpError> {
        let client = LinkedInClient::new();
        let profile = client.get_profile().await.map_err(|e| e.to_mcp_error())?;

        let text = format!(
            "Name: {}\nEmail: {}\nLinkedIn ID: {}\nPicture: {}",
            profile.name.as_deref().unwrap_or("N/A"),
            profile.email.as_deref().unwrap_or("N/A"),
            profile.sub,
            profile.picture.as_deref().unwrap_or("N/A"),
        );

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }

    /// Create a text-only post on LinkedIn.
    #[tool(description = "Create a text-only post on LinkedIn")]
    async fn create_text_post(
        &self,
        Parameters(params): Parameters<CreateTextPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let visibility = params.visibility.as_deref().unwrap_or("PUBLIC");
        let client = LinkedInClient::new();
        let post_urn = client
            .create_text_post(&params.text, visibility)
            .await
            .map_err(|e| e.to_mcp_error())?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Post created successfully.\nPost URN: {post_urn}"
        ))]))
    }

    /// Create a post with an image on LinkedIn.
    #[tool(description = "Create a post with an image on LinkedIn")]
    async fn create_image_post(
        &self,
        Parameters(params): Parameters<CreateImagePostParams>,
    ) -> Result<CallToolResult, McpError> {
        let visibility = params.visibility.as_deref().unwrap_or("PUBLIC");
        let client = LinkedInClient::new();
        let post_urn = client
            .create_image_post(
                &params.text,
                &params.image_path,
                params.image_alt.as_deref(),
                visibility,
            )
            .await
            .map_err(|e| e.to_mcp_error())?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Image post created successfully.\nPost URN: {post_urn}"
        ))]))
    }

    /// Share a URL/article on LinkedIn with an optional custom title and description.
    #[tool(description = "Share a URL on LinkedIn with preview card (title, description auto-fetched from Open Graph if not provided)")]
    async fn share_url(
        &self,
        Parameters(params): Parameters<ShareUrlParams>,
    ) -> Result<CallToolResult, McpError> {
        let visibility = params.visibility.as_deref().unwrap_or("PUBLIC");
        let client = LinkedInClient::new();
        let post_urn = client
            .create_article_post(
                &params.text,
                &params.url,
                params.title.as_deref(),
                params.description.as_deref(),
                visibility,
            )
            .await
            .map_err(|e| e.to_mcp_error())?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "URL shared successfully.\nPost URN: {post_urn}"
        ))]))
    }

    /// Create a post with a video on LinkedIn.
    #[tool(description = "Create a post with a video on LinkedIn")]
    async fn create_video_post(
        &self,
        Parameters(params): Parameters<CreateVideoPostParams>,
    ) -> Result<CallToolResult, McpError> {
        let visibility = params.visibility.as_deref().unwrap_or("PUBLIC");
        let client = LinkedInClient::new();
        let post_urn = client
            .create_video_post(
                &params.text,
                &params.video_path,
                params.title.as_deref(),
                params.description.as_deref(),
                visibility,
            )
            .await
            .map_err(|e| e.to_mcp_error())?;

        Ok(CallToolResult::success(vec![Content::text(format!(
            "Video post created successfully.\nPost URN: {post_urn}"
        ))]))
    }

    /// Check current authentication status.
    #[tool(description = "Check LinkedIn authentication status (token exists, expiry, email)")]
    async fn auth_status(&self) -> Result<CallToolResult, McpError> {
        let tokens = auth::load_tokens()
            .await
            .map_err(|e| e.to_mcp_error())?;

        let text = match tokens {
            Some(t) => {
                let expired = if t.is_expired() { " (EXPIRED)" } else { "" };
                format!(
                    "Authenticated: yes\nEmail: {}\nLinkedIn ID: {}\nToken expires: {}{expired}",
                    t.email.as_deref().unwrap_or("N/A"),
                    t.linkedin_id.as_deref().unwrap_or("N/A"),
                    t.expires_at.format("%Y-%m-%d %H:%M:%S UTC"),
                )
            }
            None => "Authenticated: no\nNo token file found. Use get_profile or create_text_post to trigger the OAuth flow.".to_string(),
        };

        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for LinkedInMcpServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("linkedin-mcp", env!("CARGO_PKG_VERSION")))
            .with_instructions(
                "LinkedIn MCP server for publishing content to LinkedIn. \
                 You can publish posts on behalf of the authenticated LinkedIn user.\n\n\
                 Available tools:\n\
                 - create_text_post: Publish a text-only post (up to 3000 chars). Use for announcements, thoughts, updates.\n\
                 - share_url: Share a URL/link with a preview card. LinkedIn auto-fetches title and thumbnail from the page. \
                   Use this for sharing articles, blog posts, GitHub repos, or any web content.\n\
                 - create_image_post: Publish a post with an attached image (provide local file path). \
                   Use for infographics, screenshots, photos.\n\
                 - create_video_post: Publish a post with an attached video (provide local file path). \
                   Use for demos, tutorials, event recordings.\n\
                 - get_profile: Get the authenticated user's LinkedIn profile (name, email, ID, picture).\n\
                 - auth_status: Check if OAuth token is valid and when it expires.\n\n\
                 All post tools accept an optional 'visibility' parameter: 'PUBLIC' (default) or 'CONNECTIONS'.\n\n\
                 Note: This server publishes as a personal profile, not a Company Page. \
                 LinkedIn API does not support creating native long-form articles or newsletters — \
                 those can only be created through the LinkedIn web UI. \
                 To share an article, use share_url with the article's URL."
                    .to_string(),
            )
    }
}
