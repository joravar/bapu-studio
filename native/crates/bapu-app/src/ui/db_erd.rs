//! Interactive Entity-Relationship Diagram (ERD) canvas using `egui::Painter`.
//! Supports draggable table cards, zoom, auto-layout, Bezier Foreign Key connectors,
//! click-to-query table selection, Mermaid `erDiagram` copy, SVG export, and visual Table Designer launch.

use crate::app::BapuApp;
use crate::ui::widgets::badge_pill;
use bapu_core::db_ddl::{generate_erd_mermaid, generate_erd_svg, generate_schema_ddl};
use bapu_core::types::TableSchema;
use egui::{pos2, vec2, FontId, Pos2, Rect, RichText, Rounding, Sense, Stroke};
use std::collections::HashMap;

pub fn ensure_layout(tables: &[TableSchema], positions: &mut HashMap<String, [f32; 2]>, force_reset: bool) {
    if force_reset {
        positions.clear();
    }
    let cols_per_row = 3usize;
    for (idx, t) in tables.iter().enumerate() {
        positions.entry(t.name.clone()).or_insert_with(|| {
            let col = idx % cols_per_row;
            let row = idx / cols_per_row;
            [28.0 + (col as f32) * 290.0, 28.0 + (row as f32) * 230.0]
        });
    }
}

