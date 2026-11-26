//! DBeaver Community + PRO / Enterprise Floating Tool Windows:
//! 1. Visual Table Designer & Forward Engineering (`table_designer_open`)
//! 2. CSV / JSON Data Import Wizard with Column Mapping (`db_import_open`)
//! 3. Schema Compare & PK Table Data Compare (`db_compare_open`)
//! 4. Foreign-Key-Aware Smart Mock Data Generator (`db_mock_gen_open`)
//! 5. Server Health & Active Sessions Monitor (`db_admin_open`)

use crate::app::{BapuApp, CompareMode};
use crate::ui::widgets::{badge_pill, emerald_button, primary_button};
use bapu_core::db_ddl::generate_table_ddl;
use bapu_core::db_export_import::{export_query_result, parse_import_payload, ExportFormat};
use bapu_core::db_mock_gen::{generate_mock_mutations, mutations_to_preview_result};
use bapu_core::types::{ColumnDefinition, ForeignKeyDef, TableSchema};
use egui::{RichText, ScrollArea};
use std::collections::HashMap;

pub fn render_all(app: &mut BapuApp, ctx: &egui::Context) {
    if app.table_designer_open {
        render_table_designer(app, ctx);
    }
    if app.db_import_open {
        render_import_wizard(app, ctx);
    }
    if app.db_compare_open {
        render_schema_and_data_compare(app, ctx);
    }
    if app.db_mock_gen_open {
        render_mock_data_generator(app, ctx);
    }
    if app.db_admin_open {
        render_server_health_modal(app, ctx);
    }
}

