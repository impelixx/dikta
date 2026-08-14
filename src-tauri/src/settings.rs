use crate::models::CustomModel;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub hotkey_push_to_talk: String,
    pub hotkey_toggle: String,
    /// 0.0 (шёпот — тишина) .. 1.0 (стройка)
    pub vad_sensitivity: f32,
    pub silence_hangover_ms: u32,
    pub autostop_push_to_talk: bool,
    pub autostop_toggle: bool,
    pub autopaste_enabled: bool,
    /// Шумоподавление (RNNoise через nnnoiseless) перед распознаванием.
    pub denoise_enabled: bool,
    pub active_model_id: String,
    pub custom_models: Vec<CustomModel>,
    /// None = устройство ввода по умолчанию в системе
    pub input_device: Option<String>,
    pub theme: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            // F13 — частый выбор для push-to-talk, но физически отсутствует
            // на большинстве ноутбучных клавиатур (в т.ч. MacBook). Берём
            // комбинацию, которая точно есть везде.
            hotkey_push_to_talk: "CmdOrCtrl+Shift+D".to_string(),
            hotkey_toggle: "CmdOrCtrl+Shift+Space".to_string(),
            vad_sensitivity: 0.5,
            // 900мс резало живую речь на обычных паузах между фразами —
            // 2с даёт время на вдох и не обрывает диктовку на полуслове.
            silence_hangover_ms: 2000,
            autostop_push_to_talk: false,
            autostop_toggle: true,
            autopaste_enabled: true,
            denoise_enabled: true,
            active_model_id: "giga-ctc-v3".to_string(),
            custom_models: Vec::new(),
            input_device: None,
            theme: "cream".to_string(),
        }
    }
}

impl AppSettings {
    pub fn load(conn: &rusqlite::Connection) -> Self {
        let mut s = Self::default();
        if let Ok(Some(v)) = crate::db::get_setting(conn, "hotkey_push_to_talk") {
            s.hotkey_push_to_talk = v;
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "hotkey_toggle") {
            s.hotkey_toggle = v;
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "vad_sensitivity") {
            if let Ok(f) = v.parse() {
                s.vad_sensitivity = f;
            }
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "silence_hangover_ms") {
            if let Ok(n) = v.parse() {
                s.silence_hangover_ms = n;
            }
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "autostop_push_to_talk") {
            s.autostop_push_to_talk = v == "true";
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "autostop_toggle") {
            s.autostop_toggle = v == "true";
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "autopaste_enabled") {
            s.autopaste_enabled = v == "true";
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "denoise_enabled") {
            s.denoise_enabled = v == "true";
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "active_model_id") {
            s.active_model_id = v;
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "custom_models") {
            if let Ok(list) = serde_json::from_str(&v) {
                s.custom_models = list;
            }
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "input_device") {
            s.input_device = if v.is_empty() { None } else { Some(v) };
        }
        if let Ok(Some(v)) = crate::db::get_setting(conn, "theme") {
            s.theme = v;
        }
        s
    }

    pub fn save_field(conn: &rusqlite::Connection, key: &str, value: &str) -> anyhow::Result<()> {
        crate::db::set_setting(conn, key, value)
    }
}

/// Игривые подписи для слайдера чувствительности автостопа — от "шёпот — тишина"
/// до "стройка", вместо голых цифр порога RMS.
pub fn sensitivity_label(sensitivity: f32) -> &'static str {
    match (sensitivity * 100.0) as u32 {
        0..=15 => "Шёпот — тишина",
        16..=35 => "Тихая комната",
        36..=55 => "Обычный разговор",
        56..=75 => "Шумное кафе",
        76..=90 => "Открытый опенспейс",
        _ => "Стройка",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_sane() {
        let s = AppSettings::default();
        assert!(s.vad_sensitivity >= 0.0 && s.vad_sensitivity <= 1.0);
        assert!(s.autostop_toggle);
        assert!(!s.autostop_push_to_talk);
    }

    #[test]
    fn sensitivity_labels_cover_range() {
        assert_eq!(sensitivity_label(0.0), "Шёпот — тишина");
        assert_eq!(sensitivity_label(1.0), "Стройка");
        assert_eq!(sensitivity_label(0.5), "Обычный разговор");
    }
}
