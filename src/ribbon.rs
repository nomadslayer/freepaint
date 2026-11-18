//! Tab row, ribbon groups and ribbon dropdown menus.

use egui::{pos2, vec2, Align2, Color32, Context, Id, Order, Pos2, Rect, Sense, Stroke, StrokeKind, Ui, UiBuilder};

use crate::app::{PaintApp, RotOp, Tab, Tool, PALETTE, PALETTE_NAMES, ZOOM_LEVELS};
use crate::brush::BrushKind;
use crate::dialogs::Dialog;
use crate::icons::Icon;
use crate::shapes::{self, ShapeKind};
use crate::text::FONT_SIZES;
use crate::theme;
use crate::widgets::{self, big_button, group, icon_button, menu_item, small_button};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PopupKind {
    Paste,
    Select,
    Rotate,
    Brushes,
    Outline,
    Fill,
    Size,
    Zoom,
    Flip,
}

#[derive(Clone, Copy, Debug)]
pub struct Popup {
    pub kind: PopupKind,
    pub pos: Pos2,
    pub opener: Rect,
    /// Open upwards (anchored at the opener's top edge).
    pub above: bool,
}

pub(crate) fn tip(r: egui::Response, title: &str, body: &str) -> egui::Response {
    if !r.hovered() {
        return r;
    }
    let title = title.to_string();
    let body = body.to_string();
    r.on_hover_ui(move |ui| {
        ui.set_max_width(260.0);
        ui.label(egui::RichText::new(title.as_str()).font(theme::bold(12.0)));
        if !body.is_empty() {
            ui.add_space(2.0);
            ui.label(body.as_str());
        }
    })
}

impl PaintApp {
    pub(crate) fn toggle_popup(&mut self, kind: PopupKind, opener: Rect) {
        if self.popup.map(|p| p.kind) == Some(kind) {
            self.popup = None;
        } else {
            let gap = if theme::modern() { 4.0 } else { 1.0 };
            self.popup = Some(Popup { kind, pos: pos2(opener.left(), opener.bottom() + gap), opener, above: false });
        }
    }

    pub(crate) fn toggle_popup_above(&mut self, kind: PopupKind, opener: Rect) {
        if self.popup.map(|p| p.kind) == Some(kind) {
            self.popup = None;
        } else {
            self.popup = Some(Popup { kind, pos: pos2(opener.left(), opener.top() - 4.0), opener, above: true });
        }
    }

    /// The single button that switches between the two ribbon looks.
    pub(crate) fn style_toggle_button(&mut self, ui: &mut Ui, rect: Rect) {
        let modern = theme::modern();
        let label = if modern { "Classic ribbon" } else { "Windows 11 style" };
        let rect = widgets::snap(rect);
        let resp = ui.interact(rect, Id::new("style-toggle"), Sense::click());
        let p = ui.painter();
        if modern {
            let cr = egui::CornerRadius::same(4);
            let pal = theme::w();
            p.rect_filled(rect, cr, if resp.hovered() { pal.button_hover } else { pal.button_bg });
            if resp.is_pointer_button_down_on() {
                p.rect_filled(rect, cr, pal.pressed);
            }
            p.rect_stroke(rect, cr, theme::sk(1.0, pal.button_border), StrokeKind::Inside);
        } else {
            widgets::paint_button_bg(p, rect, resp.hovered(), resp.is_pointer_button_down_on(), false);
        }
        self.icons.paint(p, Icon::RibbonStyle, Rect::from_min_size(pos2(rect.left() + 7.0, rect.center().y - 8.0), vec2(16.0, 16.0)), true);
        p.text(pos2(rect.left() + 29.0, rect.center().y), Align2::LEFT_CENTER, label, widgets::font(), theme::text_color());
        let body = if modern { "Switch back to the classic ribbon with Home and View tabs." } else { "Switch to the Windows 11 style toolbar with File, Edit and View menus." };
        if tip(resp, "Ribbon style", body).clicked() {
            self.toggle_style(ui.ctx());
        }
    }

    pub(crate) fn style_toggle_width(&self, ui: &Ui) -> f32 {
        let label = if theme::modern() { "Classic ribbon" } else { "Windows 11 style" };
        let w = ui.painter().layout_no_wrap(label.to_string(), widgets::font(), theme::TEXT).size().x;
        (w + 40.0).round()
    }

    // ----- tab row ---------------------------------------------------------

    pub fn tabs_ui(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::TAB_BAR);
        let base_y = rect.bottom() - 0.5;

        // File button
        let fr = widgets::snap(Rect::from_min_size(rect.min, vec2(56.0, rect.height())));
        let fresp = ui.interact(fr, Id::new("file-tab"), Sense::click());
        let fill = if fresp.hovered() || self.file_menu_open { theme::FILE_TAB_HOVER } else { theme::FILE_TAB };
        p.rect_filled(fr, 0.0, fill);
        p.text(fr.center(), Align2::CENTER_CENTER, "File", widgets::font(), Color32::WHITE);
        if fresp.clicked() {
            self.file_menu_open = !self.file_menu_open;
            self.file_menu_saveas = false;
            self.popup = None;
        }

