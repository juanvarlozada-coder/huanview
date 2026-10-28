// Bench: mide decode_color sobre imágenes reales (mismo camino que la app)
use std::time::Instant;

fn decode_color(path: &std::path::Path) -> Result<image::DynamicImage, String> {
    image::open(path).map_err(|e| e.to_string())
}

fn main() {
    let paths: Vec<_> = std::env::args().skip(1).collect();
    let mut worst = 0.0f64;
    for p in paths {
        let t = Instant::now();
        match decode_color(std::path::Path::new(&p)) {
            Ok(img) => {
                let dt = t.elapsed().as_secs_f64();
                worst = worst.max(dt);
                println!(
                    "{:>90}  {}x{}  {:6.1} ms",
                    std::path::Path::new(&p)
                        .file_name()
                        .and_then(|s| s.to_str())
                        .unwrap_or(&p),
                    img.width(),
                    img.height(),
                    dt * 1000.0
                );
            }
            Err(e) => println!("{p}: ERROR {e}"),
        }
    }
    println!("peor caso: {:.1} ms", worst * 1000.0);
}
