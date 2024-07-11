// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod discord_api;
mod ipc;
mod log;

use ipc::{
    auth::{AuthError, AuthErrorType},
    client::{IpcError, IpcErrorType, ReceiveIPCClient, SendIPCClient},
};
use std::{process, sync::Arc};
use tauri::async_runtime::Mutex;

use config::save_refresh_token;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use dotenvy_macro::{self, dotenv};

use log::log_error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum_macros::Display;
use tauri::{Manager, State, Window};
use uuid::Uuid;

#[derive(Display)]
enum EventName {
    #[strum(to_string = "error")]
    Error,
    #[strum(to_string = "critical_error")]
    CriticalError,
    #[strum(to_string = "vc_select")]
    VCSelect,
    #[strum(to_string = "vc_info")]
    VCInfo,
    #[strum(to_string = "vc_mute_update")]
    VCMuteUpdate,
    #[strum(to_string = "vc_user")]
    VCUser,
    #[strum(to_string = "vc_speak")]
    VCSpeak,
}

fn emit_event<S: Serialize + Clone>(window: &Window, event_name: EventName, payload: S) -> () {
    let event = event_name.to_string();
    if let Err(err) = &window.emit(&event, payload) {
        log_error(
            "Emit Event Error".to_string(),
            format!("Error while emitting {event_name} event.\n{err}"),
        );
    }
}

