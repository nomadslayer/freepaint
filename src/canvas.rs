//! The drawing surface: input handling for every tool plus on-screen overlays.

use egui::{pos2, vec2, Color32, CursorIcon, Id, LayerId, Modifiers, Order, PointerButton, Pos2, Rect, Response, Sense, Shape, Stroke, StrokeKind, Ui, UiBuilder, Vec2};
use tiny_skia::Pixmap;

use crate::app::{ActiveShape, Btn, CurveState, Drag, Handle, PaintApp, PolyState, Selection, Tab, TextBox, Tool, TEXT_PAD};
use crate::brush::{self, BrushStroke};
use crate::icons::Icon;
use crate::raster::{self, rgba, IRect};
use crate::shapes::ShapeKind;
use crate::theme;

const RULER: f32 = 17.0;
const HANDLE: f32 = 5.0;

#[derive(Clone, Copy)]
struct View {
    origin: Pos2,
    zoom: f32,
}

impl View {
    fn to_canvas(&self, p: Pos2) -> Pos2 {
        ((p - self.origin) / self.zoom).to_pos2()
    }
    fn to_screen(&self, c: Pos2) -> Pos2 {
        self.origin + c.to_vec2() * self.zoom
    }
    fn rect(&self, r: IRect) -> Rect {
        Rect::from_min_max(self.to_screen(pos2(r.x as f32, r.y as f32)), self.to_screen(pos2(r.right() as f32, r.bottom() as f32)))
    }
}

fn handle_points(r: Rect) -> [(Handle, Pos2); 8] {
    let c = r.center();
    [
        (Handle::NW, r.left_top()),
        (Handle::N, pos2(c.x, r.top())),
        (Handle::NE, r.right_top()),
        (Handle::E, pos2(r.right(), c.y)),
        (Handle::SE, r.right_bottom()),
        (Handle::S, pos2(c.x, r.bottom())),
        (Handle::SW, r.left_bottom()),
        (Handle::W, pos2(r.left(), c.y)),
    ]
}

fn hit_handle(r: Rect, p: Pos2) -> Option<Handle> {
    handle_points(r).into_iter().find(|(_, hp)| (hp.x - p.x).abs() <= 4.5 && (hp.y - p.y).abs() <= 4.5).map(|(h, _)| h)
}

fn handle_cursor(h: Handle) -> CursorIcon {
    match h {
        Handle::N | Handle::S => CursorIcon::ResizeVertical,
        Handle::E | Handle::W => CursorIcon::ResizeHorizontal,
        Handle::NE | Handle::SW => CursorIcon::ResizeNeSw,
        Handle::NW | Handle::SE => CursorIcon::ResizeNwSe,
        Handle::P0 | Handle::P1 => CursorIcon::Crosshair,
    }
}

fn apply_handle(r: IRect, h: Handle, d: (i32, i32)) -> IRect {
    let (mut x0, mut y0, mut x1, mut y1) = (r.x, r.y, r.right(), r.bottom());
    match h {
        Handle::N => y0 += d.1,
        Handle::S => y1 += d.1,
        Handle::E => x1 += d.0,
        Handle::W => x0 += d.0,
        Handle::NE => {
            y0 += d.1;
            x1 += d.0
        }
        Handle::NW => {
            y0 += d.1;
            x0 += d.0
        }
        Handle::SE => {
            y1 += d.1;
            x1 += d.0
        }
        Handle::SW => {
            y1 += d.1;
            x0 += d.0
        }
        _ => {}
    }
    let r = IRect::from_points((x0, y0), (x1, y1));
    IRect::new(r.x, r.y, r.w.max(1), r.h.max(1))
}

/// Constrain to a square (box shapes) or 45° steps (lines) while Shift is held.
fn constrain(a: Pos2, b: Pos2, line: bool) -> Pos2 {
    let d = b - a;
    if line {
        let ang = d.y.atan2(d.x);
        let step = std::f32::consts::FRAC_PI_4;
        let snapped = (ang / step).round() * step;
        let len = d.length();
        let p = a + vec2(snapped.cos(), snapped.sin()) * len;
        pos2(p.x.round(), p.y.round())
    } else {
        let m = d.x.abs().max(d.y.abs());
        pos2(a.x + m * d.x.signum(), a.y + m * d.y.signum())
    }
}

/// Coordinate of a crisp 1pt line at physical-pixel resolution.
fn crisp(v: f32, ppp: f32) -> f32 {
    let odd = (ppp.round() as i32) % 2 == 1;
    if odd {
        ((v * ppp - 0.5).round() + 0.5) / ppp
    } else {
        (v * ppp).round() / ppp
    }
}

fn crisp_rect(r: Rect, ppp: f32) -> Rect {
    Rect::from_min_max(pos2(crisp(r.min.x, ppp), crisp(r.min.y, ppp)), pos2(crisp(r.max.x, ppp), crisp(r.max.y, ppp)))
}

/// "Marching ants" outline.
fn ants(p: &egui::Painter, pts: &[Pos2], closed: bool) {
    let mut pts = pts.to_vec();
    if closed && !pts.is_empty() {
        pts.push(pts[0]);
    }
    p.add(Shape::line(pts.clone(), Stroke::new(1.0_f32, Color32::WHITE)));
    p.extend(Shape::dashed_line(&pts, Stroke::new(1.0_f32, Color32::from_rgb(20, 20, 20)), 3.0, 3.0));
}

fn ants_rect(p: &egui::Painter, r: Rect, ppp: f32) {
    let r = crisp_rect(r, ppp);
    ants(p, &[r.left_top(), r.right_top(), r.right_bottom(), r.left_bottom()], true);
}

fn draw_handles(p: &egui::Painter, r: Rect, ppp: f32) {
    for (_, hp) in handle_points(r) {
        draw_handle(p, hp, ppp, theme::SELECTION_HANDLE);
    }
}

fn draw_handle(p: &egui::Painter, c: Pos2, ppp: f32, border: Color32) {
    let hr = Rect::from_center_size(pos2(crisp(c.x, ppp), crisp(c.y, ppp)), vec2(HANDLE, HANDLE));
    p.rect_filled(hr, 0.0, Color32::WHITE);
    p.rect_stroke(hr, 0.0, Stroke::new(1.0_f32, border), StrokeKind::Middle);
}

