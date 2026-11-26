pub mod ai_copilot;
pub mod api_studio;
pub mod database_studio;
pub mod db_erd;
pub mod db_modals;
pub mod history_studio;
pub mod runner_modals;
pub mod secrets_studio;
pub mod stream_studio;
pub mod theme;
pub mod widgets;

use crate::app::{blank_request, BapuApp, Tab};
use crate::ui::widgets::{
    badge_pill, bapu_logo, db_brand_name, db_logo, method_badge, paint_icon, postman_logo,
    VectorIcon,
};
use bapu_core::stream_client::StreamKind;
use bapu_core::types::{Collection, DatabaseConnection, DbKind, Environment};
use egui::{vec2, Color32, Frame, Margin, RichText, Rounding, Sense, Stroke};

pub fn top_bar(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let total_reqs: usize = app.collections.iter().map(|c| c.requests.len()).sum();

    egui::TopBottomPanel::top("top_bar")
        .exact_height(50.0)
        .frame(
            Frame::none()
                .fill(pal.bg_header)
                .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                .inner_margin(Margin::symmetric(16.0_f32, 8.0_f32)),
        )
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                // Brand Identity: Real Logo + Title + AGPLv3 Tag
                ui.add(
                    egui::Image::new(bapu_logo())
                        .fit_to_exact_size(vec2(28.0, 28.0))
                        .rounding(Rounding::same(6.0_f32)),
                );
                ui.label(
                    RichText::new("Bapu Studio")
                        .size(15.5)
                        .strong()
                        .color(pal.text_main),
                );
                badge_pill(ui, concat!("v", env!("CARGO_PKG_VERSION")), pal.accent_primary);

                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);

                paint_icon(ui, VectorIcon::Folder, 14.0, pal.accent_emerald);
                ui.label(
                    RichText::new(format!(
                        "{} collections  •  {} requests",
                        app.collections.len(),
                        total_reqs
                    ))
                    .size(11.8)
                    .color(pal.text_muted),
                );

                // Right-aligned controls (drawn right-to-left)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // GitHub Sponsor Button
                    let sponsor_btn = Frame::none()
                        .fill(pal.accent_pink.gamma_multiply(0.14))
                        .stroke(Stroke::new(1.0_f32, pal.accent_pink.gamma_multiply(0.4)))
                        .rounding(Rounding::same(7.0_f32))
                        .inner_margin(Margin::symmetric(10.0_f32, 5.0_f32))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                paint_icon(ui, VectorIcon::Heart, 12.0, pal.accent_pink);
                                ui.label(
                                    RichText::new("Sponsor")
                                        .size(11.5)
                                        .strong()
                                        .color(pal.accent_pink),
                                );
                            });
                        });
                    if sponsor_btn
                        .response
                        .interact(Sense::click())
                        .on_hover_text("Support Bapu Studio on GitHub Sponsors")
                        .clicked()
                    {
                        ctx.open_url(egui::OpenUrl::new_tab(
                            "https://github.com/sponsors/joravar",
                        ));
                    }

                    // Theme Toggle (Sun / Moon vector icon)
                    let theme_frame = Frame::none()
                        .fill(pal.bg_input)
                        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                        .rounding(Rounding::same(7.0_f32))
                        .inner_margin(Margin::symmetric(8.0_f32, 5.0_f32))
                        .show(ui, |ui| {
                            let icon = if app.dark_mode {
                                VectorIcon::Sun
                            } else {
                                VectorIcon::Moon
                            };
                            let col = if app.dark_mode {
                                pal.accent_amber
                            } else {
                                pal.accent_primary
                            };
                            paint_icon(ui, icon, 14.0, col);
                        });
                    if theme_frame
                        .response
                        .interact(Sense::click())
                        .on_hover_text("Toggle Dark / Light Theme")
                        .clicked()
                    {
                        app.dark_mode = !app.dark_mode;
                        app.persist();
                    }

                    // AI Copilot Trigger
                    let ai_frame = Frame::none()
                        .fill(pal.accent_primary.gamma_multiply(0.16))
                        .stroke(Stroke::new(1.0_f32, pal.accent_primary.gamma_multiply(0.45)))
                        .rounding(Rounding::same(7.0_f32))
                        .inner_margin(Margin::symmetric(10.0_f32, 5.0_f32))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 5.0;
                                paint_icon(ui, VectorIcon::Sparkles, 13.0, pal.accent_cyan);
                                ui.label(
                                    RichText::new("AI Copilot")
                                        .size(12.0)
                                        .strong()
                                        .color(pal.text_main),
                                );
                            });
                        });
                    if ai_frame.response.interact(Sense::click()).clicked() {
                        app.ai_modal_open = !app.ai_modal_open;
                    }

                    // Quick Find (Ctrl+K)
                    let qf_frame = Frame::none()
                        .fill(pal.bg_input)
                        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                        .rounding(Rounding::same(7.0_f32))
                        .inner_margin(Margin::symmetric(10.0_f32, 5.0_f32))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = 6.0;
                                paint_icon(ui, VectorIcon::Search, 12.5, pal.text_dim);
                                ui.label(
                                    RichText::new("Quick Find...")
                                        .size(11.8)
                                        .color(pal.text_dim),
                                );
                                badge_pill(ui, "Ctrl+K", pal.text_dim);
                            });
                        });
                    if qf_frame.response.interact(Sense::click()).clicked() {
                        app.quick_find_open = true;
                    }

                    // Environment Selector with Emerald Dot
                    let active_env_name = app
                        .environments
                        .get(app.active_env_index)
                        .map(|e| e.name.clone())
                        .unwrap_or_else(|| "Default".into());
                    egui::ComboBox::from_id_salt("header_env_selector")
                        .selected_text(format!("● {active_env_name}"))
                        .width(145.0)
                        .show_ui(ui, |ui| {
                            for (idx, env) in app.environments.iter().enumerate() {
                                ui.selectable_value(&mut app.active_env_index, idx, &env.name);
                            }
                        });

                    if app.api_busy || app.db_busy || app.ai_busy {
                        ui.spinner();
                    }
                });
            });
        });
}