        let mut tabs = vec![(Tab::Home, "Home"), (Tab::View, "View")];
        if self.text_box.is_some() {
            tabs.push((Tab::Text, "Text"));
        } else if self.tab == Tab::Text {
            self.tab = Tab::Home;
        }
        let mut x = fr.right() + 2.0;
        let mut sel_span = None;
        for (tab, label) in tabs {
            let galley = p.layout_no_wrap(label.to_string(), widgets::font(), theme::TEXT);
            let w = (galley.size().x + 26.0).round();
            let tr = widgets::snap(Rect::from_min_size(pos2(x, rect.top() + 1.0), vec2(w, rect.height() - 1.0)));
            let resp = ui.interact(tr, Id::new(("tab", label)), Sense::click());
            let selected = self.tab == tab;
            if selected {
                p.rect_filled(tr, 0.0, theme::RIBBON_BG);
                let s = Stroke::new(1.0_f32, theme::RIBBON_BORDER);
                p.line_segment([pos2(tr.left() + 0.5, tr.bottom()), pos2(tr.left() + 0.5, tr.top() + 0.5)], s);
                p.line_segment([pos2(tr.left() + 0.5, tr.top() + 0.5), pos2(tr.right() - 0.5, tr.top() + 0.5)], s);
                p.line_segment([pos2(tr.right() - 0.5, tr.top() + 0.5), pos2(tr.right() - 0.5, tr.bottom())], s);
                sel_span = Some((tr.left(), tr.right()));
            } else if resp.hovered() {
                p.rect_filled(tr.shrink2(vec2(0.0, 1.0)), 0.0, Color32::from_rgb(240, 244, 249));
            }
            if tab == Tab::Text {
                p.rect_filled(Rect::from_min_size(tr.min, vec2(tr.width(), 3.0)), 0.0, theme::TEXT_TOOLS_ACCENT);
            }
            let col = if tab == Tab::Text { Color32::from_rgb(150, 100, 0) } else { theme::TEXT };
            p.galley(pos2(tr.center().x - galley.size().x / 2.0, tr.center().y - galley.size().y / 2.0 + 1.0), galley, col);
            if resp.clicked() {
                self.tab = tab;
                self.file_menu_open = false;
            }
            x = tr.right() + 1.0;
        }

        // bottom border, broken under the selected tab
        let s = Stroke::new(1.0_f32, theme::RIBBON_BORDER);
        match sel_span {
            Some((l, r)) => {
                p.line_segment([pos2(rect.left(), base_y), pos2(l, base_y)], s);
                p.line_segment([pos2(r, base_y), pos2(rect.right(), base_y)], s);
            }
            None => {
                p.line_segment([pos2(rect.left(), base_y), pos2(rect.right(), base_y)], s);
            }
        }

