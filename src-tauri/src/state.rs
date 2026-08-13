use crate::asr::GigaAmRecognizer;
use crate::audio::AudioEngine;
use crate::db::Db;
use crate::settings::AppSettings;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    PushToTalk,
    Toggle,
}

pub struct AppState {
    pub db: Db,
    pub recognizer: GigaAmRecognizer,
    pub audio: AudioEngine,
    pub settings: Mutex<AppSettings>,
    pub active_mode: Mutex<Option<RecordingMode>>,
}
