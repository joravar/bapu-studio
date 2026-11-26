use crate::app::{ApiReqSubTab, ApiRespSubTab, BapuApp};
use crate::ui::widgets::{
    badge_pill, card_frame, compact_card_frame, emerald_button, method_color, openapi_logo,
    paint_icon, postman_logo, primary_button, status_pill, subtab_button, VectorIcon,
};
use bapu_core::types::{
    ApiExample, AuthType, BodyType, HttpMethod, KeyValuePair, MultipartField,
};
use egui::{vec2, Frame, Margin, RichText, Rounding, Stroke};

const GRAPHQL_INTROSPECTION_QUERY: &str = r#"query IntrospectionQuery {
  __schema {
    queryType { name }
    mutationType { name }
    types {
      kind
      name
      description
    }
  }
}"#;

pub fn render(app: &mut BapuApp, ui: &mut egui::Ui) {
    let pal = app.palette();

    // Multi-Format Import / Export & API Documentation Window
    if app.io_modal_open {
        let mut open = app.io_modal_open;
        crate::ui::widgets::show_resizable_window(
            ui.ctx(),
            "Import / Export  —  OpenAPI 3.0, HAR 1.2, Postman v2.1 & Markdown Docs",
            &mut open,
            [680.0, 520.0],
            [460.0, 300.0],
            "api_io_win_scroll",
            |ui, _viewport_h| {
                ui.horizontal_wrapped(|ui| {
                    ui.add(egui::Image::new(openapi_logo()).fit_to_exact_size(vec2(18.0, 18.0)));
                    ui.strong("OpenAPI 3.0 / 3.1");
                    ui.separator();
                    ui.add(egui::Image::new(postman_logo()).fit_to_exact_size(vec2(18.0, 18.0)));
                    ui.strong("Postman v2.1 (Compatible)");
                    ui.separator();
                    badge_pill(ui, "HAR 1.2", pal.accent_cyan);
                    badge_pill(ui, "Markdown Docs", pal.accent_emerald);
                });

                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                    ui.label(RichText::new("Export Collection:").color(pal.text_muted));
                    for col in &app.collections {
                        if ui.small_button(format!("OpenAPI: {}", col.name)).clicked() {
                            if let Ok(j) = bapu_core::collection_io::export_to_openapi(col) {
                                app.io_modal_text = j;
                                app.io_modal_status =
                                    Some(format!("Exported '{}' as OpenAPI 3.0.3 JSON.", col.name));
                            }
                        }
                        if ui.small_button(format!("Postman: {}", col.name)).clicked() {
                            if let Ok(j) = bapu_core::collection_io::export_to_postman(col) {
                                app.io_modal_text = j;
                                app.io_modal_status =
                                    Some(format!("Exported '{}' as Postman v2.1 JSON.", col.name));
                            }
                        }
                        if ui.small_button(format!("Docs (.md): {}", col.name)).clicked() {
                            app.io_modal_text =
                                bapu_core::collection_io::export_to_markdown_docs(col);
                            app.io_modal_status = Some(format!(
                                "Generated Markdown API Docs for '{}'.",
                                col.name
                            ));
                        }
                    }
                    if let Some(env) = app.environments.get(app.active_env_index) {
                        if ui.small_button(format!("Export Env: {}", env.name)).clicked() {
                            if let Ok(j) = bapu_core::collection_io::export_postman_environment(env)
                            {
                                app.io_modal_text = j;
                                app.io_modal_status =
                                    Some(format!("Exported environment '{}'.", env.name));
                            }
                        }
                    }
                });

                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    if ui.small_button("Load File from Disk...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("API Specs & Archives", &["json", "har"])
                            .pick_file()
                        {
                            if let Ok(raw) = std::fs::read_to_string(path) {
                                app.io_modal_text = raw;
                            }
                        }
                    }
                    if ui.small_button("Copy Output").clicked() {
                        ui.output_mut(|o| o.copied_text = app.io_modal_text.clone());
                        app.io_modal_status = Some("Copied to clipboard.".into());
                    }
                });

                ui.add(
                    egui::TextEdit::multiline(&mut app.io_modal_text)
                        .code_editor()
                        .desired_rows(11)
                        .hint_text("Paste OpenAPI 3.0 JSON, HAR 1.2 JSON, Postman Collection v2.1 JSON, or Environment JSON here...")
                        .desired_width(ui.available_width()),
                );

                let has_txt = !app.io_modal_text.trim().is_empty();
                ui.horizontal_wrapped(|ui| {
                    if primary_button(ui, "Import OpenAPI 3.0", has_txt).clicked() {
                        match bapu_core::collection_io::import_from_openapi(&app.io_modal_text) {
                            Ok(col) => {
                                let msg = format!(
                                    "Imported OpenAPI '{}' ({} endpoints).",
                                    col.name,
                                    col.requests.len()
                                );
                                app.collections.push(col);
                                app.persist();
                                app.io_modal_status = Some(msg);
                            }
                            Err(e) => app.io_modal_status = Some(format!("OpenAPI error: {e}")),
                        }
                    }
                    if ui.add_enabled(has_txt, egui::Button::new("Import Postman v2.1")).clicked()
                    {
                        match bapu_core::collection_io::import_from_postman(&app.io_modal_text) {
                            Ok(col) => {
                                let msg = format!(
                                    "Imported '{}' ({} requests).",
                                    col.name,
                                    col.requests.len()
                                );
                                app.collections.push(col);
                                app.persist();
                                app.io_modal_status = Some(msg);
                            }
                            Err(e) => app.io_modal_status = Some(format!("Postman error: {e}")),
                        }
                    }
                    if ui.add_enabled(has_txt, egui::Button::new("Import HAR 1.2")).clicked() {
                        match bapu_core::collection_io::import_from_har(&app.io_modal_text) {
                            Ok(col) => {
                                let msg = format!(
                                    "Imported HAR ({} captured requests).",
                                    col.requests.len()
                                );
                                app.collections.push(col);
                                app.persist();
                                app.io_modal_status = Some(msg);
                            }
                            Err(e) => app.io_modal_status = Some(format!("HAR error: {e}")),
                        }
                    }
                    if ui.add_enabled(has_txt, egui::Button::new("Import Environment")).clicked() {
                        match bapu_core::collection_io::import_postman_environment(
                            &app.io_modal_text,
                        ) {
                            Ok(env) => {
                                let msg = format!("Imported environment '{}'.", env.name);
                                app.environments.push(env);
                                app.active_env_index = app.environments.len() - 1;
                                app.persist();
                                app.io_modal_status = Some(msg);
                            }
                            Err(e) => app.io_modal_status = Some(format!("Env error: {e}")),
                        }
                    }
                });

                if let Some(status) = &app.io_modal_status {
                    ui.colored_label(pal.accent_emerald, status);
                }
            });
        app.io_modal_open = open;
    }

    // Top Request Bar Card
    compact_card_frame(&pal).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut app.active_request.name)
                    .hint_text("Request title")
                    .desired_width(145.0),
            );

            let m_color = method_color(app.active_request.method);
            egui::ComboBox::from_id_salt("api_method_select")
                .selected_text(
                    RichText::new(app.active_request.method.as_str())
                        .monospace()
                        .strong()
                        .color(m_color),
                )
                .width(92.0)
                .show_ui(ui, |ui| {
                    for m in HttpMethod::ALL {
                        ui.selectable_value(
                            &mut app.active_request.method,
                            m,
                            RichText::new(m.as_str())
                                .monospace()
                                .strong()
                                .color(method_color(m)),
                        );
                    }
                });

            let saved_loc = app.collections.iter().enumerate().find_map(|(ci, c)| {
                c.requests
                    .iter()
                    .position(|r| r.id == app.active_request.id)
                    .map(|ri| (ci, ri))
            });
            let reserve = if saved_loc.is_some() { 415.0 } else { 355.0 };
            let url_w = (ui.available_width() - reserve).max(140.0);
            let url_resp = ui.add(
                egui::TextEdit::singleline(&mut app.active_request.url)
                    .font(egui::TextStyle::Monospace)
                    .hint_text(
                        "https://api.example.com/v1/users/:id  ({{VAR}} or {{$guid}} / {{$randomEmail}})",
                    )
                    .desired_width(url_w),
            );
            if url_resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                app.send_active_request();
            }

            let can_send = !app.api_busy && !app.active_request.url.trim().is_empty();
            if primary_button(
                ui,
                if app.api_busy { "Sending..." } else { "Send  ▶" },
                can_send,
            )
            .on_hover_text("Execute HTTP Request (Ctrl+Enter)")
            .clicked()
            {
                app.send_active_request();
            }

            if ui
                .button("</> Code")
                .on_hover_text("Generate client code in 8 languages")
                .clicked()
            {
                app.codegen_open = !app.codegen_open;
            }
            if ui
                .button(format!("Cookies ({})", app.cookie_jar.entries.len()))
                .on_hover_text("Manage domain cookies")
                .clicked()
            {
                app.cookie_modal_open = !app.cookie_modal_open;
            }
            if ui.button("cURL").on_hover_text("Import cURL").clicked() {
                app.curl_import_open = !app.curl_import_open;
            }
            if !app.collections.is_empty()
                && ui
                    .button("Save")
                    .on_hover_text("Save request into collection")
                    .clicked()
            {
                let target_idx = app
                    .active_request
                    .collection_id
                    .as_ref()
                    .and_then(|cid| app.collections.iter().position(|c| &c.id == cid))
                    .unwrap_or(0);
                app.save_active_request_to_collection(target_idx);
            }
            if let Some((ci, ri)) = saved_loc {
                if ui
                    .button(RichText::new("Delete").color(pal.accent_rose))
                    .on_hover_text("Delete this saved request from collection")
                    .clicked()
                {
                    app.delete_request_at(ci, ri);
                }
            }
        });

        if app.curl_import_open {
            ui.add_space(8.0);
            ui.separator();
            ui.label(
                RichText::new(
                    "Paste cURL command (supports -X, -H, -d, --data-urlencode, -F @file, -u, -b, -k):",
                )
                .size(12.0)
                .color(pal.text_muted),
            );
            ui.add(
                egui::TextEdit::multiline(&mut app.curl_import_text)
                    .code_editor()
                    .desired_rows(3)
                    .desired_width(ui.available_width()),
            );
            ui.horizontal(|ui| {
                if primary_button(
                    ui,
                    "Parse & Load cURL",
                    !app.curl_import_text.trim().is_empty(),
                )
                .clicked()
                {
                    app.active_request = bapu_core::curl::parse_curl_command(&app.curl_import_text);
                    app.curl_import_open = false;
                }
                if ui.button("Cancel").clicked() {
                    app.curl_import_open = false;
                }
            });
        }
    });

    if let Some(err) = &app.api_error {
        ui.add_space(6.0);
        Frame::none()
            .fill(pal.accent_rose.gamma_multiply(0.14))
            .stroke(Stroke::new(1.0_f32, pal.accent_rose.gamma_multiply(0.45)))
            .rounding(Rounding::same(8.0_f32))
            .inner_margin(Margin::symmetric(12.0_f32, 8.0_f32))
            .show(ui, |ui| {
                ui.colored_label(pal.accent_rose, err);
            });
    }

    ui.add_space(10.0);

    let avail_h = ui.available_height().max(320.0);
    let mut trigger_oauth_fetch = false;

    ui.columns(2, |cols| {
        // ---- LEFT PANE: REQUEST CONFIGURATION ----
        card_frame(&pal).show(&mut cols[0], |ui| {
            ui.set_min_height(avail_h - 30.0);
            ui.horizontal_wrapped(|ui| {
                let p_cnt =
                    app.active_request.params.len() + app.active_request.path_params.len();
                let h_cnt = app.active_request.headers.len();
                let ex_cnt = app.active_request.examples.len();

                for (tab, label) in [
                    (ApiReqSubTab::Params, format!("Params ({p_cnt})")),
                    (ApiReqSubTab::Headers, format!("Headers ({h_cnt})")),
                    (ApiReqSubTab::Body, "Body".into()),
                    (ApiReqSubTab::Auth, "Auth".into()),
                    (ApiReqSubTab::PreScript, "Pre-Script".into()),
                    (ApiReqSubTab::Tests, "Tests".into()),
                    (ApiReqSubTab::Examples, format!("Examples ({ex_cnt})")),
                    (ApiReqSubTab::Settings, "Settings".into()),
                ] {
                    if subtab_button(ui, &pal, app.api_req_subtab == tab, &label).clicked() {
                        app.api_req_subtab = tab;
                    }
                }
            });
            ui.separator();

            egui::ScrollArea::vertical()
                .id_salt("api_req_scroll")
                .max_height(avail_h - 85.0)
                .show(ui, |ui| match app.api_req_subtab {
                    ApiReqSubTab::Params => {
                        ui.label(
                            RichText::new("QUERY PARAMETERS")
                                .size(11.0)
                                .strong()
                                .color(pal.text_dim),
                        );
                        render_kv_matrix(
                            ui,
                            &pal,
                            &mut app.active_request.params,
                            "+ Add Query Param",
                        );

                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("PATH VARIABLES (:param)")
                                    .size(11.0)
                                    .strong()
                                    .color(pal.text_dim),
                            );
                            if ui
                                .small_button("Sync :params from URL")
                                .on_hover_text("Detect :id segments in the URL")
                                .clicked()
                            {
                                let detected = bapu_core::http_client::extract_path_param_names(
                                    &app.active_request.url,
                                );
                                for name in detected {
                                    if !app
                                        .active_request
                                        .path_params
                                        .iter()
                                        .any(|p| p.key == name)
                                    {
                                        app.active_request.path_params.push(KeyValuePair {
                                            id: uuid::Uuid::new_v4().to_string(),
                                            key: name,
                                            value: String::new(),
                                            enabled: true,
                                            is_secret: None,
                                        });
                                    }
                                }
                            }
                        });
                        render_kv_matrix(
                            ui,
                            &pal,
                            &mut app.active_request.path_params,
                            "+ Add Path Variable",
                        );
                    }
                    ApiReqSubTab::Headers => {
                        render_kv_matrix(
                            ui,
                            &pal,
                            &mut app.active_request.headers,
                            "+ Add HTTP Header",
                        );
                    }
                    ApiReqSubTab::Body => {
                        ui.horizontal_wrapped(|ui| {
                            for (bt, label) in [
                                (BodyType::None, "None"),
                                (BodyType::Json, "JSON"),
                                (BodyType::Form, "Form URL-Encoded"),
                                (BodyType::Multipart, "Multipart Form"),
                                (BodyType::Graphql, "GraphQL"),
                                (BodyType::Xml, "XML"),
                                (BodyType::Raw, "Raw Text"),
                            ] {
                                ui.selectable_value(&mut app.active_request.body_type, bt, label);
                            }
                        });
                        ui.add_space(6.0);
                        match app.active_request.body_type {
                            BodyType::None => {
                                ui.label(
                                    RichText::new("This request sends no body payload.")
                                        .color(pal.text_dim),
                                );
                            }
                            BodyType::Json => {
                                ui.horizontal(|ui| {
                                    if ui.small_button("Prettify JSON").clicked() {
                                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(
                                            &app.active_request.body_content,
                                        ) {
                                            if let Ok(p) = serde_json::to_string_pretty(&v) {
                                                app.active_request.body_content = p;
                                            }
                                        }
                                    }
                                    ui.label(
                                        RichText::new(
                                            "Supports {{$guid}}, {{$randomEmail}}, {{$timestamp}}, {{ENV_VAR}}",
                                        )
                                        .size(11.0)
                                        .color(pal.text_dim),
                                    );
                                });
                                ui.add(
                                    egui::TextEdit::multiline(&mut app.active_request.body_content)
                                        .code_editor()
                                        .desired_rows(13)
                                        .desired_width(ui.available_width()),
                                );
                            }
                            BodyType::Form => {
                                ui.label(
                                    RichText::new("application/x-www-form-urlencoded fields:")
                                        .size(11.5)
                                        .color(pal.text_muted),
                                );
                                render_kv_matrix(
                                    ui,
                                    &pal,
                                    &mut app.active_request.form_fields,
                                    "+ Add Form Field",
                                );
                            }
                            BodyType::Multipart => {
                                ui.label(
                                    RichText::new(
                                        "multipart/form-data fields & binary file uploads:",
                                    )
                                    .size(11.5)
                                    .color(pal.text_muted),
                                );
                                render_multipart_matrix(
                                    ui,
                                    &pal,
                                    &mut app.active_request.multipart_fields,
                                );
                            }
                            BodyType::Graphql => {
                                ui.horizontal(|ui| {
                                    ui.label(RichText::new("GraphQL Query").size(11.5).strong());
                                    if ui
                                        .small_button("Load Schema Introspection Query")
                                        .clicked()
                                    {
                                        app.active_request.graphql_query =
                                            Some(GRAPHQL_INTROSPECTION_QUERY.into());
                                        app.active_request.method = HttpMethod::POST;
                                    }
                                });
                                ui.add(
                                    egui::TextEdit::multiline(
                                        app.active_request
                                            .graphql_query
                                            .get_or_insert_with(String::new),
                                    )
                                    .code_editor()
                                    .desired_rows(8)
                                    .desired_width(ui.available_width()),
                                );
                                ui.add_space(6.0);
                                ui.label(
                                    RichText::new("GraphQL Variables (JSON)").size(11.5).strong(),
                                );
                                ui.add(
                                    egui::TextEdit::multiline(
                                        app.active_request
                                            .graphql_variables
                                            .get_or_insert_with(|| "{}".into()),
                                    )
                                    .code_editor()
                                    .desired_rows(4)
                                    .desired_width(ui.available_width()),
                                );
                            }
                            BodyType::Xml | BodyType::Raw => {
                                ui.add(
                                    egui::TextEdit::multiline(&mut app.active_request.body_content)
                                        .code_editor()
                                        .desired_rows(13)
                                        .desired_width(ui.available_width()),
                                );
                            }
                        }
                    }
                    ApiReqSubTab::Auth => {
                        ui.horizontal_wrapped(|ui| {
                            for (a, label) in [
                                (AuthType::None, "None"),
                                (AuthType::Bearer, "Bearer"),
                                (AuthType::Basic, "Basic"),
                                (AuthType::Apikey, "API Key"),
                                (AuthType::Oauth2, "OAuth 2.0"),
                                (AuthType::Awssigv4, "AWS SigV4"),
                                (AuthType::Jwt, "JWT HS256"),
                            ] {
                                ui.selectable_value(&mut app.active_request.auth_type, a, label);
                            }
                        });
                        ui.add_space(8.0);
                        match app.active_request.auth_type {
                            AuthType::None => {
                                ui.label(
                                    RichText::new("No authentication header attached.")
                                        .color(pal.text_dim),
                                );
                            }
                            AuthType::Bearer => {
                                ui.label("Bearer Token:");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .token
                                            .get_or_insert_with(String::new),
                                    )
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(ui.available_width()),
                                );
                            }
                            AuthType::Basic => {
                                ui.label("Username:");
                                ui.text_edit_singleline(
                                    app.active_request
                                        .auth_config
                                        .username
                                        .get_or_insert_with(String::new),
                                );
                                ui.label("Password:");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .password
                                            .get_or_insert_with(String::new),
                                    )
                                    .password(true),
                                );
                            }
                            AuthType::Apikey => {
                                ui.horizontal(|ui| {
                                    ui.label("Add To:");
                                    let loc = app
                                        .active_request
                                        .auth_config
                                        .add_to
                                        .get_or_insert_with(|| "header".into());
                                    ui.selectable_value(loc, "header".into(), "HTTP Header");
                                    ui.selectable_value(loc, "query".into(), "Query Param");
                                });
                                ui.label("Key Name:");
                                ui.text_edit_singleline(
                                    app.active_request
                                        .auth_config
                                        .api_key_name
                                        .get_or_insert_with(|| "X-API-Key".into()),
                                );
                                ui.label("Key Value:");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .api_key_value
                                            .get_or_insert_with(String::new),
                                    )
                                    .password(true)
                                    .desired_width(ui.available_width()),
                                );
                            }
                            AuthType::Oauth2 => {
                                ui.horizontal(|ui| {
                                    ui.label("Grant Type:");
                                    let gt = app
                                        .active_request
                                        .auth_config
                                        .oauth_grant_type
                                        .get_or_insert_with(|| "client_credentials".into());
                                    ui.selectable_value(
                                        gt,
                                        "client_credentials".into(),
                                        "Client Credentials",
                                    );
                                    ui.selectable_value(
                                        gt,
                                        "password".into(),
                                        "Password Credentials",
                                    );
                                });
                                ui.label("Access Token URL:");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .oauth_token_url
                                            .get_or_insert_with(String::new),
                                    )
                                    .font(egui::TextStyle::Monospace)
                                    .hint_text("https://auth.example.com/oauth/token")
                                    .desired_width(ui.available_width()),
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Client ID");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .oauth_client_id
                                            .get_or_insert_with(String::new),
                                    );
                                    ui.label("Secret");
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            app.active_request
                                                .auth_config
                                                .oauth_client_secret
                                                .get_or_insert_with(String::new),
                                        )
                                        .password(true),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Scope");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .oauth_scope
                                            .get_or_insert_with(String::new),
                                    );
                                    if emerald_button(ui, "Fetch Access Token", !app.api_busy)
                                        .clicked()
                                    {
                                        trigger_oauth_fetch = true;
                                    }
                                });
                                ui.label("Current Access Token (auto-fetched on Send if empty):");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .token
                                            .get_or_insert_with(String::new),
                                    )
                                    .font(egui::TextStyle::Monospace)
                                    .desired_width(ui.available_width()),
                                );
                            }
                            AuthType::Awssigv4 => {
                                badge_pill(
                                    ui,
                                    "AWS4-HMAC-SHA256 Request Signer",
                                    pal.accent_amber,
                                );
                                ui.horizontal(|ui| {
                                    ui.label("Access Key ID");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .aws_access_key
                                            .get_or_insert_with(String::new),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Secret Access Key");
                                    ui.add(
                                        egui::TextEdit::singleline(
                                            app.active_request
                                                .auth_config
                                                .aws_secret_key
                                                .get_or_insert_with(String::new),
                                        )
                                        .password(true),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Region");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .aws_region
                                            .get_or_insert_with(|| "us-east-1".into()),
                                    );
                                    ui.label("Service");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .aws_service
                                            .get_or_insert_with(|| "execute-api".into()),
                                    );
                                });
                                ui.horizontal(|ui| {
                                    ui.label("Session Token (optional)");
                                    ui.text_edit_singleline(
                                        app.active_request
                                            .auth_config
                                            .aws_session_token
                                            .get_or_insert_with(String::new),
                                    );
                                });
                            }
                            AuthType::Jwt => {
                                badge_pill(
                                    ui,
                                    "Local HS256 JWT Signer (Bearer)",
                                    pal.accent_violet,
                                );
                                ui.label("HMAC Secret:");
                                ui.add(
                                    egui::TextEdit::singleline(
                                        app.active_request
                                            .auth_config
                                            .jwt_secret
                                            .get_or_insert_with(String::new),
                                    )
                                    .password(true)
                                    .desired_width(ui.available_width()),
                                );
                                ui.label("JWT Claims Payload (JSON):");
                                ui.add(
                                    egui::TextEdit::multiline(
                                        app.active_request
                                            .auth_config
                                            .jwt_payload
                                            .get_or_insert_with(|| {
                                                "{\n  \"sub\": \"user_123\",\n  \"role\": \"admin\"\n}"
                                                    .into()
                                            }),
                                    )
                                    .code_editor()
                                    .desired_rows(5)
                                    .desired_width(ui.available_width()),
                                );
                            }
                        }
                    }
                    ApiReqSubTab::PreScript => {
                        ui.label(
                            RichText::new(
                                "Sandboxed Pre-Request JS (`bapu.environment`, `bapu.globals`, `bapu.collectionVariables`, `bapu.execution.setNextRequest`):",
                            )
                            .size(11.3)
                            .color(pal.text_muted),
                        );
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(
                                app.active_request
                                    .pre_request_script
                                    .get_or_insert_with(String::new),
                            )
                            .code_editor()
                            .desired_rows(13)
                            .desired_width(ui.available_width()),
                        );
                    }
                    ApiReqSubTab::Tests => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new("Post-Response JS Assertions:")
                                    .size(11.5)
                                    .color(pal.text_muted),
                            );
                            if let Some(ref resp) = app.last_response {
                                if ui
                                    .small_button("✨ Auto-Generate Assertions from Response")
                                    .clicked()
                                {
                                    app.active_request.test_script = Some(
                                        bapu_core::assertion_gen::generate_assertions_from_response(
                                            resp,
                                        ),
                                    );
                                }
                            }
                        });
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(
                                app.active_request
                                    .test_script
                                    .get_or_insert_with(String::new),
                            )
                            .code_editor()
                            .desired_rows(13)
                            .desired_width(ui.available_width()),
                        );
                    }
                    ApiReqSubTab::Examples => {
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(
                                    "Saved Examples power offline API Docs and the Local Mock Server.",
                                )
                                .size(11.5)
                                .color(pal.text_muted),
                            );
                            if let Some(ref resp) = app.last_response {
                                if ui.small_button("+ Save Current Response as Example").clicked() {
                                    app.active_request.examples.push(ApiExample {
                                        id: uuid::Uuid::new_v4().to_string(),
                                        name: format!("{} {}", resp.status, resp.status_text),
                                        status: resp.status,
                                        headers: resp.headers.clone(),
                                        body: serde_json::to_string_pretty(&resp.data)
                                            .unwrap_or_default(),
                                    });
                                }
                            }
                        });
                        ui.add_space(6.0);
                        let mut del_ex = None;
                        for (ei, ex) in app.active_request.examples.iter_mut().enumerate() {
                            Frame::none()
                                .fill(pal.bg_app)
                                .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                                .rounding(Rounding::same(6.0_f32))
                                .inner_margin(Margin::same(8.0_f32))
                                .show(ui, |ui| {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut ex.name)
                                                .desired_width(160.0),
                                        );
                                        ui.label("HTTP Status:");
                                        ui.add(
                                            egui::DragValue::new(&mut ex.status).range(100..=599),
                                        );
                                        if ui.small_button("Delete").clicked() {
                                            del_ex = Some(ei);
                                        }
                                    });
                                    ui.add(
                                        egui::TextEdit::multiline(&mut ex.body)
                                            .code_editor()
                                            .desired_rows(4)
                                            .desired_width(ui.available_width()),
                                    );
                                });
                            ui.add_space(4.0);
                        }
                        if let Some(i) = del_ex {
                            app.active_request.examples.remove(i);
                        }
                    }
                    ApiReqSubTab::Settings => {
                        ui.checkbox(
                            &mut app.active_request.follow_redirects,
                            "Automatically follow HTTP 3xx redirects",
                        );
                        ui.checkbox(
                            &mut app.active_request.verify_tls,
                            "Verify SSL / TLS server certificates (disable only for local self-signed dev certs)",
                        );
                        ui.horizontal(|ui| {
                            ui.label("Request Timeout (ms):");
                            let t = app.active_request.timeout_ms.get_or_insert(30_000);
                            ui.add(egui::DragValue::new(t).range(500..=300_000).speed(250));
                        });
                        ui.add_space(6.0);
                        ui.label("Endpoint Documentation / Notes (included in Markdown & OpenAPI exports):");
                        ui.add(
                            egui::TextEdit::multiline(
                                app.active_request
                                    .description
                                    .get_or_insert_with(String::new),
                            )
                            .desired_rows(5)
                            .desired_width(ui.available_width()),
                        );
                    }
                });
        });

        // ---- RIGHT PANE: RESPONSE VIEWER ----
        Frame::none()
            .fill(pal.bg_console)
            .stroke(Stroke::new(1.0_f32, pal.border_subtle))
            .rounding(Rounding::same(10.0_f32))
            .inner_margin(Margin::same(14.0_f32))
            .show(&mut cols[1], |ui| {
                ui.set_min_height(avail_h - 30.0);
                match &app.last_response {
                    None => {
                        ui.vertical_centered(|ui| {
                            ui.add_space((avail_h * 0.28).max(40.0));
                            paint_icon(ui, VectorIcon::Globe, 36.0, pal.text_dim);
                            ui.add_space(10.0);
                            ui.label(
                                RichText::new("Ready to Dispatch")
                                    .size(15.0)
                                    .strong()
                                    .color(pal.text_muted),
                            );
                            ui.label(
                                RichText::new(
                                    "Send a request (Ctrl+Enter) to inspect status, cookies, headers & assertions.",
                                )
                                .size(12.0)
                                .color(pal.text_dim),
                            );
                        });
                    }
                    Some(resp) => {
                        let resp = resp.clone();
                        let passed = resp.test_results.iter().filter(|t| t.passed).count();
                        let total_t = resp.test_results.len();

                        ui.horizontal_wrapped(|ui| {
                            status_pill(ui, resp.status, &resp.status_text);
                            badge_pill(ui, &format!("{} ms", resp.time_ms), pal.accent_cyan);
                            badge_pill(ui, &format_bytes(resp.size_bytes), pal.accent_violet);
                            if total_t > 0 {
                                let col = if passed == total_t {
                                    pal.accent_emerald
                                } else {
                                    pal.accent_rose
                                };
                                badge_pill(ui, &format!("Tests {passed}/{total_t}"), col);
                            }
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui.small_button("Copy Body").clicked() {
                                        let txt = serde_json::to_string_pretty(&resp.data)
                                            .unwrap_or_default();
                                        ui.output_mut(|o| o.copied_text = txt);
                                    }
                                },
                            );
                        });

                        ui.add_space(4.0);
                        ui.horizontal_wrapped(|ui| {
                            for (tab, label) in [
                                (ApiRespSubTab::Preview, "Preview".into()),
                                (ApiRespSubTab::Raw, "Raw".into()),
                                (
                                    ApiRespSubTab::Headers,
                                    format!("Headers ({})", resp.headers.len()),
                                ),
                                (
                                    ApiRespSubTab::Cookies,
                                    format!("Cookies ({})", resp.cookies.len()),
                                ),
                                (ApiRespSubTab::Tests, format!("Assertions ({total_t})")),
                                (
                                    ApiRespSubTab::Console,
                                    format!("Console ({})", resp.console_logs.len()),
                                ),
                            ] {
                                if subtab_button(ui, &pal, app.api_resp_subtab == tab, &label)
                                    .clicked()
                                {
                                    app.api_resp_subtab = tab;
                                }
                            }
                        });
                        ui.separator();

                        egui::ScrollArea::vertical()
                            .id_salt("api_resp_scroll")
                            .max_height(avail_h - 115.0)
                            .show(ui, |ui| match app.api_resp_subtab {
                                ApiRespSubTab::Preview => {
                                    ui.horizontal(|ui| {
                                        ui.add(
                                            egui::TextEdit::singleline(&mut app.resp_filter)
                                                .hint_text("Filter JSON lines / keys...")
                                                .desired_width(220.0),
                                        );
                                        if !app.resp_filter.is_empty()
                                            && ui.small_button("Clear").clicked()
                                        {
                                            app.resp_filter.clear();
                                        }
                                    });
                                    let full = match &resp.data {
                                        serde_json::Value::String(s) => s.clone(),
                                        other => serde_json::to_string_pretty(other)
                                            .unwrap_or_default(),
                                    };
                                    let q = app.resp_filter.trim().to_ascii_lowercase();
                                    let mut shown = if q.is_empty() {
                                        full
                                    } else {
                                        full.lines()
                                            .filter(|l| l.to_ascii_lowercase().contains(&q))
                                            .collect::<Vec<_>>()
                                            .join("\n")
                                    };
                                    ui.add(
                                        egui::TextEdit::multiline(&mut shown)
                                            .code_editor()
                                            .interactive(false)
                                            .desired_width(ui.available_width()),
                                    );
                                }
                                ApiRespSubTab::Raw => {
                                    let mut raw = resp.data.to_string();
                                    ui.add(
                                        egui::TextEdit::multiline(&mut raw)
                                            .code_editor()
                                            .interactive(false)
                                            .desired_width(ui.available_width()),
                                    );
                                }
                                ApiRespSubTab::Headers => {
                                    let mut keys: Vec<_> = resp.headers.keys().collect();
                                    keys.sort();
                                    for k in keys {
                                        if let Some(v) = resp.headers.get(k) {
                                            ui.horizontal_wrapped(|ui| {
                                                ui.label(
                                                    RichText::new(format!("{k}:"))
                                                        .monospace()
                                                        .strong()
                                                        .color(pal.accent_cyan),
                                                );
                                                ui.label(
                                                    RichText::new(v)
                                                        .monospace()
                                                        .color(pal.text_main),
                                                );
                                            });
                                        }
                                    }
                                }
                                ApiRespSubTab::Cookies => {
                                    if resp.cookies.is_empty() {
                                        ui.label(
                                            RichText::new(
                                                "No Set-Cookie headers returned by this response.",
                                            )
                                            .color(pal.text_dim),
                                        );
                                    }
                                    for c in &resp.cookies {
                                        ui.horizontal_wrapped(|ui| {
                                            badge_pill(ui, &c.name, pal.accent_cyan);
                                            ui.label(RichText::new(&c.value).monospace());
                                            ui.label(
                                                RichText::new(format!(
                                                    "({}{})",
                                                    c.domain, c.path
                                                ))
                                                .size(11.0)
                                                .color(pal.text_dim),
                                            );
                                            if c.secure {
                                                badge_pill(ui, "Secure", pal.accent_emerald);
                                            }
                                            if c.http_only {
                                                badge_pill(ui, "HttpOnly", pal.accent_amber);
                                            }
                                        });
                                    }
                                }
                                ApiRespSubTab::Tests => {
                                    if resp.test_results.is_empty() {
                                        ui.label(
                                            RichText::new("No post-response assertions ran.")
                                                .color(pal.text_dim),
                                        );
                                    }
                                    for t in &resp.test_results {
                                        let col = if t.passed {
                                            pal.accent_emerald
                                        } else {
                                            pal.accent_rose
                                        };
                                        Frame::none()
                                            .fill(col.gamma_multiply(0.1))
                                            .stroke(Stroke::new(
                                                1.0_f32,
                                                col.gamma_multiply(0.35),
                                            ))
                                            .rounding(Rounding::same(6.0_f32))
                                            .inner_margin(Margin::symmetric(10.0_f32, 6.0_f32))
                                            .show(ui, |ui| {
                                                ui.horizontal(|ui| {
                                                    badge_pill(
                                                        ui,
                                                        if t.passed { "PASS" } else { "FAIL" },
                                                        col,
                                                    );
                                                    ui.strong(&t.name);
                                                });
                                                if let Some(err) = &t.error {
                                                    ui.colored_label(col, err);
                                                }
                                            });
                                        ui.add_space(4.0);
                                    }
                                }
                                ApiRespSubTab::Console => {
                                    if resp.console_logs.is_empty() {
                                        ui.label(
                                            RichText::new("No console.log output recorded.")
                                                .color(pal.text_dim),
                                        );
                                    }
                                    for line in &resp.console_logs {
                                        ui.label(
                                            RichText::new(line)
                                                .monospace()
                                                .color(pal.text_muted),
                                        );
                                    }
                                }
                            });
                    }
                }
            });
    });

    if trigger_oauth_fetch {
        app.fetch_oauth2_for_active_request();
    }
}

