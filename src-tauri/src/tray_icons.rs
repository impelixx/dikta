use tauri::image::Image;
use tauri::AppHandle;

const IDLE: &[u8] = include_bytes!("../icons/tray/tray-idle.png");
const LISTEN_A: &[u8] = include_bytes!("../icons/tray/tray-listen-a.png");
const LISTEN_B: &[u8] = include_bytes!("../icons/tray/tray-listen-b.png");
const PROCESS_A: &[u8] = include_bytes!("../icons/tray/tray-process-a.png");
const PROCESS_B: &[u8] = include_bytes!("../icons/tray/tray-process-b.png");

fn set_icon(app: &AppHandle, bytes: &[u8]) {
    if let Some(tray) = app.tray_by_id(crate::TRAY_ID) {
        if let Ok(img) = Image::from_bytes(bytes) {
            let _ = tray.set_icon(Some(img));
        }
    }
}

pub fn set_idle(app: &AppHandle) {
    set_icon(app, IDLE);
}

/// Живая иконка трея, пока идёт запись — два кадра точки-индикатора,
/// чередуются, пока `still_active` возвращает true.
pub fn animate_listening(app: AppHandle, still_active: impl Fn() -> bool + Send + 'static) {
    std::thread::spawn(move || {
        let mut frame = false;
        while still_active() {
            set_icon(&app, if frame { LISTEN_B } else { LISTEN_A });
            frame = !frame;
            std::thread::sleep(std::time::Duration::from_millis(450));
        }
    });
}

/// Живая иконка трея на время распознавания — свои два кадра, отдельный цвет.
pub fn animate_processing(app: AppHandle, still_processing: impl Fn() -> bool + Send + 'static) {
    std::thread::spawn(move || {
        let mut frame = false;
        while still_processing() {
            set_icon(&app, if frame { PROCESS_B } else { PROCESS_A });
            frame = !frame;
            std::thread::sleep(std::time::Duration::from_millis(350));
        }
        set_idle(&app);
    });
}
