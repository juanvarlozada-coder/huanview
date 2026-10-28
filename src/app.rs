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

use crate::loader::{Loaded, Loader};
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
}

impl HuanApp {
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
            .frame(Frame::none().fill(BG))
            .exact_height(FILM_H)
            .show(ctx, |ui| self.filmstrip(ui));

        egui::CentralPanel::default().show(ctx, |ui| {
            self.canvas(ctx, ui);
        });

        self.hud_window(ctx);
        self.hud_toast(ctx);

        ctx.request_repaint();
    }
}

// --------------------------------------------------------------- HUD window

impl HuanApp {
    fn hud_window(&mut self, ctx: &egui::Context) {
        Window::new("huanhud")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .movable(false)
            .current_pos(Pos2::new(12.0, 12.0))
            .frame(Frame::window(&ctx.style()).fill(CHIP_BG))
            .show(ctx, |ui| {
                // info line
                let zoom_pct = if self.zoom_target >= 1.0 {
                    format!("{} %", (self.zoom * 100.0).round())
                } else {
                    format!("{:.1} %", self.zoom * 100.0)
                };
                let name = self
                    .files
                    .get(self.current)
                    .and_then(|p| p.file_name().and_then(|s| s.to_str()))
                    .unwrap_or("(sin imágenes)")
                    .to_string();
                let counter = if self.files.is_empty() {
                    String::new()
                } else {
                    format!(" {} de {} ", self.current + 1, self.files.len())
                };
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(name).font(FontId::proportional(12.0)));
                    ui.label(egui::RichText::new(counter).small().color(TEXT_DIM));
                    ui.label(
                        egui::RichText::new(zoom_pct)
                            .small()
                            .color(if self.zoom_target <= 1.001 { TEXT_DIM } else { ACCENT }),
                    );
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if gui_button(ui, "‹") {
                        self.navigate(-1);
                    }
                    if gui_button(ui, "›") {
                        self.navigate(1);
                    }
                    ui.add_space(4.0);
                    if gui_button(ui, "−") {
                        self.set_zoom_target(self.zoom_target / 1.3);
                    }
                    if gui_button(ui, "Ajustar") {
                        self.zoom_target = 1.0;
                        self.offset_target = Vec2::ZERO;
                    }
                    if gui_button(ui, "+") {
                        self.set_zoom_target(self.zoom_target * 1.3);
                    }
                    ui.add_space(4.0);
                    if gui_button(ui, "Ver") {
                        self.side_tap();
                    }
                    if gui_button(ui, "Lápiz") {
                        self.side_pen_drop();
                    }
                    if gui_button(ui, "Marc") {
                        self.marker_hint();
                    }
                    ui.add_space(2.0);
                    for (ci, col) in PEN_COLORS.iter().enumerate() {
                        self.color_button(ui, *col, ci);
                    }
                    ui.add_space(2.0);
                    if gui_button(ui, "Borrar") {
                        self.clear_annotations();
                    }
                });
            });
    }

    fn side_tap(&mut self) {
        self.tool = Tool::Pan;
        // tapping the hud shouldn't need alternate nav
    }

    fn side_pen_drop(&mut self) {
        self.tool = Tool::Pen;
    }

    fn marker_hint(&mut self) {
        self.tool = Tool::Marker;
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
        if let Some(p) = self.files.get(self.current) {
            self.annotations.remove(p);
        }
    }

    // color chip + tool buttons: keep small structural helpers below
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
                "Cargando…",
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

        // ---- click-to-navigate on the sides when fully fitted
        let fits_fully = self.zoom_target <= 1.001 && shown.x <= avail.size().x + 1.5;
        if !resp.dragged() && !resp.drag_started() && resp.clicked() && fits_fully {
            if let Some(pos) = ctx.input(|i| i.pointer.interact_pos()) {
                let t = (pos.x - rect.left()) / rect.width();
                if t < 0.2 {
                    self.navigate(-1);
                } else if t > 0.8 {
                    self.navigate(1);
                }
            }
        }

        // ---- drag: pan (zero latency) or draw annotation
        match self.tool {
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
                            ui.painter().rect_stroke(rect.expand(1.5), 7.0, Stroke::new(2.0_f32, ACCENT));
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
                // 1:1 (pixels reales): zoom = 1 / fit_scale
                let fit = self.last_fit.max(0.05);
                self.set_zoom_target(fit.recip());
            }
            if i.key_pressed(Key::F) {
                self.fullscreen = !self.fullscreen;
                ctx.send_viewport_cmd(ViewportCommand::Fullscreen(self.fullscreen));
            }
            if i.key_pressed(Key::A) {
                self.tool = Tool::Pen;
            }
            if i.key_pressed(Key::M) {
                self.tool = Tool::Marker;
            }
            if i.key_pressed(Key::V) {
                self.tool = Tool::Pan;
            }
            if i.modifiers.ctrl && i.key_pressed(Key::S) {
                self.n_export_ann();
            }
            if i.key_pressed(Key::Delete) {
                if let Some(p) = self.files.get(self.current) {
                    self.annotations.remove(p);
                }
            }
        });
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
            set_pending_toast(format!("Anotaciones guardadas: {}", sidecar.display()));
        } else {
            set_pending_toast("No se pudieron guardar las anotaciones".into());
        }
    }
}
