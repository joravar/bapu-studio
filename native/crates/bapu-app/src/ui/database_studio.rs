//! DBeaver-grade Multi-Engine Database Studio UI (`egui`).
//! Includes Connection Manager (5 engines + mTLS + SSH Bastion), Power SQL Editor
//! (Formatter, Multi-Statement Script Runner, `:param` Binding, Templates & Saved Snippets),
//! Editable Transactional Data Grid (Inline Filter, Column Filter, Click-Header Sort, Pagination,
//! Outgoing & Incoming Foreign Key Navigation, 8-Format Data Export), Record & Stats Inspector,
//! Table Structure & Cross-Dialect DDL Viewer, and Interactive ER Diagram (ERD).

use crate::app::{BapuApp, DbSubView};
use crate::ui::widgets::{
    badge_pill, card_frame, compact_card_frame, db_accent, db_brand_name, db_logo, emerald_button,
    paint_icon, primary_button, subtab_button, VectorIcon,
};
use bapu_core::db_ddl::{generate_table_ddl, quote_dialect_ident};
use bapu_core::db_export_import::{
    export_query_result, filter_and_sort_row_indices, ExportFormat, GridFilterOp,
};
use bapu_core::db_sql_tools::{
    builtin_sql_templates, compute_column_stats, extract_sql_params, format_sql,
};
use bapu_core::types::{
    DatabaseConnection, DbCategory, DbKind, ForeignKeyDef, SqlScriptTab, SqlSnippet,
};
use egui::{vec2, Color32, Frame, Margin, RichText, Rounding, Sense, Stroke};
use std::collections::HashMap;

