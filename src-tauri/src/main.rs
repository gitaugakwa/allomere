// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autogen {
    pub mod constants;
}

mod handlers;
mod states;

use parking_lot::{Mutex, RwLock};

use std::sync::Arc;

use tauri::api::dialog;
use tauri::{CustomMenuItem, Manager, Menu, MenuItem, State, Submenu, Window};

use std::env;

// Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
fn main() -> anyhow::Result<()> {
    let open = CustomMenuItem::new("openFile".to_string(), "Open File");
    let fileMenu = Submenu::new("Media", Menu::new().add_item(open));
    let mut menu = Menu::new()
        .add_submenu(fileMenu)
        .add_native_item(MenuItem::Separator)
        .add_native_item(MenuItem::Quit);

    #[cfg(debug_assertions)]
    {
        let refresh = CustomMenuItem::new("refresh".to_string(), "Refresh");
        menu = menu.add_native_item(MenuItem::Separator).add_item(refresh);
    }

    let window = Arc::new(Mutex::new(None));

    // tauri::async_runtime::set(tokio::runtime::Handle::current());

    tauri::Builder::default()
        .manage(states::set_default_state(window))
        .manage(states::playback::set_default_state())
        .setup(|app| {
            let main_window = app.get_window("main").unwrap();
            let handle = app.handle();

            let global_app_state: State<states::GlobalAppState> = app.state();
            let playback_state: State<states::playback::PlaybackState> = app.state();

            global_app_state.window.lock().replace(main_window.clone());

            handlers::playback::add_track(main_window.clone(), playback_state, global_app_state);

            let open_file_window = main_window.clone();
            let id = main_window.listen("openFile", move |event| {
                let payload: states::window::OpenFilePayload =
                    serde_json::from_str(event.payload().unwrap()).unwrap();

                handlers::window::open_file(open_file_window.clone(), payload);
            });

            let refresh_window = main_window.clone();
            main_window.listen("refresh", move |event| {
                handlers::window::refresh(refresh_window.clone());
            });

            Ok(())
        })
        .menu(menu)
        .on_menu_event(|event| {
            let window = event.window();
            let window_name = window.label().to_string();
            let app = window.app_handle();

            match event.menu_item_id() {
                "openFile" => {
                    dialog::FileDialogBuilder::default()
                        .add_filter("Music", &["wav", "mp3", "flac"])
                        .pick_file(move |path_buf| match path_buf {
                            Some(p) => {
                                app.windows()[window_name.as_str()]
                                    .emit_and_trigger(
                                        "openFile",
                                        states::window::OpenFilePayload { path: p },
                                    )
                                    .unwrap();
                            }
                            _ => {}
                        });
                }
                "refresh" => {
                    app.windows()[window_name.as_str()]
                        .emit_and_trigger("refresh", ())
                        .unwrap();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            handlers::playback::play,
            handlers::playback::pause,
            handlers::playback::toggle_playback,
            handlers::playback::try_seek,
            handlers::playback::add_track,
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
