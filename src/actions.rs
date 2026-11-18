//! Document level operations: files, clipboard and whole-image edits.

use std::borrow::Cow;
use std::path::{Path, PathBuf};

use egui::{Color32, Context, Pos2};
use tiny_skia::Pixmap;

use crate::app::{ActiveShape, PaintApp, PendingAction, RotOp, Selection, Tab, TextBox, Tool};
use crate::dialogs::Dialog;
use crate::raster::{self, IRect};
use crate::shapes::ShapeKind;

pub const OPEN_EXTS: [&str; 9] = ["png", "jpg", "jpeg", "bmp", "gif", "tif", "tiff", "ico", "webp"];

fn config_dir() -> Option<PathBuf> {
    let home = PathBuf::from(std::env::var_os("HOME")?);
    let (dir, old_names) = if cfg!(target_os = "macos") {
        let base = home.join("Library/Application Support");
        (base.join("FreePaint"), [base.join("MacPaint"), base.join("RustPaint")])
    } else {
        let base = std::env::var_os("XDG_CONFIG_HOME").map(PathBuf::from).unwrap_or_else(|| home.join(".config"));
        (base.join("freepaint"), [base.join("freepaint"), base.join("rustpaint")])
    };
    // carry settings over from the app's previous names
    if !dir.exists() {
        for old in old_names {
            if old.exists() && std::fs::rename(&old, &dir).is_ok() {
                break;
            }
        }
    }
    Some(dir)
}

pub fn load_settings() -> std::collections::HashMap<String, String> {
    let Some(dir) = config_dir() else { return Default::default() };
    std::fs::read_to_string(dir.join("settings.txt"))
        .map(|s| s.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim().to_string(), v.trim().to_string())).collect())
        .unwrap_or_default()
}

pub fn save_setting(key: &str, value: &str) {
    let Some(dir) = config_dir() else { return };
    let mut all = load_settings();
    all.insert(key.to_string(), value.to_string());
    let mut lines: Vec<String> = all.iter().map(|(k, v)| format!("{k}={v}")).collect();
    lines.sort();
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("settings.txt"), lines.join("\n"));
}

pub fn load_recent() -> Vec<PathBuf> {
    let Some(dir) = config_dir() else { return Vec::new() };
    std::fs::read_to_string(dir.join("recent.txt"))
        .map(|s| s.lines().filter(|l| !l.trim().is_empty()).map(PathBuf::from).collect())
        .unwrap_or_default()
}

fn save_recent(list: &[PathBuf]) {
    let Some(dir) = config_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let s: Vec<String> = list.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let _ = std::fs::write(dir.join("recent.txt"), s.join("\n"));
}

impl PaintApp {
    fn add_recent(&mut self, p: &Path) {
        let p = p.to_path_buf();
        self.recent.retain(|r| r != &p);
        self.recent.insert(0, p);
        self.recent.truncate(9);
        save_recent(&self.recent);
    }

    pub fn error(&mut self, msg: impl Into<String>) {
        self.dialog = Some(Dialog::Error(msg.into()));
    }

    /// Run `action`, asking to save first if there are unsaved changes.
    pub fn request(&mut self, ctx: &Context, action: PendingAction) {
        self.finish_drag();
        self.commit_all();
        if self.doc.dirty {
            self.dialog = Some(Dialog::Unsaved(action));
        } else {
            self.perform(ctx, action);
        }
    }

