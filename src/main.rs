//! Free Paint – a classic ribbon-style paint program for macOS and Linux.

mod actions;
mod app;
mod brush;
mod canvas;
mod dialogs;
mod icons;
mod menus;
mod modern;
mod raster;
mod ribbon;
mod script;
mod shapes;
mod text;
mod theme;
mod widgets;

use std::path::PathBuf;

use app::{PaintApp, StartupOptions};

fn parse_args() -> StartupOptions {
    let mut opts = StartupOptions::default();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--screenshot" => opts.screenshot = args.next().map(PathBuf::from),
            "--demo" => opts.demo = true,
            "--menu" => opts.menu = args.next(),
            "--script" => opts.script = args.next().map(PathBuf::from),
            "--style" => opts.style = args.next(),
            "--theme" => opts.theme = args.next(),
            "--size" => {
                if let Some((w, h)) = args.next().as_deref().and_then(|s| s.split_once('x')) {
                    opts.size = w.parse().ok().zip(h.parse().ok());
                }
            }
            "--maximized" => opts.maximized = true,
            "--dump-state" => opts.dump = args.next().map(PathBuf::from),
            "--write-iconset" => {
                let dir = args.next().map(PathBuf::from).unwrap_or_else(|| PathBuf::from("AppIcon.iconset"));
                std::process::exit(match write_iconset(&dir) {
                    Ok(()) => 0,
                    Err(e) => {
                        eprintln!("could not write iconset: {e}");
                        1
                    }
                });
            }
            "-h" | "--help" => {
                println!("Usage: freepaint [IMAGE]\n\nOpens IMAGE (png, jpg, bmp, gif, tiff, ico, webp) if given.");
                std::process::exit(0);
            }
            // ignore launcher arguments such as macOS' "-psn_0_12345"
            _ if a.starts_with('-') => {}
            _ => opts.open = Some(PathBuf::from(a)),
        }
    }
    opts
}

/// Write the PNG sizes `iconutil` needs to build an .icns file.
fn write_iconset(dir: &std::path::Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    for base in [16u32, 32, 128, 256, 512] {
        for scale in [1u32, 2] {
            let px = base * scale;
            let pm = icons::rasterize_svg(icons::APP_MAC, px).ok_or("failed to render icon")?;
            let img = image::RgbaImage::from_raw(px, px, raster::to_rgba8(&pm)).ok_or("bad icon buffer")?;
            let name = if scale == 1 { format!("icon_{base}x{base}.png") } else { format!("icon_{base}x{base}@2x.png") };
            img.save(dir.join(name)).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn app_icon() -> Option<egui::IconData> {
    let pm = icons::rasterize_svg(icons::APP_MAC, 128)?;
    Some(egui::IconData { rgba: raster::to_rgba8(&pm), width: 128, height: 128 })
}

fn main() -> eframe::Result<()> {
    let opts = parse_args();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Untitled - Free Paint")
        .with_app_id("freepaint")
        .with_inner_size(opts.size.map(|(w, h)| [w, h]).unwrap_or([1200.0, 820.0]))
        .with_maximized(opts.maximized)
        .with_min_inner_size([640.0, 420.0])
        .with_drag_and_drop(true);
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let native = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("Free Paint", native, Box::new(move |cc| Ok(Box::new(PaintApp::new(cc, opts)))))
}
