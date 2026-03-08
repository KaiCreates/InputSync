// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod core;
mod input;
mod network;
mod state;
mod ui;

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::core::session::generate_session_code;
use crate::input::capture::start_capture;
use crate::network::client::connect_to_server;
use crate::network::server::start_server;
use crate::state::{
    AppState, ClientState, ServerConfig, ServerState, SharedState, new_shared_state,
};

const INPUT_CHANNEL_CAP: usize = 512;

/// Commands sent from the UI thread → async runtime
#[derive(Debug)]
pub enum UiCommand {
    StartServer,
    StopServer,
    Connect { ip: String, code: String },
    Disconnect,
    ToggleCapture,
    UpdateConfig(ServerConfig),
}

/// Events sent from the async runtime → UI thread
#[derive(Debug, Clone)]
pub enum NetEvent {
    StatusUpdate(crate::state::AppStatus),
    /// Server is in the process of stopping — UI should show busy state
    ServerStopping,
    Error(String),
    Connected,
    Disconnected,
    LatencyUpdate(f64),
}

/// Returns the per-user data directory for config/certs.
pub fn data_dir() -> PathBuf {
    let base = dirs_next_or_home();
    base.join("inputsync")
}

fn dirs_next_or_home() -> PathBuf {
    // Try XDG_DATA_HOME / AppData / ~/.local/share
    if let Ok(v) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(v);
    }
    #[cfg(target_os = "windows")]
    if let Ok(v) = std::env::var("APPDATA") {
        return PathBuf::from(v);
    }
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    home.join(".local").join("share")
}

fn load_config(data_dir: &PathBuf) -> ServerConfig {
    let cfg_path = data_dir.join("config.json");
    if let Ok(bytes) = std::fs::read(&cfg_path) {
        if let Ok(cfg) = serde_json::from_slice::<ServerConfig>(&bytes) {
            return cfg;
        }
    }
    ServerConfig::default()
}

pub fn save_config(data_dir: &PathBuf, cfg: &ServerConfig) {
    let _ = std::fs::create_dir_all(data_dir);
    let cfg_path = data_dir.join("config.json");
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = std::fs::write(cfg_path, json);
    }
}

