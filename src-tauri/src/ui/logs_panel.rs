use eframe::egui;

use crate::ui::UiState;

pub fn show(ui_state: &mut UiState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Logs");
        ui.checkbox(&mut ui_state.log_scroll_to_bottom, "Auto-scroll");
        // Clear/Export are not yet supported by egui_logger 0.6 — hidden to
        // avoid showing non-functional buttons.
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(ui_state.log_scroll_to_bottom)
        .show(ui, |ui| {
            egui_logger::logger_ui().show(ui);
        });
}
