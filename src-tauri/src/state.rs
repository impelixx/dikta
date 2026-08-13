use crate::asr::Recognizer;
use crate::audio::AudioEngine;
use crate::db::Db;
use crate::settings::AppSettings;
use std::path::PathBuf;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordingMode {
    PushToTalk,
    Toggle,
}

pub struct ActiveRecognizer {
    pub model_id: String,
    pub inner: Recognizer,
}

pub struct AppState {
    pub db: Db,
    pub recognizer: Mutex<Option<ActiveRecognizer>>,
    pub models_dir: PathBuf,
    pub audio: AudioEngine,
    pub settings: Mutex<AppSettings>,
    pub active_mode: Mutex<Option<RecordingMode>>,
}
