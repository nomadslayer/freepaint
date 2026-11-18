//! Colors and egui style for the two looks:
//! the classic Windows 10 ribbon (always light, like the original) and the
//! Windows 11 toolbar (light or dark, following the desktop by default).

use std::sync::atomic::{AtomicBool, Ordering};

use egui::{Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle};

// ----- classic (Windows 10 ribbon) ----------------------------------------
pub const WORKSPACE: Color32 = Color32::from_rgb(201, 211, 226);
pub const TAB_BAR: Color32 = Color32::from_rgb(255, 255, 255);
pub const RIBBON_BG: Color32 = Color32::from_rgb(245, 246, 247);
pub const RIBBON_BORDER: Color32 = Color32::from_rgb(218, 219, 220);
pub const GROUP_SEP: Color32 = Color32::from_rgb(226, 227, 228);
pub const GROUP_LABEL: Color32 = Color32::from_rgb(96, 96, 96);
pub const TEXT: Color32 = Color32::from_rgb(38, 38, 38);
pub const TEXT_DISABLED: Color32 = Color32::from_rgb(160, 160, 160);
pub const FILE_TAB: Color32 = Color32::from_rgb(25, 121, 202);
pub const FILE_TAB_HOVER: Color32 = Color32::from_rgb(41, 140, 225);
pub const HOVER_BG: Color32 = Color32::from_rgb(232, 239, 247);
pub const HOVER_BORDER: Color32 = Color32::from_rgb(164, 206, 249);
pub const PRESSED_BG: Color32 = Color32::from_rgb(201, 224, 247);
pub const PRESSED_BORDER: Color32 = Color32::from_rgb(98, 162, 228);
pub const STATUS_BG: Color32 = Color32::from_rgb(240, 240, 240);
pub const STATUS_SEP: Color32 = Color32::from_rgb(215, 215, 215);
pub const MENU_BG: Color32 = Color32::from_rgb(252, 252, 252);
pub const MENU_BORDER: Color32 = Color32::from_rgb(204, 204, 204);
pub const DIALOG_BG: Color32 = Color32::from_rgb(240, 240, 240);
pub const HANDLE_BORDER: Color32 = Color32::from_rgb(85, 85, 85);
pub const SELECTION_HANDLE: Color32 = Color32::from_rgb(30, 110, 200);
pub const TEXT_TOOLS_ACCENT: Color32 = Color32::from_rgb(232, 180, 20);

/// Windows 11 palette (one instance for light, one for dark).
pub struct W11 {
    pub header_bg: Color32,
    pub toolbar_bg: Color32,
    pub edge: Color32,
    pub border: Color32,
    pub divider: Color32,
    pub label: Color32,
    pub workspace: Color32,
    pub accent: Color32,
    pub on_accent: Color32,
    pub status_bg: Color32,
    pub text: Color32,
    pub subtle: Color32,
    pub disabled: Color32,
    pub menu_bg: Color32,
    pub menu_border: Color32,
    pub hover: Color32,
    pub pressed: Color32,
    pub sel_bg: Color32,
    pub sel_border: Color32,
    pub gallery_bg: Color32,
    pub swatch_border: Color32,
    pub swatch_empty: Color32,
    pub card_bg: Color32,
    pub dialog_bg: Color32,
    pub dialog_title: Color32,
    pub button_bg: Color32,
    pub button_hover: Color32,
    pub button_border: Color32,
    pub track: Color32,
    pub ruler_bg: Color32,
    pub ruler_tick: Color32,
    pub text_edit_bg: Color32,
    /// Stroke color used by the outline icons (SVG hex).
    pub icon_hex: &'static str,
    /// Background color used inside icons (SVG hex).
    pub icon_bg_hex: &'static str,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const W11_LIGHT: W11 = W11 {
    header_bg: rgb(243, 243, 243),
    toolbar_bg: rgb(249, 249, 249),
    edge: rgb(229, 229, 229),
    border: rgb(229, 229, 229),
    divider: rgb(222, 222, 222),
    label: rgb(96, 96, 96),
    workspace: rgb(240, 240, 240),
    accent: rgb(0, 95, 184),
    on_accent: rgb(255, 255, 255),
    status_bg: rgb(243, 243, 243),
    text: rgb(26, 26, 26),
    subtle: rgb(110, 110, 110),
    disabled: rgb(160, 160, 160),
    menu_bg: rgb(255, 255, 255),
    menu_border: rgb(229, 229, 229),
    hover: Color32::from_rgba_premultiplied(0, 0, 0, 10),
    pressed: Color32::from_rgba_premultiplied(0, 0, 0, 18),
    sel_bg: rgb(218, 232, 248),
    sel_border: rgb(170, 200, 238),
    gallery_bg: rgb(255, 255, 255),
    swatch_border: Color32::from_rgba_premultiplied(0, 0, 0, 45),
    swatch_empty: rgb(196, 196, 196),
    card_bg: rgb(255, 255, 255),
    dialog_bg: rgb(249, 249, 249),
    dialog_title: rgb(243, 243, 243),
    button_bg: rgb(255, 255, 255),
    button_hover: rgb(249, 249, 249),
    button_border: rgb(214, 214, 214),
    track: rgb(196, 196, 196),
    ruler_bg: rgb(250, 250, 250),
    ruler_tick: rgb(110, 110, 110),
    text_edit_bg: rgb(255, 255, 255),
    icon_hex: "#242424",
    icon_bg_hex: "#ffffff",
};

