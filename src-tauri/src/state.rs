use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
    pub error: Option<String>,
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
            error: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EdgeTriggers {
    pub top: bool,
    pub bottom: bool,
    pub left: bool,
    pub right: bool,
    /// Pixels from edge that activate crossing
    pub trigger_px: u32,
}

impl Default for EdgeTriggers {
    fn default() -> Self {
        Self {
            top: false,
            bottom: false,
            left: false,
            right: true,
            trigger_px: 2,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadCorners {
    pub top_left: bool,
    pub top_right: bool,
    pub bottom_left: bool,
    pub bottom_right: bool,
    /// Corner dead zone square size in pixels
    pub size_px: u32,
}

impl Default for DeadCorners {
    fn default() -> Self {
        Self {
            top_left: false,
            top_right: false,
            bottom_left: false,
            bottom_right: false,
            size_px: 50,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DeadZone {
    /// Normalized 0.0–1.0
    pub x_frac: f32,
    pub y_frac: f32,
    pub w_frac: f32,
    pub h_frac: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerConfig {
    pub control_port: u16,
    pub udp_port: u16,
    pub ssl_enabled: bool,
    pub edge_triggers: EdgeTriggers,
    pub dead_corners: DeadCorners,
    pub dead_zones: Vec<DeadZone>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            control_port: 24800,
            udp_port: 24801,
            ssl_enabled: false,
            edge_triggers: EdgeTriggers::default(),
            dead_corners: DeadCorners::default(),
            dead_zones: Vec::new(),
        }
    }
}

pub struct ServerState {
    pub handle: ServerHandle,
    pub capture_handle: CaptureHandle,
    pub forwarding: Arc<AtomicBool>,
    pub client_count: Arc<AtomicUsize>,
    pub session_code: String,
    pub local_ip: String,
    pub input_tx: mpsc::Sender<InputPacket>,
    pub last_error: Option<String>,
}

pub struct ClientState {
    pub handle: ClientHandle,
    pub server_addr: String,
    pub latency_ms: Option<f64>,
    pub last_error: Option<String>,
}

pub struct AppState {
    pub server: Option<ServerState>,
    pub client: Option<ClientState>,
    pub config: ServerConfig,
}

impl AppState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            server: None,
            client: None,
            config,
        }
    }

    pub fn status(&self) -> AppStatus {
        if let Some(srv) = &self.server {
            AppStatus {
                role: Role::Server,
                session_code: Some(srv.session_code.clone()),
                local_ip: Some(format!("{}:{}", srv.local_ip, self.config.control_port)),
                server_addr: None,
                client_count: srv.client_count.load(Ordering::Relaxed),
                capturing: srv.forwarding.load(Ordering::Relaxed),
                latency_ms: None,
                error: srv.last_error.clone(),
            }
        } else if let Some(cli) = &self.client {
            AppStatus {
                role: Role::Client,
                session_code: None,
                local_ip: None,
                server_addr: Some(cli.server_addr.clone()),
                client_count: 0,
                capturing: false,
                latency_ms: cli.latency_ms,
                error: cli.last_error.clone(),
            }
        } else {
            AppStatus::default()
        }
    }
}

pub type SharedState = Arc<Mutex<AppState>>;

pub fn new_shared_state(config: ServerConfig) -> SharedState {
    Arc::new(Mutex::new(AppState::new(config)))
}
