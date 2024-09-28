use derivative::Derivative;

use parking_lot::{Mutex, RwLock};

use numpy::PyArray2;
use pyo3::prelude::*;

use anyhow::{anyhow, Result};

use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use rodio::cpal::{self, Device, Sample, Stream, SupportedStreamConfig};
use rodio::dynamic_mixer::{self, DynamicMixer, DynamicMixerController};
use rodio::queue::SourcesQueueOutput;
use rodio::source::{self, Buffered, SamplesConverter};
use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink, Source, StreamError};

use usearch::{Index, IndexOptions, MetricKind, ScalarKind};

use lazy_static::lazy_static;

use serde::{Deserialize, Serialize, Serializer};

use std::collections::HashMap;
use std::fs::File;
use std::io::{self, BufReader, Cursor, Read, Seek};
use std::path::Path;
use std::sync::{atomic::AtomicU64, atomic::Ordering::SeqCst, Arc};
use std::thread;
use std::time::{Duration, Instant};

use crate::handlers;

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
#[serde(rename_all = "camelCase")]
enum Value {
    Int64(i64),
    UInt64(u64),
    Int32(i32),
    UInt32(u32),
    UInt16(u16),
    Boolean(bool),
    String(String),
}

// #[derive(Clone)]
// pub struct Audio {
//     pub source: source::Buffered<Decoder<BufReader<File>>>,
// }

#[derive(Clone)]
pub struct CustomSource<R>
where
    R: Read + Seek,
{
    pub raw_source: Arc<Mutex<source::TrackPosition<SamplesConverter<Decoder<R>, f32>>>>,
}

impl<R: Read + Seek> CustomSource<R> {
    // type Item = f32;

    pub fn new(raw_source: Decoder<R>) -> Self {
        // let test = raw_source.track_position().convert_samples::<f32>();
        CustomSource {
            raw_source: Arc::new(Mutex::new(
                raw_source.convert_samples::<f32>().track_position(),
            )),
        }
    }
}

impl<R: Read + Seek> Iterator for CustomSource<R> {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        // if (self
        //     .raw_source
        //     .clone()
        //     .lock()
        //     .get_pos()
        //     .gt(&Duration::from_secs(4)))
        // {
        //     if let Err(e) = self
        //         .raw_source
        //         .clone()
        //         .lock()
        //         .try_seek(Duration::from_secs(0))
        //     {
        //         eprintln!("Failed to seek: {:?}", e);
        //     }
        // }
        self.raw_source.clone().lock().next()
        // Some(0.0)
    }
}

impl<R: Read + Seek> Source for CustomSource<R> {
    fn current_frame_len(&self) -> Option<usize> {
        self.raw_source.clone().lock().current_frame_len()
        // Some(1)
    }

    fn channels(&self) -> u16 {
        self.raw_source.clone().lock().channels()
        // 1
    }

    fn sample_rate(&self) -> u32 {
        self.raw_source.clone().lock().sample_rate()
        // 44_100
    }

    fn total_duration(&self) -> Option<Duration> {
        self.raw_source.clone().lock().total_duration()
        // Some(Duration::from_secs(1))
    }

    fn try_seek(&mut self, pos: Duration) -> Result<(), source::SeekError> {
        self.raw_source.clone().lock().try_seek(pos)
    }
}

#[derive(Clone)]
pub struct Audio<R>
where
    R: Read + Seek,
{
    pub source: CustomSource<R>,
}

impl<R: Read + Seek> Serialize for Audio<R> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper {
            data: HashMap<String, Value>,
        }
        let mut helper = HashMap::new();
        helper.insert(
            "length",
            Value::UInt64(
                ((self.source.total_duration().unwrap().as_secs_f64()
                    * (self.source.sample_rate() as f64)) as u64),
            ),
        );
        helper.insert("sampleRate", Value::UInt32(self.source.sample_rate()));

        helper.serialize(serializer)
    }
}