pub const W11_DARK: W11 = W11 {
    header_bg: rgb(32, 32, 32),
    toolbar_bg: rgb(39, 39, 39),
    edge: rgb(28, 28, 28),
    border: rgb(58, 58, 58),
    divider: rgb(55, 55, 55),
    label: rgb(190, 190, 190),
    workspace: rgb(32, 32, 32),
    accent: rgb(76, 194, 255),
    on_accent: rgb(0, 0, 0),
    status_bg: rgb(32, 32, 32),
    text: rgb(240, 240, 240),
    subtle: rgb(170, 170, 170),
    disabled: rgb(105, 105, 105),
    menu_bg: rgb(44, 44, 44),
    menu_border: rgb(62, 62, 62),
    hover: Color32::from_rgba_premultiplied(14, 14, 14, 14),
    pressed: Color32::from_rgba_premultiplied(9, 9, 9, 9),
    sel_bg: rgb(58, 64, 72),
    sel_border: rgb(84, 104, 128),
    gallery_bg: rgb(42, 42, 42),
    swatch_border: Color32::from_rgba_premultiplied(70, 70, 70, 70),
    swatch_empty: rgb(88, 88, 88),
    card_bg: rgb(44, 44, 44),
    dialog_bg: rgb(32, 32, 32),
    dialog_title: rgb(40, 40, 40),
    button_bg: rgb(55, 55, 55),
    button_hover: rgb(62, 62, 62),
    button_border: rgb(72, 72, 72),
    track: rgb(110, 110, 110),
    ruler_bg: rgb(40, 40, 40),
    ruler_tick: rgb(160, 160, 160),
    text_edit_bg: rgb(30, 30, 30),
    icon_hex: "#e8e8e8",
    icon_bg_hex: "#2b2b2b",
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RibbonStyle {
    Classic,
    Modern,
}

impl RibbonStyle {
    pub fn key(self) -> &'static str {
        match self {
            RibbonStyle::Classic => "classic",
            RibbonStyle::Modern => "modern",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        match s.trim() {
            "classic" | "win10" => Some(RibbonStyle::Classic),
            "modern" | "win11" => Some(RibbonStyle::Modern),
            _ => None,
        }
    }
}

/// App theme choice for the Windows 11 look.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

impl ThemeMode {
    pub fn key(self) -> &'static str {
        match self {
            ThemeMode::System => "system",
            ThemeMode::Light => "light",
            ThemeMode::Dark => "dark",
        }
    }

    pub fn from_key(s: &str) -> Option<Self> {
        match s.trim() {
            "system" => Some(ThemeMode::System),
            "light" => Some(ThemeMode::Light),
            "dark" => Some(ThemeMode::Dark),
            _ => None,
        }
    }
}

static MODERN: AtomicBool = AtomicBool::new(false);
static DARK: AtomicBool = AtomicBool::new(false);

/// True while the Windows 11 look is active.
pub fn modern() -> bool {
    MODERN.load(Ordering::Relaxed)
}

/// True while the Windows 11 look is shown in dark mode.
pub fn dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Current Windows 11 palette.
pub fn w() -> &'static W11 {
    if dark() {
        &W11_DARK
    } else {
        &W11_LIGHT
    }
}

pub fn workspace() -> Color32 {
    if modern() {
        w().workspace
    } else {
        WORKSPACE
    }
}

pub fn text_color() -> Color32 {
    if modern() {
        w().text
    } else {
        TEXT
    }
}

pub fn disabled_color() -> Color32 {
    if modern() {
        w().disabled
    } else {
        TEXT_DISABLED
    }
}

pub fn sk(w: f32, c: Color32) -> Stroke {
    Stroke::new(w, c)
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert("ui".into(), FontData::from_static(crate::text::UI_FONT).into());
    fonts.font_data.insert("ui-bold".into(), FontData::from_static(crate::text::UI_FONT_BOLD).into());
    fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "ui".into());
    let mut bold = vec!["ui-bold".to_string()];
    bold.extend(fonts.families[&FontFamily::Proportional].iter().skip(1).cloned());
    fonts.families.insert(FontFamily::Name("bold".into()), bold);
    ctx.set_fonts(fonts);
    ctx.options_mut(|o| o.zoom_with_keyboard = false);
}

