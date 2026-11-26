//! Floating studio windows for:
//! 1. Multi-Language Code Snippet Generator (`</> Code`)
//! 2. Domain Cookie Jar Manager (`Cookies`)
//! 3. Collection Runner & Data-Driven Testing (`Runner`)
//! 4. Local HTTP Mock Server (`Mock Server`)

use crate::app::BapuApp;
use crate::ui::widgets::{badge_pill, emerald_button, method_badge, primary_button, status_pill};
use bapu_core::codegen::{generate_code_snippet, CodeLang};
use bapu_core::types::CookieEntry;
use egui::{Frame, Margin, RichText, Rounding, Stroke};

pub fn render_all(app: &mut BapuApp, ctx: &egui::Context) {
    if app.codegen_open {
        render_codegen_window(app, ctx);
    }
    if app.cookie_modal_open {
        render_cookie_window(app, ctx);
    }
    if app.runner_open {
        render_runner_window(app, ctx);
    }
    if app.mock_open {
        render_mock_window(app, ctx);
    }
}

fn render_codegen_window(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.codegen_open;
    let env = app.active_env();

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Code Snippet Generator  (8 Languages)",
        &mut open,
        [650.0, 480.0],
        [440.0, 280.0],
        "codegen_win_scroll",
        |ui, viewport_h| {
            ui.horizontal_wrapped(|ui| {
                for lang in CodeLang::ALL {
                    ui.selectable_value(&mut app.codegen_lang, lang, lang.label());
                }
            });
            ui.separator();

            let mut snippet = generate_code_snippet(&app.active_request, &env, app.codegen_lang);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("Variables from the active environment are resolved automatically.")
                        .size(11.5)
                        .color(pal.text_muted),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, "Copy Snippet", true).clicked() {
                        ui.output_mut(|o| o.copied_text = snippet.clone());
                    }
                });
            });
            ui.add_space(4.0);
            let snip_h = (viewport_h - 95.0).max(160.0);
            egui::ScrollArea::vertical().id_salt("codegen_snip_inner").max_height(snip_h).show(ui, |ui| {
                ui.add(
                    egui::TextEdit::multiline(&mut snippet)
                        .code_editor()
                        .interactive(false)
                        .desired_width(ui.available_width()),
                );
            });
        },
    );
    app.codegen_open = open;
}

fn render_cookie_window(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.cookie_modal_open;

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Cookie Jar Manager",
        &mut open,
        [620.0, 420.0],
        [440.0, 260.0],
        "cookie_win_scroll",
        |ui, viewport_h| {
            ui.label(
                RichText::new(
                    "Set-Cookie headers from responses are captured automatically and sent on matching domain/path requests.",
                )
                .size(11.8)
                .color(pal.text_muted),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_cookie_domain)
                        .hint_text("domain (e.g. api.example.com)")
                        .desired_width(165.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_cookie_name)
                        .hint_text("cookie_name")
                        .desired_width(130.0),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut app.new_cookie_value)
                        .hint_text("cookie_value")
                        .desired_width(155.0),
                );
                if primary_button(
                    ui,
                    "+ Add Cookie",
                    !app.new_cookie_domain.trim().is_empty()
                        && !app.new_cookie_name.trim().is_empty(),
                )
                .clicked()
                {
                    app.cookie_jar.store_cookie(CookieEntry {
                        domain: app.new_cookie_domain.trim().to_ascii_lowercase(),
                        path: "/".into(),
                        name: app.new_cookie_name.trim().to_string(),
                        value: app.new_cookie_value.clone(),
                        secure: false,
                        http_only: false,
                        expires: None,
                    });
                    app.new_cookie_name.clear();
                    app.new_cookie_value.clear();
                    app.persist();
                }
            });

            ui.separator();
            let mut remove_idx = None;
            let mut clear_all = false;
            ui.horizontal(|ui| {
                badge_pill(
                    ui,
                    &format!("{} stored cookies", app.cookie_jar.entries.len()),
                    pal.accent_cyan,
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if !app.cookie_jar.entries.is_empty() && ui.small_button("Clear All").clicked() {
                        clear_all = true;
                    }
                });
            });

            let list_h = (viewport_h - 135.0).max(140.0);
            egui::ScrollArea::vertical().id_salt("cookie_list_inner").max_height(list_h).show(ui, |ui| {
                for (i, c) in app.cookie_jar.entries.iter().enumerate() {
                    Frame::none()
                        .fill(pal.bg_app)
                        .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                        .rounding(Rounding::same(6.0_f32))
                        .inner_margin(Margin::symmetric(10.0_f32, 6.0_f32))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                badge_pill(ui, &c.domain, pal.accent_primary);
                                ui.label(
                                    RichText::new(format!("{} = {}", c.name, c.value)).monospace(),
                                );
                                if c.secure {
                                    badge_pill(ui, "Secure", pal.accent_emerald);
                                }
                                if c.http_only {
                                    badge_pill(ui, "HttpOnly", pal.accent_amber);
                                }
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if ui.small_button("Delete").clicked() {
                                            remove_idx = Some(i);
                                        }
                                    },
                                );
                            });
                        });
                    ui.add_space(3.0);
                }
            });

            if let Some(i) = remove_idx {
                app.cookie_jar.entries.remove(i);
                app.persist();
            }
            if clear_all {
                app.cookie_jar.entries.clear();
                app.persist();
            }
        });
    app.cookie_modal_open = open;
}

