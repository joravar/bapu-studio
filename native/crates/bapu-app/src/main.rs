#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod ui;

fn load_app_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../assets/logo.png");
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (width, height) = img.dimensions();
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    env_logger::init();

    let runtime = tokio::runtime::Runtime::new().expect("failed to start the async runtime");
    let handle = runtime.handle().clone();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([1380.0, 880.0])
        .with_min_inner_size([1020.0, 660.0])
        .with_title("Bapu Studio — Developer Cockpit");

    if let Some(icon) = load_app_icon() {
        viewport = viewport.with_icon(icon);
    }

    let native_options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Bapu Studio",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::BapuApp::new(cc, handle)))),
    )
}