impl<R: Read + Seek> Audio<R> {
    pub fn total_frames(&self) -> u64 {
        (self.source.total_duration().unwrap().as_secs_f64() * (self.source.sample_rate() as f64))
            as u64
    }
}

#[derive(Clone)]
pub struct Sound(Arc<Vec<u8>>);

impl AsRef<[u8]> for Sound {
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl Sound {
    pub fn load(filename: &str) -> io::Result<Sound> {
        let mut buf = Vec::new();
        let mut file = File::open(filename)?;
        file.read_to_end(&mut buf)?;
        Ok(Sound(Arc::new(buf)))
    }
    pub fn cursor(self: &Self) -> Cursor<Sound> {
        Cursor::new(Sound(self.0.clone()))
    }
    pub fn decoder(self: &Self) -> Decoder<Cursor<Sound>> {
        Decoder::new(self.cursor()).unwrap()
    }
}

struct AudioData {
    path: String,
    beat_track: Option<Vec<u32>>,
    beat_features: Arc<Mutex<Option<Vec<Vec<f32>>>>>,
    sound: Sound,
}

impl AudioData {
    pub fn new(path: &str) -> Self {
        // let get_beats_path = path.clone();

        AudioData {
            sound: Sound::load(path).unwrap(),
            path: path.to_string(),
            beat_track: None,
            beat_features: Arc::new(Mutex::new(None)),
        }
    }
}

lazy_static! {
    static ref AUDIO_DATA_MAP: Mutex<HashMap<String, Arc<Mutex<AudioData>>>> =
        { Mutex::new(HashMap::new()) };
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    pub path: String,
    pub name: String,
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    pub audio: Option<Audio<Cursor<Sound>>>,
    // #[serde(skip_deserializing)]
    // #[serde(skip_serializing)]
    // #[derivative(Debug = "ignore")]
    // pub beat_track: Option<Vec<u64>>,
    pub start_at: Option<u64>,
}

impl Clip {
    pub fn new(path: &str) -> Self {
        let audio_data = {
            let mut audio_data_map = AUDIO_DATA_MAP.lock(); // Lock the mutex here
            audio_data_map
                .entry(path.to_string())
                .or_insert_with(|| Arc::new(Mutex::new(AudioData::new(path))))
                .clone()
        };
        // AudioDataMap.insert(path.clone(), AudioData::new(path.clone()));

        // let get_beats_path = path.clone();
        let audio_data_ref = audio_data.clone();

        tauri::async_runtime::spawn(async move {
            let (path, beat_track_exists) = {
                let audio_data_guard = audio_data_ref.lock(); // Lock the mutex here

                (
                    audio_data_guard.path.clone(),
                    audio_data_guard.beat_track.is_some(),
                )
            };

            let mut beat_features = {
                let audio_data_guard = audio_data_ref.lock(); // Lock the mutex again to access beat_features
                audio_data_guard.beat_features.clone()
            };
            let mut beat_features_guard = beat_features.lock();

            let now = Instant::now();
            if (!beat_track_exists) {
                let mut decoder = {
                    audio_data_ref
                        .lock()
                        .sound
                        .decoder()
                        .convert_samples::<f32>()
                    // .track_position()
                };
                let sample_rate = { decoder.sample_rate() };

                let mut features: Vec<Vec<f32>> = Vec::new();

                let py_result: Result<()> = Python::with_gil(|py| {
                    println!("Downloading and loading models...");

                    let beatsModule = PyModule::from_code_bound(
                        py,
                        include_str!("../../../src-python/src/beats.py"),
                        "beats.py",
                        "beats",
                    )?;

                    let featuresModule = PyModule::from_code_bound(
                        py,
                        include_str!("../../../src-python/src/features.py"),
                        "features.py",
                        "features",
                    )?;

                    let get_audio_features: Py<PyAny> =
                        featuresModule.getattr("get_audio_features")?.into();

                    let get_beats: Py<PyAny> = beatsModule.getattr("get_beats")?.into();

                    let (tempo, beat_track): (Vec<f32>, Vec<u32>) =
                        get_beats.call1(py, (path, sample_rate))?.extract(py)?;

                    // let mut beat_feature_sources = Vec::new();

                    // println!("Beat Track: {:?}", beat_track);
                    for (i, beat) in beat_track[0..15].iter().enumerate() {
                        let now = Instant::now();

                        let mut samples_buffer = Vec::new();

                        // Thinking of having a second buffer around the sample
                        // This would be |< sample_rate >| 1 |< sample_rate >| = 2 * sample_rate + 1

                        let start: i32 = *beat as i32 - sample_rate as i32;
                        // println!("{}, {}, {}", start, sample_rate, tempo[0]);
                        let seek_duration = if start < 0 {
                            Duration::from_secs(0)
                        } else {
                            match (start as u64).checked_div(sample_rate as u64) {
                                Some(duration) => Duration::from_secs(duration),
                                None => {
                                    // println!("{}, {}", start, sample_rate);
                                    eprintln!("Overflow occurred while calculating seek duration");
                                    Duration::from_secs(0)
                                    // continue; // Skip this beat if overflow occurs
                                }
                            }
                        };
                        // println!(
                        //     "Seek Duration: {:?}, {}, {:?}",
                        //     seek_duration,
                        //     start,
                        //     decoder.get_pos()
                        // );

                        let result = decoder.try_seek(
                            seek_duration
                                .clamp(Duration::from_secs(0), decoder.total_duration().unwrap()),
                        );

                        match result {
                            Ok(_) => {
                                // Seek was successful
                            }
                            Err(e) => {
                                println!("Failed to seek: {:?}", e);
                            }
                        }

                        if start < 0 {
                            samples_buffer = vec![0_f32; start.abs().try_into().unwrap()];
                        }

                        while samples_buffer.len() < ((sample_rate * 2) + 1).try_into().unwrap() {
                            // Step 2: Loop until buffer length equals sample_rate
                            match decoder.next() {
                                // Step 3: Attempt to read a sample
                                Some(sample) => samples_buffer.push(sample), // Step 4: Add sample to buffer
                                None => break, // Step 5: Break if no more samples
                            }
                        }
                        // println!(
                        //     "Buffer Length: {}, {:?}",
                        //     samples_buffer.len(),
                        //     now.elapsed().as_secs_f32()
                        // );

                        let audio_array = PyArray2::from_vec2_bound(py, &[samples_buffer])?;

                        println!("Audio Array: {:?}", now.elapsed().as_secs_f32());
                        features.extend_from_slice(
                            &get_audio_features
                                .call1(py, (audio_array, sample_rate))?
                                .extract::<Vec<Vec<f32>>>(py)?,
                        );
                        println!(
                            "{}/{}: {:?}",
                            i,
                            beat_track.len(),
                            now.elapsed().as_secs_f32()
                        );
                        // println!("Feature Extraction Time: {:?}", now.elapsed().as_secs_f32());
                    }

                    {
                        audio_data_ref.lock().beat_track = Some(beat_track);
                        beat_features_guard.replace(features);
                    }

                    Ok(())
                });
            } else {
                println!("Beat track already exists")
            }
            println!(
                "Beat track generation complete: {:?}",
                now.elapsed().as_secs_f32()
            );
        });

        let custom_source = CustomSource::new(audio_data.lock().sound.decoder());

        Clip {
            path: path.to_string(),
            name: Path::new(&path)
                .file_name()
                .expect("File name should exist")
                .to_str()
                .expect("File name should be valid")
                .to_string(),
            audio: Some(Audio {
                source: custom_source,
            }),
            start_at: None,
        }
    }

