//! Reusable brand logos, custom vector icons (zero tofu emojis!), semantic pills,
//! styled cards, and action buttons for Bapu Studio.

use crate::ui::theme::Palette;
use bapu_core::ai_copilot::AiProvider;
use bapu_core::types::{DbKind, HttpMethod};
use egui::{pos2, vec2, Color32, Frame, ImageSource, Margin, Pos2, Rect, Response, RichText, Rounding, Sense, Stroke, Ui};

// ---- Embedded Brand Logos (PNG + Crisp SVGs rendered via resvg) ----

pub fn bapu_logo() -> ImageSource<'static> {
    egui::include_image!("../../assets/logo.png")
}

pub fn db_logo(kind: DbKind) -> ImageSource<'static> {
    match kind {
        DbKind::Postgres
        | DbKind::Cockroachdb
        | DbKind::Yugabytedb
        | DbKind::Timescaledb
        | DbKind::Redshift => egui::include_image!("../../assets/logos/postgres.svg"),
        DbKind::Mysql | DbKind::Mariadb | DbKind::Tidb => {
            egui::include_image!("../../assets/logos/mysql.svg")
        }
        DbKind::Sqlite | DbKind::Libsql | DbKind::Duckdb => {
            egui::include_image!("../../assets/logos/sqlite.svg")
        }
        DbKind::Mssql | DbKind::AzureSql => egui::include_image!("../../assets/logos/mssql.svg"),
        DbKind::Oracle => egui::include_image!("../../assets/logos/oracle.svg"),
        DbKind::Clickhouse | DbKind::Questdb => {
            egui::include_image!("../../assets/logos/clickhouse.svg")
        }
        DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks | DbKind::Trino => {
            egui::include_image!("../../assets/logos/snowflake.svg")
        }
        DbKind::Mongodb | DbKind::Couchdb => egui::include_image!("../../assets/logos/mongodb.svg"),
        DbKind::Redis | DbKind::Influxdb => egui::include_image!("../../assets/logos/redis.svg"),
        DbKind::Cassandra | DbKind::Scylladb => {
            egui::include_image!("../../assets/logos/cassandra.svg")
        }
        DbKind::Elasticsearch => egui::include_image!("../../assets/logos/elasticsearch.svg"),
        DbKind::Neo4j => egui::include_image!("../../assets/logos/neo4j.svg"),
        DbKind::Db2 | DbKind::SapHana | DbKind::Teradata | DbKind::Firebird | DbKind::Odbc => {
            egui::include_image!("../../assets/logos/odbc.svg")
        }
    }
}

pub fn db_brand_name(kind: DbKind) -> &'static str {
    kind.display_name()
}

pub fn db_accent(kind: DbKind) -> Color32 {
    match kind {
        DbKind::Postgres | DbKind::Yugabytedb => Color32::from_rgb(91, 155, 213),
        DbKind::Cockroachdb => Color32::from_rgb(105, 51, 255),
        DbKind::Timescaledb => Color32::from_rgb(253, 181, 21),
        DbKind::Mysql => Color32::from_rgb(242, 145, 17),
        DbKind::Mariadb => Color32::from_rgb(192, 118, 90),
        DbKind::Tidb => Color32::from_rgb(227, 12, 52),
        DbKind::Sqlite | DbKind::Libsql => Color32::from_rgb(56, 189, 248),
        DbKind::Mssql => Color32::from_rgb(239, 68, 68),
        DbKind::AzureSql => Color32::from_rgb(0, 120, 212),
        DbKind::Oracle => Color32::from_rgb(248, 0, 0),
        DbKind::Clickhouse => Color32::from_rgb(250, 204, 21),
        DbKind::Duckdb => Color32::from_rgb(255, 240, 0),
        DbKind::Snowflake => Color32::from_rgb(41, 181, 232),
        DbKind::Bigquery => Color32::from_rgb(66, 133, 244),
        DbKind::Databricks => Color32::from_rgb(255, 54, 33),
        DbKind::Redshift => Color32::from_rgb(140, 79, 255),
        DbKind::Trino => Color32::from_rgb(221, 0, 161),
        DbKind::Questdb => Color32::from_rgb(209, 70, 113),
        DbKind::Mongodb => Color32::from_rgb(16, 185, 129),
        DbKind::Couchdb => Color32::from_rgb(228, 37, 40),
        DbKind::Redis => Color32::from_rgb(239, 68, 68),
        DbKind::Cassandra | DbKind::Scylladb => Color32::from_rgb(56, 189, 248),
        DbKind::Elasticsearch => Color32::from_rgb(20, 184, 166),
        DbKind::Neo4j => Color32::from_rgb(1, 139, 255),
        DbKind::Influxdb => Color32::from_rgb(34, 173, 246),
        DbKind::Db2 => Color32::from_rgb(15, 98, 254),
        DbKind::SapHana => Color32::from_rgb(0, 143, 211),
        DbKind::Teradata | DbKind::Firebird => Color32::from_rgb(243, 116, 33),
        DbKind::Odbc => Color32::from_rgb(148, 163, 184),
    }
}

