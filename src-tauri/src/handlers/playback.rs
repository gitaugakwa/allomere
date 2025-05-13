use std::ops::Deref;

use crate::states::{
    self, playback::AllomereMutex, playback::AudioData, playback::Clip, playback::AUDIO_DATA_MAP,
};

use rodio::cpal::traits::StreamTrait;
use std::collections::HashMap;
use tauri::{State, WebviewWindow};
use usearch;

#[tauri::command]
pub fn play(window: WebviewWindow, playback_state: State<states::playback::PlaybackState>) {
    let stream_clone = playback_state.stream.clone();
    let stream = stream_clone.lock();

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

    let _ = states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn pause(window: WebviewWindow, playback_state: State<states::playback::PlaybackState>) {
    let stream_clone = playback_state.stream.clone();
    let stream = stream_clone.lock();

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

    let _ = states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn toggle_playback(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
) {
    let is_paused = { playback_state.is_paused.read().clone() };

    if is_paused {
        play(window, playback_state);
    } else {
        pause(window, playback_state);
    }
}

#[tauri::command]
pub fn add_track(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
) {
    let mut tracks = global_app_state.tracks.lock();
    let mut track = states::playback::Track::new(
        None,
        playback_state.config.clone(),
        playback_state.total_frames.clone(),
    );

    let mixer = playback_state.mixer.clone();

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

    let _ = states::emit_state_sync("tracks", &*tracks, &window);
}

#[tauri::command]
pub fn try_seek(
    window: WebviewWindow,
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

    let _ = states::emit_state_sync("playback", playback_state.inner(), &window);
}

#[tauri::command]
pub fn get_clip(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    id: usize,
) -> Option<Clip> {
    let mut tracks = global_app_state.tracks.lock();

    let clip = (*tracks).iter().find_map(|track| {
        track.clips.iter().find_map(|clip| {
            if ((*clip).0.lock()).id == id {
                Some((*clip).clone())
            } else {
                None
            }
        })
    });

    if let Some(clip) = clip {
        println!("Found: {}", clip.0.lock().id);
        Some(clip.0.lock().clone())
    } else {
        None
    }
}

#[tauri::command]
pub fn get_clip_preferred_transition_beats(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    id: usize,
    beat: usize,
    count: usize,
) -> Option<HashMap<u64, f32>> {
    let mut tracks = global_app_state.tracks.lock();

    let track_and_clip = (*tracks).iter().find_map(|track| {
        track.clips.iter().find_map(|clip| {
            if ((*clip).0.lock()).id == id {
                Some((track, (*clip).clone()))
            } else {
                None
            }
        })
    });

    if let Some((track, clip_ref)) = track_and_clip {
        let clip = clip_ref.0.lock();
        if let Some(beat_index) = &track.beat_index {
            {
                println!("Beat Index Size: {}", beat_index.clone().lock().size())
            }
            let matches = clip.get_preferred_transition_beats(beat_index.clone(), beat, count);
            let map = matches
                .keys
                .iter()
                .cloned()
                .zip(matches.distances.iter().cloned())
                .collect();
            println!("{:#?}", map);
            Some(map)
        } else {
            None
        }
        // Some(clip_ref.0.lock().clone())
    } else {
        None
    }
}

#[tauri::command]
pub fn set_clip_loop(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    id: usize,
    start_pos: f64,
    end_pos: f64,
) -> Option<()> {
    let mut tracks = global_app_state.tracks.lock();

    let clip = (*tracks).iter().find_map(|track| {
        track.clips.iter().find_map(|clip| {
            if ((*clip).0.lock()).id == id {
                Some((*clip).clone())
            } else {
                None
            }
        })
    });

    if let Some(clip_ref) = clip {
        let mut clip = clip_ref.0.lock();
        clip.set_loop(
            std::time::Duration::from_secs_f64(start_pos),
            std::time::Duration::from_secs_f64(end_pos),
        );
        Some(())
    } else {
        None
    }
}

#[tauri::command]
pub fn set_clip_loop_frames(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    id: usize,
    start_frame: u32,
    end_frame: u32,
) -> Option<()> {
    let mut tracks = global_app_state.tracks.lock();

    let clip = (*tracks).iter().find_map(|track| {
        track.clips.iter().find_map(|clip| {
            if ((*clip).0.lock()).id == id {
                Some((*clip).clone())
            } else {
                None
            }
        })
    });

    if let Some(clip_ref) = clip {
        let mut clip = clip_ref.0.lock();
        clip.set_loop_frames((start_frame), (end_frame));
        Some(())
    } else {
        None
    }
}

#[tauri::command]
pub fn clear_clip_loop(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    id: usize,
) -> Option<()> {
    let mut tracks = global_app_state.tracks.lock();

    let clip = (*tracks).iter().find_map(|track| {
        track.clips.iter().find_map(|clip| {
            if ((*clip).0.lock()).id == id {
                Some((*clip).clone())
            } else {
                None
            }
        })
    });

    if let Some(clip_ref) = clip {
        let mut clip = clip_ref.0.lock();
        clip.clear_loop();
        Some(())
    } else {
        None
    }
}

#[tauri::command]
pub fn get_audio_data(
    window: WebviewWindow,
    playback_state: State<states::playback::PlaybackState>,
    global_app_state: State<states::GlobalAppState>,
    path: String,
) -> Option<AudioData> {
    let audio_data_ref = {
        let audio_data_map_lock = AUDIO_DATA_MAP.lock();
        audio_data_map_lock.get(&path).cloned()
        // (AUDIO_DATA_MAP.lock().get(&path))
    };

    if let Some(data) = audio_data_ref {
        let audio_data_ref = data.clone();
        let audio_data = audio_data_ref.lock();
        Some(audio_data.clone())
    } else {
        None
    }
}