pub fn side_bar(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();

    egui::SidePanel::left("side_bar")
        .resizable(true)
        .default_width(272.0)
        .width_range(220.0..=380.0)
        .frame(
            Frame::none()
                .fill(pal.bg_sidebar)
                .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                .inner_margin(Margin::same(0.0_f32)),
        )
        .show(ctx, |ui| {
            // Top 5-Studio Navigation Strip
            Frame::none()
                .fill(pal.bg_app.gamma_multiply(0.55))
                .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                .inner_margin(Margin::symmetric(8.0_f32, 7.0_f32))
                .show(ui, |ui| {
                    ui.columns(5, |cols| {
                        studio_nav_tile(&mut cols[0], app, &pal, Tab::Api, VectorIcon::Globe, "APIs");
                        studio_nav_tile(
                            &mut cols[1],
                            app,
                            &pal,
                            Tab::Database,
                            VectorIcon::Database,
                            "DBs",
                        );
                        studio_nav_tile(
                            &mut cols[2],
                            app,
                            &pal,
                            Tab::Streams,
                            VectorIcon::Radio,
                            "Streams",
                        );
                        studio_nav_tile(
                            &mut cols[3],
                            app,
                            &pal,
                            Tab::Secrets,
                            VectorIcon::ShieldKey,
                            "Secrets",
                        );
                        studio_nav_tile(
                            &mut cols[4],
                            app,
                            &pal,
                            Tab::History,
                            VectorIcon::Clock,
                            "History",
                        );
                    });
                });

            // Dynamic Studio Explorer
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    Frame::none()
                        .inner_margin(Margin::same(12.0_f32))
                        .show(ui, |ui| match app.tab {
                            Tab::Api => render_api_sidebar(app, ui, &pal),
                            Tab::Database => render_db_sidebar(app, ui, &pal),
                            Tab::Streams => render_streams_sidebar(app, ui, &pal),
                            Tab::Secrets => render_secrets_sidebar(app, ui, &pal),
                            Tab::History => render_history_sidebar(app, ui, &pal),
                        });
                });
        });
}

fn studio_nav_tile(
    ui: &mut egui::Ui,
    app: &mut BapuApp,
    pal: &theme::Palette,
    tab: Tab,
    icon: VectorIcon,
    label: &str,
) {
    let selected = app.tab == tab;
    let fill = if selected {
        pal.bg_card
    } else {
        Color32::TRANSPARENT
    };
    let stroke = if selected {
        Stroke::new(1.0_f32, pal.accent_primary.gamma_multiply(0.75))
    } else {
        Stroke::NONE
    };
    let icon_color = if selected {
        pal.accent_primary
    } else {
        pal.text_dim
    };
    let text_color = if selected { pal.text_main } else { pal.text_dim };

    let inner = Frame::none()
        .fill(fill)
        .stroke(stroke)
        .rounding(Rounding::same(7.0_f32))
        .inner_margin(Margin::symmetric(2.0_f32, 6.0_f32))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                paint_icon(ui, icon, 15.0, icon_color);
                ui.label(RichText::new(label).size(10.5).strong().color(text_color));
            });
        });

    if inner.response.interact(Sense::click()).clicked() {
        app.tab = tab;
    }
}

