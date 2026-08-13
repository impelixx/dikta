pub mod asr;
pub mod audio;
pub mod commands;
pub mod db;
pub mod hotkeys;
pub mod models;
pub mod paste;
pub mod settings;
pub mod state;
pub mod vad;

use asr::Recognizer;
use audio::AudioEngine;
use settings::AppSettings;
use state::{ActiveRecognizer, AppState};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

fn models_base_dir() -> PathBuf {
    let mut dir = dirs::data_dir().unwrap_or_else(std::env::temp_dir);
    dir.push("dikta");
    dir.push("models");
    dir
}

fn load_active_recognizer(settings: &AppSettings, models_dir: &PathBuf) -> Option<ActiveRecognizer> {
    let entry = models::find(&settings.active_model_id, &settings.custom_models)?;
    if !models::is_downloaded(models_dir, &entry) {
        return None;
    }
    let dir = models::model_root_dir(models_dir, &entry);
    match Recognizer::from_model_dir(&dir, entry.kind(), 2) {
        Ok(inner) => Some(ActiveRecognizer {
            model_id: entry.id().to_string(),
            inner,
        }),
        Err(e) => {
            eprintln!("[asr] не удалось загрузить модель {}: {e}", entry.id());
            None
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, shortcut, event| {
                    hotkeys::handle_shortcut_event(app, shortcut, event.state());
                })
                .build(),
        )
        .setup(|app| {
            let handle = app.handle().clone();
            let db = db::open().expect("не удалось открыть базу данных");
            let settings = {
                let conn = db.0.lock().unwrap();
                AppSettings::load(&conn)
            };

            let models_dir = models_base_dir();
            let recognizer = load_active_recognizer(&settings, &models_dir);

            // Если сохранённое устройство ввода пропало (отключили микрофон и т.п.),
            // тихо откатываемся на системное по умолчанию вместо падения приложения.
            let audio = AudioEngine::new(settings.input_device.as_deref())
                .or_else(|_| AudioEngine::new(None))
                .expect("не удалось инициализировать аудиовход");

            app.manage(AppState {
                db,
                recognizer: Mutex::new(recognizer),
                models_dir,
                audio,
                settings: Mutex::new(settings),
                active_mode: Mutex::new(None),
            });

            hotkeys::register_hotkeys(&handle)?;

            // Трей: минимальная точка доступа, приложение живёт в фоне без открытого окна.
            let show = MenuItem::with_id(app, "show", "Открыть Дикту", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| match event.id.as_ref() {
                    "quit" => app.exit(0),
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        let app = tray.app_handle();
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_setting,
            commands::sensitivity_label,
            commands::search_history,
            commands::delete_history_item,
            commands::get_overall_stats,
            commands::get_daily_stats,
            commands::list_models,
            commands::download_model,
            commands::set_active_model,
            commands::hf_list_files,
            commands::add_custom_model,
            commands::remove_custom_model,
            commands::list_input_devices,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
