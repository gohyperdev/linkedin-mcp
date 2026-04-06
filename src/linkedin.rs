use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::auth::{self, TokenData};
use crate::error::LinkedInError;

const LINKEDIN_API_VERSION: &str = "202401";
const USERINFO_URL: &str = "https://api.linkedin.com/v2/userinfo";
const UGC_POSTS_URL: &str = "https://api.linkedin.com/v2/ugcPosts";
const REGISTER_UPLOAD_URL: &str = "https://api.linkedin.com/v2/assets?action=registerUpload";

pub struct LinkedInClient {
    http: reqwest::Client,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct UserInfo {
    pub sub: String,
    pub name: Option<String>,
    pub email: Option<String>,
    pub picture: Option<String>,
}

impl LinkedInClient {
    pub fn new() -> Self {
        Self {
            http: reqwest::Client::new(),
        }
    }

    async fn get_token(&self) -> Result<TokenData, LinkedInError> {
        auth::get_valid_token().await
    }

    pub async fn get_profile(&self) -> Result<UserInfo, LinkedInError> {
        let token = self.get_token().await?;

        let resp = self
            .http
            .get(USERINFO_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let info: UserInfo = resp.json().await?;

        // Update stored token with profile info
        let mut tokens = token;
        tokens.linkedin_id = Some(info.sub.clone());
        tokens.email = info.email.clone();
        auth::save_tokens(&tokens).await?;

        Ok(info)
    }

    pub async fn create_text_post(
        &self,
        text: &str,
        visibility: &str,
    ) -> Result<String, LinkedInError> {
        let token = self.get_token().await?;
        let person_id = self.resolve_person_id(&token).await?;

        let body = json!({
            "author": format!("urn:li:person:{person_id}"),
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": text
                    },
                    "shareMediaCategory": "NONE"
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": visibility
            }
        });

