use enigo::{Enigo, Keyboard, Settings};
use tauri::AppHandle;
use tauri_plugin_clipboard_manager::ClipboardExt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum PasteOutcome {
    AutoInserted,
    CopiedToClipboard,
}

#[cfg(target_os = "macos")]
mod macos_ax {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
    }
    /// На macOS enigo может молча "успешно" вернуть Ok(), даже если приложению
    /// не выдан доступ в Privacy & Security → Accessibility — CGEventPost просто
    /// не долетает до целевого приложения без явной ошибки. Поэтому здесь
    /// проверяем разрешение заранее, а не доверяем результату enigo.
    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }
}

#[cfg(not(target_os = "macos"))]
mod macos_ax {
    pub fn is_trusted() -> bool {
        true
    }
}

/// Пытается напечатать текст в текущее фокусное поле через Accessibility (enigo).
/// Буфер обмена обновляется в любом случае — это безусловная страховка, а не
/// просто fallback на случай ошибки: на macOS отсутствие Accessibility-доступа
/// не всегда проявляется как ошибка, поэтому нельзя полагаться только на код
/// возврата enigo.
pub fn insert_text(app: &AppHandle, text: &str, autopaste_enabled: bool) -> PasteOutcome {
    let _ = app.clipboard().write_text(text.to_string());

    if autopaste_enabled && macos_ax::is_trusted() {
        if let Ok(mut enigo) = Enigo::new(&Settings::default()) {
            if enigo.text(text).is_ok() {
                return PasteOutcome::AutoInserted;
            }
        }
    }
    PasteOutcome::CopiedToClipboard
}