    pub fn perform(&mut self, ctx: &Context, action: PendingAction) {
        match action {
            PendingAction::New => self.new_image(),
            PendingAction::Open => self.open_dialog(),
            PendingAction::OpenPath(p) => self.open_path(&p),
            PendingAction::Exit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }

    fn reset_document(&mut self, image: Pixmap) {
        self.layers = vec![crate::app::Layer { pix: image, visible: true }];
        self.active_layer = 0;
        self.thumbs_stale = true;
        self.history.clear();
        self.selection = None;
        self.active_shape = None;
        self.curve = None;
        self.poly = None;
        self.text_box = None;
        self.layer = None;
        self.layer_base = None;
        self.zoom = 1.0;
        self.pending_scroll = Some(egui::Vec2::ZERO);
        self.display_dirty = true;
        if self.tab == Tab::Text {
            self.tab = Tab::Home;
        }
    }

    pub fn new_image(&mut self) {
        let (w, h) = crate::app::DEFAULT_CANVAS;
        self.reset_document(raster::new_canvas(w, h, Color32::WHITE));
        self.doc.path = None;
        self.doc.dirty = false;
        self.doc.file_size = None;
        self.doc.saved_at = None;
    }

    pub fn open_dialog(&mut self) {
        let mut dlg = rfd::FileDialog::new().add_filter("All Picture Files", &OPEN_EXTS).add_filter("All Files", &["*"]);
        if let Some(dir) = self.doc.path.as_ref().and_then(|p| p.parent()) {
            dlg = dlg.set_directory(dir);
        }
        if let Some(path) = dlg.pick_file() {
            self.open_path(&path);
        }
    }

    pub fn open_path(&mut self, path: &Path) {
        let img = match image::open(path) {
            Ok(i) => i.to_rgba8(),
            Err(e) => {
                self.error(format!("Paint cannot read this file.\n\n{}\n\n{e}", path.display()));
                return;
            }
        };
        let (w, h) = img.dimensions();
        if w > 16384 || h > 16384 {
            self.error(format!("This picture is too large to open ({w} × {h}px)."));
            return;
        }
        self.reset_document(raster::from_rgba8(w, h, img.as_raw(), Some(Color32::WHITE)));
        self.doc.path = Some(path.to_path_buf());
        self.doc.dirty = false;
        let meta = std::fs::metadata(path).ok();
        self.doc.file_size = meta.as_ref().map(|m| m.len());
        self.doc.saved_at = meta.and_then(|m| m.modified().ok());
        self.add_recent(path);
    }

    pub fn save(&mut self) -> bool {
        match self.doc.path.clone() {
            Some(p) => self.save_to(&p),
            None => self.save_as(None),
        }
    }

    /// Show a Save As dialog. `ext` preselects a format.
    pub fn save_as(&mut self, ext: Option<&str>) -> bool {
        let current_ext = self.doc.path.as_ref().and_then(|p| p.extension()).map(|e| e.to_string_lossy().to_lowercase());
        let ext = ext.map(str::to_string).or(current_ext).unwrap_or_else(|| "png".into());
        let stem = self.doc.path.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).unwrap_or_else(|| "Untitled".into());
        let filters: [(&str, &[&str]); 6] = [
            ("PNG (*.png)", &["png"]),
            ("JPEG (*.jpg;*.jpeg)", &["jpg", "jpeg"]),
            ("24-bit Bitmap (*.bmp)", &["bmp"]),
            ("GIF (*.gif)", &["gif"]),
            ("TIFF (*.tif;*.tiff)", &["tif", "tiff"]),
            ("WebP (*.webp)", &["webp"]),
        ];
        let mut dlg = rfd::FileDialog::new().set_file_name(format!("{stem}.{ext}"));
        // put the chosen format first so it is the default filter
        let mut ordered: Vec<_> = filters.iter().filter(|f| f.1.contains(&ext.as_str())).collect();
        ordered.extend(filters.iter().filter(|f| !f.1.contains(&ext.as_str())));
        for (name, exts) in ordered {
            dlg = dlg.add_filter(*name, exts);
        }
        if let Some(dir) = self.doc.path.as_ref().and_then(|p| p.parent()) {
            dlg = dlg.set_directory(dir);
        }
        match dlg.save_file() {
            Some(mut path) => {
                if path.extension().is_none() {
                    path.set_extension(&ext);
                }
                self.save_to(&path)
            }
            None => false,
        }
    }

