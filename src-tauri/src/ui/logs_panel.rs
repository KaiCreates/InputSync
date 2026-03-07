use eframe::egui;

use crate::ui::UiState;

pub fn show(ui_state: &mut UiState, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.label("Logs");
        ui.checkbox(&mut ui_state.log_scroll_to_bottom, "Auto-scroll");
        if ui.small_button("Clear").clicked() {
            // egui_logger 0.6 does not expose a clear function
        }
        if ui.small_button("Export").clicked() {
            // Simple export: write log lines to ~/.local/share/inputsync/inputsync.log
            // egui_logger doesn't expose raw records, so we skip file export for now
        }
    });

    ui.separator();

    egui::ScrollArea::vertical()
        .auto_shrink([false; 2])
        .stick_to_bottom(ui_state.log_scroll_to_bottom)
        .show(ui, |ui| {
            egui_logger::logger_ui().show(ui);
        });
}