fn render_runner_window(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.runner_open;

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Collection Runner & Data-Driven Testing",
        &mut open,
        [680.0, 500.0],
        [480.0, 280.0],
        "runner_win_scroll",
        |ui, viewport_h| {
            ui.horizontal(|ui| {
                ui.label("Collection:");
                let current_name = app
                    .collections
                    .get(app.runner_col_idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "None".into());
                egui::ComboBox::from_id_salt("runner_col_select")
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        for (i, c) in app.collections.iter().enumerate() {
                            ui.selectable_value(&mut app.runner_col_idx, i, &c.name);
                        }
                    });

                ui.label("Iterations:");
                ui.add(egui::DragValue::new(&mut app.runner_iterations).range(1..=500));

                ui.label("Delay (ms):");
                ui.add(egui::DragValue::new(&mut app.runner_delay_ms).range(0..=10_000));

                if emerald_button(
                    ui,
                    if app.runner_busy {
                        "Running..."
                    } else {
                        "▶  Run Collection"
                    },
                    !app.runner_busy && !app.collections.is_empty(),
                )
                .clicked()
                {
                    app.start_collection_run();
                }
            });

            ui.collapsing("Iteration Data (CSV or JSON Array — optional)", |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("Load CSV / JSON File...").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Data File", &["csv", "json"])
                            .pick_file()
                        {
                            if let Ok(raw) = std::fs::read_to_string(path) {
                                app.runner_data_text = raw;
                            }
                        }
                    }
                    ui.label(
                        RichText::new("Columns become {{variable}} values on each iteration.")
                            .size(11.2)
                            .color(pal.text_dim),
                    );
                });
                ui.add(
                    egui::TextEdit::multiline(&mut app.runner_data_text)
                        .code_editor()
                        .desired_rows(4)
                        .hint_text("userId,expectedStatus\n1,200\n2,200")
                        .desired_width(ui.available_width()),
                );
            });

            if let Some(err) = &app.runner_error {
                ui.colored_label(pal.accent_rose, err);
            }

            if let Some(rep) = &app.runner_report {
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    badge_pill(
                        ui,
                        &format!("{} requests", rep.total_requests),
                        pal.accent_primary,
                    );
                    badge_pill(
                        ui,
                        &format!("{} passed", rep.passed_assertions),
                        pal.accent_emerald,
                    );
                    if rep.failed_assertions > 0 || rep.failed_requests > 0 {
                        badge_pill(
                            ui,
                            &format!(
                                "{} failed ({} net err)",
                                rep.failed_assertions, rep.failed_requests
                            ),
                            pal.accent_rose,
                        );
                    }
                    badge_pill(ui, &format!("avg {} ms", rep.avg_ms), pal.accent_cyan);
                    badge_pill(ui, &format!("p95 {} ms", rep.p95_ms), pal.accent_violet);
                    badge_pill(
                        ui,
                        &format!("min {} / max {} ms", rep.min_ms, rep.max_ms),
                        pal.text_muted,
                    );
                });

                ui.add_space(4.0);
                let steps_h = (viewport_h - 145.0).max(160.0);
                egui::ScrollArea::vertical().id_salt("runner_steps_inner").max_height(steps_h).show(ui, |ui| {
                    for step in &rep.steps {
                        Frame::none()
                            .fill(pal.bg_app)
                            .stroke(Stroke::new(1.0_f32, pal.border_subtle))
                            .rounding(Rounding::same(6.0_f32))
                            .inner_margin(Margin::symmetric(10.0_f32, 6.0_f32))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    badge_pill(
                                        ui,
                                        &format!("Iter #{}", step.iteration),
                                        pal.text_dim,
                                    );
                                    method_badge(ui, step.method);
                                    ui.strong(&step.request_name);
                                    if let Some(code) = step.status {
                                        status_pill(ui, code, "");
                                    }
                                    ui.label(
                                        RichText::new(format!("{} ms", step.time_ms))
                                            .monospace()
                                            .size(11.0)
                                            .color(pal.text_muted),
                                    );
                                });
                                if let Some(err) = &step.error {
                                    ui.colored_label(pal.accent_rose, err);
                                }
                                for a in &step.assertions {
                                    let col = if a.passed {
                                        pal.accent_emerald
                                    } else {
                                        pal.accent_rose
                                    };
                                    ui.horizontal(|ui| {
                                        ui.colored_label(col, if a.passed { "✓" } else { "✕" });
                                        ui.label(&a.name);
                                        if let Some(e) = &a.error {
                                            ui.colored_label(col, e);
                                        }
                                    });
                                }
                            });
                        ui.add_space(4.0);
                    }
                });
            }
        });
    app.runner_open = open;
}