    pub fn save_to(&mut self, path: &Path) -> bool {
        self.finish_drag();
        self.commit_all();
        let (w, h) = (self.img_w(), self.img_h());
        let flat = self.composite_flat();
        let Some(rgba_img) = image::RgbaImage::from_raw(w, h, raster::to_rgba8(&flat)) else { return false };
        let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        let dynimg = image::DynamicImage::ImageRgba8(rgba_img);
        let result: Result<(), String> = (|| {
            match ext.as_str() {
                "jpg" | "jpeg" => {
                    let f = std::fs::File::create(path).map_err(|e| e.to_string())?;
                    let mut enc = image::codecs::jpeg::JpegEncoder::new_with_quality(std::io::BufWriter::new(f), 92);
                    enc.encode_image(&dynimg.to_rgb8()).map_err(|e| e.to_string())
                }
                "bmp" => dynimg.to_rgb8().save_with_format(path, image::ImageFormat::Bmp).map_err(|e| e.to_string()),
                "gif" => dynimg.save_with_format(path, image::ImageFormat::Gif).map_err(|e| e.to_string()),
                "tif" | "tiff" => dynimg.to_rgb8().save_with_format(path, image::ImageFormat::Tiff).map_err(|e| e.to_string()),
                "webp" => dynimg.save_with_format(path, image::ImageFormat::WebP).map_err(|e| e.to_string()),
                "ico" => dynimg.save_with_format(path, image::ImageFormat::Ico).map_err(|e| e.to_string()),
                _ => dynimg.to_rgb8().save_with_format(path, image::ImageFormat::Png).map_err(|e| e.to_string()),
            }
        })();
        match result {
            Ok(()) => {
                self.doc.path = Some(path.to_path_buf());
                self.doc.dirty = false;
                self.doc.file_size = std::fs::metadata(path).ok().map(|m| m.len());
                self.doc.saved_at = Some(std::time::SystemTime::now());
                self.add_recent(path);
                true
            }
            Err(e) => {
                self.error(format!("Paint could not save this picture.\n\n{}\n\n{e}", path.display()));
                false
            }
        }
    }

    // ----- clipboard -------------------------------------------------------

    /// Pixels of the current selection (scaled to its current size).
    pub fn selection_pixels(&self) -> Option<Pixmap> {
        let sel = self.selection.as_ref()?;
        Some(match &sel.content {
            Some(c) => self.floating_pixels(c, sel.rect),
            None => {
                let mut c = raster::crop(self.act(), sel.src);
                if let Some(m) = &sel.mask {
                    raster::apply_mask(&mut c, m);
                }
                c
            }
        })
    }

    pub fn copy(&mut self) {
        let Some(px) = self.selection_pixels() else { return };
        let data = raster::to_rgba8(&px);
        if let Some(cb) = self.clipboard.as_mut() {
            let _ = cb.set_image(arboard::ImageData { width: px.width() as usize, height: px.height() as usize, bytes: Cow::Owned(data) });
        }
        self.internal_clip = Some(px);
    }

    pub fn cut(&mut self) {
        if self.selection.is_none() {
            return;
        }
        self.copy();
        self.delete_selection();
    }

    pub fn paste(&mut self, _ctx: &Context) {
        let from_system = self.clipboard.as_mut().and_then(|cb| cb.get_image().ok()).map(|img| {
            raster::from_rgba8(img.width as u32, img.height as u32, &img.bytes, None)
        });
        let px = match from_system {
            Some(p) => p,
            None if self.clipboard.is_none() => match &self.internal_clip {
                Some(p) => p.clone(),
                None => return,
            },
            None => return,
        };
        self.paste_pixmap(px);
    }

