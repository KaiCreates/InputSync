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

/// Show a native error dialog and exit. On Windows this is a MessageBoxW;
/// on other platforms we print to stderr.
fn fatal_error(msg: &str) -> ! {
    #[cfg(target_os = "windows")]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};

        let caption: Vec<u16> = "InputSync — Startup Error\0"
            .encode_utf16()
            .collect();
        let text: Vec<u16> = format!("{}\0", msg).encode_utf16().collect();

        unsafe {
            MessageBoxW(
                None,
                PCWSTR(text.as_ptr()),
                PCWSTR(caption.as_ptr()),
                MB_OK | MB_ICONERROR,
            );
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        eprintln!("Fatal error: {}", msg);
    }
    std::process::exit(1);
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let shared_state = new_shared_state();

    if let Err(e) = tauri::Builder::default()
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
    {
        #[cfg(target_os = "windows")]
        let msg = if e.to_string().to_lowercase().contains("webview") {
            format!(
                "InputSync could not start because Microsoft Edge WebView2 Runtime is missing or damaged.\r\n\r\n\
                Error: {e}\r\n\r\n\
                Fix: Download and install WebView2 from\r\n\
                https://go.microsoft.com/fwlink/p/?LinkId=2124703\r\n\r\n\
                Then relaunch InputSync."
            )
        } else {
            format!("InputSync failed to start.\r\n\r\nError: {e}")
        };

        #[cfg(not(target_os = "windows"))]
        let msg = format!("InputSync failed to start.\n\nError: {e}");

        fatal_error(&msg);
    }
}
