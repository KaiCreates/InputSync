use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use tauri::State;
use tokio::sync::mpsc;

use crate::core::session::generate_session_code;
use crate::input::capture::start_capture;
use crate::network::client::connect_to_server;
use crate::network::server::start_server;
use crate::state::{AppStatus, ClientState, ServerState, SharedState};

/// Bounded input channel capacity — drops oldest-ish events under load (Fix #7)
const INPUT_CHANNEL_CAP: usize = 512;

#[derive(Serialize, Deserialize)]
pub struct StartServerResult {
    pub session_code: String,
    pub local_ip: String,
}

/// Start the server: generate session code, bind ports, start input capture.
/// If a server is already running it is stopped automatically before the new
/// one starts — no "address already in use" error on restart.
#[tauri::command]
pub async fn cmd_start_server(state: State<'_, SharedState>) -> Result<StartServerResult, String> {
    let mut locked = state.lock().await;

    // Auto-stop any existing server so the user can restart cleanly.
    if let Some(srv) = locked.server.take() {
        drop(srv.capture_handle);
        srv.handle.shutdown(); // cancels TCP loop → port 24800 released immediately
        log::info!("Previous server stopped before restart");
    }

    if locked.client.is_some() {
        return Err("Cannot start server while connected as client".into());
    }

    let session_code = generate_session_code();
    let local_ip = local_ip_address::local_ip()
        .map(|ip| ip.to_string())
        .unwrap_or_else(|_| "127.0.0.1".to_string());

    // Bounded channel — Fix #7
    let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_CAP);

    // Fix #5 — shared AtomicUsize between server task and ServerState
    let client_count = Arc::new(AtomicUsize::new(0));

    let handle = start_server(session_code.clone(), input_rx, client_count.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Fix #3 — shared forwarding flag; capture thread reads it, UI writes it
    let forwarding = Arc::new(AtomicBool::new(false)); // paused by default

    // Fix #1 — capture always starts; ScrollLock toggles forwarding
    let capture_handle = start_capture(input_tx.clone(), forwarding.clone())
        .map_err(|e| format!("Capture error: {}", e))?;

    locked.server = Some(ServerState {
        handle,
        capture_handle,
        forwarding,
        client_count,
        session_code: session_code.clone(),
        local_ip: local_ip.clone(),
        input_tx,
        last_error: None,
    });

    log::info!("Server started — code: {}  ip: {}", session_code, local_ip);
    Ok(StartServerResult { session_code, local_ip })
}

/// Stop the server and release all resources.
#[tauri::command]
pub async fn cmd_stop_server(state: State<'_, SharedState>) -> Result<(), String> {
    let mut locked = state.lock().await;

    if let Some(srv) = locked.server.take() {
        // Drop capture handle first — signals the capture thread to stop forwarding
        drop(srv.capture_handle);
        srv.handle.shutdown();
        log::info!("Server stopped");
        Ok(())
    } else {
        Err("No server running".into())
    }
}

/// Connect this machine as a client to a remote server.
#[tauri::command]
pub async fn cmd_connect(
    state: State<'_, SharedState>,
    server_ip: String,
    session_code: String,
) -> Result<(), String> {
    let mut locked = state.lock().await;

    if locked.client.is_some() {
        return Err("Already connected to a server".into());
    }
    if locked.server.is_some() {
        return Err("Cannot connect as client while running as server".into());
    }

    let (status_tx, _status_rx) = mpsc::unbounded_channel::<String>();

    let handle = connect_to_server(&server_ip, &session_code, status_tx)
        .await
        .map_err(|e| e.to_string())?;

    locked.client = Some(ClientState {
        handle,
        server_addr: format!("{}:24800", server_ip),
        latency_ms: None,
        last_error: None,
    });

    log::info!("Connected to {}", server_ip);
    Ok(())
}

/// Disconnect from the server.
#[tauri::command]
pub async fn cmd_disconnect(state: State<'_, SharedState>) -> Result<(), String> {
    let mut locked = state.lock().await;

    if let Some(cli) = locked.client.take() {
        cli.handle.disconnect();
        log::info!("Disconnected");
        Ok(())
    } else {
        Err("Not connected".into())
    }
}

/// Toggle input forwarding on/off (Fix #3 — UI counterpart to ScrollLock).
/// Returns the new capturing state.
#[tauri::command]
pub async fn cmd_toggle_capture(state: State<'_, SharedState>) -> Result<bool, String> {
    let locked = state.lock().await;

    if let Some(srv) = &locked.server {
        let was = srv.forwarding.fetch_xor(true, Ordering::Relaxed);
        let now = !was;
        log::info!("Capture toggled via UI → {}", now);
        Ok(now)
    } else {
        Err("Server not running".into())
    }
}

/// Get the current application status (polled by the frontend every 2s).
#[tauri::command]
pub async fn cmd_get_status(state: State<'_, SharedState>) -> Result<AppStatus, String> {
    Ok(state.lock().await.status())
}
