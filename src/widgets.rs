//! Hand-painted ribbon / toolbar widgets for both looks.

use egui::{pos2, vec2, Align2, Color32, CornerRadius, FontId, Id, Painter, Pos2, Rect, Response, Sense, Shape, StrokeKind, Ui};

use crate::icons::{Icon, Icons};
use crate::theme::{self, modern, sk};

pub fn font() -> FontId {
    FontId::proportional(12.0)
}

/// Snap a rect to whole points so 1px borders stay crisp.
pub fn snap(r: Rect) -> Rect {
    Rect::from_min_max(pos2(r.min.x.round(), r.min.y.round()), pos2(r.max.x.round(), r.max.y.round()))
}

pub fn paint_button_bg(p: &Painter, rect: Rect, hovered: bool, pressed: bool, selected: bool) {
    if modern() {
        let r = CornerRadius::same(4);
        if selected {
            p.rect_filled(rect, r, theme::w().sel_bg);
            p.rect_stroke(rect, r, sk(1.0, theme::w().sel_border), StrokeKind::Inside);
            if hovered || pressed {
                p.rect_filled(rect, r, theme::w().hover);
            }
        } else if pressed {
            p.rect_filled(rect, r, theme::w().pressed);
        } else if hovered {
            p.rect_filled(rect, r, theme::w().hover);
        }
        return;
    }
    let (fill, stroke) = if pressed || selected {
        (theme::PRESSED_BG, theme::PRESSED_BORDER)
    } else if hovered {
        (theme::HOVER_BG, theme::HOVER_BORDER)
    } else {
        return;
    };
    p.rect_filled(rect, 0.0, fill);
    p.rect_stroke(rect, 0.0, sk(1.0, stroke), StrokeKind::Inside);
}

pub fn down_arrow(p: &Painter, center: Pos2, color: Color32) {
    if modern() {
        chevron_down(p, center, color);
        return;
    }
    let pts = vec![pos2(center.x - 3.0, center.y - 1.5), pos2(center.x + 3.0, center.y - 1.5), pos2(center.x, center.y + 1.5)];
    p.add(Shape::convex_polygon(pts, color, egui::Stroke::NONE));
}

pub fn chevron_down(p: &Painter, c: Pos2, color: Color32) {
    p.add(Shape::line(vec![pos2(c.x - 3.5, c.y - 1.75), pos2(c.x, c.y + 1.75), pos2(c.x + 3.5, c.y - 1.75)], sk(1.2, color)));
}

pub fn chevron_right(p: &Painter, c: Pos2, color: Color32) {
    p.add(Shape::line(vec![pos2(c.x - 1.75, c.y - 3.5), pos2(c.x + 1.75, c.y), pos2(c.x - 1.75, c.y + 3.5)], sk(1.2, color)));
}

pub fn right_arrow(p: &Painter, center: Pos2, color: Color32) {
    if modern() {
        chevron_right(p, center, color);
        return;
    }
    let pts = vec![pos2(center.x - 1.5, center.y - 3.5), pos2(center.x + 2.0, center.y), pos2(center.x - 1.5, center.y + 3.5)];
    p.add(Shape::convex_polygon(pts, color, egui::Stroke::NONE));
}

pub fn check_mark(p: &Painter, c: Pos2, color: Color32) {
    p.add(Shape::line(vec![pos2(c.x - 4.0, c.y), pos2(c.x - 1.2, c.y + 2.8), pos2(c.x + 4.2, c.y - 3.2)], sk(1.5, color)));
}

fn text_color(enabled: bool) -> Color32 {
    if enabled {
        theme::text_color()
    } else {
        theme::disabled_color()
    }
}

pub struct SplitResponse {
    pub top: Response,
    pub bottom: Response,
}

