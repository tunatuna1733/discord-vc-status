// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod client;
mod config;
mod ipc;
mod log;

use auth::{try_reauth, AuthError, AuthErrorType};
use client::IPCClientManager;
use config::Config;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use dotenvy_macro::{self, dotenv};
use ipc::IpcError;
use log::log_error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum_macros::Display;
use tauri::{Manager, State, Window};
use uuid::Uuid;

use crate::auth::{fetch_discord_token, send_auth, send_token};
use crate::config::set_config;
use crate::ipc::{set_vc_events, subscribe, IpcErrorType};

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
    cm: State<'_, IPCClientManager>,
    handle: tauri::AppHandle,
) -> Result<(), IpcError> {
    // connect to ipc
    if let Err(err) = cm.client.lock().await.connect() {
        return Err(IpcError {
            error_type: IpcErrorType::Connect,
            message: err.to_string(),
            payload: None,
        });
    }

    // reauth --------------------------------
    let refresh_token = match try_reauth(&cm.client).await {
        Ok(t) => t,
        Err(err) => {
            // if the reauth failed, it tries to do the normal auth
            emit_event(&window, EventName::Error, err);
            if let Err(err) = send_auth(&cm.client).await {
                emit_event(&window, EventName::Error, err);
            }
            "".to_string()
        }
    };

    if let Err(err) = set_config(Config {
        refresh_token: refresh_token.to_string(),
    }) {
        emit_event(
            &window,
            EventName::Error,
            AuthError {
                error_type: AuthErrorType::ConfigSave,
                message: err.to_string(),
            },
        );
    }

    // subscribe and emit events
    tauri::async_runtime::spawn(async move {
        let client_manager = handle.state::<IPCClientManager>();
        let mut current_state = CurrentState {
            channel_id: Value::Null,
            user_id: Value::Null,
        };
        loop {
            let (_opcode, payload) = match client_manager.client.lock().await.recv() {
                Ok(res) => res,
                Err(err) => {
                    println!("Event Receive Error\n{}", err);
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
                    let tokens = match fetch_discord_token(code).await {
                        Ok(t) => t,
                        Err(err) => {
                            emit_event(&window, EventName::CriticalError, err);
                            continue;
                        }
                    };
                    // save refresh token
                    if let Err(err) = set_config(Config {
                        refresh_token: tokens.refresh_token,
                    }) {
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
                    if let Err(err) = send_token(&client_manager.client, tokens.access_token).await
                    {
                        emit_event(&window, EventName::CriticalError, err);
                    }
                } else if payload["cmd"] == "AUTHENTICATE" {
                    current_state.user_id = payload["data"]["user"]["id"].clone();
                    // subscribe events after authentication was done
                    if let Err(err) =
                        subscribe(&client_manager.client, "VOICE_SETTINGS_UPDATE", json!({})).await
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
                    if let Err(err) =
                        subscribe(&client_manager.client, "VOICE_CHANNEL_SELECT", json!({})).await
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
                    if let Err(_) = client_manager
                        .client
                        .lock()
                        .await
                        .send(current_state_payload, 1)
                    {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            IpcError {
                                error_type: IpcErrorType::EventSend,
                                message: "Failed to send GET_SELECTED_VOICE_CHANNEL request."
                                    .to_string(),
                                payload: None,
                            },
                        );
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

                        if let Err(err) =
                            set_vc_events(&client_manager.client, current_state.channel_id, true)
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
                    println!("IPC Event: {}", payload);
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
                                if let Err(err) = set_vc_events(
                                    &client_manager.client,
                                    current_state.channel_id,
                                    false,
                                )
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
                            if let Err(_) = client_manager.client.lock().await.send(
                                json!({
                                    "nonce": Uuid::new_v4().to_string(),
                                    "cmd": "GET_SELECTED_VOICE_CHANNEL"
                                }),
                                1,
                            ) {
                                emit_event(
                                    &window,
                                    EventName::Error,
                                    IpcError {
                                        error_type: IpcErrorType::EventSend,
                                        message:
                                            "Failed to send GET_SELECTED_VOICE_CHANNEL request."
                                                .to_string(),
                                        payload: None,
                                    },
                                );
                                continue;
                            }

                            // subscribe events for current channel
                            if let Err(err) = set_vc_events(
                                &client_manager.client,
                                current_state.channel_id,
                                true,
                            )
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

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![connect_ipc])
        .setup(|app| {
            // create ipc client
            let client_id = dotenv!("CLIENT_ID");
            /*
            let mut client = match DiscordIpcClient::new(client_id) {
                Ok(c) => c,
                Err(err) => {
                    Err(IpcError {
                        error_type: IpcErrorType::CreateClient,
                        message: err.to_string(),
                        payload: None,
                    });
                }
            };
            */
            let client = IPCClientManager::new(
                DiscordIpcClient::new(client_id).expect("Failed to create ipc client"),
            );
            app.manage(client);
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO
// refactor event processing for readability
// there is a rare case which fails receiving socket data every time :(
// make client to state so that it can be accessed through app
