//! Ribbon / menu icons. Every icon is an original SVG drawing that gets
//! rasterized with resvg at the exact pixel size needed and cached.

use std::collections::HashMap;

use egui::{Color32, Context, Painter, Rect, TextureHandle, TextureOptions};
use resvg::usvg;
use tiny_skia::{Pixmap, Transform};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Icon {
    Paste,
    Cut,
    Copy,
    Select,
    Crop,
    Resize,
    Rotate,
    Pencil,
    Fill,
    Text,
    Eraser,
    Picker,
    Magnifier,
    Brushes,
    Outline,
    ShapeFill,
    Size,
    EditColors,
    Save,
    Undo,
    Redo,
    New,
    Open,
    SaveAs,
    Print,
    Properties,
    About,
    Exit,
    Picture,
    ZoomIn,
    ZoomOut,
    Zoom100,
    FullScreen,
    Thumbnail,
    RectSelect,
    FreeSelect,
    SelectAll,
    InvertSelection,
    Delete,
    TransparentSelection,
    RotateRight,
    RotateLeft,
    Rotate180,
    FlipVertical,
    FlipHorizontal,
    StatusCursor,
    StatusSelection,
    StatusCanvas,
    StatusDisk,
    Help,
    Opaque,
    Transparent,
    InvertColor,
    Recent,
    RibbonStyle,
    Settings,
    Opacity,
    Layers,
    Eye,
    EyeOff,
    AddLayer,
    Merge,
    MergeUp,
    App,
}

const PENCIL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M2 11 L11 2 L14 5 L5 14 Z" fill="#f6c343" stroke="#9a6b00" stroke-width="0.8" stroke-linejoin="round"/>
<path d="M3.2 12.2 L12.2 3.2" stroke="#fde49a" stroke-width="0.9"/>
<path d="M11 2 L12.2 0.8 A1.3 1.3 0 0 1 14 0.8 L15.2 2 A1.3 1.3 0 0 1 15.2 3.8 L14 5 Z" fill="#ea7aa0" stroke="#9c3550" stroke-width="0.8" stroke-linejoin="round"/>
<path d="M10.2 2.8 L13.2 5.8" stroke="#b9b9b9" stroke-width="1.4"/>
<path d="M0.8 15.2 L2 11 L5 14 Z" fill="#f3d9b1" stroke="#9a6b00" stroke-width="0.8" stroke-linejoin="round"/>
<path d="M0.6 15.4 L1.2 13.3 L2.7 14.8 Z" fill="#333"/>
</svg>"##;

const FILL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M6 1.5 C4 1.5 3 3 4 4.6" fill="none" stroke="#555" stroke-width="1"/>
<path d="M1.3 7.6 L7.4 1.5 L13.5 7.6 L7.4 13.7 Z" fill="#f2f2f2" stroke="#4a4a4a" stroke-linejoin="round"/>
<path d="M2.4 7.6 L13.5 7.6 L7.4 13.7 Z" fill="#cfcfcf"/>
<path d="M1.3 7.6 L7.4 1.5 L13.5 7.6 L7.4 13.7 Z" fill="none" stroke="#4a4a4a" stroke-linejoin="round"/>
<path d="M13.5 7.6 C15.3 9.2 15.8 11.6 15 13.8 C13.9 12.9 13.1 10.2 13.5 7.6 Z" fill="#1f7ae0" stroke="#145bb0" stroke-width="0.6"/>
<path d="M4.5 7.6 H13.5" stroke="#1f7ae0" stroke-width="1.6"/>
</svg>"##;

const TEXT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 14.5 L6.8 1.5 H9.2 L14.5 14.5 H12 L10.7 11.2 H5.3 L4 14.5 Z M6.1 9.1 H9.9 L8 4.3 Z" fill="#1f5fbf" fill-rule="evenodd"/>
</svg>"##;

const ERASER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 10 L8.5 3 L14.5 9 L9.5 14 H5.5 Z" fill="#fafafa" stroke="#555" stroke-linejoin="round"/>
<path d="M8.5 3 L14.5 9 L11.2 12.3 L5.2 6.3 Z" fill="#f08cb4" stroke="#555" stroke-linejoin="round"/>
<path d="M5.5 14.5 H15" stroke="#555" stroke-linecap="round"/>
</svg>"##;

const PICKER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.2 14.8 L1.8 12.4 L8.4 5.8 L10.2 7.6 L3.6 14.2 Z" fill="#dbeeff" stroke="#3b3b3b" stroke-width="0.9" stroke-linejoin="round"/>
<path d="M1.6 14.4 L2.3 12.6 L3.4 13.7 Z" fill="#1f7ae0"/>
<path d="M7.4 4.8 L11.2 8.6" stroke="#3b3b3b" stroke-width="1.8" stroke-linecap="round"/>
<path d="M9.5 6.5 L12.2 3.8" stroke="#3b3b3b" stroke-width="2.2"/>
<circle cx="12.6" cy="3.4" r="2.6" fill="#3b3b3b"/>
<circle cx="11.9" cy="2.7" r="0.8" fill="#8a8a8a"/>
</svg>"##;

const MAGNIFIER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M10 10 L14.6 14.6" stroke="#7a4a1e" stroke-width="2.8" stroke-linecap="round"/>
<circle cx="6.5" cy="6.5" r="4.7" fill="#dff0ff" stroke="#3b3b3b" stroke-width="1.5"/>
<path d="M4 5 A3 3 0 0 1 6 3.2" fill="none" stroke="#fff" stroke-width="1.2" stroke-linecap="round"/>
</svg>"##;

const PASTE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<defs><linearGradient id="b" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#d9a066"/><stop offset="1" stop-color="#b37939"/></linearGradient></defs>
<rect x="3.5" y="4.5" width="19" height="25" rx="2" fill="url(#b)" stroke="#7b4f22"/>
<rect x="6.5" y="7.5" width="13" height="19" fill="#fdfdfd" stroke="#a0a0a0"/>
<rect x="8.5" y="2.5" width="9" height="6" rx="1.5" fill="#e8e8e8" stroke="#5f5f5f"/>
<rect x="11" y="4" width="4" height="2" rx="1" fill="#8a8a8a"/>
<path d="M8.5 12.5 h9 M8.5 15.5 h5" stroke="#b8c8e0"/>
<rect x="14.5" y="14.5" width="15" height="15" fill="#fff" stroke="#3d6fb5"/>
<rect x="16" y="16" width="12" height="12" fill="#d9ecff"/>
<path d="M16 28 L20.5 21.5 L23.5 25 L25.5 23 L28 28 Z" fill="#4caf50"/>
<circle cx="25" cy="19" r="1.8" fill="#f5b800"/>
</svg>"##;

const CUT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M4.6 0.8 L10.2 10.4 M11.4 0.8 L5.8 10.4" stroke="#6a6a6a" stroke-width="1.5" stroke-linecap="round"/>
<circle cx="4.6" cy="12.6" r="2.3" fill="none" stroke="#1f5fbf" stroke-width="1.5"/>
<circle cx="11.4" cy="12.6" r="2.3" fill="none" stroke="#1f5fbf" stroke-width="1.5"/>
<circle cx="8" cy="6" r="0.9" fill="#444"/>
</svg>"##;