fn render_api_sidebar(app: &mut BapuApp, ui: &mut egui::Ui, pal: &theme::Palette) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("COLLECTIONS ({})", app.collections.len()))
                .size(11.0)
                .strong()
                .color(pal.text_dim),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .small_button("+ Folder")
                .on_hover_text("Create new collection")
                .clicked()
            {
                app.new_collection_open = !app.new_collection_open;
            }
            if ui
                .small_button("+ Req")
                .on_hover_text("New blank request")
                .clicked()
            {
                app.active_request = blank_request();
                app.last_response = None;
            }
        });
    });

    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        let pm_btn = ui.add(
            egui::Button::image_and_text(
                egui::Image::new(postman_logo()).fit_to_exact_size(vec2(14.0, 14.0)),
                RichText::new("Import / Docs").size(11.2),
            )
            .rounding(Rounding::same(6.0_f32)),
        );
        if pm_btn.clicked() {
            app.io_modal_open = !app.io_modal_open;
        }
        if ui
            .button(RichText::new("▶ Runner").size(11.2))
            .on_hover_text("Open Collection Runner & Data-Driven Tester")
            .clicked()
        {
            app.runner_open = !app.runner_open;
        }
        if ui
            .button(RichText::new("Mock").size(11.2))
            .on_hover_text("Start a local HTTP Mock Server from saved examples")
            .clicked()
        {
            app.mock_open = !app.mock_open;
        }
        if ui
            .button(RichText::new("Git Dir").size(11.2))
            .on_hover_text("Load a git-synced Bapu collection folder")
            .clicked()
        {
            if let Some(folder) = rfd::FileDialog::new()
                .set_title("Load Bapu Collection from Folder")
                .pick_folder()
            {
                match bapu_core::collection_sync::read_collection_folder(&folder) {
                    Ok(col) => {
                        app.collections.push(col);
                        app.persist();
                    }
                    Err(e) => app.api_error = Some(format!("Failed to load folder: {e}")),
                }
            }
        }
    });

    if app.new_collection_open {
        ui.add_space(4.0);
        Frame::none()
            .fill(pal.bg_card)
            .stroke(Stroke::new(1.0_f32, pal.border_subtle))
            .rounding(Rounding::same(7.0_f32))
            .inner_margin(Margin::same(8.0_f32))
            .show(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_collection_name)
                        .hint_text("Collection name...")
                        .desired_width(ui.available_width()),
                );
                ui.horizontal(|ui| {
                    if ui.small_button("Create").clicked()
                        && !app.new_collection_name.trim().is_empty()
                    {
                        app.collections.push(Collection {
                            id: uuid::Uuid::new_v4().to_string(),
                            name: app.new_collection_name.trim().to_string(),
                            description: None,
                            requests: vec![],
                            variables: vec![],
                            folder_path: None,
                        });
                        app.new_collection_name.clear();
                        app.new_collection_open = false;
                        app.persist();
                    }
                    if ui.small_button("Cancel").clicked() {
                        app.new_collection_open = false;
                    }
                });
            });
    }

    ui.add_space(8.0);

    let mut select_req = None;
    let mut add_to_col = None;
    let mut link_col = None;
    let mut delete_col: Option<usize> = None;
    let mut delete_req: Option<(usize, usize)> = None;
    let mut duplicate_req: Option<(usize, usize)> = None;
    let mut toggle_col: Option<String> = None;

    for (ci, col) in app.collections.iter().enumerate() {
        let collapsed = app.collapsed_collections.contains(&col.id);
        let hdr_resp = ui.horizontal(|ui| {
            let arrow = if collapsed { "▸" } else { "▾" };
            let name_btn = ui.add(
                egui::Button::new(
                    RichText::new(format!("{arrow} {}", col.name))
                        .size(12.5)
                        .strong()
                        .color(pal.text_main),
                )
                .frame(false)
                .truncate(),
            );
            if name_btn.clicked() {
                toggle_col = Some(col.id.clone());
            }
            name_btn.context_menu(|ui| {
                if ui.button("Save Active Request Here").clicked() {
                    add_to_col = Some(ci);
                    ui.close_menu();
                }
                if ui.button("Link to Git Folder...").clicked() {
                    link_col = Some(ci);
                    ui.close_menu();
                }
                ui.separator();
                if ui
                    .button(RichText::new("Delete Collection").color(pal.accent_rose))
                    .clicked()
                {
                    delete_col = Some(ci);
                    ui.close_menu();
                }
            });
            if col.folder_path.is_some() {
                badge_pill(ui, "GIT", pal.accent_emerald);
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("×").size(12.5).color(pal.text_dim))
                            .min_size(vec2(20.0, 20.0)),
                    )
                    .on_hover_text("Delete collection")
                    .clicked()
                {
                    delete_col = Some(ci);
                }
                if ui
                    .small_button("+")
                    .on_hover_text("Save active request into this collection")
                    .clicked()
                {
                    add_to_col = Some(ci);
                }
                if ui
                    .small_button("Sync")
                    .on_hover_text("Link collection to a Git folder on disk")
                    .clicked()
                {
                    link_col = Some(ci);
                }
            });
        });
        let _ = hdr_resp;

        if !collapsed {
            ui.add_space(2.0);
            for (ri, req) in col.requests.iter().enumerate() {
                let is_active = app.active_request.id == req.id;
                let row_fill = if is_active {
                    pal.bg_card_hover
                } else {
                    Color32::TRANSPARENT
                };
                let row_stroke = if is_active {
                    Stroke::new(1.0_f32, pal.accent_primary.gamma_multiply(0.6))
                } else {
                    Stroke::NONE
                };
                Frame::none()
                    .fill(row_fill)
                    .stroke(row_stroke)
                    .rounding(Rounding::same(6.0_f32))
                    .inner_margin(Margin::symmetric(8.0_f32, 4.0_f32))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let del_reserve = 28.0;
                            let left_w = (ui.available_width() - del_reserve).max(80.0);
                            let left_ui = ui.allocate_ui_with_layout(
                                vec2(left_w, 22.0),
                                egui::Layout::left_to_right(egui::Align::Center),
                                |ui| {
                                    ui.set_max_width(left_w);
                                    method_badge(ui, req.method);
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(&req.name).size(12.0).color(
                                                if is_active {
                                                    pal.text_main
                                                } else {
                                                    pal.text_muted
                                                },
                                            ),
                                        )
                                        .selectable(false)
                                        .truncate(),
                                    );
                                },
                            );
                            let click_resp = ui.interact(
                                left_ui.response.rect,
                                ui.id().with(("api_req_row", ci, ri)),
                                Sense::click(),
                            );
                            if click_resp.clicked() {
                                select_req = Some(req.clone());
                            }
                            click_resp.context_menu(|ui| {
                                if ui.button("Open Request").clicked() {
                                    select_req = Some(req.clone());
                                    ui.close_menu();
                                }
                                if ui.button("Duplicate Request").clicked() {
                                    duplicate_req = Some((ci, ri));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if ui
                                    .button(RichText::new("Delete Request").color(pal.accent_rose))
                                    .clicked()
                                {
                                    delete_req = Some((ci, ri));
                                    ui.close_menu();
                                }
                            });

                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                RichText::new("×").size(13.0).color(pal.text_dim),
                                            )
                                            .min_size(vec2(22.0, 20.0)),
                                        )
                                        .on_hover_text("Delete request")
                                        .clicked()
                                    {
                                        delete_req = Some((ci, ri));
                                    }
                                },
                            );
                        });
                    });
            }
            ui.add_space(6.0);
        }
    }

    if let Some(id) = toggle_col {
        if !app.collapsed_collections.remove(&id) {
            app.collapsed_collections.insert(id);
        }
    }
    if let Some(req) = select_req {
        app.active_request = req;
        app.last_response = None;
    }
    if let Some(ci) = add_to_col {
        app.save_active_request_to_collection(ci);
    }
    if let Some((ci, ri)) = duplicate_req {
        if let Some(col) = app.collections.get_mut(ci) {
            if let Some(orig) = col.requests.get(ri).cloned() {
                let mut copy = orig;
                copy.id = uuid::Uuid::new_v4().to_string();
                copy.name = format!("{} (Copy)", copy.name);
                col.requests.insert(ri + 1, copy);
                app.persist();
            }
        }
    }
    if let Some((ci, ri)) = delete_req {
        app.delete_request_at(ci, ri);
    }
    if let Some(ci) = delete_col {
        app.delete_collection_at(ci);
    }
    if let Some(ci) = link_col {
        if let Some(folder) = rfd::FileDialog::new()
            .set_title("Choose Git-Synced Folder for Collection")
            .pick_folder()
        {
            if let Some(col) = app.collections.get_mut(ci) {
                col.folder_path = Some(folder.to_string_lossy().into_owned());
                let _ = bapu_core::collection_sync::write_collection_folder(&folder, col);
                app.persist();
            }
        }
    }
}