pub fn connection_for_kind(kind: DbKind) -> DatabaseConnection {
    let port = kind.default_port();
    let db = kind.default_database();
    let user = kind.default_user();
    DatabaseConnection {
        id: uuid::Uuid::new_v4().to_string(),
        name: format!("{} Local", kind.display_name()),
        kind,
        connection_string: None,
        host: if port.is_empty() {
            None
        } else {
            Some("localhost".into())
        },
        port: if port.is_empty() {
            None
        } else {
            Some(port.into())
        },
        database: db.into(),
        username: if user.is_empty() {
            None
        } else {
            Some(user.into())
        },
        password: None,
        ssl: matches!(
            kind,
            DbKind::AzureSql | DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks
        ),
        ssl_ca_cert: None,
        ssl_client_cert: None,
        ssl_client_key: None,
        ssl_reject_unauthorized: None,
        ssh_enabled: false,
        ssh_host: None,
        ssh_port: None,
        ssh_username: None,
        ssh_password: None,
        ssh_private_key: None,
        ssh_passphrase: None,
        is_connected: false,
        is_demo_db: false,
        tables: vec![],
    }
}

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();

    // Empty state hero with 5 clickable Database Engine Logo Cards
    if app.databases.is_empty() {
        ui.add_space(28.0);
        card_frame(&pal).show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                paint_icon(ui, VectorIcon::Database, 36.0, pal.accent_cyan);
                ui.add_space(8.0);
                ui.label(
                    RichText::new("Universal 33-Engine Database IDE")
                        .size(18.0)
                        .strong()
                        .color(pal.text_main),
                );
                ui.label(
                    RichText::new(
                        "PostgreSQL, SQL Server (MSSQL), Oracle, MySQL, MariaDB, SQLite, ClickHouse, Snowflake, BigQuery, Databricks, Cassandra, MongoDB, Redis, Neo4j, Elasticsearch, Db2, SAP HANA & ODBC.",
                    )
                    .size(12.2)
                    .color(pal.text_muted),
                );
                ui.add_space(14.0);
            });

            let mut chosen_kind = None;
            for row_slice in DbKind::FEATURED.chunks(5) {
                ui.columns(5, |cols| {
                    for (col_ui, &kind) in cols.iter_mut().zip(row_slice.iter()) {
                        let accent = db_accent(kind);
                        let tile = Frame::none()
                            .fill(pal.bg_app)
                            .stroke(Stroke::new(1.0_f32, accent.gamma_multiply(0.45)))
                            .rounding(Rounding::same(10.0_f32))
                            .inner_margin(Margin::symmetric(8.0_f32, 11.0_f32))
                            .show(col_ui, |ui| {
                                ui.vertical_centered(|ui| {
                                    ui.add(
                                        egui::Image::new(db_logo(kind))
                                            .fit_to_exact_size(vec2(34.0, 34.0))
                                            .rounding(Rounding::same(7.0_f32)),
                                    );
                                    ui.add_space(5.0);
                                    ui.label(
                                        RichText::new(db_brand_name(kind))
                                            .size(12.0)
                                            .strong()
                                            .color(pal.text_main),
                                    );
                                    ui.label(RichText::new("+ Connect").size(10.5).color(accent));
                                });
                            });
                        if tile.response.interact(Sense::click()).clicked() {
                            chosen_kind = Some(kind);
                        }
                    }
                });
                ui.add_space(6.0);
            }

            if let Some(k) = chosen_kind {
                app.databases.push(connection_for_kind(k));
                app.active_db_index = app.databases.len() - 1;
                app.db_config_open = true;
                app.persist();
            }

            ui.add_space(10.0);
            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    if primary_button(ui, "Browse All 33 Database Engines...", true).clicked() {
                        app.databases.push(connection_for_kind(DbKind::Postgres));
                        app.active_db_index = app.databases.len() - 1;
                        app.db_config_open = true;
                        app.persist();
                    }
                    if emerald_button(
                        ui,
                        "Launch Instant SQLite Playground (Users, Orders, FKs & Views)",
                        !app.db_busy,
                    )
                    .clicked()
                    {
                        app.create_sqlite_playground();
                    }
                });
            });
            ui.add_space(8.0);
        });
        return;
    }

    render_connection_modal(app, ui);

    // Active Connection Header Banner Card + DBeaver Power Tool Strip
    let mut open_cfg = false;
    let mut delete_active_db = false;
    let mut do_test = false;
    let mut do_schema = false;
    let mut open_health = false;
    if let Some(db) = app.databases.get(app.active_db_index) {
        compact_card_frame(&pal).show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.add(
                    egui::Image::new(db_logo(db.kind))
                        .fit_to_exact_size(vec2(30.0, 30.0))
                        .rounding(Rounding::same(7.0_f32)),
                );
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 2.0;
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&db.name)
                                .size(14.0)
                                .strong()
                                .color(pal.text_main),
                        );
                        badge_pill(ui, db_brand_name(db.kind), db_accent(db.kind));
                        if db.is_connected {
                            badge_pill(ui, "CONNECTED", pal.accent_emerald);
                        }
                        if db.ssl {
                            badge_pill(ui, "TLS", pal.accent_cyan);
                        }
                        if db.ssh_enabled {
                            badge_pill(ui, "SSH TUNNEL", pal.accent_violet);
                        }
                    });
                    let target_desc = if db.kind == DbKind::Sqlite {
                        db.database.clone()
                    } else {
                        format!(
                            "{}:{}/{}",
                            db.host.as_deref().unwrap_or("localhost"),
                            db.port.as_deref().unwrap_or("default"),
                            db.database
                        )
                    };
                    ui.label(
                        RichText::new(target_desc)
                            .monospace()
                            .size(10.8)
                            .color(pal.text_dim),
                    );
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new("Delete").color(pal.accent_rose))
                        .on_hover_text("Delete this database connection")
                        .clicked()
                    {
                        delete_active_db = true;
                    }
                    if ui.button("Configure").clicked() {
                        open_cfg = true;
                    }
                    if ui
                        .add_enabled(!app.db_busy, egui::Button::new("Refresh Schema"))
                        .clicked()
                    {
                        do_schema = true;
                    }
                    if ui
                        .add_enabled(!app.db_busy, egui::Button::new("Test"))
                        .clicked()
                    {
                        do_test = true;
                    }
                    ui.separator();
                    if ui
                        .small_button("Health")
                        .on_hover_text("Server Health & Active Sessions")
                        .clicked()
                    {
                        open_health = true;
                    }
                    if ui
                        .small_button("Mock Data")
                        .on_hover_text("FK-Aware Smart Mock Data Generator")
                        .clicked()
                    {
                        app.db_mock_gen_open = true;
                    }
                    if ui
                        .small_button("Diff & Sync")
                        .on_hover_text("Schema Compare & PK Table Data Compare")
                        .clicked()
                    {
                        app.db_compare_open = true;
                    }
                    if ui
                        .small_button("Import")
                        .on_hover_text("Import CSV or JSON with Column Mapping")
                        .clicked()
                    {
                        app.db_import_open = true;
                    }
                    if ui
                        .small_button("+ Table")
                        .on_hover_text("Visual Table Designer & Forward Engineering")
                        .clicked()
                    {
                        app.table_designer_open = true;
                    }
                });
            });
        });
    }
    if delete_active_db {
        app.delete_database_at(app.active_db_index);
        return;
    }
    if open_cfg {
        app.db_config_open = true;
    }
    if do_test {
        app.test_active_connection();
    }
    if do_schema {
        app.load_active_schema();
    }
    if open_health {
        app.db_admin_open = true;
        app.refresh_server_health();
    }

    if let Some(status) = &app.db_status {
        ui.add_space(4.0);
        ui.label(
            RichText::new(status)
                .size(12.0)
                .color(
                    if status.contains("failed")
                        || status.contains("rolled back")
                        || status.contains("Error")
                    {
                        pal.accent_rose
                    } else {
                        pal.accent_emerald
                    },
                ),
        );
    }

    ui.add_space(6.0);

    // SQL Power Editor Card (Multi-Tab + Templates/Snippets + `:param` Binder + Script Runner)
    let active_dialect = app
        .databases
        .get(app.active_db_index)
        .map(|d| d.kind)
        .unwrap_or(DbKind::Postgres);

    card_frame(&pal).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            let mut close_idx = None;
            for (i, tab) in app.sql_tabs.iter().enumerate() {
                let selected = i == app.active_sql_tab;
                if subtab_button(ui, &pal, selected, &tab.name).clicked() {
                    app.active_sql_tab = i;
                }
                if app.sql_tabs.len() > 1
                    && ui
                        .add(
                            egui::Button::new(RichText::new("×").size(11.5).color(pal.text_dim))
                                .frame(false),
                        )
                        .clicked()
                {
                    close_idx = Some(i);
                }
            }
            if let Some(i) = close_idx {
                app.sql_tabs.remove(i);
                if app.active_sql_tab >= app.sql_tabs.len() {
                    app.active_sql_tab = app.sql_tabs.len().saturating_sub(1);
                }
            }
            if ui.small_button("+ Tab").clicked() {
                let next_num = app.sql_tabs.len() + 1;
                app.sql_tabs.push(SqlScriptTab {
                    id: uuid::Uuid::new_v4().to_string(),
                    database_id: String::new(),
                    name: format!("Query {next_num}.sql"),
                    query: String::new(),
                });
                app.active_sql_tab = app.sql_tabs.len() - 1;
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("★ Save Snippet")
                    .on_hover_text("Save current query to SQL Bookmarks")
                    .clicked()
                {
                    let sql = app.active_sql().trim().to_string();
                    if !sql.is_empty() {
                        let name: String = sql.lines().next().unwrap_or("Snippet").chars().take(32).collect();
                        app.sql_snippets.push(SqlSnippet {
                            id: uuid::Uuid::new_v4().to_string(),
                            name,
                            dialect: Some(active_dialect),
                            sql,
                        });
                        let _ = app.storage.save_sql_snippets(&app.sql_snippets);
                        app.db_status = Some("Saved query to SQL Snippets.".into());
                    }
                }

                let mut chosen_template: Option<String> = None;
                egui::ComboBox::from_id_salt("sql_templates_combo")
                    .selected_text("Templates / Snippets")
                    .width(160.0)
                    .show_ui(ui, |ui| {
                        ui.label(RichText::new("Built-in Templates").size(10.5).color(pal.text_dim));
                        for (title, body) in builtin_sql_templates(active_dialect) {
                            if ui.selectable_label(false, title).clicked() {
                                chosen_template = Some(body.to_string());
                            }
                        }
                        if !app.sql_snippets.is_empty() {
                            ui.separator();
                            ui.label(RichText::new("Saved Snippets").size(10.5).color(pal.text_dim));
                            for snip in &app.sql_snippets {
                                if ui.selectable_label(false, format!("★ {}", snip.name)).clicked() {
                                    chosen_template = Some(snip.sql.clone());
                                }
                            }
                        }
                    });
                if let Some(tpl) = chosen_template {
                    *app.active_sql_mut() = tpl;
                }

                if ui
                    .small_button("Format SQL")
                    .on_hover_text("Beautify SQL keywords & clauses")
                    .clicked()
                {
                    let formatted = format_sql(app.active_sql());
                    *app.active_sql_mut() = formatted;
                }
            });
        });

        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::multiline(app.active_sql_mut())
                .code_editor()
                .desired_rows(4)
                .hint_text("SELECT * FROM users WHERE plan = :plan LIMIT 25;   -- Ctrl+Enter: Run | Alt+X: Run Script")
                .desired_width(ui.available_width()),
        );

        // Dynamic `:param` / `${param}` Parameter Bar (just like DBeaver!)
        let detected_params = extract_sql_params(app.active_sql());
        if !detected_params.is_empty() {
            ui.add_space(4.0);
            ui.horizontal_wrapped(|ui| {
                badge_pill(ui, "PARAMS", pal.accent_amber);
                for p in detected_params {
                    ui.label(RichText::new(format!(":{p} =")).monospace().size(11.5).color(pal.accent_cyan));
                    let val = app.sql_param_values.entry(p).or_default();
                    ui.add(
                        egui::TextEdit::singleline(val)
                            .font(egui::TextStyle::Monospace)
                            .hint_text("value")
                            .desired_width(110.0),
                    );
                }
            });
        }

        ui.add_space(5.0);
        ui.horizontal_wrapped(|ui| {
            if emerald_button(
                ui,
                if app.db_busy { "Running..." } else { "▶ Run Query" },
                !app.db_busy,
            )
            .on_hover_text("Execute active query (Ctrl+Enter or F5)")
            .clicked()
            {
                app.run_active_query();
            }

            if ui
                .add_enabled(
                    !app.db_busy,
                    egui::Button::new(RichText::new("▶▶ Run Script").strong().color(pal.accent_emerald)),
                )
                .on_hover_text("Execute all semicolon-separated statements sequentially (Alt+X)")
                .clicked()
            {
                app.run_sql_script();
            }

            if ui
                .add_enabled(
                    !app.db_busy,
                    egui::Button::new(RichText::new("Explain Plan").strong().color(pal.accent_cyan)),
                )
                .clicked()
            {
                app.explain_active_query();
            }

            ui.separator();
            for (sv, label) in [
                (DbSubView::Grid, "Data Grid"),
                (DbSubView::RecordStats, "Record & Stats"),
                (DbSubView::StructureDdl, "Structure & DDL"),
                (DbSubView::Erd, "ER Diagram"),
                (DbSubView::ConsoleLog, "Console & Log"),
            ] {
                if subtab_button(ui, &pal, app.db_subview == sv, label).clicked() {
                    app.db_subview = sv;
                }
            }
        });
    });

    // Pending Transactional Edits Banner
    let pending_count = app.pending_grid_change_count();
    if pending_count > 0 {
        ui.add_space(6.0);
        Frame::none()
            .fill(pal.accent_amber.gamma_multiply(0.14))
            .stroke(Stroke::new(1.2_f32, pal.accent_amber.gamma_multiply(0.55)))
            .rounding(Rounding::same(8.0_f32))
            .inner_margin(Margin::symmetric(12.0_f32, 8.0_f32))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    badge_pill(ui, "STAGED TX", pal.accent_amber);
                    ui.label(
                        RichText::new(format!(
                            "{pending_count} pending change(s) ready to commit as one transaction (Ctrl+S)"
                        ))
                        .strong()
                        .color(pal.text_main),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Revert").clicked() {
                            app.revert_grid_changes();
                        }
                        if emerald_button(ui, "Save Changes (COMMIT)", !app.db_busy).clicked() {
                            app.commit_grid_changes();
                        }
                    });
                });
            });
    }

    ui.add_space(6.0);

    let avail_h = ui.available_height().max(210.0);
    card_frame(&pal).show(ui, |ui| {
        ui.set_min_height(avail_h - 24.0);
        match app.db_subview {
            DbSubView::Grid => render_data_grid_view(app, ui, avail_h, active_dialect),
            DbSubView::RecordStats => render_record_and_stats_view(app, ui, avail_h),
            DbSubView::StructureDdl => render_structure_and_ddl_view(app, ui, avail_h, active_dialect),
            DbSubView::Erd => crate::ui::db_erd::render_erd(app, ui, avail_h),
            DbSubView::ConsoleLog => render_console_and_script_log(app, ui, avail_h),
        }
    });
}

