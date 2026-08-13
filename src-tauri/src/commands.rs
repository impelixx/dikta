use crate::audio;
use crate::db::{self, DayStat, OverallStats, Recording};
use crate::models::{self, CustomFiles, CustomModel, ModelKind};
use crate::settings::{self, AppSettings};
use crate::state::{ActiveRecognizer, AppState};
use serde::Serialize;
use tauri::{AppHandle, Emitter, State};

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppSettings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn set_setting(
    app: tauri::AppHandle,
    state: State<AppState>,
    key: String,
    value: String,
) -> Result<(), String> {
    {
        let conn = state.db.0.lock().unwrap();
        AppSettings::save_field(&conn, &key, &value).map_err(|e| e.to_string())?;
    }
    {
        let mut s = state.settings.lock().unwrap();
        *s = {
            let conn = state.db.0.lock().unwrap();
            AppSettings::load(&conn)
        };
    }
    if key.starts_with("hotkey_") {
        crate::hotkeys::register_hotkeys(&app).map_err(|e| e.to_string())?;
    }
    if key == "input_device" {
        let device = if value.is_empty() { None } else { Some(value.as_str()) };
        state.audio.switch_device(device).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[derive(Serialize)]
pub struct ModelListItem {
    #[serde(flatten)]
    entry: crate::models::ModelEntry,
    downloaded: bool,
    active: bool,
}

#[tauri::command]
pub fn list_models(state: State<AppState>) -> Vec<ModelListItem> {
    let settings = state.settings.lock().unwrap().clone();
    // "Активна" должна значить "реально загружена в память", а не просто
    // "так записано в настройках" — иначе после скачивания модели, которая
    // уже числилась активной, пользователю негде нажать "использовать".
    let loaded_id = state.recognizer.lock().unwrap().as_ref().map(|r| r.model_id.clone());
    models::full_catalog(&settings.custom_models)
        .into_iter()
        .map(|entry| {
            let downloaded = models::is_downloaded(&state.models_dir, &entry);
            let active = loaded_id.as_deref() == Some(entry.id());
            ModelListItem { entry, downloaded, active }
        })
        .collect()
}

#[derive(Serialize, Clone)]
struct DownloadProgress {
    id: String,
    downloaded_bytes: u64,
    total_bytes: u64,
}

#[tauri::command]
pub fn download_model(app: AppHandle, state: State<AppState>, id: String) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    let entry = models::find(&id, &settings.custom_models).ok_or("модель не найдена")?;
    let dir = state.models_dir.clone();
    let app_for_progress = app.clone();
    let id_for_progress = id.clone();
    let result = models::download(&entry, &dir, move |downloaded, total| {
        let _ = app_for_progress.emit(
            "model-download-progress",
            DownloadProgress { id: id_for_progress.clone(), downloaded_bytes: downloaded, total_bytes: total },
        );
    });
    match result {
        Ok(()) => {
            let _ = app.emit("model-download-done", id);
            Ok(())
        }
        Err(e) => {
            let _ = app.emit("model-download-error", format!("{id}: {e}"));
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub fn set_active_model(state: State<AppState>, id: String) -> Result<(), String> {
    let settings = state.settings.lock().unwrap().clone();
    let entry = models::find(&id, &settings.custom_models).ok_or("модель не найдена")?;
    if !models::is_downloaded(&state.models_dir, &entry) {
        return Err("модель ещё не скачана".to_string());
    }
    let dir = models::model_root_dir(&state.models_dir, &entry);
    let inner = crate::asr::Recognizer::from_model_dir(&dir, entry.kind(), 2).map_err(|e| e.to_string())?;
    *state.recognizer.lock().unwrap() = Some(ActiveRecognizer { model_id: id.clone(), inner });
    {
        let conn = state.db.0.lock().unwrap();
        AppSettings::save_field(&conn, "active_model_id", &id).map_err(|e| e.to_string())?;
    }
    state.settings.lock().unwrap().active_model_id = id;
    Ok(())
}

#[tauri::command]
pub fn hf_list_files(repo_id: String) -> Result<Vec<String>, String> {
    models::hf_list_files(&repo_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_custom_model(
    state: State<AppState>,
    repo_id: String,
    kind: ModelKind,
    tokens: String,
    model: Option<String>,
    encoder: Option<String>,
    decoder: Option<String>,
    joiner: Option<String>,
) -> Result<String, String> {
    let files = match kind {
        ModelKind::Ctc => CustomFiles::Ctc {
            model: model.ok_or("не указан файл модели")?,
            tokens,
        },
        ModelKind::Transducer => CustomFiles::Transducer {
            encoder: encoder.ok_or("не указан encoder")?,
            decoder: decoder.ok_or("не указан decoder")?,
            joiner: joiner.ok_or("не указан joiner")?,
            tokens,
        },
    };
    let id = format!("custom:{}", repo_id.replace('/', "_"));
    let custom = CustomModel { id: id.clone(), name: repo_id.clone(), repo_id, kind, files };

    let mut settings = state.settings.lock().unwrap();
    settings.custom_models.retain(|m| m.id != custom.id);
    settings.custom_models.push(custom);
    let json = serde_json::to_string(&settings.custom_models).map_err(|e| e.to_string())?;
    let conn = state.db.0.lock().unwrap();
    AppSettings::save_field(&conn, "custom_models", &json).map_err(|e| e.to_string())?;
    Ok(id)
}

#[tauri::command]
pub fn remove_custom_model(state: State<AppState>, id: String) -> Result<(), String> {
    let mut settings = state.settings.lock().unwrap();
    settings.custom_models.retain(|m| m.id != id);
    let json = serde_json::to_string(&settings.custom_models).map_err(|e| e.to_string())?;
    let conn = state.db.0.lock().unwrap();
    AppSettings::save_field(&conn, "custom_models", &json).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn list_input_devices() -> Vec<String> {
    audio::list_input_devices()
}

#[tauri::command]
pub fn sensitivity_label(value: f32) -> String {
    settings::sensitivity_label(value).to_string()
}

#[tauri::command]
pub fn search_history(
    state: State<AppState>,
    query: String,
    limit: i64,
) -> Result<Vec<Recording>, String> {
    let conn = state.db.0.lock().unwrap();
    db::search_recordings(&conn, &query, limit).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_history_item(state: State<AppState>, id: i64) -> Result<(), String> {
    let conn = state.db.0.lock().unwrap();
    db::delete_recording(&conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_overall_stats(state: State<AppState>) -> Result<OverallStats, String> {
    let conn = state.db.0.lock().unwrap();
    db::overall_stats(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_daily_stats(state: State<AppState>, days: i64) -> Result<Vec<DayStat>, String> {
    let conn = state.db.0.lock().unwrap();
    db::daily_stats(&conn, days).map_err(|e| e.to_string())
}