/// Classic large ribbon button: 32px icon on top, label (1-2 lines) below.
/// With `split`, the icon part and the label part respond separately.
#[allow(clippy::too_many_arguments)]
pub fn big_button(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Option<Icon>, label: &str, dropdown: bool, split: bool, selected: bool, enabled: bool) -> SplitResponse {
    let rect = snap(rect);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let p = ui.painter();
    let (top, bottom) = if split {
        let split_y = rect.top() + 39.0;
        let tr = Rect::from_min_max(rect.min, pos2(rect.max.x, split_y));
        let br = Rect::from_min_max(pos2(rect.min.x, split_y), rect.max);
        let top = ui.interact(tr, id.with("top"), sense);
        let bottom = ui.interact(br, id.with("bottom"), sense);
        if top.hovered() || bottom.hovered() || selected {
            paint_button_bg(p, rect, true, false, selected);
            if top.hovered() {
                paint_button_bg(p, tr, true, top.is_pointer_button_down_on(), false);
            }
            if bottom.hovered() {
                paint_button_bg(p, br, true, bottom.is_pointer_button_down_on(), false);
            }
        }
        (top, bottom)
    } else {
        let r = ui.interact(rect, id, sense);
        paint_button_bg(p, rect, r.hovered(), r.is_pointer_button_down_on(), selected);
        (r.clone(), r)
    };
    if let Some(icon) = icon {
        let ir = Rect::from_min_size(pos2(rect.center().x - 16.0, rect.top() + 4.0), vec2(32.0, 32.0));
        icons.paint(p, icon, ir, enabled);
    }
    let lines: Vec<&str> = label.split('\n').collect();
    let col = text_color(enabled);
    let mut y = rect.top() + 47.0;
    for (i, line) in lines.iter().enumerate() {
        let last = i + 1 == lines.len();
        if last && dropdown && lines.len() > 1 {
            let galley = p.layout_no_wrap(line.to_string(), font(), col);
            let w = galley.size().x + 9.0;
            let x = rect.center().x - w / 2.0;
            p.galley(pos2(x, y - galley.size().y / 2.0), galley, col);
            down_arrow(p, pos2(x + w - 3.0, y + 1.0), col);
        } else {
            p.text(pos2(rect.center().x, y), Align2::CENTER_CENTER, *line, font(), col);
        }
        y += 14.0;
    }
    if dropdown && lines.len() == 1 {
        down_arrow(p, pos2(rect.center().x, y + 1.0), col);
    }
    SplitResponse { top, bottom }
}

/// Windows 11 toolbar button: a centered icon with an optional chevron below.
/// With `split` the icon and chevron halves respond separately.
#[allow(clippy::too_many_arguments)]
pub fn w11_button(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Icon, icon_px: f32, chevron: bool, split: bool, selected: bool, enabled: bool) -> SplitResponse {
    let rect = snap(rect);
    let sense = if enabled { Sense::click() } else { Sense::hover() };
    let p = ui.painter();
    let split_y = rect.bottom() - 22.0;
    let (top, bottom) = if split {
        let tr = Rect::from_min_max(rect.min, pos2(rect.max.x, split_y));
        let br = Rect::from_min_max(pos2(rect.min.x, split_y), rect.max);
        let top = ui.interact(tr, id.with("top"), sense);
        let bottom = ui.interact(br, id.with("bottom"), sense);
        let any = top.hovered() || bottom.hovered();
        paint_button_bg(p, rect, any && enabled, false, selected);
        if enabled && top.hovered() {
            paint_button_bg(p, tr, true, top.is_pointer_button_down_on(), false);
        }
        if enabled && bottom.hovered() {
            paint_button_bg(p, br, true, bottom.is_pointer_button_down_on(), false);
        }
        (top, bottom)
    } else {
        let r = ui.interact(rect, id, sense);
        paint_button_bg(p, rect, r.hovered() && enabled, r.is_pointer_button_down_on(), selected);
        (r.clone(), r)
    };
    let icon_cy = if chevron { rect.top() + (split_y - rect.top()) / 2.0 + 2.0 } else { rect.center().y };
    icons.paint(p, icon, Rect::from_center_size(pos2(rect.center().x, icon_cy), vec2(icon_px, icon_px)), enabled);
    if chevron {
        chevron_down(p, pos2(rect.center().x, rect.bottom() - 11.0), text_color(enabled));
    }
    SplitResponse { top, bottom }
}

/// Windows 11 compact button: icon with a chevron to its right.
pub fn w11_icon_chevron(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Icon, selected: bool, enabled: bool) -> Response {
    let rect = snap(rect);
    let r = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    paint_button_bg(p, rect, r.hovered() && enabled, r.is_pointer_button_down_on(), selected);
    let ix = rect.left() + (rect.width() - 30.0) / 2.0 + 10.0;
    icons.paint(p, icon, Rect::from_center_size(pos2(ix, rect.center().y), vec2(20.0, 20.0)), enabled);
    chevron_down(p, pos2(ix + 17.0, rect.center().y + 1.0), text_color(enabled));
    r
}

