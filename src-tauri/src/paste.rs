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
    use core_foundation::base::TCFType;
    use core_foundation::boolean::CFBoolean;
    use core_foundation::dictionary::{CFDictionary, CFDictionaryRef};
    use core_foundation::string::{CFString, CFStringRef};

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> bool;
        fn AXIsProcessTrustedWithOptions(options: CFDictionaryRef) -> bool;
        static kAXTrustedCheckOptionPrompt: CFStringRef;
    }
    /// На macOS enigo может молча "успешно" вернуть Ok(), даже если приложению
    /// не выдан доступ в Privacy & Security → Accessibility — CGEventPost просто
    /// не долетает до целевого приложения без явной ошибки. Поэтому здесь
    /// проверяем разрешение заранее, а не доверяем результату enigo.
    pub fn is_trusted() -> bool {
        unsafe { AXIsProcessTrusted() }
    }

    /// Обычный AXIsProcessTrusted() ничего не просит — только тихо проверяет,
    /// поэтому без единого вызова *WithOptions(prompt: true) система никогда не
    /// покажет нативный диалог, и приложение может даже не появиться в списке
    /// Privacy & Security → Accessibility, чтобы разрешение выдать вручную.
    /// Вызывать один раз (на старте), а не при каждой вставке — иначе диалог
    /// будет всплывать на каждую диктовку, пока разрешение не выдано.
    pub fn request_trust_prompt() {
        unsafe {
            let key = CFString::wrap_under_get_rule(kAXTrustedCheckOptionPrompt);
            let dict = CFDictionary::from_CFType_pairs(&[(key.as_CFType(), CFBoolean::true_value().as_CFType())]);
            AXIsProcessTrustedWithOptions(dict.as_concrete_TypeRef());
        }
    }
}

#[cfg(not(target_os = "macos"))]
mod macos_ax {
    pub fn is_trusted() -> bool {
        true
    }
    pub fn request_trust_prompt() {}
}

/// Просит систему показать диалог разрешения Accessibility, если оно ещё не
/// выдано — см. `macos_ax::request_trust_prompt`. Вызывается один раз при
/// старте приложения (см. lib.rs), не из каждой попытки вставки текста.
pub fn request_trust_prompt_if_needed() {
    if !macos_ax::is_trusted() {
        macos_ax::request_trust_prompt();
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