fn render_table_designer(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.table_designer_open;
    let active_kind = app
        .databases
        .get(app.active_db_index)
        .map(|d| d.kind)
        .unwrap_or(bapu_core::types::DbKind::Postgres);
    let existing_tables: Vec<String> = app
        .databases
        .get(app.active_db_index)
        .map(|d| d.tables.iter().map(|t| t.name.clone()).collect())
        .unwrap_or_default();

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Visual Table Designer (Forward Engineering)",
        &mut open,
        [680.0, 540.0],
        [480.0, 320.0],
        "tbl_designer_win_scroll",
        |ui, viewport_h| {
            ui.horizontal(|ui| {
                ui.label("Table Name:");
                ui.add(
                    egui::TextEdit::singleline(&mut app.designer_table_name)
                        .font(egui::TextStyle::Monospace)
                        .desired_width(240.0),
                );
                if ui.small_button("+ Add Column").clicked() {
                    app.designer_columns.push(ColumnDefinition {
                        name: format!("col_{}", app.designer_columns.len() + 1),
                        col_type: "TEXT".into(),
                        is_nullable: true,
                        ..Default::default()
                    });
                }
                if ui.small_button("+ Add Foreign Key").clicked() {
                    let default_ref = existing_tables.first().cloned().unwrap_or_else(|| "users".into());
                    app.designer_fks.push(ForeignKeyDef {
                        constraint_name: None,
                        column: "user_id".into(),
                        referenced_table: default_ref,
                        referenced_column: "id".into(),
                        on_delete: Some("CASCADE".into()),
                        on_update: None,
                    });
                }
            });

            ui.separator();
            ui.label(RichText::new("COLUMNS").size(11.0).strong().color(pal.text_dim));

            let mut remove_col = None;
            let col_len = app.designer_columns.len();
            let cols_max_h = (viewport_h - 270.0).clamp(130.0, 480.0);
            ScrollArea::vertical().id_salt("designer_cols").max_height(cols_max_h).show(ui, |ui| {
                for (i, col) in app.designer_columns.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut col.name)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("name")
                                .desired_width(130.0),
                        );
                        egui::ComboBox::from_id_salt(("designer_type", i))
                            .selected_text(&col.col_type)
                            .width(120.0)
                            .show_ui(ui, |ui| {
                                for t in [
                                    "INTEGER",
                                    "BIGINT",
                                    "TEXT",
                                    "VARCHAR(255)",
                                    "BOOLEAN",
                                    "REAL",
                                    "TIMESTAMPTZ",
                                    "DATE",
                                    "JSONB",
                                ] {
                                    ui.selectable_value(&mut col.col_type, t.into(), t);
                                }
                            });
                        ui.checkbox(&mut col.is_primary_key, "PK");
                        ui.checkbox(&mut col.auto_increment, "Auto");
                        ui.checkbox(&mut col.is_nullable, "Null");
                        ui.checkbox(&mut col.is_unique, "Unique");
                        let def_str = col.default_value.get_or_insert_with(String::new);
                        ui.add(
                            egui::TextEdit::singleline(def_str)
                                .hint_text("default")
                                .desired_width(80.0),
                        );
                        if col.default_value.as_deref() == Some("") {
                            col.default_value = None;
                        }
                        if col_len > 1 && ui.small_button("×").clicked() {
                            remove_col = Some(i);
                        }
                    });
                }
            });
            if let Some(idx) = remove_col {
                app.designer_columns.remove(idx);
            }

            if !app.designer_fks.is_empty() {
                ui.add_space(4.0);
                ui.label(RichText::new("FOREIGN KEYS").size(11.0).strong().color(pal.text_dim));
                let mut remove_fk = None;
                for (fi, fk) in app.designer_fks.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label("Column");
                        ui.add(egui::TextEdit::singleline(&mut fk.column).desired_width(110.0));
                        ui.label("→ Table");
                        ui.add(
                            egui::TextEdit::singleline(&mut fk.referenced_table).desired_width(120.0),
                        );
                        ui.label(".");
                        ui.add(
                            egui::TextEdit::singleline(&mut fk.referenced_column).desired_width(80.0),
                        );
                        if ui.small_button("×").clicked() {
                            remove_fk = Some(fi);
                        }
                    });
                }
                if let Some(fi) = remove_fk {
                    app.designer_fks.remove(fi);
                }
            }

            let blueprint = TableSchema {
                name: app.designer_table_name.trim().to_string(),
                object_type: "TABLE".into(),
                row_count: 0,
                columns: app.designer_columns.clone(),
                foreign_keys: app.designer_fks.clone(),
                indexes: vec![],
                ddl: None,
            };
            let ddl = generate_table_ddl(&blueprint, active_kind, false);

            ui.separator();
            ui.label(RichText::new("GENERATED DDL PREVIEW").size(11.0).strong().color(pal.accent_cyan));
            let mut ddl_view = ddl.clone();
            ui.add(
                egui::TextEdit::multiline(&mut ddl_view)
                    .code_editor()
                    .desired_rows(6)
                    .interactive(false)
                    .desired_width(ui.available_width()),
            );

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if emerald_button(ui, "▶ Execute CREATE TABLE on Active DB", !app.db_busy).clicked() {
                    app.execute_ddl_and_refresh_schema(ddl.clone());
                }
                if ui.button("Copy DDL to Scratchpad").clicked() {
                    *app.active_sql_mut() = ddl;
                    app.table_designer_open = false;
                }
            });
        });

    if !open {
        app.table_designer_open = false;
    }
}