pub fn ai_logo(provider: AiProvider) -> ImageSource<'static> {
    match provider {
        AiProvider::Ollama => egui::include_image!("../../assets/logos/ollama.svg"),
        AiProvider::OpenAi => egui::include_image!("../../assets/logos/openai.svg"),
        AiProvider::Anthropic => egui::include_image!("../../assets/logos/anthropic.svg"),
    }
}

pub fn postman_logo() -> ImageSource<'static> {
    egui::include_image!("../../assets/logos/postman.svg")
}

pub fn openapi_logo() -> ImageSource<'static> {
    egui::include_image!("../../assets/logos/openapi.svg")
}

// ---- Crisp Vector Icons Drawn via egui::Painter (No Missing Font Glyphs!) ----

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub enum VectorIcon {
    Globe,
    Database,
    Radio,
    ShieldKey,
    Clock,
    Sparkles,
    Sun,
    Moon,
    Heart,
    Play,
    Bolt,
    Folder,
    Search,
    Plus,
    Key,
}

pub fn paint_icon(ui: &mut Ui, icon: VectorIcon, size: f32, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(size, size), Sense::hover());
    paint_icon_in_rect(ui.painter(), rect, icon, color);
}

pub fn paint_icon_in_rect(painter: &egui::Painter, rect: Rect, icon: VectorIcon, color: Color32) {
    let c = rect.center();
    let r = rect.width().min(rect.height()) * 0.44;
    let stroke = Stroke::new(1.55_f32, color);

    match icon {
        VectorIcon::Globe => {
            painter.circle_stroke(c, r, stroke);
            painter.line_segment([pos2(c.x - r, c.y), pos2(c.x + r, c.y)], stroke);
            painter.line_segment([pos2(c.x, c.y - r), pos2(c.x, c.y + r)], stroke);
            let inner = Stroke::new(1.2_f32, color);
            painter.rect_stroke(
                Rect::from_center_size(c, vec2(r * 1.05, r * 1.9)),
                Rounding::same(r),
                inner,
            );
        }
        VectorIcon::Database => {
            let w = r * 1.75;
            let top = pos2(c.x, c.y - r * 0.62);
            let mid = pos2(c.x, c.y);
            let bot = pos2(c.x, c.y + r * 0.62);
            painter.rect_stroke(
                Rect::from_center_size(top, vec2(w, r * 0.68)),
                Rounding::same(r * 0.34),
                stroke,
            );
            painter.line_segment([pos2(c.x - w * 0.5, top.y), pos2(c.x - w * 0.5, bot.y)], stroke);
            painter.line_segment([pos2(c.x + w * 0.5, top.y), pos2(c.x + w * 0.5, bot.y)], stroke);
            painter.line_segment(
                [pos2(c.x - w * 0.45, mid.y + 1.0), pos2(c.x + w * 0.45, mid.y + 1.0)],
                stroke,
            );
            painter.line_segment(
                [pos2(c.x - w * 0.42, bot.y + r * 0.25), pos2(c.x + w * 0.42, bot.y + r * 0.25)],
                stroke,
            );
        }
        VectorIcon::Radio => {
            painter.circle_filled(c, r * 0.24, color);
            painter.circle_stroke(c, r * 0.58, Stroke::new(1.4_f32, color.gamma_multiply(0.85)));
            painter.circle_stroke(c, r * 0.95, Stroke::new(1.3_f32, color.gamma_multiply(0.55)));
        }
        VectorIcon::ShieldKey => {
            let pts = vec![
                pos2(c.x, c.y - r),
                pos2(c.x + r * 0.85, c.y - r * 0.55),
                pos2(c.x + r * 0.72, c.y + r * 0.28),
                pos2(c.x, c.y + r),
                pos2(c.x - r * 0.72, c.y + r * 0.28),
                pos2(c.x - r * 0.85, c.y - r * 0.55),
            ];
            painter.add(egui::Shape::closed_line(pts, stroke));
            painter.circle_filled(pos2(c.x, c.y - r * 0.12), r * 0.22, color);
            painter.line_segment([pos2(c.x, c.y), pos2(c.x, c.y + r * 0.42)], stroke);
        }
        VectorIcon::Clock => {
            painter.circle_stroke(c, r, stroke);
            painter.line_segment([c, pos2(c.x, c.y - r * 0.58)], stroke);
            painter.line_segment([c, pos2(c.x + r * 0.46, c.y + r * 0.22)], stroke);
        }
        VectorIcon::Sparkles => {
            let pts = vec![
                pos2(c.x, c.y - r),
                pos2(c.x + r * 0.28, c.y - r * 0.28),
                pos2(c.x + r, c.y),
                pos2(c.x + r * 0.28, c.y + r * 0.28),
                pos2(c.x, c.y + r),
                pos2(c.x - r * 0.28, c.y + r * 0.28),
                pos2(c.x - r, c.y),
                pos2(c.x - r * 0.28, c.y - r * 0.28),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                color.gamma_multiply(0.25),
                stroke,
            ));
        }
        VectorIcon::Sun => {
            painter.circle_stroke(c, r * 0.5, stroke);
            for i in 0..8 {
                let angle = (i as f32) * std::f32::consts::TAU / 8.0;
                let dir = vec2(angle.cos(), angle.sin());
                painter.line_segment([c + dir * (r * 0.72), c + dir * r], stroke);
            }
        }
        VectorIcon::Moon => {
            painter.circle_stroke(c, r * 0.82, stroke);
            painter.circle_filled(
                pos2(c.x + r * 0.34, c.y - r * 0.28),
                r * 0.62,
                Color32::from_rgb(15, 23, 42),
            );
        }
        VectorIcon::Heart => {
            let top_l = pos2(c.x - r * 0.45, c.y - r * 0.25);
            let top_r = pos2(c.x + r * 0.45, c.y - r * 0.25);
            painter.circle_filled(top_l, r * 0.46, color);
            painter.circle_filled(top_r, r * 0.46, color);
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(c.x - r * 0.88, c.y - r * 0.05),
                    pos2(c.x + r * 0.88, c.y - r * 0.05),
                    pos2(c.x, c.y + r * 0.92),
                ],
                color,
                Stroke::NONE,
            ));
        }
        VectorIcon::Play => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(c.x - r * 0.55, c.y - r * 0.8),
                    pos2(c.x + r * 0.82, c.y),
                    pos2(c.x - r * 0.55, c.y + r * 0.8),
                ],
                color,
                Stroke::NONE,
            ));
        }
        VectorIcon::Bolt => {
            painter.add(egui::Shape::convex_polygon(
                vec![
                    pos2(c.x + r * 0.15, c.y - r),
                    pos2(c.x - r * 0.75, c.y + r * 0.1),
                    pos2(c.x - r * 0.05, c.y + r * 0.1),
                    pos2(c.x - r * 0.2, c.y + r),
                    pos2(c.x + r * 0.75, c.y - r * 0.1),
                    pos2(c.x + r * 0.05, c.y - r * 0.1),
                ],
                color,
                Stroke::NONE,
            ));
        }
        VectorIcon::Folder => {
            let pts = vec![
                pos2(c.x - r, c.y - r * 0.68),
                pos2(c.x - r * 0.22, c.y - r * 0.68),
                pos2(c.x, c.y - r * 0.35),
                pos2(c.x + r, c.y - r * 0.35),
                pos2(c.x + r, c.y + r * 0.72),
                pos2(c.x - r, c.y + r * 0.72),
            ];
            painter.add(egui::Shape::convex_polygon(
                pts,
                color.gamma_multiply(0.2),
                stroke,
            ));
        }
        VectorIcon::Search => {
            let sc = pos2(c.x - r * 0.15, c.y - r * 0.15);
            painter.circle_stroke(sc, r * 0.62, stroke);
            painter.line_segment(
                [pos2(sc.x + r * 0.44, sc.y + r * 0.44), pos2(c.x + r * 0.85, c.y + r * 0.85)],
                Stroke::new(1.9_f32, color),
            );
        }
        VectorIcon::Plus => {
            painter.line_segment([pos2(c.x - r * 0.75, c.y), pos2(c.x + r * 0.75, c.y)], stroke);
            painter.line_segment([pos2(c.x, c.y - r * 0.75), pos2(c.x, c.y + r * 0.75)], stroke);
        }
        VectorIcon::Key => {
            let kc = pos2(c.x - r * 0.35, c.y);
            painter.circle_stroke(kc, r * 0.45, stroke);
            painter.line_segment([pos2(kc.x + r * 0.45, c.y), pos2(c.x + r * 0.9, c.y)], stroke);
            painter.line_segment(
                [pos2(c.x + r * 0.65, c.y), pos2(c.x + r * 0.65, c.y + r * 0.38)],
                stroke,
            );
        }
    }
}

