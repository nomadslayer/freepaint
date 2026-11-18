//! Free-hand drawing: pencil, eraser and the nine brush styles.
//!
//! All strokes are drawn into a transparent "stroke layer" which is
//! composited onto the canvas (with the brush's opacity) every frame and
//! merged on mouse release. Writing with "max alpha" means overlapping
//! parts of the same stroke do not get darker, like the real thing.

use egui::Color32;
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::raster::{grain, sk_color, Rng};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BrushKind {
    Brush,
    Calligraphy1,
    Calligraphy2,
    Airbrush,
    Oil,
    Crayon,
    Marker,
    NaturalPencil,
    Watercolor,
}

impl BrushKind {
    pub const ALL: [BrushKind; 9] = [
        BrushKind::Brush,
        BrushKind::Calligraphy1,
        BrushKind::Calligraphy2,
        BrushKind::Airbrush,
        BrushKind::Oil,
        BrushKind::Crayon,
        BrushKind::Marker,
        BrushKind::NaturalPencil,
        BrushKind::Watercolor,
    ];

    pub fn name(self) -> &'static str {
        match self {
            BrushKind::Brush => "Brush",
            BrushKind::Calligraphy1 => "Calligraphy brush 1",
            BrushKind::Calligraphy2 => "Calligraphy brush 2",
            BrushKind::Airbrush => "Airbrush",
            BrushKind::Oil => "Oil brush",
            BrushKind::Crayon => "Crayon",
            BrushKind::Marker => "Marker",
            BrushKind::NaturalPencil => "Natural pencil",
            BrushKind::Watercolor => "Watercolor brush",
        }
    }

    /// The four widths offered by the Size menu for this brush.
    pub fn sizes(self) -> [f32; 4] {
        match self {
            BrushKind::Brush => [3.0, 6.0, 10.0, 15.0],
            BrushKind::Calligraphy1 | BrushKind::Calligraphy2 => [6.0, 10.0, 15.0, 20.0],
            BrushKind::Airbrush => [10.0, 20.0, 30.0, 40.0],
            BrushKind::Oil => [8.0, 14.0, 20.0, 28.0],
            BrushKind::Crayon => [6.0, 10.0, 15.0, 22.0],
            BrushKind::Marker => [6.0, 10.0, 15.0, 22.0],
            BrushKind::NaturalPencil => [2.0, 4.0, 6.0, 8.0],
            BrushKind::Watercolor => [8.0, 14.0, 20.0, 28.0],
        }
    }

    /// Opacity used when compositing the stroke layer.
    pub fn opacity(self) -> f32 {
        match self {
            BrushKind::Marker => 0.55,
            BrushKind::Watercolor => 0.55,
            _ => 1.0,
        }
    }

    pub fn continuous(self) -> bool {
        self == BrushKind::Airbrush
    }
}

#[inline]
fn plot_max(layer: &mut Pixmap, x: i32, y: i32, c: Color32, a: u8) {
    if x < 0 || y < 0 || x >= layer.width() as i32 || y >= layer.height() as i32 || a == 0 {
        return;
    }
    let i = ((y as u32 * layer.width() + x as u32) * 4) as usize;
    let d = layer.data_mut();
    if a > d[i + 3] {
        let a32 = a as u32;
        d[i] = ((c.r() as u32 * a32 + 127) / 255) as u8;
        d[i + 1] = ((c.g() as u32 * a32 + 127) / 255) as u8;
        d[i + 2] = ((c.b() as u32 * a32 + 127) / 255) as u8;
        d[i + 3] = a;
    }
}

#[inline]
fn put(layer: &mut Pixmap, x: i32, y: i32, c: [u8; 4]) {
    if x < 0 || y < 0 || x >= layer.width() as i32 || y >= layer.height() as i32 {
        return;
    }
    let i = ((y as u32 * layer.width() + x as u32) * 4) as usize;
    layer.data_mut()[i..i + 4].copy_from_slice(&c);
}

