//! Fonts and text rasterization for the Text tool.
//!
//! Fonts are discovered from common macOS and Linux font directories and
//! loaded lazily. A bundled font (Selawik, OFL) is always available.

use std::cell::OnceCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use ab_glyph::{point, Font, FontArc, GlyphId, PxScale, ScaleFont};
use egui::Color32;
use tiny_skia::Pixmap;

use crate::raster::IRect;

pub const UI_FONT: &[u8] = include_bytes!("../assets/fonts/Selawik-Regular.ttf");
pub const UI_FONT_BOLD: &[u8] = include_bytes!("../assets/fonts/Selawik-Bold.ttf");

/// Variant file names: [regular, bold, italic, bold italic].
type Family = (&'static str, [&'static [&'static str]; 4]);

const FAMILIES: &[Family] = &[
    ("Arial", [&["Arial.ttf", "arial.ttf"], &["Arial Bold.ttf", "arialbd.ttf"], &["Arial Italic.ttf", "ariali.ttf"], &["Arial Bold Italic.ttf", "arialbi.ttf"]]),
    ("Calibri", [&["Calibri.ttf", "calibri.ttf"], &["Calibri Bold.ttf", "calibrib.ttf"], &["Calibri Italic.ttf", "calibrii.ttf"], &["Calibri Bold Italic.ttf", "calibriz.ttf"]]),
    ("Comic Sans MS", [&["Comic Sans MS.ttf", "comic.ttf"], &["Comic Sans MS Bold.ttf", "comicbd.ttf"], &[], &[]]),
    ("Courier New", [&["Courier New.ttf", "cour.ttf"], &["Courier New Bold.ttf", "courbd.ttf"], &["Courier New Italic.ttf", "couri.ttf"], &["Courier New Bold Italic.ttf", "courbi.ttf"]]),
    ("DejaVu Sans", [&["DejaVuSans.ttf"], &["DejaVuSans-Bold.ttf"], &["DejaVuSans-Oblique.ttf"], &["DejaVuSans-BoldOblique.ttf"]]),
    ("DejaVu Sans Mono", [&["DejaVuSansMono.ttf"], &["DejaVuSansMono-Bold.ttf"], &["DejaVuSansMono-Oblique.ttf"], &["DejaVuSansMono-BoldOblique.ttf"]]),
    ("DejaVu Serif", [&["DejaVuSerif.ttf"], &["DejaVuSerif-Bold.ttf"], &["DejaVuSerif-Italic.ttf"], &["DejaVuSerif-BoldItalic.ttf"]]),
    ("Georgia", [&["Georgia.ttf", "georgia.ttf"], &["Georgia Bold.ttf", "georgiab.ttf"], &["Georgia Italic.ttf", "georgiai.ttf"], &["Georgia Bold Italic.ttf", "georgiaz.ttf"]]),
    ("Impact", [&["Impact.ttf", "impact.ttf"], &[], &[], &[]]),
    ("Liberation Mono", [&["LiberationMono-Regular.ttf"], &["LiberationMono-Bold.ttf"], &["LiberationMono-Italic.ttf"], &["LiberationMono-BoldItalic.ttf"]]),
    ("Liberation Sans", [&["LiberationSans-Regular.ttf"], &["LiberationSans-Bold.ttf"], &["LiberationSans-Italic.ttf"], &["LiberationSans-BoldItalic.ttf"]]),
    ("Liberation Serif", [&["LiberationSerif-Regular.ttf"], &["LiberationSerif-Bold.ttf"], &["LiberationSerif-Italic.ttf"], &["LiberationSerif-BoldItalic.ttf"]]),
    ("Noto Sans", [&["NotoSans-Regular.ttf"], &["NotoSans-Bold.ttf"], &["NotoSans-Italic.ttf"], &["NotoSans-BoldItalic.ttf"]]),
    ("Noto Serif", [&["NotoSerif-Regular.ttf"], &["NotoSerif-Bold.ttf"], &["NotoSerif-Italic.ttf"], &["NotoSerif-BoldItalic.ttf"]]),
    ("Tahoma", [&["Tahoma.ttf", "tahoma.ttf"], &["Tahoma Bold.ttf", "tahomabd.ttf"], &[], &[]]),
    ("Times New Roman", [&["Times New Roman.ttf", "times.ttf"], &["Times New Roman Bold.ttf", "timesbd.ttf"], &["Times New Roman Italic.ttf", "timesi.ttf"], &["Times New Roman Bold Italic.ttf", "timesbi.ttf"]]),
    ("Trebuchet MS", [&["Trebuchet MS.ttf", "trebuc.ttf"], &["Trebuchet MS Bold.ttf", "trebucbd.ttf"], &["Trebuchet MS Italic.ttf", "trebucit.ttf"], &["Trebuchet MS Bold Italic.ttf", "trebucbi.ttf"]]),
    ("Ubuntu", [&["Ubuntu-R.ttf", "Ubuntu-Regular.ttf"], &["Ubuntu-B.ttf", "Ubuntu-Bold.ttf"], &["Ubuntu-RI.ttf", "Ubuntu-Italic.ttf"], &["Ubuntu-BI.ttf", "Ubuntu-BoldItalic.ttf"]]),
    ("Verdana", [&["Verdana.ttf", "verdana.ttf"], &["Verdana Bold.ttf", "verdanab.ttf"], &["Verdana Italic.ttf", "verdanai.ttf"], &["Verdana Bold Italic.ttf", "verdanaz.ttf"]]),
];

fn font_dirs() -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = [
        "/System/Library/Fonts/Supplemental",
        "/System/Library/Fonts",
        "/Library/Fonts",
        "/usr/share/fonts/truetype/msttcorefonts",
        "/usr/share/fonts/truetype/dejavu",
        "/usr/share/fonts/truetype/liberation",
        "/usr/share/fonts/truetype/liberation2",
        "/usr/share/fonts/truetype/noto",
        "/usr/share/fonts/truetype/ubuntu",
        "/usr/share/fonts/truetype",
        "/usr/share/fonts/TTF",
        "/usr/share/fonts/dejavu",
        "/usr/share/fonts/dejavu-sans-fonts",
        "/usr/share/fonts/dejavu-sans-mono-fonts",
        "/usr/share/fonts/dejavu-serif-fonts",
        "/usr/share/fonts/liberation",
        "/usr/share/fonts/liberation-sans",
        "/usr/share/fonts/liberation-serif",
        "/usr/share/fonts/liberation-mono",
        "/usr/share/fonts/noto",
        "/usr/share/fonts/google-noto",
        "/usr/share/fonts/corefonts",
        "/usr/share/fonts/msttcore",
        "/usr/share/fonts/ubuntu",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        v.push(home.join("Library/Fonts"));
        v.push(home.join(".fonts"));
        v.push(home.join(".local/share/fonts"));
    }
    v.retain(|d| d.is_dir());
    v
}

/// All font files under the system font directories.
fn font_files() -> Vec<PathBuf> {
    let mut bases: Vec<PathBuf> = vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        let home = PathBuf::from(home);
        bases.push(home.join("Library/Fonts"));
        bases.push(home.join(".fonts"));
        bases.push(home.join(".local/share/fonts"));
    }
    let mut out = Vec::new();
    for b in bases {
        walk(&b, 0, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn walk(dir: &Path, depth: u32, out: &mut Vec<PathBuf>) {
    if depth > 4 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, depth + 1, out);
        } else if matches!(p.extension().and_then(|e| e.to_str()).map(|e| e.to_ascii_lowercase()).as_deref(), Some("ttf" | "otf" | "ttc")) {
            out.push(p);
        }
    }
}