impl PaintApp {
    pub fn canvas_ui(&mut self, ui: &mut Ui) {
        let full = ui.max_rect();
        let ruler = if self.show_rulers { RULER } else { 0.0 };
        let view_rect = Rect::from_min_max(full.min + vec2(ruler, ruler), full.max);
        let zoom = self.zoom;
        let m = self.margin();
        let (mut cw, mut ch) = (self.img_w() as f32, self.img_h() as f32);
        if let Some(r) = self.canvas_resize {
            cw = cw.max(r.right() as f32);
            ch = ch.max(r.bottom() as f32);
        }
        let extra = if theme::modern() { m } else { 40.0 };
        let content = vec2(m + cw * zoom + extra, m + ch * zoom + extra);

        // Wheel over a floating panel adjusts its slider instead of scrolling the canvas.
        let over_overlay = ui.input(|i| i.pointer.hover_pos()).is_some_and(|p| self.overlay_rects_prev.iter().any(|r| r.contains(p)));
        if over_overlay {
            self.slider_wheel += ui.input_mut(|i| {
                let d = i.raw_scroll_delta.y;
                i.raw_scroll_delta = Vec2::ZERO;
                i.smooth_scroll_delta = Vec2::ZERO;
                d
            });
        } else {
            self.slider_wheel = 0.0;
        }
        let mut sa = egui::ScrollArea::both().id_salt("canvas-scroll").auto_shrink([false, false]);
        if let Some(off) = self.pending_scroll.take() {
            sa = sa.scroll_offset(off);
        }
        let out = ui
            .scope_builder(UiBuilder::new().max_rect(view_rect), |ui| {
                sa.show(ui, |ui| {
                    let size = content.max(ui.available_size());
                    let (rect, resp) = ui.allocate_exact_size(size, Sense::click_and_drag());
                    self.canvas_contents(ui, rect, &resp);
                })
            })
            .inner;
        self.scroll_offset = out.state.offset;
        self.viewport = out.inner_rect;
        if self.show_rulers {
            self.draw_rulers(ui, full, view_rect);
        }
        if theme::modern() {
            self.w11_side_panel(ui, view_rect);
            if self.text_box.is_some() {
                self.w11_textbar_ui(ui, view_rect);
            }
        }
    }

    fn canvas_contents(&mut self, ui: &mut Ui, rect: Rect, resp: &Response) {
        let ppp = ui.ctx().pixels_per_point();
        let m = self.margin();
        // The Windows 11 look centers the picture when it is smaller than the view.
        // The offset is frozen while a canvas handle is being dragged.
        if !matches!(self.drag, Drag::CanvasResize { .. }) {
            self.center_offset = if theme::modern() {
                let pic = vec2(self.img_w() as f32, self.img_h() as f32) * self.zoom;
                ((self.viewport.size() - pic) / 2.0).max(Vec2::splat(m))
            } else {
                Vec2::splat(m)
            };
        }
        let o = rect.min + self.center_offset;
        let origin = pos2((o.x * ppp).round() / ppp, (o.y * ppp).round() / ppp);
        self.view_origin = origin;
        let view = View { origin, zoom: self.zoom };

        // Ctrl/Cmd + wheel (or pinch) zooms in steps around the cursor
        if resp.hovered() {
            let zd = ui.input(|i| i.zoom_delta());
            if (zd - 1.0).abs() > 1e-4 {
                self.zoom_accum *= zd;
                if self.zoom_accum > 1.2 || self.zoom_accum < 0.83 {
                    let dir = if self.zoom_accum > 1.0 { 1 } else { -1 };
                    self.zoom_accum = 1.0;
                    if let Some(p) = ui.input(|i| i.pointer.hover_pos()) {
                        let anchor = (view.to_canvas(p), p - self.viewport.min);
                        self.zoom_step(dir, Some(anchor));
                    }
                }
            }
        }

        let hover = ui.input(|i| i.pointer.hover_pos());
        self.cursor_px = hover.filter(|p| resp.hovered() && self.viewport.contains(*p)).map(|p| view.to_canvas(p)).and_then(|c| {
            let (x, y) = (c.x.floor() as i32, c.y.floor() as i32);
            (x >= 0 && y >= 0 && x < self.img_w() as i32 && y < self.img_h() as i32).then_some((x, y))
        });

        if self.dialog.is_none() {
            self.handle_input(ui, resp, view);
        }
        if self.display_dirty {
            self.rebuild_display(ui.ctx());
        }
        self.paint_canvas(ui, view, resp);
    }

    // ----- input ------------------------------------------------------------

    fn handle_input(&mut self, ui: &Ui, resp: &Response, view: View) {
        let (press, rel_l, rel_r, pos, mods, double, dt) = ui.input(|i| {
            let press = if i.pointer.button_pressed(PointerButton::Primary) {
                Some(Btn::Left)
            } else if i.pointer.button_pressed(PointerButton::Secondary) {
                Some(Btn::Right)
            } else {
                None
            };
            (
                press,
                i.pointer.button_released(PointerButton::Primary),
                i.pointer.button_released(PointerButton::Secondary),
                i.pointer.interact_pos().or(i.pointer.hover_pos()),
                i.modifiers,
                i.pointer.button_double_clicked(PointerButton::Primary),
                i.stable_dt,
            )
        });

        if matches!(self.drag, Drag::None) {
            let Some(screen) = pos else { return };
            if let Some(btn) = press {
                let on_overlay = self.overlay_rects_prev.iter().any(|r| r.contains(screen));
                if resp.hovered() && !self.input_blocked && !on_overlay && self.viewport.contains(screen) {
                    let c = view.to_canvas(screen);
                    self.drag_btn = btn;
                    self.drag_cur = c;
                    self.on_press(btn, c, screen, mods, view);
                }
            }
            return;
        }

        let screen = pos.unwrap_or_else(|| view.to_screen(self.drag_cur));
        let c = view.to_canvas(screen);
        let continuous = matches!(self.drag, Drag::Stroke) && self.tool == Tool::Brush && self.brush.continuous();
        if c != self.drag_cur {
            self.on_drag(c, mods, view);
            self.drag_cur = c;
        } else if continuous {
            if let (Some(layer), Some(st)) = (self.layer.as_mut(), self.stroke.as_mut()) {
                st.spray(layer, (c.x, c.y), (dt * 60.0).clamp(0.2, 3.0));
                self.display_dirty = true;
            }
        }
        if continuous {
            ui.ctx().request_repaint();
        }
        let released = match self.drag_btn {
            Btn::Left => rel_l,
            Btn::Right => rel_r,
        };
        let still_down = ui.input(|i| match self.drag_btn {
            Btn::Left => i.pointer.button_down(PointerButton::Primary),
            Btn::Right => i.pointer.button_down(PointerButton::Secondary),
        });
        if released || !still_down {
            self.on_release(c, double, view);
        }
    }