    pub fn try_seek(&mut self, pos: Duration) -> Result<(), source::SeekError> {
        self.audio.as_mut().unwrap().source.try_seek(pos)
    }

    pub fn total_frames(&self) -> u64 {
        (self.audio.as_ref().unwrap().total_frames())
    }
}

#[derive(Derivative, Serialize, Deserialize)]
#[derivative(Debug)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub name: String,
    // Should have the sections to be played
    pub clips: Vec<Clip>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    pub sink: Option<Sink>,

    // #[serde(skip_serializing)]
    // #[serde(skip_deserializing)]
    // #[derivative(Debug = "ignore")]
    // pub mixer_controller: Option<Arc<DynamicMixerController<f32>>>,

    // #[serde(skip_serializing)]
    // #[serde(skip_deserializing)]
    // #[derivative(Debug = "ignore")]
    // pub mixer_output: Option<DynamicMixer<f32>>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    pub sources_queue_output: Option<SourcesQueueOutput<f32>>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    pub playback_config: Option<Arc<SupportedStreamConfig>>,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    beat_index_options: Option<Arc<Mutex<IndexOptions>>>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    #[derivative(Debug = "ignore")]
    beat_index: Option<Arc<Mutex<Index>>>,

    current: Option<usize>,
    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    total_frames: Arc<RwLock<u64>>,
}