struct CurrentState {
    channel_id: Value,
    user_id: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    success: bool,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

#[tauri::command]
async fn connect_ipc(
    window: Window,
    send_client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
    reauth: bool,
) -> Result<(), IpcError> {
    let client_id = dotenv!("CLIENT_ID");
    let client = match DiscordIpcClient::new(client_id) {
        Ok(c) => c,
        Err(err) => {
            return Err(IpcError {
                error_type: IpcErrorType::CreateClient,
                message: err.to_string(),
                payload: None,
            });
        }
    };
    let mut receive_client = ReceiveIPCClient::new(client);

    // reauth --------------------------------
    if reauth {
        let refreshed_data = match send_client_manager.lock().await.try_reauth().await {
            Ok(t) => t,
            Err(err) => {
                // if the reauth failed, it tries to do the normal auth
                return Err(IpcError {
                    error_type: IpcErrorType::ReAuth,
                    message: format!("Failed to reauth.\n{}", err.message),
                    payload: None,
                });
            }
        };

        // connect to ipc
        if let Err(err) = receive_client.ipc_client.connect() {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Connect,
                message: err.to_string(),
                payload: None,
            });
        }
        if let Err(err) = send_client_manager.lock().await.ipc_client.connect() {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Connect,
                message: err.to_string(),
                payload: None,
            });
        }

        if let Err(err) = receive_client
            .send_token(refreshed_data.access_token.clone())
            .await
        {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Authorize,
                message: err.message,
                payload: None,
            });
        }

        if let Err(err) = send_client_manager
            .lock()
            .await
            .send_token(refreshed_data.access_token)
            .await
        {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Authorize,
                message: err.message,
                payload: None,
            });
        }

        if let Err(err) = save_refresh_token(refreshed_data.refresh_token.to_string()) {
            emit_event(
                &window,
                EventName::Error,
                AuthError {
                    error_type: AuthErrorType::ConfigSave,
                    message: err.to_string(),
                },
            );
        }
    } else {
        // connect to ipc
        if let Err(err) = receive_client.ipc_client.connect() {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Connect,
                message: err.to_string(),
                payload: None,
            });
        }
        if let Err(err) = send_client_manager.lock().await.ipc_client.connect() {
            let _ = receive_client.ipc_client.close();
            return Err(IpcError {
                error_type: IpcErrorType::Connect,
                message: err.to_string(),
                payload: None,
            });
        }

        if let Err(err) = receive_client.send_auth().await {
            return Err(IpcError {
                error_type: IpcErrorType::Authorize,
                message: err.message,
                payload: None,
            });
        }
    }

    let send_client = Arc::clone(&send_client_manager);
    // subscribe and emit events
    tauri::async_runtime::spawn(async move {
        let mut current_state = CurrentState {
            channel_id: Value::Null,
            user_id: Value::Null,
        };
        loop {
            let (_opcode, payload) = match receive_client.ipc_client.recv() {
                Ok(res) => res,
                Err(err) => {
                    emit_event(&window, EventName::CriticalError, err.to_string());
                    continue;
                }
            };
            println!("{}", payload);
            if payload["evt"].is_null() {
                if payload["cmd"] == "AUTHORIZE" {
                    // authorization succeeded
                    // this flow is only used in initial authentication or reauth failed
                    let code = payload["data"]["code"].as_str().unwrap();
                    // start authentication

                    // fetch access token
                    let tokens = match send_client
                        .lock()
                        .await
                        .api_client
                        .fetch_discord_token(code)
                        .await
                    {
                        Ok(t) => t,
                        Err(err) => {
                            emit_event(&window, EventName::CriticalError, err);
                            continue;
                        }
                    };
                    // save refresh token
                    if let Err(err) = save_refresh_token(tokens.refresh_token) {
                        emit_event(
                            &window,
                            EventName::Error,
                            AuthError {
                                error_type: AuthErrorType::ConfigSave,
                                message: err.to_string(),
                            },
                        );
                    }
                    // send token to ipc
                    if let Err(err) = receive_client.send_token(tokens.access_token.clone()).await {
                        emit_event(&window, EventName::CriticalError, err);
                    }
                    if let Err(err) = send_client
                        .lock()
                        .await
                        .send_token(tokens.access_token)
                        .await
                    {
                        emit_event(&window, EventName::CriticalError, err);
                    }
                } else if payload["cmd"] == "AUTHENTICATE" {
                    current_state.user_id = payload["data"]["user"]["id"].clone();
                    // subscribe events after authentication was done
                    if let Err(err) = receive_client
                        .subscribe("VOICE_SETTINGS_UPDATE", json!({}), true)
                        .await
                    {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            IpcError {
                                error_type: err.error_type,
                                message: err.message,
                                payload: err.payload,
                            },
                        );
                    }
                    if let Err(err) = receive_client
                        .subscribe("VOICE_CHANNEL_SELECT", json!({}), true)
                        .await
                    {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            IpcError {
                                error_type: err.error_type,
                                message: err.message,
                                payload: err.payload,
                            },
                        );
                    }
                    // get the current voice channel
                    let current_state_payload = json!({
                        "nonce": Uuid::new_v4().to_string(),
                        "cmd": "GET_SELECTED_VOICE_CHANNEL"
                    });
                    if let Err(err) = receive_client.send(current_state_payload).await {
                        emit_event(&window, EventName::CriticalError, err);
                        continue;
                    }
                } else if payload["cmd"] == "GET_SELECTED_VOICE_CHANNEL" {
                    if payload["data"].is_null() {
                        // not currently in vc
                        current_state.channel_id = Value::Null;
                        emit_event(
                            &window,
                            EventName::VCSelect,
                            json!({
                                "in_vc": false
                            }),
                        );
                    } else {
                        // in vc
                        current_state.channel_id = payload["data"]["id"].clone();
                        emit_event(
                            &window,
                            EventName::VCSelect,
                            json!({
                                "in_vc": true
                            }),
                        );
                        emit_event(
                            &window,
                            EventName::VCInfo,
                            json!({
                                "name": payload["data"]["name"],
                                "users": payload["data"]["voice_states"]
                            }),
                        );

                        if let Err(err) = receive_client
                            .set_vc_events(current_state.channel_id, true)
                            .await
                        {
                            emit_event(
                                &window,
                                EventName::Error,
                                IpcError {
                                    error_type: err.error_type,
                                    message: err.message,
                                    payload: err.payload,
                                },
                            );
                        }
                    }
                }
            } else {
                if payload["evt"] == "ERROR" {
                    // error occurred
                    if payload["cmd"] == "AUTHORIZE" {
                        // authorization error (user pressed cancel button)
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            IpcError {
                                error_type: IpcErrorType::Authorize,
                                message: "User cancelled the app authorization.".to_string(),
                                payload: None,
                            },
                        );
                        continue;
                    }
                    if payload["cmd"] == "SUBSCRIBE" {
                        // client failed to subscribe events
                        emit_event(
                            &window,
                            EventName::Error,
                            IpcError {
                                error_type: IpcErrorType::Subscribe,
                                message: "Failed to subscribe to event.".to_string(),
                                payload: None, // TODO
                            },
                        );
                        continue;
                    }
                } else if payload["cmd"] == "DISPATCH" {
                    if payload["evt"] == "VOICE_SETTINGS_UPDATE" {
                        // vc settings update event
                        emit_event(
                            &window,
                            EventName::VCMuteUpdate,
                            json!({
                                "mute": payload["data"]["mute"],
                                "deaf": payload["data"]["deaf"],
                            }),
                        );
                    } else if payload["evt"] == "VOICE_CHANNEL_SELECT" {
                        // vc select update event
                        let channel_id = &payload["data"]["channel_id"];
                        if channel_id.is_null() {
                            // left vc
                            current_state.channel_id = Value::Null;
                            let vc_select_payload = json!({
                                "in_vc": false
                            });
                            emit_event(&window, EventName::VCSelect, vc_select_payload);
                            if current_state.channel_id.is_null() {
                                // unsubscribe events
                                if let Err(err) = receive_client
                                    .set_vc_events(current_state.channel_id, false)
                                    .await
                                {
                                    emit_event(
                                        &window,
                                        EventName::Error,
                                        IpcError {
                                            error_type: err.error_type,
                                            message: err.message,
                                            payload: err.payload,
                                        },
                                    );
                                }
                            }
                        } else {
                            // joined vc
                            current_state.channel_id = json!(channel_id);
                            let vc_select_payload = json!({
                                "in_vc": true,
                            });
                            emit_event(&window, EventName::VCSelect, vc_select_payload);

                            // send current vc status command
                            if let Err(err) = receive_client
                                .send(json!({
                                    "nonce": Uuid::new_v4().to_string(),
                                    "cmd": "GET_SELECTED_VOICE_CHANNEL"
                                }))
                                .await
                            {
                                emit_event(&window, EventName::Error, err);
                                continue;
                            }

                            /*
                            // subscribe events for current channel
                            if let Err(err) = receive_client
                                .set_vc_events(current_state.channel_id, true)
                                .await
                            {
                                emit_event(&window, EventName::Error, err);
                            }
                             */
                        }
                    } else if payload["evt"] == "VOICE_STATE_CREATE" {
                        // someone joined vc
                        if payload["data"]["user"]["id"].to_string()
                            != current_state.user_id.to_string()
                        {
                            emit_event(
                                &window,
                                EventName::VCUser,
                                json!({
                                    "event": "JOIN",
                                    "data": {
                                        "id": payload["data"]["user"]["id"],
                                        "username": payload["data"]["user"]["username"],
                                        "avatar": payload["data"]["user"]["avatar"],
                                        "nick": payload["data"]["nick"],
                                        "mute": payload["data"]["voice_state"]["mute"],
                                        "self_mute": payload["data"]["voice_state"]["self_mute"],
                                        "deaf": payload["data"]["voice_state"]["deaf"],
                                        "self_deaf": payload["data"]["voice_state"]["self_deaf"],
                                    }
                                }),
                            );
                        }
                    } else if payload["evt"] == "VOICE_STATE_UPDATE" {
                        if payload["data"]["user"]["id"].to_string()
                            != current_state.user_id.to_string()
                        {
                            emit_event(
                                &window,
                                EventName::VCUser,
                                json!({
                                    "event": "UPDATE",
                                    "data": {
                                        "id": payload["data"]["user"]["id"],
                                        "username": payload["data"]["user"]["username"],
                                        "avatar": payload["data"]["user"]["avatar"],
                                        "nick": payload["data"]["nick"],
                                        "mute": payload["data"]["voice_state"]["mute"],
                                        "self_mute": payload["data"]["voice_state"]["self_mute"],
                                        "deaf": payload["data"]["voice_state"]["deaf"],
                                        "self_deaf": payload["data"]["voice_state"]["self_deaf"],
                                    }
                                }),
                            );
                        }
                    } else if payload["evt"] == "VOICE_STATE_DELETE" {
                        if payload["data"]["user"]["id"].to_string()
                            != current_state.user_id.to_string()
                        {
                            emit_event(
                                &window,
                                EventName::VCUser,
                                json!({
                                    "event": "LEAVE",
                                    "data": {
                                        "id": payload["data"]["user"]["id"]
                                    }
                                }),
                            );
                        }
                    } else if payload["evt"] == "SPEAKING_START" {
                        if payload["data"]["user_id"].to_string()
                            == current_state.user_id.to_string()
                        {
                            emit_event(
                                &window,
                                EventName::VCSpeak,
                                json!({
                                    "user_id": payload["data"]["user_id"],
                                    "is_me": true,
                                    "speaking": true
                                }),
                            );
                        } else {
                            emit_event(
                                &window,
                                EventName::VCSpeak,
                                json!({
                                    "user_id": payload["data"]["user_id"],
                                    "is_me": false,
                                    "speaking": true
                                }),
                            );
                        }
                    } else if payload["evt"] == "SPEAKING_STOP" {
                        if payload["data"]["user_id"].to_string()
                            == current_state.user_id.to_string()
                        {
                            emit_event(
                                &window,
                                EventName::VCSpeak,
                                json!({
                                    "user_id": payload["data"]["user_id"],
                                    "is_me": true,
                                    "speaking": false
                                }),
                            );
                        } else {
                            emit_event(
                                &window,
                                EventName::VCSpeak,
                                json!({
                                    "user_id": payload["data"]["user_id"],
                                    "is_me": false,
                                    "speaking": false
                                }),
                            );
                        }
                    }
                }
            }
        }
    });

    Ok(())
}

