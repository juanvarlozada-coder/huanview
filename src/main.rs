// Huanview — minimalist fast image viewer inspired by Gwenview.
mod app;
mod lang;
mod loader;
mod updater;
mod util;

use std::path::Path;

/// Icono de la ventana/barra de tareas, generado desde `packaging/appimage/huanview.png`
/// y embebido en el binario para no depender de archivos externos.
fn window_icon() -> Option<egui::IconData> {
    let bytes = include_bytes!("../packaging/appimage/huanview.png");
    let img = image::load_from_memory(bytes).ok()?.to_rgba8();
    let (width, height) = (img.width(), img.height());
    Some(egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    })
}

fn main() {
    let arg = std::env::args().nth(1).unwrap_or_else(|| ".".into());
    let start = Path::new(&arg).to_path_buf();

    let mut viewport = eframe::egui::ViewportBuilder::default()
        .with_inner_size([1100.0, 720.0])
        .with_min_inner_size([520.0, 320.0])
        .with_title("Huanview");
    if let Some(icon) = window_icon() {
        viewport = viewport.with_icon(icon);
    }
    let opts = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "Huanview",
        opts,
        Box::new(|cc| Ok(app::HuanApp::new(cc, start))),
    )
    .expect("eframe run_native fell");
}
