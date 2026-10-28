//! Folder listing (natural-sorted) of supported image formats.

use std::path::{Path, PathBuf};

pub const EXTENSIONS: &[&str] =
    &["png", "jpg", "jpeg", "gif", "webp", "bmp", "tif", "tiff", "ico", "jfif"];

pub fn collect_images(path: &Path) -> (Vec<PathBuf>, usize) {
    if path.is_file() {
        let dir = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let files = list_dir(&dir);
        let start = files.iter().position(|f| f == &target).unwrap_or(0);
        (files, start)
    } else {
        (list_dir(path), 0)
    }
}

fn list_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = Vec::new();
    if let Ok(rd) = std::fs::read_dir(dir) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_file() {
                if let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                    if EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()) {
                        files.push(p);
                    }
                }
            }
        }
    }
    files.sort_by(|a, b| {
        let na = a.file_name().and_then(|s| s.to_str()).unwrap_or("");
        let nb = b.file_name().and_then(|s| s.to_str()).unwrap_or("");
        nat_str_cmp(na, nb)
    });
    files
}

/// Byte-safe natural compare (digits ordered numerically, else case-folded chars).
fn nat_str_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    let (ab, bb) = (a.as_bytes(), b.as_bytes());
    let (mut ai, mut bi) = (0usize, 0usize);
    while ai < ab.len() && bi < bb.len() {
        if ab[ai].is_ascii_digit() && bb[bi].is_ascii_digit() {
            let (as_, ae) = digit_run(ab, ai);
            let (bs, be) = digit_run(bb, bi);
            let va: u64 = a[as_..ae].parse().unwrap_or(u64::MAX);
            let vb: u64 = b[bs..be].parse().unwrap_or(u64::MAX);
            match va.cmp(&vb) {
                std::cmp::Ordering::Equal => {
                    ai = ae;
                    bi = be;
                }
                o => return o,
            }
        } else {
            let ca = a[ai..].chars().next().unwrap_or('\0');
            let cb = b[bi..].chars().next().unwrap_or('\0');
            let la = ca.to_lowercase().next().unwrap_or(ca);
            let lb = cb.to_lowercase().next().unwrap_or(cb);
            match la.cmp(&lb) {
                std::cmp::Ordering::Equal => {
                    ai += ca.len_utf8();
                    bi += cb.len_utf8();
                }
                o => return o,
            }
        }
    }
    (ab.len() - ai).cmp(&(bb.len() - bi))
}

fn digit_run(b: &[u8], from: usize) -> (usize, usize) {
    let mut i = from;
    while i < b.len() && b[i].is_ascii_digit() {
        i += 1;
    }
    let mut s = from;
    while s < i - 1 && b[s] == b'0' {
        s += 1;
    }
    (s, i)
}
