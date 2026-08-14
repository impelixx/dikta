pub mod asr;
pub mod audio;
pub mod commands;
pub mod db;
pub mod hotkeys;
pub mod models;
pub mod paste;
pub mod settings;
pub mod state;
pub mod tray_icons;
pub mod vad;

use asr::Recognizer;
use audio::AudioEngine;
use settings::AppSettings;
use state::{ActiveRecognizer, AppState};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, Submenu};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

pub const TRAY_ID: &str = "main-tray";

/// Перестраивает трей-меню целиком (список моделей меняется динамически —
/// после скачивания или ручной активации), чтобы модель можно было переключить
/// в один клик, не открывая окно настроек.
pub fn rebuild_tray_menu(app: &AppHandle) -> tauri::Result<()> {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return Ok(());
    };
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let loaded_id = state.recognizer.lock().unwrap().as_ref().map(|r| r.model_id.clone());
    drop(state);

    let downloaded: Vec<models::ModelEntry> = models::full_catalog(&settings.custom_models)
        .into_iter()
        .filter(|entry| models::is_downloaded(&app.state::<AppState>().models_dir, entry))
        .collect();

    // Группируем по типу движка (CTC/Transducer/Whisper/whisper.cpp), чтобы
    // список моделей в трее можно было фильтровать вложенными подменю, а не
    // листать один длинный плоский список.
    let kind_label = |k: models::ModelKind| -> &'static str {
        match k {
            models::ModelKind::Ctc => "CTC",
            models::ModelKind::Transducer => "Transducer",
            models::ModelKind::Whisper => "Whisper (ONNX)",
            models::ModelKind::WhisperCpp => "Whisper (whisper.cpp)",
        }
    };
    let kinds = [
        models::ModelKind::Ctc,
        models::ModelKind::Transducer,
        models::ModelKind::Whisper,
        models::ModelKind::WhisperCpp,
    ];

    let mut group_submenus: Vec<Submenu<tauri::Wry>> = Vec::new();
    // CheckMenuItem'ы должны жить, пока живо меню — держим их в отдельном
    // векторе на весь остаток функции, а не только внутри цикла.
    let mut all_items: Vec<CheckMenuItem<tauri::Wry>> = Vec::new();
    for kind in kinds {
        let entries: Vec<&models::ModelEntry> = downloaded.iter().filter(|e| e.kind() == kind).collect();
        if entries.is_empty() {
            continue;
        }
        let start = all_items.len();
        for entry in &entries {
            if let Ok(item) = CheckMenuItem::with_id(
                app,
                format!("model:{}", entry.id()),
                entry.name(),
                true,
                loaded_id.as_deref() == Some(entry.id()),
                None::<&str>,
            ) {
                all_items.push(item);
            }
        }
        let refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
            all_items[start..].iter().map(|i| i as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
        if let Ok(submenu) = Submenu::with_items(app, kind_label(kind), true, &refs) {
            group_submenus.push(submenu);
        }
    }

    let group_refs: Vec<&dyn tauri::menu::IsMenuItem<tauri::Wry>> =
        group_submenus.iter().map(|s| s as &dyn tauri::menu::IsMenuItem<tauri::Wry>).collect();
    let models_submenu = Submenu::with_items(app, "Модель", true, &group_refs)?;

    let show = MenuItem::with_id(app, "show", "Открыть Дикту", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &models_submenu, &quit])?;
    tray.set_menu(Some(menu))?;
    Ok(())
}

/// Показывает главное окно и возвращает иконку в Dock/Cmd+Tab на macOS —
/// только пока окно реально открыто, чтобы по умолчанию оставаться в трее.
fn show_main_window(app: &AppHandle) {
    #[cfg(target_os = "macos")]
    let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.show();
        let _ = w.set_focus();
    }
}

