use tauri::WebviewWindow;

pub fn set_pet_window_defaults(window: &WebviewWindow) {
    let _ = window.set_always_on_top(true);
}
