use std::path::PathBuf;

use eframe::egui;
use tokio::sync::mpsc;

use crate::state::DeadZone;
use crate::ui::UiState;
use crate::UiCommand;

pub fn show(
    ui_state: &mut UiState,
    ui: &mut egui::Ui,
    cmd_tx: &mpsc::UnboundedSender<UiCommand>,
    _data_dir: &PathBuf,
) {
    let cfg = &mut ui_state.config_draft;

    egui::ScrollArea::vertical().show(ui, |ui| {
        // ── NETWORK ──────────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("NETWORK").strong());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.label("Control Port:");
                let mut port_str = cfg.control_port.to_string();
                if ui
                    .add(egui::TextEdit::singleline(&mut port_str).desired_width(70.0))
                    .changed()
                {
                    if let Ok(p) = port_str.parse::<u16>() {
                        cfg.control_port = p;
                    }
                }
            });

            ui.checkbox(&mut cfg.ssl_enabled, "Enable SSL/TLS (auto self-signed cert)");
        });

        ui.add_space(8.0);

        // ── EDGE TRIGGERS ────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("EDGE TRIGGERS").strong());
            ui.small("Click screen edges below to enable cursor handoff:");
            ui.add_space(4.0);

            let map_size = egui::Vec2::new(200.0, 140.0);
            let (_resp, map_result) = crate::ui::screen_map::show(
                ui,
                &cfg.edge_triggers,
                &cfg.dead_corners,
                &cfg.dead_zones,
                map_size,
            );
            cfg.edge_triggers = map_result.edge_triggers;

            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Trigger distance:");
                let mut px = cfg.edge_triggers.trigger_px as i32;
                ui.add(egui::DragValue::new(&mut px).range(1..=20));
                cfg.edge_triggers.trigger_px = px.max(1) as u32;
                ui.label("px from edge");
            });
        });

        ui.add_space(8.0);

        // ── DEAD CORNERS ────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("DEAD CORNERS").strong());
            ui.small("Prevent accidental edge trigger at corners:");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.checkbox(&mut cfg.dead_corners.top_left, "Top-Left");
                ui.checkbox(&mut cfg.dead_corners.top_right, "Top-Right");
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut cfg.dead_corners.bottom_left, "Bottom-Left");
                ui.checkbox(&mut cfg.dead_corners.bottom_right, "Bottom-Right");
            });
            ui.horizontal(|ui| {
                ui.label("Corner size:");
                let mut sz = cfg.dead_corners.size_px as i32;
                ui.add(egui::DragValue::new(&mut sz).range(10..=200));
                cfg.dead_corners.size_px = sz.max(10) as u32;
                ui.label("px");
            });
        });

        ui.add_space(8.0);

        // ── DEAD ZONES ──────────────────────────────────────────────────
        ui.group(|ui| {
            ui.label(egui::RichText::new("DEAD ZONES").strong());
            ui.small("Rectangular regions that block edge crossing:");
            ui.add_space(4.0);

            ui.horizontal(|ui| {
            if ui.small_button("+ Add Zone").clicked() {
                cfg.dead_zones.push(DeadZone {
                    x_frac: 0.1,
                    y_frac: 0.1,
                    w_frac: 0.2,
                    h_frac: 0.2,
                });
            }
            if ui.small_button("Clear All").clicked() {
                cfg.dead_zones.clear();
            }
            }); // end horizontal

            let mut to_remove = None;
            for (i, dz) in cfg.dead_zones.iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Zone {}: ", i + 1));
                    ui.add(egui::DragValue::new(&mut dz.x_frac).prefix("x:").speed(0.01).range(0.0..=1.0));
                    ui.add(egui::DragValue::new(&mut dz.y_frac).prefix(" y:").speed(0.01).range(0.0..=1.0));
                    ui.add(egui::DragValue::new(&mut dz.w_frac).prefix(" w:").speed(0.01).range(0.01..=1.0));
                    ui.add(egui::DragValue::new(&mut dz.h_frac).prefix(" h:").speed(0.01).range(0.01..=1.0));
                    if ui.small_button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(idx) = to_remove {
                cfg.dead_zones.remove(idx);
            }
        });

        ui.add_space(12.0);

        if ui
            .add(egui::Button::new("Save Settings").fill(egui::Color32::from_rgb(30, 100, 50)))
            .clicked()
        {
            let _ = cmd_tx.send(UiCommand::UpdateConfig(cfg.clone()));
        }
    });
}
