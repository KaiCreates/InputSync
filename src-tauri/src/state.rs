use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use crate::core::protocol::InputPacket;
use crate::input::capture::CaptureHandle;
use crate::network::client::ClientHandle;
use crate::network::server::ServerHandle;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Idle,
    Server,
    Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppStatus {
    pub role: Role,
    pub session_code: Option<String>,
    pub local_ip: Option<String>,
    pub server_addr: Option<String>,
    pub client_count: usize,
    pub capturing: bool,
    pub latency_ms: Option<f64>,
}

impl Default for AppStatus {
    fn default() -> Self {
        Self {
            role: Role::Idle,
            session_code: None,
            local_ip: None,
            server_addr: None,
            client_count: 0,
            capturing: false,
            latency_ms: None,
        }
    }
}

pub struct ServerState {
    pub handle: ServerHandle,
    pub capture_handle: Option<CaptureHandle>,
    pub session_code: String,
    pub local_ip: String,
    /// Channel to send captured input events to the server task
    pub input_tx: mpsc::UnboundedSender<InputPacket>,
    pub client_count: usize,
}

pub struct ClientState {
    pub handle: ClientHandle,
    pub server_addr: String,
    pub status_rx: mpsc::UnboundedReceiver<String>,
}

pub struct AppState {
    pub server: Option<ServerState>,
    pub client: Option<ClientState>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            server: None,
            client: None,
        }
    }

    pub fn status(&self) -> AppStatus {
        if let Some(srv) = &self.server {
            AppStatus {
                role: Role::Server,
                session_code: Some(srv.session_code.clone()),
                local_ip: Some(format!("{}:24800", srv.local_ip)),
                server_addr: None,
                client_count: srv.client_count,
                capturing: srv.capture_handle.is_some(),
                latency_ms: None,
            }
        } else if let Some(cli) = &self.client {
            AppStatus {
                role: Role::Client,
                session_code: None,
                local_ip: None,
                server_addr: Some(cli.server_addr.clone()),
                client_count: 0,
                capturing: false,
                latency_ms: None,
            }
        } else {
            AppStatus::default()
        }
    }
}

/// Shared app state across Tauri commands
pub type SharedState = Arc<Mutex<AppState>>;

pub fn new_shared_state() -> SharedState {
    Arc::new(Mutex::new(AppState::new()))
}
