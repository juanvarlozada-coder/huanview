// Huanview — minimalist fast image viewer inspired by Gwenview.
mod app;
mod loader;
mod util;

use std::path::Path;

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let start = Path::new(&arg).to_path_buf();

    let opts = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_inner_size([1100.0, 720.0])
            .with_min_inner_size([520.0, 320.0])
            .with_title("Huanview"),
        ..Default::default()
    };
    eframe::run_native(
        "Huanview",
        opts,
        Box::new(|cc| Ok(app::HuanApp::new(cc, start))),
    )
    .expect("eframe run_native fell");
}
