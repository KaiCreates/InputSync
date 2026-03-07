use anyhow::Result;
use serde::{Deserialize, Serialize};
use tauri::State;
use tokio::sync::mpsc;

use crate::core::session::generate_session_code;
use crate::input::capture::start_capture;
use crate::network::client::connect_to_server;
use crate::network::server::start_server;
use crate::state::{AppStatus, ClientState, ServerState, SharedState};

#[derive(Serialize, Deserialize)]
pub struct StartServerResult {
    pub session_code: String,
    pub local_ip: String,
}

/// Start the server: generate session code, bind ports, begin listening
#[tauri::command]
pub async fn cmd_start_server(state: State<'_, SharedState>) -> Result<StartServerResult, String> {
    let mut locked = state.lock().await;

    if locked.server.is_some() {
        return Err("Server already running".to_string());
    }
    if locked.client.is_some() {
        return Err("Already connected as client".to_string());
    }

    let session_code = generate_session_code();
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    let (input_tx, input_rx) = mpsc::unbounded_channel();

    let handle = start_server(session_code.clone(), input_rx)
        .await
        .map_err(|e| e.to_string())?;

    // Start input capture immediately
    let capture_handle = start_capture(input_tx.clone())
        .map_err(|e| format!("Capture failed: {}", e))
        .ok();

    locked.server = Some(ServerState {
        handle,
        capture_handle,
        session_code: session_code.clone(),
        local_ip: local_ip.clone(),
        input_tx,
        client_count: 0,
    });

    log::info!("Server started. Code: {} IP: {}", session_code, local_ip);

    Ok(StartServerResult {
        session_code,
        local_ip,
    })
}

/// Stop the running server and release all resources
#[tauri::command]
pub async fn cmd_stop_server(state: State<'_, SharedState>) -> Result<(), String> {
    let mut locked = state.lock().await;

    if let Some(srv) = locked.server.take() {
        // Drop capture handle first
        drop(srv.capture_handle);
        // Shutdown server
        srv.handle.shutdown();
        log::info!("Server stopped");
        Ok(())
    } else {
        Err("No server running".to_string())
    }
}

/// Connect this machine as a client to a remote server
#[tauri::command]
pub async fn cmd_connect(
    state: State<'_, SharedState>,
    server_ip: String,
    session_code: String,
) -> Result<(), String> {
    let mut locked = state.lock().await;

    if locked.client.is_some() {
        return Err("Already connected".to_string());
    }
    if locked.server.is_some() {
        return Err("Cannot connect as client while running as server".to_string());
    }

    let (status_tx, status_rx) = mpsc::unbounded_channel();

    let handle = connect_to_server(&server_ip, &session_code, status_tx)
        .await
        .map_err(|e| e.to_string())?;

    let server_addr = format!("{}:24800", server_ip);
    locked.client = Some(ClientState {
        handle,
        server_addr,
        status_rx,
    });

    log::info!("Connected to server {}", server_ip);
    Ok(())
}

/// Disconnect from the server
#[tauri::command]
pub async fn cmd_disconnect(state: State<'_, SharedState>) -> Result<(), String> {
    let mut locked = state.lock().await;

    if let Some(cli) = locked.client.take() {
        cli.handle.disconnect();
        log::info!("Disconnected from server");
        Ok(())
    } else {
        Err("Not connected".to_string())
    }
}

/// Toggle input capture on the server (enable/disable forwarding input to clients)
#[tauri::command]
pub async fn cmd_toggle_capture(state: State<'_, SharedState>) -> Result<bool, String> {
    let mut locked = state.lock().await;

    if let Some(srv) = &mut locked.server {
        if srv.capture_handle.is_some() {
            // Stop capture
            srv.capture_handle.take();
            log::info!("Input capture stopped");
            Ok(false)
        } else {
            // Start capture
            let handle = start_capture(srv.input_tx.clone())
                .map_err(|e| format!("Capture error: {}", e))?;
            srv.capture_handle = Some(handle);
            log::info!("Input capture started");
            Ok(true)
        }
    } else {
        Err("Server not running".to_string())
    }
}

/// Get the current application status
#[tauri::command]
pub async fn cmd_get_status(state: State<'_, SharedState>) -> Result<AppStatus, String> {
    Ok(state.lock().await.status())
}
