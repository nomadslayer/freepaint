//! Application state and the top-level frame loop.

use std::path::PathBuf;

use egui::{Color32, Context, Event, Frame, Key, KeyboardShortcut, Modifiers, Pos2, TextureHandle, TextureOptions, Vec2};
use tiny_skia::{Pixmap, PixmapPaint, Transform};

use crate::brush::{BrushKind, BrushStroke};
use crate::dialogs::Dialog;
use crate::icons::Icons;
use crate::raster::{self, rgba, IRect, Rng};
use crate::shapes::{self, ShapeKind, ShapeStyle};
use crate::text::{pt_to_px, FontLib, TextStyle};
use crate::theme::{self, RibbonStyle, ThemeMode};

pub const ZOOM_LEVELS: [f32; 11] = [0.125, 0.25, 0.5, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
/// Space between the workspace edge and the picture (classic look).
pub const CANVAS_MARGIN: f32 = 5.0;
/// Space around the picture in the Windows 11 look.
pub const W11_CANVAS_MARGIN: f32 = 28.0;
/// Largest size the size slider allows.
pub const MAX_TOOL_SIZE: f32 = 212.0;
pub const TEXT_PAD: f32 = 3.0;
pub const TAB_H: f32 = 24.0;
pub const RIBBON_H: f32 = 93.0;
pub const STATUS_H: f32 = 24.0;

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

pub const PALETTE: [[Color32; 10]; 2] = [
    [rgb(0, 0, 0), rgb(127, 127, 127), rgb(136, 0, 21), rgb(237, 28, 36), rgb(255, 127, 39), rgb(255, 242, 0), rgb(34, 177, 76), rgb(0, 162, 232), rgb(63, 72, 204), rgb(163, 73, 164)],
    [rgb(255, 255, 255), rgb(195, 195, 195), rgb(185, 122, 87), rgb(255, 174, 201), rgb(255, 201, 14), rgb(239, 228, 176), rgb(181, 230, 29), rgb(153, 217, 234), rgb(112, 146, 190), rgb(200, 191, 231)],
];

pub const PALETTE_NAMES: [[&str; 10]; 2] = [
    ["Black", "Gray-50%", "Dark red", "Red", "Orange", "Yellow", "Green", "Turquoise", "Indigo", "Purple"],
    ["White", "Gray-25%", "Brown", "Rose", "Gold", "Light yellow", "Lime", "Light turquoise", "Blue-gray", "Lavender"],
];

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tool {
    Select,
    FreeSelect,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brush,
    Shape,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Home,
    View,
    Text,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Btn {
    Left,
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Handle {
    N,
    S,
    E,
    W,
    NE,
    NW,
    SE,
    SW,
    P0,
    P1,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum RotOp {
    Right,
    Left,
    R180,
    FlipV,
    FlipH,
}

#[derive(Clone, Debug)]
pub enum PendingAction {
    New,
    Open,
    OpenPath(PathBuf),
    Exit,
}

pub struct Selection {
    /// Current position and size on the canvas.
    pub rect: IRect,
    /// Where the pixels came from.
    pub src: IRect,
    /// Free-form mask (src.w * src.h), `None` for rectangles.
    pub mask: Option<Vec<u8>>,
    /// Free-form outline relative to `src` origin, for display.
    pub outline: Option<Vec<Pos2>>,
    /// Lifted ("floating") pixels, sized like `src`.
    pub content: Option<Pixmap>,
}

impl Selection {
    pub fn rect(r: IRect) -> Self {
        Self { rect: r, src: r, mask: None, outline: None, content: None }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct ActiveShape {
    pub kind: ShapeKind,
    pub a: Pos2,
    pub b: Pos2,
    pub swap: bool,
}

impl ActiveShape {
    pub fn bounds(&self) -> egui::Rect {
        egui::Rect::from_two_pos(self.a, self.b)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct CurveState {
    pub p0: Pos2,
    pub c1: Pos2,
    pub c2: Pos2,
    pub p3: Pos2,
    pub stage: u8,
    pub swap: bool,
}

#[derive(Clone, Debug)]
pub struct PolyState {
    pub pts: Vec<Pos2>,
    pub swap: bool,
}

#[derive(Clone, Debug)]
pub struct TextBox {
    pub rect: IRect,
    pub text: String,
    pub caret: usize,
}

#[derive(Clone, Copy, Debug)]
pub struct TextSettings {
    pub face: usize,
    pub size_pt: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
    pub opaque: bool,
}

#[derive(Clone, Debug, Default)]
pub enum Drag {
    #[default]
    None,
    Stroke,
    Marquee {
        start: (i32, i32),
    },
    Lasso,
    SelMove {
        start: Pos2,
        orig: IRect,
    },
    SelResize {
        handle: Handle,
        orig: IRect,
        start: Pos2,
    },
    ShapeDraw,
    ShapeMove {
        start: Pos2,
        a: Pos2,
        b: Pos2,
    },
    ShapeHandle {
        handle: Handle,
    },
    Curve,
    Poly,
    TextNew {
        start: Pos2,
    },
    TextMove {
        start: Pos2,
        orig: IRect,
    },
    TextResize {
        handle: Handle,
        orig: IRect,
        start: Pos2,
    },
    TextCaret,
    CanvasResize {
        handle: Handle,
    },
    Picker,
}

/// One drawing layer. Layer 0 is the opaque background.
#[derive(Clone)]
pub struct Layer {
    pub pix: Pixmap,
    pub visible: bool,
}

impl Layer {
    pub fn background(w: u32, h: u32, fill: Color32) -> Self {
        Self { pix: raster::new_canvas(w, h, fill), visible: true }
    }

    pub fn clear(w: u32, h: u32) -> Self {
        Self { pix: Pixmap::new(w.max(1), h.max(1)).unwrap(), visible: true }
    }
}

/// Full document state captured for undo.
#[derive(Clone)]
pub struct Snapshot {
    pub layers: Vec<Layer>,
    pub active: usize,
}

impl Snapshot {
    fn bytes(&self) -> usize {
        self.layers.iter().map(|l| l.pix.data().len()).sum()
    }
}

pub struct History {
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
}

impl History {
    const MAX_STEPS: usize = 50;
    const MAX_BYTES: usize = 768 * 1024 * 1024;

    pub fn new() -> Self {
        Self { undo: Vec::new(), redo: Vec::new() }
    }

    pub fn push(&mut self, snapshot: Snapshot) {
        self.undo.push(snapshot);
        self.redo.clear();
        let mut total: usize = self.undo.iter().map(|s| s.bytes()).sum();
        while self.undo.len() > 1 && (self.undo.len() > Self::MAX_STEPS || total > Self::MAX_BYTES) {
            total -= self.undo[0].bytes();
            self.undo.remove(0);
        }
    }

    pub fn undo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let prev = self.undo.pop()?;
        self.redo.push(current);
        Some(prev)
    }

    pub fn redo(&mut self, current: Snapshot) -> Option<Snapshot> {
        let next = self.redo.pop()?;
        self.undo.push(current);
        Some(next)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }
}

pub struct DocInfo {
    pub path: Option<PathBuf>,
    pub dirty: bool,
    pub file_size: Option<u64>,
    pub saved_at: Option<std::time::SystemTime>,
}

impl DocInfo {
    pub fn name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_string())
    }
}

/// Options used for driving the app from the command line (debug / screenshots).
#[derive(Default, Clone)]
pub struct StartupOptions {
    pub open: Option<PathBuf>,
    pub screenshot: Option<PathBuf>,
    pub demo: bool,
    pub menu: Option<String>,
    pub script: Option<PathBuf>,
    pub style: Option<String>,
    pub theme: Option<String>,
    pub size: Option<(f32, f32)>,
    pub maximized: bool,
    pub dump: Option<PathBuf>,
}

pub struct PaintApp {
    pub layers: Vec<Layer>,
    pub active_layer: usize,
    pub doc: DocInfo,
    pub history: History,

    pub tool: Tool,
    pub prev_tool: Tool,
    pub brush: BrushKind,
    pub shape: ShapeKind,
    pub outline_on: bool,
    pub fill_on: bool,
    /// Selected size index for [pencil, brush, shape, eraser].
    pub size_idx: [usize; 4],
    pub color1: Color32,
    pub color2: Color32,
    pub active_color: u8,
    pub custom_colors: [Option<Color32>; 10],
    pub dialog_custom: [Color32; 16],
    pub transparent_sel: bool,
    pub text: TextSettings,

    pub zoom: f32,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub show_status: bool,
    pub show_thumbnail: bool,
    pub fullscreen_view: bool,
    pub tab: Tab,

    pub drag: Drag,
    pub drag_btn: Btn,
    pub drag_cur: Pos2,
    pub layer: Option<Pixmap>,
    pub layer_base: Option<Pixmap>,
    pub layer_opacity: f32,
    pub stroke: Option<BrushStroke>,
    pub last_pt: Pos2,
    pub selection: Option<Selection>,
    pub marquee: Option<IRect>,
    pub lasso: Vec<Pos2>,
    pub active_shape: Option<ActiveShape>,
    pub curve: Option<CurveState>,
    pub poly: Option<PolyState>,
    pub text_box: Option<TextBox>,
    /// New canvas bounds (in current image coordinates) while dragging a canvas handle.
    pub canvas_resize: Option<IRect>,

    pub texture: Option<TextureHandle>,
    pub display_dirty: bool,
    pub icons: Icons,
    pub fonts: FontLib,
    pub brush_previews: Vec<TextureHandle>,
    pub brush_previews_dark: Vec<TextureHandle>,
    pub dialog: Option<Dialog>,
    pub clipboard: Option<arboard::Clipboard>,
    pub internal_clip: Option<Pixmap>,
    pub cursor_px: Option<(i32, i32)>,
    pub pending_scroll: Option<Vec2>,
    pub scroll_offset: Vec2,
    pub viewport: egui::Rect,
    pub recent: Vec<PathBuf>,
    pub title: String,
    pub allow_close: bool,
    pub startup: StartupOptions,
    pub frame_no: u64,
    pub cmd_seen: f64,
    pub last_paste: f64,
    pub rng: Rng,
    pub file_menu_open: bool,
    pub file_menu_saveas: bool,
    pub context_menu: Option<Pos2>,
    pub context_menu_frame: Option<u64>,
    pub popup: Option<crate::ribbon::Popup>,
    pub zoom_accum: f32,
    pub input_blocked: bool,
    pub caret_epoch: f64,
    pub now: f64,
    pub script: Option<crate::script::Script>,
    pub script_done_frame: Option<u64>,
    pub style: RibbonStyle,
    pub w11_menu: Option<(crate::modern::W11Menu, egui::Rect)>,
    pub w11_sub: Option<(crate::modern::W11Sub, f32)>,
    pub theme_mode: ThemeMode,
    /// Theme override from the command line (not saved).
    pub theme_override: bool,
    /// Look currently applied to egui: (style, dark).
    pub applied_look: Option<(RibbonStyle, bool)>,
    /// Free size chosen with the Windows 11 size slider, per size slot.
    pub custom_size: [Option<f32>; 4],
    /// Drawing opacity (Windows 11 look only).
    pub opacity: f32,
    /// Screen position of canvas pixel (0, 0) in the last frame.
    pub view_origin: Pos2,
    /// Offset used to center the picture, frozen while resizing the canvas.
    pub center_offset: Vec2,
    /// Floating panels drawn over the canvas this frame / last frame (they block canvas input).
    pub overlay_rects: Vec<egui::Rect>,
    pub overlay_rects_prev: Vec<egui::Rect>,
    /// Scroll-wheel movement over the floating panel, used for fine slider steps.
    pub slider_wheel: f32,
    /// Layers panel open (Windows 11 style).
    pub show_layers: bool,
    pub layer_thumbs: Vec<TextureHandle>,
    pub thumbs_stale: bool,
    checker: Option<Pixmap>,
    /// Eraser writes transparency straight into the active layer (non-background layers).
    pub direct_erase: bool,
    /// Serial for the current dialog: each new dialog gets a fresh egui Area,
    /// so remembered sizes never leak between dialogs.
    pub dialog_nonce: u64,
    pub dialog_kind: Option<std::mem::Discriminant<crate::dialogs::Dialog>>,
    /// Open layer context menu: position and the frame it opened on.
    pub layer_menu: Option<Pos2>,
    pub layer_menu_frame: Option<u64>,
}

impl PaintApp {
    pub fn new(cc: &eframe::CreationContext<'_>, startup: StartupOptions) -> Self {
        let style = startup
            .style
            .as_deref()
            .and_then(RibbonStyle::from_key)
            .or_else(|| crate::actions::load_settings().get("ribbon_style").and_then(|v| RibbonStyle::from_key(v)))
            .unwrap_or(RibbonStyle::Classic);
        let settings = crate::actions::load_settings();
        let theme_from_cli = startup.theme.as_deref().and_then(ThemeMode::from_key);
        let theme_mode = theme_from_cli.or_else(|| settings.get("theme").and_then(|v| ThemeMode::from_key(v))).unwrap_or(ThemeMode::System);
        theme::install_fonts(&cc.egui_ctx);
        egui_material_init(&cc.egui_ctx);
        let fonts = FontLib::load();
        let (w, h) = default_canvas_size(&cc.egui_ctx);
        let mut app = Self {
            layers: vec![Layer::background(w, h, Color32::WHITE)],
            active_layer: 0,
            doc: DocInfo { path: None, dirty: false, file_size: None, saved_at: None },
            history: History::new(),
            tool: Tool::Pencil,
            prev_tool: Tool::Pencil,
            brush: BrushKind::Brush,
            shape: ShapeKind::Oval,
            outline_on: true,
            fill_on: false,
            size_idx: [0, 1, 0, 1],
            color1: Color32::BLACK,
            color2: Color32::WHITE,
            active_color: 1,
            custom_colors: [None; 10],
            dialog_custom: [Color32::WHITE; 16],
            transparent_sel: false,
            text: TextSettings { face: fonts.default_face, size_pt: 11.0, bold: false, italic: false, underline: false, strike: false, opaque: false },
            zoom: 1.0,
            show_grid: false,
            show_rulers: false,
            show_status: true,
            show_thumbnail: false,
            fullscreen_view: false,
            tab: Tab::Home,
            drag: Drag::None,
            drag_btn: Btn::Left,
            drag_cur: Pos2::ZERO,
            layer: None,
            layer_base: None,
            layer_opacity: 1.0,
            stroke: None,
            last_pt: Pos2::ZERO,
            selection: None,
            marquee: None,
            lasso: Vec::new(),
            active_shape: None,
            curve: None,
            poly: None,
            text_box: None,
            canvas_resize: None,
            texture: None,
            display_dirty: true,
            icons: Icons::default(),
            fonts,
            brush_previews: Vec::new(),
            brush_previews_dark: Vec::new(),
            dialog: None,
            clipboard: arboard::Clipboard::new().ok(),
            internal_clip: None,
            cursor_px: None,
            pending_scroll: None,
            scroll_offset: Vec2::ZERO,
            viewport: egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0)),
            recent: crate::actions::load_recent(),
            title: String::new(),
            allow_close: false,
            startup: startup.clone(),
            frame_no: 0,
            cmd_seen: -10.0,
            last_paste: -10.0,
            rng: Rng::new(0x5EED),
            file_menu_open: false,
            file_menu_saveas: false,
            context_menu: None,
            context_menu_frame: None,
            popup: None,
            zoom_accum: 1.0,
            input_blocked: false,
            caret_epoch: 0.0,
            now: 0.0,
            script: startup.script.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).map(|s| crate::script::Script::parse(&s)),
            script_done_frame: None,
            style,
            w11_menu: None,
            w11_sub: None,
            theme_mode,
            theme_override: theme_from_cli.is_some(),
            applied_look: None,
            custom_size: [None; 4],
            opacity: 1.0,
            view_origin: Pos2::ZERO,
            center_offset: Vec2::ZERO,
            overlay_rects: Vec::new(),
            overlay_rects_prev: Vec::new(),
            slider_wheel: 0.0,
            show_layers: false,
            layer_thumbs: Vec::new(),
            thumbs_stale: true,
            checker: None,
            direct_erase: false,
            dialog_nonce: 0,
            dialog_kind: None,
            layer_menu: None,
            layer_menu_frame: None,
        };
        let dark_now = app.wants_dark(&cc.egui_ctx);
        theme::apply_style(&cc.egui_ctx, app.style, dark_now);
        app.applied_look = Some((app.style, dark_now));
        // brush previews for the Brushes menu
        for kind in BrushKind::ALL {
            let pm = crate::brush::preview(kind, 96, 64);
            app.brush_previews.push(cc.egui_ctx.load_texture(format!("brush-{kind:?}"), raster::to_color_image(&pm), TextureOptions::LINEAR));
            let pm = crate::brush::preview_colored(kind, 96, 64, Color32::from_rgb(235, 235, 235));
            app.brush_previews_dark.push(cc.egui_ctx.load_texture(format!("brush-dark-{kind:?}"), raster::to_color_image(&pm), TextureOptions::LINEAR));
        }
        if let Some(p) = startup.open {
            app.open_path(&p);
        }
        if startup.demo {
            crate::actions::draw_demo(&mut app);
        }
        if let Some(m) = &startup.menu {
            match m.as_str() {
                "view" => app.tab = Tab::View,
                "w11file" => app.w11_menu = Some((crate::modern::W11Menu::File, egui::Rect::from_min_size(egui::pos2(6.0, 5.0), egui::vec2(42.0, 30.0)))),
                "w11edit" => app.w11_menu = Some((crate::modern::W11Menu::Edit, egui::Rect::from_min_size(egui::pos2(50.0, 5.0), egui::vec2(43.0, 30.0)))),
                "w11view" => app.w11_menu = Some((crate::modern::W11Menu::View, egui::Rect::from_min_size(egui::pos2(95.0, 5.0), egui::vec2(46.0, 30.0)))),
                "layers" => app.show_layers = true,
                "w11saveas" => {
                    app.w11_menu = Some((crate::modern::W11Menu::File, egui::Rect::from_min_size(egui::pos2(6.0, 5.0), egui::vec2(42.0, 30.0))));
                    app.w11_sub = Some((crate::modern::W11Sub::SaveAs, 180.0));
                }
                "file" => app.file_menu_open = true,
                "colors" => app.dialog = Some(Dialog::edit_colors(app.color1, 1, &app.dialog_custom)),
                "print" => app.dialog = Some(Dialog::print(&app)),
                "resize" => app.dialog = Some(Dialog::resize(&app)),
                "properties" => app.dialog = Some(Dialog::properties(&app)),
                "about" => app.dialog = Some(Dialog::About),
                "unsaved" => app.dialog = Some(Dialog::Unsaved(PendingAction::New)),
                "text" => {
                    app.tool = Tool::Text;
                    app.text_box = Some(TextBox { rect: IRect::new(420, 360, 220, 30), text: "Hello, Paint!".into(), caret: 13 });
                    app.tab = Tab::Text;
                }
                _ => {}
            }
        }
        app
    }

    // ----- tool settings -------------------------------------------------

    /// Which size slot applies to the current tool (if any).
    pub fn size_slot(&self) -> Option<usize> {
        match self.tool {
            Tool::Pencil => Some(0),
            Tool::Brush => Some(1),
            Tool::Shape => Some(2),
            Tool::Eraser => Some(3),
            _ if self.active_shape.is_some() => Some(2),
            _ => None,
        }
    }

    pub fn size_options(&self, slot: usize) -> [f32; 4] {
        match slot {
            0 => [1.0, 2.0, 3.0, 4.0],
            1 => self.brush.sizes(),
            2 => [1.0, 3.0, 5.0, 8.0],
            _ => [4.0, 6.0, 8.0, 10.0],
        }
    }

    pub fn tool_size(&self, slot: usize) -> f32 {
        if let Some(s) = self.custom_size[slot] {
            return s;
        }
        self.size_options(slot)[self.size_idx[slot].min(3)]
    }

    /// Range of the Windows 11 size slider for a size slot.
    /// Range of the Windows 11 size slider: every tool goes from 1px to 212px.
    pub fn size_range(&self, _slot: usize) -> (f32, f32) {
        (1.0, MAX_TOOL_SIZE)
    }

    /// Opacity used for pencil, brushes and shapes.
    pub fn draw_opacity(&self) -> f32 {
        if self.style == RibbonStyle::Modern {
            self.opacity.clamp(0.01, 1.0)
        } else {
            1.0
        }
    }

    pub fn margin(&self) -> f32 {
        if self.style == RibbonStyle::Modern {
            W11_CANVAS_MARGIN
        } else {
            CANVAS_MARGIN
        }
    }

    pub fn canvas_size_for_status(&self) -> (u32, u32) {
        match (self.canvas_resize, &self.drag) {
            (Some(r), Drag::CanvasResize { .. }) => (r.w as u32, r.h as u32),
            _ => (self.img_w(), self.img_h()),
        }
    }

    /// Whether the Windows 11 look should currently be dark.
    pub fn wants_dark(&self, ctx: &Context) -> bool {
        match self.theme_mode {
            ThemeMode::Light => false,
            ThemeMode::Dark => true,
            ThemeMode::System => ctx.system_theme() == Some(egui::Theme::Dark),
        }
    }

    pub fn set_theme_mode(&mut self, mode: ThemeMode) {
        self.theme_mode = mode;
        if !self.theme_override {
            crate::actions::save_setting("theme", mode.key());
        }
        self.theme_override = false;
        self.applied_look = None;
        self.display_dirty = true;
    }

    /// Keep egui visuals and the native title bar in sync with style + theme.
    fn sync_look(&mut self, ctx: &Context) {
        let dark = self.style == RibbonStyle::Modern && self.wants_dark(ctx);
        let look = (self.style, dark);
        if self.applied_look != Some(look) {
            theme::apply_style(ctx, self.style, dark);
            let native = match (self.style, self.theme_mode) {
                (RibbonStyle::Classic, _) => egui::SystemTheme::Light,
                (RibbonStyle::Modern, ThemeMode::System) => egui::SystemTheme::SystemDefault,
                (RibbonStyle::Modern, ThemeMode::Light) => egui::SystemTheme::Light,
                (RibbonStyle::Modern, ThemeMode::Dark) => egui::SystemTheme::Dark,
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::SetTheme(native));
            self.applied_look = Some(look);
            self.display_dirty = true;
        }
    }

    pub fn text_style(&self) -> TextStyle {
        TextStyle {
            face: self.text.face,
            size_px: pt_to_px(self.text.size_pt),
            bold: self.text.bold,
            italic: self.text.italic,
            underline: self.text.underline,
            strike: self.text.strike,
        }
    }

    pub fn shape_style(&self, swap: bool, open: bool) -> ShapeStyle {
        let (c_out, c_fill) = if swap { (self.color2, self.color1) } else { (self.color1, self.color2) };
        ShapeStyle {
            outline: if self.outline_on || open { Some(c_out) } else { None },
            fill: if self.fill_on && !open { Some(c_fill) } else { None },
            width: self.tool_size(2),
            open,
        }
    }

    pub fn set_tool(&mut self, t: Tool) {
        if t == self.tool {
            return;
        }
        self.commit_all();
        if t == Tool::Picker {
            self.prev_tool = self.tool;
        }
        self.tool = t;
        self.display_dirty = true;
    }

    pub fn set_color(&mut self, slot: u8, c: Color32) {
        if slot == 1 {
            self.color1 = c;
        } else {
            self.color2 = c;
        }
        self.display_dirty = true;
    }

    // ----- canvas geometry ----------------------------------------------

    pub fn img_w(&self) -> u32 {
        self.layers[0].pix.width()
    }

    pub fn img_h(&self) -> u32 {
        self.layers[0].pix.height()
    }

    /// The pixmap of the active layer.
    pub fn act(&self) -> &Pixmap {
        &self.layers[self.active_layer].pix
    }

    pub fn act_mut(&mut self) -> &mut Pixmap {
        let i = self.active_layer;
        &mut self.layers[i].pix
    }

    /// What erased / cut areas become on the active layer.
    pub fn hole(&self) -> [u8; 4] {
        if self.active_layer == 0 {
            rgba(self.color2)
        } else {
            [0, 0, 0, 0]
        }
    }

    pub fn snapshot(&self) -> Snapshot {
        Snapshot { layers: self.layers.clone(), active: self.active_layer }
    }

    pub fn restore(&mut self, s: Snapshot) {
        self.layers = s.layers;
        self.active_layer = s.active.min(self.layers.len() - 1);
        self.doc.dirty = true;
        self.display_dirty = true;
        self.thumbs_stale = true;
    }

    /// All visible layers flattened onto white (what gets saved / picked from).
    pub fn composite_flat(&self) -> Pixmap {
        let mut d = raster::new_canvas(self.img_w(), self.img_h(), Color32::WHITE);
        for l in &self.layers {
            if l.visible {
                raster::draw_over(&mut d, &l.pix, 0, 0);
            }
        }
        d
    }

    pub fn image_irect(&self) -> IRect {
        IRect::new(0, 0, self.img_w() as i32, self.img_h() as i32)
    }

    pub fn set_zoom(&mut self, z: f32, anchor: Option<(Pos2, Vec2)>) {
        let z = z.clamp(ZOOM_LEVELS[0], ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);
        let (c, s) = anchor.unwrap_or_else(|| {
            let half = self.viewport.size() / 2.0;
            let c = ((self.viewport.center() - self.view_origin) / self.zoom).to_pos2();
            (c, half)
        });
        self.zoom = z;
        let off = Vec2::splat(self.margin()) + c.to_vec2() * z - s;
        self.pending_scroll = Some(off.max(Vec2::ZERO));
    }

    pub fn zoom_to_fit(&mut self) {
        let avail = self.viewport.size() - Vec2::splat(2.0 * self.margin() + 12.0);
        let z = (avail.x / self.img_w() as f32).min(avail.y / self.img_h() as f32);
        self.zoom = z.clamp(ZOOM_LEVELS[0], ZOOM_LEVELS[ZOOM_LEVELS.len() - 1]);
        self.pending_scroll = Some(Vec2::ZERO);
    }

    /// Switch between the classic ribbon and the Windows 11 toolbar.
    pub fn toggle_style(&mut self, ctx: &Context) {
        self.style = match self.style {
            RibbonStyle::Classic => RibbonStyle::Modern,
            RibbonStyle::Modern => RibbonStyle::Classic,
        };
        crate::actions::save_setting("ribbon_style", self.style.key());
        self.popup = None;
        self.w11_menu = None;
        self.w11_sub = None;
        self.file_menu_open = false;
        if self.tab == Tab::View {
            self.tab = Tab::Home;
        }
        self.pending_scroll = Some(Vec2::ZERO);
        self.sync_look(ctx);
    }

    pub fn zoom_step(&mut self, dir: i32, anchor: Option<(Pos2, Vec2)>) {
        let z = if dir > 0 {
            ZOOM_LEVELS.iter().copied().find(|l| *l > self.zoom + 1e-4).unwrap_or(8.0)
        } else {
            ZOOM_LEVELS.iter().rev().copied().find(|l| *l < self.zoom - 1e-4).unwrap_or(0.125)
        };
        self.set_zoom(z, anchor);
    }

    // ----- display ---------------------------------------------------------

    pub fn shape_path(kind: ShapeKind, a: Pos2, b: Pos2, width: f32) -> shapes::PathData {
        let off = if (width.round() as i32) % 2 == 1 { 0.5 } else { 0.0 };
        let pa = (a.x + off, a.y + off);
        let pb = (b.x + off, b.y + off);
        if kind == ShapeKind::Line {
            shapes::line_path(pa, pb)
        } else {
            shapes::box_path(kind, pa, pb)
        }
    }

    fn curve_path(cv: &CurveState, width: f32) -> shapes::PathData {
        let off = if (width.round() as i32) % 2 == 1 { 0.5 } else { 0.0 };
        let f = |p: Pos2| (p.x + off, p.y + off);
        shapes::curve_path(f(cv.p0), f(cv.c1), f(cv.c2), f(cv.p3))
    }

    fn poly_path(pg: &PolyState, width: f32, closed: bool) -> shapes::PathData {
        let off = if (width.round() as i32) % 2 == 1 { 0.5 } else { 0.0 };
        let pts: Vec<(f32, f32)> = pg.pts.iter().map(|p| (p.x + off, p.y + off)).collect();
        shapes::polyline_path(&pts, closed)
    }

    /// Render a shape, honouring the drawing opacity (so outline and fill don't double up).
    fn render_shape(&self, target: &mut Pixmap, path: &[shapes::Seg], st: ShapeStyle) {
        let op = self.draw_opacity();
        if op >= 0.999 {
            shapes::render(target, path, st);
            return;
        }
        if let Some(mut layer) = Pixmap::new(target.width(), target.height()) {
            shapes::render(&mut layer, path, st);
            let paint = PixmapPaint { opacity: op, ..Default::default() };
            target.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
        }
    }

    pub fn render_active_shape(&self, target: &mut Pixmap) {
        if let Some(sh) = &self.active_shape {
            let open = sh.kind == ShapeKind::Line;
            let st = self.shape_style(sh.swap, open);
            self.render_shape(target, &Self::shape_path(sh.kind, sh.a, sh.b, st.width), st);
        }
        if let Some(cv) = &self.curve {
            let st = self.shape_style(cv.swap, true);
            self.render_shape(target, &Self::curve_path(cv, st.width), st);
        }
    }

    fn render_poly(&self, target: &mut Pixmap, closed: bool) {
        if let Some(pg) = &self.poly {
            let st = self.shape_style(pg.swap, !closed);
            let st = if closed { st } else { ShapeStyle { fill: None, outline: Some(if pg.swap { self.color2 } else { self.color1 }), ..st } };
            self.render_shape(target, &Self::poly_path(pg, st.width, closed), st);
        }
    }

    pub fn floating_pixels(&self, content: &Pixmap, rect: IRect) -> Pixmap {
        let mut px = raster::scale(content, rect.w.max(1) as u32, rect.h.max(1) as u32, false);
        if self.transparent_sel {
            raster::key_out(&mut px, self.color2);
        }
        px
    }

    pub fn draw_text_box(&self, target: &mut Pixmap, tb: &TextBox) {
        let r = tb.rect;
        if self.text.opaque {
            raster::fill_rect(target, r, rgba(self.color2));
        }
        let st = self.text_style();
        self.fonts.render(target, &st, &tb.text, (r.x as f32 + TEXT_PAD, r.y as f32 + TEXT_PAD), r.w as f32 - 2.0 * TEXT_PAD, self.color1, r);
    }

    pub fn rebuild_display(&mut self, ctx: &Context) {
        self.display_dirty = false;
        self.thumbs_stale = true;
        let (w, h) = (self.img_w(), self.img_h());
        if self.checker.as_ref().map(|c| (c.width(), c.height())) != Some((w, h)) {
            self.checker = Some(raster::checkerboard(w, h));
        }
        let mut d = self.checker.as_ref().unwrap().clone();
        for i in 0..self.layers.len() {
            if !self.layers[i].visible {
                continue;
            }
            if i == self.active_layer {
                // in-progress strokes, shapes, the floating selection and the
                // text box belong to the active layer's depth
                let mut top = self.layers[i].pix.clone();
                if let Some(layer) = &self.layer {
                    let paint = PixmapPaint { opacity: self.layer_opacity, ..Default::default() };
                    top.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
                }
                self.render_active_shape(&mut top);
                self.render_poly(&mut top, false);
                if let Some(sel) = &self.selection {
                    if let Some(c) = &sel.content {
                        let px = self.floating_pixels(c, sel.rect);
                        raster::draw_over(&mut top, &px, sel.rect.x, sel.rect.y);
                    }
                }
                if let Some(tb) = &self.text_box {
                    self.draw_text_box(&mut top, tb);
                }
                raster::draw_over(&mut d, &top, 0, 0);
            } else {
                raster::draw_over(&mut d, &self.layers[i].pix, 0, 0);
            }
        }
        let img = raster::to_color_image(&d);
        let opts = TextureOptions { magnification: egui::TextureFilter::Nearest, minification: egui::TextureFilter::Linear, ..TextureOptions::NEAREST };
        match &mut self.texture {
            Some(t) => t.set(img, opts),
            None => self.texture = Some(ctx.load_texture("canvas", img, opts)),
        }
    }

    // ----- committing in-progress work ----------------------------------

    pub fn push_undo(&mut self) {
        self.history.push(self.snapshot());
        self.doc.dirty = true;
        self.display_dirty = true;
    }

    pub fn commit_shape(&mut self) {
        if let Some(sh) = self.active_shape.take() {
            self.push_undo();
            let open = sh.kind == ShapeKind::Line;
            let st = self.shape_style(sh.swap, open);
            let path = Self::shape_path(sh.kind, sh.a, sh.b, st.width);
            let mut img = std::mem::replace(self.act_mut(), Pixmap::new(1, 1).unwrap());
            self.render_shape(&mut img, &path, st);
            *self.act_mut() = img;
        }
        if let Some(cv) = self.curve.take() {
            self.push_undo();
            let st = self.shape_style(cv.swap, true);
            let mut img = std::mem::replace(self.act_mut(), Pixmap::new(1, 1).unwrap());
            self.render_shape(&mut img, &Self::curve_path(&cv, st.width), st);
            *self.act_mut() = img;
        }
        if self.poly.as_ref().is_some_and(|p| p.pts.len() >= 2) {
            self.push_undo();
            let mut img = std::mem::replace(self.act_mut(), Pixmap::new(1, 1).unwrap());
            self.render_poly(&mut img, true);
            *self.act_mut() = img;
        }
        self.poly = None;
        self.display_dirty = true;
    }

    pub fn commit_text(&mut self) {
        if let Some(tb) = self.text_box.take() {
            if !tb.text.trim().is_empty() {
                self.push_undo();
                let mut img = std::mem::replace(self.act_mut(), Pixmap::new(1, 1).unwrap());
                self.draw_text_box(&mut img, &tb);
                *self.act_mut() = img;
            }
            if self.tab == Tab::Text {
                self.tab = Tab::Home;
            }
            self.display_dirty = true;
        }
    }

    pub fn lift_selection(&mut self) {
        let Some(sel) = &self.selection else { return };
        if sel.content.is_some() {
            return;
        }
        let src = sel.src;
        let mask = sel.mask.clone();
        self.push_undo();
        let mut content = raster::crop(self.act(), src);
        let c2 = self.hole();
        match &mask {
            Some(m) => {
                raster::apply_mask(&mut content, m);
                raster::fill_masked(self.act_mut(), src, m, c2);
            }
            None => raster::fill_rect(self.act_mut(), src, c2),
        }
        if let Some(sel) = self.selection.as_mut() {
            sel.content = Some(content);
        }
        self.display_dirty = true;
    }

    pub fn commit_selection(&mut self) {
        if let Some(sel) = self.selection.take() {
            if let Some(content) = &sel.content {
                let px = self.floating_pixels(content, sel.rect);
                raster::draw_over(self.act_mut(), &px, sel.rect.x, sel.rect.y);
                self.doc.dirty = true;
            }
            self.display_dirty = true;
        }
    }

    pub fn commit_all(&mut self) {
        self.commit_text();
        self.commit_shape();
        self.commit_selection();
    }

    /// Abort any pointer drag in progress, finishing strokes.
    pub fn finish_drag(&mut self) {
        if matches!(self.drag, Drag::Stroke) {
            self.commit_stroke();
        }
        self.drag = Drag::None;
        self.marquee = None;
        self.lasso.clear();
        self.canvas_resize = None;
    }

    pub fn commit_stroke(&mut self) {
        if let Some(base) = self.layer_base.take() {
            // undo state: the stack as it was before the stroke started
            let mut snap = self.snapshot();
            snap.layers[self.active_layer].pix = base;
            self.history.push(snap);
            self.doc.dirty = true;
            if let Some(layer) = self.layer.take() {
                let paint = PixmapPaint { opacity: self.layer_opacity, ..Default::default() };
                self.act_mut().draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
            }
        }
        self.layer = None;
        self.direct_erase = false;
        self.stroke = None;
        self.display_dirty = true;
    }

    pub fn undo(&mut self) {
        self.finish_drag();
        self.commit_all();
        if let Some(prev) = self.history.undo(self.snapshot()) {
            self.restore(prev);
        }
    }

    pub fn redo(&mut self) {
        self.finish_drag();
        self.commit_all();
        if let Some(next) = self.history.redo(self.snapshot()) {
            self.restore(next);
        }
    }

    // ----- text box editing ------------------------------------------------

    pub(crate) fn text_layout(&self, tb: &TextBox) -> crate::text::Layout {
        self.fonts.layout(&self.text_style(), &tb.text, tb.rect.w as f32 - 2.0 * TEXT_PAD)
    }

    pub fn fit_text_box(&mut self) {
        let Some(tb) = &self.text_box else { return };
        let lay = self.text_layout(tb);
        let need = (lay.height + 2.0 * TEXT_PAD).ceil() as i32;
        if let Some(tb) = self.text_box.as_mut() {
            if need > tb.rect.h {
                tb.rect.h = need;
            }
        }
        self.display_dirty = true;
    }

    fn text_insert(&mut self, s: &str) {
        let Some(tb) = self.text_box.as_mut() else { return };
        let byte = tb.text.char_indices().nth(tb.caret).map(|(b, _)| b).unwrap_or(tb.text.len());
        let s: String = s.chars().filter(|c| *c == '\n' || !c.is_control()).collect();
        tb.text.insert_str(byte, &s);
        tb.caret += s.chars().count();
        self.caret_epoch = self.now;
        self.fit_text_box();
    }

    fn text_key(&mut self, key: Key) {
        let lay = match &self.text_box {
            Some(tb) => self.text_layout(tb),
            None => return,
        };
        let Some(tb) = self.text_box.as_mut() else { return };
        let n = tb.text.chars().count();
        match key {
            Key::Backspace if tb.caret > 0 => {
                let b = tb.text.char_indices().nth(tb.caret - 1).map(|(b, _)| b).unwrap();
                tb.text.remove(b);
                tb.caret -= 1;
            }
            Key::Delete if tb.caret < n => {
                let b = tb.text.char_indices().nth(tb.caret).map(|(b, _)| b).unwrap();
                tb.text.remove(b);
            }
            Key::ArrowLeft => tb.caret = tb.caret.saturating_sub(1),
            Key::ArrowRight => tb.caret = (tb.caret + 1).min(n),
            Key::Home => {
                let (li, _) = lay.caret_pos(tb.caret);
                tb.caret = lay.lines[li].start;
            }
            Key::End => {
                let (li, _) = lay.caret_pos(tb.caret);
                tb.caret = lay.lines[li].end;
            }
            Key::ArrowUp | Key::ArrowDown => {
                let (li, x) = lay.caret_pos(tb.caret);
                let target = if key == Key::ArrowUp { li.checked_sub(1) } else { Some(li + 1).filter(|l| *l < lay.lines.len()) };
                if let Some(t) = target {
                    tb.caret = lay.hit(x, lay.lines[t].y + lay.line_h / 2.0);
                }
            }
            Key::Enter => {
                self.text_insert("\n");
                return;
            }
            _ => return,
        }
        self.caret_epoch = self.now;
        self.display_dirty = true;
    }

    // ----- frame -------------------------------------------------------------

    fn handle_keys(&mut self, ctx: &Context) {
        let now = ctx.input(|i| i.time);
        if ctx.input(|i| i.modifiers.command) {
            self.cmd_seen = now;
        }
        let text_active = self.text_box.is_some();
        let sc = |m: Modifiers, k: Key| ctx.input_mut(|i| i.consume_shortcut(&KeyboardShortcut::new(m, k)));
        let cmd = Modifiers::COMMAND;
        let cs = Modifiers::COMMAND | Modifiers::SHIFT;

        if text_active {
            let mut changed = false;
            if sc(cmd, Key::B) {
                self.text.bold = !self.text.bold;
                changed = true;
            }
            if sc(cmd, Key::I) {
                self.text.italic = !self.text.italic;
                changed = true;
            }
            if sc(cmd, Key::U) {
                self.text.underline = !self.text.underline;
                changed = true;
            }
            if changed {
                self.fit_text_box();
            }
        }
        if sc(cs, Key::Z) || sc(cmd, Key::Y) {
            self.redo();
        }
        if sc(cmd, Key::Z) {
            self.undo();
        }
        if sc(cmd, Key::N) {
            self.request(ctx, PendingAction::New);
        }
        if sc(cmd, Key::O) {
            self.request(ctx, PendingAction::Open);
        }
        if sc(cs, Key::S) || sc(Modifiers::NONE, Key::F12) {
            self.save_as(None);
        }
        if sc(cmd, Key::S) {
            self.save();
        }
        if sc(cs, Key::X) {
            self.crop();
        }
        if sc(cs, Key::H) {
            let i = self.active_layer;
            self.toggle_layer_visible(i);
        }
        if sc(cs, Key::PageUp) {
            self.move_layer(true);
        }
        if sc(cs, Key::PageDown) {
            self.move_layer(false);
        }
        if sc(cs, Key::Delete) || sc(cs, Key::Backspace) {
            self.delete_layer();
        }
        if sc(cs, Key::I) {
            self.invert_colors();
        }
        if sc(cmd, Key::A) && !text_active {
            self.select_all();
        }
        if sc(cmd, Key::P) {
            self.dialog = Some(Dialog::print(self));
        }
        if sc(cmd, Key::E) {
            self.dialog = Some(Dialog::properties(self));
        }
        if sc(cmd, Key::W) {
            self.dialog = Some(Dialog::resize(self));
        }
        if sc(cmd, Key::R) {
            self.show_rulers = !self.show_rulers;
        }
        if sc(cmd, Key::G) {
            self.show_grid = !self.show_grid;
        }
        if sc(cmd, Key::PageUp) || sc(cmd, Key::Plus) || sc(cmd, Key::Equals) {
            self.zoom_step(1, None);
        }
        if sc(cmd, Key::PageDown) || sc(cmd, Key::Minus) {
            self.zoom_step(-1, None);
        }
        if sc(Modifiers::NONE, Key::F11) {
            self.toggle_fullscreen_view(ctx);
        }

        let events = ctx.input(|i| i.events.clone());
        for ev in events {
            match ev {
                Event::Copy if !text_active => self.copy(),
                Event::Cut if !text_active => self.cut(),
                Event::Paste(s) => {
                    if text_active {
                        self.text_insert(&s);
                    } else {
                        self.paste(ctx);
                        self.last_paste = now;
                    }
                }
                Event::Key { key: Key::V, pressed: false, modifiers, .. } if !text_active => {
                    // Cmd/Ctrl+V with an image-only clipboard produces no Paste event.
                    if (modifiers.command || now - self.cmd_seen < 0.6) && now - self.last_paste > 1.0 {
                        self.paste(ctx);
                        self.last_paste = now;
                    }
                }
                Event::Text(t) if text_active => self.text_insert(&t),
                Event::Key { key, pressed: true, modifiers, .. } => {
                    if text_active {
                        if key == Key::Escape {
                            self.commit_text();
                        } else if !modifiers.command {
                            self.text_key(key);
                        }
                        continue;
                    }
                    if modifiers.command || modifiers.alt {
                        continue;
                    }
                    match key {
                        Key::Delete | Key::Backspace => self.delete_selection(),
                        Key::Escape => {
                            if self.poly.is_some() || self.curve.is_some() || self.active_shape.is_some() {
                                self.commit_shape();
                            } else {
                                self.commit_selection();
                            }
                            self.finish_drag();
                        }
                        Key::Enter => self.commit_all(),
                        Key::ArrowLeft | Key::ArrowRight | Key::ArrowUp | Key::ArrowDown if self.selection.is_some() => {
                            let (dx, dy) = match key {
                                Key::ArrowLeft => (-1, 0),
                                Key::ArrowRight => (1, 0),
                                Key::ArrowUp => (0, -1),
                                _ => (0, 1),
                            };
                            self.lift_selection();
                            if let Some(sel) = self.selection.as_mut() {
                                sel.rect.x += dx;
                                sel.rect.y += dy;
                            }
                            self.display_dirty = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    fn handle_close(&mut self, ctx: &Context) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close {
            self.finish_drag();
            self.commit_all();
            if self.doc.dirty {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.dialog = Some(Dialog::Unsaved(PendingAction::Exit));
            }
        }
    }

    fn handle_dropped(&mut self, ctx: &Context) {
        let dropped = ctx.input(|i| i.raw.dropped_files.clone());
        if let Some(path) = dropped.into_iter().find_map(|f| f.path) {
            self.request(ctx, PendingAction::OpenPath(path));
        }
    }

    fn update_title(&mut self, ctx: &Context) {
        let t = format!("{} - Free Paint", self.doc.name());
        if t != self.title {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(t.clone()));
            self.title = t;
        }
    }

    pub fn toggle_fullscreen_view(&mut self, ctx: &Context) {
        self.fullscreen_view = !self.fullscreen_view;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(self.fullscreen_view));
    }

    fn fullscreen_ui(&mut self, ctx: &Context) {
        egui::CentralPanel::default().frame(Frame::NONE.fill(theme::workspace())).show(ctx, |ui| {
            if self.display_dirty {
                self.rebuild_display(ui.ctx());
            }
            let avail = ui.max_rect();
            let (w, h) = (self.img_w() as f32, self.img_h() as f32);
            let s = (avail.width() / w).min(avail.height() / h).min(1.0);
            let r = egui::Rect::from_center_size(avail.center(), egui::vec2(w * s, h * s));
            if let Some(t) = &self.texture {
                ui.painter().image(t.id(), r, egui::Rect::from_min_max(Pos2::ZERO, egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            let resp = ui.interact(avail, ui.id().with("fs"), egui::Sense::click());
            let any_key = ui.input(|i| i.events.iter().any(|e| matches!(e, Event::Key { pressed: true, .. })));
            if resp.clicked() || resp.secondary_clicked() || any_key {
                self.toggle_fullscreen_view(ui.ctx());
            }
        });
    }

    fn handle_screenshot(&mut self, ctx: &Context) {
        let Some(path) = self.startup.screenshot.clone() else { return };
        let script_running = self.script.as_ref().is_some_and(|s| !s.is_done());
        if script_running {
            ctx.request_repaint();
            return;
        }
        let ready_at = *self.script_done_frame.get_or_insert(self.frame_no.max(4)) + 8;
        if self.frame_no == ready_at {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        let shots: Vec<std::sync::Arc<egui::ColorImage>> = ctx.input(|i| {
            i.raw.events.iter().filter_map(|e| if let Event::Screenshot { image, .. } = e { Some(image.clone()) } else { None }).collect()
        });
        if let Some(dump) = &self.startup.dump {
            if !ctx.input(|i| i.raw.events.is_empty()) || self.frame_no == ready_at {
                let mut out = format!("layers={} active={} tool={:?} zoom={} canvas={}x{} dirty={}\n", self.layers.len(), self.active_layer, self.tool, self.zoom, self.img_w(), self.img_h(), self.doc.dirty);
                for (i, l) in self.layers.iter().enumerate() {
                    let mut colored = 0usize;
                    let mut transparent = 0usize;
                    for px in l.pix.data().chunks_exact(4) {
                        if px[3] == 0 {
                            transparent += 1;
                        } else if !(px[0] == px[1] && px[1] == px[2]) {
                            colored += 1;
                        }
                    }
                    out.push_str(&format!("layer{i} visible={} colored={colored} transparent={transparent}\n", l.visible));
                }
                let _ = std::fs::write(dump, out);
            }
        }
        if let Some(img) = shots.first() {
            let [w, h] = img.size;
            let bytes: Vec<u8> = img.pixels.iter().flat_map(|c| c.to_array()).collect();
            if let Some(buf) = image::RgbaImage::from_raw(w as u32, h as u32, bytes) {
                let _ = buf.save(&path);
            }
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.frame_no <= ready_at + 2 {
            ctx.request_repaint();
        }
    }
}

impl eframe::App for PaintApp {
    fn raw_input_hook(&mut self, _ctx: &Context, raw_input: &mut egui::RawInput) {
        if let Some(script) = self.script.as_mut() {
            script.apply(raw_input);
        }
    }

    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.frame_no += 1;
        self.now = ctx.input(|i| i.time);
        self.sync_look(ctx);
        self.overlay_rects_prev = std::mem::take(&mut self.overlay_rects);
        self.input_blocked = self.popup.is_some() || self.file_menu_open || self.w11_menu.is_some() || self.context_menu.is_some() || self.layer_menu.is_some() || self.dialog.is_some();
        self.handle_close(ctx);
        self.handle_dropped(ctx);
        let focused = ctx.memory(|m| m.focused().is_some());
        if self.dialog.is_none() && !focused {
            self.handle_keys(ctx);
        }
        self.update_title(ctx);

        if self.fullscreen_view {
            self.fullscreen_ui(ctx);
        } else if self.style == RibbonStyle::Modern {
            use crate::modern;
            egui::TopBottomPanel::top("w11-header")
                .exact_height(modern::HEADER_H)
                .frame(Frame::NONE.fill(theme::w().header_bg))
                .show_separator_line(false)
                .show(ctx, |ui| self.w11_header_ui(ui));
            egui::TopBottomPanel::top("w11-toolbar")
                .exact_height(modern::TOOLBAR_H)
                .frame(Frame::NONE.fill(theme::w().toolbar_bg))
                .show_separator_line(false)
                .show(ctx, |ui| self.w11_toolbar_ui(ui));
            if self.show_status {
                egui::TopBottomPanel::bottom("w11-status")
                    .exact_height(modern::STATUS_H)
                    .frame(Frame::NONE.fill(theme::w().status_bg))
                    .show_separator_line(false)
                    .show(ctx, |ui| self.w11_status_ui(ui));
            }
            if self.show_layers {
                egui::SidePanel::right("w11-layers")
                    .exact_width(210.0)
                    .resizable(false)
                    .frame(Frame::NONE.fill(theme::w().toolbar_bg))
                    .show_separator_line(false)
                    .show(ctx, |ui| self.w11_layers_ui(ui));
            }
            egui::CentralPanel::default().frame(Frame::NONE.fill(theme::w().workspace)).show(ctx, |ui| self.canvas_ui(ui));
            self.w11_menus_ui(ctx);
            self.w11_layer_menu_ui(ctx);
            self.context_menu_ui(ctx);
            self.thumbnail_ui(ctx);
        } else {
            egui::TopBottomPanel::top("tabs")
                .exact_height(TAB_H)
                .frame(Frame::NONE.fill(theme::TAB_BAR))
                .show_separator_line(false)
                .show(ctx, |ui| self.tabs_ui(ui));
            egui::TopBottomPanel::top("ribbon")
                .exact_height(RIBBON_H)
                .frame(Frame::NONE.fill(theme::RIBBON_BG))
                .show_separator_line(false)
                .show(ctx, |ui| self.ribbon_ui(ui));
            if self.show_status {
                egui::TopBottomPanel::bottom("status")
                    .exact_height(STATUS_H)
                    .frame(Frame::NONE.fill(theme::STATUS_BG))
                    .show_separator_line(false)
                    .show(ctx, |ui| self.status_ui(ui));
            }
            if self.show_layers {
                egui::SidePanel::right("classic-layers-panel")
                    .exact_width(210.0)
                    .resizable(false)
                    .frame(Frame::NONE.fill(theme::RIBBON_BG))
                    .show_separator_line(false)
                    .show(ctx, |ui| self.w11_layers_ui(ui));
            }
            egui::CentralPanel::default().frame(Frame::NONE.fill(theme::WORKSPACE)).show(ctx, |ui| self.canvas_ui(ui));
            self.file_menu_ui(ctx);
            self.w11_layer_menu_ui(ctx);
            self.context_menu_ui(ctx);
            self.thumbnail_ui(ctx);
        }
        self.dialogs_ui(ctx);
        if self.display_dirty {
            ctx.request_repaint();
        }
        self.handle_screenshot(ctx);
    }
}

pub const DEFAULT_CANVAS: (u32, u32) = (960, 600);

fn default_canvas_size(_ctx: &Context) -> (u32, u32) {
    DEFAULT_CANVAS
}

fn egui_material_init(ctx: &Context) {
    // Keep egui from treating Cmd+scroll as UI zoom; we handle canvas zoom ourselves.
    ctx.options_mut(|o| {
        o.zoom_with_keyboard = false;
    });
}