fn render_import_wizard(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.db_import_open;
    let table_names: Vec<String> = app
        .databases
        .get(app.active_db_index)
        .map(|d| d.tables.iter().map(|t| t.name.clone()).collect())
        .unwrap_or_default();

    if app.import_target_table.is_empty() {
        if let Some(first) = app.grid_table_name.clone().or_else(|| table_names.first().cloned()) {
            app.import_target_table = first;
        }
    }

    let target_cols: Vec<String> = app
        .databases
        .get(app.active_db_index)
        .and_then(|d| d.tables.iter().find(|t| t.name == app.import_target_table))
        .map(|t| t.columns.iter().map(|c| c.name.clone()).collect())
        .unwrap_or_default();

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Data Import Wizard (CSV & JSON → Database)",
        &mut open,
        [650.0, 500.0],
        [460.0, 300.0],
        "import_wiz_win_scroll",
        |ui, viewport_h| {
            ui.horizontal(|ui| {
                ui.label("Target Table:");
                egui::ComboBox::from_id_salt("import_target_tbl")
                    .selected_text(&app.import_target_table)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for t in &table_names {
                            ui.selectable_value(&mut app.import_target_table, t.clone(), t);
                        }
                    });
                if ui.button("Browse CSV / JSON File...").clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("Data Files", &["csv", "json"])
                        .pick_file()
                    {
                        if let Ok(raw) = std::fs::read_to_string(&path) {
                            app.import_raw_text = raw;
                            app.reparse_import_preview(&target_cols);
                        }
                    }
                }
            });

            ui.add_space(4.0);
            ui.label(
                RichText::new("Paste RFC-4180 CSV (with header row) or JSON array of objects `[ {...} ]`:")
                    .size(11.5)
                    .color(pal.text_muted),
            );
            if ui
                .add(
                    egui::TextEdit::multiline(&mut app.import_raw_text)
                        .code_editor()
                        .desired_rows(6)
                        .hint_text("name,email,plan\nAda Lovelace,ada@bapustudio.dev,Enterprise")
                        .desired_width(ui.available_width()),
                )
                .changed()
            {
                app.reparse_import_preview(&target_cols);
            }

            if !app.import_source_cols.is_empty() {
                ui.separator();
                ui.horizontal(|ui| {
                    badge_pill(
                        ui,
                        &format!("{} source columns", app.import_source_cols.len()),
                        pal.accent_cyan,
                    );
                    if let Ok((_, ref rows)) = parse_import_payload(&app.import_raw_text) {
                        badge_pill(
                            ui,
                            &format!("{} rows ready", rows.len()),
                            pal.accent_emerald,
                        );
                    }
                });

                ui.label(RichText::new("COLUMN MAPPING (Source → Target)").size(11.0).strong().color(pal.text_dim));
                let map_max_h = (viewport_h - 280.0).clamp(120.0, 440.0);
                ScrollArea::vertical().id_salt("import_col_map").max_height(map_max_h).show(ui, |ui| {
                    for src_col in &app.import_source_cols {
                        let mapped = app
                            .import_col_mapping
                            .entry(src_col.clone())
                            .or_insert_with(|| src_col.clone());
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(src_col).monospace().strong());
                            ui.label("→");
                            egui::ComboBox::from_id_salt(("imp_map", src_col))
                                .selected_text(mapped.as_str())
                                .width(180.0)
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(mapped, "(skip)".into(), "(skip column)");
                                    for tc in &target_cols {
                                        ui.selectable_value(mapped, tc.clone(), tc);
                                    }
                                    if !target_cols.contains(src_col) {
                                        ui.selectable_value(mapped, src_col.clone(), src_col);
                                    }
                                });
                        });
                    }
                });
            }

            if let Some(err) = &app.import_status {
                ui.colored_label(
                    if err.contains("error") || err.contains("failed") {
                        pal.accent_rose
                    } else {
                        pal.accent_emerald
                    },
                    err,
                );
            }

            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if emerald_button(ui, "▶ Import Rows (Transactional Batch)", !app.db_busy).clicked() {
                    app.run_data_import();
                }
            });
        });

    if !open {
        app.db_import_open = false;
    }
}