/// Divider line and group label used by the Windows 11 toolbar.
#[allow(clippy::too_many_arguments)]
pub fn w11_group(p: &Painter, x0: f32, x1: f32, label_y: f32, top: f32, bottom: f32, label: &str, divider: bool) {
    p.text(pos2((x0 + x1) / 2.0, label_y), Align2::CENTER_CENTER, label, FontId::proportional(11.5), theme::w().label);
    if divider {
        let x = (x1 + 12.0).round() + 0.5;
        p.line_segment([pos2(x, top + 6.0), pos2(x, bottom - 6.0)], sk(1.0, theme::w().divider));
    }
}

/// Small ribbon button: 16px icon with a label to its right.
#[allow(clippy::too_many_arguments)]
pub fn small_button(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Option<Icon>, label: &str, dropdown: bool, selected: bool, enabled: bool) -> Response {
    let rect = snap(rect);
    let r = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    paint_button_bg(p, rect, r.hovered() && enabled, r.is_pointer_button_down_on(), selected);
    let mut x = rect.left() + 3.0;
    if let Some(icon) = icon {
        icons.paint(p, icon, Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0)), enabled);
        x += 20.0;
    }
    let col = text_color(enabled);
    if !label.is_empty() {
        let g = p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font(), col);
        x = g.right() + 6.0;
    }
    if dropdown {
        down_arrow(p, pos2(x, rect.center().y + 0.5), col);
    }
    r
}

/// Square icon-only button with a 16px icon.
pub fn icon_button(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Icon, selected: bool, enabled: bool) -> Response {
    icon_button_px(ui, icons, rect, id, icon, 16.0, selected, enabled)
}