        // Quick access toolbar and help on the right
        let mut rx = rect.right() - 26.0;
        let help = icon_button(ui, &mut self.icons, Rect::from_min_size(pos2(rx, rect.top() + 1.0), vec2(22.0, 22.0)), Id::new("help"), Icon::Help, false, true);
        if tip(help, "Help", "About this program.").clicked() {
            self.dialog = Some(Dialog::About);
        }
        rx -= 12.0;
        let sep_x = rx.round() + 0.5;
        p.line_segment([pos2(sep_x, rect.top() + 5.0), pos2(sep_x, rect.bottom() - 5.0)], Stroke::new(1.0_f32, theme::GROUP_SEP));
        let can_redo = self.history.can_redo();
        let can_undo = self.history.can_undo() || self.active_shape.is_some() || self.text_box.is_some();
        for (icon, name, key, en) in [(Icon::Redo, "Redo", "Ctrl+Y", can_redo), (Icon::Undo, "Undo", "Ctrl+Z", can_undo), (Icon::Save, "Save", "Ctrl+S", true)] {
            rx -= 24.0;
            let r = icon_button(ui, &mut self.icons, Rect::from_min_size(pos2(rx, rect.top() + 1.0), vec2(22.0, 22.0)), Id::new(("qat", name)), icon, false, en);
            if tip(r, &format!("{name} ({key})"), "").clicked() {
                match icon {
                    Icon::Redo => self.redo(),
                    Icon::Undo => self.undo(),
                    _ => {
                        self.save();
                    }
                }
            }
        }
        rx -= 8.0;
        let sep_x = rx.round() + 0.5;
        p.line_segment([pos2(sep_x, rect.top() + 5.0), pos2(sep_x, rect.bottom() - 5.0)], theme::sk(1.0, theme::GROUP_SEP));
        let w = self.style_toggle_width(ui);
        self.style_toggle_button(ui, Rect::from_min_size(pos2(rx - 6.0 - w, rect.top() + 1.0), vec2(w, 22.0)));
    }

    // ----- ribbon body ---------------------------------------------------------

    pub fn ribbon_ui(&mut self, ui: &mut Ui) {
        let rect = ui.max_rect();
        let p = ui.painter().clone();
        p.rect_filled(rect, 0.0, theme::RIBBON_BG);
        let y = rect.bottom() - 0.5;
        p.line_segment([pos2(rect.left(), y), pos2(rect.right(), y)], Stroke::new(1.0_f32, theme::RIBBON_BORDER));
        let top = rect.top() + 3.0;
        let bottom = rect.bottom() - 1.0;
        let mut x = rect.left() + 4.0;
        match self.tab {
            Tab::Home => {
                self.clipboard_group(ui, &mut x, top, bottom);
                self.image_group(ui, &mut x, top, bottom);
                self.tools_group(ui, &mut x, top, bottom);
                self.brushes_group(ui, &mut x, top, bottom);
                self.shapes_group(ui, &mut x, top, bottom);
                self.size_group(ui, &mut x, top, bottom);
                self.colors_group(ui, &mut x, top, bottom);
                self.layers_group(ui, &mut x, top, bottom);
            }
            Tab::View => self.view_tab(ui, &mut x, top, bottom),
            Tab::Text => {
                self.clipboard_group(ui, &mut x, top, bottom);
                self.font_group(ui, &mut x, top, bottom);
                self.background_group(ui, &mut x, top, bottom);
                self.colors_group(ui, &mut x, top, bottom);
            }
        }
        self.popups_ui(ui.ctx());
    }

    fn clipboard_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(44.0, 66.0));
        let sr = big_button(ui, &mut self.icons, r, Id::new("paste"), Some(Icon::Paste), "Paste", true, true, false, true);
        let top_r = tip(sr.top, "Paste (Ctrl+V)", "Paste the contents of the Clipboard.");
        if top_r.clicked() {
            self.paste(ui.ctx());
        }
        if sr.bottom.clicked() {
            self.toggle_popup(PopupKind::Paste, r);
        }
        *x += 46.0;
        let has_sel = self.selection.is_some();
        let w = 58.0;
        let cut = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 1.0), vec2(w, 22.0)), Id::new("cut"), Some(Icon::Cut), "Cut", false, false, has_sel);
        if tip(cut, "Cut (Ctrl+X)", "Cut the selection from the canvas and put it on the Clipboard.").clicked() {
            self.cut();
        }
        let copy = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 23.0), vec2(w, 22.0)), Id::new("copy"), Some(Icon::Copy), "Copy", false, false, has_sel);
        if tip(copy, "Copy (Ctrl+C)", "Copy the selection from the canvas and put it on the Clipboard.").clicked() {
            self.copy();
        }
        *x += w + 4.0;
        group(ui.painter(), g0, *x, top, bottom, "Clipboard");
        *x += 5.0;
    }

    fn image_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(46.0, 66.0));
        let selected = matches!(self.tool, Tool::Select | Tool::FreeSelect);
        let icon = if self.tool == Tool::FreeSelect { Icon::FreeSelect } else { Icon::Select };
        let sr = big_button(ui, &mut self.icons, r, Id::new("select"), Some(icon), "Select", true, true, selected, true);
        if tip(sr.top, "Select", "Select a part of the picture.").clicked() {
            if !selected {
                self.set_tool(Tool::Select);
            }
        }
        if sr.bottom.clicked() {
            self.toggle_popup(PopupKind::Select, r);
        }
        *x += 48.0;
        let w = 72.0;
        let has_sel = self.selection.is_some();
        let crop = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 1.0), vec2(w, 22.0)), Id::new("crop"), Some(Icon::Crop), "Crop", false, false, has_sel);
        if tip(crop, "Crop (Ctrl+Shift+X)", "Crop the picture so it only contains the current selection.").clicked() {
            self.crop();
        }
        let resize = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 23.0), vec2(w, 22.0)), Id::new("resize"), Some(Icon::Resize), "Resize", false, false, true);
        if tip(resize, "Resize and skew (Ctrl+W)", "Resize and skew the picture or selection.").clicked() {
            self.dialog = Some(Dialog::resize(self));
        }
        let rr = Rect::from_min_size(pos2(*x, top + 45.0), vec2(w, 22.0));
        let rot = small_button(ui, &mut self.icons, rr, Id::new("rotate"), Some(Icon::Rotate), "Rotate", true, false, true);
        if tip(rot, "Rotate", "Rotate or flip the picture or selection.").clicked() {
            self.toggle_popup(PopupKind::Rotate, rr);
        }
        *x += w + 4.0;
        group(ui.painter(), g0, *x, top, bottom, "Image");
        *x += 5.0;
    }

    fn tools_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let tools = [
            (Tool::Pencil, Icon::Pencil, "Pencil", "Draw a free-form line with the selected line width."),
            (Tool::Fill, Icon::Fill, "Fill with color", "Click an area on the canvas to fill it with the foreground color, or right-click to fill it with the background color."),
            (Tool::Text, Icon::Text, "Text", "Insert text into the picture."),
            (Tool::Eraser, Icon::Eraser, "Eraser", "Erase part of the picture and replace it with the background color. Right-click replaces only Color 1."),
            (Tool::Picker, Icon::Picker, "Color picker", "Pick a color from the picture and use it for drawing."),
            (Tool::Magnifier, Icon::Magnifier, "Magnifier", "Change the magnification for a part of the picture. Left-click zooms in, right-click zooms out."),
        ];
        for (i, (tool, icon, name, desc)) in tools.iter().enumerate() {
            let col = (i % 3) as f32;
            let row = (i / 3) as f32;
            let r = Rect::from_min_size(pos2(*x + 2.0 + col * 23.0, top + 6.0 + row * 24.0), vec2(22.0, 22.0));
            let resp = icon_button(ui, &mut self.icons, r, Id::new(("tool", *name)), *icon, self.tool == *tool, true);
            if tip(resp, name, desc).clicked() {
                self.set_tool(*tool);
            }
        }
        *x += 74.0;
        group(ui.painter(), g0, *x, top, bottom, "Tools");
        *x += 5.0;
    }

    fn brushes_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(52.0, 66.0));
        let sr = big_button(ui, &mut self.icons, r, Id::new("brushes"), Some(Icon::Brushes), "Brushes", true, true, self.tool == Tool::Brush, true);
        if tip(sr.top, "Brushes", &format!("Draw with different kinds of brushes.\nCurrent: {}", self.brush.name())).clicked() {
            self.set_tool(Tool::Brush);
        }
        if sr.bottom.clicked() {
            self.toggle_popup(PopupKind::Brushes, r);
        }
        *x += 54.0;
        group(ui.painter(), g0, *x, top, bottom, "");
        *x += 5.0;
    }

    fn shapes_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let cols = 8;
        let cell = 22.0;
        let gr = widgets::snap(Rect::from_min_size(pos2(*x, top + 1.0), vec2(cols as f32 * cell + 2.0, 3.0 * cell + 2.0)));
        let p = ui.painter().clone();
        p.rect_filled(gr, 0.0, Color32::WHITE);
        p.rect_stroke(gr, 0.0, Stroke::new(1.0_f32, theme::RIBBON_BORDER), StrokeKind::Inside);
        for (i, kind) in ShapeKind::ALL.iter().enumerate() {
            let c = (i % cols) as f32;
            let rrow = (i / cols) as f32;
            let r = Rect::from_min_size(pos2(gr.left() + 1.0 + c * cell, gr.top() + 1.0 + rrow * cell), vec2(cell, cell));
            let resp = ui.interact(r, Id::new(("shape", i)), Sense::click());
            let selected = self.tool == Tool::Shape && self.shape == *kind;
            widgets::paint_button_bg(&p, widgets::snap(r), resp.hovered(), resp.is_pointer_button_down_on(), selected);
            draw_shape_icon(&p, *kind, r.shrink(4.0));
            if tip(resp, kind.name(), "").clicked() {
                self.commit_all();
                self.shape = *kind;
                self.tool = Tool::Shape;
                self.display_dirty = true;
            }
        }
        *x = gr.right() + 4.0;
        let en = self.tool == Tool::Shape || self.active_shape.is_some();
        let w = 66.0;
        let orr = Rect::from_min_size(pos2(*x, top + 1.0), vec2(w, 22.0));
        let o = small_button(ui, &mut self.icons, orr, Id::new("outline"), Some(Icon::Outline), "Outline", true, false, en);
        if tip(o, "Shape outline", "Select the material for the shape outline.").clicked() {
            self.toggle_popup(PopupKind::Outline, orr);
        }
        let frr = Rect::from_min_size(pos2(*x, top + 23.0), vec2(w, 22.0));
        let f = small_button(ui, &mut self.icons, frr, Id::new("fill"), Some(Icon::ShapeFill), "Fill", true, false, en);
        if tip(f, "Shape fill", "Select the material for the shape fill. Fill uses Color 2.").clicked() {
            self.toggle_popup(PopupKind::Fill, frr);
        }
        *x += w + 4.0;
        group(ui.painter(), g0, *x, top, bottom, "Shapes");
        *x += 5.0;
    }

    fn size_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(46.0, 66.0));
        let en = self.size_slot().is_some();
        let sr = big_button(ui, &mut self.icons, r, Id::new("size"), Some(Icon::Size), "Size", true, false, false, en);
        if tip(sr.top, "Size", "Select the width for the selected tool.").clicked() {
            self.toggle_popup(PopupKind::Size, r);
        }
        *x += 48.0;
        group(ui.painter(), g0, *x, top, bottom, "");
        *x += 5.0;
    }

    fn color_button(&mut self, ui: &mut Ui, r: Rect, slot: u8) {
        let id = Id::new(("color-btn", slot));
        let resp = ui.interact(widgets::snap(r), id, Sense::click());
        let p = ui.painter();
        let selected = self.active_color == slot;
        widgets::paint_button_bg(p, widgets::snap(r), resp.hovered(), resp.is_pointer_button_down_on(), selected);
        let sw = widgets::snap(Rect::from_min_size(pos2(r.center().x - 12.0, r.top() + 6.0), vec2(24.0, 24.0)));
        p.rect_filled(sw, 0.0, Color32::WHITE);
        p.rect_stroke(sw, 0.0, Stroke::new(1.0_f32, Color32::from_rgb(128, 128, 128)), StrokeKind::Inside);
        p.rect_filled(sw.shrink(2.0), 0.0, if slot == 1 { self.color1 } else { self.color2 });
        p.text(pos2(r.center().x, r.top() + 47.0), Align2::CENTER_CENTER, "Color", widgets::font(), theme::TEXT);
        p.text(pos2(r.center().x, r.top() + 61.0), Align2::CENTER_CENTER, if slot == 1 { "1" } else { "2" }, widgets::font(), theme::TEXT);
        let (title, body) = if slot == 1 {
            ("Color 1 (foreground color)", "Click here and then select a color from the color palette. This color is used with the pencil and with brushes, as well as for shape outlines.")
        } else {
            ("Color 2 (background color)", "Click here and then select a color from the color palette. This color is used with the eraser and for shape fills.")
        };
        if tip(resp, title, body).clicked() {
            self.active_color = slot;
        }
    }

    fn colors_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        self.color_button(ui, Rect::from_min_size(pos2(*x, top), vec2(42.0, 66.0)), 1);
        *x += 44.0;
        self.color_button(ui, Rect::from_min_size(pos2(*x, top), vec2(42.0, 66.0)), 2);
        *x += 46.0;
        let pitch = 22.0;
        let sw = 18.0;
        for row in 0..3 {
            for col in 0..10 {
                let r = Rect::from_min_size(pos2(*x + col as f32 * pitch, top + 3.0 + row as f32 * pitch), vec2(sw, sw));
                let color = if row < 2 { Some(PALETTE[row][col]) } else { self.custom_colors[col] };
                let resp = ui.interact(r, Id::new(("pal", row, col)), Sense::click());
                widgets::swatch(ui.painter(), r, color, resp.hovered() && color.is_some());
                if let Some(c) = color {
                    let name = if row < 2 { PALETTE_NAMES[row][col].to_string() } else { format!("Custom color\nRed: {} Green: {} Blue: {}", c.r(), c.g(), c.b()) };
                    let resp = if resp.hovered() { resp.on_hover_text(name) } else { resp };
                    if resp.clicked() {
                        let slot = self.active_color;
                        self.set_color(slot, c);
                    }
                    if resp.secondary_clicked() {
                        self.set_color(2, c);
                    }
                }
            }
        }
        *x += 10.0 * pitch + 2.0;
        let r = Rect::from_min_size(pos2(*x, top), vec2(48.0, 66.0));
        let sr = big_button(ui, &mut self.icons, r, Id::new("edit-colors"), Some(Icon::EditColors), "Edit\ncolors", false, false, false, true);
        if tip(sr.top, "Edit colors", "Select a color from the color palette.").clicked() {
            let c = if self.active_color == 1 { self.color1 } else { self.color2 };
            self.dialog = Some(Dialog::edit_colors(c, self.active_color, &self.dialog_custom));
        }
        *x += 52.0;
        group(ui.painter(), g0, *x, top, bottom, "Colors");
        *x += 5.0;
    }

    fn layers_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(48.0, 66.0));
        let sr = big_button(ui, &mut self.icons, r, Id::new("classic-layers"), Some(Icon::Layers), "Layers", false, false, self.show_layers, true);
        if tip(sr.top, "Layers", "Show or hide the Layers panel.").clicked() {
            self.show_layers = !self.show_layers;
        }
        *x += 52.0;
        group(ui.painter(), g0, *x, top, bottom, "");
        *x += 5.0;
    }

    fn view_tab(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let items = [(Icon::ZoomIn, "Zoom\nin", 1), (Icon::ZoomOut, "Zoom\nout", -1), (Icon::Zoom100, "100\n%", 0)];
        for (icon, label, dir) in items {
            let r = Rect::from_min_size(pos2(*x, top), vec2(46.0, 66.0));
            let en = match dir {
                1 => self.zoom < 8.0,
                -1 => self.zoom > 0.125,
                _ => true,
            };
            let sr = big_button(ui, &mut self.icons, r, Id::new(("zoom", dir)), Some(icon), label, false, false, false, en);
            let (t, b) = match dir {
                1 => ("Zoom in (Ctrl+PgUp)", "Zoom in on the picture."),
                -1 => ("Zoom out (Ctrl+PgDn)", "Zoom out of the picture."),
                _ => ("100%", "View the picture at 100%."),
            };
            if tip(sr.top, t, b).clicked() {
                if dir == 0 {
                    self.set_zoom(1.0, None);
                } else {
                    self.zoom_step(dir, None);
                }
            }
            *x += 48.0;
        }
        group(ui.painter(), g0, *x, top, bottom, "Zoom");
        *x += 8.0;

        let g1 = *x;
        let mut changed = false;
        for (i, (label, key)) in [("Rulers", "Ctrl+R"), ("Gridlines", "Ctrl+G"), ("Status bar", "")].iter().enumerate() {
            let r = Rect::from_min_size(pos2(*x, top + 2.0 + i as f32 * 22.0), vec2(96.0, 20.0));
            let resp = ui.interact(r, Id::new(("show", i)), Sense::click());
            let val = match i {
                0 => self.show_rulers,
                1 => self.show_grid,
                _ => self.show_status,
            };
            let p = ui.painter();
            let bx = widgets::snap(Rect::from_min_size(pos2(r.left() + 3.0, r.center().y - 6.5), vec2(13.0, 13.0)));
            p.rect_filled(bx, 0.0, if resp.hovered() { Color32::from_rgb(229, 241, 251) } else { Color32::WHITE });
            p.rect_stroke(bx, 0.0, Stroke::new(1.0_f32, if resp.hovered() { Color32::from_rgb(0, 120, 215) } else { Color32::from_rgb(51, 51, 51) }), StrokeKind::Inside);
            if val {
                let c = bx.center();
                let s = Stroke::new(1.6_f32, Color32::from_rgb(33, 33, 33));
                p.line_segment([pos2(c.x - 3.5, c.y), pos2(c.x - 1.0, c.y + 2.8)], s);
                p.line_segment([pos2(c.x - 1.0, c.y + 2.8), pos2(c.x + 3.8, c.y - 3.0)], s);
            }
            p.text(pos2(bx.right() + 6.0, r.center().y), Align2::LEFT_CENTER, *label, widgets::font(), theme::TEXT);
            let resp = if key.is_empty() { resp } else { tip(resp, &format!("{label} ({key})"), "") };
            if resp.clicked() {
                changed = true;
                match i {
                    0 => self.show_rulers = !self.show_rulers,
                    1 => self.show_grid = !self.show_grid,
                    _ => self.show_status = !self.show_status,
                }
            }
        }
        let _ = changed;
        *x += 100.0;
        group(ui.painter(), g1, *x, top, bottom, "Show or hide");
        *x += 8.0;

        let g2 = *x;
        let r = Rect::from_min_size(pos2(*x, top), vec2(50.0, 66.0));
        let fs = big_button(ui, &mut self.icons, r, Id::new("fullscreen"), Some(Icon::FullScreen), "Full\nscreen", false, false, false, true);
        if tip(fs.top, "Full screen (F11)", "View the picture in full screen.").clicked() {
            self.toggle_fullscreen_view(ui.ctx());
        }
        *x += 52.0;
        let r = Rect::from_min_size(pos2(*x, top), vec2(62.0, 66.0));
        let th = big_button(ui, &mut self.icons, r, Id::new("thumbnail"), Some(Icon::Thumbnail), "Thumbnail", false, false, self.show_thumbnail, true);
        if tip(th.top, "Thumbnail", "Show or hide the Thumbnail window.").clicked() {
            self.show_thumbnail = !self.show_thumbnail;
        }
        *x += 64.0;
        group(ui.painter(), g2, *x, top, bottom, "Display");
    }

    fn font_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let face_rect = Rect::from_min_size(pos2(*x + 2.0, top + 4.0), vec2(150.0, 22.0));
        let size_rect = Rect::from_min_size(pos2(face_rect.right() + 6.0, top + 4.0), vec2(56.0, 22.0));
        let mut changed = self.font_combos(ui, face_rect, size_rect);
        changed |= self.font_style_buttons(ui, pos2(*x + 2.0, top + 32.0), 24.0, 25.0);
        if changed {
            self.fit_text_box();
        }
        *x += 222.0;
        group(ui.painter(), g0, *x, top, bottom, "Font");
        *x += 5.0;
    }

    /// Font family and size combo boxes. Returns true if something changed.
    pub(crate) fn font_combos(&mut self, ui: &mut Ui, face_rect: Rect, size_rect: Rect) -> bool {
        let mut changed = false;
        ui.scope_builder(UiBuilder::new().max_rect(face_rect), |ui| {
            let current = self.fonts.faces.get(self.text.face).map(|f| f.name.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt("font-face").width(face_rect.width() - 4.0).selected_text(current).show_ui(ui, |ui| {
                ui.set_min_width(180.0);
                for (i, f) in self.fonts.faces.iter().enumerate() {
                    if ui.selectable_label(i == self.text.face, &f.name).clicked() {
                        self.text.face = i;
                        changed = true;
                    }
                }
            });
        });
        ui.scope_builder(UiBuilder::new().max_rect(size_rect), |ui| {
            egui::ComboBox::from_id_salt("font-size").width(size_rect.width() - 4.0).selected_text(format!("{}", self.text.size_pt)).show_ui(ui, |ui| {
                for s in FONT_SIZES {
                    if ui.selectable_label((self.text.size_pt - s as f32).abs() < 0.01, s.to_string()).clicked() {
                        self.text.size_pt = s as f32;
                        changed = true;
                    }
                }
            });
        });
        changed
    }

    /// Bold / Italic / Underline / Strikethrough toggles laid out in a row.
    pub(crate) fn font_style_buttons(&mut self, ui: &mut Ui, origin: Pos2, size: f32, pitch: f32) -> bool {
        let mut changed = false;
        let styles = [("B", "Bold"), ("I", "Italic"), ("U", "Underline"), ("abc", "Strikethrough")];
        let col = theme::text_color();
        for (i, (_, tipt)) in styles.iter().enumerate() {
            let r = widgets::snap(Rect::from_min_size(pos2(origin.x + i as f32 * pitch, origin.y), vec2(size, size)));
            let resp = ui.interact(r, Id::new(("fontstyle", i)), Sense::click());
            let on = match i {
                0 => self.text.bold,
                1 => self.text.italic,
                2 => self.text.underline,
                _ => self.text.strike,
            };
            let p = ui.painter();
            widgets::paint_button_bg(p, r, resp.hovered(), resp.is_pointer_button_down_on(), on);
            let c = r.center();
            match i {
                0 => {
                    p.text(c, Align2::CENTER_CENTER, "B", theme::bold(14.0), col);
                }
                1 => {
                    let s = theme::sk(1.6, col);
                    p.line_segment([pos2(c.x + 2.5, c.y - 5.5), pos2(c.x - 2.5, c.y + 5.5)], s);
                    p.line_segment([pos2(c.x - 0.5, c.y - 5.5), pos2(c.x + 5.0, c.y - 5.5)], theme::sk(1.2, col));
                    p.line_segment([pos2(c.x - 5.0, c.y + 5.5), pos2(c.x + 0.5, c.y + 5.5)], theme::sk(1.2, col));
                }
                2 => {
                    p.text(pos2(c.x, c.y - 1.0), Align2::CENTER_CENTER, "U", egui::FontId::proportional(14.0), col);
                    p.line_segment([pos2(c.x - 5.0, c.y + 7.0), pos2(c.x + 5.0, c.y + 7.0)], theme::sk(1.2, col));
                }
                _ => {
                    p.text(c, Align2::CENTER_CENTER, "abc", egui::FontId::proportional(12.0), col);
                    p.line_segment([pos2(c.x - 8.0, c.y + 0.5), pos2(c.x + 8.0, c.y + 0.5)], theme::sk(1.2, col));
                }
            }
            if tip(resp, tipt, "").clicked() {
                changed = true;
                match i {
                    0 => self.text.bold = !self.text.bold,
                    1 => self.text.italic = !self.text.italic,
                    2 => self.text.underline = !self.text.underline,
                    _ => self.text.strike = !self.text.strike,
                }
            }
        }
        changed
    }

    fn background_group(&mut self, ui: &mut Ui, x: &mut f32, top: f32, bottom: f32) {
        let g0 = *x;
        let w = 96.0;
        let o = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 4.0), vec2(w, 22.0)), Id::new("opaque"), Some(Icon::Opaque), "Opaque", false, self.text.opaque, true);
        if tip(o, "Opaque", "Fill the text background with Color 2.").clicked() {
            self.text.opaque = true;
            self.display_dirty = true;
        }
        let t = small_button(ui, &mut self.icons, Rect::from_min_size(pos2(*x, top + 28.0), vec2(w, 22.0)), Id::new("transparent"), Some(Icon::Transparent), "Transparent", false, !self.text.opaque, true);
        if tip(t, "Transparent", "Let the picture show through behind the text.").clicked() {
            self.text.opaque = false;
            self.display_dirty = true;
        }
        *x += w + 4.0;
        group(ui.painter(), g0, *x, top, bottom, "Background");
        *x += 5.0;
    }

    // ----- dropdown popups -------------------------------------------------------

    pub(crate) fn popups_ui(&mut self, ctx: &Context) {
        let Some(pp) = self.popup else { return };
        let pivot = if pp.above { Align2::LEFT_BOTTOM } else { Align2::LEFT_TOP };
        let area = egui::Area::new(Id::new("ribbon-popup")).order(Order::Foreground).pivot(pivot).fixed_pos(pp.pos).constrain(true).show(ctx, |ui| {
            widgets::popup_frame()
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(0.0, 0.0);
                    self.popup_contents(ui, pp.kind)
                })
                .inner
        });
        let close = area.inner;
        let pressed_outside = ctx.input(|i| {
            i.pointer.any_pressed() && i.pointer.interact_pos().is_some_and(|p| !area.response.rect.contains(p) && !pp.opener.contains(p))
        });
        let esc = ctx.input(|i| i.key_pressed(egui::Key::Escape));
        if close || pressed_outside || esc {
            self.popup = None;
        }
    }

    fn popup_header(ui: &mut Ui, text: &str) {
        let w = ui.available_width().max(190.0);
        if theme::modern() {
            let (r, _) = ui.allocate_exact_size(vec2(w, 26.0), Sense::hover());
            ui.painter().text(pos2(r.left() + 12.0, r.center().y + 1.0), Align2::LEFT_CENTER, text, theme::bold(11.5), theme::w().subtle);
            return;
        }
        let (r, _) = ui.allocate_exact_size(vec2(w, 22.0), Sense::hover());
        ui.painter().rect_filled(r, 0.0, Color32::from_rgb(238, 238, 242));
        ui.painter().text(pos2(r.left() + 8.0, r.center().y), Align2::LEFT_CENTER, text, theme::bold(12.0), theme::TEXT);
    }

    /// Returns true when the popup should close.
    fn popup_contents(&mut self, ui: &mut Ui, kind: PopupKind) -> bool {
        let mut close = false;
        match kind {
            PopupKind::Paste => {
                ui.set_width(170.0);
                if menu_item(ui, &mut self.icons, Some(Icon::Paste), "Paste", false, true).clicked() {
                    self.paste(&ui.ctx().clone());
                    close = true;
                }
                if menu_item(ui, &mut self.icons, Some(Icon::Open), "Paste from", false, true).clicked() {
                    close = true;
                    self.paste_from_file();
                }
            }
            PopupKind::Select => {
                ui.set_width(210.0);
                Self::popup_header(ui, "Selection shapes");
                if menu_item(ui, &mut self.icons, Some(Icon::RectSelect), "Rectangular selection", self.tool == Tool::Select, true).clicked() {
                    self.set_tool(Tool::Select);
                    close = true;
                }
                if menu_item(ui, &mut self.icons, Some(Icon::FreeSelect), "Free-form selection", self.tool == Tool::FreeSelect, true).clicked() {
                    self.set_tool(Tool::FreeSelect);
                    close = true;
                }
                Self::popup_header(ui, "Selection options");
                if menu_item(ui, &mut self.icons, Some(Icon::SelectAll), "Select all", false, true).clicked() {
                    self.select_all();
                    close = true;
                }
                if menu_item(ui, &mut self.icons, Some(Icon::InvertSelection), "Invert selection", false, true).clicked() {
                    self.invert_selection();
                    close = true;
                }
                if menu_item(ui, &mut self.icons, Some(Icon::Delete), "Delete", false, self.selection.is_some()).clicked() {
                    self.delete_selection();
                    close = true;
                }
                if menu_item(ui, &mut self.icons, Some(Icon::TransparentSelection), "Transparent selection", self.transparent_sel, true).clicked() {
                    self.transparent_sel = !self.transparent_sel;
                    self.display_dirty = true;
                    close = true;
                }
            }
            PopupKind::Rotate | PopupKind::Flip => {
                ui.set_width(180.0);
                let all = [
                    (Icon::RotateRight, "Rotate right 90°", RotOp::Right),
                    (Icon::RotateLeft, "Rotate left 90°", RotOp::Left),
                    (Icon::Rotate180, "Rotate 180°", RotOp::R180),
                    (Icon::FlipVertical, "Flip vertical", RotOp::FlipV),
                    (Icon::FlipHorizontal, "Flip horizontal", RotOp::FlipH),
                ];
                let items: Vec<_> = match (kind, theme::modern()) {
                    (PopupKind::Flip, _) => all[3..].to_vec(),
                    (_, true) => all[..3].to_vec(),
                    _ => all.to_vec(),
                };
                for (icon, label, op) in items {
                    if menu_item(ui, &mut self.icons, Some(icon), label, false, true).clicked() {
                        self.rotate(op);
                        close = true;
                    }
                }
            }
            PopupKind::Brushes => {
                let (cw, ch) = (66.0, 46.0);
                let (gr, _) = ui.allocate_exact_size(vec2(cw * 3.0 + 4.0, ch * 3.0 + 4.0), Sense::hover());
                for (i, kind) in BrushKind::ALL.iter().enumerate() {
                    let r = Rect::from_min_size(pos2(gr.left() + 2.0 + (i % 3) as f32 * cw, gr.top() + 2.0 + (i / 3) as f32 * ch), vec2(cw, ch));
                    let resp = ui.interact(r, Id::new(("brushpick", i)), Sense::click());
                    let selected = self.brush == *kind;
                    widgets::paint_button_bg(ui.painter(), widgets::snap(r.shrink(1.0)), resp.hovered(), resp.is_pointer_button_down_on(), selected);
                    let previews = if theme::dark() { &self.brush_previews_dark } else { &self.brush_previews };
                    if let Some(t) = previews.get(i) {
                        let ir = Rect::from_center_size(r.center(), vec2(60.0, 40.0));
                        ui.painter().image(t.id(), ir, Rect::from_min_max(pos2(0.0, 0.0), pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    let resp = if resp.hovered() { resp.on_hover_text(kind.name()) } else { resp };
                    if resp.clicked() {
                        self.brush = *kind;
                        self.set_tool(Tool::Brush);
                        close = true;
                    }
                }
            }
            PopupKind::Outline => {
                ui.set_width(170.0);
                if menu_item(ui, &mut self.icons, None, "No outline", !self.outline_on, true).clicked() {
                    self.outline_on = false;
                    close = true;
                }
                if menu_item(ui, &mut self.icons, None, "Solid color", self.outline_on, true).clicked() {
                    self.outline_on = true;
                    close = true;
                }
                for name in ["Crayon", "Marker", "Oil", "Natural pencil", "Watercolor"] {
                    menu_item(ui, &mut self.icons, None, name, false, false);
                }
                self.display_dirty |= close;
            }
            PopupKind::Fill => {
                ui.set_width(170.0);
                if menu_item(ui, &mut self.icons, None, "No fill", !self.fill_on, true).clicked() {
                    self.fill_on = false;
                    close = true;
                }
                if menu_item(ui, &mut self.icons, None, "Solid color", self.fill_on, true).clicked() {
                    self.fill_on = true;
                    close = true;
                }
                for name in ["Crayon", "Marker", "Oil", "Natural pencil", "Watercolor"] {
                    menu_item(ui, &mut self.icons, None, name, false, false);
                }
                self.display_dirty |= close;
            }
            PopupKind::Zoom => {
                ui.set_width(170.0);
                for z in ZOOM_LEVELS.iter().rev() {
                    let label = if *z < 1.0 { format!("{}%", z * 100.0) } else { format!("{}%", (z * 100.0) as i32) };
                    if menu_item(ui, &mut self.icons, None, &label, (self.zoom - z).abs() < 1e-4, true).clicked() {
                        self.set_zoom(*z, None);
                        close = true;
                    }
                }
                widgets::menu_separator(ui);
                if menu_item(ui, &mut self.icons, Some(Icon::Thumbnail), "Zoom to fit", false, true).clicked() {
                    self.zoom_to_fit();
                    close = true;
                }
            }
            PopupKind::Size => {
                let Some(slot) = self.size_slot() else { return true };
                let opts = self.size_options(slot);
                for (i, s) in opts.iter().enumerate() {
                    let (r, resp) = ui.allocate_exact_size(vec2(150.0, 30.0), Sense::click());
                    let selected = (self.tool_size(slot) - *s).abs() < 0.01;
                    widgets::paint_button_bg(ui.painter(), widgets::snap(r.shrink(1.0)), resp.hovered(), resp.is_pointer_button_down_on(), selected);
                    ui.painter().text(pos2(r.left() + 10.0, r.center().y), Align2::LEFT_CENTER, format!("{}px", *s as i32), widgets::font(), theme::text_color());
                    let thick = s.min(14.0);
                    let lr = Rect::from_center_size(pos2(r.left() + 95.0, r.center().y), vec2(90.0, thick));
                    ui.painter().rect_filled(widgets::snap(lr), 0.0, theme::text_color());
                    if resp.clicked() {
                        self.size_idx[slot] = i;
                        self.custom_size[slot] = None;
                        self.display_dirty = true;
                        close = true;
                    }
                }
            }
        }
        close
    }
}

/// Draw a gallery icon for a shape inside `r` using the real shape geometry.
pub fn draw_shape_icon(p: &egui::Painter, kind: ShapeKind, r: Rect) {
    let stroke = Stroke::new(1.0_f32, if theme::modern() { theme::text_color() } else { Color32::from_rgb(30, 30, 30) });
    let (a, b) = ((r.left(), r.top()), (r.right(), r.bottom()));
    let path = match kind {
        ShapeKind::Line => shapes::line_path((r.left(), r.top()), (r.right(), r.bottom())),
        ShapeKind::Curve => shapes::curve_path((r.left(), r.bottom()), (r.left() + r.width() * 0.2, r.top() - 4.0), (r.right() - r.width() * 0.2, r.bottom() + 4.0), (r.right(), r.top())),
        ShapeKind::Polygon => shapes::polyline_path(
            &[(r.left() + 2.0, r.top()), (r.right(), r.top() + 4.0), (r.right() - 3.0, r.bottom()), (r.left() + 5.0, r.center().y + 1.0), (r.left(), r.bottom() - 2.0)],
            true,
        ),
        _ => shapes::box_path(kind, a, b),
    };
    for (pts, closed) in shapes::flatten(&path) {
        if closed {
            p.add(egui::Shape::closed_line(pts, stroke));
        } else {
            p.add(egui::Shape::line(pts, stroke));
        }
    }
}