/// Family name from a face's name table, strongly preferring English
/// (UI fonts here cannot draw CJK names).
fn family_name(face: &ttf_parser::Face) -> Option<String> {
    // priority: windows en-US > any windows English > unicode platform > anything
    let mut best: Option<(u8, String)> = None;
    for n in face.names() {
        let id = n.name_id;
        if id != ttf_parser::name_id::FAMILY && id != ttf_parser::name_id::TYPOGRAPHIC_FAMILY {
            continue;
        }
        let Some(s) = n.to_string() else { continue };
        if s.trim().is_empty() {
            continue;
        }
        let lang_rank = match n.platform_id {
            ttf_parser::PlatformId::Windows if n.language_id == 0x0409 => 0,
            ttf_parser::PlatformId::Windows if n.language_id & 0xFF == 0x09 => 1,
            ttf_parser::PlatformId::Unicode => 2,
            _ if n.language_id == 0 => 2,
            _ => 3,
        };
        // typographic family wins over plain family at the same language rank
        let rank = lang_rank * 2 + if id == ttf_parser::name_id::TYPOGRAPHIC_FAMILY { 0 } else { 1 };
        if best.as_ref().is_none_or(|(r, _)| rank < *r) {
            best = Some((rank, s));
        }
    }
    best.map(|(_, s)| s)
}

