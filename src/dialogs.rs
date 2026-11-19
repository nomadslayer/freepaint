//! Modal dialogs styled after the classic Windows ones.

use egui::{pos2, vec2, Align2, Color32, Context, Frame, Id, Margin, Rect, RichText, Sense, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui};

use crate::app::{PaintApp, PendingAction};
use crate::icons::Icon;
use crate::theme;
use crate::widgets::win_button;

pub struct ResizeState {
    pub by_percent: bool,
    pub h: String,
    pub v: String,
    pub keep: bool,
    pub skew_h: String,
    pub skew_v: String,
    pub base_w: u32,
    pub base_h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Units {
    Inches,
    Centimeters,
    Pixels,
}

pub struct PropState {
    pub units: Units,
    pub color: bool,
    pub w: String,
    pub h: String,
    pub px_w: f32,
    pub px_h: f32,
}

pub struct ColorState {
    pub slot: u8,
    pub h: f32,
    pub s: f32,
    pub l: f32,
    pub rgb: [u8; 3],
    pub custom: [Color32; 16],
    pub custom_sel: usize,
    pub spectrum: Option<TextureHandle>,
}

pub struct PrintState {
    pub printers: Vec<String>,
    pub sel: usize,
    pub copies: String,
    pub fit: bool,
    pub landscape: bool,
}

pub enum Dialog {
    ResizeSkew(ResizeState),
    Print(PrintState),
    /// A newer release exists on GitHub.
    Update { current: String, info: crate::update::UpdateInfo },
    /// Neutral information message.
    Info(String),
    Properties(PropState),
    EditColors(Box<ColorState>),
    Unsaved(PendingAction),
    About,
    Error(String),
}

const BASIC_COLORS: [u32; 48] = [
    0xFF8080, 0xFFFF80, 0x80FF80, 0x00FF80, 0x80FFFF, 0x0080FF, 0xFF80C0, 0xFF80FF, //
    0xFF0000, 0xFFFF00, 0x80FF00, 0x00FF40, 0x00FFFF, 0x0080C0, 0x8080C0, 0xFF00FF, //
    0x804040, 0xFF8040, 0x00FF00, 0x008080, 0x004080, 0x8080FF, 0x800040, 0xFF0080, //
    0x800000, 0xFF8000, 0x008000, 0x008040, 0x0000FF, 0x0000A0, 0x800080, 0x8000FF, //
    0x400000, 0x804000, 0x004000, 0x004040, 0x000080, 0x000040, 0x400040, 0x400080, //
    0x000000, 0x808000, 0x808040, 0x808080, 0x408080, 0xC0C0C0, 0x400040, 0xFFFFFF, //
];

fn hex(c: u32) -> Color32 {
    Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
}

/// HSL on the Windows 0..240 scale to RGB.
pub fn hsl_to_rgb(h: f32, s: f32, l: f32) -> [u8; 3] {
    let (h, s, l) = ((h / 240.0).rem_euclid(1.0), (s / 240.0).clamp(0.0, 1.0), (l / 240.0).clamp(0.0, 1.0));
    if s == 0.0 {
        let v = (l * 255.0).round() as u8;
        return [v, v, v];
    }
    let q = if l < 0.5 { l * (1.0 + s) } else { l + s - l * s };
    let p = 2.0 * l - q;
    let f = |mut t: f32| {
        t = t.rem_euclid(1.0);
        let v = if t < 1.0 / 6.0 {
            p + (q - p) * 6.0 * t
        } else if t < 0.5 {
            q
        } else if t < 2.0 / 3.0 {
            p + (q - p) * (2.0 / 3.0 - t) * 6.0
        } else {
            p
        };
        (v * 255.0).round().clamp(0.0, 255.0) as u8
    };
    [f(h + 1.0 / 3.0), f(h), f(h - 1.0 / 3.0)]
}

pub fn rgb_to_hsl(c: [u8; 3]) -> (f32, f32, f32) {
    let (r, g, b) = (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    if max == min {
        return (160.0, 0.0, (l * 240.0).round());
    }
    let d = max - min;
    let s = if l > 0.5 { d / (2.0 - max - min) } else { d / (max + min) };
    let h = if max == r {
        (g - b) / d + if g < b { 6.0 } else { 0.0 }
    } else if max == g {
        (b - r) / d + 2.0
    } else {
        (r - g) / d + 4.0
    } / 6.0;
    ((h * 240.0).round() % 240.0, (s * 240.0).round(), (l * 240.0).round())
}

/// Installed printer names via CUPS, plus the index of the default one.
fn list_printers() -> (Vec<String>, usize) {
    let run = |args: &[&str]| std::process::Command::new("lpstat").args(args).output().ok();
    let printers: Vec<String> = run(&["-e"])
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect())
        .unwrap_or_default();
    let default = run(&["-d"])
        .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
        .and_then(|t| t.rsplit_once(':').map(|(_, n)| n.trim().to_string()))
        .and_then(|n| printers.iter().position(|p| *p == n))
        .unwrap_or(0);
    (printers, default)
}

/// Arguments for CUPS `lp`.
pub(crate) fn print_args(printer: &str, copies: u32, fit: bool, landscape: bool, path: &str) -> Vec<String> {
    let mut a = vec!["-d".into(), printer.into(), "-n".into(), copies.to_string(), "-t".into(), "Free Paint".into()];
    a.push("-o".into());
    a.push(if fit { "fit-to-page".into() } else { "ppi=96".into() });
    if landscape {
        a.push("-o".into());
        a.push("landscape".into());
    }
    a.push(path.into());
    a
}

impl Dialog {
    pub fn print(app: &PaintApp) -> Self {
        let (printers, sel) = list_printers();
        Dialog::Print(PrintState { printers, sel, copies: "1".into(), fit: true, landscape: app.img_w() > app.img_h() })
    }

