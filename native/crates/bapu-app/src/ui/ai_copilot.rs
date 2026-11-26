use crate::app::{BapuApp, Tab};
use crate::ui::widgets::{ai_logo, badge_pill, emerald_button, primary_button};
use bapu_core::ai_copilot::AiProvider;
use egui::{vec2, Frame, Margin, RichText, Rounding, Sense, Stroke};

pub fn render_window(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.ai_modal_open;

    crate::ui::widgets::show_resizable_window(
        ctx,
        "AI Copilot  —  SQL, Payloads & Assertions",
        &mut open,
        [580.0, 470.0],
        [440.0, 280.0],
        "ai_copilot_win_scroll",
        |ui, _viewport_h| {
            // Provider Logo Cards (Ollama, OpenAI, Anthropic)
            ui.columns(3, |cols| {
                for (c_ui, provider) in cols.iter_mut().zip([
                    AiProvider::Ollama,
                    AiProvider::OpenAi,
                    AiProvider::Anthropic,
                ]) {
                    let selected = app.ai_provider == provider;
                    let stroke = if selected {
                        Stroke::new(1.5_f32, pal.accent_primary)
                    } else {
                        Stroke::new(1.0_f32, pal.border_subtle)
                    };
                    let tile = Frame::none()
                        .fill(if selected {
                            pal.bg_card_hover
                        } else {
                            pal.bg_app
                        })
                        .stroke(stroke)
                        .rounding(Rounding::same(8.0_f32))
                        .inner_margin(Margin::symmetric(10.0_f32, 8.0_f32))
                        .show(c_ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::Image::new(ai_logo(provider))
                                        .fit_to_exact_size(vec2(24.0, 24.0))
                                        .rounding(Rounding::same(5.0_f32)),
                                );
                                ui.label(
                                    RichText::new(provider.label())
                                        .size(12.0)
                                        .strong()
                                        .color(pal.text_main),
                                );
                            });
                        });
                    if tile.response.interact(Sense::click()).clicked() && app.ai_provider != provider
                    {
                        app.ai_provider = provider;
                        app.ai_base_url = provider.default_base_url().into();
                        app.ai_model = provider.default_model().into();
                        app.ai_api_key = if provider == AiProvider::Ollama {
                            String::new()
                        } else {
                            app.storage
                                .load_ai_api_key(provider.slug())
                                .unwrap_or_default()
                        };
                    }
                }
            });

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label("Endpoint:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.ai_base_url)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(240.0),
                );
                ui.label("Model:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.ai_model)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(160.0),
                );
            });

            if app.ai_provider != AiProvider::Ollama {
                ui.horizontal(|ui| {
                    ui.label("API Key (OS Keychain):");
                    let key_w = (ui.available_width() - 80.0).max(140.0);
                    if ui
                        .add(
                            egui::TextEdit::singleline(&mut app.ai_api_key)
                                .password(true)
                                .hint_text("sk-...")
                                .desired_width(key_w),
                        )
                        .changed()
                    {
                        app.storage
                            .save_ai_api_key(app.ai_provider.slug(), &app.ai_api_key);
                    }
                    if !app.ai_api_key.is_empty() && ui.small_button("Clear").clicked() {
                        app.ai_api_key.clear();
                        app.storage.save_ai_api_key(app.ai_provider.slug(), "");
                    }
                });
            } else {
                badge_pill(ui, "100% LOCAL — ZERO DATA LEAVES YOUR MACHINE", pal.accent_emerald);
            }

            ui.separator();
            ui.label(
                RichText::new("Describe the SQL query, JSON payload, or Postman assertion script:")
                    .size(12.0)
                    .color(pal.text_muted),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.ai_prompt)
                    .desired_rows(3)
                    .hint_text("e.g. Join users and orders to find top 5 customers by total paid amount")
                    .desired_width(ui.available_width()),
            );

            ui.horizontal(|ui| {
                if primary_button(
                    ui,
                    if app.ai_busy {
                        "Generating..."
                    } else {
                        "Generate Code"
                    },
                    !app.ai_busy && !app.ai_prompt.trim().is_empty(),
                )
                .clicked()
                {
                    app.run_ai_copilot();
                }
                if app.ai_busy {
                    ui.spinner();
                }
            });

            if let Some(err) = &app.ai_error {
                ui.colored_label(pal.accent_rose, err);
            }

            if !app.ai_output.is_empty() {
                ui.separator();
                ui.add(
                    egui::TextEdit::multiline(&mut app.ai_output)
                        .code_editor()
                        .desired_rows(8)
                        .desired_width(ui.available_width()),
                );
                ui.horizontal(|ui| {
                    if emerald_button(ui, "Insert into SQL Editor", true).clicked() {
                        *app.active_sql_mut() = app.ai_output.clone();
                        app.tab = Tab::Database;
                    }
                    if ui.button("Insert into API Body").clicked() {
                        app.active_request.body_content = app.ai_output.clone();
                        app.tab = Tab::Api;
                    }
                    if ui.button("Copy").clicked() {
                        ui.output_mut(|o| o.copied_text = app.ai_output.clone());
                    }
                });
            }
        });
    app.ai_modal_open = open;
}
