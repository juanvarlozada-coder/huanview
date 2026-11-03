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

#[derive(Clone, Serialize, Deserialize, Default)]
pub struct Prefs {
    pub lang: Lang,
    /// ¿mostrar flechas de navegación sin pasar el cursor? (op-out)
    #[serde(default = "default_true")]
    pub nav_arrows_hover: bool,
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
                return prefs;
            }
        }
    }
    Prefs {
        lang: detect_os_lang(),
        nav_arrows_hover: true,
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
    save_prefs(&Prefs {
        lang,
        nav_arrows_hover: read_settings().nav_arrows_hover,
    });
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
        // fallback: cualquier otra combinación/clave → español
        (Lang::Es, _) | (Lang::En, _) | (Lang::Pt, _) => "",
    }
}