// ---- Semantic Badges & Styled Controls ----

pub fn method_color(method: HttpMethod) -> Color32 {
    match method {
        HttpMethod::GET => Color32::from_rgb(6, 182, 212),      // #06b6d4
        HttpMethod::POST => Color32::from_rgb(16, 185, 129),    // #10b981
        HttpMethod::PUT => Color32::from_rgb(245, 158, 11),     // #f59e0b
        HttpMethod::PATCH => Color32::from_rgb(139, 92, 246),   // #8b5cf6
        HttpMethod::DELETE => Color32::from_rgb(239, 68, 68),   // #ef4444
        HttpMethod::HEAD => Color32::from_rgb(99, 102, 241),    // #6366f1
        HttpMethod::OPTIONS => Color32::from_rgb(20, 184, 166), // #14b8a6
    }
}

pub fn method_badge(ui: &mut Ui, method: HttpMethod) {
    let color = method_color(method);
    Frame::none()
        .fill(color.gamma_multiply(0.15))
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.35)))
        .rounding(Rounding::same(5.0_f32))
        .inner_margin(Margin::symmetric(6.0_f32, 2.0_f32))
        .show(ui, |ui| {
            ui.label(
                RichText::new(method.as_str())
                    .monospace()
                    .size(10.5)
                    .strong()
                    .color(color),
            );
        });
}