fn render_schema_and_data_compare(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.db_compare_open;
    let db_names: Vec<String> = app.databases.iter().map(|d| d.name.clone()).collect();

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Schema Compare & Table Data Compare",
        &mut open,
        [720.0, 520.0],
        [480.0, 300.0],
        "db_cmp_win_scroll",
        |ui, viewport_h| {
            let diff_sql_h = (viewport_h - 195.0).clamp(160.0, 680.0);
            ui.horizontal(|ui| {
                ui.selectable_value(&mut app.compare_mode, CompareMode::Schema, "Schema Compare & Migration DDL");
                ui.selectable_value(&mut app.compare_mode, CompareMode::Data, "Table Data Compare (PK Diff)");
            });
            ui.separator();

            if db_names.is_empty() {
                ui.label("Add at least one database connection first.");
                return;
            }
            app.compare_source_db_idx = app.compare_source_db_idx.min(db_names.len() - 1);
            app.compare_target_db_idx = app.compare_target_db_idx.min(db_names.len() - 1);

            ui.horizontal(|ui| {
                ui.label("Source (Desired):");
                egui::ComboBox::from_id_salt("cmp_src_db")
                    .selected_text(&db_names[app.compare_source_db_idx])
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (i, n) in db_names.iter().enumerate() {
                            ui.selectable_value(&mut app.compare_source_db_idx, i, n);
                        }
                    });
                ui.label("→ Target (Current):");
                egui::ComboBox::from_id_salt("cmp_tgt_db")
                    .selected_text(&db_names[app.compare_target_db_idx])
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for (i, n) in db_names.iter().enumerate() {
                            ui.selectable_value(&mut app.compare_target_db_idx, i, n);
                        }
                    });
            });

            match app.compare_mode {
                CompareMode::Schema => {
                    ui.add_space(4.0);
                    if primary_button(ui, "Compare Schemas & Generate Migration SQL", !app.db_busy).clicked() {
                        app.run_schema_compare();
                    }
                    if let Some(rep) = app.schema_diff_report.clone() {
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            badge_pill(
                                ui,
                                &format!("+{} new tables", rep.tables_only_in_source.len()),
                                pal.accent_emerald,
                            );
                            badge_pill(
                                ui,
                                &format!("~{} modified tables", rep.modified_tables.len()),
                                pal.accent_amber,
                            );
                            badge_pill(
                                ui,
                                &format!("-{} extra target tables", rep.tables_only_in_target.len()),
                                pal.accent_rose,
                            );
                            badge_pill(
                                ui,
                                &format!("={} identical", rep.identical_tables),
                                pal.accent_cyan,
                            );
                        });

                        let mut sql = rep.migration_sql.clone();
                        ui.add_space(4.0);
                        ScrollArea::vertical().id_salt("schema_diff_sql").max_height(diff_sql_h).show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut sql)
                                    .code_editor()
                                    .desired_rows(10)
                                    .interactive(false)
                                    .desired_width(ui.available_width()),
                            );
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Copy Migration SQL").clicked() {
                                ui.output_mut(|o| o.copied_text = rep.migration_sql.clone());
                            }
                            if ui.button("Open in SQL Scratchpad").clicked() {
                                *app.active_sql_mut() = rep.migration_sql.clone();
                                app.active_db_index = app.compare_target_db_idx;
                                app.db_compare_open = false;
                            }
                        });
                    }
                }
                CompareMode::Data => {
                    let src_tables: Vec<String> = app.databases[app.compare_source_db_idx]
                        .tables
                        .iter()
                        .map(|t| t.name.clone())
                        .collect();
                    if app.compare_table_name.is_empty() {
                        if let Some(first) = src_tables.first() {
                            app.compare_table_name = first.clone();
                        }
                    }
                    ui.horizontal(|ui| {
                        ui.label("Table to Compare:");
                        egui::ComboBox::from_id_salt("cmp_data_tbl")
                            .selected_text(&app.compare_table_name)
                            .width(200.0)
                            .show_ui(ui, |ui| {
                                for t in &src_tables {
                                    ui.selectable_value(&mut app.compare_table_name, t.clone(), t);
                                }
                            });
                        if primary_button(ui, "Compare Table Rows", !app.db_busy).clicked() {
                            app.run_data_compare();
                        }
                    });

                    if let Some(drep) = app.data_diff_report.clone() {
                        ui.add_space(6.0);
                        ui.horizontal_wrapped(|ui| {
                            badge_pill(
                                ui,
                                &format!("={} identical rows", drep.identical_rows),
                                pal.accent_cyan,
                            );
                            badge_pill(
                                ui,
                                &format!("+{} source only (INSERT)", drep.only_in_source),
                                pal.accent_emerald,
                            );
                            badge_pill(
                                ui,
                                &format!("~{} modified (UPDATE)", drep.modified_rows.len()),
                                pal.accent_amber,
                            );
                            badge_pill(
                                ui,
                                &format!("-{} target only (DELETE)", drep.only_in_target),
                                pal.accent_rose,
                            );
                        });
                        let mut ssql = drep.sync_sql.clone();
                        ScrollArea::vertical().id_salt("data_diff_sql").max_height(diff_sql_h).show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut ssql)
                                    .code_editor()
                                    .desired_rows(9)
                                    .interactive(false)
                                    .desired_width(ui.available_width()),
                            );
                        });
                        ui.horizontal(|ui| {
                            if ui.button("Copy Sync SQL").clicked() {
                                ui.output_mut(|o| o.copied_text = drep.sync_sql.clone());
                            }
                            if ui.button("Send Sync SQL to Scratchpad").clicked() {
                                *app.active_sql_mut() = drep.sync_sql.clone();
                                app.active_db_index = app.compare_target_db_idx;
                                app.db_compare_open = false;
                            }
                        });
                    }
                }
            }
        });

    if !open {
        app.db_compare_open = false;
    }
}

