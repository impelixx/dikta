use enigo::{Enigo, Keyboard, Settings};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PasteOutcome {
    AutoInserted,
    CopiedToClipboard,
}

/// Пытается напечатать текст в текущее фокусное поле через Accessibility (enigo).
/// Если это не удаётся (нет разрешения, недоступный контекст) — копирует в буфер
/// обмена, чтобы пользователь вставил вручную. Не оставляет пользователя без текста
/// ни в каком случае.
pub fn insert_text(app: &AppHandle, text: &str, autopaste_enabled: bool) -> PasteOutcome {
    if autopaste_enabled {
        if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
            if enigo.text(text).is_ok() {
                return PasteOutcome::AutoInserted;
            }
        }
    }
    let _ = app.clipboard().write_text(text.to_string());
    PasteOutcome::CopiedToClipboard
}