    pub fn paste_pixmap(&mut self, px: Pixmap) {
        self.finish_drag();
        self.commit_all();
        if self.tool != Tool::Select && self.tool != Tool::FreeSelect {
            self.tool = Tool::Select;
        }
        self.push_undo();
        let (pw, ph) = (px.width(), px.height());
        if pw > self.img_w() || ph > self.img_h() {
            let r = IRect::new(0, 0, pw.max(self.img_w()) as i32, ph.max(self.img_h()) as i32);
            self.resize_all_layers(r);
        }
        // paste at the top-left of the visible area
        let vis = (self.viewport.min - self.view_origin) / self.zoom;
        let (vx, vy) = (vis.x.max(0.0) as i32, vis.y.max(0.0) as i32);
        let x = vx.min(self.img_w() as i32 - pw as i32).max(0);
        let y = vy.min(self.img_h() as i32 - ph as i32).max(0);
        let r = IRect::new(x, y, pw as i32, ph as i32);
        self.selection = Some(Selection { rect: r, src: r, mask: None, outline: None, content: Some(px) });
        self.display_dirty = true;
    }

    // ----- selection operations -------------------------------------------

    pub fn select_all(&mut self) {
        self.finish_drag();
        self.commit_all();
        self.tool = Tool::Select;
        self.selection = Some(Selection::rect(self.image_irect()));
        self.display_dirty = true;
    }

    pub fn invert_selection(&mut self) {
        let Some(sel) = &self.selection else {
            self.select_all();
            return;
        };
        let rect = sel.rect;
        let msrc = sel.src;
        let mask = sel.mask.clone();
        if sel.content.is_some() {
            self.commit_selection();
        }
        let full = self.image_irect();
        let mut inv = vec![255u8; (full.w * full.h) as usize];
        // a mask is indexed in `src` coordinates; the selection may have been
        // scaled since it was made, so map rect -> src proportionally
        let mask = mask.filter(|m| msrc.w > 0 && msrc.h > 0 && m.len() == (msrc.w * msrc.h) as usize);
        for y in 0..rect.h {
            for x in 0..rect.w {
                let (ix, iy) = (rect.x + x, rect.y + y);
                if ix < 0 || iy < 0 || ix >= full.w || iy >= full.h {
                    continue;
                }
                let inside = match &mask {
                    Some(m) => {
                        let mx = (x as i64 * msrc.w as i64 / rect.w.max(1) as i64).clamp(0, msrc.w as i64 - 1);
                        let my = (y as i64 * msrc.h as i64 / rect.h.max(1) as i64).clamp(0, msrc.h as i64 - 1);
                        m[(my * msrc.w as i64 + mx) as usize] != 0
                    }
                    None => true,
                };
                if inside {
                    inv[(iy * full.w + ix) as usize] = 0;
                }
            }
        }
        self.selection = Some(Selection { rect: full, src: full, mask: Some(inv), outline: None, content: None });
        self.display_dirty = true;
    }

    pub fn delete_selection(&mut self) {
        let Some(sel) = &self.selection else { return };
        if sel.content.is_none() {
            let (src, mask) = (sel.src, sel.mask.clone());
            self.push_undo();
            let hole = self.hole();
            match mask {
                Some(m) => raster::fill_masked(self.act_mut(), src, &m, hole),
                None => raster::fill_rect(self.act_mut(), src, hole),
            }
        }
        self.selection = None;
        self.display_dirty = true;
    }

    /// Crop every layer to the selection's bounding box.
    pub fn crop(&mut self) {
        let Some(sel) = &self.selection else { return };
        let r = sel.rect;
        if r.is_empty() {
            return;
        }
        self.commit_selection();
        self.set_canvas_rect(r);
    }

