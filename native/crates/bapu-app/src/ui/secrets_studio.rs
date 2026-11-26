use crate::app::BapuApp;
use crate::ui::widgets::{
    badge_pill, card_frame, compact_card_frame, paint_icon, primary_button, VectorIcon,
};
use bapu_core::types::KeyValuePair;
use egui::{vec2, RichText};

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();

    let Some(env_name) = app
        .environments
        .get(app.active_env_index)
        .map(|e| e.name.clone())
    else {
        ui.label("No environment selected.");
        return;
    };

    compact_card_frame(&pal).show(ui, |ui| {
        ui.horizontal(|ui| {
            paint_icon(ui, VectorIcon::ShieldKey, 22.0, pal.accent_primary);
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("Environment, Global & Dynamic Secrets Matrix")
                            .size(16.0)
                            .strong()
                            .color(pal.text_main),
                    );
                    badge_pill(ui, &env_name, pal.accent_cyan);
                    badge_pill(ui, "OS KEYCHAIN BACKED", pal.accent_emerald);
                });
                ui.label(
                    RichText::new(
                        "Precedence: Iteration Data > Active Environment > Collection > Globals > Dynamic ({{$guid}}, {{$randomEmail}}, {{$timestamp}}).",
                    )
                    .size(11.8)
                    .color(pal.text_muted),
                );
            });

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if primary_button(ui, "+ Add Env Var", true).clicked() {
                    if let Some(env) = app.environments.get_mut(app.active_env_index) {
                        env.variables.push(KeyValuePair {
                            id: uuid::Uuid::new_v4().to_string(),
                            key: "NEW_SECRET_KEY".into(),
                            value: String::new(),
                            enabled: true,
                            is_secret: Some(true),
                        });
                        app.persist();
                    }
                }
                if ui.button("+ Add Global Var").clicked() {
                    app.globals.push(KeyValuePair {
                        id: uuid::Uuid::new_v4().to_string(),
                        key: "GLOBAL_VAR".into(),
                        value: String::new(),
                        enabled: true,
                        is_secret: Some(false),
                    });
                    app.persist();
                }
                if ui.button("Copy .env").clicked() {
                    if let Some(env) = app.environments.get(app.active_env_index) {
                        let dotenv = env
                            .variables
                            .iter()
                            .filter(|v| v.enabled)
                            .map(|v| format!("{}={}", v.key, v.value))
                            .collect::<Vec<_>>()
                            .join("\n");
                        ui.output_mut(|o| o.copied_text = dotenv);
                    }
                }
            });
        });
    });

    ui.add_space(10.0);

    let mut changed = false;
    let mut toggle_reveal: Option<String> = None;
    let mut deleted_env_secrets: Vec<(String, String)> = Vec::new();
    let mut deleted_global_secrets: Vec<String> = Vec::new();

    egui::ScrollArea::vertical().show(ui, |ui| {
        card_frame(&pal).show(ui, |ui| {
            ui.label(
                RichText::new(format!("ACTIVE ENVIRONMENT VARIABLES ({env_name})"))
                    .size(11.5)
                    .strong()
                    .color(pal.accent_cyan),
            );
            ui.separator();
            if let Some(env) = app.environments.get_mut(app.active_env_index) {
                let env_id = env.id.clone();
                let mut cleared = Vec::new();
                if render_secret_rows(
                    ui,
                    &pal,
                    &mut env.variables,
                    &app.revealed_secrets,
                    &mut toggle_reveal,
                    &mut cleared,
                ) {
                    changed = true;
                }
                for key in cleared {
                    deleted_env_secrets.push((env_id.clone(), key));
                }
            }
        });

        ui.add_space(10.0);

        card_frame(&pal).show(ui, |ui| {
            ui.label(
                RichText::new("GLOBAL WORKSPACE VARIABLES (Shared Across All Environments)")
                    .size(11.5)
                    .strong()
                    .color(pal.accent_violet),
            );
            ui.separator();
            if render_secret_rows(
                ui,
                &pal,
                &mut app.globals,
                &app.revealed_secrets,
                &mut toggle_reveal,
                &mut deleted_global_secrets,
            ) {
                changed = true;
            }
        });
    });

    for (env_id, key) in deleted_env_secrets {
        app.storage.delete_env_var_secret(&env_id, &key);
    }
    for key in deleted_global_secrets {
        app.storage.delete_global_secret(&key);
    }
    if let Some(id) = toggle_reveal {
        if !app.revealed_secrets.remove(&id) {
            app.revealed_secrets.insert(id);
        }
    }
    if changed {
        app.persist();
    }
}

fn render_secret_rows(
    ui: &mut egui::Ui,
    pal: &crate::ui::theme::Palette,
    vars: &mut Vec<KeyValuePair>,
    revealed: &std::collections::HashSet<String>,
    toggle_reveal: &mut Option<String>,
    cleared_secret_keys: &mut Vec<String>,
) -> bool {
    let mut changed = false;
    let mut remove_idx = None;

    if vars.is_empty() {
        ui.label(RichText::new("No variables defined.").color(pal.text_dim));
        return false;
    }

    for (i, v) in vars.iter_mut().enumerate() {
        let is_secret = v.is_secret.unwrap_or(false);
        let is_revealed = revealed.contains(&v.id);

        ui.horizontal(|ui| {
            if ui.checkbox(&mut v.enabled, "").changed() {
                changed = true;
            }
            if ui
                .add(
                    egui::TextEdit::singleline(&mut v.key)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("VARIABLE_NAME")
                        .desired_width(215.0),
                )
                .changed()
            {
                changed = true;
            }

            let val_w = (ui.available_width() - 185.0).max(140.0);
            let val_edit = egui::TextEdit::singleline(&mut v.value)
                .font(egui::TextStyle::Monospace)
                .password(is_secret && !is_revealed)
                .hint_text(if is_secret { "••••••••••••" } else { "value" })
                .desired_width(val_w);
            if ui.add(val_edit).changed() {
                changed = true;
            }

            if is_secret && ui.small_button(if is_revealed { "Hide" } else { "Show" }).clicked() {
                *toggle_reveal = Some(v.id.clone());
            }

            let mut sec_flag = is_secret;
            if ui.checkbox(&mut sec_flag, "Secret").changed() {
                if !sec_flag {
                    cleared_secret_keys.push(v.key.clone());
                }
                v.is_secret = Some(sec_flag);
                changed = true;
            }

            if ui
                .add(
                    egui::Button::new(RichText::new("×").color(pal.text_dim))
                        .min_size(vec2(24.0, 24.0)),
                )
                .clicked()
            {
                remove_idx = Some(i);
            }
        });
        ui.add_space(2.0);
    }

    if let Some(i) = remove_idx {
        let removed = vars.remove(i);
        if removed.is_secret.unwrap_or(false) {
            cleared_secret_keys.push(removed.key);
        }
        changed = true;
    }
    changed
}