pub fn badge_pill(ui: &mut Ui, text: &str, color: Color32) {
    Frame::none()
        .fill(color.gamma_multiply(0.15))
        .stroke(Stroke::new(1.0_f32, color.gamma_multiply(0.35)))
        .rounding(Rounding::same(5.0_f32))
        .inner_margin(Margin::symmetric(7.0_f32, 2.0_f32))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.5).strong().color(color));
        });
}

pub fn status_pill(ui: &mut Ui, status: u16, status_text: &str) {
    let color = if (200..300).contains(&status) {
        Color32::from_rgb(16, 185, 129)
    } else if (300..400).contains(&status) {
        Color32::from_rgb(6, 182, 212)
    } else if (400..500).contains(&status) {
        Color32::from_rgb(245, 158, 11)
    } else {
        Color32::from_rgb(239, 68, 68)
    };
    let label = if status_text.is_empty() {
        format!("{status}")
    } else {
        format!("{status} {status_text}")
    };
    badge_pill(ui, &label, color);
}

pub fn card_frame(pal: &Palette) -> Frame {
    Frame::none()
        .fill(pal.bg_card)
        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
        .rounding(Rounding::same(10.0_f32))
        .inner_margin(Margin::same(14.0_f32))
}

pub fn compact_card_frame(pal: &Palette) -> Frame {
    Frame::none()
        .fill(pal.bg_card)
        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
        .rounding(Rounding::same(8.0_f32))
        .inner_margin(Margin::symmetric(12.0_f32, 9.0_f32))
}