    pub fn resize(app: &PaintApp) -> Self {
        let (w, h) = match &app.selection {
            Some(s) => (s.rect.w as u32, s.rect.h as u32),
            None => (app.img_w(), app.img_h()),
        };
        Dialog::ResizeSkew(ResizeState { by_percent: true, h: "100".into(), v: "100".into(), keep: true, skew_h: "0".into(), skew_v: "0".into(), base_w: w, base_h: h })
    }

    pub fn properties(app: &PaintApp) -> Self {
        let (w, h) = (app.img_w() as f32, app.img_h() as f32);
        Dialog::Properties(PropState { units: Units::Pixels, color: true, w: format!("{w}"), h: format!("{h}"), px_w: w, px_h: h })
    }

    pub fn edit_colors(c: Color32, slot: u8, custom: &[Color32; 16]) -> Self {
        let rgb = [c.r(), c.g(), c.b()];
        let (h, s, l) = rgb_to_hsl(rgb);
        Dialog::EditColors(Box::new(ColorState { slot, h, s, l, rgb, custom: *custom, custom_sel: 0, spectrum: None }))
    }
}

enum Outcome {
    Keep,
    Close,
}

/// Window chrome: white title bar with close button over a light gray body.
fn chrome(ctx: &Context, nonce: u64, title: &str, width: f32, body: impl FnOnce(&mut Ui) -> Outcome) -> Outcome {
    let modern = theme::modern();
    let radius = if modern { egui::CornerRadius::same(8) } else { egui::CornerRadius::ZERO };
    let bg = if modern { theme::w().dialog_bg } else { theme::DIALOG_BG };
    let frame = Frame::NONE
        .fill(bg)
        .corner_radius(radius)
        .stroke(theme::sk(1.0, if modern { theme::w().menu_border } else { Color32::from_rgb(24, 131, 215) }))
        .shadow(egui::Shadow { offset: [0, if modern { 8 } else { 4 }], blur: if modern { 24 } else { 16 }, spread: 0, color: Color32::from_black_alpha(70) });
    let resp = egui::Modal::new(Id::new(("paint-dialog", nonce))).frame(frame).backdrop_color(Color32::from_black_alpha(18)).show(ctx, |ui| {
        ui.set_width(width);
        ui.spacing_mut().item_spacing = vec2(6.0, 6.0);
        let bar_h = if modern { 36.0 } else { 30.0 };
        let (bar0, _) = ui.allocate_exact_size(vec2(width, bar_h), Sense::hover());
        let out = Frame::NONE.inner_margin(Margin { left: 14, right: 14, top: 8, bottom: 12 }).show(ui, |ui| body(ui)).inner;
        // paint the title bar last so it spans the final dialog width
        let full_w = ui.min_rect().width().max(width);
        let bar = Rect::from_min_size(bar0.min, vec2(full_w, bar_h));
        let p = ui.painter();
        let top_radius = egui::CornerRadius { nw: radius.nw, ne: radius.ne, sw: 0, se: 0 };
        p.rect_filled(bar, top_radius, if modern { theme::w().dialog_title } else { Color32::WHITE });
        p.text(pos2(bar.left() + if modern { 14.0 } else { 10.0 }, bar.center().y), Align2::LEFT_CENTER, title, egui::FontId::proportional(12.0), theme::text_color());
        let close_r = Rect::from_min_size(pos2(bar.right() - 46.0, bar.top()), vec2(46.0, bar.height()));
        let close = ui.interact(close_r, Id::new("dlg-close"), Sense::click());
        if close.hovered() {
            let cr = egui::CornerRadius { nw: 0, ne: radius.ne, sw: 0, se: 0 };
            p.rect_filled(close_r, cr, Color32::from_rgb(196, 43, 28));
        }
        let col = if close.hovered() { Color32::WHITE } else { theme::text_color() };
        let c = close_r.center();
        p.line_segment([pos2(c.x - 5.0, c.y - 5.0), pos2(c.x + 5.0, c.y + 5.0)], theme::sk(1.0, col));
        p.line_segment([pos2(c.x + 5.0, c.y - 5.0), pos2(c.x - 5.0, c.y + 5.0)], theme::sk(1.0, col));
        if close.clicked() || ui.input(|i| i.key_pressed(egui::Key::Escape)) {
            Outcome::Close
        } else {
            out
        }
    });
    resp.inner
}

fn group_box(ui: &mut Ui, title: &str, add: impl FnOnce(&mut Ui)) {
    let frame = Frame::NONE.stroke(Stroke::new(1.0_f32, if theme::modern() { theme::w().border } else { Color32::from_rgb(220, 220, 220) })).inner_margin(Margin { left: 10, right: 10, top: 14, bottom: 8 });
    let resp = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    });
    let r = resp.response.rect;
    let p = ui.painter();
    let g = p.layout_no_wrap(title.to_string(), egui::FontId::proportional(12.0), theme::text_color());
    let tr = Rect::from_min_size(pos2(r.left() + 7.0, r.top() - g.size().y / 2.0), g.size() + vec2(6.0, 0.0));
    p.rect_filled(tr, 0.0, if theme::modern() { theme::w().dialog_bg } else { theme::DIALOG_BG });
    p.galley(pos2(tr.left() + 3.0, tr.top()), g, theme::text_color());
}