pub fn bresenham(a: (i32, i32), b: (i32, i32), mut f: impl FnMut(i32, i32)) {
    let (mut x0, mut y0) = a;
    let (x1, y1) = b;
    let dx = (x1 - x0).abs();
    let sx = if x0 < x1 { 1 } else { -1 };
    let dy = -(y1 - y0).abs();
    let sy = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        f(x0, y0);
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x0 += sx;
        }
        if e2 <= dx {
            err += dx;
            y0 += sy;
        }
    }
}

/// Aliased pencil line with a square nib.
pub fn pencil_line(layer: &mut Pixmap, a: (i32, i32), b: (i32, i32), size: i32, color: Color32) {
    let c = [color.r(), color.g(), color.b(), 255];
    let s = size.max(1);
    let off = (s - 1) / 2;
    bresenham(a, b, |x, y| {
        for yy in 0..s {
            for xx in 0..s {
                put(layer, x - off + xx, y - off + yy, c);
            }
        }
    });
}

/// Square eraser. With `only` set, it is a "color eraser": only pixels of
/// that color in `base` are replaced.
pub fn eraser_line(layer: &mut Pixmap, base: &Pixmap, a: (i32, i32), b: (i32, i32), size: i32, c: [u8; 4], only: Option<Color32>) {
    let s = size.max(1);
    let off = s / 2;
    let bw = base.width() as i32;
    let bh = base.height() as i32;
    let bd = base.data();
    bresenham(a, b, |x, y| {
        for yy in 0..s {
            for xx in 0..s {
                let px = x - off + xx;
                let py = y - off + yy;
                if let Some(o) = only {
                    if px < 0 || py < 0 || px >= bw || py >= bh {
                        continue;
                    }
                    let i = ((py * bw + px) * 4) as usize;
                    if bd[i] != o.r() || bd[i + 1] != o.g() || bd[i + 2] != o.b() {
                        continue;
                    }
                }
                put(layer, px, py, c);
            }
        }
    });
}

