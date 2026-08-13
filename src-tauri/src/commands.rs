use crate::db::{self, DayStat, OverallStats, Recording};
use crate::settings::{self, AppSettings};
use crate::state::AppState;
use tauri::State;

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
    Ok(())
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