unsafe impl Send for Track {}

impl Track {
    fn id() -> u64 {
        static COUNTER: AtomicU64 = AtomicU64::new(1);

        COUNTER.fetch_add(1, SeqCst)
    }

    pub fn new(
        name: Option<String>,
        playback_config: Arc<SupportedStreamConfig>,
        total_frames: Arc<RwLock<u64>>,
    ) -> Self {
        let (sink, sources_queue_output) = Sink::new_idle();
        // let (mixer_controller, mixer) = dynamic_mixer::mixer::<f32>(2, 44_100);
        // sink.append(mixer);

        // match volume {
        //     Some(volume) => sink.set_volume(volume),
        //     _ => {}
        // }

        let mut index_options = IndexOptions::default();
        index_options.dimensions = 512; // Set the number of dimensions for vectors
        index_options.metric = MetricKind::Cos; // Use cosine similarity for distance measurement
        index_options.quantization = ScalarKind::F32; // Use 32-bit floating point numbers

        let mut index = Index::new(&index_options).unwrap();

        index.reserve(1000);

        Track {
            name: name.unwrap_or_else(|| format!("Track {}", Self::id())),
            clips: (Vec::new()),
            sink: Some(sink),
            beat_index: Some(Arc::new(Mutex::new(index))),
            beat_index_options: Some(Arc::new(Mutex::new(index_options))),
            // mixer_controller: Some(mixer_controller),

            // mixer_output: None,
            playback_config: Some(playback_config),
            sources_queue_output: Some(sources_queue_output),
            current: None,
            total_frames,
        }
    }

