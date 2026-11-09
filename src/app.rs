//! Huanview UI — Gwenview-inspired: single-canvas viewer, translucent HUD,
//! animated zoom/pan, thin bottom filmstrip, freehand annotation tools.

use eframe::egui;
use egui::{
    Align, Align2, Color32, FontId, Frame, Pos2, Rect, ScrollArea, Sense, Stroke, TextureHandle,
    TextureOptions, TopBottomPanel, Vec2, ViewportCommand, Window,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Instant;

use crate::lang::{self, Bindings, Lang};
use crate::loader::{Loaded, Loader};
use crate::updater::{
    self, apply_update_and_restart, check_for_update, download_destination, download_to,
    load_config, save_config, Release, UpdaterState,
};
use std::sync::atomic::AtomicU64;
use std::sync::{Arc, mpsc::{self, Receiver}};

enum UpdateMsg {
    Checked(Result<Option<Release>, String>),
    Downloaded(Result<PathBuf, String>),
}
use crate::util::collect_images;

const BG: Color32 = Color32::from_rgb(24, 24, 27);
const CHIP_BG: Color32 = Color32::from_rgba_premultiplied(24, 24, 28, 210);
const TEXT: Color32 = Color32::from_rgb(228, 228, 232);
const TEXT_DIM: Color32 = Color32::from_rgb(150, 150, 156);
const ACCENT: Color32 = Color32::from_rgb(93, 158, 251);
const FILM_H: f32 = 92.0;
const PEN_COLORS: [Color32; 5] = [
    Color32::from_rgb(93, 158, 251),
    Color32::from_rgb(255, 105, 97),
    Color32::from_rgb(90, 230, 140),
    Color32::from_rgb(255, 206, 84),
    Color32::from_rgb(246, 246, 246),
];

#[derive(Clone, Copy, PartialEq)]
enum Tool {
    Pan,
    Pen,
    Marker,
}

#[derive(Clone)]
struct Annotation {
    color: Color32,
    width: f32,
    alpha: f32,
    points: Vec<Pos2>, // normalized image coords (0..=1)
}

pub struct HuanApp {
    loader: Loader,
    files: Vec<PathBuf>,
    current: usize,
    full: HashMap<usize, TextureHandle>,
    dims: HashMap<usize, Vec2>,
    thumbs: HashMap<PathBuf, TextureHandle>,
    // animated view state
    zoom: f32,
    zoom_target: f32,
    offset: Vec2, // shown screen-space offset of the image center
    offset_target: Vec2,
    fade: f32, // 0..=1 fade-in on navigation
    // tools
    tool: Tool,
    draw_color: Color32,
    annotations: HashMap<PathBuf, Vec<Annotation>>,
    drawing: Option<Annotation>,
    // hud
    fullscreen: bool,
    last_fit: f32, // fit scale of the current image (for the 1:1 shortcut)
    toast: Option<(String, Instant)>,
    // update
    update: UpdaterState,
    update_rx: Option<Receiver<UpdateMsg>>,
    auto_check: bool,
    /// idioma de la interfaz
    lang: Lang,
    /// ventana de preferencias
    prefs_open: bool,
    prefs_cat: usize,
    // atajos
    bind: Bindings,
    /// acción esperando que el usuario pulse una tecla
    capture: Option<String>,
    // deshacer/rehacer anotaciones: (archivo, estado)
    undo_s: Vec<(PathBuf, Vec<Annotation>)>,
    redo_s: Vec<(PathBuf, Vec<Annotation>)>,
}

impl HuanApp {
    fn t(&self, key: &str) -> &'static str {
        lang::tr(self.lang, key)
    }

    pub fn new(cc: &eframe::CreationContext<'_>, start: PathBuf) -> Box<dyn eframe::App> {
        cc.egui_ctx.style_mut(|s| {
            s.visuals.dark_mode = true;
            s.visuals.panel_fill = BG;
            s.visuals.window_fill = BG;
            s.visuals.override_text_color = Some(TEXT);
        });
        let (files, current) = collect_images(&start);
        let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(6);
        let loader = Loader::new(workers);
        loader.reload(&files, current);
        // chequeo de actualizaciones (hilo aparte, silencioso)
        let update_rx = updater_start_check();
        let auto_check = load_config().auto_check;
        let lang = lang::load_lang();
        Box::new(HuanApp {
            loader,
            files,
            current,
            full: HashMap::new(),
            dims: HashMap::new(),
            thumbs: HashMap::new(),
            zoom: 1.0,
            zoom_target: 1.0,
            offset: Vec2::ZERO,
            offset_target: Vec2::ZERO,
            fade: 0.0,
            tool: Tool::Pan,
            draw_color: PEN_COLORS[0],
            annotations: HashMap::new(),
            drawing: None,
            fullscreen: false,
            last_fit: 1.0,
            toast: None,
            update: UpdaterState::default(),
            update_rx: Some(update_rx),
            auto_check,
            lang,
            prefs_open: false,
            prefs_cat: 0,
            bind: lang::load_settings(),
            capture: None,
            undo_s: Vec::new(),
            redo_s: Vec::new(),
        })
    }

    fn headline(&self) -> String {
        match (self.files.get(self.current), self.files.len()) {
            (Some(p), n) => format!(
                "{}  ·  {} de {n}  ·  Huanview",
                p.file_name().and_then(|s| s.to_str()).unwrap_or(""),
                self.current + 1
            ),
            _ => "Huanview".into(),
        }
    }

    // navigation --------------------------------------------------------------

    fn navigate(&mut self, delta: isize) {
        if self.files.is_empty() {
            return;
        }
        let n = self.files.len() as isize;
        let idx = ((self.current as isize + delta) % n + n) % n;
        self.go_to(idx as usize);
    }

    fn go_to(&mut self, idx: usize) {
        if idx == self.current || self.files.is_empty() || idx >= self.files.len() {
            return;
        }
        self.current = idx;
        self.zoom_target = 1.0;
        self.offset_target = Vec2::ZERO;
        self.fade = 0.0;
        self.full.retain(|&i, _| i.abs_diff(idx) <= 1);
        self.dims.retain(|&i, _| i.abs_diff(idx) <= 1);
        if self.thumbs.len() > 600 {
            self.thumbs.clear();
        }
        self.loader.cancel();
        self.loader.reload(&self.files, self.current);
    }

    fn apply_loaded(&mut self, ctx: &egui::Context, item: Loaded) {
        match item {
            Loaded::Full { index, image } => {
                self.dims.insert(index, Vec2::new(image.width() as f32, image.height() as f32));
                if let Some(mut t) = self.full.get(&index).cloned() {
                    t.set(image, TextureOptions::default());
                } else {
                    let t = ctx.load_texture(format!("huanimg{index}"), image, TextureOptions::default());
                    self.full.insert(index, t);
                }
            }
            Loaded::Thumb { path, image } => {
                if let Some(mut t) = self.thumbs.get(&path).cloned() {
                    t.set(image, TextureOptions::NEAREST);
                } else {
                    let t = ctx.load_texture(
                        format!("thumb:{}", path.display()),
                        image,
                        TextureOptions::NEAREST,
                    );
                    self.thumbs.insert(path, t);
                }
            }
            Loaded::Fail { index } => {
                if index == self.current {
                    let name = self
                        .files
                        .get(self.current)
                        .and_then(|p| p.file_name().and_then(|s| s.to_str()))
                        .unwrap_or("")
                        .to_string();
                    set_pending_toast(format!("No se pudo abrir: {name}"));
                }
            }
        }
    }
}

