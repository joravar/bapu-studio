use crate::app::BapuApp;

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    ui.heading("History");
    ui.label(egui::RichText::new("Request/query history — tracked follow-up work; not yet wired up to record entries automatically.").weak());
    let _ = app; // placeholder until history recording is implemented
}