    pub fn add_clip(&mut self, mut clip: Clip) {
        // let mixer = if (self.sink.as_ref().expect("Sink should exist").len() == 0) {
        //     println!("Sink empty creating new mixer");
        //     let (mixer_controller, mixer) = dynamic_mixer::mixer::<f32>(2, 44_100);
        //     self.mixer_controller = Some(mixer_controller);
        //     Some(mixer)
        // } else {
        //     None
        // };

        // self.playback_config.unwrap().sample_rate()
        // find a clip with a start_at, every other clip follows that
        // unless explicitly states with another start_at

        if self
            .clips
            .iter()
            .find(|&clip| clip.start_at.is_some())
            .is_none()
        {
            let track_frames = self.total_frames();
            // println!("Track Duration: {}", track_duration);
            match track_frames {
                Some(track_frames) => {
                    let playback_frames = { *self.total_frames.read() };

                    if track_frames > playback_frames {
                        clip.start_at.replace(track_frames);
                    } else {
                        clip.start_at.replace(playback_frames);
                    }
                }
                _ => {}
            }
        }
        match clip.audio {
            Some(ref audio) => {
                let source = &audio.source;
                match self.sink {
                    Some(ref sink) => {
                        sink.append(source.clone().convert_samples::<f32>());

                        // mixer_controller.add(source.clone().convert_samples());
                    }
                    _ => {}
                }
            }
            None => todo!(),
        }
        // {
        //     clip.start_at = Some(*self.total_frames.read());
        // }
        let path = clip.path.clone();
        self.clips.push(clip);

        // let audio_data_ref = audio_data.clone();

        // need to fix this, will prolly spawn a thread

        let beat_index_ref = self.beat_index.as_ref().unwrap().clone();

        tauri::async_runtime::spawn(async move {
            println!("Adding features to beat index");
            let beat_features = {
                let audio_data_map = AUDIO_DATA_MAP.lock(); // Lock the mutex here
                audio_data_map
                    .get(&path)
                    .unwrap()
                    .clone()
                    .lock()
                    .beat_features
                    .clone()
            };

            let beat_features_guard = beat_features.lock();
            let beat_features_ref = beat_features_guard.as_ref().unwrap();
            println!("After beat features lock");

            let beat_index = beat_index_ref.lock();
            for (i, feature) in beat_features_ref.iter().enumerate() {
                let now = Instant::now();

                if let Err(e) = beat_index.add(i as u64, feature) {
                    eprintln!("Failed to add feature to beat index: {:?}", e);
                }
                println!(
                    "{}/{}: {:?}",
                    i,
                    beat_features_ref.len(),
                    now.elapsed().as_secs_f32()
                );
            }
            let results = beat_index
                .search(&beat_features_ref[0], 5)
                .expect("Search failed.");
            for (key, distance) in results.keys.iter().zip(results.distances.iter()) {
                println!("Key: {}, Distance: {}", key, distance);
            }
            println!("Added features to beat index");
        });

        // {
        //     let beat_index = self.beat_index.as_ref().unwrap().lock();
        //     for (i, feature) in audio_data_ref
        //         .lock()
        //         .beat_features
        //         .as_ref()
        //         .unwrap()
        //         .iter()
        //         .enumerate()
        //     {
        //         beat_index.add(i as u64, feature);
        //     }
        // }

        // clip.audio.unwrap().
        // match mixer {
        //     Some(mixer) => match self.sink {
        //         Some(ref sink) => {
        //             sink.append(mixer);
        //         }
        //         _ => {}
        //     },
        //     _ => {}
        // }
    }

    pub fn total_duration(&mut self) -> Option<Duration> {
        let mut total_duration = Duration::new(0, 0);
        for clip in &mut self.clips {
            let clip_duration = clip.audio.as_ref().unwrap().source.total_duration();
            match clip_duration {
                Some(clip_duration) => match total_duration.checked_add(clip_duration) {
                    Some(new_total_duration) => {
                        total_duration = new_total_duration;
                    }
                    _ => {
                        return None;
                    }
                },
                _ => {
                    return None;
                }
            }
        }
        Some(total_duration)
    }

    pub fn total_frames(&self) -> Option<u64> {
        let mut total_duration: u64 = 0;
        for ref clip in &self.clips {
            let clip_duration = clip.audio.as_ref().unwrap().total_frames();
            total_duration += clip_duration;
        }
        Some(total_duration)
    }

    pub fn try_seek(&mut self, pos: Duration) -> Result<(), source::SeekError> {
        self.sink.as_ref().unwrap().try_seek(pos);

        // let mut current_pos = Duration::from_secs(0);
        // for ref mut clip in &mut self.clips {
        //     let clip_duration = clip
        //         .audio
        //         .as_ref()
        //         .unwrap()
        //         .source
        //         .total_duration()
        //         .unwrap();
        //     if current_pos + clip_duration >= pos {
        //         let seek_duration = pos - current_pos;
        //         clip.try_seek(seek_duration)?;
        //         return Ok(());
        //     }
        //     current_pos += clip_duration;
        // }
        // Err(source::SeekError)
        Ok(())
    }
}

