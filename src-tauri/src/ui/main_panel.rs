use eframe::egui;
use tokio::sync::mpsc;

use crate::state::{AppStatus, Role};
use crate::ui::UiState;
use crate::UiCommand;

pub fn show(
    ui_state: &mut UiState,
    ui: &mut egui::Ui,
    status: &AppStatus,
    cmd_tx: &mpsc::UnboundedSender<UiCommand>,
) {
    // ── SERVER section ────────────────────────────────────────────────────
    ui.group(|ui| {
        ui.heading("SERVER");
        ui.add_space(4.0);

        match &status.role {
            Role::Server => {
                if let Some(code) = &status.session_code {
                    ui.horizontal(|ui| {
                        ui.label("Session:");
                        ui.strong(code);
                        if ui.small_button("Copy").clicked() {
                            ui.output_mut(|o| o.copied_text = code.clone());
                        }
                    });
                }
                if let Some(addr) = &status.local_ip {
                    ui.horizontal(|ui| {
                        ui.label("Address:");
                        ui.monospace(addr);
                    });
                }
                ui.horizontal(|ui| {
                    ui.label("Clients:");
                    ui.label(format!("{} connected", status.client_count));
                });
                ui.horizontal(|ui| {
                    let fwd_label = if status.capturing { "Forwarding: ON" } else { "Forwarding: OFF" };
                    if ui.button(fwd_label).clicked() {
                        let _ = cmd_tx.send(UiCommand::ToggleCapture);
                    }
                });
                ui.add_space(4.0);
                if ui
                    .add(egui::Button::new("Stop Server").fill(egui::Color32::from_rgb(160, 40, 40)))
                    .clicked()
                {
                    let _ = cmd_tx.send(UiCommand::StopServer);
                }
            }
            _ => {
                // Idle or Client mode — show Start Server button (disabled when client)
                let disabled = matches!(status.role, Role::Client);
                ui.add_enabled_ui(!disabled, |ui| {
                    if ui
                        .add(egui::Button::new("Start Server").fill(egui::Color32::from_rgb(30, 100, 50)))
                        .clicked()
                    {
                        let _ = cmd_tx.send(UiCommand::StartServer);
                    }
                });
                if disabled {
                    ui.small("(disconnect client first)");
                }
            }
        }
    });

    ui.add_space(6.0);

    // ── CLIENT section ────────────────────────────────────────────────────
    ui.group(|ui| {
        ui.heading("CLIENT");
        ui.add_space(4.0);

        match &status.role {
            Role::Client => {
                if let Some(addr) = &status.server_addr {
                    ui.horizontal(|ui| {
                        ui.label("Connected to");
                        ui.monospace(addr);
                    });
                }
                if let Some(lat) = status.latency_ms {
                    ui.label(format!("Latency: {:.1}ms", lat));
                }
                ui.add_space(4.0);
                if ui
                    .add(egui::Button::new("Disconnect").fill(egui::Color32::from_rgb(160, 40, 40)))
                    .clicked()
                {
                    let _ = cmd_tx.send(UiCommand::Disconnect);
                }
            }
            _ => {
                let disabled = matches!(status.role, Role::Server);
                ui.add_enabled_ui(!disabled, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Code:");
                        ui.add(
                            egui::TextEdit::singleline(&mut ui_state.code_input)
                                .hint_text("ABC123")
                                .desired_width(70.0),
                        );
                        ui.label("IP:");
                        ui.add(
                            egui::TextEdit::singleline(&mut ui_state.ip_input)
                                .hint_text("192.168.1.x")
                                .desired_width(130.0),
                        );
                    });
                    ui.add_space(4.0);
                    if ui
                        .add(egui::Button::new("Connect").fill(egui::Color32::from_rgb(30, 80, 150)))
                        .clicked()
                    {
                        let code = ui_state.code_input.trim().to_uppercase();
                        let ip = ui_state.ip_input.trim().to_string();
                        if !code.is_empty() && !ip.is_empty() {
                            let _ = cmd_tx.send(UiCommand::Connect { ip, code });
                        }
                    }
                });
                if disabled {
                    ui.small("(stop server first)");
                }
            }
        }
    });

    ui.add_space(6.0);

    // ── Status bar ────────────────────────────────────────────────────────
    ui.separator();
    ui.horizontal(|ui| {
        let (dot_color, status_text) = match &status.role {
            Role::Idle => (egui::Color32::GRAY, "Idle".to_string()),
            Role::Server => {
                let fwd = if status.capturing { "forwarding" } else { "paused" };
                (
                    egui::Color32::from_rgb(50, 200, 80),
                    format!("Server active · {} clients · {}", status.client_count, fwd),
                )
            }
            Role::Client => {
                let lat = status
                    .latency_ms
                    .map(|l| format!(" · {:.1}ms", l))
                    .unwrap_or_default();
                (egui::Color32::from_rgb(80, 150, 255), format!("Client connected{}", lat))
            }
        };

        ui.colored_label(dot_color, "●");
        ui.label(status_text);

        if let Some(err) = &ui_state.last_error {
            ui.colored_label(egui::Color32::from_rgb(255, 80, 80), format!(" ⚠ {}", err));
        }
    });
}
