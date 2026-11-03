//! Auto-update via GitHub Releases (like Gwenview's update story, minimalist):
//! - checks `releases/latest` on launch (throttled), always in a worker thread
//! - Windows: downloads the NSIS installer and runs it with /S (per-user, no admin)
//! - AppImage: atomically swaps its own file
//! - never force-installs: user must press the update button

use serde::{Deserialize, Serialize};
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

const OWNER: &str = "juanvarlozada-coder";
const REPO: &str = "huanview";
const UA: &str = "huanview-updater";
#[derive(Debug, Clone)]
pub struct Release {
    pub version: String, // "0.1.1" (sin "v")
    pub version_full: String,
    pub download_url: String,
}

#[derive(Default)]
pub struct UpdaterState {
    /// versión nueva disponible
    pub available: Option<Release>,
    /// Some(pct) mientras descarga (0..=100)
    pub progress: Option<Arc<AtomicU64>>,
    pub progress_pct: u64,
    pub err: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
pub struct UpdaterConfig {
    pub auto_check: bool,
    pub last_check: Option<u64>,
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("huanview").join("updater.json"))
}

pub fn load_config() -> UpdaterConfig {
    std::fs::read_to_string(config_path().unwrap_or_default())
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or(UpdaterConfig { auto_check: true, last_check: None })
}

pub fn save_config(cfg: &UpdaterConfig) {
    if let Some(p) = config_path() {
        if let Some(parent) = p.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(s) = serde_json::to_string_pretty(cfg) {
            let _ = std::fs::write(p, s);
        }
    }
}

fn current_version_tuple() -> (u64, u64, u64) {
    let v = env!("CARGO_PKG_VERSION");
    parse_version(v).unwrap_or((0, 0, 0))
}

fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let v = v.trim().trim_start_matches('v');
    let mut it = v.split('.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

/// ¿Hay versión nueva? Descarga la lista de releases y decide.
/// En Linux solo consideramos releases con AppImage disponible (x86_64);
/// en Windows, con instalador win64.
pub fn check_for_update() -> Result<Option<Release>, String> {
    let resp = ureq::get(&format!(
        "https://api.github.com/repos/{OWNER}/{REPO}/releases/latest"
    ))
    .timeout(std::time::Duration::from_secs(10))
    .set("User-Agent", UA)
    .call()
    .map_err(|e| e.to_string())?;
    let text: String = resp.into_string().map_err(|e| e.to_string())?;
    let body: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let tag = body.get("tag_name").and_then(|t| t.as_str()).unwrap_or("");
    let new_version = parse_version(tag).ok_or_else(|| format!("tag inválido: {tag}"))?;
    let (cur_major, cur_minor, cur_patch) = current_version_tuple();
    let no_new = new_version <= (cur_major, cur_minor, cur_patch);
    if no_new {
        return Ok(None);
    }
    let assets = body
        .get("assets")
        .and_then(|a| a.as_array())
        .cloned()
        .unwrap_or_default();
    let pick = |prefix: &str| {
        assets
            .iter()
            .find(|a| {
                let n = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                n.starts_with(prefix)
            })
            .and_then(|a| {
                a.get("browser_download_url")
                    .and_then(|u| u.as_str().map(|s| s.to_string()))
            })
    };
    let antecedent = tag.trim_start_matches('v'); // los assets no llevan "v"
    let expected_prefix = if cfg!(windows) {
        format!("Huanview-{antecedent}-installer-win64")
    } else {
        format!("Huanview-{antecedent}-x86_64")
    };
    let url = pick(&expected_prefix)
        .or_else(|| pick(if cfg!(windows) { "Huanview-installer-win64" } else { "Huanview-x86_64" }))
        .or_else(|| {
            // fallback: cualquier asset con el patrón genérico
            let generic = if cfg!(windows) { "installer-win64" } else { "AppImage" };
            let want_plain = if cfg!(windows) { "installer-win64" } else { "x86_64.AppImage" };
            assets.iter().find_map(|a| {
                let n = a.get("name").and_then(|n| n.as_str()).unwrap_or("");
                if n.ends_with(want_plain) || (cfg!(windows) && n.ends_with(generic)) {
                    a.get("browser_download_url")
                        .and_then(|u| u.as_str().map(|s| s.to_string()))
                } else {
                    None
                }
            })
        })
        .ok_or("el release no tiene el paquete para esta plataforma")?;
    Ok(Some(Release {
        version: tag.trim_start_matches('v').to_string(),
        version_full: tag.to_string(),
        download_url: url,
    }))
}

/// Descarga `url` a `dst` avisando progreso (0..=100). Devuelve error si anómalo.
pub fn download_to(url: &str, dst: &PathBuf, progress: Arc<AtomicU64>) -> Result<(), String> {
    let resp = ureq::get(url)
        .set("User-Agent", UA)
        .call()
        .map_err(|e| e.to_string())?;
    let total: u64 = resp
        .header("Content-Length")
        .and_then(|h| h.parse().ok())
        .unwrap_or(0);
    let mut reader = resp.into_reader();
    let mut file = std::io::BufWriter::new(
        std::fs::File::create(dst).map_err(|e| e.to_string())?,
    );
    let mut buf = [0u8; 64 * 1024];
    let mut downloaded: u64 = 0;
    loop {
        let n = reader.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        std::io::Write::write_all(&mut file, &buf[..n]).map_err(|e| e.to_string())?;
        downloaded += n as u64;
        if total > 0 {
            let pct = ((downloaded * 100) / total).min(100);
            progress.store(pct, Ordering::SeqCst);
        }
    }
    progress.store(100, Ordering::SeqCst);
    Ok(())
}

/// Destino de la descarga:
/// - Windows: %TEMP% (el instalador puede correr desde ahí)
/// - Linux AppImage: el MISMO directorio del AppImage (rename atómico cross-fs imposible)
pub fn download_destination() -> Result<PathBuf, String> {
    if cfg!(windows) {
        let mut p = std::env::temp_dir();
        p.push("huanview-update-installer.exe");
        Ok(p)
    } else if let Ok(appimg) = std::env::var("APPIMAGE") {
        let dir = PathBuf::from(appimg);
        dir.parent()
            .map(|d| d.join("Huanview.new.download"))
            .ok_or_else(|| "AppImage sin directorio".into())
    } else {
        Err("solo AppImage soportado para auto-actualizar en Linux".into())
    }
}

/// Ejecuta el update y termina la app.
///
/// Windows: lanza el instalador NSIS con /S (silencioso, HKCU, sin admin) y sale
/// para que el instalador pueda reemplazar huanview.exe.
///
/// Linux (AppImage): swap atómico del propio archivo y re-lanzamiento.
pub fn apply_update_and_restart(installer_path: &PathBuf) -> Result<(), String> {
    if cfg!(windows) {
        // El instalador NSIS reemplaza huanview.exe y re-crea accesos.
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            std::process::Command::new(installer_path)
                .arg("/S")
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()
                .map_err(|e| e.to_string())?;
            Ok(())
        }
        #[cfg(not(windows))]
        {
            let _ = installer_path;
            Err("instalador de Windows solo en Windows".into())
        }
    } else if let Ok(appimg_str) = std::env::var("APPIMAGE") {
        let appimg = PathBuf::from(appimg_str);
        // 1) preparar el nuevo AppImage como .new en el MISMO directorio (rename atómico)
        let dir = appimg
            .parent()
            .ok_or_else(|| "AppImage sin directorio".to_string())?;
        let new_path = dir.join("Huanview.new");
        std::fs::rename(installer_path, &new_path).map_err(|e| e.to_string())?;
        // 2) permisos + swap
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&new_path, std::fs::Permissions::from_mode(0o755));
        }
        let old = dir.join("Huanview.old");
        let _ = std::fs::remove_file(&old);
        // el rename sobre el AppImage en ejecución es válido en Linux/LinuxKit FUSE
        std::fs::rename(&appimg, &old).map_err(|e| e.to_string())?;
        std::fs::rename(&new_path, &appimg).map_err(|e| e.to_string())?;
        // 3) relanzar
        let self_args: Vec<String> = std::env::args().skip(1).collect();
        std::process::Command::new(&appimg)
            .args(self_args)
            .spawn()
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err(
            "actualización automática no soportada para este formato (usa el AppImage o el instalador)"
                .into(),
        )
    }
}