const COPY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 1.5 H7.5 L9.5 3.5 V11.5 H1.5 Z" fill="#fff" stroke="#6b6b6b" stroke-linejoin="round"/>
<path d="M6.5 4.5 H12.5 L14.5 6.5 V14.5 H6.5 Z" fill="#fff" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M8 8.5 H13 M8 10.5 H13 M8 12.5 H11.5" stroke="#9db7e0"/>
</svg>"##;

const SELECT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="3.5" y="5.5" width="25" height="19" fill="#ffffff" fill-opacity="0.7" stroke="#2f2f2f" stroke-dasharray="2 2"/>
<path d="M17 15 V29 L20.4 25.8 L23 31 L25 30 L22.5 25 L27 24.6 Z" fill="#fff" stroke="#222" stroke-width="1" stroke-linejoin="round"/>
</svg>"##;

const CROP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="5" y="5" width="6" height="6" fill="#a9cdf5"/>
<path d="M4.5 0.5 V11.5 H15.5 M0.5 4.5 H11.5 V15.5" fill="none" stroke="#333" stroke-width="1.2"/>
</svg>"##;

const RESIZE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="1.5" width="13" height="13" fill="none" stroke="#7a7a7a" stroke-dasharray="1.5 1.5"/>
<rect x="1.5" y="7.5" width="7" height="7" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M7.5 8.5 L13 3" stroke="#1f5fbf" stroke-width="1.4"/>
<path d="M9 2.5 H13.5 V7" fill="none" stroke="#1f5fbf" stroke-width="1.4"/>
</svg>"##;

const ROTATE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="5.5" y="5.5" width="9" height="9" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M2.2 9.5 A6.8 6.8 0 0 1 8.5 2.2" fill="none" stroke="#333" stroke-width="1.4"/>
<path d="M7.4 0 L11 2.2 L7.4 4.6 Z" fill="#333"/>
</svg>"##;

const BRUSHES: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M3 28 C8 22 11 27 16 23" fill="none" stroke="#1f5fbf" stroke-width="3" stroke-linecap="round"/>
<path d="M15.3 14.7 L26.2 3.8 A2.1 2.1 0 0 1 29.2 6.8 L18.3 17.7 Z" fill="#c67b35" stroke="#7b4f22" stroke-linejoin="round"/>
<path d="M12.6 13.4 L18.6 19.4 L16.8 21.2 L10.8 15.2 Z" fill="#c8c8c8" stroke="#666"/>
<path d="M10.8 15.2 L16.8 21.2 C14.4 24.4 10.6 25.6 6.9 25.1 C7.2 21.4 8.4 17.8 10.8 15.2 Z" fill="#1f5fbf" stroke="#173f7a" stroke-linejoin="round"/>
</svg>"##;

const OUTLINE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="4.5" width="10" height="10" fill="none" stroke="#1f5fbf" stroke-width="1.4"/>
<path d="M8 9.5 L13.8 3.7 L15.3 5.2 L9.5 11 L7.5 11.5 Z" fill="#f6c343" stroke="#9a6b00" stroke-width="0.7" stroke-linejoin="round"/>
</svg>"##;

const SHAPE_FILL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="4.5" width="10" height="10" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M8.3 5.6 L11.8 2.1 L15.3 5.6 L11.8 9.1 Z" fill="#f2f2f2" stroke="#4a4a4a" stroke-width="0.8" stroke-linejoin="round"/>
<path d="M15.3 5.6 C16 6.6 16 8 15.4 9 C14.9 8.2 14.8 6.8 15.3 5.6 Z" fill="#1f7ae0"/>
</svg>"##;

const SIZE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="3" y="5" width="26" height="1" fill="#222"/>
<rect x="3" y="10" width="26" height="2.5" fill="#222"/>
<rect x="3" y="16.5" width="26" height="4" fill="#222"/>
<rect x="3" y="24" width="26" height="6" fill="#222"/>
</svg>"##;

const EDIT_COLORS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<g transform="translate(14 15)">
<path d="M0 0 L0 -12 A12 12 0 0 1 10.39 -6 Z" fill="#ff3b30"/>
<path d="M0 0 L10.39 -6 A12 12 0 0 1 10.39 6 Z" fill="#ffcc00"/>
<path d="M0 0 L10.39 6 A12 12 0 0 1 0 12 Z" fill="#34c759"/>
<path d="M0 0 L0 12 A12 12 0 0 1 -10.39 6 Z" fill="#00c7be"/>
<path d="M0 0 L-10.39 6 A12 12 0 0 1 -10.39 -6 Z" fill="#007aff"/>
<path d="M0 0 L-10.39 -6 A12 12 0 0 1 0 -12 Z" fill="#af52de"/>
<circle r="4.5" fill="#fff"/>
<circle r="12" fill="none" stroke="#555" stroke-width="0.8"/>
</g>
<path d="M19.5 29.5 L23 21 L30 24.5 Z" fill="#f6c343" stroke="#9a6b00" stroke-width="0.8" stroke-linejoin="round"/>
<path d="M18.6 31.4 L19.5 29.5 L21.3 30.4 Z" fill="#333"/>
</svg>"##;

const SAVE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 1.5 H12.5 L14.5 3.5 V14.5 H1.5 Z" fill="#3c73c9" stroke="#1d4a8f" stroke-linejoin="round"/>
<rect x="4" y="1.5" width="7.5" height="5" fill="#e8eef8"/>
<rect x="8.6" y="2.4" width="1.7" height="3.1" fill="#3c73c9"/>
<rect x="3.5" y="9" width="9" height="5.5" fill="#ffffff"/>
<path d="M5 11 H11 M5 12.8 H11" stroke="#9db7e0" stroke-width="0.8"/>
</svg>"##;

const UNDO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M4.5 6.5 H10 A4 4 0 0 1 10 14.5 H6" fill="none" stroke="#1f5fbf" stroke-width="2" stroke-linecap="round"/>
<path d="M0.8 6.5 L5.6 2.2 V10.8 Z" fill="#1f5fbf"/>
</svg>"##;

const REDO: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<g transform="matrix(-1 0 0 1 16 0)">
<path d="M4.5 6.5 H10 A4 4 0 0 1 10 14.5 H6" fill="none" stroke="#1f5fbf" stroke-width="2" stroke-linecap="round"/>
<path d="M0.8 6.5 L5.6 2.2 V10.8 Z" fill="#1f5fbf"/>
</g>
</svg>"##;

const NEW: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M6.5 2.5 H19.5 L26.5 9.5 V29.5 H6.5 Z" fill="#fff" stroke="#6b6b6b" stroke-linejoin="round"/>
<path d="M19.5 2.5 V9.5 H26.5" fill="#e6e6e6" stroke="#6b6b6b" stroke-linejoin="round"/>
</svg>"##;

const OPEN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M2.5 6.5 H11 L13.5 9 H27.5 V27.5 H2.5 Z" fill="#dca52d" stroke="#a87914" stroke-linejoin="round"/>
<rect x="6.5" y="4.5" width="17" height="15" fill="#fff" stroke="#8a8a8a"/>
<path d="M9 8.5 H21 M9 11.5 H21" stroke="#b8c8e0"/>
<path d="M2.5 13.5 H30 L27.5 27.5 H2.5 Z" fill="#f6cf5d" stroke="#a87914" stroke-linejoin="round"/>
</svg>"##;

