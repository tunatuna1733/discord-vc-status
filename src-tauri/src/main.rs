// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod auth;
mod config;
mod ipc;
mod log;

use auth::try_reauth;
use config::Config;
use discord_rich_presence::{DiscordIpc, DiscordIpcClient};
use dotenvy_macro::{self, dotenv};
use log::log_error;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use strum_macros::Display;
use tauri::Window;
use uuid::Uuid;

use crate::auth::{fetch_discord_token, send_auth, send_token};
use crate::config::set_config;
use crate::ipc::{set_vc_events, subscribe, IpcErrorType};

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[derive(Serialize, Deserialize)]
struct IpcError {
    error_type: IpcErrorType,
    message: String,
    pub payload: Option<Value>,
}

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
}

fn emit_event(window: &Window, event_name: EventName, payload: Value) -> () {
    let event = event_name.to_string();
    if let Err(err) = &window.emit(&event, payload.clone()) {
        log_error(
            "Emit Event Error".to_string(),
            format!("Error while emitting {event_name} event\n{payload}\n{err}"),
        );
    }
}

struct CurrentState {
    channel_id: Value,
}

#[derive(Debug, Deserialize, Serialize)]
struct TokenResponse {
    success: bool,
    access_token: Option<String>,
    refresh_token: Option<String>,
}

#[tauri::command]
async fn connect_ipc(window: Window) -> Result<(), IpcError> {
    // create ipc client
    let client_id = dotenv!("CLIENT_ID");
    let mut client = match DiscordIpcClient::new(client_id) {
        Ok(c) => c,
        Err(err) => {
            return Err(IpcError {
                error_type: IpcErrorType::CreateClient,
                message: err.to_string(),
                payload: None,
            });
        }
    };

    // connect to ipc
    if let Err(err) = client.connect() {
        return Err(IpcError {
            error_type: IpcErrorType::Connect,
            message: err.to_string(),
            payload: None,
        });
    }

    // reauth --------------------------------
    let refresh_token = match try_reauth(&mut client).await {
        Ok(t) => t,
        Err(err) => {
            // if the reauth failed, it tries to do the normal auth
            emit_event(
                &window,
                EventName::Error,
                json!({"event_name": err.error_type, "detail": err.message}),
            );
            if let Err(err) = send_auth(&mut client) {
                emit_event(
                    &window,
                    EventName::Error,
                    json!({"event_name": err.error_type, "detail": err.message}),
                );
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
            json!({"event_name": "SAVE", "detail": format!("Failed to save refresh token.\n{}", err.to_string())}),
        );
    }

    // subscribe and emit events
    tauri::async_runtime::spawn(async move {
        let mut current_state = CurrentState {
            channel_id: Value::Null,
        };
        loop {
            let (_opcode, payload) = match client.recv() {
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
                            emit_event(
                                &window,
                                EventName::CriticalError,
                                json!({"event_name": err.error_type, "detail": err.message}),
                            );
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
                            json!({"event_name": "SAVE", "detail": format!("Failed to save refresh token.\n{}", err.to_string())}),
                        );
                    }
                    // send token to ipc
                    if let Err(err) = send_token(&mut client, tokens.access_token) {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            json!({"event_name": err.error_type, "detail": err.message}),
                        );
                    }
                } else if payload["cmd"] == "AUTHENTICATE" {
                    // subscribe events after authentication was done
                    if let Err(err) = subscribe(&mut client, "VOICE_SETTINGS_UPDATE", json!({})) {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            json!({"event_name": "VOICE_SETTINGS_UPDATE", "detail": err}),
                        );
                    }
                    if let Err(err) = subscribe(&mut client, "VOICE_CHANNEL_SELECT", json!({})) {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            json!({"event_name": "VOICE_CHANNEL_SELECT", "detail": err}),
                        );
                    }
                    // get the current voice channel
                    let current_state_payload = json!({
                        "nonce": Uuid::new_v4().to_string(),
                        "cmd": "GET_SELECTED_VOICE_CHANNEL"
                    });
                    if let Err(_) = client.send(current_state_payload, 1) {
                        emit_event(
                            &window,
                            EventName::CriticalError,
                            json!({"event_name": "GET_SELECTED_VOICE_CHANNEL", "detail": "Could not get initial state."}),
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
                            EventName::VCInfo,
                            json!({
                                "name": payload["data"]["name"],
                                "users": payload["data"]["voice_states"]
                            }),
                        );
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
                            json!({"event_name": "AUTHORIZE", "detail": "User cancelled the authorization."}),
                        );
                        continue;
                    }
                    if payload["cmd"] == "SUBSCRIBE" {
                        // client failed to subscribe events
                        emit_event(
                            &window,
                            EventName::Error,
                            json!({"event_name": "SUBSCRIBE", "detail": "Could not subscribe event."}),
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
                                if let Err(err) =
                                    set_vc_events(&mut client, current_state.channel_id, false)
                                {
                                    emit_event(
                                        &window,
                                        EventName::Error,
                                        json!({
                                            "event_name": "error",
                                            "detail": err
                                        }),
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
                            if let Err(_) = client.send(
                                json!({
                                    "nonce": Uuid::new_v4().to_string(),
                                    "cmd": "GET_SELECTED_VOICE_CHANNEL"
                                }),
                                1,
                            ) {
                                emit_event(
                                    &window,
                                    EventName::Error,
                                    json!({"event_name": "GET_SELECTED_VOICE_CHANNEL", "detail": "Could not get vc state."}),
                                );
                                continue;
                            }

                            // subscribe events for current channel
                            if let Err(err) =
                                set_vc_events(&mut client, current_state.channel_id, true)
                            {
                                emit_event(
                                    &window,
                                    EventName::Error,
                                    json!({
                                        "event_name": "error",
                                        "detail": err
                                    }),
                                );
                            }
                        }
                    } else if payload["evt"] == "VOICE_STATE_CREATE" {
                    } else if payload["evt"] == "VOICE_STATE_UPDATE" {
                    } else if payload["evt"] == "VOICE_STATE_DELETE" {
                    }
                }
            }
        }
    });

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![greet, connect_ipc])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

// TODO
// need to implement voice state events
// refactor event processing for readability
// there is a rare case which fails receiving socket data every time :(
// makes the error which is emit to have `IpcError` structure
