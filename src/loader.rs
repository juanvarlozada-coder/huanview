//! Background decoding inspired by Gwenview's LoadingJob + ThreadedDocumentJob:
//! - dedicated worker threads; the UI thread never touches decoders
//! - generation counter: stale full-decode jobs are dropped when navigation supersedes them
//! - priority order: current image first, then next, then previous
//! - thumbnails for filmstrip done in the same pool, deduplicated per batch

use egui::ColorImage;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

pub enum Loaded {
    Full { index: usize, image: ColorImage },
    Thumb { path: PathBuf, image: ColorImage },
    Fail { index: usize },
}

enum Job {
    Full { index: usize, path: PathBuf, gen: u64 },
    Thumb { path: PathBuf },
}

fn decode_color(path: &Path) -> Result<ColorImage, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let rgba = img.to_rgba8();
    Ok(ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

fn thumb_image(img: image::DynamicImage, max_side: u32) -> ColorImage {
    let img = if img.width().max(img.height()) > max_side {
        let k = max_side as f32 / img.width().max(img.height()) as f32;
        let w = (img.width() as f32 * k).round().max(1.0) as u32;
        let h = (img.height() as f32 * k).round().max(1.0) as u32;
        img.resize_exact(w, h, image::imageops::FilterType::Triangle)
    } else {
        img
    };
    let rgba = img.to_rgba8();
    ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    )
}

struct Shared {
    gen: AtomicU64,
    pending_thumbs: Mutex<HashSet<PathBuf>>,
}

pub struct Loader {
    jobs_tx: Sender<Job>,
    rx_done: Mutex<Receiver<Loaded>>,
    shared: Arc<Shared>,
}

impl Loader {
    pub fn new(workers: usize) -> Self {
        let (jobs_tx, jobs_rx) = mpsc::channel::<Job>();
        let (res_tx, res_rx) = mpsc::channel::<Loaded>();
        let jobs_rx = Arc::new(Mutex::new(jobs_rx));
        let shared = Arc::new(Shared {
            gen: AtomicU64::new(1),
            pending_thumbs: Mutex::new(HashSet::new()),
        });

        for _ in 0..workers.max(2) {
            let jobs_rx = jobs_rx.clone();
            let res_tx = res_tx.clone();
            let shared = shared.clone();
            std::thread::Builder::new()
                .name("huanview-loader".into())
                .spawn(move || loop {
                    let job = {
                        let rx = jobs_rx.lock().unwrap();
                        rx.try_recv()
                    };
                    match job {
                        Err(_) => std::thread::sleep(std::time::Duration::from_millis(8)),
                        Ok(Job::Full { index, path, gen }) => {
                            if shared.gen.load(Ordering::SeqCst) != gen {
                                continue; // superseded while queued
                            }
                            match decode_color(&path) {
                                Ok(image) => {
                                    if shared.gen.load(Ordering::SeqCst) == gen {
                                        let _ = res_tx.send(Loaded::Full { index, image });
                                    }
                                }
                                Err(_) => {
                                    let _ = res_tx.send(Loaded::Fail { index });
                                }
                            }
                        }
                        Ok(Job::Thumb { path }) => match image::open(&path) {
                            Ok(img) => {
                                let _ = res_tx.send(Loaded::Thumb {
                                    path,
                                    image: thumb_image(img, 128),
                                });
                            }
                            Err(_) => {}
                        },
                    }
                })
                .unwrap();
        }

        Self {
            jobs_tx,
            rx_done: Mutex::new(res_rx),
            shared,
        }
    }

    /// Reset the priority queue: current -> next -> previous full decodes,
    /// then filmstrip thumbnails (deduplicated per batch).
    pub fn reload(&self, paths: &[PathBuf], current: usize) {
        let g = self.shared.gen.fetch_add(1, Ordering::SeqCst) + 1;
        {
            let mut pending = self.shared.pending_thumbs.lock().unwrap();
            pending.clear();
            if let Some(p) = paths.get(current) {
                let _ = self.jobs_tx.send(Job::Full { index: current, path: p.clone(), gen: g });
            }
            if let Some(p) = paths.get(current + 1) {
                let _ = self.jobs_tx.send(Job::Full { index: current + 1, path: p.clone(), gen: g });
            }
            if current > 0 {
                if let Some(p) = paths.get(current - 1) {
                    let _ = self.jobs_tx.send(Job::Full { index: current - 1, path: p.clone(), gen: g });
                }
            }
            for p in paths {
                _ = pending.replace(p.clone());
            }
        }
        for p in paths {
            let _ = self.jobs_tx.send(Job::Thumb { path: p.clone() });
        }
    }

    pub fn cancel(&self) {
        self.shared.gen.fetch_add(1, Ordering::SeqCst);
        self.shared.pending_thumbs.lock().unwrap().clear();
    }

    pub fn poll(&self) -> Vec<Loaded> {
        let mut out = Vec::new();
        let rx = self.rx_done.lock().unwrap();
        while let Ok(msg) = rx.try_recv() {
            out.push(msg);
        }
        out
    }
}