fn num_field(ui: &mut Ui, s: &mut String) -> bool {
    let r = ui.add(egui::TextEdit::singleline(s).desired_width(64.0));
    r.changed()
}

fn buttons_row(ui: &mut Ui, labels: &[&str]) -> Option<usize> {
    let mut clicked = None;
    ui.add_space(12.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        for (i, l) in labels.iter().enumerate().rev() {
            if win_button(ui, l, i == 0).clicked() {
                clicked = Some(i);
            }
        }
    });
    if clicked.is_none() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
        clicked = Some(0);
    }
    clicked
}

impl PaintApp {
    pub fn dialogs_ui(&mut self, ctx: &Context) {
        let Some(mut dlg) = self.dialog.take() else {
            self.dialog_kind = None;
            return;
        };
        // a different dialog (or a reopened one) gets a fresh window area
        let kind = std::mem::discriminant(&dlg);
        if self.dialog_kind != Some(kind) {
            self.dialog_kind = Some(kind);
            self.dialog_nonce = self.dialog_nonce.wrapping_add(1);
        }
        let outcome = match &mut dlg {
            Dialog::ResizeSkew(st) => self.resize_dialog(ctx, st),
            Dialog::Print(st) => self.print_dialog(ctx, st),
            Dialog::Update { current, info } => {
                let (current, info) = (current.clone(), info.clone());
                self.update_dialog(ctx, &current, &info)
            }
            Dialog::Info(msg) => {
                let msg = msg.clone();
                self.info_dialog(ctx, &msg)
            }
            Dialog::Properties(st) => self.properties_dialog(ctx, st),
            Dialog::EditColors(st) => self.colors_dialog(ctx, st),
            Dialog::Unsaved(action) => {
                let action = action.clone();
                self.unsaved_dialog(ctx, action)
            }
            Dialog::About => self.about_dialog(ctx),
            Dialog::Error(msg) => {
                let msg = msg.clone();
                self.message_dialog(ctx, &msg)
            }
        };
        if matches!(outcome, Outcome::Keep) && self.dialog.is_none() {
            self.dialog = Some(dlg);
        }
    }

    /// Flatten the picture into a temp PNG for printing / preview.
    fn write_print_file(&mut self) -> Option<std::path::PathBuf> {
        self.finish_drag();
        self.commit_all();
        let flat = self.composite_flat();
        let path = std::env::temp_dir().join("freepaint-print.png");
        let (w, h) = (flat.width(), flat.height());
        match image::RgbaImage::from_raw(w, h, crate::raster::to_rgba8(&flat)) {
            Some(img) => match img.save(&path) {
                Ok(()) => Some(path),
                Err(e) => {
                    self.error(format!("Could not prepare the picture for printing.\n\n{e}"));
                    None
                }
            },
            None => None,
        }
    }

    fn update_dialog(&mut self, ctx: &Context, current: &str, info: &crate::update::UpdateInfo) -> Outcome {
        let mut out_action = 0;
        let out = chrome(ctx, self.dialog_nonce, "Update available", 360.0, |ui| {
            ui.add_space(12.0);
            ui.label(RichText::new(format!("Free Paint {} is available.", info.version.trim_start_matches('v'))).font(theme::bold(14.0)));
            ui.add_space(4.0);
            ui.label(format!("You have version {current}. The download page has the macOS .dmg and the Ubuntu .deb."));
            match buttons_row(ui, &["Download", "Later"]) {
                Some(0) => {
                    out_action = 1;
                    Outcome::Close
                }
                Some(_) => Outcome::Close,
                None => Outcome::Keep,
            }
        });
        if out_action == 1 {
            ctx.open_url(egui::OpenUrl::new_tab(&info.url));
            return Outcome::Close;
        }
        out
    }

