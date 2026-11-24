mod app;
mod ui;

fn main() -> eframe::Result<()> {
    env_logger::init();

    // A multi-thread tokio runtime backs every async call this app makes (HTTP requests, DB
    // queries). `eframe::run_native` blocks the current (main) thread running the UI event
    // loop, so async work is spawned onto this runtime's worker threads and results are sent
    // back to the UI thread over a channel (see `app::BapuApp::poll_async_results`) rather than
    // ever `.await`-ed directly inside egui's synchronous `update()` callback.
    let runtime = tokio::runtime::Runtime::new().expect("failed to start the async runtime");
    let handle = runtime.handle().clone();

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1320.0, 860.0])
            .with_min_inner_size([960.0, 620.0])
            .with_title("Bapu Studio — Developer Cockpit"),
        ..Default::default()
    };

    eframe::run_native(
        "Bapu Studio",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::BapuApp::new(cc, handle)))),
    )

    // `runtime` is still alive here (not dropped until main() returns), so its worker threads
    // keep running for the whole lifetime of `run_native`'s blocking event loop above.
}
