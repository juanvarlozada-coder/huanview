//! Soporte multi-idioma mínimo (ES / EN / PT), sin dependencias.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Lang {
    #[default]
    Es,
    En,
    Pt,
}

pub const LANGS: [(&str, Lang); 3] = [
    ("Español", Lang::Es),
    ("English", Lang::En),
    ("Português", Lang::Pt),
];

pub fn settings_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("huanview").join("settings.json"))
}

pub fn load_lang() -> Lang {
    read_settings().lang
}

/// Diccionario acción → combinación ("Ctrl+C", "A", "F2", "Escape",…).
pub type Bindings = std::collections::BTreeMap<String, String>;

/// Atajos por defecto (los que el usuario no haya redefinido en settings.json).
pub fn default_bindings() -> Bindings {
    std::collections::BTreeMap::from([
        ("copy".into(), "Ctrl+C".into()),
        ("undo".into(), "Ctrl+Z".into()),
        ("redo".into(), "Ctrl+Y".into()),
        ("pan".into(), "V".into()),
        ("pen".into(), "A".into()),
        ("marker".into(), "M".into()),
        ("settings".into(), "Ctrl+Comma".into()),
    ])
}

pub fn load_settings() -> Bindings {
    let p = read_settings();
    let mut b = default_bindings();
    for (k, v) in p.bindings.iter() {
        if v.is_empty() {
            b.remove(k);
        } else {
            b.insert(k.clone(), v.clone());
        }
    }
    b
}

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Prefs {
    pub lang: Lang,
    /// ¿mostrar flechas de navegación sin pasar el cursor? (op-out)
    #[serde(default = "default_true")]
    pub nav_arrows_hover: bool,
    /// atajos redefinidos por el usuario (solo los que cambian)
    #[serde(default)]
    pub bindings: Bindings,
}

fn default_true() -> bool {
    true
}

fn read_settings() -> Prefs {
    if let Some(p) = settings_path() {
        if let Ok(s) = std::fs::read_to_string(p) {
            if let Ok(mut prefs) = serde_json::from_str::<Prefs>(&s) {
                if matches!(prefs.lang, Lang::Es) && !s.contains("\"lang\"") {
                    prefs.lang = detect_os_lang();
                }
                prefs.bindings.retain(|_, v| !v.is_empty() );
                return prefs;
            }
        }
    }
    Prefs {
        lang: detect_os_lang(),
        nav_arrows_hover: true,
        bindings: Default::default(),
    }
}

fn detect_os_lang() -> Lang {
    match std::env::var("LANG").unwrap_or_default().to_lowercase().as_str() {
        l if l.starts_with("es") => Lang::Es,
        l if l.starts_with("pt") => Lang::Pt,
        _ => Lang::En,
    }
}

pub fn save_lang(lang: Lang) {
    let mut p = read_settings();
    p.lang = lang;
    save_prefs(&p);
}

/// Guarda los atajos definidos preservando el resto de preferencias.
pub fn save_bindings(b: &Bindings) {
    let mut p = read_settings();
    p.bindings = b.clone();
    save_prefs(&p);
}

pub fn save_prefs(prefs: &Prefs) {
    if let Some(p) = settings_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string_pretty(prefs) {
            let _ = std::fs::write(p, s);
        }
    }
}