/// Configure egui visuals for the chosen look. `dark` only applies to the Windows 11 look.
pub fn apply_style(ctx: &egui::Context, style: RibbonStyle, dark: bool) {
    let modern = style == RibbonStyle::Modern;
    let dark = modern && dark;
    MODERN.store(modern, Ordering::Relaxed);
    DARK.store(dark, Ordering::Relaxed);
    ctx.set_theme(if dark { egui::Theme::Dark } else { egui::Theme::Light });
    let p = if dark { &W11_DARK } else { &W11_LIGHT };
    ctx.style_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(10.5)),
            (TextStyle::Body, FontId::proportional(12.0)),
            (TextStyle::Button, FontId::proportional(12.0)),
            (TextStyle::Heading, FontId::proportional(15.0)),
            (TextStyle::Monospace, FontId::monospace(12.0)),
        ]
        .into();
        s.spacing.item_spacing = egui::vec2(6.0, 4.0);
        s.spacing.button_padding = egui::vec2(6.0, 2.0);
        s.spacing.interact_size.y = 20.0;
        s.interaction.tooltip_delay = 0.45;
        let v = &mut s.visuals;
        *v = if dark { egui::Visuals::dark() } else { egui::Visuals::light() };
        let r_widget = if modern { CornerRadius::same(4) } else { CornerRadius::ZERO };
        let r_window = if modern { CornerRadius::same(8) } else { CornerRadius::ZERO };
        v.window_corner_radius = r_window;
        v.menu_corner_radius = r_window;
        v.window_shadow = egui::Shadow { offset: [0, if modern { 6 } else { 3 }], blur: if modern { 16 } else { 8 }, spread: 0, color: Color32::from_black_alpha(if dark { 110 } else { 45 }) };
        v.popup_shadow = egui::Shadow { offset: [0, if modern { 4 } else { 2 }], blur: if modern { 12 } else { 5 }, spread: 0, color: Color32::from_black_alpha(if dark { 100 } else { 40 }) };
        for wv in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
            wv.corner_radius = r_widget;
            wv.expansion = 0.0;
        }
        if modern {
            v.panel_fill = p.toolbar_bg;
            v.window_fill = p.menu_bg;
            v.window_stroke = sk(1.0, p.menu_border);
            v.override_text_color = Some(p.text);
            v.extreme_bg_color = p.text_edit_bg;
            v.selection.bg_fill = if dark { Color32::from_rgb(0, 90, 158) } else { Color32::from_rgb(204, 225, 245) };
            v.selection.stroke = sk(1.0, p.accent);
            v.widgets.noninteractive.bg_stroke = sk(1.0, p.border);
            v.widgets.noninteractive.bg_fill = p.menu_bg;
            v.widgets.inactive.bg_fill = p.button_bg;
            v.widgets.inactive.weak_bg_fill = p.button_bg;
            v.widgets.inactive.bg_stroke = sk(1.0, p.button_border);
            v.widgets.hovered.bg_fill = p.button_hover;
            v.widgets.hovered.weak_bg_fill = p.button_hover;
            v.widgets.hovered.bg_stroke = sk(1.0, p.button_border);
            v.widgets.active.bg_fill = p.button_hover;
            v.widgets.active.weak_bg_fill = p.button_hover;
            v.widgets.active.bg_stroke = sk(1.0, p.accent);
            v.widgets.open.bg_fill = p.button_hover;
            v.widgets.open.weak_bg_fill = p.button_hover;
            v.widgets.open.bg_stroke = sk(1.0, p.accent);
            for wv in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
                wv.fg_stroke.color = p.text;
            }
        } else {
            v.panel_fill = RIBBON_BG;
            v.window_fill = Color32::WHITE;
            v.window_stroke = sk(1.0, Color32::from_rgb(160, 160, 160));
            v.override_text_color = Some(TEXT);
            v.extreme_bg_color = Color32::WHITE;
            v.selection.bg_fill = PRESSED_BG;
            v.selection.stroke = sk(1.0, PRESSED_BORDER);
            v.widgets.noninteractive.bg_stroke = sk(1.0, Color32::from_rgb(200, 200, 200));
            v.widgets.inactive.bg_fill = Color32::from_rgb(225, 225, 225);
            v.widgets.inactive.weak_bg_fill = Color32::from_rgb(225, 225, 225);
            v.widgets.inactive.bg_stroke = sk(1.0, Color32::from_rgb(173, 173, 173));
            v.widgets.hovered.bg_fill = Color32::from_rgb(229, 241, 251);
            v.widgets.hovered.weak_bg_fill = Color32::from_rgb(229, 241, 251);
            v.widgets.hovered.bg_stroke = sk(1.0, Color32::from_rgb(0, 120, 215));
            v.widgets.active.bg_fill = Color32::from_rgb(204, 228, 247);
            v.widgets.active.weak_bg_fill = Color32::from_rgb(204, 228, 247);
            v.widgets.active.bg_stroke = sk(1.0, Color32::from_rgb(0, 84, 153));
            v.widgets.open.bg_fill = Color32::from_rgb(204, 228, 247);
            v.widgets.open.weak_bg_fill = Color32::from_rgb(204, 228, 247);
        }
    });
}

pub fn bold(size: f32) -> FontId {
    FontId::new(size, FontFamily::Name("bold".into()))
}