fn render_db_sidebar(app: &mut BapuApp, ui: &mut egui::Ui, pal: &theme::Palette) {
    ui.horizontal(|ui| {
        ui.label(
            RichText::new(format!("CONNECTIONS ({})", app.databases.len()))
                .size(11.0)
                .strong()
                .color(pal.text_dim),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("+ New").clicked() {
                app.databases.push(DatabaseConnection {
                    id: uuid::Uuid::new_v4().to_string(),
                    name: "Postgres Local".into(),
                    kind: DbKind::Postgres,
                    connection_string: None,
                    host: Some("localhost".into()),
                    port: Some("5432".into()),
                    database: "postgres".into(),
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
                });
                app.active_db_index = app.databases.len() - 1;
                app.db_config_open = true;
                app.persist();
            }
        });
    });

    ui.add_space(6.0);

    if app.databases.is_empty() {
        Frame::none()
            .fill(pal.bg_card)
            .stroke(Stroke::new(1.0_f32, pal.border_subtle))
            .rounding(Rounding::same(8.0_f32))
            .inner_margin(Margin::same(12.0_f32))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("No databases connected yet.")
                        .size(12.0)
                        .color(pal.text_muted),
                );
                ui.add_space(6.0);
                if widgets::emerald_button(ui, "Launch SQLite Playground", !app.db_busy).clicked() {
                    app.create_sqlite_playground();
                }
            });
    } else {
        let mut select_idx = None;
        let mut configure_idx = None;
        let mut remove_idx = None;
        for (i, db) in app.databases.iter().enumerate() {
            let selected = i == app.active_db_index;
            let fill = if selected { pal.bg_card_hover } else { pal.bg_card };
            let stroke = if selected {
                Stroke::new(1.2_f32, pal.accent_primary)
            } else {
                Stroke::new(1.0_f32, pal.border_subtle)
            };
            Frame::none()
                .fill(fill)
                .stroke(stroke)
                .rounding(Rounding::same(8.0_f32))
                .inner_margin(Margin::symmetric(10.0_f32, 8.0_f32))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let del_reserve = 30.0;
                        let left_w = (ui.available_width() - del_reserve).max(90.0);
                        let left_ui = ui.allocate_ui_with_layout(
                            vec2(left_w, 32.0),
                            egui::Layout::left_to_right(egui::Align::Center),
                            |ui| {
                                ui.set_max_width(left_w);
                                ui.add(
                                    egui::Image::new(db_logo(db.kind))
                                        .fit_to_exact_size(vec2(24.0, 24.0))
                                        .rounding(Rounding::same(5.0_f32)),
                                );
                                ui.vertical(|ui| {
                                    ui.set_max_width((left_w - 32.0).max(50.0));
                                    ui.spacing_mut().item_spacing.y = 1.0;
                                    ui.horizontal(|ui| {
                                        let dot_col = if db.is_connected {
                                            pal.accent_emerald
                                        } else {
                                            pal.text_dim
                                        };
                                        ui.colored_label(dot_col, "●");
                                        ui.add(
                                            egui::Label::new(
                                                RichText::new(&db.name)
                                                    .size(12.5)
                                                    .strong()
                                                    .color(pal.text_main),
                                            )
                                            .selectable(false)
                                            .truncate(),
                                        );
                                    });
                                    ui.add(
                                        egui::Label::new(
                                            RichText::new(format!(
                                                "{} • {}",
                                                db_brand_name(db.kind),
                                                if db.database.is_empty() {
                                                    "default"
                                                } else {
                                                    &db.database
                                                }
                                            ))
                                            .monospace()
                                            .size(10.5)
                                            .color(pal.text_dim),
                                        )
                                        .selectable(false)
                                        .truncate(),
                                    );
                                });
                            },
                        );
                        let click_resp = ui.interact(
                            left_ui.response.rect,
                            ui.id().with(("db_conn_card", i)),
                            Sense::click(),
                        );
                        if click_resp.clicked() {
                            select_idx = Some(i);
                        }
                        if click_resp.double_clicked() {
                            configure_idx = Some(i);
                        }
                        click_resp.context_menu(|ui| {
                            if ui.button("Configure Connection...").clicked() {
                                configure_idx = Some(i);
                                ui.close_menu();
                            }
                            ui.separator();
                            if ui
                                .button(RichText::new("Delete Connection").color(pal.accent_rose))
                                .clicked()
                            {
                                remove_idx = Some(i);
                                ui.close_menu();
                            }
                        });

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("×").size(13.5).color(pal.text_dim),
                                    )
                                    .min_size(vec2(24.0, 24.0)),
                                )
                                .on_hover_text("Delete connection")
                                .clicked()
                            {
                                remove_idx = Some(i);
                            }
                        });
                    });
                });
            ui.add_space(4.0);
        }

        if let Some(i) = select_idx {
            app.active_db_index = i;
        }
        if let Some(i) = configure_idx {
            app.active_db_index = i;
            app.db_config_open = true;
        }
        if let Some(i) = remove_idx {
            app.delete_database_at(i);
        }

        ui.horizontal_wrapped(|ui| {
            if ui
                .small_button("+ SQLite Playground")
                .on_hover_text("Create a sample local SQLite database with Foreign Keys & Views")
                .clicked()
            {
                app.create_sqlite_playground();
            }
            if ui
                .small_button("+ Table")
                .on_hover_text("Visual Table Designer & Forward Engineering")
                .clicked()
            {
                app.table_designer_open = true;
            }
            if ui
                .small_button("Import")
                .on_hover_text("Import CSV or JSON into a table")
                .clicked()
            {
                app.db_import_open = true;
            }
            if ui
                .small_button("Diff")
                .on_hover_text("Schema Compare & PK Table Data Compare")
                .clicked()
            {
                app.db_compare_open = true;
            }
            if ui
                .small_button("Mock")
                .on_hover_text("Generate FK-aware Mock Data")
                .clicked()
            {
                app.db_mock_gen_open = true;
            }
        });
    }

    // Schema Explorer for Active DB
    let mut clicked_table: Option<(DbKind, String, Option<String>)> = None;
    let mut open_erd_view = false;
    if let Some(db) = app.databases.get(app.active_db_index) {
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(
                RichText::new(format!("SCHEMA ({})", db.tables.len()))
                    .size(11.0)
                    .strong()
                    .color(pal.text_dim),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .small_button("ERD")
                    .on_hover_text("Open interactive Entity-Relationship Diagram")
                    .clicked()
                {
                    open_erd_view = true;
                }
            });
        });
        ui.add_space(4.0);
        ui.add(
            egui::TextEdit::singleline(&mut app.db_schema_filter)
                .hint_text("Filter tables / columns...")
                .desired_width(ui.available_width()),
        );
        ui.add_space(4.0);

        let schema_q = app.db_schema_filter.trim().to_ascii_lowercase();
        if db.tables.is_empty() {
            ui.label(
                RichText::new("Click 'Refresh Schema' in the toolbar to introspect tables.")
                    .size(11.5)
                    .color(pal.text_dim),
            );
        } else {
            for table in &db.tables {
                if !schema_q.is_empty()
                    && !table.name.to_ascii_lowercase().contains(&schema_q)
                    && !table
                        .columns
                        .iter()
                        .any(|c| c.name.to_ascii_lowercase().contains(&schema_q))
                {
                    continue;
                }
                let is_selected = app.grid_table_name.as_deref() == Some(table.name.as_str());
                let icon_prefix = if table.object_type == "VIEW" { "👁" } else { "▤" };
                let header_resp = ui.horizontal(|ui| {
                    let color = if is_selected {
                        pal.accent_cyan
                    } else if table.object_type == "VIEW" {
                        pal.accent_violet
                    } else {
                        pal.text_main
                    };
                    if ui
                        .selectable_label(
                            is_selected,
                            RichText::new(format!("{icon_prefix}  {}", table.name))
                                .size(12.2)
                                .strong()
                                .color(color),
                        )
                        .clicked()
                    {
                        let first_col = table.columns.first().map(|c| c.name.clone());
                        clicked_table = Some((db.kind, table.name.clone(), first_col));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!("{}", table.row_count))
                                .monospace()
                                .size(10.5)
                                .color(pal.text_dim),
                        );
                        if !table.foreign_keys.is_empty() {
                            badge_pill(
                                ui,
                                &format!("{} FK", table.foreign_keys.len()),
                                pal.accent_cyan,
                            );
                        }
                    });
                });
                let _ = header_resp;

                if is_selected {
                    ui.indent(format!("cols_{}", table.name), |ui| {
                        for col in &table.columns {
                            let is_fk = table.foreign_keys.iter().any(|fk| fk.column == col.name);
                            ui.horizontal(|ui| {
                                if col.is_primary_key {
                                    paint_icon(ui, VectorIcon::Key, 11.0, pal.accent_amber);
                                } else if is_fk {
                                    ui.label(RichText::new("↗").size(10.5).color(pal.accent_cyan));
                                }
                                ui.label(
                                    RichText::new(&col.name).monospace().size(11.0).color(
                                        if col.is_primary_key {
                                            pal.accent_amber
                                        } else if is_fk {
                                            pal.accent_cyan
                                        } else {
                                            pal.text_muted
                                        },
                                    ),
                                );
                                ui.label(
                                    RichText::new(&col.col_type)
                                        .monospace()
                                        .size(10.0)
                                        .color(pal.text_dim),
                                );
                            });
                        }
                    });
                }
            }
        }
    }

    if open_erd_view {
        app.db_subview = crate::app::DbSubView::Erd;
    }

    if let Some((kind, name, first_col)) = clicked_table {
        let qt = bapu_core::db_ddl::quote_dialect_ident(&name, kind);
        let limit = app.grid_page_size;
        let query = match kind {
            DbKind::Mongodb => format!("{name}.find({{}})"),
            DbKind::Couchdb => "{\n  \"selector\": {},\n  \"limit\": 50\n}".into(),
            DbKind::Neo4j => format!("MATCH (n:`{name}`) RETURN n LIMIT {limit}"),
            DbKind::Mssql | DbKind::AzureSql => format!("SELECT TOP {limit} * FROM {qt};"),
            DbKind::Oracle | DbKind::Db2 => {
                format!("SELECT * FROM {qt} FETCH FIRST {limit} ROWS ONLY")
            }
            DbKind::Redis => match (name.as_str(), first_col.as_deref()) {
                ("string", Some(k)) => format!("GET {k}"),
                ("hash", Some(k)) => format!("HGETALL {k}"),
                ("list", Some(k)) => format!("LRANGE {k} 0 24"),
                ("set", Some(k)) => format!("SMEMBERS {k}"),
                ("zset", Some(k)) => format!("ZRANGE {k} 0 24 WITHSCORES"),
                _ => "SCAN 0 MATCH * COUNT 50".into(),
            },
            _ => format!("SELECT * FROM {qt} LIMIT {limit};"),
        };
        *app.active_sql_mut() = query;
        app.grid_table_name = Some(name);
        app.grid_page = 0;
        app.run_active_query();
    }
}