/// Async runtime loop — processes UiCommands and emits NetEvents.
async fn async_main(
    shared: SharedState,
    mut cmd_rx: mpsc::UnboundedReceiver<UiCommand>,
    net_tx: mpsc::UnboundedSender<NetEvent>,
    data_dir: PathBuf,
) {
    while let Some(cmd) = cmd_rx.recv().await {
        match cmd {
            UiCommand::StartServer => {
                let mut locked = shared.lock().await;

                // Stop existing server if running, awaiting full port release
                if let Some(srv) = locked.server.take() {
                    let _ = net_tx.send(NetEvent::ServerStopping);
                    drop(srv.capture_handle);
                    srv.handle.shutdown().await; // waits until TcpListener is dropped
                }

                if locked.client.is_some() {
                    let _ = net_tx.send(NetEvent::Error(
                        "Cannot start server while connected as client".into(),
                    ));
                    continue;
                }

                let config = locked.config.clone();
                let ssl_enabled = config.ssl_enabled;
                let control_port = config.control_port;
                let udp_port = config.udp_port;

                let session_code = generate_session_code();
                let local_ip = local_ip_address::local_ip()
                    .map(|ip| ip.to_string())
                    .unwrap_or_else(|_| "127.0.0.1".to_string());

                let (input_tx, input_rx) = mpsc::channel(INPUT_CHANNEL_CAP);
                let client_count = Arc::new(AtomicUsize::new(0));

                let tls_acceptor = if ssl_enabled {
                    match crate::network::tls::make_tls_acceptor(&data_dir) {
                        Ok(a) => Some(a),
                        Err(e) => {
                            let _ = net_tx
                                .send(NetEvent::Error(format!("TLS setup failed: {}", e)));
                            continue;
                        }
                    }
                } else {
                    None
                };

                let handle = match start_server(
                    session_code.clone(),
                    input_rx,
                    client_count.clone(),
                    control_port,
                    udp_port,
                    tls_acceptor,
                )
                .await
                {
                    Ok(h) => h,
                    Err(e) => {
                        let _ = net_tx
                            .send(NetEvent::Error(format!("Start server failed: {}", e)));
                        continue;
                    }
                };

                let forwarding = Arc::new(AtomicBool::new(false));
                let capture_handle =
                    match start_capture(input_tx.clone(), forwarding.clone(), config) {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = net_tx
                                .send(NetEvent::Error(format!("Capture error: {}", e)));
                            handle.shutdown().await;
                            continue;
                        }
                    };

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

                tracing::info!("Server started — code: {}  ip: {}", session_code, local_ip);
                let _ = net_tx.send(NetEvent::StatusUpdate(locked.status()));
            }

            UiCommand::StopServer => {
                let mut locked = shared.lock().await;
                if let Some(srv) = locked.server.take() {
                    let _ = net_tx.send(NetEvent::ServerStopping);
                    drop(srv.capture_handle);
                    // Drop the lock before awaiting so the UI can still poll
                    drop(locked);
                    srv.handle.shutdown().await; // waits until TcpListener is dropped
                    tracing::info!("Server stopped");
                    let locked2 = shared.lock().await;
                    let _ = net_tx.send(NetEvent::StatusUpdate(locked2.status()));
                }
            }

            UiCommand::Connect { ip, code } => {
                let mut locked = shared.lock().await;

                if locked.client.is_some() {
                    let _ = net_tx.send(NetEvent::Error("Already connected".into()));
                    continue;
                }
                if locked.server.is_some() {
                    let _ = net_tx
                        .send(NetEvent::Error("Cannot connect as client while server is running".into()));
                    continue;
                }

                let ssl_enabled = locked.config.ssl_enabled;
                let control_port = locked.config.control_port;
                let tls_connector = if ssl_enabled {
                    Some(crate::network::tls::make_tls_connector())
                } else {
                    None
                };

                let (status_tx, mut status_rx) = mpsc::unbounded_channel::<String>();
                let handle =
                    match connect_to_server(&ip, &code, control_port, status_tx, tls_connector).await {
                        Ok(h) => h,
                        Err(e) => {
                            let _ = net_tx
                                .send(NetEvent::Error(format!("Connect failed: {}", e)));
                            continue;
                        }
                    };

                locked.client = Some(ClientState {
                    handle,
                    server_addr: format!("{}:{}", ip, control_port),
                    latency_ms: None,
                    last_error: None,
                });

                // Forward disconnect events from the client UDP task to the UI.
                // Without this the receiver is dropped and all status sends silently fail.
                let net_tx_status = net_tx.clone();
                let shared_status = shared.clone();
                tokio::spawn(async move {
                    while let Some(msg) = status_rx.recv().await {
                        if msg.starts_with("disconnected") || msg.starts_with("simulator_error") {
                            shared_status.lock().await.client = None;
                            let _ = net_tx_status.send(NetEvent::Disconnected);
                        }
                    }
                });

                tracing::info!("Connected to {}", ip);
                let _ = net_tx.send(NetEvent::Connected);
                let _ = net_tx.send(NetEvent::StatusUpdate(locked.status()));
            }

            UiCommand::Disconnect => {
                let mut locked = shared.lock().await;
                if let Some(cli) = locked.client.take() {
                    cli.handle.disconnect();
                    tracing::info!("Disconnected");
                    let _ = net_tx.send(NetEvent::Disconnected);
                    let _ = net_tx.send(NetEvent::StatusUpdate(locked.status()));
                }
            }

            UiCommand::ToggleCapture => {
                let locked = shared.lock().await;
                if let Some(srv) = &locked.server {
                    let was = srv.forwarding.fetch_xor(true, Ordering::Relaxed);
                    tracing::info!("Capture toggled → {}", !was);
                    let _ = net_tx.send(NetEvent::StatusUpdate(locked.status()));
                }
            }

            UiCommand::UpdateConfig(new_cfg) => {
                let mut locked = shared.lock().await;
                locked.config = new_cfg.clone();
                save_config(&data_dir, &new_cfg);
                tracing::info!("Config saved");
            }
        }
    }
}

fn main() {
    // egui_logger is the sole global logger — it captures all log/tracing records
    // into a ring buffer displayed in the Logs tab.
    // tracing macros (info!, warn!, etc.) route through the log crate bridge
    // enabled by the "log" feature on the tracing crate — no separate subscriber needed.
    egui_logger::builder().init().ok();

    let data_dir = data_dir();
    std::fs::create_dir_all(&data_dir).ok();

    let config = load_config(&data_dir);

    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<UiCommand>();
    let (net_tx, net_rx) = mpsc::unbounded_channel::<NetEvent>();

    let shared = new_shared_state(config.clone());
    let shared_async = shared.clone();
    let data_dir_async = data_dir.clone();
    let net_tx_async = net_tx.clone();

    // Spawn OS thread that owns the tokio runtime
    std::thread::Builder::new()
        .name("inputsync-async".into())
        .spawn(move || {
            tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("Failed to build tokio runtime")
                .block_on(async_main(shared_async, cmd_rx, net_tx_async, data_dir_async));
        })
        .expect("Failed to spawn async thread");

    // Launch eframe on main thread
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("InputSync")
            .with_inner_size([480.0, 580.0])
            .with_resizable(false)
            .with_icon(load_app_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "InputSync",
        native_options,
        Box::new(move |cc| {
            egui_extras::install_image_loaders(&cc.egui_ctx);
            Ok(Box::new(app::InputSyncApp::new(
                cc,
                cmd_tx,
                net_rx,
                shared,
                config,
                data_dir,
            )))
        }),
    )
    .expect("eframe run_native failed");
}

fn load_app_icon() -> egui::IconData {
    // Embed 32x32 icon if available, otherwise return empty
    egui::IconData::default()
}
