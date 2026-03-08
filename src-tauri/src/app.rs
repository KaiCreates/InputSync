use std::path::PathBuf;
use std::sync::Arc;

use eframe::egui;
use tokio::sync::mpsc;
use tokio::sync::Mutex;

use crate::state::{AppStatus, Role, ServerConfig, SharedState};
use crate::ui::UiState;
use crate::{NetEvent, UiCommand};

pub struct InputSyncApp {
    pub cmd_tx: mpsc::UnboundedSender<UiCommand>,
    pub net_rx: mpsc::UnboundedReceiver<NetEvent>,
    pub shared: SharedState,

    /// Latest polled status
    pub status: AppStatus,

    /// Mutable UI state (form fields, selected tab, etc.)
    pub ui: UiState,

    pub data_dir: PathBuf,
}

impl InputSyncApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        cmd_tx: mpsc::UnboundedSender<UiCommand>,
        net_rx: mpsc::UnboundedReceiver<NetEvent>,
        shared: SharedState,
        config: ServerConfig,
        data_dir: PathBuf,
    ) -> Self {
        // Set dark theme once at startup instead of every frame.
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self {
            cmd_tx,
            net_rx,
            shared,
            status: AppStatus::default(),
            ui: UiState::new(config),
            data_dir,
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.net_rx.try_recv() {
            match ev {
                NetEvent::StatusUpdate(s) => {
                    // Server finished stopping — clear the busy flag
                    self.ui.server_stopping = false;
                    self.status = s;
                }
                NetEvent::ServerStopping => {
                    self.ui.server_stopping = true;
                }
                NetEvent::Error(e) => {
                    self.ui.server_stopping = false;
                    self.ui.last_error = Some(e);
                }
                NetEvent::Connected => {
                    self.ui.last_error = None;
                }
                NetEvent::Disconnected => {}
                NetEvent::LatencyUpdate(ms) => {
                    self.status.latency_ms = Some(ms);
                }
            }
        }
    }
}

impl eframe::App for InputSyncApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Poll async events
        self.drain_events();

        // Faster repaint while stopping so the spinner animates smoothly
        let repaint_delay = if self.ui.server_stopping {
            std::time::Duration::from_millis(50)
        } else {
            std::time::Duration::from_millis(500)
        };
        ctx.request_repaint_after(repaint_delay);

        egui::CentralPanel::default().show(ctx, |ui| {
            crate::ui::show(&mut self.ui, ui, &self.status, &self.cmd_tx, &self.data_dir);
        });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Cleanup: stop server if running
        let _ = self.cmd_tx.send(UiCommand::StopServer);
    }
}
