use crate::states;

use rodio::cpal::traits::StreamTrait;
use tauri::{State, Window};

#[tauri::command]
pub fn play(window: Window, playback_state: State<states::playback::PlaybackState>) {
    let stream_clone = playback_state.stream.clone();
    let mut stream = stream_clone.lock();

    match stream.play() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to play: {}", e);
            // return;
        }
    };

    {
        let is_paused_clone = playback_state.is_paused.clone();
        *(is_paused_clone.write()) = false;
    }

    states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn pause(window: Window, playback_state: State<states::playback::PlaybackState>) {
    let stream_clone = playback_state.stream.clone();
    let mut stream = stream_clone.lock();

    match stream.pause() {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Failed to pause: {}", e);
            // return;
        }
    };

    {
        let is_paused_clone = playback_state.is_paused.clone();
        *(is_paused_clone.write()) = true;
    }

    states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn toggle_playback(window: Window, playback_state: State<states::playback::PlaybackState>) {
    let is_paused = { playback_state.is_paused.read().clone() };

    if is_paused {
        play(window, playback_state);
    } else {
        pause(window, playback_state);
    }
}

#[tauri::command]
pub fn add_track(
    window: Window,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
) {
    let mut tracks = global_app_state.tracks.lock();
    let mut track = states::playback::Track::new(
        None,
        playback_state.config.clone(),
        playback_state.total_frames.clone(),
    );

    let mut mixer = playback_state.mixer.clone();

    match track.sources_queue_output.take() {
        Some(sources_queue_output) => {
            mixer.add(sources_queue_output)
            // match mixer.add(sources_queue_output) {
            //     Ok(_) => {}
            //     Err(e) => {
            //         eprintln!("Failed to play_raw: {}", e);
            //         return;
            //     }
            // };
        }
        _ => {}
    }

    // let mut tracks = global_app_state.tracks.lock();

    (*tracks).push(track);

    println!("current tracks {:?}", tracks);

    states::emit_state_sync("tracks", &*tracks, &window);
}

#[tauri::command]
pub fn try_seek(
    window: Window,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    pos: f64,
) {
    println!("try_seek {:?}", pos);
    let mut tracks = global_app_state.tracks.lock();

    for track in &mut *tracks {
        match track.try_seek(std::time::Duration::from_secs_f64(pos)) {
            Ok(_) => {}
            Err(e) => {
                eprintln!("Failed to try_seek: {}", e);
                return;
            }
        };
    }

    println!("Finished try_seek");

    *(playback_state.total_frames.write()) =
        (pos * (playback_state.config.sample_rate().0 as f64)) as u64;

    states::emit_state_sync("playback", playback_state.inner(), &window);
}