/// True when a name can be shown by the UI font (has some ASCII letters).
fn displayable(name: &str) -> bool {
    name.chars().any(|c| c.is_ascii_alphanumeric())
}

enum Source {
    /// Font file and face index (for .ttc collections).
    Path(PathBuf, u32),
    Static(&'static [u8]),
}

pub struct FontFace {
    pub name: String,
    sources: [Option<Source>; 4],
    loaded: [OnceCell<Option<FontArc>>; 4],
}

impl FontFace {
    fn get(&self, variant: usize) -> Option<FontArc> {
        self.loaded[variant]
            .get_or_init(|| match self.sources[variant].as_ref()? {
                Source::Path(p, i) => ab_glyph::FontVec::try_from_vec_and_index(std::fs::read(p).ok()?, *i).ok().map(FontArc::from),
                Source::Static(b) => FontArc::try_from_slice(b).ok(),
            })
            .clone()
    }
}

pub struct FontLib {
    pub faces: Vec<FontFace>,
    pub default_face: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextStyle {
    pub face: usize,
    pub size_px: f32,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub strike: bool,
}

pub struct Line {
    pub start: usize,
    pub end: usize,
    /// Caret x positions for every char boundary from `start` to `end`.
    pub xs: Vec<f32>,
    /// Glyph drawing positions (x) for chars start..end.
    gx: Vec<f32>,
    pub y: f32,
}

pub struct Layout {
    pub lines: Vec<Line>,
    pub line_h: f32,
    pub height: f32,
}

impl Layout {
    /// Line index and x position of a caret.
    pub fn caret_pos(&self, idx: usize) -> (usize, f32) {
        let mut li = 0;
        for (i, l) in self.lines.iter().enumerate() {
            if l.start <= idx {
                li = i;
            }
        }
        let l = &self.lines[li];
        let k = idx.saturating_sub(l.start).min(l.xs.len() - 1);
        (li, l.xs[k])
    }

    /// Nearest char index to a local point.
    pub fn hit(&self, x: f32, y: f32) -> usize {
        let li = ((y / self.line_h).floor().max(0.0) as usize).min(self.lines.len() - 1);
        let l = &self.lines[li];
        let mut best = l.start;
        let mut best_d = f32::MAX;
        for (k, cx) in l.xs.iter().enumerate() {
            let d = (cx - x).abs();
            if d < best_d {
                best_d = d;
                best = l.start + k;
            }
        }
        best.min(l.end)
    }
}

impl FontLib {
    pub fn load() -> Self {
        let dirs = font_dirs();
        let find = |names: &[&str]| -> Option<Source> {
            for d in &dirs {
                for n in names {
                    let p = d.join(n);
                    if p.is_file() {
                        return Some(Source::Path(p, 0));
                    }
                }
            }
            None
        };
        let mut faces = Vec::new();
        for (name, variants) in FAMILIES {
            let regular = find(variants[0]);
            if regular.is_none() {
                continue;
            }
            faces.push(FontFace {
                name: name.to_string(),
                sources: [regular, find(variants[1]), find(variants[2]), find(variants[3])],
                loaded: Default::default(),
            });
        }
        faces.push(FontFace {
            name: "Selawik".into(),
            sources: [Some(Source::Static(UI_FONT)), Some(Source::Static(UI_FONT_BOLD)), None, None],
            loaded: Default::default(),
        });
        faces.sort_by(|a, b| a.name.cmp(&b.name));
        let default_face = ["Calibri", "Arial", "Liberation Sans", "DejaVu Sans", "Noto Sans", "Selawik"]
            .iter()
            .find_map(|n| faces.iter().position(|f| f.name == *n))
            .unwrap_or(0);
        Self { faces, default_face }
    }

