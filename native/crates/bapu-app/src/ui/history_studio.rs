use crate::app::{BapuApp, Tab};
use crate::ui::widgets::{
    badge_pill, card_frame, compact_card_frame, paint_icon, status_pill, VectorIcon,
};
use egui::{Frame, Margin, RichText, Rounding, Stroke};

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();

    compact_card_frame(&pal).show(ui, |ui| {
        ui.horizontal(|ui| {
            paint_icon(ui, VectorIcon::Clock, 20.0, pal.accent_cyan);
            ui.label(
                RichText::new("Execution Audit & Request History")
                    .size(15.5)
                    .strong()
                    .color(pal.text_main),
            );
            badge_pill(
                ui,
                &format!("{} entries", app.history.len()),
                pal.accent_primary,
            );

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Clear History").clicked() {
                    app.history.clear();
                    let _ = app.storage.save_history(&app.history);
                }
                ui.add(
                    egui::TextEdit::singleline(&mut app.history_filter)
                        .hint_text("Filter history...")
                        .desired_width(220.0),
                );
            });
        });
    });

    ui.add_space(10.0);

    if app.history.is_empty() {
        card_frame(&pal).show(ui, |ui| {
            ui.label(
                RichText::new("No API calls or SQL queries recorded yet — send a request or run a query to populate history.")
                    .color(pal.text_muted),
            );
        });
        return;
    }

    let filter = app.history_filter.trim().to_ascii_lowercase();
    let mut replay_sql: Option<String> = None;

    egui::ScrollArea::vertical().show(ui, |ui| {
        for item in &app.history {
            if !filter.is_empty()
                && !item.title.to_ascii_lowercase().contains(&filter)
                && !item.subtitle.to_ascii_lowercase().contains(&filter)
            {
                continue;
            }
            Frame::none()
                .fill(pal.bg_card)
                .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                .rounding(Rounding::same(8.0_f32))
                .inner_margin(Margin::symmetric(12.0_f32, 9.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        if item.kind == "api" {
                            badge_pill(ui, "HTTP", pal.accent_cyan);
                        } else {
                            badge_pill(ui, "SQL", pal.accent_emerald);
                        }
                        if let Some(code) = item.status {
                            status_pill(ui, code, "");
                        }
                        ui.label(
                            RichText::new(&item.title)
                                .monospace()
                                .strong()
                                .color(pal.text_main),
                        );
                        ui.label(RichText::new(&item.subtitle).size(11.5).color(pal.text_muted));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if item.kind == "sql" && ui.small_button("Load SQL").clicked() {
                                replay_sql = Some(item.title.clone());
                            }
                            ui.label(
                                RichText::new(&item.timestamp)
                                    .monospace()
                                    .size(11.0)
                                    .color(pal.text_dim),
                            );
                        });
                    });
                });
            ui.add_space(5.0);
        }
    });

    if let Some(sql) = replay_sql {
        *app.active_sql_mut() = sql;
        app.tab = Tab::Database;
    }
}