thread_local! {
    static PENDING_TOAST: std::cell::RefCell<Option<(String, Instant)>> = const { std::cell::RefCell::new(None) };
}
fn set_pending_toast(t: String) {
    PENDING_TOAST.with(|c| *c.borrow_mut() = Some((t, Instant::now())));
}
fn take_pending_toast() -> Option<(String, Instant)> {
    PENDING_TOAST.with_borrow_mut(std::mem::take)
}

impl eframe::App for HuanApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(self.headline()));

        for m in self.loader.poll() {
            self.apply_loaded(ctx, m);
        }
        self.poll_update(ctx);
        if let Some((t, _)) = take_pending_toast() {
            self.toast = Some((t, Instant::now()));
        }
        self.toast = match &self.toast {
            Some(t) if t.1.elapsed().as_secs_f64() < 2.0 => Some(t.clone()),
            _ => None,
        };

        // bounded lerp playback: smooth, no spin
        self.fade = (self.fade + 0.14).min(1.0);
        let k = 0.22;
        self.zoom += (self.zoom_target - self.zoom) * k;
        self.offset += (self.offset_target - self.offset) * k;
        if (self.zoom_target - self.zoom).abs() < 0.002 {
            self.zoom = self.zoom_target;
        }
        if (self.offset_target - self.offset).length() < 0.7 {
            self.offset = self.offset_target;
        }

        self.keys(ctx);

        TopBottomPanel::bottom("filmstrip")
            .frame(Frame::none().fill(BG).inner_margin(egui::Margin::symmetric(6.0, 4.0)))
            .exact_height(FILM_H)
            .show(ctx, |ui| self.filmstrip(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            self.canvas(ctx, ui);
        });

        self.top_bar(ctx);
        self.prefs_window(ctx);
        self.zoom_chip(ctx);
        self.hud_toast(ctx);

        ctx.request_repaint();
    }
}

// ------------------------------------------------- top bar + zoom chip

#[derive(Clone, Copy, PartialEq)]
enum Ic {
    Pan,
    Pen,
    Marker,
    Erase,
    Open,
    Save,
    SaveAs,
    ZoomIn,
    ZoomOut,
    Fit,
    Gear,
    Refresh,
}