fn render_streams_sidebar(app: &mut BapuApp, ui: &mut egui::Ui, pal: &theme::Palette) {
    ui.label(
        RichText::new("STREAM PRESETS")
            .size(11.0)
            .strong()
            .color(pal.text_dim),
    );
    ui.add_space(6.0);

    let presets = [
        (
            "Postman WebSocket Echo",
            StreamKind::WebSocket,
            "wss://ws.postman-echo.com/raw",
        ),
        (
            "WebSocket Events Echo",
            StreamKind::WebSocket,
            "wss://echo.websocket.events",
        ),
        (
            "Wikimedia RecentChanges SSE",
            StreamKind::Sse,
            "https://stream.wikimedia.org/v2/stream/recentchange",
        ),
    ];

    for (title, kind, url) in presets {
        let card = Frame::none()
            .fill(pal.bg_card)
            .stroke(Stroke::new(1.0_f32, pal.border_subtle))
            .rounding(Rounding::same(8.0_f32))
            .inner_margin(Margin::symmetric(10.0_f32, 8.0_f32))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let (tag, col) = match kind {
                        StreamKind::WebSocket => ("WS", pal.accent_cyan),
                        StreamKind::Sse => ("SSE", pal.accent_violet),
                    };
                    badge_pill(ui, tag, col);
                    ui.label(RichText::new(title).size(12.0).strong().color(pal.text_main));
                });
                ui.label(
                    RichText::new(url)
                        .monospace()
                        .size(10.2)
                        .color(pal.text_dim),
                );
            });
        if card.response.interact(Sense::click()).clicked() {
            app.stream_kind = kind;
            app.stream_url = url.into();
        }
        ui.add_space(5.0);
    }
}