const SAVE_AS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M2.5 3.5 H22.5 L26.5 7.5 V27.5 H2.5 Z" fill="#3c73c9" stroke="#1d4a8f" stroke-linejoin="round"/>
<rect x="7" y="3.5" width="13" height="9" fill="#e8eef8"/>
<rect x="15.5" y="5" width="3" height="6" fill="#3c73c9"/>
<rect x="6.5" y="17" width="16" height="10.5" fill="#fff"/>
<path d="M17 28 L27.5 17.5 L30.5 20.5 L20 31 L16.5 31.5 Z" fill="#f6c343" stroke="#9a6b00" stroke-linejoin="round"/>
</svg>"##;

const PRINT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="8.5" y="3.5" width="15" height="10" fill="#fff" stroke="#6b6b6b"/>
<rect x="2.5" y="11.5" width="27" height="12" rx="2" fill="#8d97a5" stroke="#4b5563"/>
<rect x="8.5" y="19.5" width="15" height="9" fill="#fff" stroke="#6b6b6b"/>
<circle cx="25" cy="15.5" r="1.3" fill="#5ad65a"/>
</svg>"##;

const PROPERTIES: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M5.5 2.5 H19.5 L24.5 7.5 V29.5 H5.5 Z" fill="#fff" stroke="#6b6b6b" stroke-linejoin="round"/>
<path d="M9 11 H21 M9 15 H21 M9 19 H16" stroke="#9db7e0" stroke-width="1.6"/>
<path d="M15.5 24 L19.5 28 L29.5 17.5" fill="none" stroke="#2e9d3a" stroke-width="3.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"##;

const ABOUT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<circle cx="16" cy="16" r="13.5" fill="#1f6fd0" stroke="#174f96"/>
<circle cx="16" cy="9.5" r="2.2" fill="#fff"/>
<rect x="14.1" y="13.2" width="3.8" height="11.5" rx="1" fill="#fff"/>
</svg>"##;

const EXIT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="4.5" y="3.5" width="14" height="25" fill="#c67b35" stroke="#7b4f22"/>
<rect x="7" y="6" width="9" height="20" fill="#d99355"/>
<circle cx="15" cy="16" r="1.3" fill="#3d2a12"/>
<path d="M19 16 H29 M25.2 11.8 L29.4 16 L25.2 20.2" fill="none" stroke="#d93025" stroke-width="2.4" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"##;

const PICTURE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="2.5" y="5.5" width="27" height="21" fill="#fff" stroke="#3d6fb5"/>
<rect x="4" y="7" width="24" height="18" fill="#d9ecff"/>
<path d="M4 25 L12 14 L18 21 L22 17 L28 25 Z" fill="#4caf50"/>
<circle cx="22.5" cy="11.5" r="2.5" fill="#f5b800"/>
</svg>"##;

const ZOOM_IN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M19.5 19.5 L28 28" stroke="#7a4a1e" stroke-width="4.6" stroke-linecap="round"/>
<circle cx="13" cy="13" r="9.2" fill="#e3f1ff" stroke="#3b3b3b" stroke-width="2.2"/>
<path d="M8.4 13 H17.6 M13 8.4 V17.6" stroke="#1f5fbf" stroke-width="2.6"/>
</svg>"##;

const ZOOM_OUT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<path d="M19.5 19.5 L28 28" stroke="#7a4a1e" stroke-width="4.6" stroke-linecap="round"/>
<circle cx="13" cy="13" r="9.2" fill="#e3f1ff" stroke="#3b3b3b" stroke-width="2.2"/>
<path d="M8.4 13 H17.6" stroke="#1f5fbf" stroke-width="2.6"/>
</svg>"##;

const ZOOM_100: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="3.5" y="3.5" width="25" height="25" fill="#fff" stroke="#6b6b6b"/>
<path d="M8 11 L10.5 9.5 V22.5 M14.5 13.5 V14.5 M14.5 18.5 V19.5 M19 11 L21.5 9.5 V22.5" fill="none" stroke="#1f5fbf" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"/>
</svg>"##;

const FULL_SCREEN: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="2.5" y="4.5" width="27" height="19" rx="1" fill="#2a5daa" stroke="#1d4a8f"/>
<rect x="4.5" y="6.5" width="23" height="15" fill="#a9cdf5"/>
<path d="M12 28.5 H20 M16 23.5 V28.5" stroke="#555" stroke-width="2"/>
<path d="M7 9 h4 M7 9 v3 M25 9 h-4 M25 9 v3 M7 19 h4 M7 19 v-3 M25 19 h-4 M25 19 v-3" stroke="#1d4a8f" stroke-width="1.6"/>
</svg>"##;

const THUMBNAIL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32">
<rect x="2.5" y="4.5" width="27" height="23" fill="#fff" stroke="#6b6b6b"/>
<rect x="15.5" y="14.5" width="12" height="11" fill="#d9ecff" stroke="#1f5fbf"/>
<path d="M16 25 L19.5 20 L22 23 L23.5 21.5 L27 25 Z" fill="#4caf50"/>
</svg>"##;

const RECT_SELECT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="2.5" width="13" height="11" fill="none" stroke="#333" stroke-dasharray="1.5 1.5"/>
</svg>"##;

const FREE_SELECT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M3 3.5 C6 0.5 13 1.5 14 5 C15 9 10 8 11 12 C11.6 14.5 6 15 3.5 12.5 C1 10 1 5.5 3 3.5 Z" fill="none" stroke="#333" stroke-dasharray="1.5 1.5"/>
</svg>"##;

const SELECT_ALL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="1.5" width="13" height="13" fill="#a9cdf5" stroke="#333" stroke-dasharray="1.5 1.5"/>
</svg>"##;

const INVERT_SELECTION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="1.5" width="13" height="13" fill="#a9cdf5" stroke="#333" stroke-dasharray="1.5 1.5"/>
<rect x="5.5" y="5.5" width="5" height="5" fill="#fff" stroke="#333" stroke-dasharray="1 1"/>
</svg>"##;

const DELETE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M3.5 3.5 L12.5 12.5 M12.5 3.5 L3.5 12.5" stroke="#d93025" stroke-width="2.6" stroke-linecap="round"/>
</svg>"##;

const TRANSPARENT_SELECTION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="1.5" width="13" height="13" fill="#fff" stroke="#333" stroke-dasharray="1.5 1.5"/>
<path d="M3 3 h3 v3 h-3 z M9 3 h3 v3 h-3 z M6 6 h3 v3 h-3 z M3 9 h3 v3 h-3 z M9 9 h3 v3 h-3 z" fill="#c8c8c8"/>
<circle cx="10" cy="10" r="3" fill="#1f7ae0" fill-opacity="0.6"/>
</svg>"##;

const ROTATE_RIGHT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="5.5" width="9" height="9" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M6 3.5 A7 7 0 0 1 13.8 9.5" fill="none" stroke="#333" stroke-width="1.4"/>
<path d="M11.4 8.6 L13.9 12.3 L16 8.4 Z" fill="#333"/>
</svg>"##;

const ROTATE_LEFT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<g transform="matrix(-1 0 0 1 16 0)">
<rect x="1.5" y="5.5" width="9" height="9" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M6 3.5 A7 7 0 0 1 13.8 9.5" fill="none" stroke="#333" stroke-width="1.4"/>
<path d="M11.4 8.6 L13.9 12.3 L16 8.4 Z" fill="#333"/>
</g>
</svg>"##;

