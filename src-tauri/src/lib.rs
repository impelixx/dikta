pub mod asr;
pub mod audio;
pub mod commands;
pub mod db;
pub mod hotkeys;
pub mod paste;
pub mod settings;
pub mod state;
pub mod vad;

use asr::GigaAmRecognizer;
use audio::AudioEngine;
use settings::AppSettings;
use state::AppState;
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::Manager;

fn model_dir(app: &tauri::AppHandle) -> PathBuf {
    // В бандле ресурсы лежат рядом с исполняемым файлом; в dev-режиме - в src-tauri/resources.
    if let Ok(resource_dir) = app.path().resource_dir() {
        let bundled = resource_dir
            .join("resources/model/sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24");
        if bundled.exists() {
            return bundled;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("resources/model/sherpa-onnx-nemo-ctc-giga-am-russian-2024-10-24")
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

            let dir = model_dir(&handle);
            let model_path = dir.join("model.int8.onnx");
            let tokens_path = dir.join("tokens.txt");
            let recognizer = GigaAmRecognizer::new(
                model_path.to_str().expect("некорректный путь к модели"),
                tokens_path.to_str().expect("некорректный путь к токенам"),
                2,
            )
            .expect("не удалось инициализировать распознаватель GigaAM");

            let audio = AudioEngine::new().expect("не удалось инициализировать аудиовход");

            app.manage(AppState {
                db,
                recognizer,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