fn render_secrets_sidebar(app: &mut BapuApp, ui: &mut egui::Ui, pal: &theme::Palette) {
    ui.label(
        RichText::new(format!("ENVIRONMENTS ({})", app.environments.len()))
            .size(11.0)
            .strong()
            .color(pal.text_dim),
    );
    ui.add_space(6.0);

    let can_delete_env = app.environments.len() > 1;
    let mut select_env = None;
    let mut delete_env = None;
    for (idx, env) in app.environments.iter().enumerate() {
        let selected = idx == app.active_env_index;
        let secret_count = env
            .variables
            .iter()
            .filter(|v| v.is_secret.unwrap_or(false))
            .count();
        Frame::none()
            .fill(if selected { pal.bg_card_hover } else { pal.bg_card })
            .stroke(Stroke::new(
                1.0_f32,
                if selected {
                    pal.accent_primary
                } else {
                    pal.border_subtle
                },
            ))
            .rounding(Rounding::same(8.0_f32))
            .inner_margin(Margin::symmetric(10.0_f32, 8.0_f32))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    let del_reserve = if can_delete_env { 28.0 } else { 0.0 };
                    let left_w = (ui.available_width() - del_reserve).max(90.0);
                    let left_ui = ui.allocate_ui_with_layout(
                        vec2(left_w, 22.0),
                        egui::Layout::left_to_right(egui::Align::Center),
                        |ui| {
                            ui.set_max_width(left_w);
                            ui.colored_label(pal.accent_emerald, "●");
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&env.name)
                                        .size(12.5)
                                        .strong()
                                        .color(pal.text_main),
                                )
                                .selectable(false)
                                .truncate(),
                            );
                            badge_pill(
                                ui,
                                &format!("{} ({} sec)", env.variables.len(), secret_count),
                                pal.accent_primary,
                            );
                        },
                    );
                    if ui
                        .interact(
                            left_ui.response.rect,
                            ui.id().with(("env_card_row", idx)),
                            Sense::click(),
                        )
                        .clicked()
                    {
                        select_env = Some(idx);
                    }
                    if can_delete_env {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("×").size(13.0).color(pal.text_dim),
                                    )
                                    .min_size(vec2(22.0, 20.0)),
                                )
                                .on_hover_text("Delete environment")
                                .clicked()
                            {
                                delete_env = Some(idx);
                            }
                        });
                    }
                });
            });
        ui.add_space(4.0);
    }
    if let Some(idx) = select_env {
        app.active_env_index = idx;
    }
    if let Some(idx) = delete_env {
        let removed = app.environments.remove(idx);
        app.storage.delete_environment_secrets(&removed);
        if app.active_env_index >= app.environments.len() {
            app.active_env_index = app.environments.len().saturating_sub(1);
        }
        app.persist();
    }

    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(&mut app.new_env_name)
                .hint_text("New environment (e.g. Prod)")
                .desired_width(165.0),
        );
        if ui.button("+ Add").clicked() && !app.new_env_name.trim().is_empty() {
            app.environments.push(Environment {
                id: uuid::Uuid::new_v4().to_string(),
                name: app.new_env_name.trim().to_string(),
                variables: vec![],
            });
            app.active_env_index = app.environments.len() - 1;
            app.new_env_name.clear();
            app.persist();
        }
    });
}