    fn btn_color(&self, btn: Btn) -> Color32 {
        match btn {
            Btn::Left => self.color1,
            Btn::Right => self.color2,
        }
    }

    fn begin_layer(&mut self, opacity: f32) {
        self.commit_all();
        self.layer = Some(Pixmap::new(self.img_w(), self.img_h()).unwrap());
        self.layer_base = Some(self.act().clone());
        self.layer_opacity = opacity;
        self.direct_erase = false;
        self.drag = Drag::Stroke;
        self.display_dirty = true;
    }

    /// Start an eraser stroke. On layers above the background the eraser
    /// writes transparency straight into the layer.
    fn begin_eraser(&mut self) {
        self.commit_all();
        if self.active_layer == 0 {
            self.begin_layer(1.0);
        } else {
            self.layer = None;
            self.layer_base = Some(self.act().clone());
            self.layer_opacity = 1.0;
            self.direct_erase = true;
            self.drag = Drag::Stroke;
            self.display_dirty = true;
        }
    }

    /// Apply one eraser segment in whichever mode is active.
    fn eraser_segment(&mut self, a: (i32, i32), b: (i32, i32), only: Option<egui::Color32>) {
        let size = self.tool_size(3) as i32;
        if self.direct_erase {
            let ai = self.active_layer;
            if let Some(base) = self.layer_base.as_ref() {
                brush::eraser_line(&mut self.layers[ai].pix, base, a, b, size, [0, 0, 0, 0], only);
            }
        } else {
            let col = rgba(self.color2);
            if let (Some(l), Some(base)) = (self.layer.as_mut(), self.layer_base.as_ref()) {
                brush::eraser_line(l, base, a, b, size, col, only);
            }
        }
    }

    fn clamp_px(&self, c: Pos2) -> (i32, i32) {
        (c.x.round().clamp(0.0, self.img_w() as f32) as i32, c.y.round().clamp(0.0, self.img_h() as f32) as i32)
    }

    /// Canvas resize handles: right / bottom / corner in the classic look,
    /// all eight sides and corners in the Windows 11 look.
    fn canvas_handles(&self, view: View) -> Vec<(Handle, Pos2)> {
        let r = view.rect(self.image_irect());
        if theme::modern() {
            handle_points(r.expand(3.0)).to_vec()
        } else {
            vec![(Handle::E, pos2(r.right() + 3.0, r.center().y)), (Handle::S, pos2(r.center().x, r.bottom() + 3.0)), (Handle::SE, pos2(r.right() + 3.0, r.bottom() + 3.0))]
        }
    }

    fn hit_canvas_handle(&self, s: Pos2, view: View) -> Option<Handle> {
        self.canvas_handles(view).into_iter().find(|(_, p)| (p.x - s.x).abs() <= 5.0 && (p.y - s.y).abs() <= 5.0).map(|(h, _)| h)
    }

    fn shape_handle_hit(&self, sh: &ActiveShape, s: Pos2, view: View) -> Option<Handle> {
        if sh.kind == ShapeKind::Line {
            for (h, p) in [(Handle::P0, sh.a), (Handle::P1, sh.b)] {
                let sp = view.to_screen(p);
                if (sp.x - s.x).abs() <= 5.0 && (sp.y - s.y).abs() <= 5.0 {
                    return Some(h);
                }
            }
            None
        } else {
            let r = Rect::from_min_max(view.to_screen(sh.bounds().min), view.to_screen(sh.bounds().max));
            hit_handle(r, s)
        }
    }

    fn on_press(&mut self, btn: Btn, c: Pos2, s: Pos2, mods: Modifiers, view: View) {
        let px = (c.x.floor() as i32, c.y.floor() as i32);
        let cp = pos2(px.0 as f32, px.1 as f32);
        if btn == Btn::Left && self.selection.is_none() && self.text_box.is_none() {
            if let Some(h) = self.hit_canvas_handle(s, view) {
                self.commit_all();
                self.drag = Drag::CanvasResize { handle: h };
                self.canvas_resize = Some(self.image_irect());
                return;
            }
        }
        match self.tool {
            Tool::Select | Tool::FreeSelect => self.press_select(btn, c, s, mods, view),
            Tool::Pencil => {
                self.begin_layer(self.draw_opacity());
                let size = self.tool_size(0) as i32;
                let col = self.btn_color(btn);
                if let Some(l) = self.layer.as_mut() {
                    brush::pencil_line(l, px, px, size, col);
                }
                self.last_pt = cp;
            }
            Tool::Eraser => {
                self.begin_eraser();
                let only = (btn == Btn::Right).then_some(self.color1);
                self.eraser_segment(px, px, only);
                self.last_pt = cp;
            }
            Tool::Brush => {
                self.begin_layer(self.brush.opacity() * self.draw_opacity());
                let seed = self.rng.next_u32() as u64;
                let mut st = BrushStroke::new(self.brush, self.btn_color(btn), self.tool_size(1), seed);
                if let Some(l) = self.layer.as_mut() {
                    st.dab(l, (c.x, c.y));
                }
                self.stroke = Some(st);
                self.last_pt = c;
            }
            Tool::Fill => {
                self.commit_all();
                if let Some(cur) = raster::pixel(self.act(), px.0, px.1) {
                    let col = rgba(self.btn_color(btn));
                    if cur != col {
                        self.push_undo();
                        raster::flood_fill(self.act_mut(), px.0, px.1, col);
                    }
                }
            }
            Tool::Picker => self.drag = Drag::Picker,
            Tool::Magnifier => {
                let anchor = (c, s - self.viewport.min);
                self.zoom_step(if btn == Btn::Left { 1 } else { -1 }, Some(anchor));
            }
            Tool::Text => self.press_text(btn, c, s, view),
            Tool::Shape => self.press_shape(btn, cp, c, s, view),
        }
        self.display_dirty = true;
    }