    /// Every installed font family, discovered by reading each font file's
    /// name table. Slow-ish (it opens every font file), so it runs on a
    /// background thread; the curated `load()` list is used until it is done.
    pub fn scan_system() -> Self {
        let mut families: HashMap<String, [Option<Source>; 4]> = HashMap::new();
        for file in font_files() {
            let Ok(data) = std::fs::read(&file) else { continue };
            let count = ttf_parser::fonts_in_collection(&data).unwrap_or(1);
            for index in 0..count {
                let Ok(face) = ttf_parser::Face::parse(&data, index) else { continue };
                // outline fonts only (skips bitmap and emoji fonts)
                if face.tables().glyf.is_none() && face.tables().cff.is_none() {
                    continue;
                }
                let mut name = family_name(&face).unwrap_or_default();
                // a font with no readable (Latin) name is listed by its file name
                if !displayable(&name) {
                    let stem = file.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    name = if count > 1 { format!("{stem} {}", index + 1) } else { stem };
                }
                // hidden system families start with a dot
                if name.is_empty() || name.starts_with('.') || !displayable(&name) {
                    continue;
                }
                let slot = (face.is_bold() as usize) + ((face.is_italic() as usize) << 1);
                let entry = families.entry(name).or_default();
                if entry[slot].is_none() {
                    entry[slot] = Some(Source::Path(file.clone(), index));
                }
            }
        }
        // keep the curated list as a fallback for families the scan missed
        let basic = Self::load();
        let mut faces: Vec<FontFace> = families
            .into_iter()
            .map(|(name, sources)| FontFace { name, sources, loaded: Default::default() })
            .collect();
        for b in basic.faces {
            if !faces.iter().any(|f| f.name == b.name) {
                faces.push(b);
            }
        }
        faces.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        let default_face = ["Calibri", "Arial", "Helvetica", "Liberation Sans", "DejaVu Sans", "Noto Sans", "Selawik"]
            .iter()
            .find_map(|n| faces.iter().position(|f| f.name == *n))
            .unwrap_or(0);
        Self { faces, default_face }
    }

