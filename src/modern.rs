//! The Windows 11 style layout: menu row, left-aligned single-row toolbar,
//! floating size/opacity sliders, floating text formatting bar, flyout menus
//! and status bar. Colors come from `theme::w()` so it follows light/dark mode.

use std::path::PathBuf;

use egui::{pos2, vec2, Align2, Color32, Context, CornerRadius, Id, Order, Pos2, Rect, Sense, StrokeKind, Ui};

use crate::app::{PaintApp, PendingAction, Tool, PALETTE, PALETTE_NAMES, ZOOM_LEVELS};
use crate::dialogs::Dialog;
use crate::icons::Icon;
use crate::ribbon::{draw_shape_icon, tip, PopupKind};
use crate::shapes::ShapeKind;
use crate::theme::{self, sk, ThemeMode};
use crate::widgets::{self, icon_button, icon_button_px, menu_row, menu_separator, paint_button_bg, w11_button, w11_group, w11_icon_chevron};

pub const HEADER_H: f32 = 40.0;
pub const TOOLBAR_H: f32 = 92.0;
pub const STATUS_H: f32 = 30.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum W11Menu {
    File,
    Edit,
    View,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum W11Sub {
    Recent,
    SaveAs,
}

enum MenuAction {
    New,
    Open,
    OpenRecent(PathBuf),
    Save,
    SaveAs(Option<&'static str>),
    Print,
    Properties,
    About,
    Exit,
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    PasteFrom,
    SelectAll,
    InvertSelection,
    Delete,
    Crop,
    InvertColor,
    ZoomIn,
    ZoomOut,
    Zoom100,
    ZoomFit,
    Rulers,
    Grid,
    Status,
    Thumbnail,
    FullScreen,
    Theme(ThemeMode),
}

/// Platform-appropriate shortcut label.
fn sc(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("⌘{key}")
    } else {
        format!("Ctrl+{key}")
    }
}

fn sc_shift(key: &str) -> String {
    if cfg!(target_os = "macos") {
        format!("Shift+⌘{key}")
    } else {
        format!("Ctrl+Shift+{key}")
    }
}

enum SliderInput {
    None,
    Set(f32),
    Steps(i32),
}

// Group content widths.
const SELECT_W: f32 = 44.0;
const IMAGE_W: f32 = 84.0;
const TOOLS_W: f32 = 104.0;
const BRUSH_W: f32 = 48.0;
const SHAPES_W: f32 = 226.0;
const COLORS_W: f32 = 302.0;
const GAP: f32 = 24.0;

impl PaintApp {
    // ----- header: File / Edit / View + quick access -------------------------

    pub fn w11_header_ui(&mut self, ui: &mut Ui) {
        let pal = theme::w();
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, pal.header_bg);
        let cy = rect.center().y;
        let mut x = rect.left() + 6.0;
        for (menu, label) in [(W11Menu::File, "File"), (W11Menu::Edit, "Edit"), (W11Menu::View, "View")] {
            let g = p.layout_no_wrap(label.to_string(), widgets::font(), pal.text);
            let r = widgets::snap(Rect::from_min_size(pos2(x, cy - 15.0), vec2(g.size().x + 22.0, 30.0)));
            let open = self.w11_menu.map(|m| m.0) == Some(menu);
            self.menu_opener(ui, r, menu, open);
            p.galley(pos2(r.center().x - g.size().x / 2.0, r.center().y - g.size().y / 2.0), g, pal.text);
            x = r.right() + 2.0;
        }
        let dx = (x + 6.0).round() + 0.5;
        p.line_segment([pos2(dx, cy - 9.0), pos2(dx, cy + 9.0)], sk(1.0, pal.divider));
        x = dx + 6.0;
        let can_undo = self.history.can_undo() || self.active_shape.is_some() || self.text_box.is_some();
        let can_redo = self.history.can_redo();
        let qat = [
            (Icon::Save, "Save", sc("S"), true),
            (Icon::Open, "Open", sc("O"), true),
            (Icon::Undo, "Undo", sc("Z"), can_undo),
            (Icon::Redo, "Redo", sc("Y"), can_redo),
        ];
        for (icon, name, key, en) in qat {
            let r = Rect::from_min_size(pos2(x, cy - 15.0), vec2(32.0, 30.0));
            let resp = icon_button(ui, &mut self.icons, r, Id::new(("w11qat", name)), icon, false, en);
            if tip(resp, &format!("{name} ({key})"), "").clicked() {
                match icon {
                    Icon::Save => {
                        self.save();
                    }
                    Icon::Open => self.request(ui.ctx(), PendingAction::Open),
                    Icon::Undo => self.undo(),
                    _ => self.redo(),
                }
            }
            x += 34.0;
        }
        // right side: style switch + settings
        let gear = widgets::snap(Rect::from_min_size(pos2(rect.right() - 40.0, cy - 15.0), vec2(32.0, 30.0)));
        let open = self.w11_menu.map(|m| m.0) == Some(W11Menu::Settings);
        let resp = icon_button(ui, &mut self.icons, gear, Id::new("w11-settings"), Icon::Settings, open, true);
        if tip(resp, "Settings", "App theme and more.").clicked() {
            self.w11_menu = if open { None } else { Some((W11Menu::Settings, gear)) };
            self.w11_sub = None;
            self.popup = None;
        }
        let w = self.style_toggle_width(ui);
        self.style_toggle_button(ui, Rect::from_min_size(pos2(gear.left() - 8.0 - w, cy - 14.0), vec2(w, 28.0)));
    }

    fn menu_opener(&mut self, ui: &Ui, r: Rect, menu: W11Menu, open: bool) {
        let resp = ui.interact(r, Id::new(("w11menu", format!("{menu:?}"))), Sense::click());
        paint_button_bg(ui.painter(), r, resp.hovered() || open, resp.is_pointer_button_down_on(), false);
        if resp.clicked() {
            self.w11_menu = if open { None } else { Some((menu, r)) };
            self.w11_sub = None;
            self.popup = None;
        } else if resp.hovered() && self.w11_menu.is_some_and(|m| m.0 != W11Menu::Settings) && !open {
            self.w11_menu = Some((menu, r));
            self.w11_sub = None;
        }
    }

    // ----- toolbar -------------------------------------------------------------

    pub fn w11_toolbar_ui(&mut self, ui: &mut Ui) {
        let pal = theme::w();
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, pal.toolbar_bg);
        p.line_segment([pos2(rect.left(), rect.top() + 0.5), pos2(rect.right(), rect.top() + 0.5)], sk(1.0, pal.edge));
        p.line_segment([pos2(rect.left(), rect.bottom() - 0.5), pos2(rect.right(), rect.bottom() - 0.5)], sk(1.0, pal.edge));
        let mut x = rect.left() + 10.0;
        let top = rect.top() + 7.0;
        let label_y = rect.bottom() - 12.0;
        let (dt, db) = (rect.top() + 8.0, rect.bottom() - 8.0);

        self.w11_selection(ui, x, top);
        w11_group(&p, x, x + SELECT_W, label_y, dt, db, "Selection", true);
        x += SELECT_W + GAP;
        self.w11_image(ui, x, top);
        w11_group(&p, x, x + IMAGE_W, label_y, dt, db, "Image", true);
        x += IMAGE_W + GAP;
        self.w11_tools(ui, x, top);
        w11_group(&p, x, x + TOOLS_W, label_y, dt, db, "Tools", true);
        x += TOOLS_W + GAP;
        self.w11_brushes(ui, x, top);
        w11_group(&p, x, x + BRUSH_W, label_y, dt, db, "Brushes", true);
        x += BRUSH_W + GAP;
        self.w11_shapes(ui, x, top);
        w11_group(&p, x, x + SHAPES_W, label_y, dt, db, "Shapes", true);
        x += SHAPES_W + GAP;
        self.w11_colors(ui, x, top);
        w11_group(&p, x, x + COLORS_W, label_y, dt, db, "Colors", true);
        x += COLORS_W + GAP;

        // Layers: its own group, label aligned with the others
        {
            let lr = Rect::from_min_size(pos2(x, top), vec2(44.0, 64.0));
            let sr = w11_button(ui, &mut self.icons, lr, Id::new("w11-layers-btn"), Icon::Layers, 26.0, false, false, self.show_layers, true);
            if tip(sr.top, "Layers", "Show or hide the Layers panel.").clicked() {
                self.show_layers = !self.show_layers;
            }
            w11_group(&p, x, x + 44.0, label_y, dt, db, "Layers", false);
        }

        self.popups_ui(ui.ctx());
    }

    fn w11_selection(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let r = Rect::from_min_size(pos2(x, top), vec2(44.0, 64.0));
        let selected = matches!(self.tool, Tool::Select | Tool::FreeSelect);
        let icon = if self.tool == Tool::FreeSelect { Icon::FreeSelect } else { Icon::Select };
        let sr = w11_button(ui, &mut self.icons, r, Id::new("w11-select"), icon, 24.0, true, true, selected, true);
        if tip(sr.top, "Select", "Select a part of the picture.").clicked() && !selected {
            self.set_tool(Tool::Select);
        }
        if sr.bottom.clicked() {
            self.toggle_popup(PopupKind::Select, r);
        }
    }

    fn w11_image(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let has_sel = self.selection.is_some();
        let crop = icon_button_px(ui, &mut self.icons, Rect::from_min_size(pos2(x, top), vec2(32.0, 32.0)), Id::new("w11-crop"), Icon::Crop, 20.0, false, has_sel);
        if tip(crop, &format!("Crop ({})", sc_shift("X")), "Crop the picture so it only contains the current selection.").clicked() {
            self.crop();
        }
        let resize = icon_button_px(ui, &mut self.icons, Rect::from_min_size(pos2(x, top + 32.0), vec2(32.0, 32.0)), Id::new("w11-resize"), Icon::Resize, 20.0, false, true);
        if tip(resize, &format!("Resize and skew ({})", sc("W")), "Resize and skew the picture or selection.").clicked() {
            self.dialog = Some(Dialog::resize(self));
        }
        let rr = Rect::from_min_size(pos2(x + 34.0, top), vec2(50.0, 32.0));
        if tip(w11_icon_chevron(ui, &mut self.icons, rr, Id::new("w11-rotate"), Icon::Rotate, false, true), "Rotate", "Rotate the picture or selection.").clicked() {
            self.toggle_popup(PopupKind::Rotate, rr);
        }
        let fr = Rect::from_min_size(pos2(x + 34.0, top + 32.0), vec2(50.0, 32.0));
        if tip(w11_icon_chevron(ui, &mut self.icons, fr, Id::new("w11-flip"), Icon::FlipHorizontal, false, true), "Flip", "Flip the picture or selection.").clicked() {
            self.toggle_popup(PopupKind::Flip, fr);
        }
    }

    fn w11_tools(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let tools = [
            (Tool::Pencil, Icon::Pencil, "Pencil", "Draw a free-form line with the selected width."),
            (Tool::Fill, Icon::Fill, "Fill", "Click an area to fill it with Color 1, right-click for Color 2."),
            (Tool::Text, Icon::Text, "Text", "Insert text into the picture."),
            (Tool::Eraser, Icon::Eraser, "Eraser", "Erase with Color 2. Right-click replaces only Color 1."),
            (Tool::Picker, Icon::Picker, "Color picker", "Pick a color from the picture."),
            (Tool::Magnifier, Icon::Magnifier, "Magnifier", "Left-click zooms in, right-click zooms out."),
        ];
        for (i, (tool, icon, name, desc)) in tools.iter().enumerate() {
            let r = Rect::from_min_size(pos2(x + (i % 3) as f32 * 36.0, top + (i / 3) as f32 * 32.0), vec2(32.0, 32.0));
            let resp = icon_button_px(ui, &mut self.icons, r, Id::new(("w11tool", *name)), *icon, 20.0, self.tool == *tool, true);
            if tip(resp, name, desc).clicked() {
                self.set_tool(*tool);
            }
        }
    }

    fn w11_brushes(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let r = widgets::snap(Rect::from_min_size(pos2(x, top), vec2(48.0, 64.0)));
        let pal = theme::w();
        // the brushes button sits in its own rounded "card", like the real app
        ui.painter().rect_filled(r, CornerRadius::same(6), pal.gallery_bg);
        ui.painter().rect_stroke(r, CornerRadius::same(6), sk(1.0, pal.border), StrokeKind::Inside);
        let sr = w11_button(ui, &mut self.icons, r.shrink(2.0), Id::new("w11-brushes"), Icon::Brushes, 24.0, true, true, self.tool == Tool::Brush, true);
        if tip(sr.top, "Brushes", &format!("Current: {}", self.brush.name())).clicked() {
            self.set_tool(Tool::Brush);
        }
        if sr.bottom.clicked() {
            self.toggle_popup(PopupKind::Brushes, r);
        }
    }

    fn w11_shapes(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let pal = theme::w();
        let (cols, cw, ch) = (8usize, 22.0, 20.0);
        let gr = widgets::snap(Rect::from_min_size(pos2(x, top + 1.0), vec2(cols as f32 * cw + 2.0, 3.0 * ch + 2.0)));
        let p = ui.painter().clone();
        p.rect_filled(gr, CornerRadius::same(6), pal.gallery_bg);
        p.rect_stroke(gr, CornerRadius::same(6), sk(1.0, pal.border), StrokeKind::Inside);
        for (i, kind) in ShapeKind::ALL.iter().enumerate() {
            let r = Rect::from_min_size(pos2(gr.left() + 1.0 + (i % cols) as f32 * cw, gr.top() + 1.0 + (i / cols) as f32 * ch), vec2(cw, ch));
            let resp = ui.interact(r, Id::new(("w11shape", i)), Sense::click());
            let selected = self.tool == Tool::Shape && self.shape == *kind;
            paint_button_bg(&p, widgets::snap(r.shrink(1.0)), resp.hovered(), resp.is_pointer_button_down_on(), selected);
            draw_shape_icon(&p, *kind, r.shrink(4.5));
            if tip(resp, kind.name(), "").clicked() {
                self.commit_all();
                self.shape = *kind;
                self.tool = Tool::Shape;
                self.display_dirty = true;
            }
        }
        let en = self.tool == Tool::Shape || self.active_shape.is_some();
        let orr = Rect::from_min_size(pos2(gr.right() + 4.0, top), vec2(44.0, 32.0));
        if tip(w11_icon_chevron(ui, &mut self.icons, orr, Id::new("w11-outline"), Icon::Outline, false, en), "Shape outline", "Outline uses Color 1.").clicked() {
            self.toggle_popup(PopupKind::Outline, orr);
        }
        let frr = Rect::from_min_size(pos2(gr.right() + 4.0, top + 32.0), vec2(44.0, 32.0));
        if tip(w11_icon_chevron(ui, &mut self.icons, frr, Id::new("w11-fill"), Icon::ShapeFill, false, en), "Shape fill", "Fill uses Color 2.").clicked() {
            self.toggle_popup(PopupKind::Fill, frr);
        }
    }

    fn w11_colors(&mut self, ui: &mut Ui, x: f32, top: f32) {
        let p = ui.painter().clone();
        for slot in [1u8, 2u8] {
            let (cy, r_dot) = if slot == 1 { (top + 17.0, 12.0) } else { (top + 50.0, 9.5) };
            let r = Rect::from_center_size(pos2(x + 15.0, cy), vec2(30.0, 30.0));
            let resp = ui.interact(r, Id::new(("w11color", slot)), Sense::click());
            let c = if slot == 1 { self.color1 } else { self.color2 };
            widgets::swatch_circle(&p, r.center(), r_dot, Some(c), resp.hovered(), self.active_color == slot);
            let (t, b) = if slot == 1 { ("Color 1", "Used by the pencil, brushes and shape outlines.") } else { ("Color 2", "Used by the eraser and shape fills.") };
            if tip(resp, t, b).clicked() {
                self.active_color = slot;
            }
        }
        let px0 = x + 38.0;
        for row in 0..3 {
            for col in 0..10 {
                let c = pos2(px0 + 11.0 + col as f32 * 22.0, top + 11.0 + row as f32 * 21.0);
                let r = Rect::from_center_size(c, vec2(20.0, 20.0));
                let color = if row < 2 { Some(PALETTE[row][col]) } else { self.custom_colors[col] };
                let resp = ui.interact(r, Id::new(("w11pal", row, col)), Sense::click());
                widgets::swatch_circle(&p, c, 8.0, color, resp.hovered() && color.is_some(), false);
                if let Some(cl) = color {
                    let name = if row < 2 { PALETTE_NAMES[row][col].to_string() } else { format!("Custom color ({}, {}, {})", cl.r(), cl.g(), cl.b()) };
                    let resp = if resp.hovered() { resp.on_hover_text(name) } else { resp };
                    if resp.clicked() {
                        let slot = self.active_color;
                        self.set_color(slot, cl);
                    }
                    if resp.secondary_clicked() {
                        self.set_color(2, cl);
                    }
                }
            }
        }
        let r = Rect::from_min_size(pos2(x + 262.0, top), vec2(40.0, 64.0));
        let sr = w11_button(ui, &mut self.icons, r, Id::new("w11-editcolors"), Icon::EditColors, 26.0, false, false, false, true);
        if tip(sr.top, "Edit colors", "Pick any color with the color editor.").clicked() {
            let c = if self.active_color == 1 { self.color1 } else { self.color2 };
            self.dialog = Some(Dialog::edit_colors(c, self.active_color, &self.dialog_custom));
        }
    }

    // ----- floating size / opacity sliders --------------------------------------

    /// Vertical size and opacity sliders floating on the left of the canvas area.
    pub fn w11_side_panel(&mut self, ui: &mut Ui, area: Rect) {
        let Some(slot) = self.size_slot() else { return };
        let pal = theme::w();
        let show_opacity = matches!(self.tool, Tool::Pencil | Tool::Brush | Tool::Shape);
        let sections = if show_opacity { 2.0 } else { 1.0 };
        let track_len = ((area.height() - 70.0 - 60.0 * sections) / sections).clamp(50.0, 190.0);
        let section_h = 32.0 + track_len + 18.0;
        let panel = widgets::snap(Rect::from_min_size(pos2(area.left() + 10.0, area.center().y - section_h * sections / 2.0), vec2(36.0, section_h * sections)));
        let p = ui.painter().clone();
        p.add(egui::Shadow { offset: [0, 2], blur: 10, spread: 0, color: Color32::from_black_alpha(if theme::dark() { 90 } else { 30 }) }.as_shape(panel, CornerRadius::same(8)));
        p.rect_filled(panel, CornerRadius::same(8), pal.card_bg);
        p.rect_stroke(panel, CornerRadius::same(8), sk(1.0, pal.border), StrokeKind::Inside);
        // claim the panel area so clicks and drags never reach the canvas below
        ui.interact(panel, Id::new("w11-side-panel-bg"), Sense::click_and_drag());
        self.overlay_rects.push(panel);

        let (min, max) = self.size_range(slot);
        let size = self.tool_size(slot);
        let t_size = ((size - min) / (max - min)).clamp(0.0, 1.0).sqrt();
        match self.vslider(ui, panel.top(), panel, track_len, Icon::Size, "size", t_size, &format!("Size: {}px", size.round() as i32)) {
            SliderInput::Set(t) => {
                self.custom_size[slot] = Some((min + (max - min) * t * t).round().clamp(min, max));
                self.display_dirty = true;
            }
            SliderInput::Steps(n) => {
                self.custom_size[slot] = Some((size.round() + n as f32).clamp(min, max));
                self.display_dirty = true;
            }
            SliderInput::None => {}
        }
        if show_opacity {
            let top = panel.top() + section_h;
            let y = top.round() + 0.5;
            p.line_segment([pos2(panel.left() + 6.0, y), pos2(panel.right() - 6.0, y)], sk(1.0, pal.divider));
            let label = format!("Opacity: {}%", (self.opacity * 100.0).round() as i32);
            match self.vslider(ui, top, panel, track_len, Icon::Opacity, "opacity", self.opacity, &label) {
                SliderInput::Set(t) => {
                    self.opacity = (t * 100.0).round().max(1.0) / 100.0;
                    self.display_dirty = true;
                }
                SliderInput::Steps(n) => {
                    self.opacity = ((self.opacity * 100.0).round() + n as f32).clamp(1.0, 100.0) / 100.0;
                    self.display_dirty = true;
                }
                SliderInput::None => {}
            }
        }
    }

    /// One vertical slider (top = 1.0). Dragging sets the value; the wheel steps it.
    #[allow(clippy::too_many_arguments)]
    fn vslider(&mut self, ui: &mut Ui, top: f32, panel: Rect, len: f32, icon: Icon, id: &str, t: f32, label: &str) -> SliderInput {
        let pal = theme::w();
        let p = ui.painter().clone();
        let cx = panel.center().x;
        self.icons.paint(&p, icon, Rect::from_center_size(pos2(cx, top + 16.0), vec2(16.0, 16.0)), true);
        let y0 = top + 34.0;
        let y1 = y0 + len;
        let hit = Rect::from_min_max(pos2(panel.left(), y0 - 8.0), pos2(panel.right(), y1 + 8.0));
        let resp = ui.interact(hit, Id::new(("w11-vslider", id)), Sense::click_and_drag());
        let ty = y1 - t * len;
        p.rect_filled(Rect::from_min_max(pos2(cx - 2.0, y0), pos2(cx + 2.0, y1)), CornerRadius::same(2), pal.track);
        p.rect_filled(Rect::from_min_max(pos2(cx - 2.0, ty), pos2(cx + 2.0, y1)), CornerRadius::same(2), pal.accent);
        p.circle_filled(pos2(cx, ty), 8.0, pal.card_bg);
        p.circle_stroke(pos2(cx, ty), 8.0, sk(1.0, pal.button_border));
        let inner = if resp.dragged() { 3.5 } else if resp.hovered() { 5.5 } else { 4.5 };
        p.circle_filled(pos2(cx, ty), inner, pal.accent);
        if resp.hovered() || resp.dragged() {
            // value bubble to the right of the thumb
            let g = p.layout_no_wrap(label.to_string(), widgets::font(), pal.text);
            let br = Rect::from_min_size(pos2(panel.right() + 8.0, ty - g.size().y / 2.0 - 5.0), g.size() + vec2(16.0, 10.0));
            let fg = ui.ctx().layer_painter(egui::LayerId::new(Order::Tooltip, Id::new(("w11-vslider-tip", id))));
            fg.rect_filled(br, CornerRadius::same(4), pal.menu_bg);
            fg.rect_stroke(br, CornerRadius::same(4), sk(1.0, pal.menu_border), StrokeKind::Inside);
            fg.galley(br.min + vec2(8.0, 5.0), g, pal.text);
        }
        if let Some(pp) = resp.interact_pointer_pos() {
            if resp.dragged() || resp.clicked() {
                // the last few pixels at each end snap to the limits
                let raw = (y1 - pp.y) / len;
                return SliderInput::Set(((raw - 0.03) / 0.94).clamp(0.0, 1.0));
            }
        }
        if resp.hovered() && self.slider_wheel.abs() >= 8.0 {
            let steps = (self.slider_wheel / 8.0).trunc();
            self.slider_wheel -= steps * 8.0;
            return SliderInput::Steps(steps as i32);
        }
        SliderInput::None
    }

    // ----- floating text formatting bar ---------------------------------------

    /// Floating text formatting bar, drawn over the top of the workspace.
    pub fn w11_textbar_ui(&mut self, ui: &mut Ui, area: Rect) {
        let pal = theme::w();
        let p = ui.painter().clone();
        let total = 170.0 + 6.0 + 64.0 + 20.0 + 4.0 * 34.0 + 20.0 + 2.0 * 34.0 + 24.0;
        let card = widgets::snap(Rect::from_min_size(pos2(area.center().x - total / 2.0, area.top() + 8.0), vec2(total, 38.0)));
        p.add(egui::Shadow { offset: [0, 2], blur: 8, spread: 0, color: Color32::from_black_alpha(if theme::dark() { 90 } else { 28 }) }.as_shape(card, CornerRadius::same(8)));
        p.rect_filled(card, CornerRadius::same(8), pal.card_bg);
        p.rect_stroke(card, CornerRadius::same(8), sk(1.0, pal.border), StrokeKind::Inside);
        ui.interact(card, Id::new("w11-textbar-bg"), Sense::click_and_drag());
        self.overlay_rects.push(card);
        let cy = card.center().y;
        let mut x = card.left() + 12.0;
        let face = Rect::from_min_size(pos2(x, cy - 11.0), vec2(170.0, 22.0));
        let size = Rect::from_min_size(pos2(face.right() + 6.0, cy - 11.0), vec2(64.0, 22.0));
        let mut changed = self.font_combos(ui, face, size);
        x = size.right() + 10.0;
        let d = x.round() + 0.5;
        p.line_segment([pos2(d, cy - 10.0), pos2(d, cy + 10.0)], sk(1.0, pal.divider));
        x += 10.0;
        changed |= self.font_style_buttons(ui, pos2(x, cy - 15.0), 30.0, 34.0);
        x += 4.0 * 34.0 + 6.0;
        let d = x.round() + 0.5;
        p.line_segment([pos2(d, cy - 10.0), pos2(d, cy + 10.0)], sk(1.0, pal.divider));
        x += 10.0;
        let o = icon_button_px(ui, &mut self.icons, Rect::from_min_size(pos2(x, cy - 15.0), vec2(30.0, 30.0)), Id::new("w11-opaque"), Icon::Opaque, 20.0, self.text.opaque, true);
        if tip(o, "Fill background", "Fill the text background with Color 2.").clicked() {
            self.text.opaque = true;
            self.display_dirty = true;
        }
        let t = icon_button_px(ui, &mut self.icons, Rect::from_min_size(pos2(x + 34.0, cy - 15.0), vec2(30.0, 30.0)), Id::new("w11-transparent"), Icon::Transparent, 20.0, !self.text.opaque, true);
        if tip(t, "Transparent background", "Let the picture show through behind the text.").clicked() {
            self.text.opaque = false;
            self.display_dirty = true;
        }
        if changed {
            self.fit_text_box();
        }
    }

    // ----- status bar ----------------------------------------------------------

    pub fn w11_status_ui(&mut self, ui: &mut Ui) {
        let pal = theme::w();
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, pal.status_bg);
        p.line_segment([pos2(rect.left(), rect.top() + 0.5), pos2(rect.right(), rect.top() + 0.5)], sk(1.0, pal.edge));
        let cy = rect.center().y + 0.5;
        let canvas = self.canvas_size_for_status();
        let segs: [(Icon, Option<String>, f32); 4] = [
            (Icon::StatusCursor, self.cursor_px.map(|(x, y)| format!("{x}, {y}px")), 140.0),
            (Icon::StatusSelection, self.selection_size_text(), 140.0),
            (Icon::StatusCanvas, Some(format!("{} × {}px", canvas.0, canvas.1)), 150.0),
            (Icon::StatusDisk, self.doc.file_size.map(|s| format!("Size: {}", crate::menus::fmt_size(s))), 150.0),
        ];
        let mut x = rect.left() + 12.0;
        for (icon, text, w) in segs {
            self.icons.paint(&p, icon, Rect::from_min_size(pos2(x, cy - 8.0), vec2(16.0, 16.0)), true);
            if let Some(t) = text {
                p.text(pos2(x + 24.0, cy), Align2::LEFT_CENTER, t, widgets::font(), pal.text);
            }
            x += w;
        }

        // zoom: [100% v] [-] ----o---- [+]
        let plus = Rect::from_center_size(pos2(rect.right() - 22.0, cy), vec2(26.0, 24.0));
        let track_r = plus.left() - 8.0;
        let track_l = track_r - 140.0;
        let minus = Rect::from_center_size(pos2(track_l - 17.0, cy), vec2(26.0, 24.0));
        for (r, is_plus) in [(minus, false), (plus, true)] {
            let resp = ui.interact(r, Id::new(("w11zoombtn", is_plus)), Sense::click());
            paint_button_bg(&p, r, resp.hovered(), resp.is_pointer_button_down_on(), false);
            let c = r.center();
            p.line_segment([pos2(c.x - 5.0, c.y), pos2(c.x + 5.0, c.y)], sk(1.2, pal.text));
            if is_plus {
                p.line_segment([pos2(c.x, c.y - 5.0), pos2(c.x, c.y + 5.0)], sk(1.2, pal.text));
            }
            let resp = tip(resp, if is_plus { "Zoom in" } else { "Zoom out" }, "");
            if resp.clicked() {
                self.zoom_step(if is_plus { 1 } else { -1 }, None);
            }
        }
        let track = Rect::from_min_max(pos2(track_l, cy - 10.0), pos2(track_r, cy + 10.0));
        let resp = ui.interact(track, Id::new("w11-zoom-slider"), Sense::click_and_drag());
        let t = ((self.zoom.log2() + 3.0) / 6.0).clamp(0.0, 1.0);
        let tx = track_l + t * (track_r - track_l);
        p.rect_filled(Rect::from_min_max(pos2(track_l, cy - 2.0), pos2(track_r, cy + 2.0)), CornerRadius::same(2), pal.track);
        p.rect_filled(Rect::from_min_max(pos2(track_l, cy - 2.0), pos2(tx, cy + 2.0)), CornerRadius::same(2), pal.accent);
        p.circle_filled(pos2(tx, cy), 8.0, pal.card_bg);
        p.circle_stroke(pos2(tx, cy), 8.0, sk(1.0, pal.button_border));
        let inner = if resp.dragged() { 3.5 } else if resp.hovered() { 5.5 } else { 4.5 };
        p.circle_filled(pos2(tx, cy), inner, pal.accent);
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
        let ztext = if self.zoom < 1.0 { format!("{}%", (self.zoom * 1000.0).round() / 10.0) } else { format!("{}%", (self.zoom * 100.0).round()) };
        let zr = widgets::snap(Rect::from_min_max(pos2(minus.left() - 72.0, cy - 12.0), pos2(minus.left() - 6.0, cy + 12.0)));
        let zresp = ui.interact(zr, Id::new("w11-zoom-menu"), Sense::click());
        paint_button_bg(&p, zr, zresp.hovered(), zresp.is_pointer_button_down_on(), self.popup.is_some_and(|pp| pp.kind == PopupKind::Zoom));
        p.text(pos2(zr.left() + 10.0, cy), Align2::LEFT_CENTER, ztext, widgets::font(), pal.text);
        widgets::chevron_down(&p, pos2(zr.right() - 12.0, cy + 0.5), pal.text);
        if zresp.clicked() {
            self.toggle_popup_above(PopupKind::Zoom, zr);
        }
    }

    // ----- flyout menus ----------------------------------------------------------

    pub fn w11_menus_ui(&mut self, ctx: &Context) {
        let Some((menu, opener)) = self.w11_menu else { return };
        let mut action: Option<MenuAction> = None;
        let (pivot, pos) = if menu == W11Menu::Settings { (Align2::RIGHT_TOP, pos2(opener.right(), opener.bottom() + 4.0)) } else { (Align2::LEFT_TOP, pos2(opener.left(), opener.bottom() + 4.0)) };
        let main = egui::Area::new(Id::new("w11-flyout")).order(Order::Foreground).pivot(pivot).fixed_pos(pos).constrain(true).show(ctx, |ui| {
            widgets::popup_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.set_width(260.0);
                match menu {
                    W11Menu::File => self.w11_file_items(ui, &mut action),
                    W11Menu::Edit => self.w11_edit_items(ui, &mut action),
                    W11Menu::View => self.w11_view_items(ui, &mut action),
                    W11Menu::Settings => self.w11_settings_items(ui, &mut action),
                }
            });
        });
        let mut areas = vec![main.response.rect, opener];
        if let (Some((sub, y)), W11Menu::File) = (self.w11_sub, menu) {
            let sa = egui::Area::new(Id::new("w11-subflyout")).order(Order::Foreground).fixed_pos(pos2(main.response.rect.right() - 2.0, y - 5.0)).constrain(true).show(ctx, |ui| {
                widgets::popup_frame().show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    match sub {
                        W11Sub::SaveAs => {
                            ui.set_width(220.0);
                            for (ext, label) in [("png", "PNG picture"), ("jpg", "JPEG picture"), ("bmp", "BMP picture"), ("gif", "GIF picture"), ("", "Other format")] {
                                if menu_row(ui, &mut self.icons, Some(Icon::Picture), label, "", false, true, false).clicked() {
                                    action = Some(MenuAction::SaveAs(if ext.is_empty() { None } else { Some(ext) }));
                                }
                            }
                        }
                        W11Sub::Recent => {
                            ui.set_width(300.0);
                            if self.recent.is_empty() {
                                menu_row(ui, &mut self.icons, None, "No recent pictures", "", false, false, false);
                            }
                            for path in self.recent.clone() {
                                let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                                let r = menu_row(ui, &mut self.icons, Some(Icon::Picture), &name, "", false, true, false);
                                let r = r.on_hover_text(path.display().to_string());
                                if r.clicked() {
                                    action = Some(MenuAction::OpenRecent(path.clone()));
                                }
                            }
                        }
                    }
                });
            });
            areas.push(sa.response.rect);
        }
        let pressed_outside = ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !areas.iter().any(|a| a.contains(p))));
        if pressed_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.w11_menu = None;
            self.w11_sub = None;
        }
        if let Some(a) = action {
            self.w11_menu = None;
            self.w11_sub = None;
            self.run_menu_action(ctx, a);
        }
    }

    fn sub_hover(&mut self, r: &egui::Response, sub: Option<W11Sub>) {
        if r.hovered() {
            self.w11_sub = sub.map(|s| (s, r.rect.top()));
        }
    }

    fn w11_file_items(&mut self, ui: &mut Ui, action: &mut Option<MenuAction>) {
        let r = menu_row(ui, &mut self.icons, Some(Icon::New), "New", &sc("N"), false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::New);
        }
        let r = menu_row(ui, &mut self.icons, Some(Icon::Open), "Open", &sc("O"), false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::Open);
        }
        let r = menu_row(ui, &mut self.icons, Some(Icon::Recent), "Recent", "", false, true, true);
        self.sub_hover(&r, Some(W11Sub::Recent));
        menu_separator(ui);
        let r = menu_row(ui, &mut self.icons, Some(Icon::Save), "Save", &sc("S"), false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::Save);
        }
        let r = menu_row(ui, &mut self.icons, Some(Icon::SaveAs), "Save as", "", false, true, true);
        self.sub_hover(&r, Some(W11Sub::SaveAs));
        if r.clicked() {
            *action = Some(MenuAction::SaveAs(None));
        }
        let r = menu_row(ui, &mut self.icons, Some(Icon::Print), "Print", &sc("P"), false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::Print);
        }
        menu_separator(ui);
        let r = menu_row(ui, &mut self.icons, Some(Icon::Properties), "Image properties", &sc("E"), false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::Properties);
        }
        let r = menu_row(ui, &mut self.icons, Some(Icon::About), "About Free Paint", "", false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::About);
        }
        menu_separator(ui);
        let r = menu_row(ui, &mut self.icons, Some(Icon::Exit), "Exit", "", false, true, false);
        self.sub_hover(&r, None);
        if r.clicked() {
            *action = Some(MenuAction::Exit);
        }
    }

    fn w11_edit_items(&mut self, ui: &mut Ui, action: &mut Option<MenuAction>) {
        let has_sel = self.selection.is_some();
        let can_undo = self.history.can_undo() || self.active_shape.is_some() || self.text_box.is_some();
        let rows: Vec<(Option<Icon>, &str, String, bool, Option<MenuAction>)> = vec![
            (Some(Icon::Undo), "Undo", sc("Z"), can_undo, Some(MenuAction::Undo)),
            (Some(Icon::Redo), "Redo", sc("Y"), self.history.can_redo(), Some(MenuAction::Redo)),
            (None, "-", String::new(), true, None),
            (Some(Icon::Cut), "Cut", sc("X"), has_sel, Some(MenuAction::Cut)),
            (Some(Icon::Copy), "Copy", sc("C"), has_sel, Some(MenuAction::Copy)),
            (Some(Icon::Paste), "Paste", sc("V"), true, Some(MenuAction::Paste)),
            (Some(Icon::Open), "Paste from file…", String::new(), true, Some(MenuAction::PasteFrom)),
            (None, "-", String::new(), true, None),
            (Some(Icon::SelectAll), "Select all", sc("A"), true, Some(MenuAction::SelectAll)),
            (Some(Icon::InvertSelection), "Invert selection", String::new(), true, Some(MenuAction::InvertSelection)),
            (Some(Icon::Delete), "Delete selection", "Del".into(), has_sel, Some(MenuAction::Delete)),
            (Some(Icon::Crop), "Crop to selection", sc_shift("X"), has_sel, Some(MenuAction::Crop)),
            (None, "-", String::new(), true, None),
            (Some(Icon::InvertColor), "Invert colors", sc_shift("I"), true, Some(MenuAction::InvertColor)),
        ];
        for (icon, label, key, en, act) in rows {
            if label == "-" {
                menu_separator(ui);
                continue;
            }
            if menu_row(ui, &mut self.icons, icon, label, &key, false, en, false).clicked() {
                *action = act;
            }
        }
    }

    fn w11_view_items(&mut self, ui: &mut Ui, action: &mut Option<MenuAction>) {
        if menu_row(ui, &mut self.icons, Some(Icon::ZoomIn), "Zoom in", &sc("+"), false, self.zoom < 8.0, false).clicked() {
            *action = Some(MenuAction::ZoomIn);
        }
        if menu_row(ui, &mut self.icons, Some(Icon::ZoomOut), "Zoom out", &sc("-"), false, self.zoom > 0.125, false).clicked() {
            *action = Some(MenuAction::ZoomOut);
        }
        if menu_row(ui, &mut self.icons, Some(Icon::Zoom100), "Actual size (100%)", "", false, true, false).clicked() {
            *action = Some(MenuAction::Zoom100);
        }
        if menu_row(ui, &mut self.icons, Some(Icon::Thumbnail), "Zoom to fit", "", false, true, false).clicked() {
            *action = Some(MenuAction::ZoomFit);
        }
        menu_separator(ui);
        if menu_row(ui, &mut self.icons, None, "Rulers", &sc("R"), self.show_rulers, true, false).clicked() {
            *action = Some(MenuAction::Rulers);
        }
        if menu_row(ui, &mut self.icons, None, "Gridlines", &sc("G"), self.show_grid, true, false).clicked() {
            *action = Some(MenuAction::Grid);
        }
        if menu_row(ui, &mut self.icons, None, "Status bar", "", self.show_status, true, false).clicked() {
            *action = Some(MenuAction::Status);
        }
        if menu_row(ui, &mut self.icons, None, "Thumbnail", "", self.show_thumbnail, true, false).clicked() {
            *action = Some(MenuAction::Thumbnail);
        }
        menu_separator(ui);
        if menu_row(ui, &mut self.icons, Some(Icon::FullScreen), "Full screen", "F11", false, true, false).clicked() {
            *action = Some(MenuAction::FullScreen);
        }
    }

    fn w11_settings_items(&mut self, ui: &mut Ui, action: &mut Option<MenuAction>) {
        let (r, _) = ui.allocate_exact_size(vec2(ui.available_width(), 28.0), Sense::hover());
        ui.painter().text(pos2(r.left() + 12.0, r.center().y + 1.0), Align2::LEFT_CENTER, "App theme", theme::bold(11.5), theme::w().subtle);
        for (mode, label) in [(ThemeMode::Light, "Light"), (ThemeMode::Dark, "Dark"), (ThemeMode::System, "Use system setting")] {
            if menu_row(ui, &mut self.icons, None, label, "", self.theme_mode == mode, true, false).clicked() {
                *action = Some(MenuAction::Theme(mode));
            }
        }
        menu_separator(ui);
        if menu_row(ui, &mut self.icons, Some(Icon::About), "About Free Paint", "", false, true, false).clicked() {
            *action = Some(MenuAction::About);
        }
    }

    // ----- layers panel ---------------------------------------------------

    pub fn w11_layers_ui(&mut self, ui: &mut Ui) {
        let pal = theme::w();
        let modern = theme::modern();
        // the same panel serves the classic style with classic colors
        let (c_bg, c_edge, c_border, c_accent, c_text, c_disabled) = if modern {
            (pal.toolbar_bg, pal.edge, pal.border, pal.accent, pal.text, pal.disabled)
        } else {
            (theme::RIBBON_BG, theme::RIBBON_BORDER, theme::RIBBON_BORDER, theme::PRESSED_BORDER, theme::TEXT, theme::TEXT_DISABLED)
        };
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, c_bg);
        p.line_segment([pos2(rect.left() + 0.5, rect.top()), pos2(rect.left() + 0.5, rect.bottom())], sk(1.0, c_edge));
        self.overlay_rects.push(rect);

        // centered add button on top, like the real app
        let add = Rect::from_center_size(pos2(rect.center().x, rect.top() + 24.0), vec2(32.0, 28.0));
        if tip(icon_button_px(ui, &mut self.icons, add, Id::new("ly-add"), Icon::AddLayer, 18.0, false, true), "Add layer", "Insert a new transparent layer above the selected one.").clicked() {
            self.add_layer();
        }

        // thumbnails (top layer first), rebuilt lazily
        let (iw, ih) = (self.img_w().max(1) as f32, self.img_h().max(1) as f32);
        let tw = 150u32;
        let th = ((tw as f32 * ih / iw).round() as u32).clamp(32, 110);
        if self.thumbs_stale || self.layer_thumbs.len() != self.layers.len() {
            self.thumbs_stale = false;
            let ctx = ui.ctx().clone();
            self.layer_thumbs = self
                .layers
                .iter()
                .enumerate()
                .map(|(i, l)| {
                    let mut t = crate::raster::checkerboard(tw, th);
                    let sc = crate::raster::scale(&l.pix, tw, th, true);
                    crate::raster::draw_over(&mut t, &sc, 0, 0);
                    ctx.load_texture(format!("layer-thumb-{i}"), crate::raster::to_color_image(&t), egui::TextureOptions::LINEAR)
                })
                .collect();
        }

        let row_h = th as f32 + 14.0;
        let list = Rect::from_min_max(pos2(rect.left() + 8.0, rect.top() + 46.0), pos2(rect.right() - 8.0, rect.bottom() - 44.0));
        let mut clicked: Option<usize> = None;
        let mut toggled: Option<usize> = None;
        let mut menu_at: Option<(usize, Pos2)> = None;
        egui::ScrollArea::vertical().id_salt("layers-scroll").max_height(list.height()).show(ui, |ui| {
            ui.allocate_space(vec2(0.0, 42.0));
            for i in (0..self.layers.len()).rev() {
                let (ar, resp) = ui.allocate_exact_size(vec2(list.width(), row_h), Sense::click());
                let active = i == self.active_layer;
                let visible = self.layers[i].visible;
                // thumbnail centered in the panel
                let trect = widgets::snap(Rect::from_center_size(pos2(rect.center().x, ar.center().y - 3.0), vec2(tw as f32, th as f32)));
                let pr = ui.painter();
                if let Some(t) = self.layer_thumbs.get(i) {
                    pr.image(t.id(), trect, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                }
                if !visible {
                    pr.rect_filled(trect, 0.0, Color32::from_black_alpha(100));
                }
                pr.rect_stroke(trect.expand(2.0), CornerRadius::same(4), sk(if active { 2.0 } else { 1.0 }, if active { c_accent } else { c_border }), StrokeKind::Inside);
                // eye toggle overlaid on the thumbnail's top-right corner
                let er = Rect::from_min_size(pos2(trect.right() - 24.0, trect.top() + 2.0), vec2(22.0, 22.0));
                let show_eye = !visible || resp.hovered() || er.contains(ui.ctx().pointer_hover_pos().unwrap_or(Pos2::ZERO));
                if show_eye {
                    let eresp = icon_button_px(ui, &mut self.icons, er, Id::new(("ly-eye", i)), if visible { Icon::Eye } else { Icon::EyeOff }, 15.0, false, true);
                    if tip(eresp, if visible { "Hide layer" } else { "Show layer" }, "").clicked() {
                        toggled = Some(i);
                    }
                }
                let name = if i == 0 { "Background".to_string() } else { format!("Layer {i}") };
                let resp = resp.on_hover_text(name);
                if resp.clicked() {
                    clicked = Some(i);
                }
                if resp.secondary_clicked() {
                    if let Some(pp) = ui.ctx().pointer_interact_pos() {
                        menu_at = Some((i, pp));
                    }
                }
            }
        });
        if let Some(i) = toggled {
            self.toggle_layer_visible(i);
        } else if let Some(i) = clicked {
            self.select_layer(i);
        } else if let Some((i, pp)) = menu_at {
            self.select_layer(i);
            self.layer_menu = Some(pp);
            self.layer_menu_frame = Some(self.frame_no);
        }

        // bottom action bar
        let by = rect.bottom() - 36.0;
        let acts: [(Icon, &str, &str, bool); 4] = [
            (Icon::Copy, "ly-dup", "Duplicate layer", true),
            (Icon::Merge, "ly-merge", "Merge down", self.active_layer > 0),
            (Icon::MergeUp, "ly-mergeup", "Merge up", self.active_layer + 1 < self.layers.len()),
            (Icon::Delete, "ly-del", "Delete layer", self.layers.len() > 1),
        ];
        let mut x = rect.left() + 8.0;
        for (icon, id, tipt, en) in acts {
            let r = Rect::from_min_size(pos2(x, by), vec2(28.0, 28.0));
            if tip(icon_button_px(ui, &mut self.icons, r, Id::new(id), icon, 17.0, false, en), tipt, "").clicked() {
                match id {
                    "ly-dup" => self.duplicate_layer(),
                    "ly-merge" => self.merge_layer_down(),
                    "ly-mergeup" => self.merge_layer_up(),
                    _ => self.delete_layer(),
                }
            }
            x += 32.0;
        }
        for (up, tipt) in [(true, "Move layer up"), (false, "Move layer down")] {
            let en = if up { self.active_layer + 1 < self.layers.len() } else { self.active_layer > 0 };
            let r = widgets::snap(Rect::from_min_size(pos2(x, by), vec2(28.0, 28.0)));
            let resp = ui.interact(r, Id::new(("ly-move", up)), if en { Sense::click() } else { Sense::hover() });
            paint_button_bg(&p, r, resp.hovered() && en, resp.is_pointer_button_down_on(), false);
            let col = if en { c_text } else { c_disabled };
            let c = r.center();
            if up {
                p.add(egui::Shape::line(vec![pos2(c.x - 4.5, c.y + 2.0), pos2(c.x, c.y - 2.8), pos2(c.x + 4.5, c.y + 2.0)], sk(1.4, col)));
            } else {
                p.add(egui::Shape::line(vec![pos2(c.x - 4.5, c.y - 2.0), pos2(c.x, c.y + 2.8), pos2(c.x + 4.5, c.y - 2.0)], sk(1.4, col)));
            }
            if tip(resp, tipt, "").clicked() && en {
                self.move_layer(up);
            }
            x += 32.0;
        }
    }

    /// Right-click menu for the selected layer, matching the real app.
    pub fn w11_layer_menu_ui(&mut self, ctx: &Context) {
        let Some(pos) = self.layer_menu else {
            self.layer_menu_frame = None;
            return;
        };
        let first_frame = self.layer_menu_frame == Some(self.frame_no);
        let visible = self.layers[self.active_layer].visible;
        let can_down = self.active_layer > 0;
        let can_up = self.active_layer + 1 < self.layers.len();
        let can_del = self.layers.len() > 1;
        let mut act: Option<&'static str> = None;
        let area = egui::Area::new(Id::new("layer-menu")).order(Order::Foreground).pivot(Align2::RIGHT_TOP).fixed_pos(pos).constrain(true).show(ctx, |ui| {
            widgets::popup_frame().show(ui, |ui| {
                ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                ui.set_width(230.0);
                let hide_label = if visible { "Hide layer" } else { "Show layer" };
                let hide_icon = if visible { Icon::EyeOff } else { Icon::Eye };
                let item = widgets::menu_item_sc;
                if item(ui, &mut self.icons, Some(hide_icon), hide_label, &sc_shift("H"), false, true).clicked() {
                    act = Some("hide");
                }
                if item(ui, &mut self.icons, Some(Icon::Copy), "Duplicate layer", "", false, true).clicked() {
                    act = Some("dup");
                }
                if item(ui, &mut self.icons, Some(Icon::Merge), "Merge down", "", false, can_down).clicked() {
                    act = Some("mergedown");
                }
                if item(ui, &mut self.icons, Some(Icon::MergeUp), "Merge up", "", false, can_up).clicked() {
                    act = Some("mergeup");
                }
                menu_separator(ui);
                if item(ui, &mut self.icons, None, "Move up", &sc_shift("PgUp"), false, can_up).clicked() {
                    act = Some("up");
                }
                if item(ui, &mut self.icons, None, "Move down", &sc_shift("PgDn"), false, can_down).clicked() {
                    act = Some("down");
                }
                menu_separator(ui);
                if item(ui, &mut self.icons, Some(Icon::Delete), "Delete layer", &sc_shift("Del"), false, can_del).clicked() {
                    act = Some("del");
                }
            });
        });
        let pressed_outside = !first_frame && ctx.input(|i| i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p)));
        if act.is_some() || pressed_outside || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.layer_menu = None;
        }
        match act {
            Some("hide") => {
                let i = self.active_layer;
                self.toggle_layer_visible(i);
            }
            Some("dup") => self.duplicate_layer(),
            Some("mergedown") => self.merge_layer_down(),
            Some("mergeup") => self.merge_layer_up(),
            Some("up") => self.move_layer(true),
            Some("down") => self.move_layer(false),
            Some("del") => self.delete_layer(),
            _ => {}
        }
    }

    fn run_menu_action(&mut self, ctx: &Context, a: MenuAction) {
        match a {
            MenuAction::New => self.request(ctx, PendingAction::New),
            MenuAction::Open => self.request(ctx, PendingAction::Open),
            MenuAction::OpenRecent(p) => self.request(ctx, PendingAction::OpenPath(p)),
            MenuAction::Save => {
                self.save();
            }
            MenuAction::SaveAs(ext) => {
                self.save_as(ext);
            }
            MenuAction::Print => self.dialog = Some(Dialog::print(self)),
            MenuAction::Properties => self.dialog = Some(Dialog::properties(self)),
            MenuAction::About => self.dialog = Some(Dialog::About),
            MenuAction::Exit => self.request(ctx, PendingAction::Exit),
            MenuAction::Undo => self.undo(),
            MenuAction::Redo => self.redo(),
            MenuAction::Cut => self.cut(),
            MenuAction::Copy => self.copy(),
            MenuAction::Paste => self.paste(ctx),
            MenuAction::PasteFrom => self.paste_from_file(),
            MenuAction::SelectAll => self.select_all(),
            MenuAction::InvertSelection => self.invert_selection(),
            MenuAction::Delete => self.delete_selection(),
            MenuAction::Crop => self.crop(),
            MenuAction::InvertColor => self.invert_colors(),
            MenuAction::ZoomIn => self.zoom_step(1, None),
            MenuAction::ZoomOut => self.zoom_step(-1, None),
            MenuAction::Zoom100 => self.set_zoom(1.0, None),
            MenuAction::ZoomFit => self.zoom_to_fit(),
            MenuAction::Rulers => self.show_rulers = !self.show_rulers,
            MenuAction::Grid => self.show_grid = !self.show_grid,
            MenuAction::Status => self.show_status = !self.show_status,
            MenuAction::Thumbnail => self.show_thumbnail = !self.show_thumbnail,
            MenuAction::FullScreen => self.toggle_fullscreen_view(ctx),
            MenuAction::Theme(mode) => self.set_theme_mode(mode),
        }
    }
}