    fn press_select(&mut self, btn: Btn, c: Pos2, s: Pos2, mods: Modifiers, view: View) {
        if let Some(sel) = &self.selection {
            let sr = view.rect(sel.rect);
            if btn == Btn::Left {
                if let Some(h) = hit_handle(sr, s) {
                    self.lift_selection();
                    let r = self.selection.as_ref().unwrap().rect;
                    self.drag = Drag::SelResize { handle: h, orig: r, start: c };
                    return;
                }
            }
            if sel.rect.contains(c.x, c.y) {
                if btn == Btn::Right {
                    self.context_menu = Some(s);
                    return;
                }
                let was_lifted = sel.content.is_some();
                if mods.command && was_lifted {
                    // Ctrl+drag a floating selection: stamp a copy where it is
                    let sel = self.selection.as_ref().unwrap();
                    let px = self.floating_pixels(sel.content.as_ref().unwrap(), sel.rect);
                    let (x, y) = (sel.rect.x, sel.rect.y);
                    raster::draw_over(self.act_mut(), &px, x, y);
                }
                self.lift_selection();
                if mods.command && !was_lifted {
                    // Ctrl+drag a fresh selection: leave the original in place
                    let sel = self.selection.as_ref().unwrap();
                    let content = sel.content.clone().unwrap();
                    let src = sel.src;
                    raster::draw_over(self.act_mut(), &content, src.x, src.y);
                }
                let r = self.selection.as_ref().unwrap().rect;
                self.drag = Drag::SelMove { start: c, orig: r };
                return;
            }
            self.commit_selection();
        }
        if btn == Btn::Right {
            self.context_menu = Some(s);
            return;
        }
        let p = self.clamp_px(c);
        if self.tool == Tool::Select {
            self.drag = Drag::Marquee { start: p };
            self.marquee = Some(IRect::new(p.0, p.1, 0, 0));
        } else {
            self.drag = Drag::Lasso;
            self.lasso = vec![pos2(p.0 as f32, p.1 as f32)];
        }
    }

    fn press_text(&mut self, btn: Btn, c: Pos2, s: Pos2, view: View) {
        if let Some(tb) = &self.text_box {
            let sr = view.rect(tb.rect);
            if let Some(h) = hit_handle(sr, s) {
                self.drag = Drag::TextResize { handle: h, orig: tb.rect, start: c };
                return;
            }
            if sr.expand(4.0).contains(s) && !sr.shrink(3.0).contains(s) {
                self.drag = Drag::TextMove { start: c, orig: tb.rect };
                return;
            }
            if sr.contains(s) {
                let lay = self.text_layout(tb);
                let idx = lay.hit(c.x - tb.rect.x as f32 - TEXT_PAD, c.y - tb.rect.y as f32 - TEXT_PAD);
                if let Some(tb) = self.text_box.as_mut() {
                    tb.caret = idx;
                }
                self.caret_epoch = self.now;
                self.drag = Drag::TextCaret;
                return;
            }
            self.commit_text();
            return;
        }
        if btn == Btn::Left {
            self.commit_all();
            self.drag = Drag::TextNew { start: c };
        }
    }

    fn press_shape(&mut self, btn: Btn, cp: Pos2, c: Pos2, s: Pos2, view: View) {
        let swap = btn == Btn::Right;
        match self.shape {
            ShapeKind::Curve => {
                if let Some(cv) = self.curve.as_mut() {
                    if cv.stage == 1 {
                        cv.c1 = cp;
                        cv.c2 = cp;
                    } else {
                        cv.c2 = cp;
                    }
                } else {
                    self.commit_all();
                    self.curve = Some(CurveState { p0: cp, c1: cp, c2: cp, p3: cp, stage: 0, swap });
                }
                self.drag = Drag::Curve;
            }
            ShapeKind::Polygon => {
                if let Some(pg) = self.poly.as_mut() {
                    pg.pts.push(cp);
                } else {
                    self.commit_all();
                    self.poly = Some(PolyState { pts: vec![cp, cp], swap });
                }
                self.drag = Drag::Poly;
            }
            kind => {
                if let Some(sh) = self.active_shape {
                    if btn == Btn::Left {
                        if let Some(h) = self.shape_handle_hit(&sh, s, view) {
                            self.drag = Drag::ShapeHandle { handle: h };
                            return;
                        }
                        let tol = 4.0 / self.zoom;
                        if sh.bounds().expand(tol).contains(c) {
                            self.drag = Drag::ShapeMove { start: cp, a: sh.a, b: sh.b };
                            return;
                        }
                    }
                    self.commit_shape();
                }
                self.commit_all();
                self.active_shape = Some(ActiveShape { kind, a: cp, b: cp, swap });
                self.drag = Drag::ShapeDraw;
            }
        }
    }