    pub fn rotate(&mut self, op: RotOp) {
        let f = |p: &Pixmap| match op {
            RotOp::Right => raster::rotate_cw(p),
            RotOp::Left => raster::rotate_ccw(p),
            RotOp::R180 => raster::rotate_180(p),
            RotOp::FlipV => raster::flip_v(p),
            RotOp::FlipH => raster::flip_h(p),
        };
        if self.selection.is_some() {
            self.lift_selection();
            let sel = self.selection.as_mut().unwrap();
            let scaled = raster::scale(sel.content.as_ref().unwrap(), sel.rect.w as u32, sel.rect.h as u32, false);
            let rotated = f(&scaled);
            let (cx, cy) = (sel.rect.x + sel.rect.w / 2, sel.rect.y + sel.rect.h / 2);
            let (nw, nh) = (rotated.width() as i32, rotated.height() as i32);
            sel.rect = IRect::new(cx - nw / 2, cy - nh / 2, nw, nh);
            sel.src = sel.rect;
            sel.mask = None;
            sel.outline = None;
            sel.content = Some(rotated);
        } else {
            self.finish_drag();
            self.commit_all();
            self.push_undo();
            for l in &mut self.layers {
                l.pix = f(&l.pix);
            }
        }
        self.display_dirty = true;
    }

    pub fn invert_colors(&mut self) {
        if self.selection.is_some() {
            self.lift_selection();
            if let Some(c) = self.selection.as_mut().and_then(|s| s.content.as_mut()) {
                raster::invert(c);
            }
        } else {
            self.commit_all();
            self.push_undo();
            raster::invert(self.act_mut());
        }
        self.display_dirty = true;
    }

    /// Resize (percent or pixels already resolved) and skew the selection or the whole image.
    pub fn resize_skew(&mut self, new_w: u32, new_h: u32, skew_h: f32, skew_v: f32) {
        let new_w = new_w.clamp(1, 16384);
        let new_h = new_h.clamp(1, 16384);
        if self.selection.is_some() {
            self.lift_selection();
            let sel = self.selection.as_mut().unwrap();
            let mut px = raster::scale(sel.content.as_ref().unwrap(), new_w, new_h, true);
            if skew_h != 0.0 || skew_v != 0.0 {
                px = raster::skew(&px, skew_h, skew_v, None);
            }
            sel.rect = IRect::new(sel.rect.x, sel.rect.y, px.width() as i32, px.height() as i32);
            sel.src = sel.rect;
            sel.mask = None;
            sel.outline = None;
            sel.content = Some(px);
        } else {
            self.finish_drag();
            self.commit_all();
            self.push_undo();
            let c2 = self.color2;
            for (i, l) in self.layers.iter_mut().enumerate() {
                let mut px = raster::scale(&l.pix, new_w, new_h, true);
                if skew_h != 0.0 || skew_v != 0.0 {
                    px = raster::skew(&px, skew_h, skew_v, (i == 0).then_some(c2));
                }
                l.pix = if i == 0 { raster::flatten_onto(&px, c2) } else { px };
            }
            // skewing can change the size; keep all layers the same size
            let (w, h) = (self.layers[0].pix.width(), self.layers[0].pix.height());
            for l in self.layers.iter_mut().skip(1) {
                if l.pix.width() != w || l.pix.height() != h {
                    let mut out = Pixmap::new(w, h).unwrap();
                    raster::copy_raw(&mut out, &l.pix, 0, 0);
                    l.pix = out;
                }
            }
        }
        self.display_dirty = true;
    }

    /// Paste an image file as a floating selection.
    pub fn paste_from_file(&mut self) {
        if let Some(path) = rfd::FileDialog::new().add_filter("All Picture Files", &OPEN_EXTS).pick_file() {
            match image::open(&path) {
                Ok(img) => {
                    let img = img.to_rgba8();
                    let px = raster::from_rgba8(img.width(), img.height(), img.as_raw(), None);
                    self.paste_pixmap(px);
                }
                Err(e) => self.error(format!("Paint cannot read this file.\n\n{e}")),
            }
        }
    }

    /// Change the canvas to cover `r` (in current image coordinates). Areas that
    /// were not part of the picture are filled with Color 2.
    pub fn set_canvas_rect(&mut self, r: IRect) {
        let r = IRect::new(r.x, r.y, r.w.clamp(1, 16384), r.h.clamp(1, 16384));
        if r == self.image_irect() {
            return;
        }
        self.finish_drag();
        self.commit_all();
        self.push_undo();
        self.resize_all_layers(r);
        self.display_dirty = true;
    }

