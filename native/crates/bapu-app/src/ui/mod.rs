pub mod api_studio;
pub mod database_studio;
pub mod history_studio;
pub mod secrets_studio;

use crate::app::{BapuApp, Tab};

pub fn top_bar(app: &mut BapuApp, ctx: &egui::Context) {
    egui::TopBottomPanel::top("top_bar").show(ctx, |ui| {
        ui.horizontal(|ui| {
            ui.heading("⚡ Bapu Studio");
            ui.label(egui::RichText::new("developer cockpit").weak());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if app.api_busy || app.db_busy {
                    ui.spinner();
                }
            });
        });
    });
}

pub fn side_bar(app: &mut BapuApp, ctx: &egui::Context) {
    egui::SidePanel::left("side_bar").resizable(false).show(ctx, |ui| {
        ui.add_space(8.0);
        tab_button(ui, app, Tab::Api, "🚀 API Studio");
        tab_button(ui, app, Tab::Database, "🗄 Database Studio");
        tab_button(ui, app, Tab::Secrets, "🔐 Secrets");
        tab_button(ui, app, Tab::History, "🕘 History");
    });
}

fn tab_button(ui: &mut egui::Ui, app: &mut BapuApp, tab: Tab, label: &str) {
    let selected = app.tab == tab;
    if ui.selectable_label(selected, label).clicked() {
        app.tab = tab;
    }
}
