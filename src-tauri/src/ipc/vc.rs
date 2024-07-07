use serde_json::{json, Value};

use super::client::{IpcError, ReceiveIPCClient};

const VC_EVENTS: [&str; 5] = [
    "VOICE_STATE_CREATE",
    "VOICE_STATE_UPDATE",
    "VOICE_STATE_DELETE",
    "SPEAKING_START",
    "SPEAKING_STOP",
];

impl ReceiveIPCClient {
    pub async fn set_vc_events(
        &mut self,
        channel_id: Value,
        is_subscribe: bool,
    ) -> Result<(), IpcError> {
        for event_name in VC_EVENTS {
            if let Err(err) = self
                .subscribe(event_name, json!({"channel_id": channel_id}), is_subscribe)
                .await
            {
                return Err(err);
            }
        }
        Ok(())
    }
}
