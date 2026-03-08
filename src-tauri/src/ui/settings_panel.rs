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
        ui.add_space(8.0);

        // ── NETWORK ──────────────────────────────────────────────────────
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("NETWORK SETTINGS").strong().color(egui::Color32::WHITE));
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

            ui.checkbox(&mut cfg.ssl_enabled, "Enable SSL/TLS Encryption");
            if cfg.ssl_enabled {
                ui.label(egui::RichText::new("Self-signed certificate will be generated automatically.").small().italics());
            }
        });

        ui.add_space(12.0);

        // ── EDGE TRIGGERS ────────────────────────────────────────────────
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("EDGE TRIGGERS").strong().color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("Click screen edges to enable cursor handoff:").small());
            ui.add_space(8.0);

            ui.centered_and_justified(|ui| {
                let map_size = egui::Vec2::new(240.0, 160.0);
                let (_resp, map_result) = crate::ui::screen_map::show(
                    ui,
                    &cfg.edge_triggers,
                    &cfg.dead_corners,
                    &cfg.dead_zones,
                    map_size,
                );
                cfg.edge_triggers = map_result.edge_triggers;
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Trigger Sensitivity:");
                let mut px = cfg.edge_triggers.trigger_px as i32;
                ui.add(egui::Slider::new(&mut px, 1..=20).text("px"));
                cfg.edge_triggers.trigger_px = px.max(1) as u32;
            });
        });

        ui.add_space(12.0);

        // ── DEAD CORNERS ────────────────────────────────────────────────
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("DEAD CORNERS").strong().color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("Blocks triggers at corners to prevent accidental activation:").small());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                ui.checkbox(&mut cfg.dead_corners.top_left, "Top-Left");
                ui.checkbox(&mut cfg.dead_corners.top_right, "Top-Right");
                ui.checkbox(&mut cfg.dead_corners.bottom_left, "Bottom-Left");
                ui.checkbox(&mut cfg.dead_corners.bottom_right, "Bottom-Right");
            });
            ui.horizontal(|ui| {
                ui.label("Corner Radius:");
                let mut sz = cfg.dead_corners.size_px as i32;
                ui.add(egui::Slider::new(&mut sz, 10..=200).text("px"));
                cfg.dead_corners.size_px = sz.max(10) as u32;
            });
        });

        ui.add_space(12.0);

        // ── DEAD ZONES ──────────────────────────────────────────────────
        ui.group(|ui| {
            ui.set_width(ui.available_width());
            ui.label(egui::RichText::new("DEAD ZONES").strong().color(egui::Color32::WHITE));
            ui.label(egui::RichText::new("Rectangular areas where edge crossing is blocked:").small());
            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button(egui::RichText::new("+ Add Zone").strong()).clicked() {
                    cfg.dead_zones.push(DeadZone {
                        x_frac: 0.1,
                        y_frac: 0.1,
                        w_frac: 0.2,
                        h_frac: 0.2,
                    });
                }
                if ui.button("Clear All").clicked() {
                    cfg.dead_zones.clear();
                }
            });

            let mut to_remove = None;
            for (i, dz) in cfg.dead_zones.iter_mut().enumerate() {
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.label(format!("{}:", i + 1));
                    ui.add(egui::DragValue::new(&mut dz.x_frac).prefix("X: ").speed(0.01).range(0.0..=0.99));
                    ui.add(egui::DragValue::new(&mut dz.y_frac).prefix("Y: ").speed(0.01).range(0.0..=0.99));
                    ui.add(egui::DragValue::new(&mut dz.w_frac).prefix("W: ").speed(0.01).range(0.01..=1.0));
                    ui.add(egui::DragValue::new(&mut dz.h_frac).prefix("H: ").speed(0.01).range(0.01..=1.0));
                    
                    if ui.button("✕").clicked() {
                        to_remove = Some(i);
                    }
                });
            }
            if let Some(idx) = to_remove {
                cfg.dead_zones.remove(idx);
            }
        });

        ui.add_space(20.0);

        if ui
            .add_sized([ui.available_width(), 40.0], egui::Button::new(egui::RichText::new("SAVE ALL SETTINGS").strong().size(15.0)).fill(egui::Color32::from_rgb(46, 204, 113)))
            .clicked()
        {
            let _ = cmd_tx.send(UiCommand::UpdateConfig(cfg.clone()));
        }
        
        ui.add_space(12.0);
    });
}
