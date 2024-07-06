use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tauri::async_runtime::Mutex;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Clone)]
pub enum IpcErrorType {
    CreateClient,
    Connect,
    Authorize,
    Subscribe,
    EventReceive,
    EventSend,
    EventEncode,
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

pub async fn subscribe(
    client: &Mutex<DiscordIpcClient>,
    event_name: &str,
    args: Value,
) -> Result<(), InternalIpcError> {
    let payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "SUBSCRIBE",
        "evt": event_name,
        "args": args
    });
    if let Err(err) = client.lock().await.send(payload.clone(), 1) {
        return Err(InternalIpcError {
            error_type: IpcErrorType::Subscribe,
            message: format!("Failed to subscribe event: {}", event_name),
            internal: err.to_string(),
            payload: Some(payload),
        });
    }
    Ok(())
}

pub async fn unsubscribe(
    client: &Mutex<DiscordIpcClient>,
    event_name: &str,
    args: Value,
) -> Result<(), InternalIpcError> {
    let payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "UNSUBSCRIBE",
        "evt": event_name,
        "args": args
    });
    if let Err(err) = client.lock().await.send(payload.clone(), 1) {
        return Err(InternalIpcError {
            error_type: IpcErrorType::Subscribe,
            message: format!("Failed to unsubscribe event: {}", event_name),
            internal: err.to_string(),
            payload: Some(payload),
        });
    }
    Ok(())
}

pub async fn set_vc_events(
    client: &Mutex<DiscordIpcClient>,
    channel_id: Value,
    is_subscribe: bool,
) -> Result<(), InternalIpcError> {
    if is_subscribe {
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_CREATE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_UPDATE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_DELETE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }

        if let Err(err) =
            subscribe(client, "SPEAKING_START", json!({"channel_id": channel_id})).await
        {
            return Err(err);
        }
        if let Err(err) =
            subscribe(client, "SPEAKING_STOP", json!({"channel_id": channel_id})).await
        {
            return Err(err);
        }
    } else {
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_CREATE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_UPDATE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_DELETE",
            json!({"channel_id": channel_id}),
        )
        .await
        {
            return Err(err);
        }

        if let Err(err) =
            unsubscribe(client, "SPEAKING_START", json!({"channel_id": channel_id})).await
        {
            return Err(err);
        }
        if let Err(err) =
            unsubscribe(client, "SPEAKING_STOP", json!({"channel_id": channel_id})).await
        {
            return Err(err);
        }
    }

    Ok(())
}