    /// Rebuild every layer to cover `r` (in current image coordinates), without undo.
    fn resize_all_layers(&mut self, r: IRect) {
        let c2 = self.color2;
        for (i, l) in self.layers.iter_mut().enumerate() {
            let mut out = if i == 0 {
                raster::new_canvas(r.w as u32, r.h as u32, c2)
            } else {
                Pixmap::new(r.w.max(1) as u32, r.h.max(1) as u32).unwrap()
            };
            raster::copy_raw(&mut out, &l.pix, -r.x, -r.y);
            l.pix = out;
        }
        self.thumbs_stale = true;
    }

    // ----- layers ---------------------------------------------------------

    fn layer_edit(&mut self) {
        self.finish_drag();
        self.commit_all();
        self.push_undo();
        self.thumbs_stale = true;
    }

    /// Insert an empty layer above the active one and select it.
    pub fn add_layer(&mut self) {
        self.layer_edit();
        let (w, h) = (self.img_w(), self.img_h());
        self.layers.insert(self.active_layer + 1, crate::app::Layer::clear(w, h));
        self.active_layer += 1;
        self.display_dirty = true;
    }

    pub fn duplicate_layer(&mut self) {
        self.layer_edit();
        let copy = self.layers[self.active_layer].clone();
        self.layers.insert(self.active_layer + 1, copy);
        self.active_layer += 1;
        self.display_dirty = true;
    }

    pub fn delete_layer(&mut self) {
        if self.layers.len() <= 1 {
            return;
        }
        self.layer_edit();
        self.layers.remove(self.active_layer);
        self.active_layer = self.active_layer.min(self.layers.len() - 1);
        self.display_dirty = true;
    }

    /// Merge the active layer into the one below it.
    pub fn merge_layer_down(&mut self) {
        if self.active_layer == 0 {
            return;
        }
        self.layer_edit();
        let top = self.layers.remove(self.active_layer);
        self.active_layer -= 1;
        if top.visible {
            let i = self.active_layer;
            raster::draw_over(&mut self.layers[i].pix, &top.pix, 0, 0);
        }
        self.display_dirty = true;
    }

    /// Merge the active layer into the one above it (the upper layer stays on top).
    pub fn merge_layer_up(&mut self) {
        if self.active_layer + 1 >= self.layers.len() {
            return;
        }
        self.layer_edit();
        let cur = self.layers.remove(self.active_layer);
        let i = self.active_layer;
        if cur.visible {
            let mut base = cur.pix;
            raster::draw_over(&mut base, &self.layers[i].pix, 0, 0);
            self.layers[i].pix = base;
        }
        self.display_dirty = true;
    }

    pub fn move_layer(&mut self, up: bool) {
        let i = self.active_layer;
        let j = if up { i + 1 } else { i.wrapping_sub(1) };
        if j >= self.layers.len() {
            return;
        }
        self.layer_edit();
        self.layers.swap(i, j);
        self.active_layer = j;
        self.display_dirty = true;
    }

    pub fn toggle_layer_visible(&mut self, i: usize) {
        if i >= self.layers.len() {
            return;
        }
        self.finish_drag();
        self.commit_all();
        self.layers[i].visible = !self.layers[i].visible;
        self.doc.dirty = true;
        self.thumbs_stale = true;
        self.display_dirty = true;
    }

    pub fn select_layer(&mut self, i: usize) {
        if i >= self.layers.len() || i == self.active_layer {
            return;
        }
        self.finish_drag();
        self.commit_all();
        self.active_layer = i;
        self.display_dirty = true;
    }

    pub fn set_canvas_size(&mut self, w: u32, h: u32) {
        let (w, h) = (w.clamp(1, 16384), h.clamp(1, 16384));
        if w == self.img_w() && h == self.img_h() {
            return;
        }
        self.set_canvas_rect(IRect::new(0, 0, w as i32, h as i32));
    }