fn render_data_grid_view(
    app: &mut BapuApp,
    ui: &mut egui::Ui,
    avail_h: f32,
    active_dialect: DbKind,
) {
    let pal = app.palette();
    let can_edit = app.grid_table_name.is_some()
        && app
            .databases
            .get(app.active_db_index)
            .is_some_and(|d| d.kind != DbKind::Redis);

    // Gather outgoing & incoming FKs for the current table (for hyperlink navigation!)
    let mut outgoing_fks: HashMap<String, ForeignKeyDef> = HashMap::new();
    let mut incoming_fks: Vec<(String, String, String)> = Vec::new(); // (child_table, child_col, parent_pk_col)
    if let (Some(db), Some(cur_tbl)) = (
        app.databases.get(app.active_db_index),
        app.grid_table_name.as_deref(),
    ) {
        if let Some(t) = db.tables.iter().find(|t| t.name == cur_tbl) {
            for fk in &t.foreign_keys {
                outgoing_fks.insert(fk.column.clone(), fk.clone());
            }
        }
        for other in &db.tables {
            for fk in &other.foreign_keys {
                if fk.referenced_table == cur_tbl {
                    incoming_fks.push((
                        other.name.clone(),
                        fk.column.clone(),
                        fk.referenced_column.clone(),
                    ));
                }
            }
        }
    }

    let mut add_new_row_cols: Option<Vec<String>> = None;
    let mut toggle_delete_row: Option<usize> = None;
    let mut remove_staged_insert: Option<usize> = None;
    let mut fk_jump_query: Option<(String, String)> = None; // (target_table, sql)
    let mut paginate_delta: Option<i32> = None;

    match &app.last_query_result {
        None => {
            ui.vertical_centered(|ui| {
                ui.add_space((avail_h * 0.22).max(24.0));
                paint_icon(ui, VectorIcon::Play, 28.0, pal.text_dim);
                ui.add_space(6.0);
                ui.label(
                    RichText::new("Run a query or click a table in the Schema Explorer")
                        .size(13.0)
                        .color(pal.text_muted),
                );
            });
        }
        Some(Err(e)) => {
            ui.colored_label(pal.accent_rose, format!("Query Error: {e}"));
        }
        Some(Ok(result)) => {
            let result = result.clone();

            // Row 1: Badges + Pagination + Multi-Format Export + Add Row
            ui.horizontal_wrapped(|ui| {
                badge_pill(ui, &format!("{} rows", result.row_count), pal.accent_emerald);
                badge_pill(
                    ui,
                    &format!("{} ms", result.execution_time_ms),
                    pal.accent_cyan,
                );
                if let Some(cmd) = &result.command {
                    badge_pill(ui, cmd, pal.accent_primary);
                }
                if let Some(tbl) = &app.grid_table_name {
                    badge_pill(ui, &format!("Table: {tbl}"), pal.accent_amber);
                    if active_dialect != DbKind::Mongodb && active_dialect != DbKind::Redis {
                        if ui
                            .add_enabled(app.grid_page > 0, egui::Button::new("◀ Prev").small())
                            .clicked()
                        {
                            paginate_delta = Some(-1);
                        }
                        ui.label(
                            RichText::new(format!("Page {}", app.grid_page + 1))
                                .monospace()
                                .size(11.0)
                                .color(pal.text_dim),
                        );
                        if ui.small_button("Next ▶").clicked() {
                            paginate_delta = Some(1);
                        }
                    }
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let tbl_label = app
                        .grid_table_name
                        .as_deref()
                        .unwrap_or("query_result")
                        .to_string();
                    if ui.small_button("Save File...").clicked() {
                        let content = export_query_result(
                            &result,
                            &tbl_label,
                            active_dialect,
                            app.db_export_format,
                        );
                        let ext = app.db_export_format.extension();
                        if let Some(path) = rfd::FileDialog::new()
                            .set_file_name(format!("{tbl_label}.{ext}"))
                            .add_filter(app.db_export_format.label(), &[ext])
                            .save_file()
                        {
                            let _ = std::fs::write(&path, content);
                            app.db_status = Some(format!("Exported to {}", path.display()));
                        }
                    }
                    if ui.small_button("Copy").clicked() {
                        let content = export_query_result(
                            &result,
                            &tbl_label,
                            active_dialect,
                            app.db_export_format,
                        );
                        ui.output_mut(|o| o.copied_text = content);
                        app.db_status = Some(format!(
                            "Copied {} to clipboard.",
                            app.db_export_format.label()
                        ));
                    }
                    egui::ComboBox::from_id_salt("db_export_fmt")
                        .selected_text(app.db_export_format.label())
                        .width(135.0)
                        .show_ui(ui, |ui| {
                            for fmt in ExportFormat::ALL {
                                ui.selectable_value(&mut app.db_export_format, fmt, fmt.label());
                            }
                        });
                    if can_edit && !result.columns.is_empty() && ui.small_button("+ Add Row").clicked()
                    {
                        add_new_row_cols = Some(result.columns.clone());
                    }
                });
            });

            // Row 2: DBeaver Inline Quick Filter + Column Filter Bar + Incoming FK Relations
            if !result.columns.is_empty() {
                ui.add_space(3.0);
                ui.horizontal_wrapped(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut app.grid_quick_filter)
                            .hint_text("Quick search all columns...")
                            .desired_width(175.0),
                    );
                    egui::ComboBox::from_id_salt("grid_filter_col")
                        .selected_text(if app.grid_filter_col.is_empty() {
                            "Column filter..."
                        } else {
                            &app.grid_filter_col
                        })
                        .width(120.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut app.grid_filter_col, String::new(), "(none)");
                            for c in &result.columns {
                                ui.selectable_value(&mut app.grid_filter_col, c.clone(), c);
                            }
                        });
                    if !app.grid_filter_col.is_empty() {
                        egui::ComboBox::from_id_salt("grid_filter_op")
                            .selected_text(app.grid_filter_op.label())
                            .width(90.0)
                            .show_ui(ui, |ui| {
                                for op in GridFilterOp::ALL {
                                    ui.selectable_value(&mut app.grid_filter_op, op, op.label());
                                }
                            });
                        if !matches!(
                            app.grid_filter_op,
                            GridFilterOp::IsNull | GridFilterOp::IsNotNull
                        ) {
                            ui.add(
                                egui::TextEdit::singleline(&mut app.grid_filter_val)
                                    .hint_text("value...")
                                    .desired_width(110.0),
                            );
                        }
                    }
                    if (!app.grid_quick_filter.is_empty()
                        || !app.grid_filter_col.is_empty()
                        || app.grid_sort_col.is_some())
                        && ui.small_button("Clear Filter/Sort").clicked()
                    {
                        app.grid_quick_filter.clear();
                        app.grid_filter_col.clear();
                        app.grid_filter_val.clear();
                        app.grid_sort_col = None;
                    }

                    // Reverse Incoming FK Navigation for selected row!
                    if let Some(sel_row) = result.rows.get(app.selected_grid_row) {
                        for (child_tbl, child_col, parent_pk) in &incoming_fks {
                            if let Some(pk_val) = sel_row.get(parent_pk) {
                                let pk_lit = match pk_val {
                                    serde_json::Value::Number(n) => n.to_string(),
                                    serde_json::Value::String(s) => {
                                        format!("'{}'", s.replace('\'', "''"))
                                    }
                                    other => other.to_string(),
                                };
                                if ui
                                    .small_button(
                                        RichText::new(format!("↘ {child_tbl} ({child_col}={pk_lit})"))
                                            .color(pal.accent_cyan),
                                    )
                                    .on_hover_text("Navigate to referencing child rows")
                                    .clicked()
                                {
                                    let qt = quote_dialect_ident(child_tbl, active_dialect);
                                    let qc = quote_dialect_ident(child_col, active_dialect);
                                    fk_jump_query = Some((
                                        child_tbl.clone(),
                                        format!("SELECT * FROM {qt} WHERE {qc} = {pk_lit} LIMIT 50;"),
                                    ));
                                }
                            }
                        }
                    }
                });
            }

            if let Some(plan) = &result.query_plan {
                ui.add_space(4.0);
                ui.collapsing("Execution Plan Output", |ui| {
                    let mut p = plan.clone();
                    ui.add(
                        egui::TextEdit::multiline(&mut p)
                            .code_editor()
                            .interactive(false)
                            .desired_width(ui.available_width()),
                    );
                });
            }

            ui.separator();

            let visible_indices = filter_and_sort_row_indices(
                &result,
                &app.grid_quick_filter,
                Some(app.grid_filter_col.as_str()),
                app.grid_filter_op,
                &app.grid_filter_val,
                app.grid_sort_col.as_deref(),
                app.grid_sort_asc,
            );

            let col_count = result.columns.len().max(1);
            let total_cols = col_count + 1; // Row # / Action column + data columns
            egui::ScrollArea::both()
                .max_height((avail_h - 110.0).max(120.0))
                .show(ui, |ui| {
                    egui_extras::TableBuilder::new(ui)
                        .striped(true)
                        .resizable(true)
                        .cell_layout(egui::Layout::left_to_right(egui::Align::Center))
                        .columns(egui_extras::Column::auto().at_least(105.0), total_cols)
                        .header(26.0, |mut header| {
                            header.col(|ui| {
                                ui.label(
                                    RichText::new("# / ROW")
                                        .size(10.5)
                                        .strong()
                                        .color(pal.text_dim),
                                );
                            });
                            for col in &result.columns {
                                header.col(|ui| {
                                    let sort_arrow = if app.grid_sort_col.as_deref() == Some(col.as_str()) {
                                        if app.grid_sort_asc { " ▲" } else { " ▼" }
                                    } else {
                                        ""
                                    };
                                    let is_fk = outgoing_fks.contains_key(col);
                                    let hdr_text = if is_fk {
                                        format!("↗ {col}{sort_arrow}")
                                    } else {
                                        format!("{col}{sort_arrow}")
                                    };
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new(hdr_text)
                                                    .monospace()
                                                    .size(11.6)
                                                    .strong()
                                                    .color(pal.accent_cyan),
                                            )
                                            .frame(false),
                                        )
                                        .on_hover_text("Click to sort ASC / DESC")
                                        .clicked()
                                    {
                                        if app.grid_sort_col.as_deref() == Some(col.as_str()) {
                                            app.grid_sort_asc = !app.grid_sort_asc;
                                        } else {
                                            app.grid_sort_col = Some(col.clone());
                                            app.grid_sort_asc = true;
                                        }
                                    }
                                });
                            }
                        })
                        .body(|mut body| {
                            for &row_idx in &visible_indices {
                                let row = &result.rows[row_idx];
                                let marked_delete = app.grid_staged_deletes.contains(&row_idx);
                                let is_sel = app.selected_grid_row == row_idx;
                                body.row(26.0, |mut table_row| {
                                    table_row.col(|ui| {
                                        if ui
                                            .selectable_label(
                                                is_sel,
                                                RichText::new(format!("#{}", row_idx + 1))
                                                    .monospace()
                                                    .size(10.8),
                                            )
                                            .on_hover_text("Select row for Record Inspector & child FK links")
                                            .clicked()
                                        {
                                            app.selected_grid_row = row_idx;
                                        }
                                        if can_edit {
                                            let label = if marked_delete { "Undo" } else { "Del" };
                                            if ui.small_button(label).clicked() {
                                                toggle_delete_row = Some(row_idx);
                                            }
                                        }
                                    });
                                    for col in &result.columns {
                                        table_row.col(|ui| {
                                            let cell_val = row.get(col);
                                            let orig_text = cell_val
                                                .map(|v| match v {
                                                    serde_json::Value::String(s) => s.clone(),
                                                    serde_json::Value::Null => "NULL".into(),
                                                    other => other.to_string(),
                                                })
                                                .unwrap_or_default();

                                            if can_edit && !marked_delete {
                                                let key = (row_idx, col.clone());
                                                let is_edited =
                                                    app.grid_pending_edits.contains_key(&key);
                                                let mut val = app
                                                    .grid_pending_edits
                                                    .get(&key)
                                                    .cloned()
                                                    .unwrap_or_else(|| orig_text.clone());
                                                let mut edit = egui::TextEdit::singleline(&mut val)
                                                    .font(egui::TextStyle::Monospace)
                                                    .desired_width(120.0);
                                                if is_edited {
                                                    edit = edit.text_color(pal.accent_amber);
                                                }
                                                if ui.add(edit).changed() {
                                                    if val == orig_text {
                                                        app.grid_pending_edits.remove(&key);
                                                    } else {
                                                        app.grid_pending_edits.insert(key, val);
                                                    }
                                                }
                                            } else if marked_delete {
                                                ui.label(
                                                    RichText::new(&orig_text)
                                                        .monospace()
                                                        .strikethrough()
                                                        .color(pal.accent_rose),
                                                );
                                            } else {
                                                ui.label(
                                                    RichText::new(&orig_text)
                                                        .monospace()
                                                        .color(Color32::from_rgb(226, 232, 240)),
                                                );
                                            }

                                            // Outgoing Foreign Key Hyperlink Jump (`↗`)
                                            if let Some(fk) = outgoing_fks.get(col) {
                                                if !matches!(
                                                    cell_val,
                                                    None | Some(serde_json::Value::Null)
                                                ) && ui
                                                    .small_button(
                                                        RichText::new("↗")
                                                            .size(10.5)
                                                            .color(pal.accent_cyan),
                                                    )
                                                    .on_hover_text(format!(
                                                        "Jump to {}.{} = {}",
                                                        fk.referenced_table,
                                                        fk.referenced_column,
                                                        orig_text
                                                    ))
                                                    .clicked()
                                                {
                                                    let qt = quote_dialect_ident(
                                                        &fk.referenced_table,
                                                        active_dialect,
                                                    );
                                                    let qc = quote_dialect_ident(
                                                        &fk.referenced_column,
                                                        active_dialect,
                                                    );
                                                    let lit = match cell_val {
                                                        Some(serde_json::Value::Number(n)) => {
                                                            n.to_string()
                                                        }
                                                        _ => format!(
                                                            "'{}'",
                                                            orig_text.replace('\'', "''")
                                                        ),
                                                    };
                                                    fk_jump_query = Some((
                                                        fk.referenced_table.clone(),
                                                        format!(
                                                            "SELECT * FROM {qt} WHERE {qc} = {lit};"
                                                        ),
                                                    ));
                                                }
                                            }
                                        });
                                    }
                                });
                            }

                            for (ins_idx, new_row) in
                                app.grid_staged_inserts.iter_mut().enumerate()
                            {
                                body.row(26.0, |mut table_row| {
                                    table_row.col(|ui| {
                                        if ui.small_button("× New").clicked() {
                                            remove_staged_insert = Some(ins_idx);
                                        }
                                    });
                                    for col in &result.columns {
                                        table_row.col(|ui| {
                                            let entry = new_row.entry(col.clone()).or_default();
                                            ui.add(
                                                egui::TextEdit::singleline(entry)
                                                    .font(egui::TextStyle::Monospace)
                                                    .hint_text(col)
                                                    .text_color(pal.accent_emerald)
                                                    .desired_width(120.0),
                                            );
                                        });
                                    }
                                });
                            }
                        });
                });
        }
    }

    if let Some(cols) = add_new_row_cols {
        let mut map = HashMap::new();
        for c in cols {
            map.insert(c, String::new());
        }
        app.grid_staged_inserts.push(map);
    }
    if let Some(row_idx) = toggle_delete_row {
        if !app.grid_staged_deletes.remove(&row_idx) {
            app.grid_staged_deletes.insert(row_idx);
        }
    }
    if let Some(ins_idx) = remove_staged_insert {
        if ins_idx < app.grid_staged_inserts.len() {
            app.grid_staged_inserts.remove(ins_idx);
        }
    }
    if let Some((target_tbl, sql)) = fk_jump_query {
        *app.active_sql_mut() = sql;
        app.grid_table_name = Some(target_tbl);
        app.grid_page = 0;
        app.run_active_query();
    }
    if let Some(delta) = paginate_delta {
        if let Some(tbl) = app.grid_table_name.clone() {
            app.grid_page = (app.grid_page as i32 + delta).max(0) as usize;
            let offset = app.grid_page * app.grid_page_size;
            let qt = quote_dialect_ident(&tbl, active_dialect);
            *app.active_sql_mut() = format!(
                "SELECT * FROM {qt} LIMIT {} OFFSET {offset};",
                app.grid_page_size
            );
            app.run_active_query();
        }
    }
}

