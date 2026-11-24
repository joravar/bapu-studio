use crate::app::BapuApp;
use bapu_core::types::{DatabaseConnection, DbKind};

fn blank_connection() -> DatabaseConnection {
    DatabaseConnection {
        id: uuid::Uuid::new_v4().to_string(),
        name: "New Connection".into(),
        kind: DbKind::Postgres,
        connection_string: None,
        host: Some("localhost".into()),
        port: Some("5432".into()),
        database: String::new(),
        username: Some("postgres".into()),
        password: None,
        ssl: false,
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
    ui.horizontal(|ui| {
        let current_name = app
            .databases
            .get(app.active_db_index)
            .map(|d| d.name.clone())
            .unwrap_or_else(|| "No connections yet".into());
        egui::ComboBox::from_id_salt("db_select")
            .selected_text(current_name)
            .show_ui(ui, |ui| {
                for (i, db) in app.databases.iter().enumerate() {
                    ui.selectable_value(&mut app.active_db_index, i, &db.name);
                }
            });
        if ui.button("+ New Connection").clicked() {
            app.databases.push(blank_connection());
            app.active_db_index = app.databases.len() - 1;
        }
        if ui.add_enabled(!app.db_busy && !app.databases.is_empty(), egui::Button::new("Test Connection")).clicked() {
            app.test_active_connection();
        }
    });

    if let Some(db) = app.databases.get_mut(app.active_db_index) {
        ui.collapsing("Connection details", |ui| {
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut db.name);
            });
            egui::ComboBox::from_id_salt("db_kind")
                .selected_text(format!("{:?}", db.kind))
                .show_ui(ui, |ui| {
                    for kind in [DbKind::Postgres, DbKind::Mysql, DbKind::Sqlite, DbKind::Mongodb, DbKind::Redis] {
                        ui.selectable_value(&mut db.kind, kind, format!("{kind:?}"));
                    }
                });
            ui.horizontal(|ui| {
                ui.label("Host");
                ui.text_edit_singleline(db.host.get_or_insert_with(|| "localhost".into()));
                ui.label("Port");
                ui.text_edit_singleline(db.port.get_or_insert_with(|| "5432".into()));
            });
            ui.horizontal(|ui| {
                ui.label("Database");
                ui.text_edit_singleline(&mut db.database);
            });
            ui.horizontal(|ui| {
                ui.label("Username");
                ui.text_edit_singleline(db.username.get_or_insert_with(String::new));
                ui.label("Password");
                ui.add(egui::TextEdit::singleline(db.password.get_or_insert_with(String::new)).password(true));
            });
            ui.checkbox(&mut db.ssl, "Require TLS");
        });
    }

    if let Some(status) = &app.db_status {
        ui.label(status);
    }

    ui.separator();
    ui.add(
        egui::TextEdit::multiline(&mut app.sql_text)
            .desired_rows(5)
            .code_editor()
            .desired_width(ui.available_width()),
    );
    if ui.add_enabled(!app.db_busy, egui::Button::new("▶ Run Query")).clicked() {
        app.run_active_query();
    }

    ui.separator();
    match &app.last_query_result {
        None => {
            ui.label(egui::RichText::new("No query run yet.").weak());
        }
        Some(Err(e)) => {
            ui.colored_label(egui::Color32::from_rgb(239, 68, 68), e);
        }
        Some(Ok(result)) => {
            ui.label(format!("{} rows • {} ms", result.row_count, result.execution_time_ms));
            egui::ScrollArea::both().max_height(400.0).show(ui, |ui| {
                egui_extras::TableBuilder::new(ui)
                    .striped(true)
                    .resizable(true)
                    .columns(egui_extras::Column::auto().at_least(80.0), result.columns.len().max(1))
                    .header(20.0, |mut header| {
                        for col in &result.columns {
                            header.col(|ui| {
                                ui.strong(col);
                            });
                        }
                    })
                    .body(|mut body| {
                        for row in &result.rows {
                            body.row(18.0, |mut table_row| {
                                for col in &result.columns {
                                    table_row.col(|ui| {
                                        let text = row
                                            .get(col)
                                            .map(|v| match v {
                                                serde_json::Value::String(s) => s.clone(),
                                                other => other.to_string(),
                                            })
                                            .unwrap_or_default();
                                        ui.label(text);
                                    });
                                }
                            });
                        }
                    });
            });
        }
    }
}