fn render_kv_matrix(
    ui: &mut egui::Ui,
    pal: &crate::ui::theme::Palette,
    items: &mut Vec<KeyValuePair>,
    add_label: &str,
) {
    let mut remove_idx = None;
    for (i, kv) in items.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut kv.enabled, "");
            ui.add(
                egui::TextEdit::singleline(&mut kv.key)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("key")
                    .desired_width(140.0),
            );
            let val_w = (ui.available_width() - 38.0).max(90.0);
            ui.add(
                egui::TextEdit::singleline(&mut kv.value)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("value")
                    .desired_width(val_w),
            );
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
    }
    if let Some(i) = remove_idx {
        items.remove(i);
    }
    ui.add_space(4.0);
    if ui.button(add_label).clicked() {
        items.push(KeyValuePair {
            id: uuid::Uuid::new_v4().to_string(),
            key: String::new(),
            value: String::new(),
            enabled: true,
            is_secret: None,
        });
    }
}

fn render_multipart_matrix(
    ui: &mut egui::Ui,
    pal: &crate::ui::theme::Palette,
    fields: &mut Vec<MultipartField>,
) {
    let mut remove_idx = None;
    for (i, f) in fields.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.checkbox(&mut f.enabled, "");
            ui.add(
                egui::TextEdit::singleline(&mut f.key)
                    .font(egui::TextStyle::Monospace)
                    .hint_text("field_name")
                    .desired_width(120.0),
            );
            ui.checkbox(&mut f.is_file, "File");
            let val_w = (ui.available_width() - if f.is_file { 105.0 } else { 38.0 }).max(80.0);
            ui.add(
                egui::TextEdit::singleline(&mut f.value)
                    .font(egui::TextStyle::Monospace)
                    .hint_text(if f.is_file { "/path/to/file" } else { "value" })
                    .desired_width(val_w),
            );
            if f.is_file && ui.small_button("Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_file() {
                    f.value = path.to_string_lossy().into_owned();
                }
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
    }
    if let Some(i) = remove_idx {
        fields.remove(i);
    }
    ui.add_space(4.0);
    if ui.button("+ Add Multipart Field").clicked() {
        fields.push(MultipartField {
            id: uuid::Uuid::new_v4().to_string(),
            key: String::new(),
            value: String::new(),
            is_file: false,
            enabled: true,
        });
    }
}

fn format_bytes(b: u64) -> String {
    if b < 1024 {
        format!("{b} B")
    } else if b < 1024 * 1024 {
        format!("{:.1} KB", b as f64 / 1024.0)
    } else {
        format!("{:.2} MB", b as f64 / (1024.0 * 1024.0))
    }
}