        let resp = self
            .http
            .post(UGC_POSTS_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let result: serde_json::Value = resp.json().await?;
        let post_urn = result["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(post_urn)
    }

    pub async fn create_image_post(
        &self,
        text: &str,
        image_path: &str,
        image_alt: Option<&str>,
        visibility: &str,
    ) -> Result<String, LinkedInError> {
        let token = self.get_token().await?;
        let person_id = self.resolve_person_id(&token).await?;
        let author = format!("urn:li:person:{person_id}");

        // Step 1: Register upload
        let register_body = json!({
            "registerUploadRequest": {
                "recipes": ["urn:li:digitalmediaRecipe:feedshare-image"],
                "owner": &author,
                "serviceRelationships": [{
                    "relationshipType": "OWNER",
                    "identifier": "urn:li:userGeneratedContent"
                }]
            }
        });

        let resp = self
            .http
            .post(REGISTER_UPLOAD_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .json(&register_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: format!("Register upload failed: {body}"),
            });
        }

        let register_result: serde_json::Value = resp.json().await?;
        let upload_url = register_result["value"]["uploadMechanism"]
            ["com.linkedin.digitalmedia.uploading.MediaUploadHttpRequest"]["uploadUrl"]
            .as_str()
            .ok_or_else(|| LinkedInError::Other("No upload URL in response".into()))?
            .to_string();

        let asset = register_result["value"]["asset"]
            .as_str()
            .ok_or_else(|| LinkedInError::Other("No asset URN in response".into()))?
            .to_string();

        // Step 2: Upload image binary
        let path = std::path::Path::new(image_path);
        if !path.exists() {
            return Err(LinkedInError::FileNotFound(image_path.to_string()));
        }
        let image_bytes = tokio::fs::read(path).await?;

        let resp = self
            .http
            .put(&upload_url)
            .bearer_auth(&token.access_token)
            .header("Content-Type", "application/octet-stream")
            .body(image_bytes)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: format!("Image upload failed: {body}"),
            });
        }

        // Step 3: Create post with image
        let media_entry = json!({
            "status": "READY",
            "description": {
                "text": image_alt.unwrap_or("Image")
            },
            "media": &asset,
            "title": {
                "text": image_alt.unwrap_or("Image")
            }
        });

        let post_body = json!({
            "author": &author,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": text
                    },
                    "shareMediaCategory": "IMAGE",
                    "media": [media_entry]
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": visibility
            }
        });

        let resp = self
            .http
            .post(UGC_POSTS_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&post_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let result: serde_json::Value = resp.json().await?;
        let post_urn = result["id"]
            .as_str()
            .unwrap_or("unknown")
            .to_string();

        Ok(post_urn)
    }

    pub async fn create_article_post(
        &self,
        text: &str,
        url: &str,
        title: Option<&str>,
        description: Option<&str>,
        visibility: &str,
    ) -> Result<String, LinkedInError> {
        let token = self.get_token().await?;
        let person_id = self.resolve_person_id(&token).await?;

        let mut media_entry = json!({
            "status": "READY",
            "originalUrl": url
        });

        if let Some(t) = title {
            media_entry["title"] = json!({"text": t});
        }
        if let Some(d) = description {
            media_entry["description"] = json!({"text": d});
        }

        let body = json!({
            "author": format!("urn:li:person:{person_id}"),
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": text
                    },
                    "shareMediaCategory": "ARTICLE",
                    "media": [media_entry]
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": visibility
            }
        });

        let resp = self
            .http
            .post(UGC_POSTS_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let result: serde_json::Value = resp.json().await?;
        let post_urn = result["id"].as_str().unwrap_or("unknown").to_string();
        Ok(post_urn)
    }

    pub async fn create_video_post(
        &self,
        text: &str,
        video_path: &str,
        title: Option<&str>,
        description: Option<&str>,
        visibility: &str,
    ) -> Result<String, LinkedInError> {
        let token = self.get_token().await?;
        let person_id = self.resolve_person_id(&token).await?;
        let author = format!("urn:li:person:{person_id}");

        // Step 1: Register upload (video recipe)
        let register_body = json!({
            "registerUploadRequest": {
                "recipes": ["urn:li:digitalmediaRecipe:feedshare-video"],
                "owner": &author,
                "serviceRelationships": [{
                    "relationshipType": "OWNER",
                    "identifier": "urn:li:userGeneratedContent"
                }]
            }
        });

        let resp = self
            .http
            .post(REGISTER_UPLOAD_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .json(&register_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: format!("Register video upload failed: {body}"),
            });
        }

        let register_result: serde_json::Value = resp.json().await?;
        let upload_url = register_result["value"]["uploadMechanism"]
            ["com.linkedin.digitalmedia.uploading.MediaUploadHttpRequest"]["uploadUrl"]
            .as_str()
            .ok_or_else(|| LinkedInError::Other("No upload URL in response".into()))?
            .to_string();

        let asset = register_result["value"]["asset"]
            .as_str()
            .ok_or_else(|| LinkedInError::Other("No asset URN in response".into()))?
            .to_string();

        // Step 2: Upload video binary
        let path = std::path::Path::new(video_path);
        if !path.exists() {
            return Err(LinkedInError::FileNotFound(video_path.to_string()));
        }
        let video_bytes = tokio::fs::read(path).await?;

        let resp = self
            .http
            .put(&upload_url)
            .bearer_auth(&token.access_token)
            .header("Content-Type", "application/octet-stream")
            .body(video_bytes)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: format!("Video upload failed: {body}"),
            });
        }

        // Step 3: Create post with video
        let mut media_entry = json!({
            "status": "READY",
            "media": &asset
        });
        if let Some(t) = title {
            media_entry["title"] = json!({"text": t});
        }
        if let Some(d) = description {
            media_entry["description"] = json!({"text": d});
        }

        let post_body = json!({
            "author": &author,
            "lifecycleState": "PUBLISHED",
            "specificContent": {
                "com.linkedin.ugc.ShareContent": {
                    "shareCommentary": {
                        "text": text
                    },
                    "shareMediaCategory": "VIDEO",
                    "media": [media_entry]
                }
            },
            "visibility": {
                "com.linkedin.ugc.MemberNetworkVisibility": visibility
            }
        });

        let resp = self
            .http
            .post(UGC_POSTS_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .header("X-Restli-Protocol-Version", "2.0.0")
            .json(&post_body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let result: serde_json::Value = resp.json().await?;
        let post_urn = result["id"].as_str().unwrap_or("unknown").to_string();
        Ok(post_urn)
    }

    async fn resolve_person_id(&self, token: &TokenData) -> Result<String, LinkedInError> {
        if let Some(ref id) = token.linkedin_id {
            return Ok(id.clone());
        }

        // Fetch profile to get the person ID
        let resp = self
            .http
            .get(USERINFO_URL)
            .bearer_auth(&token.access_token)
            .header("LinkedIn-Version", LINKEDIN_API_VERSION)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status().as_u16();
            let body = resp.text().await.unwrap_or_default();
            return Err(LinkedInError::ApiError {
                status,
                message: body,
            });
        }

        let info: UserInfo = resp.json().await?;
        Ok(info.sub)
    }
}