pub fn primary_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let fill = if enabled {
        Color32::from_rgb(37, 99, 235)
    } else {
        Color32::from_rgb(51, 65, 85)
    };
    let btn = egui::Button::new(
        RichText::new(label)
            .strong()
            .size(13.0)
            .color(Color32::WHITE),
    )
    .fill(fill)
    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(96, 165, 250).gamma_multiply(0.5)))
    .rounding(Rounding::same(7.0_f32))
    .min_size(vec2(0.0, 32.0));
    ui.add_enabled(enabled, btn)
}

pub fn emerald_button(ui: &mut Ui, label: &str, enabled: bool) -> Response {
    let fill = if enabled {
        Color32::from_rgb(5, 150, 105)
    } else {
        Color32::from_rgb(51, 65, 85)
    };
    let btn = egui::Button::new(
        RichText::new(label)
            .strong()
            .size(12.8)
            .color(Color32::WHITE),
    )
    .fill(fill)
    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(52, 211, 153).gamma_multiply(0.5)))
    .rounding(Rounding::same(7.0_f32))
    .min_size(vec2(0.0, 30.0));
    ui.add_enabled(enabled, btn)
}

pub fn subtab_button(ui: &mut Ui, pal: &Palette, selected: bool, label: &str) -> Response {
    let text_color = if selected { pal.text_main } else { pal.text_dim };
    let fill = if selected {
        pal.bg_card_hover
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if selected {
        Stroke::new(1.0_f32, pal.accent_primary.gamma_multiply(0.65))
    } else {
        Stroke::NONE
    };
    let resp = ui.add(
        egui::Button::new(RichText::new(label).size(12.2).strong().color(text_color))
            .fill(fill)
            .stroke(stroke)
            .rounding(Rounding::same(6.0_f32)),
    );
    if selected {
        let r = resp.rect;
        ui.painter().line_segment(
            [Pos2::new(r.left() + 6.0, r.bottom()), Pos2::new(r.right() - 6.0, r.bottom())],
            Stroke::new(2.0_f32, pal.accent_primary),
        );
    }
    resp
}

/// Renders a floating studio modal window that resizes smoothly in both horizontal and
/// vertical directions (growing beyond content height without snapping back, and shrinking
/// below content height with smooth vertical scrolling).
pub fn show_resizable_window<R>(
    ctx: &egui::Context,
    title: impl Into<egui::WidgetText>,
    open: &mut bool,
    default_size: [f32; 2],
    min_size: [f32; 2],
    scroll_id: &'static str,
    add_contents: impl FnOnce(&mut Ui, f32) -> R,
) -> Option<Rect> {
    egui::Window::new(title)
        .open(open)
        .resizable([true, true])
        .default_size(vec2(default_size[0], default_size[1]))
        .min_size(vec2(min_size[0], min_size[1]))
        .show(ctx, |ui| {
            let viewport_h = ui.available_height().max(min_size[1] - 48.0);
            egui::ScrollArea::vertical()
                .id_salt(scroll_id)
                .auto_shrink([false, false])
                .show(ui, |ui| add_contents(ui, viewport_h));
        })
        .map(|ir| ir.response.rect)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, PointerButton, RawInput};

    fn run_modal_frame(ctx: &egui::Context, events: Vec<Event>) -> Rect {
        let mut open = true;
        let mut rect_out = Rect::NOTHING;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(1600.0, 1200.0))),
            events,
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            if let Some(r) = show_resizable_window(
                ctx,
                "Universal Database Connection Configuration (33 Engines)",
                &mut open,
                [680.0, 620.0],
                [480.0, 320.0],
                "test_db_conn_scroll",
                |ui, viewport_h| {
                    let picker_h = (viewport_h - 320.0).clamp(135.0, 520.0);
                    egui::ScrollArea::vertical()
                        .id_salt("test_picker")
                        .max_height(picker_h)
                        .show(ui, |ui| {
                            for row in 0..9 {
                                ui.label(format!("Engine row {row}"));
                                ui.add_space(18.0);
                            }
                        });
                    for field in 0..10 {
                        let mut val = format!("field_value_{field}");
                        ui.add(
                            egui::TextEdit::singleline(&mut val)
                                .desired_width(ui.available_width()),
                        );
                    }
                },
            ) {
                rect_out = r;
            }
        });
        rect_out
    }

    #[test]
    fn db_connection_modal_resizes_both_vertically_and_horizontally() {
        let ctx = egui::Context::default();

        // Warm-up initial layout frames
        let mut initial = Rect::NOTHING;
        for _ in 0..3 {
            initial = run_modal_frame(&ctx, vec![]);
        }
        assert!(
            initial.width() >= 660.0 && initial.height() >= 600.0,
            "unexpected initial rect: {initial:?}"
        );

        // 1. Drag bottom-right corner UP (-200px vertically) and LEFT (-100px horizontally)
        //    to verify shrinking below intrinsic content height works and persists!
        let corner = pos2(initial.right() - 2.0, initial.bottom() - 2.0);
        let shrunk_target = corner + vec2(-100.0, -200.0);
        let _ = run_modal_frame(
            &ctx,
            vec![
                Event::PointerMoved(corner),
                Event::PointerButton {
                    pos: corner,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = run_modal_frame(&ctx, vec![Event::PointerMoved(shrunk_target)]);
        let _ = run_modal_frame(
            &ctx,
            vec![Event::PointerButton {
                pos: shrunk_target,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        let mut after_shrink = Rect::NOTHING;
        for _ in 0..3 {
            after_shrink = run_modal_frame(&ctx, vec![]);
        }
        assert!(
            after_shrink.height() < initial.height() - 150.0,
            "vertical shrink snapped back! initial_h={}, after_shrink_h={}",
            initial.height(),
            after_shrink.height()
        );
        assert!(
            after_shrink.width() < initial.width() - 70.0,
            "horizontal shrink failed! initial_w={}, after_shrink_w={}",
            initial.width(),
            after_shrink.width()
        );

        // 2. Drag bottom-right corner DOWN (+440px vertically, beyond intrinsic content height!)
        //    and RIGHT (+260px horizontally) and verify it stays expanded after release!
        let corner2 = pos2(after_shrink.right() - 2.0, after_shrink.bottom() - 2.0);
        let grown_target = corner2 + vec2(260.0, 440.0);
        let _ = run_modal_frame(
            &ctx,
            vec![
                Event::PointerMoved(corner2),
                Event::PointerButton {
                    pos: corner2,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let _ = run_modal_frame(&ctx, vec![Event::PointerMoved(grown_target)]);
        let _ = run_modal_frame(
            &ctx,
            vec![Event::PointerButton {
                pos: grown_target,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        let mut after_grow = Rect::NOTHING;
        for _ in 0..3 {
            after_grow = run_modal_frame(&ctx, vec![]);
        }
        assert!(
            after_grow.height() > after_shrink.height() + 380.0,
            "vertical grow snapped back! after_shrink_h={}, after_grow_h={}",
            after_shrink.height(),
            after_grow.height()
        );
        assert!(
            after_grow.width() > after_shrink.width() + 220.0,
            "horizontal grow failed! after_shrink_w={}, after_grow_w={}",
            after_shrink.width(),
            after_grow.width()
        );
    }

    fn run_card_frame(ctx: &egui::Context, events: Vec<Event>) -> (Rect, Rect, bool, bool) {
        let mut left_rect_out = Rect::NOTHING;
        let mut del_rect_out = Rect::NOTHING;
        let mut selected = false;
        let mut deleted = false;
        let raw = RawInput {
            screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 600.0))),
            events,
            ..Default::default()
        };
        let _ = ctx.run(raw, |ctx| {
            egui::SidePanel::left("test_side").exact_width(272.0).show(ctx, |ui| {
                Frame::none()
                    .inner_margin(Margin::symmetric(10.0_f32, 8.0_f32))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let del_reserve = 30.0;
                            let left_w = (ui.available_width() - del_reserve).max(90.0);
                            let left_ui = ui.allocate_ui_with_layout(
                                vec2(left_w, 30.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_max_width(left_w);
                                    ui.add(
                                        egui::Label::new(
                                            "Very Long Database Or API Request Name That Must Truncate Cleanly",
                                        )
                                        .selectable(false)
                                        .truncate(),
                                    );
                                },
                            );
                            left_rect_out = left_ui.response.rect;
                            if ui
                                .interact(
                                    left_ui.response.rect,
                                    ui.id().with("row_select"),
                                    Sense::click(),
                                )
                                .clicked()
                            {
                                selected = true;
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    let del_btn = ui.add(
                                        egui::Button::new("×").min_size(vec2(24.0, 24.0)),
                                    );
                                    del_rect_out = del_btn.rect;
                                    if del_btn.clicked() {
                                        deleted = true;
                                    }
                                },
                            );
                        });
                    });
            });
        });
        (left_rect_out, del_rect_out, selected, deleted)
    }

    #[test]
    fn sidebar_disjoint_card_allows_delete_click_without_click_stealing() {
        let ctx = egui::Context::default();

        // Warm-up layout
        let (left_rect_out, del_rect_out, _, _) = run_card_frame(&ctx, vec![]);
        assert!(
            !left_rect_out.intersects(del_rect_out),
            "select rect {left_rect_out:?} must not overlap delete button rect {del_rect_out:?}"
        );
        assert!(
            del_rect_out.right() <= 272.0,
            "delete button pushed off sidebar: {del_rect_out:?}"
        );

        // 1. Click directly on the `×` delete button -> MUST trigger `deleted = true` and `selected = false`
        let del_center = del_rect_out.center();
        let _ = run_card_frame(
            &ctx,
            vec![
                Event::PointerMoved(del_center),
                Event::PointerButton {
                    pos: del_center,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let (_, _, selected, deleted) = run_card_frame(
            &ctx,
            vec![Event::PointerButton {
                pos: del_center,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(deleted, "clicking `×` did not fire delete!");
        assert!(!selected, "clicking `×` was stolen by row select!");

        // 2. Click on the card label area -> MUST trigger `selected = true` and `deleted = false`
        let left_center = left_rect_out.center();
        let _ = run_card_frame(
            &ctx,
            vec![
                Event::PointerMoved(left_center),
                Event::PointerButton {
                    pos: left_center,
                    button: PointerButton::Primary,
                    pressed: true,
                    modifiers: Default::default(),
                },
            ],
        );
        let (_, _, selected, deleted) = run_card_frame(
            &ctx,
            vec![Event::PointerButton {
                pos: left_center,
                button: PointerButton::Primary,
                pressed: false,
                modifiers: Default::default(),
            }],
        );
        assert!(selected, "clicking card body did not fire select!");
        assert!(!deleted, "clicking card body accidentally fired delete!");
    }
}