fn render_record_and_stats_view(app: &mut BapuApp, ui: &mut egui::Ui, avail_h: f32) {
    let pal = app.palette();
    let Some(Ok(ref result)) = app.last_query_result else {
        ui.label("Run a query first to inspect individual records and column statistics.");
        return;
    };
    let stats = compute_column_stats(result);

    ui.columns(2, |cols| {
        // Left: DBeaver Single-Row Record Value Inspector
        cols[0].label(
            RichText::new("SINGLE-ROW RECORD INSPECTOR")
                .size(11.0)
                .strong()
                .color(pal.accent_cyan),
        );
        if result.rows.is_empty() {
            cols[0].label("Result set has 0 rows.");
        } else {
            let idx = app.selected_grid_row.min(result.rows.len() - 1);
            cols[0].horizontal(|ui| {
                if ui
                    .add_enabled(idx > 0, egui::Button::new("◀ Prev Row").small())
                    .clicked()
                {
                    app.selected_grid_row = idx.saturating_sub(1);
                }
                ui.label(
                    RichText::new(format!("Row #{} of {}", idx + 1, result.rows.len()))
                        .monospace()
                        .strong(),
                );
                if ui
                    .add_enabled(
                        idx + 1 < result.rows.len(),
                        egui::Button::new("Next Row ▶").small(),
                    )
                    .clicked()
                {
                    app.selected_grid_row = idx + 1;
                }
            });
            let row = &result.rows[idx];
            egui::ScrollArea::vertical()
                .id_salt("record_inspector_scroll")
                .max_height((avail_h - 75.0).max(140.0))
                .show(&mut cols[0], |ui| {
                    for col in &result.columns {
                        let v_str = match row.get(col) {
                            None | Some(serde_json::Value::Null) => "NULL".into(),
                            Some(serde_json::Value::String(s)) => s.clone(),
                            Some(other) => {
                                serde_json::to_string_pretty(other).unwrap_or_else(|_| other.to_string())
                            }
                        };
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{col}:"))
                                    .monospace()
                                    .strong()
                                    .color(pal.accent_amber),
                            );
                            ui.label(RichText::new(v_str).monospace().color(pal.text_main));
                        });
                    }
                });
        }

        // Right: Column Statistics Summary
        cols[1].label(
            RichText::new("COLUMN STATISTICS (COUNT / DISTINCT / MIN / MAX / AVG)")
                .size(11.0)
                .strong()
                .color(pal.accent_emerald),
        );
        egui::ScrollArea::vertical()
            .id_salt("col_stats_scroll")
            .max_height((avail_h - 55.0).max(140.0))
            .show(&mut cols[1], |ui| {
                for st in &stats {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&st.column).monospace().strong().color(pal.text_main));
                        badge_pill(
                            ui,
                            &format!("{} distinct", st.distinct_count),
                            pal.accent_cyan,
                        );
                        if st.null_count > 0 {
                            badge_pill(ui, &format!("{} NULL", st.null_count), pal.accent_rose);
                        }
                        if let Some(avg) = st.avg {
                            badge_pill(ui, &format!("avg {avg:.2}"), pal.accent_emerald);
                        }
                        if let (Some(mn), Some(mx)) = (&st.min, &st.max) {
                            ui.label(
                                RichText::new(format!("[{mn} .. {mx}]"))
                                    .monospace()
                                    .size(10.5)
                                    .color(pal.text_dim),
                            );
                        }
                    });
                }
            });
    });
}

