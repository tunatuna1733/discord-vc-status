use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use dotenvy_macro::dotenv;
use reqwest::header::{self, HeaderMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::get_config, log::log_error};

#[derive(Serialize, Deserialize)]
pub enum AuthErrorType {
    TokenFetch,
    RefreshToken,
    ConfigRead,
    Decode,
    IpcSend,
}

#[derive(Serialize, Deserialize)]
pub struct AuthError {
    pub error_type: AuthErrorType,
    pub message: String,
}

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

pub fn send_auth(client: &mut DiscordIpcClient) -> Result<(), AuthError> {
    let client_id = dotenv!("CLIENT_ID");
    let payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "AUTHORIZE",
        "args": {
            "client_id": client_id,
            "scopes": ["rpc", "identify"]
        }
    });
    if let Err(err) = client.send(payload, 1) {
        return Err(AuthError {
            error_type: AuthErrorType::IpcSend,
            message: format!("Failed to send authorization request.\n{}", err.to_string()),
        });
    }
    Ok(())
}

pub async fn fetch_discord_token(code: &str) -> Result<TokenData, AuthError> {
    let url = "https://discord.com/api/oauth2/token";
    let rq_client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    let client_id = dotenv!("CLIENT_ID");
    let client_secret = dotenv!("CLIENT_SECRET");
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    let body = format!("grant_type=authorization_code&code={code}&redirect_uri=http://localhost");
    let response = match rq_client
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

pub async fn refresh_discord_token(refresh_token: String) -> Result<TokenData, AuthError> {
    let url = "https://discord.com/api/oauth2/token";
    let rq_client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    let client_id = dotenv!("CLIENT_ID");
    let client_secret = dotenv!("CLIENT_SECRET");
    headers.insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/x-www-form-urlencoded"),
    );
    let body = format!("grant_type=refresh_token&refresh_token={refresh_token}");
    let response = match rq_client
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

pub async fn try_reauth(client: &mut DiscordIpcClient) -> Result<String, AuthError> {
    let config = match get_config() {
        Ok(c) => c,
        Err(err) => {
            // TODO: replace here with send_auth func
            log_error(
                "config".to_string(),
                format!("Could not get config contents.\n{}", err.to_string()),
            );
            return Err(AuthError {
                error_type: AuthErrorType::ConfigRead,
                message: err.to_string(),
            });
        }
    };

    let tokens = match refresh_discord_token(config.refresh_token).await {
        Ok(t) => t,
        Err(err) => {
            log_error(
                "fetch token".to_string(),
                format!("Could not refresh token\n{}", err.message),
            );
            return Err(AuthError {
                error_type: AuthErrorType::RefreshToken,
                message: err.message,
            });
        }
    };

    if let Err(err) = send_token(client, tokens.access_token) {
        return Err(err);
    }

    Ok(tokens.refresh_token)
}

pub fn send_token(client: &mut DiscordIpcClient, access_token: String) -> Result<(), AuthError> {
    let auth_payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "AUTHENTICATE",
        "args": {
            "access_token": access_token
        }
    });
    if let Err(err) = client.send(auth_payload, 1) {
        return Err(AuthError {
            error_type: AuthErrorType::IpcSend,
            message: format!("Failed to send access token to ipc.\n{}", err.to_string()),
        });
    }
    Ok(())
}
