use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::discord_api::api_client::DiscordAPIClient;

#[derive(Serialize, Deserialize, Clone)]
pub enum IpcErrorType {
    CreateClient,
    Connect,
    Authorize,
    ReAuth,
    Subscribe,
    Unsubscribe,
    EventReceive,
    EventSend,
    EventEncode,
    EventDecode,
    LeaveVC,
}

#[derive(Serialize, Deserialize)]
pub struct InternalIpcError {
    pub error_type: IpcErrorType,
    pub message: String,
    pub internal: String,
    pub payload: Option<Value>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct IpcError {
    pub error_type: IpcErrorType,
    pub message: String,
    pub payload: Option<Value>,
}

pub struct SendIPCClient {
    pub ipc_client: DiscordIpcClient,
    pub api_client: DiscordAPIClient,
}

pub struct ReceiveIPCClient {
    pub ipc_client: DiscordIpcClient,
}

impl SendIPCClient {
    pub fn new(c: DiscordIpcClient) -> Self {
        Self {
            ipc_client: c,
            api_client: DiscordAPIClient::new(),
        }
    }

    pub async fn send(&mut self, payload: Value) -> Result<Value, IpcError> {
        let nonce = payload["nonce"].clone();
        println!("Sent payload: {}", payload.clone());
        if let Err(err) = self.ipc_client.send(payload.clone(), 1) {
            // error while sending data to discord ipc
            return Err(IpcError {
                error_type: IpcErrorType::EventSend,
                message: format!("Failed to decode response.\n{}", err),
                payload: Some(payload),
            });
        }
        let response = loop {
            let (_opcode, response) = match self.ipc_client.recv() {
                Ok(r) => r,
                Err(err) => {
                    return Err(IpcError {
                        error_type: IpcErrorType::EventDecode,
                        message: format!("Failed to decode response.\n{}", err),
                        payload: Some(payload),
                    });
                }
            };
            if !response["nonce"].is_null() && response["nonce"] == nonce {
                break response;
            }
            return Err(IpcError {
                error_type: IpcErrorType::EventReceive,
                message: format!("Invalid message received.\n{}", response),
                payload: Some(payload),
            });
        };
        Ok(response)
    }
}

impl ReceiveIPCClient {
    pub fn new(c: DiscordIpcClient) -> Self {
        Self { ipc_client: c }
    }

    pub async fn send(&mut self, payload: Value) -> Result<(), IpcError> {
        if let Err(err) = self.ipc_client.send(payload.clone(), 1) {
            // error while sending data to discord ipc
            return Err(IpcError {
                error_type: IpcErrorType::EventSend,
                message: format!("Failed to decode response.\n{}", err),
                payload: Some(payload),
            });
        }
        Ok(())
    }

    pub async fn subscribe(
        &mut self,
        event_name: &str,
        args: Value,
        is_subscribe: bool,
    ) -> Result<(), IpcError> {
        let event_type = if is_subscribe {
            json!("SUBSCRIBE")
        } else {
            json!("UNSUBSCRIBE")
        };
        let payload = serde_json::json!({
            "nonce": Uuid::new_v4().to_string(),
            "cmd": event_type,
            "evt": event_name,
            "args": args
        });
        if let Err(err) = self.send(payload.clone()).await {
            return Err(err);
        };
        Ok(())
    }
}