#[allow(clippy::too_many_arguments)]
pub fn icon_button_px(ui: &Ui, icons: &mut Icons, rect: Rect, id: Id, icon: Icon, px: f32, selected: bool, enabled: bool) -> Response {
    let rect = snap(rect);
    let r = ui.interact(rect, id, if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    paint_button_bg(p, rect, r.hovered() && enabled, r.is_pointer_button_down_on(), selected);
    icons.paint(p, icon, Rect::from_center_size(rect.center(), vec2(px, px)), enabled);
    r
}

/// Draw the label under a classic ribbon group and the separator line to its right.
pub fn group(p: &Painter, x0: f32, x1: f32, top: f32, bottom: f32, label: &str) {
    if !label.is_empty() {
        p.text(pos2((x0 + x1) / 2.0, bottom - 10.0), Align2::CENTER_CENTER, label, font(), theme::GROUP_LABEL);
    }
    let x = x1.round() + 0.5;
    p.line_segment([pos2(x, top + 3.0), pos2(x, bottom - 3.0)], sk(1.0, theme::GROUP_SEP));
}

/// Classic colored swatch with Windows-style double border.
pub fn swatch(p: &Painter, rect: Rect, color: Option<Color32>, hovered: bool) {
    let rect = snap(rect);
    let outer = if hovered { theme::PRESSED_BORDER } else { Color32::from_rgb(160, 160, 160) };
    p.rect_filled(rect, 0.0, Color32::WHITE);
    p.rect_stroke(rect, 0.0, sk(1.0, outer), StrokeKind::Inside);
    if let Some(c) = color {
        p.rect_filled(rect.shrink(2.0), 0.0, c);
    }
}

/// Windows 11 round swatch.
pub fn swatch_circle(p: &Painter, center: Pos2, r: f32, color: Option<Color32>, hovered: bool, selected: bool) {
    match color {
        Some(c) => {
            p.circle_filled(center, r, c);
            p.circle_stroke(center, r - 0.5, sk(1.0, theme::w().swatch_border));
        }
        None => {
            p.circle_stroke(center, r - 0.5, sk(1.0, theme::w().swatch_empty));
        }
    }
    if selected {
        p.circle_stroke(center, r + 3.0, sk(2.0, theme::w().accent));
    } else if hovered {
        p.circle_stroke(center, r + 2.5, sk(1.5, theme::w().subtle));
    }
}

/// Popup menu row: optional icon, label, check state and optional submenu arrow.
pub fn menu_item(ui: &mut Ui, icons: &mut Icons, icon: Option<Icon>, label: &str, checked: bool, enabled: bool) -> Response {
    menu_item_ex(ui, icons, icon, label, checked, enabled, false, 24.0, 16.0)
}

#[allow(clippy::too_many_arguments)]
/// Style-aware menu row with a right-aligned shortcut label.
pub fn menu_item_sc(ui: &mut Ui, icons: &mut Icons, icon: Option<Icon>, label: &str, shortcut: &str, checked: bool, enabled: bool) -> Response {
    if modern() {
        return menu_row(ui, icons, icon, label, shortcut, checked, enabled, false);
    }
    let r = menu_item_ex(ui, icons, icon, label, checked, enabled, false, 24.0, 16.0);
    if !shortcut.is_empty() {
        ui.painter().text(
            pos2(r.rect.right() - 8.0, r.rect.center().y),
            Align2::RIGHT_CENTER,
            shortcut,
            FontId::proportional(11.0),
            if enabled { theme::GROUP_LABEL } else { theme::TEXT_DISABLED },
        );
    }
    r
}

pub fn menu_item_ex(ui: &mut Ui, icons: &mut Icons, icon: Option<Icon>, label: &str, checked: bool, enabled: bool, submenu: bool, h: f32, icon_size: f32) -> Response {
    if modern() {
        return menu_row(ui, icons, icon, label, "", checked, enabled, submenu);
    }
    let w = ui.available_width().max(170.0);
    let (rect, r) = ui.allocate_exact_size(vec2(w, h), if enabled { Sense::click() } else { Sense::hover() });
    let rect = snap(rect);
    let p = ui.painter();
    if r.hovered() && enabled {
        p.rect_filled(rect, 0.0, theme::HOVER_BG);
        p.rect_stroke(rect, 0.0, sk(1.0, theme::HOVER_BORDER), StrokeKind::Inside);
    }
    let icon_area = Rect::from_min_size(pos2(rect.left() + 4.0, rect.center().y - icon_size / 2.0 - 2.0), vec2(icon_size + 4.0, icon_size + 4.0));
    if checked {
        p.rect_filled(icon_area, 0.0, theme::PRESSED_BG);
        p.rect_stroke(icon_area, 0.0, sk(1.0, theme::PRESSED_BORDER), StrokeKind::Inside);
        if icon.is_none() {
            check_mark(p, icon_area.center(), theme::TEXT);
        }
    }
    if let Some(icon) = icon {
        icons.paint(p, icon, Rect::from_center_size(icon_area.center(), vec2(icon_size, icon_size)), enabled);
    }
    p.text(pos2(icon_area.right() + 8.0, rect.center().y), Align2::LEFT_CENTER, label, font(), text_color(enabled));
    if submenu {
        right_arrow(p, pos2(rect.right() - 10.0, rect.center().y), text_color(enabled));
    }
    r
}

/// Windows 11 flyout menu row with optional shortcut text.
#[allow(clippy::too_many_arguments)]
pub fn menu_row(ui: &mut Ui, icons: &mut Icons, icon: Option<Icon>, label: &str, shortcut: &str, checked: bool, enabled: bool, submenu: bool) -> Response {
    let w = ui.available_width().max(180.0);
    let (rect, r) = ui.allocate_exact_size(vec2(w, 32.0), if enabled { Sense::click() } else { Sense::hover() });
    let p = ui.painter();
    let hl = rect.shrink2(vec2(2.0, 1.0));
    if r.hovered() && enabled {
        p.rect_filled(hl, CornerRadius::same(4), if r.is_pointer_button_down_on() { theme::w().pressed } else { theme::w().hover });
    }
    let col = text_color(enabled);
    let mut x = rect.left() + 12.0;
    match (checked, icon) {
        (true, None) => check_mark(p, pos2(x + 8.0, rect.center().y), col),
        (true, Some(_)) => {
            p.rect_filled(Rect::from_min_size(pos2(rect.left() + 3.0, rect.center().y - 8.0), vec2(3.0, 16.0)), CornerRadius::same(2), theme::w().accent);
        }
        _ => {}
    }
    if let Some(icon) = icon {
        icons.paint(p, icon, Rect::from_min_size(pos2(x, rect.center().y - 8.0), vec2(16.0, 16.0)), enabled);
    }
    x += 28.0;
    p.text(pos2(x, rect.center().y), Align2::LEFT_CENTER, label, font(), col);
    if submenu {
        chevron_right(p, pos2(rect.right() - 14.0, rect.center().y), col);
    } else if !shortcut.is_empty() {
        p.text(pos2(rect.right() - 12.0, rect.center().y), Align2::RIGHT_CENTER, shortcut, FontId::proportional(11.5), theme::w().subtle);
    }
    r
}

/// Thin separator line inside a popup menu.
pub fn menu_separator(ui: &mut Ui) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(vec2(w, if modern() { 9.0 } else { 7.0 }), Sense::hover());
    let y = rect.center().y.round() + 0.5;
    let (l, col) = if modern() { (rect.left() + 4.0, theme::w().divider) } else { (rect.left() + 30.0, Color32::from_rgb(215, 215, 215)) };
    ui.painter().line_segment([pos2(l, y), pos2(rect.right() - 4.0, y)], sk(1.0, col));
}