    pub fn pick_color(&mut self, p: (i32, i32), slot: u8) {
        let flat = self.composite_flat();
        if let Some(c) = raster::pixel(&flat, p.0, p.1) {
            self.set_color(slot, Color32::from_rgb(c[0], c[1], c[2]));
        }
    }

    pub fn add_custom_color(&mut self, c: Color32) {
        if let Some(slot) = self.custom_colors.iter_mut().find(|s| s.is_none()) {
            *slot = Some(c);
        } else {
            self.custom_colors.rotate_left(1);
            self.custom_colors[9] = Some(c);
        }
    }
}

/// Paints a small sample picture (used with `--demo` for screenshots).
pub fn draw_demo(app: &mut PaintApp) {
    use crate::brush::{BrushKind, BrushStroke};
    let c1 = app.color1;
    let shapes = [
        (ShapeKind::Rectangle, 40.0, 40.0, 180.0, 140.0, Color32::from_rgb(63, 72, 204), true),
        (ShapeKind::Oval, 220.0, 40.0, 360.0, 140.0, Color32::from_rgb(237, 28, 36), true),
        (ShapeKind::Star5, 400.0, 30.0, 520.0, 150.0, Color32::from_rgb(255, 201, 14), true),
        (ShapeKind::Heart, 560.0, 40.0, 680.0, 150.0, Color32::from_rgb(255, 174, 201), true),
        (ShapeKind::RightArrow, 40.0, 190.0, 200.0, 270.0, Color32::from_rgb(34, 177, 76), false),
        (ShapeKind::Lightning, 240.0, 180.0, 330.0, 300.0, Color32::from_rgb(255, 242, 0), true),
        (ShapeKind::CloudCallout, 370.0, 180.0, 530.0, 300.0, Color32::WHITE, true),
        (ShapeKind::OvalCallout, 560.0, 180.0, 700.0, 290.0, Color32::from_rgb(153, 217, 234), true),
        (ShapeKind::Hexagon, 40.0, 320.0, 160.0, 420.0, Color32::from_rgb(200, 191, 231), true),
        (ShapeKind::RoundedCallout, 190.0, 320.0, 340.0, 430.0, Color32::from_rgb(239, 228, 176), true),
    ];
    app.size_idx[2] = 1;
    for (kind, x0, y0, x1, y1, fill, filled) in shapes {
        app.color2 = fill;
        app.fill_on = filled;
        app.active_shape = Some(ActiveShape { kind, a: Pos2::new(x0, y0), b: Pos2::new(x1, y1), swap: false });
        app.commit_shape();
    }
    app.color2 = Color32::WHITE;
    app.fill_on = false;
    let mut y = 470.0;
    for (i, kind) in BrushKind::ALL.iter().enumerate() {
        let color = crate::app::PALETTE[0][(i + 2) % 10];
        let mut layer = Pixmap::new(app.img_w(), app.img_h()).unwrap();
        let mut st = BrushStroke::new(*kind, color, kind.sizes()[1], i as u64 + 1);
        let mut prev = (40.0f32, y);
        for k in 1..=60 {
            let x = 40.0 + k as f32 * 4.0;
            let yy = y + (k as f32 * 0.25).sin() * 8.0;
            st.segment(&mut layer, prev, (x, yy));
            prev = (x, yy);
        }
        let paint = tiny_skia::PixmapPaint { opacity: kind.opacity(), ..Default::default() };
        app.act_mut().draw_pixmap(0, 0, layer.as_ref(), &paint, tiny_skia::Transform::identity(), None);
        y += 14.0;
        if y > 590.0 {
            break;
        }
    }
    app.color1 = c1;
    app.text_box = Some(TextBox { rect: IRect::new(380, 340, 300, 60), text: "Hello from Free Paint!".into(), caret: 0 });
    app.text.size_pt = 20.0;
    app.commit_text();
    app.history.clear();
    app.doc.dirty = false;
    app.display_dirty = true;
}