impl HuanApp {
    /// Botón con icono vectorial dibujado a mano (sin depender de fuentes).
    fn icon_button(&self, ui: &mut egui::Ui, ic: Ic, tip: &str) -> egui::Response {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 5.0, Color32::from_rgba_premultiplied(255, 255, 255, 14));
        }
        let fg = if resp.hovered() { ACCENT } else { TEXT_DIM };
        let thick = Stroke::new(2.6_f32, fg);
        let mid = Stroke::new(1.8_f32, fg);
        let thin = Stroke::new(1.3_f32, fg);
        let c = rect.center();
        let v = egui::vec2;
        match ic {
            Ic::Pan => {
                p.line_segment([c - v(8.0, 0.0), c + v(8.0, 0.0)], mid);
                p.line_segment([c - v(0.0, 8.0), c + v(0.0, 8.0)], mid);
                for dx in [-1.0f32, 1.0] {
                    let tip = c + v(dx * 9.5, 0.0);
                    p.line_segment([tip, tip + v(-dx * 3.0, -2.5)], thin);
                    p.line_segment([tip, tip + v(-dx * 3.0, 2.5)], thin);
                }
                for dy in [-1.0f32, 1.0] {
                    let tip = c + v(0.0, dy * 9.5);
                    p.line_segment([tip, tip + v(-2.5, -dy * 3.0)], thin);
                    p.line_segment([tip, tip + v(2.5, -dy * 3.0)], thin);
                }
            }
            Ic::Pen => {
                p.line_segment([c + v(-3.5, 3.5), c + v(5.5, -5.5)], thick);
                p.circle_filled(c + v(-5.5, 5.5), 1.6, fg);
            }
            Ic::Marker => {
                p.line_segment([c + v(-2.5, 2.5), c + v(4.5, -4.5)], thick);
                p.rect_filled(
                    Rect::from_center_size(c + v(6.0, -6.0), Vec2::splat(4.0)),
                    1.0,
                    fg,
                );
            }
            Ic::Erase => {
                p.line_segment([c + v(-4.0, 4.0), c + v(4.5, -4.5)], Stroke::new(3.5_f32, fg));
            }
            Ic::Open => {
                // carpeta
                p.rect_stroke(
                    Rect::from_min_max(c + v(-8.0, -4.0), c + v(8.0, 6.5)),
                    1.5,
                    thin,
                );
                p.line_segment([c + v(-8.0, -4.0), c + v(-5.5, -7.0)], mid);
                p.line_segment([c + v(-5.5, -7.0), c + v(0.0, -7.0)], mid);
                p.line_segment([c + v(0.0, -7.0), c + v(0.0, -4.0)], mid);
            }
            Ic::Save | Ic::SaveAs => {
                p.rect_stroke(
                    Rect::from_min_max(c + v(-7.5, -7.5), c + v(7.5, 7.5)),
                    1.5,
                    thin,
                );
                p.rect_filled(
                    Rect::from_min_max(c + v(1.0, -7.5), c + v(5.5, -3.5)),
                    0.5,
                    fg,
                );
                p.rect_stroke(
                    Rect::from_min_max(c + v(-5.0, -0.5), c + v(5.0, 6.0)),
                    1.0,
                    thin,
                );
                if ic == Ic::SaveAs {
                    p.line_segment([c + v(5.0, 5.0), c + v(9.0, 9.0)], mid);
                    p.line_segment([c + v(7.0, 7.0), c + v(7.0, 7.0)], mid);
                    p.line_segment([c + v(4.8, 7.0), c + v(9.2, 7.0)], mid);
                    p.line_segment([c + v(7.0, 4.8), c + v(7.0, 9.2)], mid);
                }
            }
            Ic::ZoomIn | Ic::ZoomOut => {
                p.circle_stroke(c + v(-2.0, -2.0), 5.0, mid);
                p.line_segment([c + v(1.8, 1.8), c + v(7.5, 7.5)], thick);
                p.line_segment([c + v(-4.5, -2.0), c + v(0.5, -2.0)], mid);
                if ic == Ic::ZoomIn {
                    p.line_segment([c + v(-2.0, -4.5), c + v(-2.0, 0.5)], mid);
                }
            }
            Ic::Fit => {
                p.rect_stroke(
                    Rect::from_min_max(c + v(-7.0, -5.0), c + v(7.0, 5.0)),
                    1.5,
                    thin,
                );
                p.line_segment([c + v(3.5, -3.0), c + v(-3.5, 3.0)], mid);
                let tip = c + v(3.5, -3.0);
                p.line_segment([tip, tip + v(-2.5, 0.4)], thin);
                p.line_segment([tip, tip + v(0.4, -2.5)], thin);
            }
            Ic::Gear => {
                p.circle_stroke(c, 3.6, mid);
                for k in 0..6 {
                    let a = k as f32 * std::f32::consts::TAU / 6.0 + 0.26;
                    let d = v(a.cos(), a.sin());
                    p.line_segment([c + d * 5.2, c + d * 8.0], mid);
                }
            }
            Ic::Refresh => {
                p.circle_stroke(c, 6.5, mid);
                // punta de flecha arriba-derecha (indicación de rotación)
                let tri = [
                    c + v(3.2, -7.4),
                    c + v(7.8, -5.4),
                    c + v(5.4, -2.2),
                ];
                p.add(egui::Shape::convex_polygon(tri.to_vec(), fg, Stroke::NONE));
            }
        }
        resp.on_hover_text(tip.to_string())
    }

    /// Barra pequeña pegada arriba: dibujo, archivos y configuración.
    fn top_bar(&mut self, ctx: &egui::Context) {
        let name = self
            .files
            .get(self.current)
            .and_then(|p| p.file_name().and_then(|s| s.to_str()))
            .unwrap_or(lang::tr(self.lang, "no_images"))
            .to_string();
        let counter = if self.files.is_empty() {
            String::new()
        } else {
            format!("{} / {}", self.current + 1, self.files.len())
        };

        TopBottomPanel::top("htop")
            .frame(Frame::none().fill(CHIP_BG).inner_margin(egui::Margin::symmetric(10.0, 3.0)))
            .resizable(false)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(name).font(FontId::proportional(12.0)),
                    );
                    if !counter.is_empty() {
                        ui.label(egui::RichText::new(counter).small().color(TEXT_DIM));
                    }

                    ui.separator();
                    // —— herramientas de dibujo (iconos) ——
                    if self.icon_button(ui, Ic::Pan, self.t("view")).clicked() {
                        self.tool = Tool::Pan;
                    }
                    if self.icon_button(ui, Ic::Pen, self.t("pen")).clicked() {
                        self.tool = Tool::Pen;
                    }
                    if self.icon_button(ui, Ic::Marker, self.t("marker")).clicked() {
                        self.tool = Tool::Marker;
                    }
                    ui.add_space(2.0);
                    for (ci, col) in PEN_COLORS.iter().enumerate() {
                        self.color_button(ui, *col, ci);
                    }
                    if self.icon_button(ui, Ic::Erase, self.t("erase")).clicked() {
                        self.clear_annotations();
                    }

                    ui.separator();
                    // —— archivo (iconos) ——
                    if self.icon_button(ui, Ic::Open, self.t("open")).clicked() {
                        self.open_file_dialog();
                    }
                    if self.icon_button(ui, Ic::Save, self.t("save")).clicked() {
                        self.n_export_ann();
                    }
                    if self.icon_button(ui, Ic::SaveAs, self.t("save_as")).clicked() {
                        self.export_annotated_png();
                    }

                    ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
                        // —— actualización disponible ——
                        if let Some(rel) = self.update.available.clone() {
                            ui.label(
                                egui::RichText::new(format!("v{} {}", rel.version, self.t("available")))
                                    .strong()
                                    .color(ACCENT),
                            );
                            if self.update.progress.is_none() && gui_button(ui, self.t("install")) {
                                self.start_update_download();
                            }
                            if self.update.progress.is_some() {
                                let frac =
                                    (self.update.progress_pct as f32 / 100.0).clamp(0.0, 1.0);
                                ui.add(
                                    egui::ProgressBar::new(frac)
                                        .desired_width(80.0)
                                        .show_percentage(),
                                );
                            }
                            ui.separator();
                        }
                        // —— buscar actualizaciones manualmente ——
                        if self.icon_button(ui, Ic::Refresh, self.t("check_now")).clicked() {
                            self.update.available = None;
                            self.update_rx = Some(updater_manual_check());
                            set_pending_toast(self.t("checking").to_string());
                        }
                        // —— preferencias (ventana) ——
                        if self.icon_button(ui, Ic::Gear, self.t("prefs")).clicked() {
                            self.prefs_open = true;
                        }
                    });
                });
            });
    }

    /// Ventana de preferencias con categorías (en lugar del desplegable).
    fn prefs_window(&mut self, ctx: &egui::Context) {
        if !self.prefs_open {
            return;
        }
        let mut open = self.prefs_open;
        let resp_window = Window::new(self.t("prefs"))
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.set_min_width(540.0);
                ui.horizontal(|ui| {
                    // —— columna de categorías ——
                    ui.vertical(|ui| {
                        let cats: [(&str, usize); 4] = [
                            ("🗂", 0),
                            ("🔄", 1),
                            ("⌨", 3),
                            ("ℹ️", 2),
                        ];
                        for (icon, cat) in cats {
                            let name = match cat {
                                0 => self.t("cat_general"),
                                1 => self.t("cat_updates"),
                                3 => self.t("cat_shortcuts"),
                                _ => self.t("cat_about"),
                            };
                            let sel = self.prefs_cat == cat;
                            let fill = if sel {
                                Color32::from_rgba_premultiplied(93, 158, 251, 55)
                            } else {
                                Color32::TRANSPARENT
                            };
                            let txt = format!("  {icon} {name}  ");
                            let b = egui::Button::new(
                                egui::RichText::new(txt).size(13.0),
                            )
                            .fill(fill)
                            .min_size(Vec2::new(140.0, 24.0));
                            if ui.add(b).clicked() {
                                self.prefs_cat = cat;
                            }
                        }
                    });
                    ui.separator();

                    // —— contenido según categoría ——
                    ui.vertical(|ui| {
                        match self.prefs_cat {
                            0 => {
                                // ————————————————— General
                                ui.label(egui::RichText::new(self.t("language")).small().color(TEXT_DIM));
                                ui.add_space(2.0);
                                egui::ComboBox::from_id_source("langsel")
                                    .selected_text(
                                        lang::LANGS
                                            .iter()
                                            .find(|(_, l)| *l == self.lang)
                                            .map(|(n, _)| *n)
                                            .unwrap_or("Español"),
                                    )
                                    .width(180.0)
                                    .show_ui(ui, |ui| {
                                        for (name, l) in lang::LANGS {
                                            let sel = ui.selectable_value(
                                                &mut self.lang,
                                                l,
                                                name,
                                            );
                                            if sel.changed() {
                                                lang::save_lang(self.lang);
                                            }
                                        }
                                    });
                            }
                            1 => {
                                // ————————————————— Actualizaciones
                                let check_txt = self.t("check_updates");
                                if ui.checkbox(&mut self.auto_check, check_txt).changed() {
                                    save_config(&updater::UpdaterConfig {
                                        auto_check: self.auto_check,
                                        last_check: None,
                                    });
                                }
                                ui.add_space(6.0);
                                if ui.button(self.t("check_now")).clicked() {
                                    self.update.available = None;
                                    self.update_rx = Some(updater_manual_check());
                                    set_pending_toast(self.t("checking").to_string());
                                }
                                if let Some(rel) = self.update.available.clone() {
                                    ui.add_space(6.0);
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "v{} {}",
                                            rel.version,
                                            self.t("available")
                                        ))
                                        .strong()
                                        .color(ACCENT),
                                    );
                                    if self.update.progress.is_none()
                                        && ui.button(self.t("install")).clicked()
                                    {
                                        self.start_update_download();
                                    }
                                    if self.update.progress.is_some() {
                                        let frac =
                                            (self.update.progress_pct as f32 / 100.0)
                                                .clamp(0.0, 1.0);
                                        ui.add(
                                            egui::ProgressBar::new(frac)
                                                .desired_width(120.0)
                                                .show_percentage(),
                                        );
                                    }
                                }
                            }
                            3 => {
                                // ————————————————— Atajos de teclado
                                ui.label(
                                    egui::RichText::new(self.t("shortcuts_hint"))
                                        .small()
                                        .color(TEXT_DIM),
                                );
                                ui.add_space(4.0);
                                ScrollArea::vertical().max_height(240.0).show(ui, |ui| {
                                    ui.set_width(420.0);
                                    let actions = [
                                        ("copy", self.t("act_copy")),
                                        ("undo", self.t("act_undo")),
                                        ("redo", self.t("act_redo")),
                                        ("pan", self.t("act_pan")),
                                        ("pen", self.t("act_pen")),
                                        ("marker", self.t("act_marker")),
                                        ("settings", self.t("act_settings")),
                                    ];
                                    egui::Grid::new("bindgrid")
                                        .num_columns(2)
                                        .spacing([12.0, 4.0])
                                        .show(ui, |ui| {
                                            for (act, label) in actions {
                                                ui.label(label);
                                                let current = if let Some(cap) = &self.capture {
                                                    if cap == act {
                                                        format!("␣  {}", self.t("capture_prompt"))
                                                    } else {
                                                        self.bind.get(act).cloned().unwrap_or_default()
                                                    }
                                                } else {
                                                    self.bind.get(act).cloned().unwrap_or_default()
                                                };
                                                let txt = if current.is_empty() {
                                                    "—".to_string()
                                                } else {
                                                    current.replace("Ctrl+", "Ctrl+").to_string()
                                                };
                                                let accent = self.capture.as_deref() == Some(act);
                                                let rt = egui::RichText::new(txt)
                                                    .color(if accent { ACCENT } else { TEXT });
                                                let rt = if accent { rt.strong() } else { rt };
                                                let b = egui::Button::new(rt);
                                                if ui.add(b).clicked() {
                                                    self.capture = Some(act.to_string());
                                                }
                                                ui.end_row();
                                            }
                                        });
                                    ui.add_space(4.0);
                                    ui.label(
                                        egui::RichText::new(self.t("hand_note")).small().color(TEXT_DIM),
                                    );
                                });
                            }
                            _ => {
                                // ————————————————— Acerca de
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} v{}",
                                        self.t("about_label"),
                                        env!("CARGO_PKG_VERSION")
                                    ))
                                    .strong(),
                                );
                                ui.label(
                                    egui::RichText::new(self.t("about_sub"))
                                        .small()
                                        .color(TEXT_DIM),
                                );
                                ui.separator();
                                ui.label(egui::RichText::new(self.t("whats_new")).strong());
                                ScrollArea::vertical()
                                    .max_height(220.0)
                                    .show(ui, |ui| {
                                        ui.add(
                                            egui::TextEdit::multiline(
                                                &mut include_str!("../CHANGELOG.md").to_string(),
                                            )
                                            .interactive(false)
                                            .desired_width(430.0)
                                            .font(FontId::proportional(11.0)),
                                        );
                                    });
                            }
                        }
                    });
                });
            });
        self.prefs_open = open && resp_window.is_some();
    }

    /// Chip de zoom esquina inferior derecha (encima del filmstrip).
    fn zoom_chip(&mut self, ctx: &egui::Context) {
        let zoom_pct = if self.zoom_target >= 1.0 {
            format!("{} %", (self.zoom * 100.0).round())
        } else {
            format!("{:.1} %", self.zoom * 100.0)
        };
        egui::Area::new("hzoom".into())
            .order(egui::Order::Foreground)
            .anchor(egui::Align2::RIGHT_BOTTOM, [-14.0, -(FILM_H + 14.0)])
            .show(ctx, |ui| {
                Frame::window(&ctx.style())
                    .fill(CHIP_BG)
                    .inner_margin(egui::Margin::symmetric(6.0, 2.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if self.icon_button(ui, Ic::ZoomIn, self.t("zoom_in")).clicked() {
                                self.set_zoom_target(self.zoom_target * 1.3);
                            }
                            ui.label(
                                egui::RichText::new(zoom_pct)
                                    .small()
                                    .color(if self.zoom_target <= 1.001 {
                                        TEXT_DIM
                                    } else {
                                        ACCENT
                                    }),
                            );
                            if self.icon_button(ui, Ic::ZoomOut, self.t("zoom_out")).clicked() {
                                self.set_zoom_target(self.zoom_target / 1.3);
                            }
                            if self.icon_button(ui, Ic::Fit, self.t("fit")).clicked() {
                                self.zoom_target = 1.0;
                                self.offset_target = Vec2::ZERO;
                            }
                        });
                    });
            });
    }

    fn color_button(&mut self, ui: &mut egui::Ui, color: Color32, which: usize) {
        let (rect, resp) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::click());
        let sel = self.draw_color == color && matches!(self.tool, Tool::Pen | Tool::Marker);
        ui.painter().rect_filled(rect, 9.0, color);
        if sel {
            ui.painter().rect_stroke(rect.shrink(1.5), 9.0, Stroke::new(2.0_f32, TEXT));
        }
        if resp.clicked() {
            self.draw_color = color;
            self.draw_color_bump(which);
        }
    }

    fn draw_color_bump(&mut self, _which: usize) {}

    fn clear_annotations(&mut self) {
        let p = self.files.get(self.current).cloned();
        if let Some(p) = p {
            if self.annotations.contains_key(&p) {
                self.push_undo();
                self.annotations.remove(&p);
            }
        }
    }

    /// Abrir otra imagen con diálogo nativo (rfd): carga la carpeta de esa imagen.
    fn open_file_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new()
            .add_filter("Imágenes", &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico"])
            .pick_file()
        {
            let start = if p.is_file() {
                p.parent().map(|d| d.join(p.file_name().unwrap_or_default())).clone().unwrap_or_else(|| p.clone())
            } else {
                p.clone()
            };
            let (files, current) = collect_images(&p);
            if files.is_empty() {
                set_pending_toast(self.t("no_images_folder").to_string());
                return;
            }
            let _ = start;
            let workers =
                std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4).min(6);
            let loader = Loader::new(workers);
            loader.reload(&files, current);
            self.loader = loader;
            self.files = files;
            self.current = current;
            self.full.clear();
            self.dims.clear();
            self.thumbs.clear();
            self.zoom_target = 1.0;
            self.zoom = 1.0;
            self.offset_target = Vec2::ZERO;
            self.offset = Vec2::ZERO;
            self.fade = 0.0;
        }
    }

    /// Guardar como: exporta la imagen actual con las anotaciones incrustadas (PNG).
    fn export_annotated_png(&self) {
        let Some(path) = self.files.get(self.current) else {
            return;
        };
        let Some(save_to) = rfd::FileDialog::new()
            .set_file_name(format!(
                "{}_anotado.png",
                path.file_stem().and_then(|s| s.to_str()).unwrap_or("imagen")
            ))
            .add_filter("PNG", &["png"])
            .save_file()
        else {
            return;
        };
        match self.bake_annotated(path) {
            Ok(rgba) => {
                if rgba.save(&save_to).is_ok() {
                    set_pending_toast(format!("{} {}", self.t("saved"), save_to.display()));
                } else {
                    set_pending_toast(self.t("png_failed").to_string());
                }
            }
            Err(e) => set_pending_toast(format!("Error al exportar: {e}")),
        }
    }

    /// Rasteriza la imagen + trazos normalizados (alpha y grosor en px de la IMAGEN).
    fn bake_annotated(
        &self,
        path: &PathBuf,
    ) -> Result<image::RgbaImage, String> {
        let mut img = image::open(path)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        let (iw, ih) = (img.width() as f32, img.height() as f32);
        let stroke_list = self.annotations.get(path).cloned().unwrap_or_default();
        for a in stroke_list {
            let color = image::Rgba([a.color.r(), a.color.g(), a.color.b(), a.color.a()]);
            let alpha = a.alpha.clamp(0.05, 1.0);
            let lw = (a.width * (iw.min(ih) / 900.0).max(0.5)).max(1.0);
            let mut stamped: std::collections::HashSet<(u32, u32)> = std::collections::HashSet::new();
            for pts in a.points.windows(2) {
                let (x0, y0) = (pts[0].x * iw, pts[0].y * ih);
                let (x1, y1) = (pts[1].x * iw, pts[1].y * ih);
                let steps = ((x1 - x0).hypot(y1 - y0) / (lw * 0.35).max(0.6)).ceil() as i32;
                for s in 0..=steps {
                    let t = if steps == 0 { 0.0 } else { s as f32 / steps as f32 };
                    let (x, y) = (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t);
                    let rx = ((x - lw / 2.0) as i32)..=((x + lw / 2.0) as i32);
                    let ry = ((y - lw / 2.0) as i32)..=((y + lw / 2.0) as i32);
                    for py in ry.clone() {
                        for px in rx.clone() {
                            if px < 0 || py < 0 || px >= img.width() as i32 || py >= img.height() as i32 {
                                continue;
                            }
                            let (px, py) = (px as u32, py as u32);
                            if !stamped.insert((px, py)) {
                                continue; // el mismo trazo no se mezcla dos veces
                            }
                            let p = img.get_pixel(px, py);
                            let out = [
                                (p[0] as f32 * (1.0 - alpha) + color[0] as f32 * alpha) as u8,
                                (p[1] as f32 * (1.0 - alpha) + color[1] as f32 * alpha) as u8,
                                (p[2] as f32 * (1.0 - alpha) + color[2] as f32 * alpha) as u8,
                                255,
                            ];
                            img.get_pixel_mut(px, py).0 = out;
                        }
                    }
                }
            }
        }
        Ok(img)
    }
}