/// Push button used in dialogs.
pub fn win_button(ui: &mut Ui, label: &str, default: bool) -> Response {
    let (rect, r) = ui.allocate_exact_size(vec2(if modern() { 84.0 } else { 75.0 }, if modern() { 28.0 } else { 23.0 }), Sense::click());
    let rect = snap(rect);
    let p = ui.painter();
    if modern() {
        let cr = CornerRadius::same(4);
        let pal = theme::w();
        if default {
            p.rect_filled(rect, cr, pal.accent);
            if r.hovered() || r.is_pointer_button_down_on() {
                p.rect_filled(rect, cr, if theme::dark() { Color32::from_black_alpha(30) } else { Color32::from_black_alpha(25) });
            }
            p.text(rect.center(), Align2::CENTER_CENTER, label, font(), pal.on_accent);
        } else {
            p.rect_filled(rect, cr, if r.hovered() { pal.button_hover } else { pal.button_bg });
            if r.is_pointer_button_down_on() {
                p.rect_filled(rect, cr, pal.pressed);
            }
            p.rect_stroke(rect, cr, sk(1.0, pal.button_border), StrokeKind::Inside);
            p.text(rect.center(), Align2::CENTER_CENTER, label, font(), pal.text);
        }
        return r;
    }
    let (fill, border) = if r.is_pointer_button_down_on() {
        (Color32::from_rgb(204, 228, 247), Color32::from_rgb(0, 84, 153))
    } else if r.hovered() {
        (Color32::from_rgb(229, 241, 251), Color32::from_rgb(0, 120, 215))
    } else if default {
        (Color32::from_rgb(225, 225, 225), Color32::from_rgb(0, 120, 215))
    } else {
        (Color32::from_rgb(225, 225, 225), Color32::from_rgb(173, 173, 173))
    };
    p.rect_filled(rect, 0.0, fill);
    p.rect_stroke(rect, 0.0, sk(if default { 2.0 } else { 1.0 }, border), StrokeKind::Inside);
    p.text(rect.center(), Align2::CENTER_CENTER, label, font(), theme::TEXT);
    r
}

/// Popup frame used by dropdowns and context menus.
pub fn popup_frame() -> egui::Frame {
    if modern() {
        return egui::Frame::NONE
            .fill(theme::w().menu_bg)
            .stroke(sk(1.0, theme::w().menu_border))
            .corner_radius(CornerRadius::same(8))
            .inner_margin(egui::Margin::same(4))
            .shadow(egui::Shadow { offset: [0, 6], blur: 16, spread: 0, color: Color32::from_black_alpha(if theme::dark() { 110 } else { 38 }) });
    }
    egui::Frame::NONE
        .fill(theme::MENU_BG)
        .stroke(sk(1.0, theme::MENU_BORDER))
        .inner_margin(egui::Margin::same(2))
        .shadow(egui::Shadow { offset: [2, 2], blur: 4, spread: 0, color: Color32::from_black_alpha(40) })
}
