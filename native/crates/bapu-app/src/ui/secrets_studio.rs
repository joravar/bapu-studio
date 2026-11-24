use crate::app::BapuApp;
use bapu_core::types::KeyValuePair;

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    ui.heading("Environment & Secrets");
    ui.label(
        egui::RichText::new(
            "Values marked \"secret\" are stored in your OS keychain, never in a plaintext file.",
        )
        .weak(),
    );
    ui.separator();

    let Some(env) = app.environments.get_mut(app.active_env_index) else {
        ui.label("No environment selected.");
        return;
    };

    let mut remove_idx = None;
    for (i, v) in env.variables.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut v.enabled, "");
            ui.add(egui::TextEdit::singleline(&mut v.key).hint_text("KEY").desired_width(180.0));
            let is_secret = v.is_secret.unwrap_or(false);
            if is_secret {
                ui.add(egui::TextEdit::singleline(&mut v.value).password(true).desired_width(220.0));
            } else {
                ui.add(egui::TextEdit::singleline(&mut v.value).desired_width(220.0));
            }
            let mut secret_flag = is_secret;
            if ui.checkbox(&mut secret_flag, "secret").changed() {
                v.is_secret = Some(secret_flag);
            }
            if ui.small_button("✕").clicked() {
                remove_idx = Some(i);
            }
        });
    }
    if let Some(i) = remove_idx {
        env.variables.remove(i);
    }
    if ui.button("+ Add variable").clicked() {
        env.variables.push(KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: "NEW_VARIABLE".into(),
            value: String::new(),
            enabled: true,
            is_secret: Some(false),
        });
    }
}
