pub mod commands;
pub mod error;
pub mod export;
pub mod menu;
pub mod photo;
pub mod state;
pub mod warp;

use tauri_plugin_window_state::StateFlags;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_window_state::Builder::default()
                .with_state_flags(StateFlags::SIZE)
                .build(),
        )
        .manage(state::AppState::default())
        .setup(|app| {
            menu::install(app.handle())?;
            Ok(())
        })
        .on_window_event(commands::on_window_event)
        .invoke_handler(tauri::generate_handler![
            commands::choose_photo,
            commands::open_photo,
            commands::open_sample,
            commands::choose_export_path,
            commands::export_photo,
            commands::cancel_export,
            commands::confirm_discard,
            commands::set_ui_state,
        ])
        .run(tauri::generate_context!())
        .expect("error while running FPV Shifter");
}
