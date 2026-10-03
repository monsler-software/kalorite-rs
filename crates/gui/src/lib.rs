use std::{sync::mpsc::{self, Receiver, Sender}, thread::{self, JoinHandle}};

use audio_engine::AudioEngine;
use qtbridge::{QApp, QmlObject, invoke_method, qobject, qsignal};
mod enums;

use crate::enums::{AudioCommand, AudioResponse};

/*
    Kalorite GUI module
        crates used:
        - qtbridge
    created at 01.10.2026
*/

struct MainWindow {
    _audio_thread: Option<JoinHandle<()>>,
    sender: Sender<AudioCommand>,
    receiver: Receiver<AudioResponse>
}


fn audio_loop(rx: Receiver<AudioCommand>, resptx: Sender<AudioResponse>, engine: &mut AudioEngine) {
    while let Ok(command) = rx.recv() {
        match command {
            AudioCommand::LoadTrack(path) => {
                match engine.load_track(&path) {
                    Ok(_) => {
                        println!("Loaded successfully!");
                    }

                    Err(e) => {
                        eprintln!("Error decoding: {e}");
                    }
                }
            }

            AudioCommand::SetPosition(pos) => {

            }

            AudioCommand::GetPosition => {

            }

            AudioCommand::SetVolume(vol) => {
                engine.set_volume(vol);
            }

            AudioCommand::GetVolume => {
                let _ = resptx.send(AudioResponse::Volume(engine.volume()));
            }

            AudioCommand::Pause => {
                if let Err(e) = engine.pause() {
                    eprintln!("Error pausing: {e}")
                }
            } 

            AudioCommand::Stop => {
                if let Err(e) = engine.stop() {
                    eprintln!("Error stopping: {e}")
                }
            }

            AudioCommand::Play => {
                if let Err(e) = engine.play() {
                    eprintln!("Playback error: {e}");
                }
            }
        }
    }
}

impl Default for MainWindow {
    fn default() -> Self {
        let (tx, rx) = mpsc::channel();
        let (rtx, rrx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let mut ae = AudioEngine::new();
            audio_loop(rx, rtx, &mut ae);
        });
        let _ = tx.send(AudioCommand::LoadTrack(String::from("testo.mp3")));
        Self { _audio_thread: Some(handle), sender: tx, receiver: rrx }
    }
}

#[qobject(Singleton)]
impl MainWindow {
    #[qsignal(qml_name="volumeChanged")]
    fn volume_changed(&mut self, vol: usize);

    #[qslot]
    fn poll_audio_events(&mut self) {
        while let Ok(command) = self.receiver.try_recv() {
            match command {
                AudioResponse::Volume(vol) => {
                    self.volume_changed(vol);
                }

                _ => {}
            }
        }
    }

    #[qslot]
    fn play(&mut self) {
       let _ = self.sender.send(AudioCommand::Play);
    }

    #[qslot]
    fn stop(&mut self) {
       let _ = self.sender.send(AudioCommand::Stop);
    }

    #[qslot]
    fn pause(&mut self) {
       let _ = self.sender.send(AudioCommand::Pause);
    }

    #[qslot]
    fn load_track(&mut self, path: String) {
       let _ = self.sender.send(AudioCommand::LoadTrack(path));
    }

    #[qslot]
    fn get_volume(&mut self) {
       let _ = self.sender.send(AudioCommand::GetVolume);
    }

    #[qslot]
    fn set_volume(&mut self, volume: usize) {
        println!("set volume to {volume}");
        let _ = self.sender.send(AudioCommand::SetVolume(volume));
    }
}

pub fn create() {
    QApp::new()
        .register::<MainWindow>()
        .load_qml(include_bytes!("../qml/MainWindow.qml"))
        .run();
}