fn render_structure_and_ddl_view(
    app: &mut BapuApp,
    ui: &mut egui::Ui,
    avail_h: f32,
    active_dialect: DbKind,
) {
    let pal = app.palette();
    let Some(db) = app.databases.get(app.active_db_index).cloned() else {
        return;
    };
    let selected_tbl = app
        .grid_table_name
        .clone()
        .or_else(|| db.tables.first().map(|t| t.name.clone()));
    let Some(tname) = selected_tbl else {
        ui.label("Click 'Refresh Schema' and select a table to inspect columns, FKs, indexes, and DDL.");
        return;
    };
    let Some(tbl) = db.tables.iter().find(|t| t.name == tname) else {
        ui.label(format!("Table `{tname}` not found in loaded schema."));
        return;
    };

    ui.horizontal_wrapped(|ui| {
        badge_pill(ui, &format!("{} {}", tbl.object_type, tbl.name), pal.accent_amber);
        badge_pill(ui, &format!("{} columns", tbl.columns.len()), pal.accent_cyan);
        badge_pill(ui, &format!("{} FKs", tbl.foreign_keys.len()), pal.accent_violet);
        badge_pill(ui, &format!("{} indexes", tbl.indexes.len()), pal.accent_emerald);

        ui.separator();
        ui.label("DDL Dialect:");
        for d in [
            DbKind::Postgres,
            DbKind::Mysql,
            DbKind::Sqlite,
            DbKind::Mongodb,
        ] {
            ui.selectable_value(&mut app.ddl_preview_dialect, Some(d), db_brand_name(d));
        }
        ui.selectable_value(&mut app.ddl_preview_dialect, None, "Native DB");
    });

    let target_dialect = app.ddl_preview_dialect.unwrap_or(active_dialect);
    let prefer_native = app.ddl_preview_dialect.is_none();
    let ddl = generate_table_ddl(tbl, target_dialect, prefer_native);

    ui.separator();
    egui::ScrollArea::vertical()
        .max_height((avail_h - 68.0).max(150.0))
        .show(ui, |ui| {
            ui.columns(2, |cols| {
                cols[0].label(RichText::new("COLUMNS, FOREIGN KEYS & INDEXES").size(11.0).strong().color(pal.text_dim));
                for c in &tbl.columns {
                    cols[0].horizontal_wrapped(|ui| {
                        ui.label(RichText::new(&c.name).monospace().strong().color(pal.text_main));
                        ui.label(RichText::new(&c.col_type).monospace().color(pal.accent_cyan));
                        if c.is_primary_key {
                            badge_pill(ui, "PK", pal.accent_amber);
                        }
                        if c.auto_increment {
                            badge_pill(ui, "AUTO", pal.accent_violet);
                        }
                        if c.is_unique {
                            badge_pill(ui, "UNIQUE", pal.accent_emerald);
                        }
                        if !c.is_nullable {
                            badge_pill(ui, "NOT NULL", pal.text_dim);
                        }
                    });
                }
                if !tbl.foreign_keys.is_empty() {
                    cols[0].add_space(6.0);
                    cols[0].label(RichText::new("FOREIGN KEYS").size(11.0).strong().color(pal.accent_cyan));
                    for fk in &tbl.foreign_keys {
                        cols[0].label(
                            RichText::new(format!(
                                "• {} → {}.{}",
                                fk.column, fk.referenced_table, fk.referenced_column
                            ))
                            .monospace()
                            .size(11.5),
                        );
                    }
                }
                if !tbl.indexes.is_empty() {
                    cols[0].add_space(6.0);
                    cols[0].label(RichText::new("INDEXES").size(11.0).strong().color(pal.accent_emerald));
                    for idx in &tbl.indexes {
                        cols[0].label(
                            RichText::new(format!(
                                "• {} ({}) {}",
                                idx.name,
                                idx.columns.join(", "),
                                if idx.is_unique { "[UNIQUE]" } else { "" }
                            ))
                            .monospace()
                            .size(11.5),
                        );
                    }
                }

                cols[1].horizontal(|ui| {
                    ui.label(RichText::new("GENERATED DDL").size(11.0).strong().color(pal.accent_cyan));
                    if ui.small_button("Copy DDL").clicked() {
                        ui.output_mut(|o| o.copied_text = ddl.clone());
                    }
                    if ui.small_button("Open in Scratchpad").clicked() {
                        *app.active_sql_mut() = ddl.clone();
                    }
                });
                let mut ddl_box = ddl;
                cols[1].add(
                    egui::TextEdit::multiline(&mut ddl_box)
                        .code_editor()
                        .desired_rows(12)
                        .interactive(false)
                        .desired_width(cols[1].available_width()),
                );
            });
        });
}