    fn on_drag(&mut self, c: Pos2, mods: Modifiers, view: View) {
        let px = (c.x.floor() as i32, c.y.floor() as i32);
        let cp = pos2(px.0 as f32, px.1 as f32);
        match self.drag.clone() {
            Drag::Stroke => {
                let last = self.last_pt;
                match self.tool {
                    Tool::Pencil => {
                        let size = self.tool_size(0) as i32;
                        let col = self.btn_color(self.drag_btn);
                        if let Some(l) = self.layer.as_mut() {
                            brush::pencil_line(l, (last.x as i32, last.y as i32), px, size, col);
                        }
                        self.last_pt = cp;
                    }
                    Tool::Eraser => {
                        let only = (self.drag_btn == Btn::Right).then_some(self.color1);
                        self.eraser_segment((last.x as i32, last.y as i32), px, only);
                        self.last_pt = cp;
                    }
                    Tool::Brush => {
                        if let (Some(l), Some(st)) = (self.layer.as_mut(), self.stroke.as_mut()) {
                            st.segment(l, (last.x, last.y), (c.x, c.y));
                        }
                        self.last_pt = c;
                    }
                    _ => {}
                }
                self.display_dirty = true;
            }
            Drag::Marquee { start } => {
                let p = self.clamp_px(c);
                self.marquee = Some(IRect::from_points(start, p));
            }
            Drag::Lasso => {
                let p = pos2(c.x.clamp(0.0, self.img_w() as f32), c.y.clamp(0.0, self.img_h() as f32));
                if self.lasso.last().is_none_or(|l| (*l - p).length() * self.zoom >= 1.0) {
                    self.lasso.push(p);
                }
            }
            Drag::SelMove { start, orig } => {
                let d = c - start;
                if let Some(sel) = self.selection.as_mut() {
                    sel.rect.x = orig.x + d.x.round() as i32;
                    sel.rect.y = orig.y + d.y.round() as i32;
                }
                self.display_dirty = true;
            }
            Drag::SelResize { handle, orig, start } => {
                let d = c - start;
                if let Some(sel) = self.selection.as_mut() {
                    sel.rect = apply_handle(orig, handle, (d.x.round() as i32, d.y.round() as i32));
                }
                self.display_dirty = true;
            }
            Drag::ShapeDraw => {
                if let Some(sh) = self.active_shape.as_mut() {
                    sh.b = if mods.shift { constrain(sh.a, cp, sh.kind == ShapeKind::Line) } else { cp };
                }
                self.display_dirty = true;
            }
            Drag::ShapeMove { start, a, b } => {
                let d = cp - start;
                if let Some(sh) = self.active_shape.as_mut() {
                    sh.a = a + d;
                    sh.b = b + d;
                }
                self.display_dirty = true;
            }
            Drag::ShapeHandle { handle } => {
                if let Some(sh) = self.active_shape.as_mut() {
                    match handle {
                        Handle::P0 => sh.a = cp,
                        Handle::P1 => sh.b = cp,
                        h => {
                            let r = sh.bounds();
                            let (mut x0, mut y0, mut x1, mut y1) = (r.min.x, r.min.y, r.max.x, r.max.y);
                            match h {
                                Handle::N => y0 = cp.y,
                                Handle::S => y1 = cp.y,
                                Handle::E => x1 = cp.x,
                                Handle::W => x0 = cp.x,
                                Handle::NE => {
                                    y0 = cp.y;
                                    x1 = cp.x
                                }
                                Handle::NW => {
                                    y0 = cp.y;
                                    x0 = cp.x
                                }
                                Handle::SE => {
                                    y1 = cp.y;
                                    x1 = cp.x
                                }
                                Handle::SW => {
                                    y1 = cp.y;
                                    x0 = cp.x
                                }
                                _ => {}
                            }
                            sh.a = pos2(x0, y0);
                            sh.b = pos2(x1, y1);
                        }
                    }
                }
                self.display_dirty = true;
            }
            Drag::Curve => {
                if let Some(cv) = self.curve.as_mut() {
                    match cv.stage {
                        0 => cv.p3 = if mods.shift { constrain(cv.p0, cp, true) } else { cp },
                        1 => {
                            cv.c1 = cp;
                            cv.c2 = cp;
                        }
                        _ => cv.c2 = cp,
                    }
                }
                self.display_dirty = true;
            }
            Drag::Poly => {
                if let Some(pg) = self.poly.as_mut() {
                    let n = pg.pts.len();
                    let p = if mods.shift && n >= 2 { constrain(pg.pts[n - 2], cp, true) } else { cp };
                    if let Some(last) = pg.pts.last_mut() {
                        *last = p;
                    }
                }
                self.display_dirty = true;
            }
            Drag::TextMove { start, orig } => {
                let d = c - start;
                if let Some(tb) = self.text_box.as_mut() {
                    tb.rect.x = orig.x + d.x.round() as i32;
                    tb.rect.y = orig.y + d.y.round() as i32;
                }
                self.display_dirty = true;
            }
            Drag::TextResize { handle, orig, start } => {
                let d = c - start;
                if let Some(tb) = self.text_box.as_mut() {
                    let r = apply_handle(orig, handle, (d.x.round() as i32, d.y.round() as i32));
                    tb.rect = IRect::new(r.x, r.y, r.w.max(20), r.h.max(12));
                }
                self.fit_text_box();
            }
            Drag::CanvasResize { handle } => {
                let (w, h) = (self.img_w() as i32, self.img_h() as i32);
                let (cx, cy) = (c.x.round() as i32, c.y.round() as i32);
                let (mut x0, mut y0, mut x1, mut y1) = (0, 0, w, h);
                match handle {
                    Handle::E => x1 = cx,
                    Handle::W => x0 = cx,
                    Handle::S => y1 = cy,
                    Handle::N => y0 = cy,
                    Handle::SE => {
                        x1 = cx;
                        y1 = cy
                    }
                    Handle::SW => {
                        x0 = cx;
                        y1 = cy
                    }
                    Handle::NE => {
                        x1 = cx;
                        y0 = cy
                    }
                    Handle::NW => {
                        x0 = cx;
                        y0 = cy
                    }
                    _ => {}
                }
                // keep at least 1px and never flip
                let x0 = x0.min(x1 - 1);
                let y0 = y0.min(y1 - 1);
                let x1 = x1.max(x0 + 1);
                let y1 = y1.max(y0 + 1);
                self.canvas_resize = Some(IRect::new(x0, y0, x1 - x0, y1 - y0));
            }
            Drag::TextNew { .. } | Drag::TextCaret | Drag::Picker | Drag::None => {}
        }
        let _ = view;
    }