    fn info_dialog(&mut self, ctx: &Context, msg: &str) -> Outcome {
        let mut close = false;
        let out = chrome(ctx, self.dialog_nonce, "Free Paint", 380.0, |ui| {
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                let c = r.center();
                let accent = if theme::modern() { theme::w().accent } else { Color32::from_rgb(31, 111, 208) };
                ui.painter().circle_filled(c, 15.0, accent);
                let on = if theme::modern() { theme::w().on_accent } else { Color32::WHITE };
                ui.painter().circle_filled(egui::pos2(c.x, c.y - 6.5), 1.8, on);
                ui.painter().rect_filled(Rect::from_min_size(egui::pos2(c.x - 1.7, c.y - 3.0), vec2(3.4, 10.0)), 1.0, on);
                ui.add_space(8.0);
                ui.label(msg);
            });
            if buttons_row(ui, &["OK"]).is_some() {
                close = true;
            }
            Outcome::Keep
        });
        if close {
            Outcome::Close
        } else {
            out
        }
    }

    fn print_dialog(&mut self, ctx: &Context, st: &mut PrintState) -> Outcome {
        #[derive(PartialEq)]
        enum Act {
            None,
            Print,
            Preview,
        }
        let mut act = Act::None;
        let out = chrome(ctx, self.dialog_nonce, "Print", 320.0, |ui| {
            let have = !st.printers.is_empty();
            egui::Grid::new("print-grid").num_columns(2).spacing(vec2(10.0, 8.0)).show(ui, |ui| {
                ui.label("Printer:");
                if have {
                    let current = st.printers.get(st.sel).cloned().unwrap_or_default();
                    egui::ComboBox::from_id_salt("printer").width(180.0).selected_text(current).show_ui(ui, |ui| {
                        for (i, p) in st.printers.iter().enumerate() {
                            if ui.selectable_label(i == st.sel, p).clicked() {
                                st.sel = i;
                            }
                        }
                    });
                } else {
                    ui.label(RichText::new("No printers were found").color(theme::disabled_color()));
                }
                ui.end_row();
                ui.label("Copies:");
                ui.add(egui::TextEdit::singleline(&mut st.copies).desired_width(48.0));
                ui.end_row();
                ui.label("Scale:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut st.fit, true, "Fit to page");
                    ui.radio_value(&mut st.fit, false, "Actual size");
                });
                ui.end_row();
                ui.label("Orientation:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut st.landscape, false, "Portrait");
                    ui.radio_value(&mut st.landscape, true, "Landscape");
                });
                ui.end_row();
            });
            if !have {
                ui.add_space(4.0);
                ui.label(RichText::new("Add a printer in the system settings, or use Preview to print from the system viewer.").size(11.0).color(theme::disabled_color()));
            }
            ui.add_space(4.0);
            let labels: &[&str] = if have { &["Print", "Preview", "Cancel"] } else { &["Preview", "Cancel"] };
            match buttons_row(ui, labels) {
                Some(0) if have => {
                    act = Act::Print;
                    Outcome::Close
                }
                Some(i) if (have && i == 1) || (!have && i == 0) => {
                    act = Act::Preview;
                    Outcome::Close
                }
                Some(_) => Outcome::Close,
                None => Outcome::Keep,
            }
        });
        match act {
            Act::Print => {
                let copies = st.copies.trim().parse::<u32>().unwrap_or(1).clamp(1, 99);
                if let Some(path) = self.write_print_file() {
                    let printer = st.printers[st.sel.min(st.printers.len() - 1)].clone();
                    let args = print_args(&printer, copies, st.fit, st.landscape, &path.to_string_lossy());
                    match std::process::Command::new("lp").args(&args).output() {
                        Ok(o) if o.status.success() => {}
                        Ok(o) => self.error(format!("Printing failed.\n\n{}", String::from_utf8_lossy(&o.stderr).trim())),
                        Err(e) => self.error(format!("Printing failed. Is CUPS installed?\n\n{e}")),
                    }
                }
                Outcome::Close
            }
            Act::Preview => {
                if let Some(path) = self.write_print_file() {
                    let opener = if cfg!(target_os = "macos") { "open" } else { "xdg-open" };
                    let _ = std::process::Command::new(opener).arg(&path).spawn();
                }
                Outcome::Close
            }
            Act::None => out,
        }
    }

    fn resize_dialog(&mut self, ctx: &Context, st: &mut ResizeState) -> Outcome {
        chrome(ctx, self.dialog_nonce, "Resize and Skew", 300.0, |ui| {
            group_box(ui, "Resize", |ui| {
                ui.horizontal(|ui| {
                    ui.label("By:");
                    ui.add_space(16.0);
                    let was = st.by_percent;
                    ui.radio_value(&mut st.by_percent, true, "Percentage");
                    ui.radio_value(&mut st.by_percent, false, "Pixels");
                    if was != st.by_percent {
                        if st.by_percent {
                            st.h = "100".into();
                            st.v = "100".into();
                        } else {
                            st.h = st.base_w.to_string();
                            st.v = st.base_h.to_string();
                        }
                    }
                });
                egui::Grid::new("resize-grid").num_columns(3).spacing(vec2(10.0, 6.0)).show(ui, |ui| {
                    let unit = if st.by_percent { "%" } else { "px" };
                    ui.label("Horizontal:");
                    if num_field(ui, &mut st.h) && st.keep {
                        if let Ok(v) = st.h.trim().parse::<f32>() {
                            st.v = if st.by_percent { format!("{}", v.round()) } else { format!("{}", (v * st.base_h as f32 / st.base_w as f32).round()) };
                        }
                    }
                    ui.label(unit);
                    ui.end_row();
                    ui.label("Vertical:");
                    if num_field(ui, &mut st.v) && st.keep {
                        if let Ok(v) = st.v.trim().parse::<f32>() {
                            st.h = if st.by_percent { format!("{}", v.round()) } else { format!("{}", (v * st.base_w as f32 / st.base_h as f32).round()) };
                        }
                    }
                    ui.label(unit);
                    ui.end_row();
                });
                ui.checkbox(&mut st.keep, "Maintain aspect ratio");
            });
            ui.add_space(6.0);
            group_box(ui, "Skew (Degrees)", |ui| {
                egui::Grid::new("skew-grid").num_columns(3).spacing(vec2(10.0, 6.0)).show(ui, |ui| {
                    ui.label("Horizontal:");
                    num_field(ui, &mut st.skew_h);
                    ui.label("°");
                    ui.end_row();
                    ui.label("Vertical:");
                    num_field(ui, &mut st.skew_v);
                    ui.label("°");
                    ui.end_row();
                });
            });
            match buttons_row(ui, &["OK", "Cancel"]) {
                Some(0) => {
                    let h: f32 = st.h.trim().parse().unwrap_or(100.0);
                    let v: f32 = st.v.trim().parse().unwrap_or(100.0);
                    let sh: f32 = st.skew_h.trim().parse::<f32>().unwrap_or(0.0).clamp(-89.0, 89.0);
                    let sv: f32 = st.skew_v.trim().parse::<f32>().unwrap_or(0.0).clamp(-89.0, 89.0);
                    let (w, hh) = if st.by_percent {
                        let h = h.clamp(1.0, 500.0);
                        let v = v.clamp(1.0, 500.0);
                        ((st.base_w as f32 * h / 100.0).round().max(1.0) as u32, (st.base_h as f32 * v / 100.0).round().max(1.0) as u32)
                    } else {
                        (h.max(1.0) as u32, v.max(1.0) as u32)
                    };
                    if w != st.base_w || hh != st.base_h || sh != 0.0 || sv != 0.0 {
                        self.resize_skew(w, hh, sh, sv);
                    }
                    Outcome::Close
                }
                Some(_) => Outcome::Close,
                None => Outcome::Keep,
            }
        })
    }

    fn properties_dialog(&mut self, ctx: &Context, st: &mut PropState) -> Outcome {
        let last_saved = self
            .doc
            .saved_at
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| format_time(d.as_secs()))
            .unwrap_or_else(|| "Not Available".into());
        let size = self.doc.file_size.map(|s| format!("{} bytes", group_digits(s))).unwrap_or_else(|| "Not Available".into());
        chrome(ctx, self.dialog_nonce, "Image Properties", 340.0, |ui| {
            group_box(ui, "File attributes", |ui| {
                egui::Grid::new("attr").num_columns(2).spacing(vec2(16.0, 4.0)).show(ui, |ui| {
                    ui.label("Last saved:");
                    ui.label(&last_saved);
                    ui.end_row();
                    ui.label("Size on disk:");
                    ui.label(&size);
                    ui.end_row();
                    ui.label("Resolution:");
                    ui.label("96 DPI");
                    ui.end_row();
                });
            });
            ui.add_space(4.0);
            let factor = |u: Units| match u {
                Units::Inches => 1.0 / 96.0,
                Units::Centimeters => 2.54 / 96.0,
                Units::Pixels => 1.0,
            };
            ui.horizontal(|ui| {
                ui.label("Units:");
                let before = st.units;
                ui.radio_value(&mut st.units, Units::Inches, "Inches");
                ui.radio_value(&mut st.units, Units::Centimeters, "Centimeters");
                ui.radio_value(&mut st.units, Units::Pixels, "Pixels");
                if before != st.units {
                    let f = factor(st.units);
                    let fmt = |v: f32| if st.units == Units::Pixels { format!("{}", v.round()) } else { format!("{:.2}", v) };
                    st.w = fmt(st.px_w * f);
                    st.h = fmt(st.px_h * f);
                }
            });
            ui.horizontal(|ui| {
                ui.label("Colors:");
                ui.radio_value(&mut st.color, false, "Black and white");
                ui.radio_value(&mut st.color, true, "Color");
            });
            ui.horizontal(|ui| {
                ui.label("Width:");
                if num_field(ui, &mut st.w) {
                    if let Ok(v) = st.w.trim().parse::<f32>() {
                        st.px_w = v / factor(st.units);
                    }
                }
                ui.add_space(8.0);
                ui.label("Height:");
                if num_field(ui, &mut st.h) {
                    if let Ok(v) = st.h.trim().parse::<f32>() {
                        st.px_h = v / factor(st.units);
                    }
                }
            });
            match buttons_row(ui, &["OK", "Cancel", "Default"]) {
                Some(0) => {
                    let (w, h) = (st.px_w.round().max(1.0) as u32, st.px_h.round().max(1.0) as u32);
                    self.set_canvas_size(w, h);
                    if !st.color {
                        self.to_black_and_white();
                    }
                    Outcome::Close
                }
                Some(2) => {
                    let (w, h) = crate::app::DEFAULT_CANVAS;
                    st.px_w = w as f32;
                    st.px_h = h as f32;
                    st.units = Units::Pixels;
                    st.w = w.to_string();
                    st.h = h.to_string();
                    Outcome::Keep
                }
                Some(_) => Outcome::Close,
                None => Outcome::Keep,
            }
        })
    }

    fn to_black_and_white(&mut self) {
        self.commit_all();
        self.push_undo();
        for l in &mut self.layers {
            for px in l.pix.data_mut().chunks_exact_mut(4) {
                if px[3] == 0 {
                    continue;
                }
                // premultiplied: compare luminance against half the alpha
                let lum = 0.299 * px[0] as f32 + 0.587 * px[1] as f32 + 0.114 * px[2] as f32;
                let v = if lum >= px[3] as f32 * 0.5 { px[3] } else { 0 };
                px[0] = v;
                px[1] = v;
                px[2] = v;
            }
        }
        self.display_dirty = true;
    }

    fn colors_dialog(&mut self, ctx: &Context, st: &mut ColorState) -> Outcome {
        const SW: f32 = 175.0;
        const SH: f32 = 187.0;
        if st.spectrum.is_none() {
            let (w, h) = (SW as usize, SH as usize);
            let mut img = egui::ColorImage::new([w, h], Color32::BLACK);
            for y in 0..h {
                for x in 0..w {
                    let c = hsl_to_rgb(x as f32 / w as f32 * 240.0, 240.0 - y as f32 / (h - 1) as f32 * 240.0, 120.0);
                    img.pixels[y * w + x] = Color32::from_rgb(c[0], c[1], c[2]);
                }
            }
            st.spectrum = Some(ctx.load_texture("spectrum", img, TextureOptions::LINEAR));
        }
        let mut result = Outcome::Keep;
        let out = chrome(ctx, self.dialog_nonce, "Edit Colors", 456.0, |ui| {
            ui.horizontal_top(|ui| {
                // left column: basic & custom colors
                ui.vertical(|ui| {
                    ui.set_width(220.0);
                    ui.label("Basic colors:");
                    for row in 0..6 {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for col in 0..8 {
                                let c = hex(BASIC_COLORS[row * 8 + col]);
                                let (r, resp) = ui.allocate_exact_size(vec2(21.0, 17.0), Sense::click());
                                let sel = st.rgb == [c.r(), c.g(), c.b()];
                                color_cell(ui, r, c, sel || resp.hovered());
                                if resp.clicked() {
                                    st.rgb = [c.r(), c.g(), c.b()];
                                    (st.h, st.s, st.l) = rgb_to_hsl(st.rgb);
                                }
                            }
                        });
                    }
                    ui.add_space(8.0);
                    ui.label("Custom colors:");
                    for row in 0..2 {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 5.0;
                            for col in 0..8 {
                                let i = row * 8 + col;
                                let c = st.custom[i];
                                let (r, resp) = ui.allocate_exact_size(vec2(21.0, 17.0), Sense::click());
                                color_cell(ui, r, c, st.custom_sel == i || resp.hovered());
                                if resp.clicked() {
                                    st.custom_sel = i;
                                    st.rgb = [c.r(), c.g(), c.b()];
                                    (st.h, st.s, st.l) = rgb_to_hsl(st.rgb);
                                }
                            }
                        });
                    }
                    ui.add_space(10.0);
                    ui.add_enabled(false, egui::Button::new("Define Custom Colors >>").min_size(vec2(214.0, 23.0)));
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if win_button(ui, "OK", true).clicked() {
                            result = Outcome::Close;
                            let c = Color32::from_rgb(st.rgb[0], st.rgb[1], st.rgb[2]);
                            self.set_color(st.slot, c);
                            self.dialog_custom = st.custom;
                            if !PALETTE_CONTAINS(c) && !self.custom_colors.contains(&Some(c)) {
                                self.add_custom_color(c);
                            }
                        }
                        if win_button(ui, "Cancel", false).clicked() {
                            result = Outcome::Close;
                        }
                    });
                });
                ui.add_space(6.0);
                // right column: spectrum, luminance and numeric fields
                ui.vertical(|ui| {
                    ui.horizontal_top(|ui| {
                        let (sr, sresp) = ui.allocate_exact_size(vec2(SW, SH), Sense::click_and_drag());
                        let p = ui.painter();
                        if let Some(t) = &st.spectrum {
                            p.image(t.id(), sr, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                        }
                        p.rect_stroke(sr, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(120, 120, 120)), StrokeKind::Outside);
                        if let Some(pp) = sresp.interact_pointer_pos() {
                            if sresp.is_pointer_button_down_on() || sresp.clicked() {
                                let rel = ((pp - sr.min) / sr.size()).clamp(vec2(0.0, 0.0), vec2(1.0, 1.0));
                                st.h = (rel.x * 239.0).round();
                                st.s = (240.0 - rel.y * 240.0).round();
                                if st.l <= 0.0 || st.l >= 240.0 {
                                    st.l = 120.0;
                                }
                                st.rgb = hsl_to_rgb(st.h, st.s, st.l);
                            }
                        }
                        let cx = sr.left() + st.h / 239.0 * SW;
                        let cy = sr.top() + (240.0 - st.s) / 240.0 * SH;
                        let pc = ui.painter().with_clip_rect(sr);
                        for (a, b) in [((-9.0, 0.0), (-3.0, 0.0)), ((3.0, 0.0), (9.0, 0.0)), ((0.0, -9.0), (0.0, -3.0)), ((0.0, 3.0), (0.0, 9.0))] {
                            pc.line_segment([pos2(cx + a.0, cy + a.1), pos2(cx + b.0, cy + b.1)], Stroke::new(3.0_f32, Color32::BLACK));
                        }
                        ui.add_space(4.0);
                        let (lr, lresp) = ui.allocate_exact_size(vec2(26.0, SH), Sense::click_and_drag());
                        let bar = Rect::from_min_size(lr.min, vec2(12.0, SH));
                        let p = ui.painter();
                        let n = SH as usize;
                        for i in 0..n {
                            let l = 240.0 - i as f32 / (n - 1) as f32 * 240.0;
                            let c = hsl_to_rgb(st.h, st.s, l);
                            p.rect_filled(Rect::from_min_size(pos2(bar.left(), bar.top() + i as f32), vec2(12.0, 1.0)), 0.0, Color32::from_rgb(c[0], c[1], c[2]));
                        }
                        p.rect_stroke(bar, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(120, 120, 120)), StrokeKind::Outside);
                        if let Some(pp) = lresp.interact_pointer_pos() {
                            if lresp.is_pointer_button_down_on() || lresp.clicked() {
                                let rel = ((pp.y - lr.top()) / SH).clamp(0.0, 1.0);
                                st.l = (240.0 - rel * 240.0).round();
                                st.rgb = hsl_to_rgb(st.h, st.s, st.l);
                            }
                        }
                        let ay = lr.top() + (240.0 - st.l) / 240.0 * SH;
                        let ax = bar.right() + 3.0;
                        p.add(egui::Shape::convex_polygon(vec![pos2(ax, ay), pos2(ax + 7.0, ay - 5.0), pos2(ax + 7.0, ay + 5.0)], Color32::BLACK, Stroke::NONE));
                    });
                    ui.add_space(6.0);
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            let (pr, _) = ui.allocate_exact_size(vec2(64.0, 42.0), Sense::hover());
                            let c = Color32::from_rgb(st.rgb[0], st.rgb[1], st.rgb[2]);
                            ui.painter().rect_filled(pr, 0.0, c);
                            ui.painter().rect_stroke(pr, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(120, 120, 120)), StrokeKind::Outside);
                            ui.label("Color|Solid");
                        });
                        ui.add_space(4.0);
                        egui::Grid::new("hsl-rgb").num_columns(4).spacing(vec2(4.0, 4.0)).show(ui, |ui| {
                            let mut hsl_changed = false;
                            let mut rgb_changed = false;
                            let mut r = st.rgb[0] as f32;
                            let mut g = st.rgb[1] as f32;
                            let mut b = st.rgb[2] as f32;
                            ui.label("Hue:");
                            hsl_changed |= ui.add(egui::DragValue::new(&mut st.h).range(0.0..=239.0).speed(1.0)).changed();
                            ui.label("Red:");
                            rgb_changed |= ui.add(egui::DragValue::new(&mut r).range(0.0..=255.0).speed(1.0)).changed();
                            ui.end_row();
                            ui.label("Sat:");
                            hsl_changed |= ui.add(egui::DragValue::new(&mut st.s).range(0.0..=240.0).speed(1.0)).changed();
                            ui.label("Green:");
                            rgb_changed |= ui.add(egui::DragValue::new(&mut g).range(0.0..=255.0).speed(1.0)).changed();
                            ui.end_row();
                            ui.label("Lum:");
                            hsl_changed |= ui.add(egui::DragValue::new(&mut st.l).range(0.0..=240.0).speed(1.0)).changed();
                            ui.label("Blue:");
                            rgb_changed |= ui.add(egui::DragValue::new(&mut b).range(0.0..=255.0).speed(1.0)).changed();
                            ui.end_row();
                            if rgb_changed {
                                st.rgb = [r.round() as u8, g.round() as u8, b.round() as u8];
                                (st.h, st.s, st.l) = rgb_to_hsl(st.rgb);
                            } else if hsl_changed {
                                st.rgb = hsl_to_rgb(st.h, st.s, st.l);
                            }
                        });
                    });
                    ui.add_space(6.0);
                    if ui.add(egui::Button::new("Add to Custom Colors").min_size(vec2(220.0, 23.0))).clicked() {
                        st.custom[st.custom_sel] = Color32::from_rgb(st.rgb[0], st.rgb[1], st.rgb[2]);
                        st.custom_sel = (st.custom_sel + 1) % 16;
                    }
                });
            });
            Outcome::Keep
        });
        match (out, result) {
            (Outcome::Close, _) | (_, Outcome::Close) => Outcome::Close,
            _ => Outcome::Keep,
        }
    }

    fn unsaved_dialog(&mut self, ctx: &Context, action: PendingAction) -> Outcome {
        let name = self.doc.name();
        let mut choice = None;
        let out = chrome(ctx, self.dialog_nonce, "Free Paint", 380.0, |ui| {
            ui.add_space(16.0);
            ui.label(RichText::new(format!("Do you want to save changes to {name}?")).size(15.0).color(if theme::modern() { theme::w().accent } else { Color32::from_rgb(0, 51, 153) }));
            ui.add_space(10.0);
            choice = buttons_row(ui, &["Save", "Don't Save", "Cancel"]);
            Outcome::Keep
        });
        match choice {
            Some(0) => {
                if self.save() {
                    self.perform(ctx, action);
                }
                Outcome::Close
            }
            Some(1) => {
                self.doc.dirty = false;
                self.perform(ctx, action);
                Outcome::Close
            }
            Some(_) => Outcome::Close,
            None => out,
        }
    }

    fn about_dialog(&mut self, ctx: &Context) -> Outcome {
        let mut close = false;
        let out = chrome(ctx, self.dialog_nonce, "About Free Paint", 360.0, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(10.0);
                let (r, _) = ui.allocate_exact_size(vec2(72.0, 72.0), Sense::hover());
                self.icons.paint(ui.painter(), Icon::App, r, true);
                ui.add_space(8.0);
                ui.label(RichText::new("Free Paint").font(theme::bold(20.0)));
                ui.add_space(2.0);
                ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
                ui.add_space(8.0);
                ui.label(RichText::new("Built by nomadslayer").font(theme::bold(13.0)));
                ui.scope(|ui| {
                    ui.visuals_mut().override_text_color = None;
                    ui.hyperlink("https://github.com/nomadslayer");
                    ui.add_space(2.0);
                    ui.hyperlink_to("Source: github.com/nomadslayer/freepaint", "https://github.com/nomadslayer/freepaint");
                });
                ui.add_space(8.0);
                ui.label("A classic ribbon-style paint program for macOS and Linux, written in Rust.");
                ui.add_space(6.0);
                ui.label(RichText::new("UI font: Selawik, © Microsoft, SIL Open Font License 1.1.").size(11.0).color(theme::disabled_color()));
                ui.add_space(4.0);
            });
            if buttons_row(ui, &["OK"]).is_some() {
                close = true;
            }
            Outcome::Keep
        });
        if close {
            Outcome::Close
        } else {
            out
        }
    }

    fn message_dialog(&mut self, ctx: &Context, msg: &str) -> Outcome {
        let mut close = false;
        let out = chrome(ctx, self.dialog_nonce, "Free Paint", 400.0, |ui| {
            ui.horizontal_top(|ui| {
                let (r, _) = ui.allocate_exact_size(vec2(32.0, 32.0), Sense::hover());
                ui.painter().circle_filled(r.center(), 15.0, Color32::from_rgb(200, 30, 30));
                ui.painter().text(r.center(), Align2::CENTER_CENTER, "✖", egui::FontId::proportional(16.0), Color32::WHITE);
                ui.add_space(8.0);
                ui.label(msg);
            });
            if buttons_row(ui, &["OK"]).is_some() {
                close = true;
            }
            Outcome::Keep
        });
        if close {
            Outcome::Close
        } else {
            out
        }
    }
}

