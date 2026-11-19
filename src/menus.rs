//! File menu, canvas context menu, thumbnail window and status bar.

use egui::{pos2, vec2, Align2, Color32, Context, Frame, Id, Order, Rect, Sense, Stroke, StrokeKind, Ui};

use crate::app::{PaintApp, PendingAction, RotOp, TAB_H, ZOOM_LEVELS};
use crate::dialogs::Dialog;
use crate::icons::Icon;
use crate::theme;
use crate::widgets::{self, menu_item, menu_item_ex, menu_separator};

#[derive(Clone, Copy, PartialEq, Eq)]
enum FileItem {
    New,
    Open,
    Save,
    SaveAs,
    Print,
    Properties,
    CheckUpdates,
    About,
    Exit,
}

impl PaintApp {
    pub fn file_menu_ui(&mut self, ctx: &Context) {
        if !self.file_menu_open {
            return;
        }
        let items = [
            (FileItem::New, Icon::New, "New", true, false),
            (FileItem::Open, Icon::Open, "Open", true, false),
            (FileItem::Save, Icon::Save, "Save", true, false),
            (FileItem::SaveAs, Icon::SaveAs, "Save as", true, true),
            (FileItem::Print, Icon::Print, "Print", true, false),
            (FileItem::Properties, Icon::Properties, "Properties", true, false),
            (FileItem::CheckUpdates, Icon::Redo, "Check for updates", true, false),
            (FileItem::About, Icon::About, "About Free Paint", true, false),
            (FileItem::Exit, Icon::Exit, "Exit", true, false),
        ];
        let mut action: Option<FileItem> = None;
        let mut save_fmt: Option<&'static str> = None;
        let mut open_recent: Option<std::path::PathBuf> = None;
        let frame = Frame::NONE
            .fill(Color32::WHITE)
            .stroke(Stroke::new(1.0_f32, theme::MENU_BORDER))
            .shadow(egui::Shadow { offset: [2, 3], blur: 8, spread: 0, color: Color32::from_black_alpha(50) });
        let area = egui::Area::new(Id::new("file-menu")).order(Order::Foreground).fixed_pos(pos2(0.0, TAB_H - 1.0)).show(ctx, |ui| {
            frame.show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.horizontal_top(|ui| {
                    let left = ui.vertical(|ui| {
                        ui.set_width(200.0);
                        ui.add_space(4.0);
                        for (item, icon, label, enabled, sub) in items {
                            let r = menu_item_ex(ui, &mut self.icons, Some(icon), label, false, enabled, sub, 44.0, 32.0);
                            if r.hovered() {
                                self.file_menu_saveas = item == FileItem::SaveAs;
                            }
                            if r.clicked() && item != FileItem::SaveAs {
                                action = Some(item);
                            }
                            if item == FileItem::Print || item == FileItem::Properties {
                                menu_separator(ui);
                            }
                        }
                        ui.add_space(4.0);
                    });
                    let h = left.response.rect.height();
                    let (sep, _) = ui.allocate_exact_size(vec2(1.0, h), Sense::hover());
                    ui.painter().rect_filled(sep, 0.0, Color32::from_rgb(226, 226, 226));
                    ui.vertical(|ui| {
                        ui.set_width(300.0);
                        ui.set_min_height(h);
                        let (hr, _) = ui.allocate_exact_size(vec2(300.0, 30.0), Sense::hover());
                        let p = ui.painter();
                        let title = if self.file_menu_saveas { "Save a copy of the document" } else { "Recent pictures" };
                        p.text(pos2(hr.left() + 12.0, hr.center().y + 2.0), Align2::LEFT_CENTER, title, theme::bold(12.0), theme::TEXT);
                        let y = hr.bottom() - 0.5;
                        p.line_segment([pos2(hr.left() + 10.0, y), pos2(hr.right() - 10.0, y)], Stroke::new(1.0_f32, Color32::from_rgb(226, 226, 226)));
                        ui.add_space(4.0);
                        if self.file_menu_saveas {
                            let fmts = [
                                ("png", "PNG picture", "Save a photo or drawing with high quality and use it on your computer or on the web."),
                                ("jpg", "JPEG picture", "Save a photo with good quality and use it on your computer, in email, or on the web."),
                                ("bmp", "BMP picture", "Save any kind of picture with high quality and use it on your computer."),
                                ("gif", "GIF picture", "Save a simple drawing with lower quality and use it in email or on the web."),
                                ("tiff", "Other formats", "Open the Save As dialog box to select from all possible file types."),
                            ];
                            for (ext, name, desc) in fmts {
                                let (r, resp) = ui.allocate_exact_size(vec2(300.0, 52.0), Sense::click());
                                let p = ui.painter();
                                if resp.hovered() {
                                    p.rect_filled(r.shrink(2.0), 0.0, theme::HOVER_BG);
                                    p.rect_stroke(r.shrink(2.0), 0.0, Stroke::new(1.0_f32, theme::HOVER_BORDER), StrokeKind::Inside);
                                }
                                self.icons.paint(p, Icon::Picture, Rect::from_min_size(pos2(r.left() + 10.0, r.top() + 10.0), vec2(32.0, 32.0)), true);
                                p.text(pos2(r.left() + 52.0, r.top() + 12.0), Align2::LEFT_CENTER, name, theme::bold(12.0), theme::TEXT);
                                let galley = p.layout(desc.to_string(), egui::FontId::proportional(11.0), Color32::from_rgb(90, 90, 90), 236.0);
                                p.galley(pos2(r.left() + 52.0, r.top() + 21.0), galley, Color32::from_rgb(90, 90, 90));
                                if resp.clicked() {
                                    save_fmt = Some(if ext == "tiff" { "" } else { ext });
                                }
                            }
                        } else if self.recent.is_empty() {
                            ui.add_space(6.0);
                            ui.horizontal(|ui| {
                                ui.add_space(12.0);
                                ui.label(egui::RichText::new("No recent pictures").color(Color32::from_rgb(120, 120, 120)));
                            });
                        } else {
                            for (i, path) in self.recent.clone().iter().enumerate() {
                                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                let (r, resp) = ui.allocate_exact_size(vec2(300.0, 24.0), Sense::click());
                                let p = ui.painter();
                                if resp.hovered() {
                                    p.rect_filled(r.shrink2(vec2(4.0, 1.0)), 0.0, theme::HOVER_BG);
                                    p.rect_stroke(r.shrink2(vec2(4.0, 1.0)), 0.0, Stroke::new(1.0_f32, theme::HOVER_BORDER), StrokeKind::Inside);
                                }
                                p.text(pos2(r.left() + 14.0, r.center().y), Align2::LEFT_CENTER, format!("{}", i + 1), widgets::font(), theme::TEXT);
                                p.text(pos2(r.left() + 32.0, r.center().y), Align2::LEFT_CENTER, &name, widgets::font(), theme::TEXT);
                                let resp = resp.on_hover_text(path.display().to_string());
                                if resp.clicked() {
                                    open_recent = Some(path.clone());
                                }
                            }
                        }
                    });
                });
            });
        });
        let file_tab = Rect::from_min_size(pos2(0.0, 0.0), vec2(56.0, TAB_H));
        let pressed_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p) && !file_tab.contains(p)));
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if pressed_outside || esc {
            self.file_menu_open = false;
        }
        if let Some(fmt) = save_fmt {
            self.file_menu_open = false;
            self.save_as(if fmt.is_empty() { None } else { Some(fmt) });
        }
        if let Some(p) = open_recent {
            self.file_menu_open = false;
            self.request(ctx, PendingAction::OpenPath(p));
        }
        if let Some(a) = action {
            self.file_menu_open = false;
            match a {
                FileItem::New => self.request(ctx, PendingAction::New),
                FileItem::Open => self.request(ctx, PendingAction::Open),
                FileItem::Save => {
                    self.save();
                }
                FileItem::SaveAs => {}
                FileItem::Print => self.dialog = Some(Dialog::print(self)),
                FileItem::Properties => self.dialog = Some(Dialog::properties(self)),
                FileItem::CheckUpdates => self.start_update_check(true),
                FileItem::About => self.dialog = Some(Dialog::About),
                FileItem::Exit => self.request(ctx, PendingAction::Exit),
            }
        }
    }

    pub fn context_menu_ui(&mut self, ctx: &Context) {
        let Some(pos) = self.context_menu else {
            self.context_menu_frame = None;
            return;
        };
        let first_frame = *self.context_menu_frame.get_or_insert(self.frame_no) == self.frame_no;
        let has_sel = self.selection.is_some();
        let mut close = false;
        let area = egui::Area::new(Id::new("ctx-menu")).order(Order::Foreground).fixed_pos(pos).constrain(true).show(ctx, |ui| {
            widgets::popup_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.set_width(200.0);
                let item = |ui: &mut Ui, app: &mut PaintApp, icon: Option<Icon>, label: &str, en: bool| menu_item(ui, &mut app.icons, icon, label, false, en).clicked();
                if item(ui, self, Some(Icon::Cut), "Cut", has_sel) {
                    self.cut();
                    close = true;
                }
                if item(ui, self, Some(Icon::Copy), "Copy", has_sel) {
                    self.copy();
                    close = true;
                }
                if item(ui, self, Some(Icon::Paste), "Paste", true) {
                    self.paste(&ui.ctx().clone());
                    close = true;
                }
                menu_separator(ui);
                if item(ui, self, Some(Icon::SelectAll), "Select all", true) {
                    self.select_all();
                    close = true;
                }
                if item(ui, self, Some(Icon::InvertSelection), "Invert selection", true) {
                    self.invert_selection();
                    close = true;
                }
                if item(ui, self, Some(Icon::Delete), "Delete", has_sel) {
                    self.delete_selection();
                    close = true;
                }
                menu_separator(ui);
                for (icon, label, op) in [
                    (Icon::RotateRight, "Rotate right 90°", RotOp::Right),
                    (Icon::RotateLeft, "Rotate left 90°", RotOp::Left),
                    (Icon::Rotate180, "Rotate 180°", RotOp::R180),
                    (Icon::FlipVertical, "Flip vertical", RotOp::FlipV),
                    (Icon::FlipHorizontal, "Flip horizontal", RotOp::FlipH),
                ] {
                    if item(ui, self, Some(icon), label, true) {
                        self.rotate(op);
                        close = true;
                    }
                }
                menu_separator(ui);
                if item(ui, self, Some(Icon::Resize), "Resize", true) {
                    self.dialog = Some(Dialog::resize(self));
                    close = true;
                }
                if item(ui, self, Some(Icon::InvertColor), "Invert color", true) {
                    self.invert_colors();
                    close = true;
                }
            });
        });
        let pressed_outside = !first_frame && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p)));
        if close || pressed_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.context_menu = None;
        }
    }

    pub fn thumbnail_ui(&mut self, ctx: &Context) {
        if !self.show_thumbnail {
            return;
        }
        let mut open = true;
        let screen = ctx.screen_rect();
        egui::Window::new("Thumbnail")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([220.0, 160.0])
            .default_pos(pos2(screen.right() - 250.0, 140.0))
            .show(ctx, |ui| {
                let avail = ui.available_size().max(vec2(60.0, 40.0));
                let (w, h) = (self.img_w() as f32, self.img_h() as f32);
                let s = (avail.x / w).min(avail.y / h);
                let (r, _) = ui.allocate_exact_size(vec2(w * s, h * s), Sense::hover());
                if let Some(t) = &self.texture {
                    ui.painter().image(t.id(), r, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
            });
        if !open {
            self.show_thumbnail = false;
        }
    }

    // ----- status bar ----------------------------------------------------------

    pub(crate) fn selection_size_text(&self) -> Option<String> {
        let wh = if let Some(m) = self.marquee {
            Some((m.w, m.h))
        } else if !self.lasso.is_empty() {
            let minx = self.lasso.iter().map(|p| p.x).fold(f32::MAX, f32::min);
            let maxx = self.lasso.iter().map(|p| p.x).fold(f32::MIN, f32::max);
            let miny = self.lasso.iter().map(|p| p.y).fold(f32::MAX, f32::min);
            let maxy = self.lasso.iter().map(|p| p.y).fold(f32::MIN, f32::max);
            Some(((maxx - minx).round() as i32, (maxy - miny).round() as i32))
        } else if let Some(s) = &self.selection {
            Some((s.rect.w, s.rect.h))
        } else if let Some(sh) = &self.active_shape {
            let d = sh.b - sh.a;
            Some((d.x.abs() as i32 + 1, d.y.abs() as i32 + 1))
        } else {
            self.text_box.as_ref().map(|tb| (tb.rect.w, tb.rect.h))
        };
        wh.map(|(w, h)| format!("{w} × {h}px"))
    }

    pub fn status_ui(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::STATUS_BG);
        p.line_segment([pos2(rect.left(), rect.top() + 0.5), pos2(rect.right(), rect.top() + 0.5)], Stroke::new(1.0_f32, theme::RIBBON_BORDER));
        let cy = rect.center().y + 0.5;
        let mut x = rect.left();
        let canvas = self.canvas_size_for_status();
        let segs: [(Icon, Option<String>, f32); 4] = [
            (Icon::StatusCursor, self.cursor_px.map(|(x, y)| format!("{x}, {y}px")), 150.0),
            (Icon::StatusSelection, self.selection_size_text(), 150.0),
            (Icon::StatusCanvas, Some(format!("{} × {}px", canvas.0, canvas.1)), 150.0),
            (Icon::StatusDisk, self.doc.file_size.map(|s| format!("Size: {}", fmt_size(s))), 160.0),
        ];
        for (icon, text, w) in segs {
            self.icons.paint(&p, icon, Rect::from_min_size(pos2(x + 6.0, cy - 8.0), vec2(16.0, 16.0)), true);
            if let Some(t) = text {
                p.text(pos2(x + 28.0, cy), Align2::LEFT_CENTER, t, widgets::font(), theme::TEXT);
            }
            x += w;
            let sx = x.round() + 0.5;
            p.line_segment([pos2(sx, rect.top() + 4.0), pos2(sx, rect.bottom() - 3.0)], Stroke::new(1.0_f32, theme::STATUS_SEP));
        }

        // zoom controls on the right
        let right = rect.right() - 12.0;
        let plus_c = pos2(right - 8.0, cy);
        let track_r = right - 22.0;
        let track_l = track_r - 110.0;
        let minus_c = pos2(track_l - 14.0, cy);
        let label_x = minus_c.x - 14.0;
        let ztext = if self.zoom < 1.0 { format!("{}%", (self.zoom * 100.0 * 10.0).round() / 10.0) } else { format!("{}%", (self.zoom * 100.0).round()) };
        p.text(pos2(label_x, cy), Align2::RIGHT_CENTER, ztext, widgets::font(), theme::TEXT);
        for (c, plus) in [(minus_c, false), (plus_c, true)] {
            let r = Rect::from_center_size(c, vec2(16.0, 16.0));
            let resp = ui.interact(r, Id::new(("zoombtn", plus)), Sense::click());
            let col = if resp.hovered() { Color32::from_rgb(0, 102, 204) } else { Color32::from_rgb(80, 80, 80) };
            p.circle_stroke(c, 6.5, Stroke::new(1.0_f32, col));
            p.line_segment([pos2(c.x - 3.5, c.y), pos2(c.x + 3.5, c.y)], Stroke::new(1.3_f32, col));
            if plus {
                p.line_segment([pos2(c.x, c.y - 3.5), pos2(c.x, c.y + 3.5)], Stroke::new(1.3_f32, col));
            }
            if resp.clicked() {
                self.zoom_step(if plus { 1 } else { -1 }, None);
            }
        }
        let track = Rect::from_min_max(pos2(track_l, cy - 8.0), pos2(track_r, cy + 8.0));
        let resp = ui.interact(track, Id::new("zoom-slider"), Sense::click_and_drag());
        p.line_segment([pos2(track_l, cy), pos2(track_r, cy)], Stroke::new(1.0_f32, Color32::from_rgb(140, 140, 140)));
        let mid = ((track_l + track_r) / 2.0).round() + 0.5;
        p.line_segment([pos2(mid, cy - 4.0), pos2(mid, cy + 4.0)], Stroke::new(1.0_f32, Color32::from_rgb(140, 140, 140)));
        let t = (self.zoom.log2() + 3.0) / 6.0;
        let tx = track_l + t * (track_r - track_l);
        let thumb = widgets::snap(Rect::from_center_size(pos2(tx, cy), vec2(5.0, 14.0)));
        let tc = if resp.hovered() || resp.dragged() { Color32::from_rgb(0, 102, 204) } else { Color32::from_rgb(70, 70, 70) };
        p.rect_filled(thumb, 0.0, tc);
        if let Some(pp) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                let t = ((pp.x - track_l) / (track_r - track_l)).clamp(0.0, 1.0);
                let target = (t * 6.0 - 3.0).exp2();
                let z = ZOOM_LEVELS.iter().copied().min_by(|a, b| (a.log2() - target.log2()).abs().total_cmp(&(b.log2() - target.log2()).abs())).unwrap_or(1.0);
                if (z - self.zoom).abs() > 1e-4 {
                    self.set_zoom(z, None);
                }
            }
        }
    }
}

pub(crate) fn fmt_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / 1024.0 / 1024.0)
    }
}