/// Traducción por clave. Las claves son estables ('static).
pub fn tr(lang: Lang, key: &str) -> &'static str {
    match (lang, key) {
        // ---------- español (original)
        (Lang::Es, "no_images") => "(sin imágenes)",
        (Lang::Es, "loading") => "Cargando…",
        (Lang::Es, "view") => "Vista",
        (Lang::Es, "pen") => "Lápiz",
        (Lang::Es, "marker") => "Marcador",
        (Lang::Es, "erase") => "Borrar",
        (Lang::Es, "open") => "Abrir",
        (Lang::Es, "save") => "Guardar",
        (Lang::Es, "save_as") => "Guardar como",
        (Lang::Es, "install") => "Instalar",
        (Lang::Es, "available") => "disponible",
        (Lang::Es, "downloading") => "Descargando",
        (Lang::Es, "download_failed") => "Fallo la descarga:",
        (Lang::Es, "apply_failed") => "No se pudo actualizar:",
        (Lang::Es, "updating") => "Actualizando…",
        (Lang::Es, "nowhere") => "No hay imágenes en esa carpeta",
        (Lang::Es, "saved") => "Guardado:",
        (Lang::Es, "png_failed") => "No se pudo guardar el PNG",
        (Lang::Es, "export_error") => "Error al exportar:",
        (Lang::Es, "ann_saved") => "Anotaciones guardadas:",
        (Lang::Es, "ann_failed") => "No se pudieron guardar las anotaciones",
        (Lang::Es, "settings") => "Ajustes",
        (Lang::Es, "whats_new") => "Novedades",
        (Lang::Es, "new_version") => "Nueva versión",
        (Lang::Es, "check_updates") => "Buscar actualizaciones al iniciar",
        (Lang::Es, "language") => "Idioma",
        (Lang::Es, "version_av") => "· Nueva versión",
        (Lang::Es, "no_images_folder") => "No hay imágenes en esa carpeta",
        (Lang::Es, "update_found") => "Versión nueva disponible — pulsa Instalar",
        (Lang::Es, "viewer") => "Visor de imágenes",
        (Lang::Es, "about_sub") => "Visor de imágenes minimalista y rápido",
        (Lang::Es, "shortcuts_hint") => "Haz clic en la combinación y pulsa la nueva tecla",
        (Lang::Es, "shortcut_set") => "Atajo asignado: {k}",
        (Lang::Es, "hand_note") => "Mantén Espacio para usar la mano temporalmente, sin importar la herramienta (fijo, no se puede cambiar).",
        (Lang::Es, "copy_failed") => "No se pudo copiar:",
        (Lang::Es, "copied") => "Imagen copiada al portapapeles",
        (Lang::Es, "capture_prompt") => "Pulsa una tecla… (Esc para cancelar)",
        (Lang::Es, "act_settings") => "Abrir ajustes",
        (Lang::Es, "act_pan") => "Vista / mano",
        (Lang::Es, "act_marker") => "Marcador (herramienta)",
        (Lang::Es, "act_pen") => "Lápiz (herramienta)",
        (Lang::Es, "act_redo") => "Rehacer",
        (Lang::Es, "act_undo") => "Deshacer",
        (Lang::Es, "act_copy") => "Copiar imagen",
        (Lang::Es, "cat_shortcuts") => "Atajos de teclado",
        (Lang::Es, "about_label") => "Huanview",
        (Lang::Es, "fit") => "Ajustar a la ventana",
        (Lang::Es, "zoom_out") => "Alejar",
        (Lang::Es, "zoom_in") => "Acercar",
        (Lang::Es, "checking") => "Buscando actualizaciones…",
        (Lang::Es, "check_now") => "Buscar ahora",
        (Lang::Es, "cat_about") => "Acerca de",
        (Lang::Es, "cat_updates") => "Actualizaciones",
        (Lang::Es, "cat_general") => "General",
        (Lang::Es, "prefs") => "Ajustes",
        // ---------- inglés
        (Lang::En, "no_images") => "(no images)",
        (Lang::En, "loading") => "Loading…",
        (Lang::En, "view") => "View",
        (Lang::En, "pen") => "Pen",
        (Lang::En, "marker") => "Marker",
        (Lang::En, "erase") => "Erase",
        (Lang::En, "open") => "Open",
        (Lang::En, "save") => "Save",
        (Lang::En, "save_as") => "Save as",
        (Lang::En, "install") => "Install",
        (Lang::En, "available") => "available",
        (Lang::En, "downloading") => "Downloading",
        (Lang::En, "download_failed") => "Download failed:",
        (Lang::En, "apply_failed") => "Couldn't update:",
        (Lang::En, "updating") => "Updating…",
        (Lang::En, "nowhere") => "No images in that folder",
        (Lang::En, "saved") => "Saved:",
        (Lang::En, "png_failed") => "Couldn't save the PNG",
        (Lang::En, "export_error") => "Export error:",
        (Lang::En, "ann_saved") => "Annotations saved:",
        (Lang::En, "ann_failed") => "Couldn't save annotations",
        (Lang::En, "settings") => "Settings",
        (Lang::En, "whats_new") => "What's new",
        (Lang::En, "new_version") => "New version",
        (Lang::En, "check_updates") => "Check for updates on startup",
        (Lang::En, "language") => "Language",
        (Lang::En, "version_av") => "· New version",
        (Lang::En, "no_images_folder") => "No images in that folder",
        (Lang::En, "update_found") => "New version available — press Install",
        (Lang::En, "viewer") => "Image viewer",
        (Lang::En, "about_sub") => "Fast, minimalist image viewer",
        (Lang::En, "shortcuts_hint") => "Click a binding and press the new key",
        (Lang::En, "shortcut_set") => "Shortcut set: {k}",
        (Lang::En, "hand_note") => "Hold Space for the temporary hand tool regardless of the active tool (fixed).",
        (Lang::En, "copy_failed") => "Couldn't copy:",
        (Lang::En, "copied") => "Image copied to clipboard",
        (Lang::En, "capture_prompt") => "Press a key… (Esc to cancel)",
        (Lang::En, "act_settings") => "Open settings",
        (Lang::En, "act_pan") => "View / hand",
        (Lang::En, "act_marker") => "Marker (tool)",
        (Lang::En, "act_pen") => "Pen (tool)",
        (Lang::En, "act_redo") => "Redo",
        (Lang::En, "act_undo") => "Undo",
        (Lang::En, "act_copy") => "Copy image",
        (Lang::En, "cat_shortcuts") => "Shortcuts",
        (Lang::En, "about_label") => "Huanview",
        (Lang::En, "fit") => "Fit to window",
        (Lang::En, "zoom_out") => "Zoom out",
        (Lang::En, "zoom_in") => "Zoom in",
        (Lang::En, "checking") => "Checking for updates…",
        (Lang::En, "check_now") => "Check now",
        (Lang::En, "cat_about") => "About",
        (Lang::En, "cat_updates") => "Updates",
        (Lang::En, "cat_general") => "General",
        (Lang::En, "prefs") => "Settings",
        // ---------- portugués
        (Lang::Pt, "no_images") => "(sem imagens)",
        (Lang::Pt, "loading") => "Carregando…",
        (Lang::Pt, "view") => "Ver",
        (Lang::Pt, "pen") => "Lápis",
        (Lang::Pt, "marker") => "Marcador",
        (Lang::Pt, "erase") => "Apagar",
        (Lang::Pt, "open") => "Abrir",
        (Lang::Pt, "save") => "Salvar",
        (Lang::Pt, "save_as") => "Salvar como",
        (Lang::Pt, "install") => "Instalar",
        (Lang::Pt, "available") => "disponível",
        (Lang::Pt, "downloading") => "Baixando",
        (Lang::Pt, "download_failed") => "Falha no download:",
        (Lang::Pt, "apply_failed") => "Não foi possível atualizar:",
        (Lang::Pt, "updating") => "Atualizando…",
        (Lang::Pt, "nowhere") => "Não há imagens nessa pasta",
        (Lang::Pt, "saved") => "Salvo:",
        (Lang::Pt, "png_failed") => "Não foi possível salvar o PNG",
        (Lang::Pt, "export_error") => "Erro na exportação:",
        (Lang::Pt, "ann_saved") => "Anotações salvas:",
        (Lang::Pt, "ann_failed") => "Não foi possível salvar as anotações",
        (Lang::Pt, "settings") => "Ajustes",
        (Lang::Pt, "whats_new") => "Novidades",
        (Lang::Pt, "new_version") => "Nova versão",
        (Lang::Pt, "check_updates") => "Verificar atualizações ao iniciar",
        (Lang::Pt, "language") => "Idioma",
        (Lang::Pt, "version_av") => "· Nova versão",
        (Lang::Pt, "no_images_folder") => "Não há imagens nessa pasta",
        (Lang::Pt, "update_found") => "Nova versão disponível — clique em Instalar",
        (Lang::Pt, "viewer") => "Visualizador de imagens",
        (Lang::Pt, "about_sub") => "Visualizador de imagens minimalista e rápido",
        (Lang::Pt, "shortcuts_hint") => "Clique no atalho e pressione a nova tecla",
        (Lang::Pt, "shortcut_set") => "Atalho definido: {k}",
        (Lang::Pt, "hand_note") => "Mantenha Espaço para usar a mão temporariamente (fixo).",
        (Lang::Pt, "copy_failed") => "Não foi possível copiar:",
        (Lang::Pt, "copied") => "Imagem copiada para a área de transferência",
        (Lang::Pt, "capture_prompt") => "Pressione uma tecla… (Esc para cancelar)",
        (Lang::Pt, "act_settings") => "Abrir ajustes",
        (Lang::Pt, "act_pan") => "Ver / mão",
        (Lang::Pt, "act_marker") => "Marcador (ferramenta)",
        (Lang::Pt, "act_pen") => "Lápis (ferramenta)",
        (Lang::Pt, "act_redo") => "Refazer",
        (Lang::Pt, "act_undo") => "Desfazer",
        (Lang::Pt, "act_copy") => "Copiar imagem",
        (Lang::Pt, "cat_shortcuts") => "Atalhos de teclado",
        (Lang::Pt, "about_label") => "Huanview",
        (Lang::Pt, "fit") => "Ajustar à janela",
        (Lang::Pt, "zoom_out") => "Diminuir",
        (Lang::Pt, "zoom_in") => "Ampliar",
        (Lang::Pt, "checking") => "Procurando atualizações…",
        (Lang::Pt, "check_now") => "Verificar agora",
        (Lang::Pt, "cat_about") => "Sobre",
        (Lang::Pt, "cat_updates") => "Atualizações",
        (Lang::Pt, "cat_general") => "Geral",
        (Lang::Pt, "prefs") => "Ajustes",
        // fallback: cualquier otra combinación/clave → español
        (Lang::Es, _) | (Lang::En, _) | (Lang::Pt, _) => "",
    }
}