#[allow(non_snake_case)]
fn PALETTE_CONTAINS(c: Color32) -> bool {
    crate::app::PALETTE.iter().flatten().any(|p| *p == c)
}

fn color_cell(ui: &Ui, r: Rect, c: Color32, highlight: bool) {
    let p = ui.painter();
    p.rect_filled(r, 0.0, c);
    p.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(110, 110, 110)), StrokeKind::Inside);
    if highlight {
        p.rect_stroke(r.expand(2.0), 0.0, Stroke::new(1.0_f32, Color32::BLACK), StrokeKind::Inside);
    }
}

fn group_digits(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::new();
    for (i, ch) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// Format a UNIX timestamp as "YYYY-MM-DD HH:MM" (UTC) without extra deps.
fn format_time(secs: u64) -> String {
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m) = (rem / 3600, (rem % 3600) / 60);
    // civil-from-days (Howard Hinnant)
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mo = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if mo <= 2 { y + 1 } else { y };
    format!("{y:04}-{mo:02}-{d:02} {h:02}:{m:02} UTC")
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn print_arguments() {
        let a = print_args("Office", 2, true, false, "/tmp/x.png");
        assert_eq!(a, ["-d", "Office", "-n", "2", "-t", "Free Paint", "-o", "fit-to-page", "/tmp/x.png"]);
        let a = print_args("P", 1, false, true, "f.png");
        assert_eq!(a, ["-d", "P", "-n", "1", "-t", "Free Paint", "-o", "ppi=96", "-o", "landscape", "f.png"]);
    }

    #[test]
    fn hsl_round_trip() {
        for c in [[255, 0, 0], [0, 128, 255], [12, 200, 90], [255, 255, 255], [0, 0, 0], [128, 128, 128]] {
            let (h, s, l) = rgb_to_hsl(c);
            let back = hsl_to_rgb(h, s, l);
            for i in 0..3 {
                assert!((back[i] as i32 - c[i] as i32).abs() <= 3, "{c:?} -> {back:?}");
            }
        }
    }
}
