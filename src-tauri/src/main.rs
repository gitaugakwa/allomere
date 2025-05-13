// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autogen {
    pub mod constants;
}

mod handlers;
mod states;

use parking_lot::{Mutex, RwLock};

use std::sync::Arc;

use tauri::menu::{MenuBuilder, MenuItemBuilder, PredefinedMenuItem, SubmenuBuilder};
use tauri::{Manager, State, Window};
use tauri_plugin_dialog::{DialogExt, FileDialogBuilder};

use std::env;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
fn main() -> anyhow::Result<()> {
    let window = Arc::new(Mutex::new(None));

    // tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(states::set_default_state(window))
        .manage(states::playback::set_default_state())
        .setup(|app| {
            let main_window = app.get_webview_window("main").unwrap();
            let handle = app.app_handle();

            {
                states::APP_HANDLE.lock().replace(handle.clone())
            };

            let open = MenuItemBuilder::new("Open File")
                .id("openFile".to_string())
                .build(app)?;
            let file_menu = SubmenuBuilder::new(app, "Media").item(&open).build()?;
            let mut menu_builder = MenuBuilder::new(app).item(&file_menu).separator().quit();

            // #[cfg(debug_assertions)]
            // {
            let refresh = MenuItemBuilder::new("Refresh")
                .id("refresh".to_string())
                .build(app)?;
            menu_builder = menu_builder.separator().item(&refresh);
            // }

            let menu = menu_builder.build()?;

            let global_app_state: State<states::GlobalAppState> = app.state();
            let playback_state: State<states::playback::PlaybackState> = app.state();

            global_app_state.window.lock().replace(main_window.clone());

            handlers::playback::add_track(main_window.clone(), playback_state, global_app_state);

            // let open_file_window = main_window.clone();
            // let _id = main_window.listen("openFile", move |event| {
            //     let payload: states::window::OpenFilePayload =
            //         serde_json::from_str(event.payload().unwrap()).unwrap();

            //     handlers::window::open_file(open_file_window.clone(), payload);
            // });

            // let refresh_window = main_window.clone();
            // main_window.listen("refresh", move |_event| {
            //     handlers::window::refresh(refresh_window.clone());
            // });

            app.set_menu(menu)?;

            app.on_menu_event(|app, event| {
                if let Some(webview_window) = app.get_webview_window("main") {
                    match event.id().as_ref() {
                        "openFile" => {
                            app.dialog()
                                .file()
                                .add_filter("Music", &["wav", "mp3", "flac"])
                                .pick_file(move |path_buf| match path_buf {
                                    Some(p) => {
                                        handlers::window::open_file(
                                            webview_window,
                                            states::window::OpenFilePayload {
                                                path: p.into_path().expect("Should be PathBuf"),
                                            },
                                        );
                                    }
                                    _ => {}
                                });
                        }
                        "refresh" => {
                            handlers::window::refresh(webview_window);
                        }
                        _ => {}
                    }
                }
            });

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            handlers::playback::play,
            handlers::playback::pause,
            handlers::playback::toggle_playback,
            handlers::playback::try_seek,
            handlers::playback::add_track,
            handlers::playback::get_clip,
            handlers::playback::get_audio_data,
            handlers::playback::clear_clip_loop,
            handlers::playback::set_clip_loop,
            handlers::playback::set_clip_loop_frames,
            handlers::playback::get_clip_preferred_transition_beats,
            handlers::audio::get_beats,
            // get_beats,
        ])
        // .on_page_load(|window, event| {
        //     let playback_state: State<handlers::playback::PlaybackState> = window.state();
        //     states::emit_state_sync("playback", playback_state.inner(), &window);
        //     // window.state()
        // })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
    Ok(())
}