fn gui_button(ui: &mut egui::Ui, label: &str) -> bool {
    let b = egui::Button::new(egui::RichText::new(label).size(14.0))
        .fill(Color32::TRANSPARENT)
        .min_size(Vec2::new(0.0, 22.0));
    ui.add(b).clicked()
}

// --------------------------------------------------------------- TOAST chips

impl HuanApp {
    fn hud_toast(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = self.toast.clone() else {
            return;
        };
        let age = at.elapsed().as_secs_f64();
        let alpha = ((2.0f64 - age).clamp(0.0, 1.0) * 255.0) as u8;
        let chip = Color32::from_rgba_premultiplied(24, 24, 28, alpha);
        let text = Color32::from_rgba_premultiplied(228, 228, 232, alpha);
        let screen = ctx.screen_rect();
        let size = Vec2::new(420.0, 34.0);
        let rect = Rect::from_center_size(
            Pos2::new(screen.center().x, screen.bottom() - FILM_H - 46.0),
            size,
        );
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            "toast".into(),
        ));
        painter.rect_filled(rect, 12.0, chip);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            &msg,
            FontId::proportional(13.0),
            text,
        );
    }
}

// ---------------------------------------------------------------- canvas UI

impl HuanApp {
    fn canvas(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) {
        let (rect, resp) = ui.allocate_exact_size(ui.available_size(), Sense::click_and_drag());
        let avail = rect.shrink(14.0);

        let img = self.dims.get(&self.current).copied().unwrap_or(Vec2::splat(1.0));
        let fit_scale = if img.x > 1.0 && img.y > 1.0 {
            let s = (avail.width() / img.x).min(avail.height() / img.y);
            if s.is_finite() && s > 0.0 {
                s
            } else {
                1.0
            }
        } else {
            1.0
        };
        let scale = fit_scale * self.zoom;
        self.last_fit = fit_scale;
        let shown = img * scale;
        let avail_half = Vec2::new(avail.width() * 0.5, avail.height() * 0.5);
        lim_check(&mut self.offset_target, shown, avail_half);
        lim_check(&mut self.offset, shown, avail_half);
        let center = avail.center() + self.offset;
        let image_rect = Rect::from_center_size(center, shown);

        // image (fade-in tint)
        if let Some(tex) = self.full.get(&self.current) {
            let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            let a = (self.fade * self.fade).min(1.0);
            let tint = Color32::WHITE.gamma_multiply(a);
            ui.painter().image(tex.id(), image_rect, uv, tint);
            self.draw_annotations(ui, image_rect);
        } else {
            ui.painter().text(
                avail.center(),
                Align2::CENTER_CENTER,
                crate::lang::tr(self.lang, "loading"),
                FontId::proportional(14.0),
                TEXT_DIM,
            );
        }

        // ---- wheel: zoom anchored at cursor
        let scroll = ctx.input(|i| i.smooth_scroll_delta);
        if scroll.y.abs() > 0.0 && resp.hovered() {
            let old = self.zoom_target;
            let new = (old * (1.0 + scroll.y / 450.0)).clamp(0.05, 40.0);
            if let Some(cursor) = ctx.input(|i| i.pointer.hover_pos()) {
                self.zoom_at(cursor, old, new, avail, fit_scale);
            }
            self.set_zoom_target(new);
        }
        if scroll.x.abs() > 0.0 && resp.hovered() {
            self.offset_target.x += scroll.x * 2.0;
        }

        // ---- double-click: toggle fit <-> 2.5x
        let dbl_click = ctx.input(|i| i.pointer.button_double_clicked(egui::PointerButton::Primary));
        if dbl_click && resp.hovered() {
            if self.zoom_target > 1.2 {
                self.zoom_target = 1.0;
                self.offset_target = Vec2::ZERO;
            } else if let Some(c) = ctx.input(|i| i.pointer.hover_pos()) {
                self.zoom_at(c, self.zoom_target, 2.5, avail, fit_scale);
                self.set_zoom_target(2.5);
            }
        }

        // ---- click-to-navigate: grandes flechas al hover + zonas laterales
        let fits_fully = self.zoom_target <= 1.001 && shown.x <= avail.size().x + 1.5;
        let arrow_l =
            Rect::from_center_size(Pos2::new(rect.left() + 38.0, rect.center().y), Vec2::splat(56.0));
        let arrow_r =
            Rect::from_center_size(Pos2::new(rect.right() - 38.0, rect.center().y), Vec2::splat(56.0));
        if self.files.len() > 1 && resp.hovered() {
            let painter = ui.painter();
            let (gl, gr) = (&"‹", &"›");
            for (r, g, dim) in
                [(arrow_l, gl, self.current == 0), (arrow_r, gr, self.current + 1 == self.files.len())]
            {
                let a = if dim { 60 } else { 165 };
                let bg = Color32::from_rgba_premultiplied(24, 24, 28, a);
                let fg = Color32::from_rgba_premultiplied(228, 228, 232, a);
                painter.circle_filled(r.center(), 27.0, bg);
                painter.text(
                    r.center(),
                    Align2::CENTER_CENTER,
                    g,
                    FontId::proportional(30.0),
                    fg,
                );
            }
        }
        if !resp.dragged() && !resp.drag_started() && resp.clicked() {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                if arrow_l.contains(pos) {
                    self.navigate(-1);
                } else if arrow_r.contains(pos) {
                    self.navigate(1);
                } else if fits_fully {
                    let t = (pos.x - rect.left()) / rect.width();
                    if t < 0.2 {
                        self.navigate(-1);
                    } else if t > 0.8 {
                        self.navigate(1);
                    }
                }
            }
        }