pub fn render_erd(app: &mut BapuApp, ui: &mut egui::Ui, avail_h: f32) {
    let pal = app.palette();
    let Some(db) = app.databases.get(app.active_db_index).cloned() else {
        ui.label("No active database connection.");
        return;
    };

    ensure_layout(&db.tables, &mut app.erd_positions, false);

    // ERD Action Toolbar
    ui.horizontal_wrapped(|ui| {
        badge_pill(ui, &format!("{} entities", db.tables.len()), pal.accent_cyan);
        let fk_total: usize = db.tables.iter().map(|t| t.foreign_keys.len()).sum();
        badge_pill(ui, &format!("{fk_total} FK relations"), pal.accent_amber);

        if ui.small_button("Auto-Layout").clicked() {
            ensure_layout(&db.tables, &mut app.erd_positions, true);
        }
        if ui.small_button("Zoom -").clicked() {
            app.erd_zoom = (app.erd_zoom - 0.1).clamp(0.55, 1.6);
        }
        ui.label(RichText::new(format!("{:.0}%", app.erd_zoom * 100.0)).monospace().size(11.0));
        if ui.small_button("Zoom +").clicked() {
            app.erd_zoom = (app.erd_zoom + 0.1).clamp(0.55, 1.6);
        }

        ui.separator();
        if ui.small_button("+ Design New Table (DDL)").clicked() {
            app.table_designer_open = true;
        }
        if ui.small_button("Copy Mermaid ERD").clicked() {
            let mmd = generate_erd_mermaid(&db.tables);
            ui.output_mut(|o| o.copied_text = mmd);
            app.db_status = Some("Copied Mermaid `erDiagram` to clipboard.".into());
        }
        if ui.small_button("Export SVG...").clicked() {
            let svg = generate_erd_svg(&db.tables);
            if let Some(path) = rfd::FileDialog::new()
                .set_file_name(format!("{}_erd.svg", db.database.replace(':', "_")))
                .add_filter("SVG Vector Diagram", &["svg"])
                .save_file()
            {
                let _ = std::fs::write(&path, svg);
                app.db_status = Some(format!("Saved ER Diagram SVG to {}", path.display()));
            }
        }
        if ui.small_button("Copy Full Schema DDL").clicked() {
            let ddl = generate_schema_ddl(&db.tables, db.kind);
            ui.output_mut(|o| o.copied_text = ddl);
            app.db_status = Some("Copied topological full-schema DDL to clipboard.".into());
        }
    });

    ui.add_space(6.0);

    if db.tables.is_empty() {
        ui.label(
            RichText::new("No tables loaded yet. Click 'Refresh Schema' or '+ Design New Table' to start modeling.")
                .color(pal.text_muted),
        );
        return;
    }

    let zoom = app.erd_zoom;
    let card_w = 232.0 * zoom;
    let header_h = 28.0 * zoom;
    let row_h = 19.0 * zoom;

    egui::ScrollArea::both()
        .max_height((avail_h - 68.0).max(200.0))
        .show(ui, |ui| {
            // Compute canvas bounding size from node positions
            let mut max_x = 920.0_f32;
            let mut max_y = 520.0_f32;
            for t in &db.tables {
                if let Some(&[x, y]) = app.erd_positions.get(&t.name) {
                    let h = header_h + (t.columns.len().max(1) as f32) * row_h + 10.0 * zoom;
                    max_x = max_x.max(x * zoom + card_w + 80.0);
                    max_y = max_y.max(y * zoom + h + 80.0);
                }
            }

            let (canvas_rect, _) = ui.allocate_exact_size(vec2(max_x, max_y), Sense::hover());
            let painter = ui.painter_at(canvas_rect);
            painter.rect_filled(canvas_rect, Rounding::same(8.0_f32), pal.bg_app);

            // Subtle dot grid
            let step = (28.0 * zoom).max(16.0);
            let mut gx = canvas_rect.min.x + step;
            while gx < canvas_rect.max.x {
                let mut gy = canvas_rect.min.y + step;
                while gy < canvas_rect.max.y {
                    painter.circle_filled(
                        pos2(gx, gy),
                        1.0_f32,
                        pal.border_subtle.gamma_multiply(0.55),
                    );
                    gy += step;
                }
                gx += step;
            }

            // Precompute screen rects per table
            let mut screen_rects: HashMap<&str, Rect> = HashMap::new();
            for t in &db.tables {
                let [lx, ly] = app.erd_positions.get(&t.name).copied().unwrap_or([24.0, 24.0]);
                let h = header_h + (t.columns.len().max(1) as f32) * row_h + 8.0 * zoom;
                let min = pos2(canvas_rect.min.x + lx * zoom, canvas_rect.min.y + ly * zoom);
                screen_rects.insert(t.name.as_str(), Rect::from_min_size(min, vec2(card_w, h)));
            }

            // 1. Draw Bezier Foreign Key Connectors behind cards
            for t in &db.tables {
                let Some(&child_rect) = screen_rects.get(t.name.as_str()) else { continue };
                for fk in &t.foreign_keys {
                    let Some(&parent_rect) = screen_rects.get(fk.referenced_table.as_str()) else {
                        continue;
                    };
                    let child_col_idx = t
                        .columns
                        .iter()
                        .position(|c| c.name == fk.column)
                        .unwrap_or(0);
                    let parent_col_idx = db
                        .tables
                        .iter()
                        .find(|pt| pt.name == fk.referenced_table)
                        .and_then(|pt| pt.columns.iter().position(|c| c.name == fk.referenced_column))
                        .unwrap_or(0);

                    let sy = child_rect.min.y + header_h + (child_col_idx as f32 + 0.5) * row_h;
                    let ty = parent_rect.min.y + header_h + (parent_col_idx as f32 + 0.5) * row_h;

                    let (p0, p3, c1, c2) = if child_rect.center().x >= parent_rect.center().x {
                        let s = pos2(child_rect.min.x, sy);
                        let e = pos2(parent_rect.max.x, ty);
                        let dx = ((s.x - e.x).abs() * 0.45).max(36.0 * zoom);
                        (s, e, pos2(s.x - dx, s.y), pos2(e.x + dx, e.y))
                    } else {
                        let s = pos2(child_rect.max.x, sy);
                        let e = pos2(parent_rect.min.x, ty);
                        let dx = ((e.x - s.x).abs() * 0.45).max(36.0 * zoom);
                        (s, e, pos2(s.x + dx, s.y), pos2(e.x - dx, e.y))
                    };

                    // Sample cubic Bezier into smooth polyline segments
                    let mut pts: Vec<Pos2> = Vec::with_capacity(21);
                    for step_i in 0..=20 {
                        let u = step_i as f32 / 20.0;
                        let inv = 1.0 - u;
                        let bx = inv * inv * inv * p0.x
                            + 3.0 * inv * inv * u * c1.x
                            + 3.0 * inv * u * u * c2.x
                            + u * u * u * p3.x;
                        let by = inv * inv * inv * p0.y
                            + 3.0 * inv * inv * u * c1.y
                            + 3.0 * inv * u * u * p3.y;
                        pts.push(pos2(bx, by));
                    }
                    painter.add(egui::Shape::line(
                        pts,
                        Stroke::new(1.8_f32, pal.accent_cyan.gamma_multiply(0.85)),
                    ));
                    painter.circle_filled(p0, 3.8 * zoom, pal.accent_amber);
                    painter.circle_filled(p3, 3.8 * zoom, pal.accent_emerald);
                }
            }

            // 2. Draw Interactive Draggable Table Cards
            let mut double_clicked_table: Option<String> = None;
            for t in &db.tables {
                let Some(&rect) = screen_rects.get(t.name.as_str()) else { continue };
                let id = ui.id().with(("erd_node", &t.name));
                let resp = ui.interact(rect, id, Sense::click_and_drag());

                if resp.dragged() {
                    let delta = resp.drag_delta() / zoom;
                    if let Some(pos) = app.erd_positions.get_mut(&t.name) {
                        pos[0] = (pos[0] + delta.x).max(6.0);
                        pos[1] = (pos[1] + delta.y).max(6.0);
                    }
                }
                if resp.double_clicked() {
                    double_clicked_table = Some(t.name.clone());
                } else if resp.clicked() {
                    app.grid_table_name = Some(t.name.clone());
                }
                let resp = resp.on_hover_text("Drag to rearrange • Click to inspect • Double-click to query rows");
                let _ = resp;

                let is_selected = app.grid_table_name.as_deref() == Some(t.name.as_str());
                let border_col = if is_selected {
                    pal.accent_cyan
                } else {
                    pal.border_subtle
                };

                painter.rect_filled(rect, Rounding::same(8.0_f32), pal.bg_card);
                let header_rect = Rect::from_min_size(rect.min, vec2(rect.width(), header_h));
                let hdr_fill = if t.object_type == "VIEW" {
                    pal.accent_violet.gamma_multiply(0.28)
                } else {
                    pal.accent_primary.gamma_multiply(0.26)
                };
                painter.rect_filled(header_rect, Rounding::same(8.0_f32), hdr_fill);
                painter.rect_stroke(rect, Rounding::same(8.0_f32), Stroke::new(1.4_f32, border_col));

                let prefix = if t.object_type == "VIEW" { "VIEW " } else { "" };
                painter.text(
                    pos2(header_rect.min.x + 10.0 * zoom, header_rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{prefix}{}", t.name),
                    FontId::monospace(12.0 * zoom),
                    pal.text_main,
                );
                painter.text(
                    pos2(header_rect.max.x - 8.0 * zoom, header_rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", t.row_count),
                    FontId::monospace(10.0 * zoom),
                    pal.text_dim,
                );

                for (ci, col) in t.columns.iter().enumerate() {
                    let y = rect.min.y + header_h + (ci as f32 + 0.55) * row_h;
                    let is_fk = t.foreign_keys.iter().any(|fk| fk.column == col.name);
                    let (tag, col_color) = if col.is_primary_key {
                        ("PK ", pal.accent_amber)
                    } else if is_fk {
                        ("FK ", pal.accent_cyan)
                    } else if col.is_unique {
                        ("UK ", pal.accent_emerald)
                    } else {
                        ("   ", pal.text_main)
                    };
                    painter.text(
                        pos2(rect.min.x + 8.0 * zoom, y),
                        egui::Align2::LEFT_CENTER,
                        format!("{tag}{}", col.name),
                        FontId::monospace(10.8 * zoom),
                        col_color,
                    );
                    let short_type: String = col.col_type.chars().take(12).collect();
                    painter.text(
                        pos2(rect.max.x - 8.0 * zoom, y),
                        egui::Align2::RIGHT_CENTER,
                        short_type,
                        FontId::monospace(9.8 * zoom),
                        pal.text_dim,
                    );
                }
            }

            if let Some(tbl) = double_clicked_table {
                app.select_and_query_table(&tbl);
            }
        });
}