// pub struct TrackController {

// }

// impl Iterator for Track {
//     type Item = dyn Sample;

//     fn next(&mut self) -> Option<Self::Item> {
//         loop {
//             return Some(
//                 self.clips
//                     .get(self.current.unwrap())
//                     .unwrap()
//                     .audio
//                     .unwrap()
//                     .source
//                     .next(),
//             );
//         }
//     }
// }

// impl Source for Track {
//     fn current_frame_len(&self) -> Option<usize> {
//         todo!()
//     }

//     fn channels(&self) -> u16 {
//         todo!()
//     }

//     fn sample_rate(&self) -> u32 {
//         todo!()
//     }

//     fn total_duration(&self) -> Option<std::time::Duration> {
//         todo!()
//     }
// }

#[derive(Derivative)]
#[derivative(Debug)]
pub struct PlaybackState {
    #[derivative(Debug = "ignore")]
    pub mixer: Arc<DynamicMixerController<f32>>,
    #[derivative(Debug = "ignore")]
    pub stream: Arc<Mutex<Stream>>,
    #[derivative(Debug = "ignore")]
    pub config: Arc<SupportedStreamConfig>,
    #[derivative(Debug = "ignore")]
    pub device: Device,

    pub is_paused: Arc<RwLock<bool>>,
    pub total_frames: Arc<RwLock<u64>>,
}

unsafe impl Send for PlaybackState {}
unsafe impl Sync for PlaybackState {}

impl Serialize for PlaybackState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        #[derive(Serialize)]
        struct Helper {
            data: HashMap<String, Value>,
        }
        let mut helper = HashMap::new();
        helper.insert("isPaused", Value::Boolean(*self.is_paused.read()));
        helper.insert("totalFrames", Value::UInt64(*self.total_frames.read()));
        helper.insert("channels", Value::UInt16(self.config.channels()));
        helper.insert("sampleRate", Value::UInt32(self.config.sample_rate().0));

        helper.serialize(serializer)
    }
}

pub fn set_default_state() -> PlaybackState {
    let default_device = cpal::default_host()
        .default_output_device()
        .ok_or(StreamError::NoDevice)
        .expect("Here");

    let config = default_device.default_output_config().expect("Here 2");
    let error_callback = |err| eprintln!("an error occurred on output stream: {}", err);

    let (mixer_tx, mut mixer_rx) =
        dynamic_mixer::mixer::<f32>(config.channels(), config.sample_rate().0);
    let total_frames = Arc::new(RwLock::new(0u64));
    let total_frames_clone = total_frames.clone();

    let channels = config.channels() as usize;

    let stream = default_device
        .build_output_stream::<f32, _, _>(
            &config.config(),
            move |data, _| {
                {
                    // let mut total_samples_guard = total_samples_clone.write();
                    *(total_frames_clone.write()) += (data.len() / channels) as u64;
                    // let total_samples = *(total_samples_guard);
                    // drop(total_samples_guard);

                    // match *window_clone.lock() {
                    //     Some(ref window) => {
                    //         states::emit_state_sync("totalSamples", &total_samples, &window);
                    //     }
                    //     _ => {}
                    // }
                    // states::emit_state_sync("totalSamples", &total_samples, &window);
                }
                data.iter_mut()
                    .for_each(|d| *d = mixer_rx.next().unwrap_or(0f32))
            },
            error_callback,
            None,
        )
        .expect("Here");

    PlaybackState {
        mixer: mixer_tx,
        stream: Arc::new(Mutex::new(stream)),
        device: default_device,
        config: Arc::new(config),
        is_paused: Arc::new(RwLock::new(true)),
        total_frames,
    }
}
