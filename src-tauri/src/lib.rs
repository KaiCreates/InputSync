mod commands;
mod core;
mod input;
mod network;
mod state;

use commands::{
    cmd_connect, cmd_disconnect, cmd_get_status, cmd_start_server, cmd_stop_server,
    cmd_toggle_capture,
};
use state::new_shared_state;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let shared_state = new_shared_state();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .manage(shared_state)
        .invoke_handler(tauri::generate_handler![
            cmd_start_server,
            cmd_stop_server,
            cmd_connect,
            cmd_disconnect,
            cmd_toggle_capture,
            cmd_get_status,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running InputSync");
}
