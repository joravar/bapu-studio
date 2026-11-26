use crate::app::BapuApp;
use crate::ui::widgets::{
    badge_pill, compact_card_frame, emerald_button, paint_icon, primary_button, VectorIcon,
};
use bapu_core::stream_client::{FrameDirection, StreamKind};
use egui::{Frame, Margin, RichText, Rounding, Stroke};

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();

    // Stream Connection Bar Card
    compact_card_frame(&pal).show(ui, |ui| {
        ui.horizontal(|ui| {
            paint_icon(
                ui,
                VectorIcon::Radio,
                18.0,
                if app.stream_connected {
                    pal.accent_emerald
                } else {
                    pal.accent_cyan
                },
            );

            egui::ComboBox::from_id_salt("stream_protocol")
                .selected_text(match app.stream_kind {
                    StreamKind::WebSocket => "WebSocket",
                    StreamKind::Sse => "SSE EventStream",
                })
                .width(135.0)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut app.stream_kind, StreamKind::WebSocket, "WebSocket");
                    ui.selectable_value(
                        &mut app.stream_kind,
                        StreamKind::Sse,
                        "SSE EventStream",
                    );
                });

            let url_w = (ui.available_width() - 210.0).max(160.0);
            ui.add(
                egui::TextEdit::singleline(&mut app.stream_url)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("wss://echo.websocket.events  or  https://.../events")
                    .desired_width(url_w),
            );

            if app.stream_connected {
                if ui
                    .add(
                        egui::Button::new(
                            RichText::new("■  Disconnect")
                                .strong()
                                .color(pal.accent_rose),
                        )
                        .stroke(Stroke::new(1.0_f32, pal.accent_rose)),
                    )
                    .clicked()
                {
                    app.disconnect_stream();
                }
            } else if emerald_button(ui, "▶  Connect", !app.stream_url.trim().is_empty()).clicked()
            {
                app.connect_stream();
            }

            if ui.button("Clear").clicked() {
                app.stream_frames.clear();
            }
        });

        if app.stream_kind == StreamKind::WebSocket {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("Outbound Frame:").size(12.0).color(pal.text_muted));
                let msg_w = (ui.available_width() - 105.0).max(140.0);
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut app.stream_outbound)
                        .font(egui::TextStyle::Monospace)
                        .hint_text("{\"event\": \"ping\"}")
                        .desired_width(msg_w),
                );
                if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && app.stream_connected {
                    app.send_stream_message();
                }
                if primary_button(ui, "Send Frame", app.stream_connected).clicked() {
                    app.send_stream_message();
                }
            });
        }
    });

    if let Some(err) = &app.stream_error {
        ui.add_space(6.0);
        ui.colored_label(pal.accent_rose, err);
    }

    ui.add_space(10.0);

    // Live Frame Console
    let avail_h = ui.available_height().max(240.0);
    Frame::none()
        .fill(pal.bg_console)
        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
        .rounding(Rounding::same(10.0_f32))
        .inner_margin(Margin::same(14.0_f32))
        .show(ui, |ui| {
            ui.set_min_height(avail_h - 28.0);
            ui.horizontal(|ui| {
                badge_pill(
                    ui,
                    if app.stream_connected {
                        "LIVE SOCKET"
                    } else {
                        "IDLE"
                    },
                    if app.stream_connected {
                        pal.accent_emerald
                    } else {
                        pal.text_dim
                    },
                );
                ui.label(
                    RichText::new(format!("{} frame(s) captured", app.stream_frames.len()))
                        .size(12.0)
                        .color(pal.text_muted),
                );
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(avail_h - 80.0)
                .show(ui, |ui| {
                    for frame in &app.stream_frames {
                        let (badge, color) = match frame.direction {
                            FrameDirection::Incoming => ("↙ IN ", pal.accent_emerald),
                            FrameDirection::Outgoing => ("↗ OUT", pal.accent_primary),
                            FrameDirection::System => ("● SYS", pal.text_dim),
                        };
                        ui.horizontal_wrapped(|ui| {
                            ui.label(
                                RichText::new(&frame.timestamp)
                                    .monospace()
                                    .size(11.0)
                                    .color(pal.text_dim),
                            );
                            badge_pill(ui, badge, color);
                            ui.label(
                                RichText::new(&frame.payload)
                                    .monospace()
                                    .color(pal.text_main),
                            );
                        });
                        ui.add_space(2.0);
                    }
                });
        });
}