        // ---- drag: pan (zero latency) or draw annotation
        // mantener Espacio = mano temporal, sin importar la herramienta
        let space_held = ctx.input(|i| i.key_down(egui::Key::Space));
        match if space_held { Tool::Pan } else { self.tool } {
            Tool::Pan => {
                if resp.dragged() {
                    self.offset_target += resp.drag_delta();
                    self.offset = self.offset_target;
                }
            }
            Tool::Pen | Tool::Marker => {
                if resp.drag_started() {
                    self.drawing = Some(Annotation {
                        color: self.draw_color,
                        width: if self.tool == Tool::Marker { 26.0 } else { 3.5 },
                        alpha: if self.tool == Tool::Marker { 0.45 } else { 1.0 },
                        points: Vec::new(),
                    });
                }
                if resp.dragged() {
                    if let Some(p) = ctx.input(|i| i.pointer.interact_pos()) {
                        let n = norm_pt(p, image_rect);
                        if let Some(d) = self.drawing.as_mut() {
                            let push = match d.points.last() {
                                Some(last) => {
                                    (egui::vec2(last.x, last.y) - egui::vec2(n.x, n.y)).length()
                                        > 0.012
                                }
                                None => true,
                            };
                            if push {
                                d.points.push(n);
                            }
                        }
                    }
                }
                if resp.drag_stopped() {
                    if let Some(d) = self.drawing.take() {
                        if d.points.len() >= 2 {
                            self.push_undo();
                            if let Some(path) = self.files.get(self.current).cloned() {
                                self.annotations.entry(path).or_default().push(d);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Cursor-anchored zoom (like Gwenview): image point under cursor stays put.
    fn zoom_at(&mut self, cursor: Pos2, old_z: f32, new_z: f32, avail: Rect, fit: f32) {
        let center = avail.center() + self.offset_target;
        let scn = fit * old_z;
        let content_p = (cursor - center) / scn.max(0.05);
        let new_center = cursor - content_p * (fit * new_z);
        self.offset_target = new_center - avail.center();
    }

    fn set_zoom_target(&mut self, t: f32) {
        self.zoom_target = t.clamp(0.05, 40.0);
    }

    fn draw_annotations(&self, ui: &egui::Ui, r: Rect) {
        let painter = ui.painter();
        if let Some(path) = self.files.get(self.current) {
            if let Some(list) = self.annotations.get(path) {
                for a in list {
                    paint_stroke(painter, r, a);
                }
            }
        }
        if let Some(d) = &self.drawing {
            paint_stroke(painter, r, d);
        }
    }
}


fn lim_check(v: &mut Vec2, shown: Vec2, avail_half: Vec2) {
    // allow some slack when panning a large image; keep small ones centered
    let l = (shown * 0.5 - avail_half).max(Vec2::ZERO) + Vec2::splat(40.0);
    *v = v.clamp(-l, l);
}

fn norm_pt(p: Pos2, r: Rect) -> Pos2 {
    Pos2::new(
        ((p.x - r.left()) / r.width().max(1.0)).clamp(0.0, 1.0),
        ((p.y - r.top()) / r.height().max(1.0)).clamp(0.0, 1.0),
    )
}

fn paint_stroke(painter: &egui::Painter, r: Rect, a: &Annotation) {
    let pts: Vec<Pos2> = a
        .points
        .iter()
        .map(|p| Pos2::new(r.left() + p.x * r.width(), r.top() + p.y * r.height()))
        .collect();
    let color = Color32::from_rgba_unmultiplied(
        a.color.r(),
        a.color.g(),
        a.color.b(),
        (a.alpha * 255.0) as u8,
    );
    painter.add(egui::Shape::line(pts, Stroke::new(a.width, color)));
}

// ---------------------------------------------------------------- filmstrip

impl HuanApp {
    fn filmstrip(&mut self, ui: &mut egui::Ui) {
        ScrollArea::horizontal()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    for (i, p) in self.files.clone().iter().enumerate() {
                        let selected = i == self.current;
                        let tex = self.thumbs.get(p).map(|t| (t.id(), t.size_vec2()));
                        let (rect, resp) =
                            ui.allocate_exact_size(THUMB_SIZE, Sense::click());
                        ui.painter().rect_filled(rect, 6.0, FRAME);
                        if let Some((id, size)) = tex {
                            let r = fit_inside(size, rect.size());
                            let dst = Rect::from_center_size(rect.center(), r);
                            ui.painter().image(
                                id,
                                dst,
                                Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                                Color32::WHITE,
                            );
                        } else {
                            ui.painter().text(
                                rect.center(),
                                Align2::CENTER_CENTER,
                                "…",
                                FontId::proportional(14.0),
                                TEXT_DIM,
                            );
                        }
                        if selected {
                            // trazado interior para que el ScrollArea no lo recorte
                            ui.painter().rect_stroke(rect.shrink(1.0), 6.0, Stroke::new(2.0_f32, ACCENT));
                            resp.scroll_to_me(Some(Align::Center));
                        }
                        if resp.clicked() && i != self.current {
                            self.go_to(i);
                        }
                    }
                });
            });
    }
}

const THUMB_SIZE: Vec2 = Vec2::new(130.0, 76.0);
const FRAME: Color32 = Color32::from_rgb(32, 32, 36);

fn fit_inside(src: Vec2, dst: Vec2) -> Vec2 {
    let k = (dst.x / src.x.max(1.0)).min(dst.y / src.y.max(1.0));
    src * k.min(1.0)
}

// ---------------------------------------------------------------- shortcuts

impl HuanApp {
    fn keys(&mut self, ctx: &egui::Context) {
        ctx.input(|i| {
            use egui::Key;
            // ---------- captura de atajo en curso ----------
            if let Some(act) = self.capture.clone() {
                for ev in i.events.iter() {
                    if let egui::Event::Key {
                        key,
                        pressed,
                        modifiers: evmods,
                        ..
                    } = ev
                    {
                        if !pressed {
                            continue;
                        }
                        if *key == egui::Key::Escape && !evmods.any() {
                            self.capture = None;
                            return;
                        }
                        if key_name(*key).is_none() {
                            continue; // teclas sin nombre: no se pueden asignar
                        }
                        let combo = binding_str(evmods, *key);
                        self.bind.insert(act.clone(), combo.clone());
                        lang::save_bindings(&self.bind);
                        self.capture = None;
                        set_pending_toast(
                            self.t("shortcut_set").replace("{k}", &combo),
                        );
                        return;
                    }
                }
                return;
            }

            // ---------- navegación/ar fixed ----------
            if i.key_pressed(Key::ArrowLeft) {
                self.navigate(-1);
            }
            if i.key_pressed(Key::ArrowRight) {
                self.navigate(1);
            }
            if i.key_pressed(Key::Home) && !self.files.is_empty() {
                self.go_to(0);
            }
            if i.key_pressed(Key::End) {
                let n = self.files.len();
                if n > 0 {
                    self.go_to(n - 1);
                }
            }
            if i.key_pressed(Key::Plus) || i.key_pressed(Key::Equals) {
                self.set_zoom_target(self.zoom_target * 1.3);
            }
            if i.key_pressed(Key::Minus) {
                self.set_zoom_target(self.zoom_target / 1.3);
            }
            if i.key_pressed(Key::Num1) {
                self.zoom_target = 1.0;
                self.offset_target = Vec2::ZERO;
            }
            if i.key_pressed(Key::Num2) {
                let fit = self.last_fit.max(0.05);
                self.set_zoom_target(fit.recip());
            }
            if i.key_pressed(Key::F) {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
            if i.modifiers.ctrl && i.key_pressed(Key::S) && !i.modifiers.shift {
                self.n_export_ann();
            }
            if i.modifiers.ctrl && i.key_pressed(Key::S) && i.modifiers.shift {
                self.export_annotated_png();
            }
            if i.modifiers.ctrl && i.key_pressed(Key::O) {
                self.open_file_dialog();
            }
            if i.key_pressed(Key::Delete) {
                self.push_undo();
                self.clear_annotations();
            }

            // ---------- atajos personalizables ----------
            if self.hit(i, "copy") {
                self.copy_image();
            }
            if self.hit(i, "undo") {
                self.do_undo();
            }
            if self.hit(i, "redo") {
                self.do_redo();
            }
            if self.hit(i, "pan") {
                self.tool = Tool::Pan;
            }
            if self.hit(i, "pen") {
                self.tool = Tool::Pen;
            }
            if self.hit(i, "marker") {
                self.tool = Tool::Marker;
            }
            if self.hit(i, "settings") {
                self.prefs_open = !self.prefs_open;
            }
        });
    }

    /// ¿La combinación del atajo `action` está presionada en este frame?
    fn hit(&self, i: &egui::InputState, action: &str) -> bool {
        let Some(b) = self.bind.get(action) else { return false };
        let (want_ctrl, want_shift, want_alt, ek) = parse_binding(b);
        let Some(ek) = ek else { return false };
        if !i.key_pressed(ek) {
            return false;
        }
        i.modifiers.ctrl == want_ctrl
            && i.modifiers.shift == want_shift
            && i.modifiers.alt == want_alt
    }

    /// Guarda el estado actual de anotaciones para deshacer.
    fn push_undo(&mut self) {
        let Some(path) = self.files.get(self.current) else { return };
        let st = self.annotations.get(path).cloned().unwrap_or_default();
        self.undo_s.push((path.clone(), st));
        if self.undo_s.len() > 60 {
            self.undo_s.remove(0);
        }
        self.redo_s.clear();
    }

    fn do_undo(&mut self) {
        let Some((path, prev)) = self.undo_s.pop() else { return };
        let cur = self.annotations.get(&path).cloned().unwrap_or_default();
        self.redo_s.push((path.clone(), cur));
        self.annotations.insert(path, prev);
        set_pending_toast(self.t("act_undo").to_string());
    }

    fn do_redo(&mut self) {
        let Some((path, cur)) = self.redo_s.pop() else { return };
        let prev = self.annotations.get(&path).cloned().unwrap_or_default();
        self.undo_s.push((path.clone(), prev));
        self.annotations.insert(path, cur);
        set_pending_toast(self.t("act_redo").to_string());
    }

    /// Copia la imagen actual (sin anotaciones) al portapapeles.
    fn copy_image(&self) {
        let Some(path) = self.files.get(self.current) else { return };
        match image_to_clipboard(path) {
            Ok(()) => set_pending_toast(self.t("copied").to_string()),
            Err(e) => set_pending_toast(format!("{} {e}", self.t("copy_failed"))),
        }
    }

    fn n_export_ann(&mut self) {
        // sidecar export: keeps annotations out of the image pixels (non-destructive)
        let Some(path) = self.files.get(self.current) else { return };
        let Some(list) = self.annotations.get(path) else { return };
        let mut out = String::from("{\n \"format\": \"huanview-annotations\", \"version\": 1,\n \"strokes\": [\n");
        for (j, a) in list.iter().enumerate() {
            out.push_str(&format!(
                "  {{\"width\":{:.1},\"alpha\":{:.2},\"color\":[{},{},{},{}],\"points\":[",
                a.width,
                a.alpha,
                a.color.r(),
                a.color.g(),
                a.color.b(),
                a.color.a()
            ));
            for (k, pt) in a.points.iter().enumerate() {
                if k > 0 {
                    out.push(',');
                }
                out.push_str(&format!("{:.5},{:.5}", pt.x, pt.y));
            }
            out.push_str("]}");
            if j + 1 < list.len() {
                out.push(',');
            }
            out.push('\n');
        }
        out.push_str(" ]\n}\n");
        let sidecar = path.with_extension(format!(
            "{}huanview.json",
            path.extension().and_then(|e| e.to_str()).unwrap_or("img")
        ));
        if std::fs::write(&sidecar, out).is_ok() {
            set_pending_toast(format!("{} {}", self.t("ann_saved"), sidecar.display()));
        } else {
            set_pending_toast(self.t("ann_failed").to_string());
        }
    }
}

// ----------------------------------------------------------- motor de atajos

/// Convierte un egui::Key a su nombre canónico (el mismo que se guarda en settings).
fn key_name(k: egui::Key) -> Option<&'static str> {
    Some(match k {
        egui::Key::A => "A",
        egui::Key::B => "B",
        egui::Key::C => "C",
        egui::Key::D => "D",
        egui::Key::E => "E",
        egui::Key::F => "F",
        egui::Key::G => "G",
        egui::Key::H => "H",
        egui::Key::I => "I",
        egui::Key::J => "J",
        egui::Key::K => "K",
        egui::Key::L => "L",
        egui::Key::M => "M",
        egui::Key::N => "N",
        egui::Key::O => "O",
        egui::Key::P => "P",
        egui::Key::Q => "Q",
        egui::Key::R => "R",
        egui::Key::S => "S",
        egui::Key::T => "T",
        egui::Key::U => "U",
        egui::Key::V => "V",
        egui::Key::W => "W",
        egui::Key::X => "X",
        egui::Key::Y => "Y",
        egui::Key::Z => "Z",
        egui::Key::Num0 => "Num0",
        egui::Key::Num1 => "Num1",
        egui::Key::Num2 => "Num2",
        egui::Key::Num3 => "Num3",
        egui::Key::Num4 => "Num4",
        egui::Key::Num5 => "Num5",
        egui::Key::Num6 => "Num6",
        egui::Key::Num7 => "Num7",
        egui::Key::Num8 => "Num8",
        egui::Key::Num9 => "Num9",
        egui::Key::Comma => "Comma",
        egui::Key::Period => "Period",
        egui::Key::Space => "Space",
        egui::Key::Minus => "Minus",
        egui::Key::Plus => "Plus",
        egui::Key::Equals => "Equals",
        egui::Key::F1 => "F1",
        egui::Key::F2 => "F2",
        egui::Key::F3 => "F3",
        egui::Key::F4 => "F4",
        egui::Key::F5 => "F5",
        egui::Key::F6 => "F6",
        egui::Key::F7 => "F7",
        egui::Key::F8 => "F8",
        egui::Key::F9 => "F9",
        egui::Key::F10 => "F10",
        egui::Key::F11 => "F11",
        egui::Key::F12 => "F12",
        egui::Key::Escape => "Escape",
        egui::Key::Backspace => "Backspace",
        egui::Key::Delete => "Delete",
        egui::Key::Home => "Home",
        egui::Key::End => "End",
        egui::Key::PageUp => "PageUp",
        egui::Key::PageDown => "PageDown",
        _ => return None,
    })
}

fn key_from_name(name: &str) -> Option<egui::Key> {
    use egui::Key;
    Some(match name.to_ascii_uppercase().as_str() {
        "A" => Key::A, "B" => Key::B, "C" => Key::C, "D" => Key::D, "E" => Key::E,
        "F" => Key::F, "G" => Key::G, "H" => Key::H, "I" => Key::I, "J" => Key::J,
        "K" => Key::K, "L" => Key::L, "M" => Key::M, "N" => Key::N, "O" => Key::O,
        "P" => Key::P, "Q" => Key::Q, "R" => Key::R, "S" => Key::S, "T" => Key::T,
        "U" => Key::U, "V" => Key::V, "W" => Key::W, "X" => Key::X, "Y" => Key::Y,
        "Z" => Key::Z,
        "NUM0" | "0" => Key::Num0, "NUM1" | "1" => Key::Num1, "NUM2" | "2" => Key::Num2,
        "NUM3" | "3" => Key::Num3, "NUM4" | "4" => Key::Num4, "NUM5" | "5" => Key::Num5,
        "NUM6" | "6" => Key::Num6, "NUM7" | "7" => Key::Num7, "NUM8" | "8" => Key::Num8,
        "NUM9" | "9" => Key::Num9,
        "COMMA" | "," => Key::Comma, "PERIOD" | "." => Key::Period,
        "SPACE" | " " => Key::Space, "MINUS" | "-" => Key::Minus,
        "PLUS" | "+" => Key::Plus, "EQUALS" | "=" => Key::Equals,
        "F1" => Key::F1, "F2" => Key::F2, "F3" => Key::F3, "F4" => Key::F4,
        "F5" => Key::F5, "F6" => Key::F6, "F7" => Key::F7, "F8" => Key::F8,
        "F9" => Key::F9, "F10" => Key::F10, "F11" => Key::F11, "F12" => Key::F12,
        _ => return None,
    })
}

/// Pone la imagen (RGBA) en el portapapeles del sistema.
fn image_to_clipboard(path: &PathBuf) -> Result<(), String> {
    let rgba = image::open(path)
        .map_err(|e| e.to_string())?
        .to_rgba8();
    let (w, h) = (rgba.width() as usize, rgba.height() as usize);
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_image(arboard::ImageData {
        width: w,
        height: h,
        bytes: std::borrow::Cow::Owned(rgba.into_raw()),
    })
    .map_err(|e| e.to_string())?;
    Ok(())
}

/// Parsea "Ctrl+C" -> (ctrl, shift, alt, Some(Key::C)).
fn parse_binding(b: &str) -> (bool, bool, bool, Option<egui::Key>) {
    let mut ctrl = false;
    let mut shift = false;
    let mut alt = false;
    let mut key = None;
    for tok in b.split('+') {
        let t = tok.trim();
        match t.to_ascii_lowercase().as_str() {
            "ctrl" | "control" => ctrl = true,
            "shift" => shift = true,
            "alt" => alt = true,
            other => {
                if !other.is_empty() {
                    key = key_from_name(other);
                }
            }
        }
    }
    (ctrl, shift, alt, key)
}

fn binding_str(mods: &egui::Modifiers, k: egui::Key) -> String {
    let name = key_name(k).unwrap_or("");
    let mut s = String::new();
    if mods.ctrl {
        s.push_str("Ctrl+");
    }
    if mods.shift {
        s.push_str("Shift+");
    }
    if mods.alt {
        s.push_str("Alt+");
    }
    s.push_str(name);
    s
}

// ----------------------------------------------------------- updater glue

/// Chequeo manual (ignora auto_check): usado por el botón "Buscar ahora".
fn updater_manual_check() -> Receiver<UpdateMsg> {
    let (tx, rx) = mpsc::channel::<UpdateMsg>();
    std::thread::Builder::new()
        .name("huanview-updater-manual".into())
        .spawn(move || {
            let res = check_for_update();
            let _ = tx.send(UpdateMsg::Checked(res));
        })
        .unwrap();
    rx
}

fn updater_start_check() -> Receiver<UpdateMsg> {
    let (tx, rx) = mpsc::channel::<UpdateMsg>();
    std::thread::Builder::new()
        .name("huanview-updater".into())
        .spawn(move || {
            let cfg = load_config();
            if !cfg.auto_check {
                return; // desactivado: no envía mensaje
            }
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let res = check_for_update();
            save_config(&updater::UpdaterConfig { auto_check: cfg.auto_check, last_check: Some(now) });
            let _ = tx.send(UpdateMsg::Checked(res));
        })
        .unwrap();
    rx
}

impl HuanApp {
    fn poll_update(&mut self, ctx: &egui::Context) {
        // descarga en curso: actualizar progreso
        if let Some(p) = self.update.progress.as_ref() {
            let v = p.load(std::sync::atomic::Ordering::SeqCst);
            if v > 0 {
                self.update.progress_pct = v;
            }
        }
        let Some(rx) = self.update_rx.as_ref() else { return };
        let mut new_rx: Option<Receiver<UpdateMsg>> = None;
        loop {
            match rx.try_recv() {
                Ok(UpdateMsg::Checked(res)) => {
                    match res {
                        Ok(Some(rel)) => {
                            set_pending_toast(self.t("update_found").to_string());
                            self.update.available = Some(rel);
                        }
                        Ok(None) => {}
                        Err(e) => self.update.err = Some(e), // silencioso
                    }
                    new_rx = Some(updater_start_check()); // deja listo el siguiente chequeo (tecla U o relanzamiento)
                    break;
                }
                Ok(UpdateMsg::Downloaded(res)) => {
                    match res {
                        Ok(path) => {
                            self.update.progress = None;
                            match apply_update_and_restart(&path) {
                                Ok(()) => {
                                    self.update_rx = None;
                                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                                    std::process::exit(0);
                                }
                                Err(e) => set_pending_toast(format!("No se pudo aplicar: {e}")),
                            }
                        }
                        Err(e) => set_pending_toast(format!("Fallo la descarga: {e}")),
                    }
                    break;
                }
                Err(_) => break,
            }
        }
        if let Some(rx2) = new_rx {
            self.update_rx = Some(rx2);
        }
    }

    fn start_update_download(&mut self) {
        let Some(rel) = self.update.available.clone() else { return };
        let dest = match download_destination() {
            Ok(d) => d,
            Err(e) => {
                set_pending_toast(e);
                return;
            }
        };
        let (tx, rx) = mpsc::channel();
        let url = rel.download_url.clone();
        let prog = Arc::new(AtomicU64::new(0));
        self.update.progress = Some(prog.clone());
        self.update.progress_pct = 0;
        self.update_rx = Some(rx);
        set_pending_toast(format!("{} {}", self.t("downloading"), rel.version_full));
        std::thread::Builder::new()
            .name("huanview-update-dl".into())
            .spawn(move || {
                let res = match download_to(&url, &dest, prog) {
                    Ok(()) => Ok(dest),
                    Err(e) => Err(e),
                };
                let _ = tx.send(UpdateMsg::Downloaded(res));
            })
            .unwrap();
    }
}
