use dotenvy_macro::dotenv;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{config::get_refresh_token, discord_api::api_client::TokenData, log::log_error};

use super::client::{ReceiveIPCClient, SendIPCClient};

#[derive(Serialize, Deserialize, Clone)]
pub enum AuthErrorType {
    TokenFetch,
    RefreshToken,
    ConfigRead,
    ConfigSave,
    Decode,
    IpcSend,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AuthError {
    pub error_type: AuthErrorType,
    pub message: String,
}

impl SendIPCClient {
    pub async fn try_reauth(&mut self) -> Result<TokenData, AuthError> {
        let refresh_token: String = match get_refresh_token() {
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

        let tokens = match self.api_client.refresh_discord_token(refresh_token).await {
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

        Ok(tokens)
    }

    pub async fn send_token(&mut self, access_token: String) -> Result<(), AuthError> {
        let auth_payload = serde_json::json!({
            "nonce": Uuid::new_v4().to_string(),
            "cmd": "AUTHENTICATE",
            "args": {
                "access_token": access_token
            }
        });
        if let Err(err) = self.send(auth_payload).await {
            return Err(AuthError {
                error_type: AuthErrorType::IpcSend,
                message: format!("Failed to send access token to ipc.\n{}", err.message),
            });
        }
        Ok(())
    }
}

impl ReceiveIPCClient {
    pub async fn send_auth(&mut self) -> Result<(), AuthError> {
        let client_id = dotenv!("CLIENT_ID");
        let payload = serde_json::json!({
            "nonce": Uuid::new_v4().to_string(),
            "cmd": "AUTHORIZE",
            "args": {
                "client_id": client_id,
                "scopes": ["rpc", "identify"]
            }
        });
        if let Err(err) = self.send(payload).await {
            return Err(AuthError {
                error_type: AuthErrorType::IpcSend,
                message: format!("Failed to send authorization request.\n{}", err.message),
            });
        }
        Ok(())
    }

    pub async fn send_token(&mut self, access_token: String) -> Result<(), AuthError> {
        let auth_payload = serde_json::json!({
            "nonce": Uuid::new_v4().to_string(),
            "cmd": "AUTHENTICATE",
            "args": {
                "access_token": access_token
            }
        });
        if let Err(err) = self.send(auth_payload).await {
            return Err(AuthError {
                error_type: AuthErrorType::IpcSend,
                message: format!("Failed to send access token to ipc.\n{}", err.message),
            });
        }
        Ok(())
    }
}