fn render_mock_window(app: &mut BapuApp, ctx: &egui::Context) {
    let pal = app.palette();
    let mut open = app.mock_open;

    crate::ui::widgets::show_resizable_window(
        ctx,
        "Local HTTP Mock Server",
        &mut open,
        [620.0, 420.0],
        [440.0, 260.0],
        "mock_win_scroll",
        |ui, viewport_h| {
            ui.label(
                RichText::new(
                    "Serves saved Examples from any collection on localhost (supports :param route matching and {{$dynamic}} variables).",
                )
                .size(11.8)
                .color(pal.text_muted),
            );
            ui.add_space(6.0);

            ui.horizontal(|ui| {
                ui.label("Collection:");
                let current_name = app
                    .collections
                    .get(app.mock_col_idx)
                    .map(|c| c.name.clone())
                    .unwrap_or_else(|| "None".into());
                egui::ComboBox::from_id_salt("mock_col_select")
                    .selected_text(current_name)
                    .show_ui(ui, |ui| {
                        for (i, c) in app.collections.iter().enumerate() {
                            ui.selectable_value(&mut app.mock_col_idx, i, &c.name);
                        }
                    });

                ui.label("Port:");
                ui.add(egui::TextEdit::singleline(&mut app.mock_port).desired_width(65.0));

                if let Some(ref handle) = app.mock_handle {
                    let base = handle.base_url();
                    badge_pill(ui, &format!("LIVE {base}"), pal.accent_emerald);
                    if ui.small_button("Copy URL").clicked() {
                        ui.output_mut(|o| o.copied_text = base);
                    }
                    if ui.button("Stop Server").clicked() {
                        app.stop_mock();
                    }
                } else if emerald_button(ui, "Start Mock Server", !app.collections.is_empty())
                    .clicked()
                {
                    app.start_mock();
                }
            });

            if let Some(err) = &app.mock_error {
                ui.colored_label(pal.accent_rose, err);
            }

            ui.separator();
            ui.horizontal(|ui| {
                ui.strong(format!("Incoming Mock Requests ({})", app.mock_hits.len()));
                if !app.mock_hits.is_empty() && ui.small_button("Clear Log").clicked() {
                    app.mock_hits.clear();
                }
            });
            let hits_h = (viewport_h - 135.0).max(140.0);
            egui::ScrollArea::vertical().id_salt("mock_hits_inner").max_height(hits_h).show(ui, |ui| {
                for hit in &app.mock_hits {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&hit.timestamp)
                                .monospace()
                                .size(11.0)
                                .color(pal.text_dim),
                        );
                        status_pill(ui, hit.status, "");
                        ui.label(
                            RichText::new(format!("{} {}", hit.method, hit.path))
                                .monospace()
                                .strong(),
                        );
                        if let Some(name) = &hit.matched_request {
                            ui.label(RichText::new(format!("→ {name}")).color(pal.text_muted));
                        }
                    });
                }
            });
        });
    app.mock_open = open;
}