#[tauri::command]
async fn disconnect_ipc(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    if let Err(err) = client.lock().await.ipc_client.close() {
        return Err(IpcError {
            error_type: IpcErrorType::Connect,
            message: format!("Failed to close ipc socket.\n{}", err.to_string()),
            payload: None,
        });
    }
    Ok(())
}

#[tauri::command]
async fn disconnect_vc(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    let payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "SELECT_VOICE_CHANNEL",
        "args": {
            "channel_id": Value::Null
        }
    });
    if let Err(err) = client.lock().await.send(payload.clone()).await {
        return Err(err);
    }
    Ok(())
}

#[tauri::command]
async fn toggle_mute(client_manager: State<'_, Arc<Mutex<SendIPCClient>>>) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    let get_payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "GET_VOICE_SETTINGS"
    });
    let response = match client.lock().await.send(get_payload).await {
        Ok(r) => r,
        Err(err) => {
            return Err(err);
        }
    };
    let mute = response["data"]["mute"].as_bool().unwrap();
    let set_payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "SET_VOICE_SETTINGS",
        "args": {
            "mute": !mute
        }
    });
    if let Err(err) = client.lock().await.send(set_payload).await {
        return Err(err);
    };
    Ok(())
}

#[tauri::command]
async fn toggle_deafen(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    let get_payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "GET_VOICE_SETTINGS"
    });
    let response = match client.lock().await.send(get_payload).await {
        Ok(r) => r,
        Err(err) => {
            return Err(err);
        }
    };
    let deaf = response["data"]["deaf"].as_bool().unwrap();
    let set_payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "SET_VOICE_SETTINGS",
        "args": {
            "deaf": !deaf
        }
    });
    if let Err(err) = client.lock().await.send(set_payload).await {
        return Err(err);
    };
    Ok(())
}