/// Общая логика активации модели — используется и Tauri-командой из настроек,
/// и кликом по трей-меню.
pub fn activate_model(app: &AppHandle, id: &str) -> anyhow::Result<()> {
    let state = app.state::<AppState>();
    let settings = state.settings.lock().unwrap().clone();
    let entry = models::find(id, &settings.custom_models).ok_or_else(|| anyhow::anyhow!("модель не найдена"))?;
    if !models::is_downloaded(&state.models_dir, &entry) {
        anyhow::bail!("модель ещё не скачана");
    }
    let dir = models::model_root_dir(&state.models_dir, &entry);
    let inner = Recognizer::from_entry(&dir, &entry, 2)?;
    *state.recognizer.lock().unwrap() = Some(ActiveRecognizer { model_id: id.to_string(), inner });
    {
        let conn = state.db.0.lock().unwrap();
        AppSettings::save_field(&conn, "active_model_id", id)?;
    }
    state.settings.lock().unwrap().active_model_id = id.to_string();
    drop(state);
    let _ = rebuild_tray_menu(app);
    Ok(())
}

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
    match Recognizer::from_entry(&dir, &entry, 2) {
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

/// Плавающее окно-индикатор поверх всех приложений — единственный способ
/// увидеть статус записи/распознавания, когда фокус в другой программе
/// (а он почти всегда там, раз хоткей глобальный).
fn create_overlay_window(app: &tauri::App) -> tauri::Result<()> {
    let width = 400.0;
    let height = 170.0;
    let mut builder = tauri::WebviewWindowBuilder::new(app, "overlay", tauri::WebviewUrl::App("overlay.html".into()))
        .title("Дикта")
        .inner_size(width, height)
        .resizable(false)
        .decorations(false)
        .transparent(true)
        .always_on_top(true)
        .skip_taskbar(true)
        .shadow(false)
        .focused(false)
        .visible(false);

    if let Ok(Some(monitor)) = app.primary_monitor() {
        let scale = monitor.scale_factor();
        let screen_size = monitor.size().to_logical::<f64>(scale);
        let x = (screen_size.width - width) / 2.0;
        let y = screen_size.height - height - 90.0;
        builder = builder.position(x.max(0.0), y.max(0.0));
    }

    builder.build()?;
    Ok(())
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
                processing: std::sync::atomic::AtomicBool::new(false),
            });

            hotkeys::register_hotkeys(&handle)?;

            // Трей: минимальная точка доступа, приложение живёт в фоне без открытого окна.
            // Меню строится динамически (rebuild_tray_menu), тут — только заготовка.
            let show = MenuItem::with_id(app, "show", "Открыть Дикту", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "Выход", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let idle_icon = tauri::image::Image::from_bytes(include_bytes!("../icons/tray/tray-idle.png"))
                .unwrap_or_else(|_| app.default_window_icon().unwrap().clone());
            TrayIconBuilder::with_id(TRAY_ID)
                .icon(idle_icon)
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(move |app, event| {
                    let id = event.id.as_ref();
                    if let Some(model_id) = id.strip_prefix("model:") {
                        if let Err(e) = activate_model(app, model_id) {
                            eprintln!("[tray] не удалось переключить модель: {e}");
                        }
                        return;
                    }
                    match id {
                        "quit" => app.exit(0),
                        "show" => show_main_window(app),
                        _ => {}
                    }
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        show_main_window(tray.app_handle());
                    }
                })
                .build(app)?;

            let _ = rebuild_tray_menu(&handle);

            // Крестик прячет окно в трей вместо выхода из приложения — оно
            // остаётся живо в фоне (хоткеи, оверлей), выход только через
            // трей-меню "Выход". Иконка в Dock/Cmd+Tab появляется только
            // пока окно реально открыто.
            if let Some(main_window) = app.get_webview_window("main") {
                let window_to_hide = main_window.clone();
                let app_for_hide = handle.clone();
                main_window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                        #[cfg(target_os = "macos")]
                        let _ = app_for_hide.set_activation_policy(tauri::ActivationPolicy::Accessory);
                    }
                });
            }

            create_overlay_window(app)?;

            // Только трей — без иконки в Dock (macOS) и без записи в панели задач.
            #[cfg(target_os = "macos")]
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
