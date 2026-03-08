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
    // Disable the entire panel while the server is stopping
    let busy = ui_state.server_stopping;

    if busy {
        ui.centered_and_justified(|ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new("Cleaning up server resources...").italics().color(
                    egui::Color32::from_rgb(200, 180, 80),
                ));
            });
        });
        ui.add_space(8.0);
    }

    ui.add_enabled_ui(!busy, |ui| {

    // ── SERVER section ────────────────────────────────────────────────────
    let server_active = matches!(status.role, Role::Server);
    let server_color = if server_active { egui::Color32::from_rgb(46, 204, 113) } else { egui::Color32::from_rgb(44, 62, 80) };
    
    egui::Frame::default()
        .fill(egui::Color32::from_rgb(32, 36, 48))
        .corner_radius(8.0)
        .stroke(egui::Stroke::new(1.0, server_color.gamma_multiply(0.3)))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("HOST / SERVER").strong().size(16.0).color(egui::Color32::WHITE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if server_active {
                        ui.label(egui::RichText::new(" ACTIVE ").strong().background_color(egui::Color32::from_rgb(30, 80, 50)).color(egui::Color32::WHITE));
                    }
                });
            });
            ui.add_space(8.0);

            match &status.role {
                Role::Server => {
                    ui.horizontal(|ui| {
                        ui.label("Session Code:");
                        if let Some(code) = &status.session_code {
                            ui.strong(code);
                            if ui.button("📋").clicked() {
                                ui.ctx().copy_text(code.clone());
                            }
                        }
                    });
                    if let Some(addr) = &status.local_ip {
                        ui.horizontal(|ui| {
                            ui.label("Network Address:");
                            ui.monospace(addr);
                        });
                    }
                    ui.horizontal(|ui| {
                        ui.label("Connected Clients:");
                        ui.label(status.client_count.to_string());
                    });
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let fwd_label = if status.capturing { "Pause Forwarding" } else { "Resume Forwarding" };
                        let fwd_color = if status.capturing { egui::Color32::from_rgb(230, 126, 34) } else { egui::Color32::from_rgb(52, 152, 219) };
                        if ui.add(egui::Button::new(egui::RichText::new(fwd_label).strong()).fill(fwd_color)).clicked() {
                            let _ = cmd_tx.send(UiCommand::ToggleCapture);
                        }
                        
                        if ui.add(egui::Button::new(egui::RichText::new("Stop Server").strong()).fill(egui::Color32::from_rgb(192, 57, 43))).clicked() {
                            let _ = cmd_tx.send(UiCommand::StopServer);
                        }
                    });
                }
                _ => {
                    let disabled = matches!(status.role, Role::Client);
                    ui.add_enabled_ui(!disabled, |ui| {
                        if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(egui::RichText::new("Start Server").strong().size(14.0)).fill(egui::Color32::from_rgb(46, 204, 113))).clicked() {
                            let _ = cmd_tx.send(UiCommand::StartServer);
                        }
                    });
                    if disabled {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Disconnect from server to start hosting").small().italics());
                        });
                    }
                }
            }
        });

    ui.add_space(16.0);

    // ── CLIENT section ────────────────────────────────────────────────────
    let client_active = matches!(status.role, Role::Client);
    let client_color = if client_active { egui::Color32::from_rgb(52, 152, 219) } else { egui::Color32::from_rgb(44, 62, 80) };

    egui::Frame::default()
        .fill(egui::Color32::from_rgb(32, 36, 48))
        .corner_radius(8.0)
        .stroke(egui::Stroke::new(1.0, client_color.gamma_multiply(0.3)))
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("CONNECT TO SERVER").strong().size(16.0).color(egui::Color32::WHITE));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if client_active {
                        ui.label(egui::RichText::new(" CONNECTED ").strong().background_color(egui::Color32::from_rgb(25, 60, 100)).color(egui::Color32::WHITE));
                    }
                });
            });
            ui.add_space(8.0);

            match &status.role {
                Role::Client => {
                    ui.horizontal(|ui| {
                        ui.label("Server:");
                        if let Some(addr) = &status.server_addr {
                            ui.monospace(addr);
                        }
                    });
                    if let Some(lat) = status.latency_ms {
                        ui.horizontal(|ui| {
                            ui.label("Latency:");
                            ui.strong(format!("{:.1}ms", lat));
                        });
                    }
                    ui.add_space(8.0);
                    if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(egui::RichText::new("Disconnect").strong()).fill(egui::Color32::from_rgb(192, 57, 43))).clicked() {
                        let _ = cmd_tx.send(UiCommand::Disconnect);
                    }
                }
                _ => {
                    let disabled = matches!(status.role, Role::Server);
                    ui.add_enabled_ui(!disabled, |ui| {
                        egui::Grid::new("client_form")
                            .num_columns(2)
                            .spacing([8.0, 8.0])
                            .show(ui, |ui| {
                                ui.label("Session Code:");
                                ui.add(egui::TextEdit::singleline(&mut ui_state.code_input).hint_text("ABC123").desired_width(ui.available_width()));
                                ui.end_row();

                                ui.label("Server IP:");
                                ui.add(egui::TextEdit::singleline(&mut ui_state.ip_input).hint_text("192.168.x.x").desired_width(ui.available_width()));
                                ui.end_row();
                            });
                        ui.add_space(8.0);
                        if ui.add_sized([ui.available_width(), 32.0], egui::Button::new(egui::RichText::new("Connect").strong().size(14.0)).fill(egui::Color32::from_rgb(52, 152, 219))).clicked() {
                            let code = ui_state.code_input.trim().to_uppercase();
                            let ip = ui_state.ip_input.trim().to_string();
                            if !code.is_empty() && !ip.is_empty() {
                                let _ = cmd_tx.send(UiCommand::Connect { ip, code });
                            }
                        }
                    });
                    if disabled {
                        ui.vertical_centered(|ui| {
                            ui.label(egui::RichText::new("Stop server to connect as client").small().italics());
                        });
                    }
                }
            }
        });

    ui.add_space(16.0);

    }); // end add_enabled_ui(!busy)

    // ── Status bar ────────────────────────────────────────────────────────
    ui.with_layout(egui::Layout::bottom_up(egui::Align::Min), |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            let (dot_color, status_text) = match &status.role {
                Role::Idle => (egui::Color32::GRAY, "System Idle".to_string()),
                Role::Server => {
                    let fwd = if status.capturing { "Forwarding active" } else { "Capture paused" };
                    (
                        egui::Color32::from_rgb(46, 204, 113),
                        format!("Host Active · {} · {} clients", fwd, status.client_count),
                    )
                }
                Role::Client => {
                    let lat = status
                        .latency_ms
                        .map(|l| format!(" · {:.1}ms", l))
                        .unwrap_or_default();
                    (egui::Color32::from_rgb(52, 152, 219), format!("Client Connected{}", lat))
                }
            };

            ui.colored_label(dot_color, "●");
            ui.label(egui::RichText::new(status_text).small());

            if let Some(err) = &ui_state.last_error {
                let err_msg = if err.contains("113") || err.contains("timed out") {
                    "Server unreachable. Verify IP and network."
                } else {
                    err
                };
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.colored_label(egui::Color32::from_rgb(231, 76, 60), format!("⚠ {}", err_msg));
                });
            }
        });
        ui.separator();
    });
}
