use crate::db;
use crate::paste::insert_text;
use crate::state::{AppState, RecordingMode};
use crate::vad::SilenceDetector;
use std::time::Instant;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

/// Регистрирует push-to-talk и toggle хоткеи по текущим настройкам.
/// Вызывается при старте и при изменении хоткеев в настройках (сначала unregister_all).
pub fn register_hotkeys(app: &AppHandle) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    drop(state);

    let gs = app.global_shortcut();
    gs.unregister_all()?;

    let push_shortcut: Shortcut = settings
        .hotkey_push_to_talk
        .parse()
        .map_err(|e| anyhow::anyhow!("неверный хоткей push-to-talk: {e}"))?;
    let toggle_shortcut: Shortcut = settings
        .hotkey_toggle
        .parse()
        .map_err(|e| anyhow::anyhow!("неверный хоткей toggle: {e}"))?;

    gs.register(push_shortcut)?;
    gs.register(toggle_shortcut)?;

    Ok(())
}

pub fn handle_shortcut_event(app: &AppHandle, shortcut: &Shortcut, event_state: ShortcutState) {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let push_shortcut: Option<Shortcut> = settings.hotkey_push_to_talk.parse().ok();
    let toggle_shortcut: Option<Shortcut> = settings.hotkey_toggle.parse().ok();
    drop(state);

    let is_push = push_shortcut.as_ref() == Some(shortcut);
    let is_toggle = toggle_shortcut.as_ref() == Some(shortcut);

    if is_push {
        match event_state {
            ShortcutState::Pressed => start_recording(app, RecordingMode::PushToTalk),
            ShortcutState::Released => stop_recording(app),
        }
    } else if is_toggle && event_state == ShortcutState::Pressed {
        let state = app.state::<AppState>();
        let active = *state.active_mode.lock().unwrap();
        drop(state);
        if active.is_some() {
            stop_recording(app);
        } else {
            start_recording(app, RecordingMode::Toggle);
        }
    }
}

fn start_recording(app: &AppHandle, mode: RecordingMode) {
    let state = app.state::<AppState>();
    {
        let mut active = state.active_mode.lock().unwrap();
        if active.is_some() {
            return;
        }
        *active = Some(mode);
    }
    let settings = state.settings.lock().unwrap().clone();

    let autostop = match mode {
        RecordingMode::PushToTalk => settings.autostop_push_to_talk,
        RecordingMode::Toggle => settings.autostop_toggle,
    };
    let vad = if autostop {
        Some(SilenceDetector::new(
            settings.vad_sensitivity,
            settings.silence_hangover_ms,
            16000,
        ))
    } else {
        None
    };

    state.audio.start(vad);
    let _ = app.emit("recording-started", mode == RecordingMode::Toggle);

    if autostop {
        spawn_silence_watcher(app.clone());
    }
    spawn_level_watcher(app.clone());
    spawn_partial_transcript_watcher(app.clone());
}

/// "Живые субтитры": пока запись идёт, периодически передекодируем уже
/// наговоренное целиком. Это не настоящий streaming (нет отдельной online-
/// модели), а честный компромисс — модель быстрая (RTF << 1), так что
/// повторный полный декод растущего буфера раз в ~900мс не создаёт заметной
/// нагрузки для типичной длины диктовки.
fn spawn_partial_transcript_watcher(app: AppHandle) {
    std::thread::spawn(move || {
        let mut last_text = String::new();
        loop {
            std::thread::sleep(std::time::Duration::from_millis(900));
            let state = app.state::<AppState>();
            let still_active = state.active_mode.lock().unwrap().is_some();
            if !still_active {
                return;
            }
            let samples = state.audio.snapshot();
            if samples.len() < 16000 / 2 {
                continue; // меньше 0.5с — decode не имеет смысла
            }
            let text = {
                let recognizer = state.recognizer.lock().unwrap();
                match recognizer.as_ref() {
                    Some(r) => r.inner.decode(&samples),
                    None => return,
                }
            };
            drop(state);
            if text != last_text && !text.trim().is_empty() {
                last_text = text.clone();
                let _ = app.emit("partial-transcript", text);
            }
        }
    });
}

/// Рассылает уровень громкости во фронтенд для живой волны, пока идёт запись.
fn spawn_level_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(80));
        let state = app.state::<AppState>();
        let still_active = state.active_mode.lock().unwrap().is_some();
        if !still_active {
            return;
        }
        let level = state.audio.recent_level();
        drop(state);
        let _ = app.emit("audio-level", level);
    });
}

/// Лёгкий поллер, останавливающий запись, когда AudioEngine зафиксировал
/// достаточную тишину. Живёт, пока идёт конкретная сессия записи.
fn spawn_silence_watcher(app: AppHandle) {
    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_millis(100));
        let state = app.state::<AppState>();
        let still_active = state.active_mode.lock().unwrap().is_some();
        if !still_active {
            return;
        }
        if state.audio.silence_triggered() {
            drop(state);
            stop_recording(&app);
            return;
        }
    });
}

fn stop_recording(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mode = {
        let mut active = state.active_mode.lock().unwrap();
        match active.take() {
            Some(m) => m,
            None => return,
        }
    };
    let started_at = Instant::now();
    let samples = state.audio.stop();
    let duration_ms = (samples.len() as f64 / 16000.0 * 1000.0) as i64;
    let _ = app.emit("recording-stopped", ());

    if samples.is_empty() || duration_ms < 200 {
        return; // слишком коротко, скорее всего случайное нажатие
    }

    let text = {
        let recognizer = state.recognizer.lock().unwrap();
        match recognizer.as_ref() {
            Some(r) => r.inner.decode(&samples),
            None => {
                let _ = app.emit("no-model-active", ());
                return;
            }
        }
    };
    if text.trim().is_empty() {
        return;
    }
    let quality = crate::asr::signal_quality(&samples);
    let settings = state.settings.lock().unwrap().clone();
    let mode_str = match mode {
        RecordingMode::PushToTalk => "push_to_talk",
        RecordingMode::Toggle => "toggle",
    };

    let outcome = insert_text(app, &text, settings.autopaste_enabled);

    let conn = state.db.0.lock().unwrap();
    let _ = db::insert_recording(&conn, &text, duration_ms, quality, mode_str);
    drop(conn);

    let _ = app.emit(
        "transcription-done",
        serde_json::json!({ "text": text, "outcome": format!("{:?}", outcome) }),
    );
    let _ = started_at; // зарезервировано под будущую метрику задержки инференса
}