fn dist_to_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> f32 {
    let (vx, vy) = (b.0 - a.0, b.1 - a.1);
    let len2 = vx * vx + vy * vy;
    let t = if len2 > 0.0 { (((p.0 - a.0) * vx + (p.1 - a.1) * vy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (qx, qy) = (a.0 + t * vx, a.1 + t * vy);
    ((p.0 - qx).powi(2) + (p.1 - qy).powi(2)).sqrt()
}

/// Rasterize a capsule (thick segment) calling `alpha(x, y, d)` where d is the
/// normalized distance from the centre line (0..1).
fn capsule(layer: &mut Pixmap, a: (f32, f32), b: (f32, f32), r: f32, color: Color32, mut alpha: impl FnMut(i32, i32, f32) -> u8) {
    let r = r.max(0.5);
    let x0 = (a.0.min(b.0) - r - 1.0).floor() as i32;
    let x1 = (a.0.max(b.0) + r + 1.0).ceil() as i32;
    let y0 = (a.1.min(b.1) - r - 1.0).floor() as i32;
    let y1 = (a.1.max(b.1) + r + 1.0).ceil() as i32;
    let x0 = x0.max(0);
    let y0 = y0.max(0);
    let x1 = x1.min(layer.width() as i32 - 1);
    let y1 = y1.min(layer.height() as i32 - 1);
    for y in y0..=y1 {
        for x in x0..=x1 {
            let d = dist_to_segment((x as f32 + 0.5, y as f32 + 0.5), a, b);
            if d <= r {
                let al = alpha(x, y, d / r);
                plot_max(layer, x, y, color, al);
            }
        }
    }
}

struct Bristle {
    off: f32,
    alpha: f32,
    width: f32,
    shade: f32,
}

pub struct BrushStroke {
    pub kind: BrushKind,
    pub color: Color32,
    pub size: f32,
    rng: Rng,
    bristles: Vec<Bristle>,
    travelled: f32,
}

impl BrushStroke {
    pub fn new(kind: BrushKind, color: Color32, size: f32, seed: u64) -> Self {
        let mut rng = Rng::new(seed);
        let mut bristles = Vec::new();
        if kind == BrushKind::Oil {
            let n = ((size / 2.0).round() as usize).max(4);
            for i in 0..n {
                let base = -0.5 + (i as f32 + 0.5) / n as f32;
                bristles.push(Bristle {
                    off: (base + (rng.f32() - 0.5) * 0.08) * size,
                    alpha: 0.55 + rng.f32() * 0.45,
                    width: (size / n as f32) * (1.2 + rng.f32() * 0.8),
                    shade: 0.8 + rng.f32() * 0.35,
                });
            }
        }
        Self { kind, color, size, rng, bristles, travelled: 0.0 }
    }

    fn shaded(&self, shade: f32) -> Color32 {
        let f = |c: u8| ((c as f32 * shade).clamp(0.0, 255.0)) as u8;
        Color32::from_rgb(f(self.color.r()), f(self.color.g()), f(self.color.b()))
    }

    /// First contact at a single point.
    pub fn dab(&mut self, layer: &mut Pixmap, p: (f32, f32)) {
        match self.kind {
            BrushKind::Airbrush => self.spray(layer, p, 1.0),
            _ => self.segment(layer, p, (p.0 + 0.01, p.1 + 0.01)),
        }
    }

    pub fn segment(&mut self, layer: &mut Pixmap, a: (f32, f32), b: (f32, f32)) {
        let s = self.size;
        let color = self.color;
        match self.kind {
            BrushKind::Brush | BrushKind::Marker => {
                stroke_line(layer, a, b, s, color, LineCap::Round);
            }
            BrushKind::Calligraphy1 | BrushKind::Calligraphy2 => {
                let d = std::f32::consts::FRAC_1_SQRT_2 * s / 2.0;
                let n = if self.kind == BrushKind::Calligraphy1 { (d, -d) } else { (d, d) };
                let mut pb = PathBuilder::new();
                pb.move_to(a.0 + n.0, a.1 + n.1);
                pb.line_to(b.0 + n.0, b.1 + n.1);
                pb.line_to(b.0 - n.0, b.1 - n.1);
                pb.line_to(a.0 - n.0, a.1 - n.1);
                pb.close();
                let mut paint = Paint::default();
                paint.set_color(sk_color(color));
                paint.anti_alias = true;
                if let Some(path) = pb.finish() {
                    layer.fill_path(&path, &paint, FillRule::Winding, Transform::identity(), None);
                }
                // make sure the nib itself is always visible
                stroke_line(layer, (a.0 - n.0, a.1 - n.1), (a.0 + n.0, a.1 + n.1), 1.2, color, LineCap::Butt);
                stroke_line(layer, (b.0 - n.0, b.1 - n.1), (b.0 + n.0, b.1 + n.1), 1.2, color, LineCap::Butt);
            }
            BrushKind::Airbrush => {
                let len = ((b.0 - a.0).powi(2) + (b.1 - a.1).powi(2)).sqrt();
                let step = (s / 4.0).max(1.0);
                let n = (len / step).ceil().max(1.0) as usize;
                for i in 1..=n {
                    let t = i as f32 / n as f32;
                    self.spray(layer, (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t), 0.35);
                }
            }
            BrushKind::Oil => {
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len = (dx * dx + dy * dy).sqrt().max(0.001);
                let (px, py) = (-dy / len, dx / len);
                self.travelled += len;
                // the brush slowly runs dry over a long stroke
                let dry = (1.0 - self.travelled / 2500.0).clamp(0.45, 1.0);
                for i in 0..self.bristles.len() {
                    let br = &self.bristles[i];
                    let (o, w, al, sh) = (br.off, br.width, br.alpha * dry, br.shade);
                    let c = self.shaded(sh);
                    let pa = (a.0 + px * o, a.1 + py * o);
                    let pb = (b.0 + px * o, b.1 + py * o);
                    capsule(layer, pa, pb, w / 2.0, c, |x, y, d| {
                        let g = grain(x, y) as f32 / 255.0;
                        let edge = (1.0 - d).clamp(0.0, 1.0).sqrt();
                        (al * edge * (0.75 + 0.25 * g) * 255.0) as u8
                    });
                }
            }
            BrushKind::Crayon => {
                capsule(layer, a, b, s / 2.0, color, |x, y, d| {
                    let g = grain(x, y) as i32;
                    let g2 = grain(x / 2 + 7919, y / 2 + 104_729) as i32;
                    let v = (g + g2) / 2;
                    let thresh = 70 + (d * 90.0) as i32;
                    if v > thresh {
                        (130 + (v - thresh) * 2).min(255) as u8
                    } else {
                        0
                    }
                });
            }
            BrushKind::NaturalPencil => {
                capsule(layer, a, b, (s / 2.0).max(0.8), color, |x, y, d| {
                    let g = grain(x, y) as f32 / 255.0;
                    ((0.45 + 0.5 * g) * (1.0 - d * 0.6) * 255.0) as u8
                });
            }
            BrushKind::Watercolor => {
                capsule(layer, a, b, s / 2.0, color, |x, y, d| {
                    let g = grain(x / 3, y / 3) as f32 / 255.0;
                    let edge = if d > 0.82 { 1.0 } else { 0.55 + 0.25 * d };
                    ((edge * (0.8 + 0.2 * g)) * 255.0) as u8
                });
            }
        }
    }

    /// Airbrush spray at a point. `amount` scales the number of dots.
    pub fn spray(&mut self, layer: &mut Pixmap, p: (f32, f32), amount: f32) {
        let r = self.size / 2.0;
        let n = ((r * r * 0.12 + 4.0) * amount).ceil() as usize;
        let c = [self.color.r(), self.color.g(), self.color.b(), 255];
        for _ in 0..n {
            loop {
                let dx = self.rng.f32() * 2.0 - 1.0;
                let dy = self.rng.f32() * 2.0 - 1.0;
                if dx * dx + dy * dy <= 1.0 {
                    put(layer, (p.0 + dx * r).floor() as i32, (p.1 + dy * r).floor() as i32, c);
                    break;
                }
            }
        }
    }
}

fn stroke_line(layer: &mut Pixmap, a: (f32, f32), b: (f32, f32), width: f32, color: Color32, cap: LineCap) {
    let mut pb = PathBuilder::new();
    pb.move_to(a.0, a.1);
    pb.line_to(b.0, b.1);
    let Some(path) = pb.finish() else { return };
    let mut paint = Paint::default();
    paint.set_color(sk_color(color));
    paint.anti_alias = true;
    let stroke = Stroke { width, line_cap: cap, line_join: LineJoin::Round, ..Default::default() };
    layer.stroke_path(&path, &paint, &stroke, Transform::identity(), None);
}

/// Render a small sample stroke for the brushes menu.
pub fn preview(kind: BrushKind, w: u32, h: u32) -> Pixmap {
    preview_colored(kind, w, h, Color32::BLACK)
}

pub fn preview_colored(kind: BrushKind, w: u32, h: u32, color: Color32) -> Pixmap {
    let mut layer = Pixmap::new(w, h).unwrap();
    let size = (kind.sizes()[1] * 0.6).max(2.0);
    let mut stroke = BrushStroke::new(kind, color, size, 7);
    let n = 40;
    let mut prev = None;
    for i in 0..=n {
        let t = i as f32 / n as f32;
        let x = w as f32 * (0.18 + 0.64 * t);
        let y = h as f32 * (0.5 - 0.22 * (t * std::f32::consts::TAU).sin());
        if let Some(p) = prev {
            stroke.segment(&mut layer, p, (x, y));
        }
        prev = Some((x, y));
    }
    if kind.opacity() < 1.0 {
        let mut out = Pixmap::new(w, h).unwrap();
        let paint = tiny_skia::PixmapPaint { opacity: kind.opacity(), ..Default::default() };
        out.draw_pixmap(0, 0, layer.as_ref(), &paint, Transform::identity(), None);
        out
    } else {
        layer
    }
}