    /// Run the full system scan on a background thread.
    pub fn spawn_scan() -> std::sync::mpsc::Receiver<FontLib> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(FontLib::scan_system());
        });
        rx
    }

    /// Returns the font to use plus whether bold / italic must be synthesized.
    fn resolve(&self, st: &TextStyle) -> (FontArc, bool, bool) {
        let face = &self.faces[st.face.min(self.faces.len() - 1)];
        let want = match (st.bold, st.italic) {
            (false, false) => 0,
            (true, false) => 1,
            (false, true) => 2,
            (true, true) => 3,
        };
        if let Some(f) = face.get(want) {
            return (f, false, false);
        }
        if want == 3 {
            if let Some(f) = face.get(1) {
                return (f, false, true);
            }
            if let Some(f) = face.get(2) {
                return (f, true, false);
            }
        }
        let regular = face.get(0).or_else(|| FontArc::try_from_slice(UI_FONT).ok()).expect("bundled font");
        (regular, st.bold, st.italic)
    }

    pub fn layout(&self, st: &TextStyle, text: &str, max_w: f32) -> Layout {
        let (font, faux_bold, _) = self.resolve(st);
        let sf = font.as_scaled(PxScale::from(st.size_px));
        let line_h = (sf.ascent() - sf.descent() + sf.line_gap()).ceil().max(1.0);
        let extra = if faux_bold { (st.size_px / 16.0).round().max(1.0) } else { 0.0 };
        let chars: Vec<char> = text.chars().collect();
        let mut lines = Vec::new();
        let mut start = 0usize;
        loop {
            let mut xs = vec![0.0f32];
            let mut gx = Vec::new();
            let mut x = 0.0f32;
            let mut last_space: Option<usize> = None;
            let mut prev: Option<GlyphId> = None;
            let mut j = start;
            let (end, next_start);
            loop {
                if j >= chars.len() {
                    end = j;
                    next_start = j + 1;
                    break;
                }
                let c = chars[j];
                if c == '\n' {
                    end = j;
                    next_start = j + 1;
                    break;
                }
                let gid = font.glyph_id(c);
                let kern = prev.map(|p| sf.kern(p, gid)).unwrap_or(0.0);
                let adv = sf.h_advance(gid) + extra;
                let nx = x + kern + adv;
                if nx > max_w && j > start && c != ' ' {
                    if let Some(sp) = last_space {
                        end = sp + 1;
                        next_start = sp + 1;
                        xs.truncate(end - start + 1);
                        gx.truncate(end - start);
                    } else {
                        end = j;
                        next_start = j;
                    }
                    break;
                }
                gx.push(x + kern);
                x = nx;
                xs.push(x);
                if c == ' ' || c == '\t' {
                    last_space = Some(j);
                }
                prev = Some(gid);
                j += 1;
            }
            let y = lines.len() as f32 * line_h;
            lines.push(Line { start, end, xs, gx, y });
            if next_start > chars.len() {
                break;
            }
            start = next_start;
        }
        let height = lines.len() as f32 * line_h;
        Layout { lines, line_h, height }
    }

    /// Draw `text` with its top-left at `origin`, wrapping at `max_w`, clipped to `clip`.
    #[allow(clippy::too_many_arguments)]
    pub fn render(&self, pix: &mut Pixmap, st: &TextStyle, text: &str, origin: (f32, f32), max_w: f32, color: Color32, clip: IRect) {
        let bounds = IRect::new(0, 0, pix.width() as i32, pix.height() as i32);
        let Some(clip) = clip.intersect(&bounds) else { return };
        let (font, faux_bold, faux_italic) = self.resolve(st);
        let layout = self.layout(st, text, max_w);
        let scale = PxScale::from(st.size_px);
        let sf = font.as_scaled(scale);
        let (cw, ch) = (clip.w as usize, clip.h as usize);
        let mut cov = vec![0f32; cw * ch];
        let mut plot = |x: i32, y: i32, c: f32| {
            let lx = x - clip.x;
            let ly = y - clip.y;
            if lx >= 0 && ly >= 0 && (lx as usize) < cw && (ly as usize) < ch {
                let i = ly as usize * cw + lx as usize;
                if c > cov[i] {
                    cov[i] = c;
                }
            }
        };
        let chars: Vec<char> = text.chars().collect();
        for line in &layout.lines {
            let baseline = origin.1 + line.y + sf.ascent();
            for (k, ci) in (line.start..line.end).enumerate() {
                let c = chars[ci];
                if c.is_whitespace() {
                    continue;
                }
                let gid = font.glyph_id(c);
                let glyph = gid.with_scale_and_position(scale, point(origin.0 + line.gx[k], baseline));
                if let Some(og) = font.outline_glyph(glyph) {
                    let bb = og.px_bounds();
                    og.draw(|gx, gy, v| {
                        let px = bb.min.x as i32 + gx as i32;
                        let py = bb.min.y as i32 + gy as i32;
                        let shift = if faux_italic { ((baseline - py as f32) * 0.2).round() as i32 } else { 0 };
                        plot(px + shift, py, v);
                    });
                }
            }
            let width = line.xs.last().copied().unwrap_or(0.0);
            let thick = (st.size_px / 15.0).round().max(1.0) as i32;
            let mut bar = |y: f32| {
                let y0 = y.round() as i32;
                for yy in y0..y0 + thick {
                    for xx in (origin.0.round() as i32)..((origin.0 + width).round() as i32) {
                        plot(xx, yy, 1.0);
                    }
                }
            };
            if st.underline && line.end > line.start {
                bar(baseline + (st.size_px * 0.1).max(1.0));
            }
            if st.strike && line.end > line.start {
                bar(baseline - sf.ascent() * 0.32);
            }
        }
        if faux_bold {
            let k = (st.size_px / 16.0).round().max(1.0) as usize;
            for row in cov.chunks_exact_mut(cw) {
                for x in (0..cw).rev() {
                    let mut m = row[x];
                    for d in 1..=k {
                        if x >= d {
                            m = m.max(row[x - d]);
                        }
                    }
                    row[x] = m;
                }
            }
        }
        // composite
        let pw = pix.width() as usize;
        let data = pix.data_mut();
        let (r, g, b) = (color.r() as f32, color.g() as f32, color.b() as f32);
        for ly in 0..ch {
            for lx in 0..cw {
                let a = cov[ly * cw + lx].clamp(0.0, 1.0);
                if a <= 0.0 {
                    continue;
                }
                let i = ((clip.y as usize + ly) * pw + clip.x as usize + lx) * 4;
                let inv = 1.0 - a;
                data[i] = (r * a + data[i] as f32 * inv).round() as u8;
                data[i + 1] = (g * a + data[i + 1] as f32 * inv).round() as u8;
                data[i + 2] = (b * a + data[i + 2] as f32 * inv).round() as u8;
                data[i + 3] = (255.0 * a + data[i + 3] as f32 * inv).round() as u8;
            }
        }
    }
}