fn render_console_and_script_log(app: &mut BapuApp, ui: &mut egui::Ui, avail_h: f32) {
    let pal = app.palette();
    egui::ScrollArea::vertical()
        .max_height((avail_h - 48.0).max(140.0))
        .show(ui, |ui| {
            if let Some(ref rep) = app.last_script_report {
                ui.horizontal_wrapped(|ui| {
                    badge_pill(
                        ui,
                        &format!("{}/{} statements succeeded", rep.succeeded, rep.total_statements),
                        if rep.failed == 0 {
                            pal.accent_emerald
                        } else {
                            pal.accent_rose
                        },
                    );
                    badge_pill(
                        ui,
                        &format!("{} total rows", rep.total_rows_affected),
                        pal.accent_cyan,
                    );
                    badge_pill(
                        ui,
                        &format!("{} ms", rep.total_elapsed_ms),
                        pal.accent_primary,
                    );
                });
                for item in &rep.items {
                    ui.horizontal_wrapped(|ui| {
                        let col = if item.error.is_some() {
                            pal.accent_rose
                        } else {
                            pal.accent_emerald
                        };
                        ui.colored_label(col, format!("#{} ({}ms):", item.index, item.elapsed_ms));
                        ui.label(RichText::new(&item.sql_preview).monospace().size(11.2));
                        if let Some(ref err) = item.error {
                            ui.colored_label(pal.accent_rose, format!("Error: {err}"));
                        }
                    });
                }
                ui.separator();
            }

            if let Some(Ok(ref res)) = app.last_query_result {
                let mut dump = serde_json::to_string_pretty(&res.rows).unwrap_or_default();
                ui.add(
                    egui::TextEdit::multiline(&mut dump)
                        .code_editor()
                        .interactive(false)
                        .desired_width(ui.available_width()),
                );
            } else {
                ui.label("No JSON result output yet.");
            }
        });
}

