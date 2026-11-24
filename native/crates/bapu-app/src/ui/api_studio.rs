use crate::app::BapuApp;
use bapu_core::types::HttpMethod;

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("method")
            .selected_text(app.active_request.method.as_str())
            .show_ui(ui, |ui| {
                for m in [HttpMethod::GET, HttpMethod::POST, HttpMethod::PUT, HttpMethod::PATCH, HttpMethod::DELETE] {
                    ui.selectable_value(&mut app.active_request.method, m, m.as_str());
                }
            });
        ui.add(
            egui::TextEdit::singleline(&mut app.active_request.url)
                .hint_text("https://api.example.com/v1/resource")
                .desired_width(ui.available_width() - 80.0),
        );
        let send_enabled = !app.api_busy && !app.active_request.url.trim().is_empty();
        if ui.add_enabled(send_enabled, egui::Button::new("Send")).clicked() {
            app.send_active_request();
        }
    });

    ui.separator();
    ui.collapsing("Headers", |ui| {
        let mut remove_idx = None;
        for (i, h) in app.active_request.headers.iter_mut().enumerate() {
            ui.horizontal(|ui| {
                ui.checkbox(&mut h.enabled, "");
                ui.add(egui::TextEdit::singleline(&mut h.key).hint_text("Key").desired_width(160.0));
                ui.add(egui::TextEdit::singleline(&mut h.value).hint_text("Value").desired_width(240.0));
                if ui.small_button("✕").clicked() {
                    remove_idx = Some(i);
                }
            });
        }
        if let Some(i) = remove_idx {
            app.active_request.headers.remove(i);
        }
        if ui.button("+ Add header").clicked() {
            app.active_request.headers.push(bapu_core::types::KeyValuePair {
                id: uuid::Uuid::new_v4().to_string(),
                key: String::new(),
                value: String::new(),
                enabled: true,
                is_secret: None,
            });
        }
    });

    ui.collapsing("Body", |ui| {
        ui.add(
            egui::TextEdit::multiline(&mut app.active_request.body_content)
                .desired_rows(6)
                .code_editor(),
        );
    });

    ui.separator();
    ui.heading("Response");
    match &app.last_response {
        None => {
            ui.label(egui::RichText::new("No response yet — send a request.").weak());
        }
        Some(resp) => {
            ui.horizontal(|ui| {
                let color = if resp.status < 300 {
                    egui::Color32::from_rgb(34, 197, 94)
                } else if resp.status < 500 {
                    egui::Color32::from_rgb(234, 179, 8)
                } else {
                    egui::Color32::from_rgb(239, 68, 68)
                };
                ui.colored_label(color, format!("{} {}", resp.status, resp.status_text));
                ui.label(format!("{} ms", resp.time_ms));
                ui.label(format!("{} bytes", resp.size_bytes));
            });
            egui::ScrollArea::vertical().max_height(360.0).show(ui, |ui| {
                let mut pretty = serde_json::to_string_pretty(&resp.data).unwrap_or_default();
                ui.add(
                    egui::TextEdit::multiline(&mut pretty)
                        .code_editor()
                        .interactive(false) // read-only, but still selectable/copyable
                        .desired_width(ui.available_width()),
                );
            });
        }
    }
}