    fn on_release(&mut self, c: Pos2, double: bool, view: View) {
        let drag = std::mem::take(&mut self.drag);
        let px = (c.x.floor() as i32, c.y.floor() as i32);
        match drag {
            Drag::Stroke => self.commit_stroke(),
            Drag::Marquee { .. } => {
                if let Some(m) = self.marquee.take() {
                    if !m.is_empty() {
                        self.selection = Some(Selection::rect(m));
                    }
                }
            }
            Drag::Lasso => {
                let pts = std::mem::take(&mut self.lasso);
                if pts.len() >= 3 {
                    let minx = pts.iter().map(|p| p.x).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
                    let miny = pts.iter().map(|p| p.y).fold(f32::MAX, f32::min).floor().max(0.0) as i32;
                    let maxx = (pts.iter().map(|p| p.x).fold(f32::MIN, f32::max).ceil() as i32).min(self.img_w() as i32);
                    let maxy = (pts.iter().map(|p| p.y).fold(f32::MIN, f32::max).ceil() as i32).min(self.img_h() as i32);
                    let r = IRect::new(minx, miny, maxx - minx, maxy - miny);
                    if !r.is_empty() {
                        let local: Vec<Pos2> = pts.iter().map(|p| pos2(p.x - minx as f32, p.y - miny as f32)).collect();
                        let lp: Vec<(f32, f32)> = local.iter().map(|p| (p.x, p.y)).collect();
                        let mask = raster::polygon_mask(r.w, r.h, &lp);
                        self.selection = Some(Selection { rect: r, src: r, mask: Some(mask), outline: Some(local), content: None });
                    }
                }
            }
            Drag::ShapeDraw => {
                if let Some(sh) = self.active_shape {
                    let d = sh.b - sh.a;
                    if d.x.abs() < 1.0 && d.y.abs() < 1.0 {
                        self.active_shape = None;
                    }
                }
            }
            Drag::Curve => {
                if let Some(cv) = self.curve.as_mut() {
                    match cv.stage {
                        0 => {
                            if cv.p0 == cv.p3 {
                                self.curve = None;
                            } else {
                                cv.stage = 1;
                                cv.c1 = cv.p0;
                                cv.c2 = cv.p3;
                            }
                        }
                        1 => cv.stage = 2,
                        _ => self.commit_shape(),
                    }
                }
            }
            Drag::Poly => {
                if let Some(pg) = self.poly.as_mut() {
                    let n = pg.pts.len();
                    let first = pg.pts[0];
                    let last = pg.pts[n - 1];
                    let near_start = n > 3 && (last - first).length() * self.zoom <= 6.0;
                    if n == 2 && pg.pts[0] == pg.pts[1] {
                        self.poly = None;
                    } else if double || near_start {
                        if near_start {
                            pg.pts.pop();
                        }
                        pg.pts.dedup_by(|a, b| (*a - *b).length() < 1.0);
                        self.commit_shape();
                    }
                }
            }
            Drag::TextNew { start } => {
                let s = (start.x.floor() as i32, start.y.floor() as i32);
                let r = IRect::from_points(s, px);
                let st = self.text_style();
                let min_h = (st.size_px * 1.3 + 2.0 * TEXT_PAD).ceil() as i32;
                let r = if r.w < 6 && r.h < 6 {
                    IRect::new(s.0, s.1, 180, min_h)
                } else {
                    IRect::new(r.x, r.y, r.w.max(30), r.h.max(min_h))
                };
                let max_w = self.img_w() as i32 - r.x;
                let r = IRect::new(r.x, r.y, r.w.min(max_w.max(30)), r.h);
                self.text_box = Some(TextBox { rect: r, text: String::new(), caret: 0 });
                self.caret_epoch = self.now;
                self.tab = Tab::Text;
            }
            Drag::CanvasResize { .. } => {
                if let Some(r) = self.canvas_resize.take() {
                    self.set_canvas_rect(r);
                }
            }
            Drag::Picker => {
                self.pick_color(px, if self.drag_btn == Btn::Left { 1 } else { 2 });
                let prev = self.prev_tool;
                self.tool = if prev == Tool::Picker { Tool::Pencil } else { prev };
            }
            _ => {}
        }
        let _ = view;
        self.display_dirty = true;
    }

    // ----- painting ---------------------------------------------------------