fn render_connection_modal(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();
    if !app.db_config_open {
        return;
    }
    let mut open = app.db_config_open;
    let mut engine_filter = app.db_engine_picker_filter.clone();
    let mut selected_cat = app.db_engine_picker_cat;

    crate::ui::widgets::show_resizable_window(
        ui.ctx(),
        "Universal Database Connection Configuration (33 Engines)",
        &mut open,
        [680.0, 620.0],
        [480.0, 320.0],
        "db_conn_modal_scroll",
        |ui, viewport_h| {
            // Category filter tabs + search box
            ui.horizontal_wrapped(|ui| {
                ui.selectable_value(&mut selected_cat, None, "All (33)");
                for cat in DbCategory::ALL {
                    ui.selectable_value(&mut selected_cat, Some(cat), cat.label());
                }
            });
            ui.add_space(4.0);
            ui.add(
                egui::TextEdit::singleline(&mut engine_filter)
                    .hint_text("Search 33 engines (e.g. MSSQL, Oracle, Postgres, Snowflake, ClickHouse, Cassandra, Db2, ODBC)...")
                    .desired_width(ui.available_width()),
            );
            ui.add_space(6.0);

            if let Some(db) = app.databases.get_mut(app.active_db_index) {
                let q = engine_filter.trim().to_ascii_lowercase();
                let matching: Vec<DbKind> = DbKind::ALL
                    .into_iter()
                    .filter(|k| {
                        selected_cat.is_none_or(|c| k.category() == c)
                            && (q.is_empty()
                                || k.display_name().to_ascii_lowercase().contains(&q)
                                || k.short_badge().to_ascii_lowercase().contains(&q))
                    })
                    .collect();

                let form_reserve = if db.kind == DbKind::Sqlite { 175.0 } else { 320.0 };
                let picker_max_h = (viewport_h - form_reserve).clamp(135.0, 520.0);
                let n_cols = if ui.available_width() < 520.0 {
                    3
                } else if ui.available_width() > 860.0 {
                    5
                } else {
                    4
                };

                let mut switched_to: Option<DbKind> = None;
                egui::ScrollArea::vertical()
                    .id_salt("engine_picker_grid")
                    .max_height(picker_max_h)
                    .show(ui, |ui| {
                        for row_chunk in matching.chunks(n_cols) {
                            ui.columns(n_cols, |cols| {
                                for (c_ui, &kind) in cols.iter_mut().zip(row_chunk.iter()) {
                                    let selected = db.kind == kind;
                                    let stroke = if selected {
                                        Stroke::new(1.6_f32, db_accent(kind))
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
                                        .rounding(Rounding::same(7.0_f32))
                                        .inner_margin(Margin::symmetric(7.0_f32, 6.0_f32))
                                        .show(c_ui, |ui| {
                                            ui.horizontal(|ui| {
                                                ui.add(
                                                    egui::Image::new(db_logo(kind))
                                                        .fit_to_exact_size(vec2(20.0, 20.0))
                                                        .rounding(Rounding::same(4.0_f32)),
                                                );
                                                ui.vertical(|ui| {
                                                    ui.spacing_mut().item_spacing.y = 0.0;
                                                    ui.label(
                                                        RichText::new(db_brand_name(kind))
                                                            .size(11.0)
                                                            .strong()
                                                            .color(pal.text_main),
                                                    );
                                                    let p = kind.default_port();
                                                    ui.label(
                                                        RichText::new(if p.is_empty() {
                                                            kind.short_badge()
                                                        } else {
                                                            p
                                                        })
                                                        .monospace()
                                                        .size(9.5)
                                                        .color(pal.text_dim),
                                                    );
                                                });
                                            });
                                        });
                                    if tile.response.interact(Sense::click()).clicked() {
                                        switched_to = Some(kind);
                                    }
                                }
                            });
                            ui.add_space(3.0);
                        }
                    });

                if let Some(new_kind) = switched_to {
                    db.kind = new_kind;
                    db.name = format!("{} Local", new_kind.display_name());
                    let p = new_kind.default_port();
                    db.port = if p.is_empty() { None } else { Some(p.into()) };
                    db.host = if p.is_empty() {
                        None
                    } else {
                        Some("localhost".into())
                    };
                    db.database = new_kind.default_database().into();
                    let u = new_kind.default_user();
                    db.username = if u.is_empty() { None } else { Some(u.into()) };
                }

                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    badge_pill(ui, db.kind.category().label(), db_accent(db.kind));
                    ui.label("Connection Name:");
                    ui.add(
                        egui::TextEdit::singleline(&mut db.name)
                            .desired_width(ui.available_width()),
                    );
                });

                ui.separator();
                if db.kind == DbKind::Sqlite {
                    ui.label("SQLite Database File Path (or `:memory:`):");
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut db.database)
                                .font(egui::TextStyle::Monospace)
                                .hint_text("C:/path/to/database.sqlite or :memory:")
                                .desired_width((ui.available_width() - 95.0).max(160.0)),
                        );
                        if ui.button("Browse...").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("SQLite Database", &["sqlite", "sqlite3", "db"])
                                .pick_file()
                            {
                                db.database = path.to_string_lossy().into_owned();
                            }
                        }
                    });
                } else {
                    ui.label(
                        RichText::new("Connection URI / ADO / EZConnect / ODBC String (optional — overrides host/port):")
                            .size(11.5)
                            .color(pal.text_muted),
                    );
                    ui.add(
                        egui::TextEdit::singleline(
                            db.connection_string.get_or_insert_with(String::new),
                        )
                        .font(egui::TextStyle::Monospace)
                        .hint_text(db.kind.uri_hint())
                        .desired_width(ui.available_width()),
                    );

                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        let row_w = ui.available_width();
                        let host_w = (row_w - 170.0).max(140.0);
                        ui.label("Host");
                        ui.add(
                            egui::TextEdit::singleline(
                                db.host.get_or_insert_with(|| "localhost".into()),
                            )
                            .desired_width(host_w),
                        );
                        ui.label("Port");
                        ui.add(
                            egui::TextEdit::singleline(
                                db.port
                                    .get_or_insert_with(|| db.kind.default_port().into()),
                            )
                            .desired_width(85.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        let db_lbl = match db.kind {
                            DbKind::Oracle => "Service / SID",
                            DbKind::Cassandra | DbKind::Scylladb => "Keyspace",
                            DbKind::Bigquery => "Project.Dataset",
                            DbKind::Odbc => "DSN",
                            _ => "Database",
                        };
                        let user_lbl = match db.kind {
                            DbKind::Databricks => "Warehouse ID",
                            DbKind::Snowflake => "Warehouse",
                            _ => "User",
                        };
                        let avail = (ui.available_width() - 180.0).max(200.0);
                        ui.label(db_lbl);
                        ui.add(
                            egui::TextEdit::singleline(&mut db.database)
                                .desired_width(avail * 0.54),
                        );
                        ui.label(user_lbl);
                        ui.add(
                            egui::TextEdit::singleline(db.username.get_or_insert_with(String::new))
                                .desired_width(ui.available_width()),
                        );
                    });
                    ui.horizontal(|ui| {
                        let pw_lbl = match db.kind {
                            DbKind::Snowflake | DbKind::Bigquery | DbKind::Databricks | DbKind::Libsql => {
                                "Bearer / PAT Token (OS Keychain)"
                            }
                            _ => "Password (OS Keychain)",
                        };
                        ui.label(pw_lbl);
                        ui.add(
                            egui::TextEdit::singleline(db.password.get_or_insert_with(String::new))
                                .password(true)
                                .desired_width(ui.available_width()),
                        );
                    });

                    ui.add_space(6.0);
                    ui.checkbox(&mut db.ssl, "Require TLS / Custom Root CA / Mutual TLS (mTLS)");
                    if db.ssl {
                        ui.indent("tls_cfg", |ui| {
                            let mut verify = db.ssl_reject_unauthorized.unwrap_or(true);
                            if ui
                                .checkbox(
                                    &mut verify,
                                    "Verify server certificate authority (recommended)",
                                )
                                .changed()
                            {
                                db.ssl_reject_unauthorized = Some(verify);
                            }
                            ui.label(RichText::new("Root CA PEM:").size(11.5));
                            ui.add(
                                egui::TextEdit::multiline(
                                    db.ssl_ca_cert.get_or_insert_with(String::new),
                                )
                                .code_editor()
                                .desired_rows(2)
                                .desired_width(ui.available_width()),
                            );
                            ui.label(RichText::new("Client Cert PEM (mTLS):").size(11.5));
                            ui.add(
                                egui::TextEdit::multiline(
                                    db.ssl_client_cert.get_or_insert_with(String::new),
                                )
                                .code_editor()
                                .desired_rows(2)
                                .desired_width(ui.available_width()),
                            );
                            ui.label(RichText::new("Client Private Key PEM (mTLS):").size(11.5));
                            ui.add(
                                egui::TextEdit::multiline(
                                    db.ssl_client_key.get_or_insert_with(String::new),
                                )
                                .code_editor()
                                .desired_rows(2)
                                .desired_width(ui.available_width()),
                            );
                        });
                    }

                    ui.checkbox(&mut db.ssh_enabled, "Connect through SSH Bastion / Jump Host");
                    if db.ssh_enabled {
                        ui.indent("ssh_cfg", |ui| {
                            ui.horizontal(|ui| {
                                ui.label("Bastion Host");
                                ui.text_edit_singleline(
                                    db.ssh_host.get_or_insert_with(String::new),
                                );
                                ui.label("Port");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        db.ssh_port.get_or_insert_with(|| "22".into()),
                                    )
                                    .desired_width(60.0),
                                );
                            });
                            ui.horizontal(|ui| {
                                ui.label("SSH User");
                                ui.text_edit_singleline(
                                    db.ssh_username.get_or_insert_with(String::new),
                                );
                                ui.label("Password");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        db.ssh_password.get_or_insert_with(String::new),
                                    )
                                    .password(true),
                                );
                            });
                            ui.label(RichText::new("SSH Private Key PEM (optional):").size(11.5));
                            ui.add(
                                egui::TextEdit::multiline(
                                    db.ssh_private_key.get_or_insert_with(String::new),
                                )
                                .code_editor()
                                .desired_rows(3)
                                .desired_width(ui.available_width()),
                            );
                        });
                    }
                }
            }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if primary_button(ui, "Save & Test Connection", !app.db_busy).clicked() {
                    app.persist();
                    app.test_active_connection();
                }
                if ui.button("Close").clicked() {
                    app.persist();
                    app.db_config_open = false;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(RichText::new("Delete Connection").color(pal.accent_rose))
                        .clicked()
                    {
                        app.delete_database_at(app.active_db_index);
                    }
                });
            });
        });

    app.db_engine_picker_filter = engine_filter;
    app.db_engine_picker_cat = selected_cat;
    if !open {
        app.db_config_open = false;
    }
}