fn render_mock_data_generator(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.db_mock_gen_open;
    let Some(db) = app.databases.get(app.active_db_index).cloned() else {
        app.db_mock_gen_open = false;
        return;
    };
    let table_names: Vec<String> = db
        .tables
        .iter()
        .filter(|t| t.object_type != "VIEW")
        .map(|t| t.name.clone())
        .collect();
    if app.mock_gen_table.is_empty() {
        if let Some(first) = app.grid_table_name.clone().or_else(|| table_names.first().cloned()) {
            app.mock_gen_table = first;
        }
    }

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Smart Mock Data Generator (FK-Aware)",
        &mut open,
        [640.0, 230.0],
        [440.0, 175.0],
        "mock_gen_win_scroll",
        |ui, _viewport_h| {
            ui.horizontal(|ui| {
                ui.label("Target Table:");
                egui::ComboBox::from_id_salt("mock_gen_tbl")
                    .selected_text(&app.mock_gen_table)
                    .width(200.0)
                    .show_ui(ui, |ui| {
                        for t in &table_names {
                            ui.selectable_value(&mut app.mock_gen_table, t.clone(), t);
                        }
                    });
                ui.label("Row Count:");
                ui.add(egui::DragValue::new(&mut app.mock_gen_count).range(1..=500));
            });

            if let Some(tbl) = db.tables.iter().find(|t| t.name == app.mock_gen_table) {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!(
                        "Detected {} columns and {} Foreign Key constraint(s) (parent IDs fetched automatically).",
                        tbl.columns.len(),
                        tbl.foreign_keys.len()
                    ))
                    .size(11.5)
                    .color(pal.text_muted),
                );

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if emerald_button(
                        ui,
                        &format!("▶ Generate & Insert {} Rows into `{}`", app.mock_gen_count, tbl.name),
                        !app.db_busy,
                    )
                    .clicked()
                    {
                        app.run_mock_data_insert();
                    }
                    if ui.button("Copy as SQL INSERT Script").clicked() {
                        let muts = generate_mock_mutations(tbl, app.mock_gen_count, &HashMap::new(), false);
                        let preview_qr = mutations_to_preview_result(tbl, &muts);
                        let sql = export_query_result(&preview_qr, &tbl.name, db.kind, ExportFormat::SqlInsert);
                        ui.output_mut(|o| o.copied_text = sql);
                        app.db_status = Some("Copied generated mock SQL INSERTs to clipboard.".into());
                    }
                });
            }
        },
    );

    if !open {
        app.db_mock_gen_open = false;
    }
}

fn render_server_health_modal(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.db_admin_open;

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Server Health & Active Sessions Monitor",
        &mut open,
        [680.0, 460.0],
        [460.0, 260.0],
        "srv_health_win_scroll",
        |ui, viewport_h| {
            ui.horizontal(|ui| {
                if primary_button(ui, "Refresh Diagnostics", !app.db_busy).clicked() {
                    app.refresh_server_health();
                }
            });
            ui.separator();

            match &app.server_health {
                None => {
                    ui.label("Click 'Refresh Diagnostics' to query engine health & active sessions.");
                }
                Some(Err(e)) => {
                    ui.colored_label(pal.accent_rose, format!("Health check failed: {e}"));
                }
                Some(Ok(rep)) => {
                    ui.label(RichText::new(&rep.engine_banner).strong().color(pal.accent_cyan));
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        for m in &rep.metrics {
                            badge_pill(ui, &format!("{}: {}", m.label, m.value), pal.accent_emerald);
                        }
                    });
                    if let Some(ref sess) = rep.active_sessions {
                        ui.add_space(8.0);
                        ui.label(RichText::new("ACTIVE SESSIONS / DIAGNOSTICS").size(11.0).strong().color(pal.text_dim));
                        let mut json = serde_json::to_string_pretty(&sess.rows).unwrap_or_default();
                        let sess_max_h = (viewport_h - 160.0).clamp(150.0, 680.0);
                        ScrollArea::vertical().id_salt("srv_sess").max_height(sess_max_h).show(ui, |ui| {
                            ui.add(
                                egui::TextEdit::multiline(&mut json)
                                    .code_editor()
                                    .interactive(false)
                                    .desired_width(ui.available_width()),
                            );
                        });
                    }
                }
            }
        },
    );

    if !open {
        app.db_admin_open = false;
    }
}
