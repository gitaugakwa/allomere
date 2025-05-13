use crate::states;
use tauri::{Manager, State, WebviewWindow, Window};

use super::audio;

#[tauri::command]
pub fn refresh(window: WebviewWindow) {
    let handle = window.app_handle();

    let global_app_state: State<states::GlobalAppState> = handle.state();
    let playback_state: State<states::playback::PlaybackState> = handle.state();

    // let window = main_window.clone();

    let tracks = global_app_state.tracks.lock();

    let _ = states::emit_state_sync("tracks", &*tracks, &window);
    let _ = states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn open_file(window: WebviewWindow, payload: states::window::OpenFilePayload) {
    let handle = window.app_handle();

    let global_app_state: State<states::GlobalAppState> = handle.state();
    // let playback_state: State<handlers::playback::PlaybackState> = handle.state();

    // states::emit_state_sync("playback", playback_state.inner(), &window);

    let mut tracks = global_app_state.tracks.lock();

    // let playlist_state: State<Playlist> = handle.state();
    match payload.path.to_str() {
        Some(path) => match tracks.last_mut() {
            Some(track) => {
                let clip = states::playback::Clip::new(path);
                track.add_clip(clip);
                let _ = states::emit_state_sync("tracks", &*tracks, &window);
            }
            _ => {}
        },
        _ => {}
    }

    println!("got window openFile with payload {:?}", payload);
}
