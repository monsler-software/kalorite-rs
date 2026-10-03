pub enum AudioCommand {
    LoadTrack(String),
    Play,
    Stop,
    Pause,
    GetVolume,
    GetPosition,
    SetVolume(usize),
    SetPosition(usize)
}

pub enum AudioResponse {
    Position(usize),
    Volume(usize)

}