fn render_history_sidebar(app: &mut BapuApp, ui: &mut egui::Ui, pal: &theme::Palette) {
    let api_count = app.history.iter().filter(|h| h.kind == "api").count();
    let sql_count = app.history.iter().filter(|h| h.kind == "sql").count();

    ui.label(
        RichText::new("ACTIVITY SUMMARY")
            .size(11.0)
            .strong()
            .color(pal.text_dim),
    );
    ui.add_space(6.0);

    widgets::compact_card_frame(pal).show(ui, |ui| {
        ui.horizontal(|ui| {
            badge_pill(ui, "HTTP", pal.accent_cyan);
            ui.label(RichText::new(format!("{api_count} API requests")).size(12.0));
        });
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            badge_pill(ui, "SQL", pal.accent_emerald);
            ui.label(RichText::new(format!("{sql_count} DB queries")).size(12.0));
        });
    });
}

pub fn render_quick_find(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.quick_find_open;
    egui::Window::new("Quick Find")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .default_width(500.0)
        .anchor(egui::Align2::CENTER_TOP, vec2(0.0, 80.0))
        .show(ctx, |ui| {
            let resp = ui.add(
                egui::TextEdit::singleline(&mut app.quick_find_query)
                    .hint_text("Search API requests, database tables, or history...")
                    .desired_width(ui.available_width()),
            );
            resp.request_focus();

            let q = app.quick_find_query.trim().to_ascii_lowercase();
            ui.add_space(6.0);
            egui::ScrollArea::vertical().max_height(280.0).show(ui, |ui| {
                let mut picked_req = None;
                for col in &app.collections {
                    for req in &col.requests {
                        if q.is_empty()
                            || req.name.to_ascii_lowercase().contains(&q)
                            || req.url.to_ascii_lowercase().contains(&q)
                        {
                            let row = ui.horizontal(|ui| {
                                method_badge(ui, req.method);
                                if ui
                                    .selectable_label(
                                        false,
                                        format!("{}  —  {}", req.name, col.name),
                                    )
                                    .clicked()
                                {
                                    picked_req = Some(req.clone());
                                }
                            });
                            let _ = row;
                        }
                    }
                }
                if let Some(r) = picked_req {
                    app.active_request = r;
                    app.tab = Tab::Api;
                    app.quick_find_open = false;
                }

                let mut picked_db_table = None;
                for (di, db) in app.databases.iter().enumerate() {
                    for t in &db.tables {
                        if q.is_empty() || t.name.to_ascii_lowercase().contains(&q) {
                            ui.horizontal(|ui| {
                                badge_pill(ui, db_brand_name(db.kind), pal.accent_emerald);
                                if ui
                                    .selectable_label(false, format!("{}  ({})", t.name, db.name))
                                    .clicked()
                                {
                                    picked_db_table = Some((di, t.name.clone()));
                                }
                            });
                        }
                    }
                }
                if let Some((di, tname)) = picked_db_table {
                    app.active_db_index = di;
                    *app.active_sql_mut() = format!("SELECT * FROM \"{tname}\" LIMIT 50;");
                    app.grid_table_name = Some(tname);
                    app.tab = Tab::Database;
                    app.quick_find_open = false;
                }
            });
        });
    if !open {
        app.quick_find_open = false;
    }
}