    fn paint_canvas(&mut self, ui: &mut Ui, view: View, resp: &Response) {
        let ppp = ui.ctx().pixels_per_point();
        let p = ui.painter().clone();
        let img_rect = view.rect(self.image_irect());

        // drop shadow under the picture
        if theme::modern() {
            p.add(egui::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(30) }.as_shape(img_rect, 0.0));
        } else {
            p.rect_filled(img_rect.translate(vec2(1.0, 1.0)).expand(0.5), 0.0, Color32::from_black_alpha(18));
        }
        if let Some(t) = &self.texture {
            p.image(t.id(), img_rect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
        }

        // gridlines
        if self.show_grid && self.zoom >= 2.0 {
            let clip = self.viewport.intersect(img_rect);
            let gp = p.with_clip_rect(clip);
            let col = Color32::from_rgba_unmultiplied(128, 128, 128, 110);
            let c0 = view.to_canvas(clip.min);
            let c1 = view.to_canvas(clip.max);
            for x in (c0.x.floor() as i32).max(1)..=(c1.x.ceil() as i32) {
                let sx = crisp(view.origin.x + x as f32 * self.zoom, ppp);
                gp.line_segment([pos2(sx, clip.top()), pos2(sx, clip.bottom())], Stroke::new(1.0 / ppp, col));
            }
            for y in (c0.y.floor() as i32).max(1)..=(c1.y.ceil() as i32) {
                let sy = crisp(view.origin.y + y as f32 * self.zoom, ppp);
                gp.line_segment([pos2(clip.left(), sy), pos2(clip.right(), sy)], Stroke::new(1.0 / ppp, col));
            }
        }

        // canvas resize handles
        if !matches!(self.drag, Drag::CanvasResize { .. }) {
            for (_, c) in self.canvas_handles(view) {
                draw_handle(&p, c, ppp, theme::HANDLE_BORDER);
            }
        }
        if let Some(r) = self.canvas_resize {
            ants_rect(&p, view.rect(r), ppp);
            for (_, c) in handle_points(view.rect(r)) {
                if theme::modern() {
                    draw_handle(&p, c, ppp, theme::HANDLE_BORDER);
                }
            }
        }

        // selection overlays
        if let Some(m) = self.marquee {
            ants_rect(&p, view.rect(m), ppp);
        }
        if self.lasso.len() > 1 {
            let pts: Vec<Pos2> = self.lasso.iter().map(|c| view.to_screen(*c)).collect();
            ants(&p, &pts, false);
        }
        if let Some(sel) = &self.selection {
            let sr = view.rect(sel.rect);
            if let (Some(outline), None) = (&sel.outline, &sel.content) {
                let sx = sel.rect.w as f32 / sel.src.w.max(1) as f32;
                let sy = sel.rect.h as f32 / sel.src.h.max(1) as f32;
                let pts: Vec<Pos2> = outline.iter().map(|o| view.to_screen(pos2(sel.rect.x as f32 + o.x * sx, sel.rect.y as f32 + o.y * sy))).collect();
                ants(&p, &pts, true);
            }
            ants_rect(&p, sr, ppp);
            draw_handles(&p, sr, ppp);
        }

        // active shape bounding box
        if let Some(sh) = &self.active_shape {
            if !matches!(self.drag, Drag::ShapeDraw) || sh.kind != ShapeKind::Line {
                if sh.kind == ShapeKind::Line {
                    if !matches!(self.drag, Drag::ShapeDraw) {
                        draw_handle(&p, view.to_screen(sh.a + vec2(0.5, 0.5)), ppp, theme::SELECTION_HANDLE);
                        draw_handle(&p, view.to_screen(sh.b + vec2(0.5, 0.5)), ppp, theme::SELECTION_HANDLE);
                    }
                } else {
                    let r = Rect::from_min_max(view.to_screen(sh.bounds().min), view.to_screen(sh.bounds().max + vec2(1.0, 1.0)));
                    ants_rect(&p, r, ppp);
                    if !matches!(self.drag, Drag::ShapeDraw) {
                        draw_handles(&p, r, ppp);
                    }
                }
            }
        }

        // text box
        if let Some(tb) = &self.text_box {
            let sr = view.rect(tb.rect);
            ants_rect(&p, sr, ppp);
            draw_handles(&p, sr, ppp);
            let blink = ((self.now - self.caret_epoch) * 1.9).floor() as i64 % 2 == 0;
            if blink {
                let lay = self.text_layout(tb);
                let (li, x) = lay.caret_pos(tb.caret);
                let cx = tb.rect.x as f32 + TEXT_PAD + x;
                let cy = tb.rect.y as f32 + TEXT_PAD + lay.lines[li].y;
                let a = view.to_screen(pos2(cx, cy));
                let b = view.to_screen(pos2(cx, cy + lay.line_h));
                let x = crisp(a.x, ppp);
                p.line_segment([pos2(x, a.y), pos2(x, b.y)], Stroke::new(1.0_f32, Color32::BLACK));
            }
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(250));
        }
        if let Drag::TextNew { start } = self.drag {
            ants_rect(&p, Rect::from_two_pos(view.to_screen(start), view.to_screen(self.drag_cur)), ppp);
        }

