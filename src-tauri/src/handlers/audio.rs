use anyhow::{anyhow, Result};
use numpy::PyArray2;
use pyo3::prelude::*;
use std::time::{Duration, Instant};

#[tauri::command]
pub fn get_beats(path: &str, sample_rate: u32) -> (Vec<f32>, Vec<u32>) {
    let now = Instant::now();

    let py_result: Result<(Vec<f32>, Vec<u32>)> = Python::with_gil(|py| {
        println!("Downloading and loading models...");

        let module = PyModule::from_code_bound(
            py,
            include_str!("../../../src-python/src/beats.py"),
            "beats.py",
            "beats",
        )?;

        let get_beats: Py<PyAny> = module.getattr("get_beats")?.into();

        let (tempo, beat_track): (Vec<f32>, Vec<u32>) =
            get_beats.call1(py, (path, sample_rate))?.extract(py)?;

        // println!("{}", tempo[0]);

        Ok((tempo, beat_track))
    });

    println!(
        "get_beats - Elapsed time: {:?}",
        now.elapsed().as_secs_f32()
    );

    py_result.unwrap()
}

#[tauri::command]
pub fn get_features(audio: Vec<Vec<f32>>, sample_rate: u32) -> Vec<Vec<f32>> {
    let now = Instant::now();

    let py_result: Result<(Vec<Vec<f32>>)> = Python::with_gil(|py| {
        println!("Downloading and loading models...");

        let module = PyModule::from_code_bound(
            py,
            include_str!("../../../src-python/src/features.py"),
            "features.py",
            "features",
        )?;

        let get_audio_features: Py<PyAny> = module.getattr("get_audio_features")?.into();

        let mut features: Vec<Vec<f32>> = Vec::new();

        for (i, audio_chunk) in audio.chunks(10).enumerate() {
            let audio_array = PyArray2::from_vec2_bound(py, audio_chunk)?;

            println!("Processing chunk: {}", i);

            features.extend_from_slice(
                &get_audio_features
                    .call1(py, (audio_array, sample_rate))?
                    .extract::<Vec<Vec<f32>>>(py)?,
            );

            // features.push(
            //     (get_audio_features
            //         .call1(py, (audio_array, sample_rate))?
            //         .extract::<Vec<Vec<f32>>>(py)?)[0]
            //         .clone(),
            // );
        }

        // let audio_array = PyArray2::from_vec2_bound(py, &audio)?;

        // let features: Vec<Vec<f32>> = get_audio_features
        //     .call1(py, (audio_array, sample_rate))?
        //     .extract(py)?;

        // println!("{}", tempo[0]);

        Ok((features))
    });

    println!(
        "get_beat_features - Elapsed time: {:?}",
        now.elapsed().as_secs_f32()
    );

    py_result.unwrap()
}
