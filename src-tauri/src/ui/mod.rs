pub mod logs_panel;
pub mod main_panel;
pub mod screen_map;
pub mod settings_panel;

use std::path::PathBuf;

use eframe::egui;
use tokio::sync::mpsc;

use crate::state::{AppStatus, ServerConfig};
use crate::UiCommand;

#[derive(PartialEq, Clone, Copy)]
pub enum Tab {
    Main,
    Settings,
    Logs,
}

/// All mutable UI-side state (not business logic)
pub struct UiState {
    pub active_tab: Tab,

    // Main panel fields
    pub code_input: String,
    pub ip_input: String,
    pub last_error: Option<String>,

    // Settings
    pub config_draft: ServerConfig,

    // Logs
    pub log_scroll_to_bottom: bool,
}

impl UiState {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            active_tab: Tab::Main,
            code_input: String::new(),
            ip_input: String::new(),
            last_error: None,
            config_draft: config,
            log_scroll_to_bottom: true,
        }
    }
}

/// Top-level render function called from `App::update`.
pub fn show(
    ui_state: &mut UiState,
    ui: &mut egui::Ui,
    status: &AppStatus,
    cmd_tx: &mpsc::UnboundedSender<UiCommand>,
    data_dir: &PathBuf,
) {
    // Tab bar
    ui.horizontal(|ui| {
        ui.selectable_value(&mut ui_state.active_tab, Tab::Main, "Main");
        ui.selectable_value(&mut ui_state.active_tab, Tab::Settings, "Settings");
        ui.selectable_value(&mut ui_state.active_tab, Tab::Logs, "Logs");
    });

    ui.separator();

    match ui_state.active_tab {
        Tab::Main => {
            main_panel::show(ui_state, ui, status, cmd_tx);
        }
        Tab::Settings => {
            settings_panel::show(ui_state, ui, cmd_tx, data_dir);
        }
        Tab::Logs => {
            logs_panel::show(ui_state, ui);
        }
    }
}
