use dotenvy_macro::dotenv;
use reqwest::{
    header::{self, HeaderMap},
    Client,
};
use serde::{Deserialize, Serialize};

use crate::ipc::auth::{AuthError, AuthErrorType};

#[derive(Debug, Deserialize, Serialize)]
struct DiscordTokenResponse {
    access_token: String,
    token_type: String,
    expires_in: i32,
    refresh_token: String,
    scope: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
}

pub struct DiscordAPIClient {
    client: Client,
}

impl DiscordAPIClient {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn fetch_discord_token(&self, code: &str) -> Result<TokenData, AuthError> {
        let url = "https://discord.com/api/oauth2/token";
        let mut headers = HeaderMap::new();
        let client_id = dotenv!("CLIENT_ID");
        let client_secret = dotenv!("CLIENT_SECRET");
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let body =
            format!("grant_type=authorization_code&code={code}&redirect_uri=http://localhost");
        let response = match self
            .client
            .post(url)
            .headers(headers)
            .basic_auth(client_id, Some(client_secret))
            .body(body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(err) => {
                return Err(AuthError {
                    error_type: AuthErrorType::TokenFetch,
                    message: format!("Failed to fetch token.\n{}", err.to_string()),
                });
            }
        };

        let response_json: DiscordTokenResponse = match response.text().await {
            Ok(t) => serde_json::from_str(&t).unwrap(),
            Err(err) => {
                return Err(AuthError {
                    error_type: AuthErrorType::TokenFetch,
                    message: format!("Failed to decode token response.\n{}", err.to_string()),
                });
            }
        };

        Ok(TokenData {
            access_token: response_json.access_token,
            refresh_token: response_json.refresh_token,
        })
    }

    pub async fn refresh_discord_token(
        &self,
        refresh_token: String,
    ) -> Result<TokenData, AuthError> {
        let url = "https://discord.com/api/oauth2/token";
        let mut headers = HeaderMap::new();
        let client_id = dotenv!("CLIENT_ID");
        let client_secret = dotenv!("CLIENT_SECRET");
        headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/x-www-form-urlencoded"),
        );
        let body = format!("grant_type=refresh_token&refresh_token={refresh_token}");
        let response = match self
            .client
            .post(url)
            .headers(headers)
            .basic_auth(client_id, Some(client_secret))
            .body(body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(err) => {
                return Err(AuthError {
                    error_type: AuthErrorType::RefreshToken,
                    message: format!("Failed to refresh token.\n{}", err.to_string()),
                });
            }
        };

        match response.error_for_status_ref() {
            Ok(_) => (),
            Err(err) => {
                return Err(AuthError {
                    error_type: AuthErrorType::RefreshToken,
                    message: format!("Failed to refresh token.\n{}", err.to_string()),
                });
            }
        }

        let response_json: DiscordTokenResponse = match response.text().await {
            Ok(t) => serde_json::from_str(&t).unwrap(),
            Err(err) => {
                return Err(AuthError {
                    error_type: AuthErrorType::RefreshToken,
                    message: format!("Failed to decode token response.\n{}", err.to_string()),
                });
            }
        };

        Ok(TokenData {
            access_token: response_json.access_token,
            refresh_token: response_json.refresh_token,
        })
    }
}
