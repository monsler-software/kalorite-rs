use std::{fs::File, path::Path, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicUsize, Ordering}}};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::{codecs::audio::AudioDecoderOptions, formats::{FormatOptions, TrackType, probe::Hint}, io::MediaSourceStream, meta::MetadataOptions};
use symphonia::core::errors::Error as SymphoniaError;

/*
    Kalorite audio module
        crates used:
        - symphonia
        - cpal
    created at 01.10.2026
*/

pub struct AudioEngine {
    volume: Arc<AtomicUsize>,
    playback_position: Arc<AtomicUsize>,
    samples: Arc<Mutex<Vec<f32>>>,
    samples_count: usize,
    sample_rate: u32,
    is_playing: Arc<AtomicBool>,
    finished: Arc<AtomicBool>,
    paused: Arc<AtomicBool>,
    channels_count: usize,
    stream: Option<cpal::Stream>
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioEngine {
    pub fn new() -> AudioEngine {
        AudioEngine { 
            volume: Arc::new(AtomicUsize::new(100)),
            playback_position: Arc::new(AtomicUsize::new(0)),
            samples: Arc::new(Mutex::new(Vec::new())),
            samples_count: 0,
            sample_rate: 44100,
            channels_count: 0,
            is_playing: Arc::new(AtomicBool::new(false)),
            finished: Arc::new(AtomicBool::new(false)),
            paused: Arc::new(AtomicBool::new(false)),
            stream: None
        }
    }

    pub fn load_track(&mut self, audio_path: &str) -> Result<(), String> {
        let path = Path::new(audio_path);

        if !path.exists() {
            return Err(format!("Error! file '{}' doesn't exist", audio_path))
        }

        let file = Box::new(File::open(path).unwrap());
        let mss = MediaSourceStream::new(file, Default::default());

        let fmt_opts: FormatOptions = Default::default();
        let meta_opts: MetadataOptions = Default::default();
        let dec_opts: AudioDecoderOptions = Default::default();

        let hint = Hint::new();
        
        let mut format =
        symphonia::default::get_probe().probe(&hint, mss, fmt_opts, meta_opts).unwrap();

        let track = format.default_track(TrackType::Audio).unwrap();
        let track_id = track.id;

        // decoder initialization
        let mut decoder = symphonia::default::get_codecs()
            .make_audio_decoder(track.codec_params.as_ref().unwrap().audio().unwrap(), &dec_opts)
            .unwrap();

        self.samples = Arc::new(Mutex::new(Vec::new()));
        self.samples_count = 0;

        if let Some(codec) = &track.codec_params {
            if let Some(audio) = codec.audio() {
                self.sample_rate = audio.sample_rate.unwrap();
                let chan = audio.channels.as_ref().unwrap();
                self.channels_count = chan.count();
            } else {
                println!("Unable to determine sample rate; will use 44100");
                self.sample_rate = 44100;
            }
        }

        let mut all_samples = Vec::new();

        while let Some(packet) = format.next_packet().unwrap() {
            if packet.track_id != track_id {
                continue;
            }

            match decoder.decode(&packet) {
                Ok(audio_buf) => {
                    let sample_count = audio_buf.samples_interleaved();

                    let mut decoded = vec![0.0f32; sample_count];
                    audio_buf.copy_to_slice_interleaved(&mut decoded);

                    self.samples_count += decoded.len();
                    all_samples.extend_from_slice(&decoded);
                }

                Err(SymphoniaError::DecodeError(_)) => {
                    continue;
                }

                Err(e) => {
                    return Err(e.to_string());
                }
            }
        }

        let mut lock = self.samples.lock().unwrap();
        *lock = all_samples;

        self.finished.store(false, Ordering::Relaxed);
        self.playback_position.store(0, Ordering::Relaxed);

        Ok(())
    }

    pub fn is_finished(&self) -> bool {
        self.finished.load(Ordering::Relaxed)
    }

    pub fn position(&self) -> usize {
        self.playback_position.load(Ordering::Relaxed)
    }

    pub fn volume(&self) -> usize {
        self.volume.load(Ordering::Relaxed)
    }

    pub fn set_volume(&mut self, volume: usize) {
        self.volume.store(volume, Ordering::Relaxed);
    }

    pub fn stop(&mut self) -> Result<(), String> {
        self.pause()?;
        // store needed here in order to reset the progress of playback
        self.playback_position.store(0, Ordering::Relaxed);
        // this actually isn't needed btw but i rather leave it here cuz else i think it may cause ram leaks
        self.stream = None;
        Ok(())
    }

    pub fn pause(&mut self) -> Result<(), String> {
        if let Some(stream) = &self.stream {
            stream.pause().map_err(|e| e.to_string())?;
            self.paused.store(true, Ordering::Relaxed);
        }
        Ok(())
    }

    pub fn play(&mut self) -> Result<(), String> {
        let host = cpal::default_host();

        self.finished.store(false, Ordering::Relaxed);
        self.is_playing.store(true, Ordering::Relaxed);
        self.paused.store(false, Ordering::Relaxed);

        if let Some(stream) = &self.stream {
            let _ = stream.play();
            return Ok(())
        }

        let samples_clone = Arc::clone(&self.samples);
        let finished = Arc::clone(&self.finished);
        let position = Arc::clone(&self.playback_position);
        let volume = Arc::clone(&self.volume);

        let device = host
            .default_output_device()
            .ok_or("No output device found")?;

        let config = cpal::StreamConfig {
            channels: self.channels_count as u16,
            sample_rate: self.sample_rate,
            buffer_size: cpal::BufferSize::Default,
        };

        let stream = device.build_output_stream(
            config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                let samples = samples_clone.lock().unwrap();
                let mut pos = position.load(Ordering::Relaxed);
               
                for sample in data.iter_mut() {
                    let vol = volume.load(Ordering::Relaxed) as f32 / 100.0;
                    if pos >= samples.len() {
                        finished.store(true, Ordering::Relaxed);
                        *sample = 0.0;
                        continue;
                    }

                    *sample = samples[pos] * vol;
                    pos += 1;
                    position.store(pos, Ordering::Relaxed);
                }                
            },

            |err| eprintln!("An error occurred on stream: {}", err),
            None
        ).map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;
        self.stream = Some(stream);

        Ok(())
    }
}