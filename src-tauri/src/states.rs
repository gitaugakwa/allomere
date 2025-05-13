use lazy_static::lazy_static;
use parking_lot::{Mutex, RwLock};
use rodio::cpal::Stream;
use rodio::dynamic_mixer::DynamicMixerController;
use std::sync::Arc;
use tauri::{AppHandle, WebviewWindow, Emitter};

use crate::autogen::constants::STATE_SYNC_EVENT;

pub mod playback;
pub mod window;

lazy_static! {
    pub static ref APP_HANDLE: Mutex<Option<AppHandle>> = Mutex::new(None);
}

// the payload type must implement `Serialize` and `Clone`.
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Payload {
    pub key: String,
    pub value: String,
}

// #[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct GlobalAppState {
    pub tracks: Mutex<Vec<playback::Track>>,
    // pub stream_handle: Mutex<OutputStreamHandle>,
    pub window: Arc<Mutex<Option<WebviewWindow>>>,
}

pub fn set_default_state(window: Arc<Mutex<Option<WebviewWindow>>>) -> GlobalAppState {
    GlobalAppState {
        tracks: Mutex::new(Vec::new()),
        // stream_handle: Mutex::new(stream_handle),
        window,
    }
}

pub fn emit_state_sync<T>(key: &str, value: &T, window: &WebviewWindow) -> serde_json::Result<()>
where
    T: ?Sized + serde::Serialize,
{
    window
        .emit(
            STATE_SYNC_EVENT,
            Payload {
                key: key.into(),
                value: serde_json::to_string(&value)?,
            },
        )
        .unwrap();

    Ok(())
}

pub fn emit_state_sync_handle<T>(
    key: &str,
    value: &T,
    app_handle: &AppHandle,
) -> serde_json::Result<()>
where
    T: ?Sized + serde::Serialize,
{
    app_handle
        .emit(
            STATE_SYNC_EVENT,
            Payload {
                key: key.into(),
                value: serde_json::to_string(&value)?,
            },
        )
        .unwrap();

    Ok(())
}