/// Font sizes offered in the ribbon (points).
pub const FONT_SIZES: [u32; 16] = [8, 9, 10, 11, 12, 14, 16, 18, 20, 22, 24, 26, 28, 36, 48, 72];

pub fn pt_to_px(pt: f32) -> f32 {
    pt * 96.0 / 72.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_scan_finds_more_than_the_curated_list() {
        let t = std::time::Instant::now();
        let full = FontLib::scan_system();
        let basic = FontLib::load();
        println!("scan: {} families (curated {}) in {:?}", full.faces.len(), basic.faces.len(), t.elapsed());
        assert!(full.faces.len() >= basic.faces.len());
        // every discovered face must produce a usable regular font
        let arial = full.faces.iter().position(|f| f.name == "Arial" || f.name == "Helvetica" || f.name == "DejaVu Sans");
        assert!(arial.is_some(), "no common family found");
        assert!(full.faces.iter().all(|f| !f.name.starts_with('.')));
        // every listed name must be drawable by the UI font
        let bad: Vec<&str> = full.faces.iter().filter(|f| !f.name.chars().any(|c| c.is_ascii_alphanumeric())).map(|f| f.name.as_str()).collect();
        assert!(bad.is_empty(), "undisplayable names: {bad:?}");
        let nonascii: Vec<&str> = full.faces.iter().filter(|f| !f.name.is_ascii()).map(|f| f.name.as_str()).collect();
        println!("names with non-ASCII chars: {nonascii:?}");
    }

    #[test]
    fn layout_wraps_and_carets_are_monotonic() {
        let lib = FontLib::load();
        let face = lib.faces.iter().position(|f| f.name == "Selawik").unwrap();
        let st = TextStyle { face, size_px: 16.0, bold: false, italic: false, underline: false, strike: false };
        let lay = lib.layout(&st, "hello world this wraps\nnext", 60.0);
        assert!(lay.lines.len() >= 3);
        for l in &lay.lines {
            assert!(l.xs.windows(2).all(|w| w[1] >= w[0]));
        }
        let (li, _) = lay.caret_pos(lay.lines.last().unwrap().start);
        assert_eq!(li, lay.lines.len() - 1);
    }
}