        self.paint_cursor(ui, view, resp);
    }

    fn paint_cursor(&mut self, ui: &mut Ui, view: View, resp: &Response) {
        let Some(hp) = ui.input(|i| i.pointer.hover_pos()) else { return };
        let dragging = !matches!(self.drag, Drag::None);
        if !(resp.hovered() || dragging) || !self.viewport.contains(hp) && !dragging {
            return;
        }
        if self.input_blocked || (!dragging && self.overlay_rects_prev.iter().any(|r| r.contains(hp))) {
            return;
        }
        let c = view.to_canvas(hp);
        // handles & move areas
        let set = |icon: CursorIcon| ui.ctx().set_cursor_icon(icon);
        if !dragging {
            if self.selection.is_none() && self.text_box.is_none() {
                if let Some(h) = self.hit_canvas_handle(hp, view) {
                    set(handle_cursor(h));
                    return;
                }
            }
            if matches!(self.tool, Tool::Select | Tool::FreeSelect) {
                if let Some(sel) = &self.selection {
                    let sr = view.rect(sel.rect);
                    if let Some(h) = hit_handle(sr, hp) {
                        set(handle_cursor(h));
                        return;
                    }
                    if sel.rect.contains(c.x, c.y) {
                        set(CursorIcon::Move);
                        return;
                    }
                }
            }
            if self.tool == Tool::Shape {
                if let Some(sh) = self.active_shape {
                    if let Some(h) = self.shape_handle_hit(&sh, hp, view) {
                        set(handle_cursor(h));
                        return;
                    }
                    if sh.bounds().expand(4.0 / self.zoom).contains(c) {
                        set(CursorIcon::Move);
                        return;
                    }
                }
            }
            if let (Tool::Text, Some(tb)) = (self.tool, &self.text_box) {
                let sr = view.rect(tb.rect);
                if let Some(h) = hit_handle(sr, hp) {
                    set(handle_cursor(h));
                    return;
                }
                if sr.expand(4.0).contains(hp) && !sr.shrink(3.0).contains(hp) {
                    set(CursorIcon::Move);
                    return;
                }
            }
        } else {
            match self.drag {
                Drag::SelMove { .. } | Drag::ShapeMove { .. } | Drag::TextMove { .. } => {
                    set(CursorIcon::Move);
                    return;
                }
                Drag::SelResize { handle, .. } | Drag::TextResize { handle, .. } | Drag::CanvasResize { handle } | Drag::ShapeHandle { handle } => {
                    set(handle_cursor(handle));
                    return;
                }
                _ => {}
            }
        }
        let fg = ui.ctx().layer_painter(LayerId::new(Order::Tooltip, Id::new("paint-cursor")));
        match self.tool {
            Tool::Select | Tool::FreeSelect | Tool::Shape => set(CursorIcon::Crosshair),
            Tool::Text => set(CursorIcon::Text),
            Tool::Pencil => {
                set(CursorIcon::None);
                self.icons.paint(&fg, Icon::Pencil, Rect::from_min_size(pos2(hp.x - 1.0, hp.y - 15.0), vec2(16.0, 16.0)), true);
            }
            Tool::Fill => {
                set(CursorIcon::None);
                self.icons.paint(&fg, Icon::Fill, Rect::from_min_size(pos2(hp.x - 15.0, hp.y - 14.0), vec2(16.0, 16.0)), true);
            }
            Tool::Picker => {
                set(CursorIcon::None);
                self.icons.paint(&fg, Icon::Picker, Rect::from_min_size(pos2(hp.x - 1.0, hp.y - 15.0), vec2(16.0, 16.0)), true);
            }
            Tool::Magnifier => {
                set(CursorIcon::None);
                self.icons.paint(&fg, Icon::Magnifier, Rect::from_min_size(pos2(hp.x - 6.5, hp.y - 6.5), vec2(16.0, 16.0)), true);
            }
            Tool::Eraser => {
                set(CursorIcon::None);
                let s = self.tool_size(3) * self.zoom;
                let px = pos2(c.x.floor(), c.y.floor());
                let off = (self.tool_size(3) as i32 / 2) as f32;
                let tl = view.to_screen(pos2(px.x - off, px.y - off));
                let r = Rect::from_min_size(tl, vec2(s, s));
                let fill = if self.active_layer > 0 { Color32::from_white_alpha(70) } else { Color32::from_white_alpha(200) };
                fg.rect_filled(r, 0.0, fill);
                fg.rect_stroke(r, 0.0, Stroke::new(1.0_f32, Color32::BLACK), StrokeKind::Inside);
            }
            Tool::Brush => {
                let d = self.tool_size(1) * self.zoom;
                if d >= 5.0 {
                    set(CursorIcon::None);
                    fg.circle_stroke(hp, d / 2.0 + 0.5, Stroke::new(1.0_f32, Color32::WHITE));
                    fg.circle_stroke(hp, d / 2.0, Stroke::new(1.0_f32, Color32::BLACK));
                } else {
                    set(CursorIcon::Crosshair);
                }
            }
        }
    }

    fn draw_rulers(&self, ui: &Ui, full: Rect, view_rect: Rect) {
        let p = ui.painter();
        let modern = theme::modern();
        let bg = if modern { theme::w().ruler_bg } else { Color32::from_rgb(250, 250, 250) };
        let line = if modern { theme::w().border } else { Color32::from_rgb(160, 160, 160) };
        let hr = Rect::from_min_max(pos2(view_rect.left(), full.top()), pos2(full.right(), view_rect.top()));
        let vr = Rect::from_min_max(pos2(full.left(), view_rect.top()), pos2(view_rect.left(), full.bottom()));
        p.rect_filled(Rect::from_min_max(full.min, view_rect.min), 0.0, theme::workspace());
        p.rect_filled(hr, 0.0, bg);
        p.rect_filled(vr, 0.0, bg);
        p.line_segment([pos2(hr.left(), hr.bottom() - 0.5), pos2(hr.right(), hr.bottom() - 0.5)], Stroke::new(1.0_f32, line));
        p.line_segment([pos2(vr.right() - 0.5, vr.top()), pos2(vr.right() - 0.5, vr.bottom())], Stroke::new(1.0_f32, line));
        let z = self.zoom;
        let steps = [1.0f32, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0, 2000.0];
        let minor = steps.iter().copied().find(|s| s * z >= 5.0).unwrap_or(1000.0);
        let label = steps.iter().copied().find(|s| s * z >= 55.0).unwrap_or(2000.0);
        let font = egui::FontId::proportional(9.0);
        let tick_col = if modern { theme::w().ruler_tick } else { Color32::from_rgb(110, 110, 110) };
        let org_x = self.view_origin.x;
        let org_y = self.view_origin.y;
        let hp = p.with_clip_rect(hr);
        let c0 = ((hr.left() - org_x) / z / minor).floor() as i64;
        let c1 = ((hr.right() - org_x) / z / minor).ceil() as i64;
        for k in c0..=c1 {
            let v = k as f32 * minor;
            let x = (org_x + v * z).round() + 0.5;
            let is_label = (v / label).fract().abs() < 1e-4;
            let len = if is_label { 8.0 } else if (v / (label / 2.0)).fract().abs() < 1e-4 { 5.0 } else { 3.0 };
            hp.line_segment([pos2(x, hr.bottom() - len), pos2(x, hr.bottom())], Stroke::new(1.0_f32, tick_col));
            if is_label {
                hp.text(pos2(x + 2.0, hr.top() + 1.0), egui::Align2::LEFT_TOP, format!("{}", v as i64), font.clone(), tick_col);
            }
        }
        let vp = p.with_clip_rect(vr);
        let c0 = ((vr.top() - org_y) / z / minor).floor() as i64;
        let c1 = ((vr.bottom() - org_y) / z / minor).ceil() as i64;
        for k in c0..=c1 {
            let v = k as f32 * minor;
            let y = (org_y + v * z).round() + 0.5;
            let is_label = (v / label).fract().abs() < 1e-4;
            let len = if is_label { 8.0 } else if (v / (label / 2.0)).fract().abs() < 1e-4 { 5.0 } else { 3.0 };
            vp.line_segment([pos2(vr.right() - len, y), pos2(vr.right(), y)], Stroke::new(1.0_f32, tick_col));
            if is_label {
                let g = vp.layout_no_wrap(format!("{}", v as i64), font.clone(), tick_col);
                let mut shape = egui::epaint::TextShape::new(pos2(vr.left() + 1.0, y + 2.0 + g.size().x), g, tick_col);
                shape.angle = -std::f32::consts::FRAC_PI_2;
                vp.add(shape);
            }
        }
        if let Some((cx, cy)) = self.cursor_px {
            let x = (org_x + (cx as f32 + 0.5) * z).round() + 0.5;
            let y = (org_y + (cy as f32 + 0.5) * z).round() + 0.5;
            hp.line_segment([pos2(x, hr.top()), pos2(x, hr.bottom())], Stroke::new(1.0_f32, Color32::from_rgb(30, 110, 200)));
            vp.line_segment([pos2(vr.left(), y), pos2(vr.right(), y)], Stroke::new(1.0_f32, Color32::from_rgb(30, 110, 200)));
        }
        let _: Vec2 = Vec2::ZERO;
    }
}