#[tauri::command]
async fn get_vc_info(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
) -> Result<Value, IpcError> {
    let client = Arc::clone(&client_manager);
    let payload = json!({
        "nonce": Uuid::new_v4().to_string(),
        "cmd": "GET_SELECTED_VOICE_CHANNEL"
    });
    let response = match client.lock().await.send(payload).await {
        Ok(r) => r,
        Err(err) => {
            return Err(err);
        }
    };
    if response["data"].is_null() {
        // not currently in vc
        Ok(json!({
            "in_vc": false
        }))
    } else {
        // in vc
        Ok(json!({
            "in_vc": true,
            "name": response["data"]["name"],
            "users": response["data"]["voice_states"]
        }))
    }
}

#[tauri::command]
async fn set_activity(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
    activity: Value,
) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    let nonce = Uuid::new_v4().to_string();
    let payload = json!({
        "nonce": nonce.clone(),
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": process::id(),
            "activity": activity
        }
    });
    let response = match client.lock().await.send(payload.clone()).await {
        Ok(r) => r,
        Err(err) => {
            return Err(err);
        }
    };
    if response["evt"] == "ERROR" && response["nonce"] == nonce {
        return Err(IpcError {
            error_type: IpcErrorType::EventSend,
            message: format!("Failed to set activity.\n{}", payload["data"]["message"]),
            payload: Some(payload),
        });
    }
    Ok(())
}

#[tauri::command]
async fn clear_activity(
    client_manager: State<'_, Arc<Mutex<SendIPCClient>>>,
) -> Result<(), IpcError> {
    let client = Arc::clone(&client_manager);
    let nonce = Uuid::new_v4().to_string();
    let payload = json!({
        "nonce": nonce.clone(),
        "cmd": "SET_ACTIVITY",
        "args": {
            "pid": process::id(),
        }
    });
    let response = match client.lock().await.send(payload.clone()).await {
        Ok(r) => r,
        Err(err) => {
            return Err(err);
        }
    };
    if response["evt"] == "ERROR" && response["nonce"] == nonce {
        return Err(IpcError {
            error_type: IpcErrorType::EventSend,
            message: format!("Failed to clear activity.\n{}", payload["data"]["message"]),
            payload: Some(payload),
        });
    }
    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .invoke_handler(tauri::generate_handler![
            connect_ipc,
            disconnect_vc,
            toggle_mute,
            toggle_deafen,
            disconnect_ipc,
            get_vc_info,
            set_activity,
            clear_activity
        ])
        .setup(|app| {
            // create ipc client
            let client_id = dotenv!("CLIENT_ID");
            /*

            */
            let client = Arc::new(Mutex::from(SendIPCClient::new(
                DiscordIpcClient::new(client_id).expect("Failed to create client"),
            )));
            app.manage(client);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO
// refactor event processing for readability
// there is a rare case which fails receiving socket data every time :(