const ROTATE_180: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="4.5" y="5.5" width="7" height="7" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M2 9 A6 6 0 0 1 14 9" fill="none" stroke="#333" stroke-width="1.4"/>
<path d="M11.6 8.2 L14 12 L16 8 Z" fill="#333"/>
</svg>"##;

const FLIP_VERTICAL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M2.5 1.5 H13.5 L8 6.5 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M2.5 14.5 H13.5 L8 9.5 Z" fill="#fff" stroke="#1f5fbf" stroke-linejoin="round" stroke-dasharray="1.5 1"/>
<path d="M0.5 8 H15.5" stroke="#333" stroke-dasharray="2 1"/>
</svg>"##;

const FLIP_HORIZONTAL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 2.5 V13.5 L6.5 8 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M14.5 2.5 V13.5 L9.5 8 Z" fill="#fff" stroke="#1f5fbf" stroke-linejoin="round" stroke-dasharray="1.5 1"/>
<path d="M8 0.5 V15.5" stroke="#333" stroke-dasharray="2 1"/>
</svg>"##;

const STATUS_CURSOR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 1 V15 M1 8 H15" stroke="#444" stroke-width="1"/>
<path d="M5 2.5 V12.5 L7.4 10.3 L9 14 L10.4 13.4 L8.8 9.8 L12 9.6 Z" fill="#fff" stroke="#222" stroke-width="0.9" stroke-linejoin="round"/>
</svg>"##;

const STATUS_SELECTION: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="3.5" width="13" height="9" fill="none" stroke="#333" stroke-dasharray="1.5 1.5"/>
</svg>"##;

const STATUS_CANVAS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="2.5" width="11" height="9" fill="#fff" stroke="#555"/>
<rect x="11.5" y="10.5" width="3" height="3" fill="#fff" stroke="#333"/>
<rect x="12" y="5.5" width="2" height="2" fill="#333"/>
<rect x="6" y="12" width="2" height="2" fill="#333"/>
</svg>"##;

const STATUS_DISK: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 1.5 H12.5 L14.5 3.5 V14.5 H1.5 Z" fill="#7d8796" stroke="#4b5563" stroke-linejoin="round"/>
<rect x="4" y="1.5" width="7.5" height="5" fill="#eef1f5"/>
<rect x="3.5" y="9" width="9" height="5.5" fill="#ffffff"/>
</svg>"##;

const HELP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<circle cx="8" cy="8" r="7" fill="#1f6fd0"/>
<path d="M5.7 6.2 A2.3 2.3 0 1 1 9.1 8.2 C8.3 8.7 8 9.1 8 10.1" fill="none" stroke="#fff" stroke-width="1.7" stroke-linecap="round"/>
<circle cx="8" cy="12.3" r="1.05" fill="#fff"/>
</svg>"##;

const OPAQUE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="0.5" y="1.5" width="15" height="13" fill="#a9cdf5" stroke="#1f5fbf"/>
<path d="M3 13 L7.2 3 H8.8 L13 13 H11 L10 10.5 H6 L5 13 Z M6.6 9 H9.4 L8 5.3 Z" fill="#222" fill-rule="evenodd"/>
</svg>"##;

const TRANSPARENT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="0.5" y="1.5" width="15" height="13" fill="#fff" stroke="#888" stroke-dasharray="1.5 1"/>
<path d="M1 2 h3 v3 h-3 z M7 2 h3 v3 h-3 z M13 2 h2 v3 h-2 z M4 5 h3 v3 h-3 z M10 5 h3 v3 h-3 z M1 8 h3 v3 h-3 z M7 8 h3 v3 h-3 z M13 8 h2 v3 h-2 z M4 11 h3 v3 h-3 z M10 11 h3 v3 h-3 z" fill="#dedede"/>
<path d="M3 13 L7.2 3 H8.8 L13 13 H11 L10 10.5 H6 L5 13 Z M6.6 9 H9.4 L8 5.3 Z" fill="#222" fill-rule="evenodd"/>
</svg>"##;

const INVERT_COLOR: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="1.5" width="13" height="13" fill="#fff" stroke="#555"/>
<path d="M2 14 L14 2 V14 Z" fill="#222"/>
</svg>"##;

const RECENT: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<circle cx="8" cy="8" r="6.5" fill="#fff" stroke="#1f5fbf" stroke-width="1.2"/>
<path d="M8 4 V8 L10.8 9.8" fill="none" stroke="#333" stroke-width="1.2" stroke-linecap="round"/>
</svg>"##;

const RIBBON_STYLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<rect x="1.5" y="2.5" width="13" height="11" rx="1.5" fill="#fff" stroke="#1f5fbf"/>
<rect x="2" y="3" width="12" height="3.5" fill="#a9cdf5"/>
<path d="M3.5 4.8 H6 M7.5 4.8 H10" stroke="#1f5fbf" stroke-width="1.1"/>
<circle cx="11.5" cy="10.5" r="2.4" fill="#f6c343" stroke="#9a6b00" stroke-width="0.7"/>
</svg>"##;

const SETTINGS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<circle cx="8" cy="8" r="5.2" fill="none" stroke="#555" stroke-width="2.6" stroke-dasharray="2.2 1.9"/>
<circle cx="8" cy="8" r="4" fill="#9aa3ad" stroke="#555"/><circle cx="8" cy="8" r="1.6" fill="#fff"/>
</svg>"##;

const OPACITY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 1.5 C8 1.5 3 7 3 10 A5 5 0 0 0 13 10 C13 7 8 1.5 8 1.5 Z" fill="#fff" stroke="#1f5fbf"/>
<path d="M3 10 H13 A5 5 0 0 1 3 10 Z" fill="#1f5fbf"/>
</svg>"##;

const LAYERS: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 1.5 L15 5 L8 8.5 L1 5 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M1 8 L8 11.5 L15 8" fill="none" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M1 11 L8 14.5 L15 11" fill="none" stroke="#1f5fbf" stroke-linejoin="round"/>
</svg>"##;

const EYE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 8 C4 3.8 12 3.8 14.5 8 C12 12.2 4 12.2 1.5 8 Z" fill="#fff" stroke="#444"/>
<circle cx="8" cy="8" r="2.2" fill="#1f5fbf"/>
</svg>"##;

const EYE_OFF: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M1.5 8 C4 3.8 12 3.8 14.5 8 C12 12.2 4 12.2 1.5 8 Z" fill="none" stroke="#999"/>
<path d="M3 13 L13 3" stroke="#999" stroke-width="1.4" stroke-linecap="round"/>
</svg>"##;

const ADD_LAYER: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 1.5 L14 4.5 L8 7.5 L2 4.5 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
<path d="M2 7.5 L8 10.5 L14 7.5" fill="none" stroke="#1f5fbf" stroke-linejoin="round"/>
<circle cx="11.8" cy="11.8" r="3.7" fill="#2e9d3a"/>
<path d="M11.8 9.9 V13.7 M9.9 11.8 H13.7" stroke="#fff" stroke-width="1.3"/>
</svg>"##;

const MERGE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 1 V6.2 M5.7 4 L8 6.5 L10.3 4" fill="none" stroke="#1f5fbf" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M8 8.5 L14.5 11.75 L8 15 L1.5 11.75 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
</svg>"##;

const MERGE_UP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16">
<path d="M8 6.5 V1.3 M5.7 3.5 L8 1 L10.3 3.5" fill="none" stroke="#1f5fbf" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"/>
<path d="M8 8.5 L14.5 11.75 L8 15 L1.5 11.75 Z" fill="#a9cdf5" stroke="#1f5fbf" stroke-linejoin="round"/>
</svg>"##;

