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

    /// True while the server is in the process of stopping (port not yet released).
    /// Buttons are disabled during this window.
    pub server_stopping: bool,

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
            server_stopping: false,
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
    // Header
    ui.add_space(8.0);
    ui.vertical_centered(|ui| {
        ui.heading(
            egui::RichText::new("InputSync")
                .size(28.0)
                .strong()
                .color(egui::Color32::from_rgb(110, 100, 255)),
        );
        ui.label(
            egui::RichText::new("Software KVM Switch")
                .small()
                .color(egui::Color32::from_rgb(120, 130, 160)),
        );
    });
    ui.add_space(12.0);

    // Tab bar
    ui.horizontal(|ui| {
        ui.add_space(12.0);
        let tab_style = |ui: &mut egui::Ui, selected: bool, label: &str| {
            let color = if selected {
                egui::Color32::from_rgb(110, 100, 255)
            } else {
                egui::Color32::from_rgb(120, 130, 160)
            };
            let text = egui::RichText::new(label).size(15.0).strong();
            let resp = ui.selectable_label(selected, text);
            if selected {
                // Draw a small indicator underline
                let rect = resp.rect;
                let line_y = rect.bottom() + 2.0;
                ui.painter().line_segment(
                    [egui::pos2(rect.left(), line_y), egui::pos2(rect.right(), line_y)],
                    egui::Stroke::new(2.0, color),
                );
            }
            resp
        };

        if tab_style(ui, ui_state.active_tab == Tab::Main, "Main").clicked() {
            ui_state.active_tab = Tab::Main;
        }
        ui.add_space(8.0);
        if tab_style(ui, ui_state.active_tab == Tab::Settings, "Settings").clicked() {
            ui_state.active_tab = Tab::Settings;
        }
        ui.add_space(8.0);
        if tab_style(ui, ui_state.active_tab == Tab::Logs, "Logs").clicked() {
            ui_state.active_tab = Tab::Logs;
        }
    });

    ui.add_space(4.0);
    ui.separator();
    ui.add_space(8.0);

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
