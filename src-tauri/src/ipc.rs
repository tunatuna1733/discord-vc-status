use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

#[derive(Serialize, Deserialize)]
pub enum IpcErrorType {
    CreateClient,
    Connect,
    Authorize,
    Subscribe,
    EventReceive,
    EventEncode,
}

#[derive(Serialize, Deserialize)]
pub struct InternalIpcError {
    pub error_type: IpcErrorType,
    pub message: String,
    pub internal: String,
}

pub fn subscribe(
    client: &mut DiscordIpcClient,
    event_name: &str,
    args: Value,
) -> Result<(), InternalIpcError> {
    let payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "SUBSCRIBE",
        "evt": event_name,
        "args": args
    });
    if let Err(err) = client.send(payload, 1) {
        return Err(InternalIpcError {
            error_type: IpcErrorType::Subscribe,
            message: format!("Failed to subscribe event: {}", event_name),
            internal: err.to_string(),
        });
    }
    Ok(())
}

pub fn unsubscribe(
    client: &mut DiscordIpcClient,
    event_name: &str,
    args: Value,
) -> Result<(), InternalIpcError> {
    let payload = serde_json::json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "UNSUBSCRIBE",
        "evt": event_name,
        "args": args
    });
    if let Err(err) = client.send(payload, 1) {
        return Err(InternalIpcError {
            error_type: IpcErrorType::Subscribe,
            message: format!("Failed to unsubscribe event: {}", event_name),
            internal: err.to_string(),
        });
    }
    Ok(())
}

pub fn set_vc_events(
    client: &mut DiscordIpcClient,
    channel_id: Value,
    is_subscribe: bool,
) -> Result<(), InternalIpcError> {
    if is_subscribe {
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_CREATE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_UPDATE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }
        if let Err(err) = subscribe(
            client,
            "VOICE_STATE_DELETE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }

        if let Err(err) = subscribe(client, "SPEAKING_START", json!({"channel_id": channel_id})) {
            return Err(err);
        }
        if let Err(err) = subscribe(client, "SPEAKING_STOP", json!({"channel_id": channel_id})) {
            return Err(err);
        }
    } else {
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_CREATE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_UPDATE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }
        if let Err(err) = unsubscribe(
            client,
            "VOICE_STATE_DELETE",
            json!({"channel_id": channel_id}),
        ) {
            return Err(err);
        }

        if let Err(err) = unsubscribe(client, "SPEAKING_START", json!({"channel_id": channel_id})) {
            return Err(err);
        }
        if let Err(err) = unsubscribe(client, "SPEAKING_STOP", json!({"channel_id": channel_id})) {
            return Err(err);
        }
    }

    Ok(())
}