/// macOS application icon: rounded tile (Big Sur+ grid) with the palette.
pub const APP_MAC: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 1024 1024">
<defs>
<linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="#ffffff"/><stop offset="1" stop-color="#d9e4f2"/></linearGradient>
<linearGradient id="p" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fbe3bd"/><stop offset="1" stop-color="#e2b57a"/></linearGradient>
<filter id="s" x="-10%" y="-10%" width="120%" height="120%"><feGaussianBlur stdDeviation="14"/></filter>
</defs>
<rect x="100" y="114" width="824" height="824" rx="185" fill="#000" fill-opacity="0.22" filter="url(#s)"/>
<rect x="100" y="100" width="824" height="824" rx="185" fill="url(#bg)"/>
<rect x="101" y="101" width="822" height="822" rx="184" fill="none" stroke="#000" stroke-opacity="0.08" stroke-width="2"/>
<g transform="translate(192 196) scale(10)">
<path d="M32 5 C14 5 4 18 4 31 C4 46 16 58 30 58 C35 58 37 55 35.5 51.5 C34 48 36 45 40 45 H47 C54 45 60 40 60 31 C60 16 48 5 32 5 Z" fill="url(#p)" stroke="#8a5a2b" stroke-width="2.2"/>
<circle cx="17" cy="28" r="5.5" fill="#e53935"/>
<circle cx="26" cy="16" r="5.5" fill="#fbc02d"/>
<circle cx="40" cy="15" r="5.5" fill="#43a047"/>
<circle cx="50" cy="26" r="5.5" fill="#1e88e5"/>
<circle cx="17" cy="42" r="5.5" fill="#8e24aa"/>
<path d="M28 44 L52 18 A3 3 0 0 1 56.5 22.5 L32.5 48 Z" fill="#c67b35" stroke="#6d4219" stroke-width="1.4" stroke-linejoin="round"/>
<path d="M27 43 L33.5 49.5 C30 55 24 57 18 56 C19 50.5 22 46 27 43 Z" fill="#1f5fbf" stroke="#173f7a" stroke-width="1.4" stroke-linejoin="round"/>
</g>
</svg>"##;

pub const APP: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">
<defs><linearGradient id="p" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#fbe3bd"/><stop offset="1" stop-color="#e2b57a"/></linearGradient></defs>
<path d="M32 5 C14 5 4 18 4 31 C4 46 16 58 30 58 C35 58 37 55 35.5 51.5 C34 48 36 45 40 45 H47 C54 45 60 40 60 31 C60 16 48 5 32 5 Z" fill="url(#p)" stroke="#8a5a2b" stroke-width="2.5"/>
<circle cx="17" cy="28" r="5.5" fill="#e53935"/>
<circle cx="26" cy="16" r="5.5" fill="#fbc02d"/>
<circle cx="40" cy="15" r="5.5" fill="#43a047"/>
<circle cx="50" cy="26" r="5.5" fill="#1e88e5"/>
<circle cx="17" cy="42" r="5.5" fill="#8e24aa"/>
<path d="M28 44 L52 18 A3 3 0 0 1 56.5 22.5 L32.5 48 Z" fill="#c67b35" stroke="#6d4219" stroke-width="1.5" stroke-linejoin="round"/>
<path d="M27 43 L33.5 49.5 C30 55 24 57 18 56 C19 50.5 22 46 27 43 Z" fill="#1f5fbf" stroke="#173f7a" stroke-width="1.5" stroke-linejoin="round"/>
</svg>"##;

impl Icon {
    fn svg(self) -> &'static str {
        match self {
            Icon::Paste => PASTE,
            Icon::Cut => CUT,
            Icon::Copy => COPY,
            Icon::Select => SELECT,
            Icon::Crop => CROP,
            Icon::Resize => RESIZE,
            Icon::Rotate => ROTATE,
            Icon::Pencil => PENCIL,
            Icon::Fill => FILL,
            Icon::Text => TEXT,
            Icon::Eraser => ERASER,
            Icon::Picker => PICKER,
            Icon::Magnifier => MAGNIFIER,
            Icon::Brushes => BRUSHES,
            Icon::Outline => OUTLINE,
            Icon::ShapeFill => SHAPE_FILL,
            Icon::Size => SIZE,
            Icon::EditColors => EDIT_COLORS,
            Icon::Save => SAVE,
            Icon::Undo => UNDO,
            Icon::Redo => REDO,
            Icon::New => NEW,
            Icon::Open => OPEN,
            Icon::SaveAs => SAVE_AS,
            Icon::Print => PRINT,
            Icon::Properties => PROPERTIES,
            Icon::About => ABOUT,
            Icon::Exit => EXIT,
            Icon::Picture => PICTURE,
            Icon::ZoomIn => ZOOM_IN,
            Icon::ZoomOut => ZOOM_OUT,
            Icon::Zoom100 => ZOOM_100,
            Icon::FullScreen => FULL_SCREEN,
            Icon::Thumbnail => THUMBNAIL,
            Icon::RectSelect => RECT_SELECT,
            Icon::FreeSelect => FREE_SELECT,
            Icon::SelectAll => SELECT_ALL,
            Icon::InvertSelection => INVERT_SELECTION,
            Icon::Delete => DELETE,
            Icon::TransparentSelection => TRANSPARENT_SELECTION,
            Icon::RotateRight => ROTATE_RIGHT,
            Icon::RotateLeft => ROTATE_LEFT,
            Icon::Rotate180 => ROTATE_180,
            Icon::FlipVertical => FLIP_VERTICAL,
            Icon::FlipHorizontal => FLIP_HORIZONTAL,
            Icon::StatusCursor => STATUS_CURSOR,
            Icon::StatusSelection => STATUS_SELECTION,
            Icon::StatusCanvas => STATUS_CANVAS,
            Icon::StatusDisk => STATUS_DISK,
            Icon::Help => HELP,
            Icon::Opaque => OPAQUE,
            Icon::Transparent => TRANSPARENT,
            Icon::InvertColor => INVERT_COLOR,
            Icon::Recent => RECENT,
            Icon::RibbonStyle => RIBBON_STYLE,
            Icon::Settings => SETTINGS,
            Icon::Opacity => OPACITY,
            Icon::Layers => LAYERS,
            Icon::Eye => EYE,
            Icon::EyeOff => EYE_OFF,
            Icon::AddLayer => ADD_LAYER,
            Icon::Merge => MERGE,
            Icon::MergeUp => MERGE_UP,
            Icon::App => APP,
        }
    }
}

fn fluent(body: &str) -> String {
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="none" stroke="#242424" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round">{body}</svg>"##
    )
}

fn rainbow_disc() -> String {
    let colors = ["#ff4b4b", "#ffb300", "#4cd964", "#18c4d8", "#2f7bff", "#b44dff"];
    let mut body = String::new();
    for (i, c) in colors.iter().enumerate() {
        let a0 = (i as f32 * 60.0 - 90.0).to_radians();
        let a1 = ((i + 1) as f32 * 60.0 - 90.0).to_radians();
        let (x0, y0, x1, y1) = (9.0 + 7.5 * a0.cos(), 9.0 + 7.5 * a0.sin(), 9.0 + 7.5 * a1.cos(), 9.0 + 7.5 * a1.sin());
        body.push_str(&format!(r##"<path d="M9 9 L{x0:.2} {y0:.2} A7.5 7.5 0 0 1 {x1:.2} {y1:.2} Z" fill="{c}" stroke="none"/>"##));
    }
    body.push_str(r##"<circle cx="9" cy="9" r="2.6" fill="#fff" stroke="none"/><circle cx="15" cy="15" r="4" fill="#fff" stroke="#242424"/><path d="M15 13 V17 M13 15 H17"/>"##);
    fluent(&body)
}

impl Icon {
    /// Windows 11 style (Fluent outline) variant, if one exists.
    fn svg_modern(self) -> Option<String> {
        let body = match self {
            Icon::Paste => r##"<path d="M7 4 H5.5 A1.5 1.5 0 0 0 4 5.5 V16 A1.5 1.5 0 0 0 5.5 17.5 H8.5"/><path d="M13 4 H14.5 A1.5 1.5 0 0 1 16 5.5 V7.5"/><rect x="7" y="2.5" width="6" height="3" rx="1"/><rect x="10.5" y="9.5" width="7" height="8" rx="1.2"/>"##,
            Icon::Cut => r##"<circle cx="5.8" cy="14.2" r="2.6"/><circle cx="14.2" cy="14.2" r="2.6"/><path d="M7.6 12.3 L14.5 2.5"/><path d="M12.4 12.3 L5.5 2.5"/>"##,
            Icon::Copy => r##"<rect x="7" y="6" width="9.5" height="11.5" rx="1.5"/><path d="M4 13.5 V4 A1.5 1.5 0 0 1 5.5 2.5 H12.5"/>"##,
            Icon::Select | Icon::RectSelect | Icon::StatusSelection => r##"<path d="M3 6 V4.5 A1.5 1.5 0 0 1 4.5 3 H6 M9 3 H11 M14 3 H15.5 A1.5 1.5 0 0 1 17 4.5 V6 M17 9 V11 M17 14 V15.5 A1.5 1.5 0 0 1 15.5 17 H14 M11 17 H9 M6 17 H4.5 A1.5 1.5 0 0 1 3 15.5 V14 M3 11 V9"/>"##,
            Icon::FreeSelect => r##"<ellipse cx="10" cy="8" rx="7" ry="5" stroke-dasharray="2 1.8"/><path d="M6 12.2 C5 14 6 16 8 16.5 C9 16.8 9.5 17.5 9 18.5"/>"##,
            Icon::Crop => r##"<path d="M5.5 2.5 V12.5 A2 2 0 0 0 7.5 14.5 H17.5"/><path d="M2.5 5.5 H12.5 A2 2 0 0 1 14.5 7.5 V17.5"/>"##,
            Icon::Resize => r##"<rect x="2.5" y="10.5" width="7" height="7" rx="1.2"/><path d="M2.5 7.5 V4 A1.5 1.5 0 0 1 4 2.5 H16 A1.5 1.5 0 0 1 17.5 4 V16 A1.5 1.5 0 0 1 16 17.5 H12.5"/><path d="M9 11 L14 6 M10.5 6 H14 V9.5"/>"##,
            Icon::Rotate => r##"<rect x="8" y="8.5" width="9" height="9" rx="1.5"/><path d="M3.5 11 A7.5 7.5 0 0 1 11 3.5 H12.5"/><path d="M10.5 1.5 L12.5 3.5 L10.5 5.5"/>"##,
            Icon::Pencil => r##"<path d="M12.8 3.7 A2 2 0 0 1 15.6 3.7 L16.3 4.4 A2 2 0 0 1 16.3 7.2 L7.2 16.3 L3 17 L3.7 12.8 Z" fill="#FFC83D"/><path d="M12.8 3.7 A2 2 0 0 1 15.6 3.7 L16.3 4.4 A2 2 0 0 1 16.3 7.2 L14.8 8.7 L11.3 5.2 Z" fill="#F08CAE"/><path d="M11.3 5.2 L14.8 8.7"/>"##,
            Icon::Fill => r##"<path d="M2.8 10 H15.2 L9.8 15.4 A1.6 1.6 0 0 1 7.5 15.4 L3.2 11.1 A1.6 1.6 0 0 1 2.8 10 Z" fill="#E8435A" stroke="none"/><path d="M8.6 3.4 L15.2 10 L9.8 15.4 A1.6 1.6 0 0 1 7.5 15.4 L3.2 11.1 A1.6 1.6 0 0 1 3.2 8.8 Z"/><path d="M2.8 10 H15.2"/><path d="M8.6 3.4 L6.6 1.4"/><path d="M16.8 12.5 C16.8 12.5 18.3 14.3 18.3 15.4 A1.5 1.5 0 0 1 15.3 15.4 C15.3 14.3 16.8 12.5 16.8 12.5 Z" fill="#E8435A"/>"##,
            Icon::Text => r##"<path d="M4 17 L10 3 L16 17"/><path d="M6.4 11.5 H13.6"/>"##,
            Icon::Eraser => r##"<path d="M11.6 3.1 A1.8 1.8 0 0 1 14.2 3.1 L16.9 5.8 A1.8 1.8 0 0 1 16.9 8.4 L12.5 12.8 L7.2 7.5 Z" fill="#F4A3C6"/><path d="M11.6 3.1 A1.8 1.8 0 0 1 14.2 3.1 L16.9 5.8 A1.8 1.8 0 0 1 16.9 8.4 L9 16.3 H6 L3.1 13.4 A1.8 1.8 0 0 1 3.1 10.8 Z"/><path d="M7.2 7.5 L12.5 12.8"/><path d="M9 16.3 H17"/>"##,
            Icon::Picker => r##"<path d="M3 17.5 L4.7 15.8 L6.1 15.5 L4.5 13.9 Z" fill="#3AA0F3" stroke="none"/><path d="M11.3 4.6 L12.9 3 A2.2 2.2 0 0 1 16 6.1 L14.4 7.7"/><path d="M10.2 3.5 L15.5 8.8"/><path d="M12.4 6.6 L5.4 13.6 A1.5 1.5 0 0 0 5 14.4 L4.7 15.8 L3 17.5 M4.7 15.8 L6.1 15.5 A1.5 1.5 0 0 0 6.9 15.1 L13.9 8.1"/>"##,
            Icon::Magnifier => r##"<circle cx="8.5" cy="8.5" r="5.5" fill="#3AA0F3" fill-opacity="0.28"/><path d="M12.5 12.5 L17 17"/>"##,
            Icon::Brushes => r##"<path d="M16.4 2.9 A1.5 1.5 0 0 1 17.1 4.9 L11.1 11.4 L8.9 9.2 L15.4 3.1 A1.5 1.5 0 0 1 16.4 2.9 Z"/><path d="M8.9 9.2 L11.1 11.4 A3 3 0 0 1 10.2 14.3 C8.5 16 5.6 16.6 3 17 C3.6 14.6 4 11.6 5.7 10 A3 3 0 0 1 8.9 9.2 Z" fill="#3AA0F3"/>"##,
            Icon::Outline => r##"<rect x="2.5" y="5.5" width="10" height="10" rx="1.5"/><path d="M11 13.6 L16 8.6 L17.6 10.2 L12.6 15.2 L10.5 15.7 Z"/>"##,
            Icon::ShapeFill => r##"<rect x="2.5" y="5.5" width="10" height="10" rx="1.5" fill="#242424" fill-opacity="0.2"/><path d="M15.5 8.5 C15.5 8.5 17.5 11 17.5 12.4 A2 2 0 0 1 13.5 12.4 C13.5 11 15.5 8.5 15.5 8.5 Z"/>"##,
            Icon::Size => r##"<path d="M3 4.5 H17" stroke-width="1"/><path d="M3 8.5 H17" stroke-width="1.8"/><path d="M3 13.2 H17" stroke-width="2.8"/>"##,
            Icon::EditColors => return Some(rainbow_disc()),
            Icon::Save | Icon::StatusDisk => r##"<path d="M4.5 2.5 H13 L17.5 7 V15.5 A2 2 0 0 1 15.5 17.5 H4.5 A2 2 0 0 1 2.5 15.5 V4.5 A2 2 0 0 1 4.5 2.5 Z"/><path d="M6.5 2.5 V6 A1 1 0 0 0 7.5 7 H11.5 A1 1 0 0 0 12.5 6 V2.5"/><path d="M5.5 17.5 V12 A1 1 0 0 1 6.5 11 H13.5 A1 1 0 0 1 14.5 12 V17.5"/>"##,
            Icon::Undo => r##"<path d="M4 8 H12.5 A4.5 4.5 0 0 1 12.5 17 H9"/><path d="M7.5 4.5 L4 8 L7.5 11.5"/>"##,
            Icon::Redo => r##"<path d="M16 8 H7.5 A4.5 4.5 0 0 0 7.5 17 H11"/><path d="M12.5 4.5 L16 8 L12.5 11.5"/>"##,
            Icon::New => r##"<path d="M5.5 2.5 H11.5 L16 7 V16 A1.5 1.5 0 0 1 14.5 17.5 H5.5 A1.5 1.5 0 0 1 4 16 V4 A1.5 1.5 0 0 1 5.5 2.5 Z"/><path d="M11.5 2.5 V5.5 A1.5 1.5 0 0 0 13 7 H16"/>"##,
            Icon::Open => r##"<path d="M2.5 6 V14.5 A2 2 0 0 0 4.5 16.5 H15.5 A2 2 0 0 0 17.5 14.5 V8 A2 2 0 0 0 15.5 6 H10 L8 3.5 H4.5 A2 2 0 0 0 2.5 5.5 Z"/>"##,
            Icon::Recent => r##"<circle cx="10" cy="10" r="7.5"/><path d="M10 5.5 V10 L13 12"/>"##,
            Icon::SaveAs => r##"<path d="M9 17.5 H4.5 A2 2 0 0 1 2.5 15.5 V4.5 A2 2 0 0 1 4.5 2.5 H13 L17.5 7 V9"/><path d="M6.5 2.5 V6 A1 1 0 0 0 7.5 7 H11.5 A1 1 0 0 0 12.5 6 V2.5"/><path d="M15.6 10.6 A1.4 1.4 0 0 1 17.6 12.6 L12.8 17.4 L10.5 18 L11.1 15.7 Z"/>"##,
            Icon::Print => r##"<path d="M5.5 7 V3.5 A1 1 0 0 1 6.5 2.5 H13.5 A1 1 0 0 1 14.5 3.5 V7"/><path d="M5.5 14 H4 A1.5 1.5 0 0 1 2.5 12.5 V8.5 A1.5 1.5 0 0 1 4 7 H16 A1.5 1.5 0 0 1 17.5 8.5 V12.5 A1.5 1.5 0 0 1 16 14 H14.5"/><rect x="5.5" y="11" width="9" height="6.5" rx="1"/>"##,
            Icon::Properties => r##"<rect x="3.5" y="2.5" width="13" height="15" rx="1.5"/><path d="M6.5 6.5 H13.5 M6.5 10 H13.5 M6.5 13.5 H10.5"/>"##,
            Icon::About => r##"<circle cx="10" cy="10" r="7.5"/><path d="M10 9 V14"/><circle cx="10" cy="6.3" r="0.7" fill="#242424" stroke="none"/>"##,
            Icon::Exit => r##"<path d="M8 17.5 H4.5 A2 2 0 0 1 2.5 15.5 V4.5 A2 2 0 0 1 4.5 2.5 H8"/><path d="M7.5 10 H17.5 M14 6.5 L17.5 10 L14 13.5"/>"##,
            Icon::Picture => r##"<rect x="2.5" y="3.5" width="15" height="13" rx="2"/><circle cx="13" cy="7.5" r="1.5"/><path d="M2.5 13.5 L7 9.5 L12.5 15"/><path d="M11 13.5 L13 11.5 L17.5 15.5"/>"##,
            Icon::ZoomIn => r##"<circle cx="8.5" cy="8.5" r="5.5"/><path d="M12.5 12.5 L17 17 M6 8.5 H11 M8.5 6 V11"/>"##,
            Icon::ZoomOut => r##"<circle cx="8.5" cy="8.5" r="5.5"/><path d="M12.5 12.5 L17 17 M6 8.5 H11"/>"##,
            Icon::Zoom100 => r##"<rect x="2.5" y="2.5" width="15" height="15" rx="2"/><path d="M6 7.5 L7.5 6.5 V13.5 M12 7.5 L13.5 6.5 V13.5"/><circle cx="10" cy="8.5" r="0.6" fill="#242424" stroke="none"/><circle cx="10" cy="11.5" r="0.6" fill="#242424" stroke="none"/>"##,
            Icon::FullScreen => r##"<path d="M3 7 V4.5 A1.5 1.5 0 0 1 4.5 3 H7 M13 3 H15.5 A1.5 1.5 0 0 1 17 4.5 V7 M17 13 V15.5 A1.5 1.5 0 0 1 15.5 17 H13 M7 17 H4.5 A1.5 1.5 0 0 1 3 15.5 V13"/>"##,
            Icon::Thumbnail => r##"<rect x="2.5" y="3.5" width="15" height="13" rx="2"/><rect x="10" y="10" width="5" height="4" rx="0.8"/>"##,
            Icon::SelectAll => r##"<rect x="3" y="3" width="14" height="14" rx="1.5" stroke-dasharray="2 1.8"/><rect x="6" y="6" width="8" height="8" rx="1" fill="#242424" fill-opacity="0.22" stroke="none"/>"##,
            Icon::InvertSelection => r##"<rect x="3" y="3" width="14" height="14" rx="1.5" fill="#242424" fill-opacity="0.16" stroke-dasharray="2 1.8"/><rect x="7" y="7" width="6" height="6" rx="1" fill="#fff"/>"##,
            Icon::Delete => r##"<path d="M3.5 5.5 H16.5 M8 5.5 V4 A1 1 0 0 1 9 3 H11 A1 1 0 0 1 12 4 V5.5 M5 5.5 L5.8 16 A1.5 1.5 0 0 0 7.3 17.5 H12.7 A1.5 1.5 0 0 0 14.2 16 L15 5.5 M8.5 8.5 V14.5 M11.5 8.5 V14.5"/>"##,
            Icon::TransparentSelection => r##"<rect x="3" y="3" width="14" height="14" rx="1.5" stroke-dasharray="2 1.8"/><path d="M6 6 h2.7 v2.7 h-2.7 z M11.3 6 h2.7 v2.7 h-2.7 z M8.7 8.7 h2.6 v2.6 h-2.6 z M6 11.3 h2.7 v2.7 h-2.7 z M11.3 11.3 h2.7 v2.7 h-2.7 z" fill="#242424" fill-opacity="0.35" stroke="none"/>"##,
            Icon::RotateRight => r##"<rect x="3" y="8.5" width="9" height="9" rx="1.5"/><path d="M7 4.5 H9 A7 7 0 0 1 16.5 11.5 V13"/><path d="M14.5 11 L16.5 13 L18.5 11"/>"##,
            Icon::RotateLeft => r##"<g transform="matrix(-1 0 0 1 20 0)"><rect x="3" y="8.5" width="9" height="9" rx="1.5"/><path d="M7 4.5 H9 A7 7 0 0 1 16.5 11.5 V13"/><path d="M14.5 11 L16.5 13 L18.5 11"/></g>"##,
            Icon::Rotate180 => r##"<rect x="5.5" y="9.5" width="9" height="8" rx="1.5"/><path d="M3.5 7.5 A7 5 0 0 1 16.5 7.5"/><path d="M14.3 5.6 L16.5 7.5 L18.4 5.4"/>"##,
            Icon::FlipVertical => r##"<path d="M2.5 10 H17.5" stroke-dasharray="2 1.8"/><path d="M5 7.5 L10 3 L15 7.5 Z"/><path d="M5 12.5 L10 17 L15 12.5 Z" fill="#242424" fill-opacity="0.2"/>"##,
            Icon::FlipHorizontal => r##"<path d="M10 2.5 V17.5" stroke-dasharray="2 1.8"/><path d="M7.5 5 L3 10 L7.5 15 Z"/><path d="M12.5 5 L17 10 L12.5 15 Z" fill="#242424" fill-opacity="0.2"/>"##,
            Icon::StatusCursor => r##"<path d="M5 3 V15 L8.2 12.2 L10.5 17 L12.3 16.2 L10 11.5 L14.2 11.2 Z"/>"##,
            Icon::StatusCanvas => r##"<rect x="2.5" y="3.5" width="12" height="10" rx="1.5"/><path d="M17.5 9 V15.5 A1.5 1.5 0 0 1 16 17 H9"/>"##,
            Icon::Help => r##"<circle cx="10" cy="10" r="7.5"/><path d="M7.8 7.8 A2.3 2.3 0 1 1 10.9 10 C10.3 10.3 10 10.8 10 11.5"/><circle cx="10" cy="14" r="0.7" fill="#242424" stroke="none"/>"##,
            Icon::Opaque => r##"<rect x="2.5" y="3.5" width="15" height="13" rx="2" fill="#242424" fill-opacity="0.15"/><path d="M6.5 14 L10 6 L13.5 14 M7.8 11 H12.2"/>"##,
            Icon::Transparent => r##"<rect x="2.5" y="3.5" width="15" height="13" rx="2" stroke-dasharray="2 1.8"/><path d="M6.5 14 L10 6 L13.5 14 M7.8 11 H12.2"/>"##,
            Icon::InvertColor => r##"<circle cx="10" cy="10" r="7.5"/><path d="M10 2.5 A7.5 7.5 0 0 1 10 17.5 Z" fill="#242424" stroke="none"/>"##,
            Icon::RibbonStyle => r##"<rect x="2.5" y="3.5" width="15" height="13" rx="2"/><path d="M2.5 8 H17.5"/><path d="M5 5.8 H7 M9 5.8 H11"/>"##,
            Icon::Settings => r##"<circle cx="10" cy="10" r="2.6"/><path d="M8.6 2.5 H11.4 L11.9 4.6 A5.8 5.8 0 0 1 13.5 5.5 L15.5 4.8 L16.9 7.2 L15.3 8.6 A5.8 5.8 0 0 1 15.3 10.4 L16.9 11.8 L15.5 14.2 L13.5 13.5 A5.8 5.8 0 0 1 11.9 14.4 L11.4 16.5 H8.6 L8.1 14.4 A5.8 5.8 0 0 1 6.5 13.5 L4.5 14.2 L3.1 11.8 L4.7 10.4 A5.8 5.8 0 0 1 4.7 8.6 L3.1 7.2 L4.5 4.8 L6.5 5.5 A5.8 5.8 0 0 1 8.1 4.6 Z"/>"##,
            Icon::Opacity => r##"<path d="M10 2.5 C10 2.5 4.5 8.5 4.5 12 A5.5 5.5 0 0 0 15.5 12 C15.5 8.5 10 2.5 10 2.5 Z"/><path d="M4.5 12 H15.5 A5.5 5.5 0 0 1 4.5 12 Z" fill="#242424"/>"##,
            Icon::Layers => return None, // shared colored icon in both styles

            Icon::Eye => return None, // shared colored icon in both styles

            Icon::EyeOff => return None, // shared colored icon in both styles

            Icon::AddLayer => return None, // shared colored icon in both styles

            Icon::Merge => return None, // shared colored icon in both styles

            Icon::MergeUp => return None, // shared colored icon in both styles

            Icon::App => return None,
        };
        Some(fluent(body))
    }
}

/// Rasterize an SVG string into a square pixmap of `px` pixels.
pub fn rasterize_svg(svg: &str, px: u32) -> Option<Pixmap> {
    let tree = usvg::Tree::from_str(svg, &usvg::Options::default()).ok()?;
    let size = tree.size();
    let mut pm = Pixmap::new(px.max(1), px.max(1))?;
    let t = Transform::from_scale(px as f32 / size.width(), px as f32 / size.height());
    resvg::render(&tree, t, &mut pm.as_mut());
    Some(pm)
}

#[derive(Default)]
pub struct Icons {
    cache: HashMap<(Icon, u32, u8), Option<TextureHandle>>,
}

impl Icons {
    pub fn texture(&mut self, ctx: &Context, icon: Icon, px: u32) -> Option<egui::TextureId> {
        let modern = crate::theme::modern();
        let dark = crate::theme::dark();
        let variant = if !modern { 0 } else if dark { 2 } else { 1 };
        self.cache
            .entry((icon, px, variant))
            .or_insert_with(|| {
                let svg = if modern {
                    icon.svg_modern().map(|s| {
                        let pal = crate::theme::w();
                        s.replace("#242424", pal.icon_hex).replace("fill=\"#fff\"", &format!("fill=\"{}\"", pal.icon_bg_hex))
                    })
                } else {
                    None
                };
                let pm = match &svg {
                    Some(s) => rasterize_svg(s, px)?,
                    None => rasterize_svg(icon.svg(), px)?,
                };
                let img = crate::raster::to_color_image(&pm);
                Some(ctx.load_texture(format!("icon-{icon:?}-{px}-{variant}"), img, TextureOptions::LINEAR))
            })
            .as_ref()
            .map(|t| t.id())
    }

    /// Paint an icon into `rect` (rect is in points).
    pub fn paint(&mut self, painter: &Painter, icon: Icon, rect: Rect, enabled: bool) {
        let ppp = painter.ctx().pixels_per_point();
        let px = (rect.width() * ppp).round().max(1.0) as u32;
        // snap to physical pixels for crisp rendering
        let min = (rect.min.to_vec2() * ppp).round() / ppp;
        let rect = Rect::from_min_size(min.to_pos2(), rect.size());
        if let Some(id) = self.texture(painter.ctx(), icon, px) {
            let tint = if enabled { Color32::WHITE } else { Color32::from_white_alpha(90) };
            painter.image(id, rect, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), tint);
        }
    }
}
