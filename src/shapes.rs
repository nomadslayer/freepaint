//! Geometry for the shapes gallery. The same path data is used for drawing
//! on the canvas (via tiny-skia) and for the little gallery icons (via egui).

use std::f32::consts::PI;

use egui::{Color32, Pos2};
use tiny_skia::{FillRule, LineCap, LineJoin, Paint, PathBuilder, Pixmap, Stroke, Transform};

use crate::raster::sk_color;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ShapeKind {
    Line,
    Curve,
    Oval,
    Rectangle,
    RoundedRectangle,
    Polygon,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    RightArrow,
    LeftArrow,
    UpArrow,
    DownArrow,
    Star4,
    Star5,
    Star6,
    RoundedCallout,
    OvalCallout,
    CloudCallout,
    Heart,
    Lightning,
}

impl ShapeKind {
    pub const ALL: [ShapeKind; 23] = [
        ShapeKind::Line,
        ShapeKind::Curve,
        ShapeKind::Oval,
        ShapeKind::Rectangle,
        ShapeKind::RoundedRectangle,
        ShapeKind::Polygon,
        ShapeKind::Triangle,
        ShapeKind::RightTriangle,
        ShapeKind::Diamond,
        ShapeKind::Pentagon,
        ShapeKind::Hexagon,
        ShapeKind::RightArrow,
        ShapeKind::LeftArrow,
        ShapeKind::UpArrow,
        ShapeKind::DownArrow,
        ShapeKind::Star4,
        ShapeKind::Star5,
        ShapeKind::Star6,
        ShapeKind::RoundedCallout,
        ShapeKind::OvalCallout,
        ShapeKind::CloudCallout,
        ShapeKind::Heart,
        ShapeKind::Lightning,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ShapeKind::Line => "Line",
            ShapeKind::Curve => "Curve",
            ShapeKind::Oval => "Oval",
            ShapeKind::Rectangle => "Rectangle",
            ShapeKind::RoundedRectangle => "Rounded rectangle",
            ShapeKind::Polygon => "Polygon",
            ShapeKind::Triangle => "Triangle",
            ShapeKind::RightTriangle => "Right triangle",
            ShapeKind::Diamond => "Diamond",
            ShapeKind::Pentagon => "Pentagon",
            ShapeKind::Hexagon => "Hexagon",
            ShapeKind::RightArrow => "Right arrow",
            ShapeKind::LeftArrow => "Left arrow",
            ShapeKind::UpArrow => "Up arrow",
            ShapeKind::DownArrow => "Down arrow",
            ShapeKind::Star4 => "Four-point star",
            ShapeKind::Star5 => "Five-point star",
            ShapeKind::Star6 => "Six-point star",
            ShapeKind::RoundedCallout => "Rounded rectangular callout",
            ShapeKind::OvalCallout => "Oval callout",
            ShapeKind::CloudCallout => "Cloud callout",
            ShapeKind::Heart => "Heart",
            ShapeKind::Lightning => "Lightning",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum Seg {
    M(f32, f32),
    L(f32, f32),
    C(f32, f32, f32, f32, f32, f32),
    Z,
}

pub type PathData = Vec<Seg>;

const K: f32 = 0.552_284_8;

fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32) -> PathData {
    use Seg::*;
    vec![
        M(cx + rx, cy),
        C(cx + rx, cy + ry * K, cx + rx * K, cy + ry, cx, cy + ry),
        C(cx - rx * K, cy + ry, cx - rx, cy + ry * K, cx - rx, cy),
        C(cx - rx, cy - ry * K, cx - rx * K, cy - ry, cx, cy - ry),
        C(cx + rx * K, cy - ry, cx + rx, cy - ry * K, cx + rx, cy),
        Z,
    ]
}

fn polygon(pts: &[(f32, f32)]) -> PathData {
    let mut out = Vec::with_capacity(pts.len() + 2);
    for (i, p) in pts.iter().enumerate() {
        out.push(if i == 0 { Seg::M(p.0, p.1) } else { Seg::L(p.0, p.1) });
    }
    out.push(Seg::Z);
    out
}

fn unit_poly(pts: &[(f32, f32)], x0: f32, y0: f32, w: f32, h: f32) -> PathData {
    let mapped: Vec<(f32, f32)> = pts.iter().map(|(u, v)| (x0 + u * w, y0 + v * h)).collect();
    polygon(&mapped)
}

/// Regular star / polygon points normalized so that they fill the unit box.
fn star_unit(points: usize, inner: Option<f32>) -> Vec<(f32, f32)> {
    let mut raw = Vec::new();
    let steps = if inner.is_some() { points * 2 } else { points };
    for i in 0..steps {
        let r = match inner {
            Some(ir) if i % 2 == 1 => ir,
            _ => 1.0,
        };
        let a = -PI / 2.0 + i as f32 * 2.0 * PI / steps as f32;
        raw.push((r * a.cos(), r * a.sin()));
    }
    let minx = raw.iter().map(|p| p.0).fold(f32::MAX, f32::min);
    let maxx = raw.iter().map(|p| p.0).fold(f32::MIN, f32::max);
    let miny = raw.iter().map(|p| p.1).fold(f32::MAX, f32::min);
    let maxy = raw.iter().map(|p| p.1).fold(f32::MIN, f32::max);
    raw.iter().map(|(x, y)| ((x - minx) / (maxx - minx), (y - miny) / (maxy - miny))).collect()
}

fn rounded_rect_segs(x0: f32, y0: f32, x1: f32, y1: f32, r: f32, bottom_tail: Option<[(f32, f32); 3]>) -> PathData {
    use Seg::*;
    let r = r.min((x1 - x0) / 2.0).min((y1 - y0) / 2.0).max(0.0);
    let k = r * (1.0 - K);
    let mut v = vec![
        M(x0 + r, y0),
        L(x1 - r, y0),
        C(x1 - k, y0, x1, y0 + k, x1, y0 + r),
        L(x1, y1 - r),
        C(x1, y1 - k, x1 - k, y1, x1 - r, y1),
    ];
    if let Some(t) = bottom_tail {
        // tail points listed right-to-left along the bottom edge: entry, tip, exit
        for p in t {
            v.push(L(p.0, p.1));
        }
    }
    v.extend([L(x0 + r, y1), C(x0 + k, y1, x0, y1 - k, x0, y1 - r), L(x0, y0 + r), C(x0, y0 + k, x0 + k, y0, x0 + r, y0), Z]);
    v
}

/// Path for a box-defined shape spanning the two corner points.
pub fn box_path(kind: ShapeKind, a: (f32, f32), b: (f32, f32)) -> PathData {
    use Seg::*;
    let (x0, x1) = (a.0.min(b.0), a.0.max(b.0));
    let (y0, y1) = (a.1.min(b.1), a.1.max(b.1));
    let (w, h) = (x1 - x0, y1 - y0);
    let (cx, cy) = (x0 + w / 2.0, y0 + h / 2.0);
    match kind {
        ShapeKind::Line | ShapeKind::Curve | ShapeKind::Polygon => vec![M(a.0, a.1), L(b.0, b.1)],
        ShapeKind::Oval => ellipse(cx, cy, w / 2.0, h / 2.0),
        ShapeKind::Rectangle => polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)]),
        ShapeKind::RoundedRectangle => rounded_rect_segs(x0, y0, x1, y1, w.min(h) * 0.17, None),
        ShapeKind::Triangle => unit_poly(&[(0.5, 0.0), (1.0, 1.0), (0.0, 1.0)], x0, y0, w, h),
        ShapeKind::RightTriangle => unit_poly(&[(0.0, 0.0), (1.0, 1.0), (0.0, 1.0)], x0, y0, w, h),
        ShapeKind::Diamond => unit_poly(&[(0.5, 0.0), (1.0, 0.5), (0.5, 1.0), (0.0, 0.5)], x0, y0, w, h),
        ShapeKind::Pentagon => unit_poly(&star_unit(5, None), x0, y0, w, h),
        ShapeKind::Hexagon => unit_poly(&[(0.25, 0.0), (0.75, 0.0), (1.0, 0.5), (0.75, 1.0), (0.25, 1.0), (0.0, 0.5)], x0, y0, w, h),
        ShapeKind::RightArrow | ShapeKind::LeftArrow => {
            let hl = (h / 2.0).min(w);
            let t = h / 4.0;
            let pts = [(0.0, cy - t), (w - hl, cy - t), (w - hl, y0), (w, cy), (w - hl, y1), (w - hl, cy + t), (0.0, cy + t)];
            let mapped: Vec<(f32, f32)> = pts
                .iter()
                .map(|(dx, y)| if kind == ShapeKind::RightArrow { (x0 + dx, *y) } else { (x1 - dx, *y) })
                .collect();
            polygon(&mapped)
        }
        ShapeKind::UpArrow | ShapeKind::DownArrow => {
            let hl = (w / 2.0).min(h);
            let t = w / 4.0;
            let pts = [(cx - t, h), (cx - t, hl), (x0, hl), (cx, 0.0), (x1, hl), (cx + t, hl), (cx + t, h)];
            let mapped: Vec<(f32, f32)> =
                pts.iter().map(|(x, dy)| if kind == ShapeKind::UpArrow { (*x, y0 + dy) } else { (*x, y1 - dy) }).collect();
            polygon(&mapped)
        }
        ShapeKind::Star4 => unit_poly(&star_unit(4, Some(0.32)), x0, y0, w, h),
        ShapeKind::Star5 => unit_poly(&star_unit(5, Some(0.382)), x0, y0, w, h),
        ShapeKind::Star6 => unit_poly(&star_unit(6, Some(0.577)), x0, y0, w, h),
        ShapeKind::RoundedCallout => {
            let yb = y0 + h * 0.75;
            let tail = [(x0 + w * 0.42, yb), (x0 + w * 0.12, y1), (x0 + w * 0.24, yb)];
            rounded_rect_segs(x0, y0, x1, yb, w.min(yb - y0) * 0.2, Some(tail))
        }
        ShapeKind::OvalCallout => {
            let (ecx, ecy, rx, ry) = (cx, y0 + h * 0.4, w / 2.0, h * 0.4);
            let a_start = 125f32.to_radians();
            let a_end = 100f32.to_radians() + 2.0 * PI;
            let n = 64;
            let mut v = Vec::new();
            for i in 0..=n {
                let a = a_start + (a_end - a_start) * i as f32 / n as f32;
                let p = (ecx + rx * a.cos(), ecy + ry * a.sin());
                v.push(if i == 0 { M(p.0, p.1) } else { L(p.0, p.1) });
            }
            v.push(L(x0 + w * 0.14, y1));
            v.push(Z);
            v
        }
        ShapeKind::CloudCallout => cloud(x0, y0, w, h),
        ShapeKind::Heart => {
            let top = y0 + h / 4.0;
            let ytop = y0 - h / 3.0;
            let dx1 = w * 49.0 / 48.0;
            let dx2 = w * 10.0 / 48.0;
            vec![
                M(cx, top),
                C(cx + dx2, ytop, cx + dx1, top, cx, y1),
                C(cx - dx1, top, cx - dx2, ytop, cx, top),
                Z,
            ]
        }
        ShapeKind::Lightning => {
            const P: [(f32, f32); 11] = [
                (8472.0, 0.0),
                (12860.0, 6080.0),
                (11050.0, 6797.0),
                (16577.0, 12007.0),
                (14767.0, 12877.0),
                (21600.0, 21600.0),
                (10012.0, 14915.0),
                (12222.0, 13987.0),
                (5022.0, 9705.0),
                (7602.0, 8382.0),
                (0.0, 3890.0),
            ];
            let unit: Vec<(f32, f32)> = P.iter().map(|(x, y)| (x / 21600.0, y / 21600.0)).collect();
            unit_poly(&unit, x0, y0, w, h)
        }
    }
}

fn cloud(x0: f32, y0: f32, w: f32, h: f32) -> PathData {
    use Seg::*;
    let (cx, cy) = (x0 + w * 0.52, y0 + h * 0.38);
    let (rx, ry) = (w * 0.40, h * 0.30);
    let bumps = 9;
    let mut v = Vec::new();
    let pt = |a: f32, s: f32| (cx + rx * s * a.cos(), cy + ry * s * a.sin());
    for i in 0..bumps {
        let a0 = i as f32 * 2.0 * PI / bumps as f32 + 0.2;
        let a1 = (i + 1) as f32 * 2.0 * PI / bumps as f32 + 0.2;
        let p0 = pt(a0, 1.0);
        let p1 = pt(a1, 1.0);
        let c0 = pt(a0 + (a1 - a0) * 0.15, 1.42);
        let c1 = pt(a0 + (a1 - a0) * 0.85, 1.42);
        if i == 0 {
            v.push(M(p0.0, p0.1));
        }
        v.push(C(c0.0, c0.1, c1.0, c1.1, p1.0, p1.1));
    }
    v.push(Z);
    // trailing thought bubbles towards the bottom-left corner
    for (ux, uy, ur) in [(0.22, 0.83, 0.065), (0.12, 0.93, 0.045), (0.05, 0.985, 0.015)] {
        v.extend(ellipse(x0 + w * ux, y0 + h * uy, w * ur, h * ur * 1.1));
    }
    v
}

pub fn line_path(a: (f32, f32), b: (f32, f32)) -> PathData {
    vec![Seg::M(a.0, a.1), Seg::L(b.0, b.1)]
}

pub fn curve_path(p0: (f32, f32), c1: (f32, f32), c2: (f32, f32), p3: (f32, f32)) -> PathData {
    vec![Seg::M(p0.0, p0.1), Seg::C(c1.0, c1.1, c2.0, c2.1, p3.0, p3.1)]
}

pub fn polyline_path(pts: &[(f32, f32)], closed: bool) -> PathData {
    let mut v: PathData = pts.iter().enumerate().map(|(i, p)| if i == 0 { Seg::M(p.0, p.1) } else { Seg::L(p.0, p.1) }).collect();
    if closed {
        v.push(Seg::Z);
    }
    v
}

fn to_skia(path: &[Seg]) -> Option<tiny_skia::Path> {
    let mut pb = PathBuilder::new();
    for s in path {
        match *s {
            Seg::M(x, y) => pb.move_to(x, y),
            Seg::L(x, y) => pb.line_to(x, y),
            Seg::C(a, b, c, d, e, f) => pb.cubic_to(a, b, c, d, e, f),
            Seg::Z => pb.close(),
        }
    }
    pb.finish()
}

#[derive(Clone, Copy, Debug)]
pub struct ShapeStyle {
    pub outline: Option<Color32>,
    pub fill: Option<Color32>,
    pub width: f32,
    pub open: bool,
}

pub fn render(pix: &mut Pixmap, path: &[Seg], style: ShapeStyle) {
    let Some(p) = to_skia(path) else {
        // degenerate path: draw a dot for zero-length lines
        if let (Some(c), Some(Seg::M(x, y))) = (style.outline, path.first()) {
            if let Some(dot) = PathBuilder::from_circle(*x, *y, (style.width / 2.0).max(0.5)) {
                let mut paint = Paint::default();
                paint.set_color(sk_color(c));
                paint.anti_alias = true;
                pix.fill_path(&dot, &paint, FillRule::Winding, Transform::identity(), None);
            }
        }
        return;
    };
    if let (Some(fill), false) = (style.fill, style.open) {
        let mut paint = Paint::default();
        paint.set_color(sk_color(fill));
        paint.anti_alias = true;
        pix.fill_path(&p, &paint, FillRule::Winding, Transform::identity(), None);
    }
    if let Some(c) = style.outline {
        let mut paint = Paint::default();
        paint.set_color(sk_color(c));
        paint.anti_alias = true;
        let stroke = Stroke {
            width: style.width.max(1.0),
            miter_limit: 4.0,
            line_cap: if style.open { LineCap::Round } else { LineCap::Butt },
            line_join: if style.open { LineJoin::Round } else { LineJoin::Miter },
            dash: None,
        };
        pix.stroke_path(&p, &paint, &stroke, Transform::identity(), None);
    }
}

/// Flatten path data into egui polylines (for icons / previews).
pub fn flatten(path: &[Seg]) -> Vec<(Vec<Pos2>, bool)> {
    let mut out = Vec::new();
    let mut cur: Vec<Pos2> = Vec::new();
    for s in path {
        match *s {
            Seg::M(x, y) => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), false));
                }
                cur.clear();
                cur.push(Pos2::new(x, y));
            }
            Seg::L(x, y) => cur.push(Pos2::new(x, y)),
            Seg::C(ax, ay, bx, by, ex, ey) => {
                let s = *cur.last().unwrap_or(&Pos2::ZERO);
                for i in 1..=12 {
                    let t = i as f32 / 12.0;
                    let mt = 1.0 - t;
                    let x = mt * mt * mt * s.x + 3.0 * mt * mt * t * ax + 3.0 * mt * t * t * bx + t * t * t * ex;
                    let y = mt * mt * mt * s.y + 3.0 * mt * mt * t * ay + 3.0 * mt * t * t * by + t * t * t * ey;
                    cur.push(Pos2::new(x, y));
                }
            }
            Seg::Z => {
                if cur.len() > 1 {
                    out.push((std::mem::take(&mut cur), true));
                }
            }
        }
    }
    if cur.len() > 1 {
        out.push((cur, false));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shape_renders() {
        for kind in ShapeKind::ALL {
            let mut pix = Pixmap::new(64, 64).unwrap();
            let path = match kind {
                ShapeKind::Line => line_path((4.0, 4.0), (60.0, 60.0)),
                ShapeKind::Curve => curve_path((4.0, 60.0), (10.0, 0.0), (50.0, 64.0), (60.0, 4.0)),
                ShapeKind::Polygon => polyline_path(&[(4.0, 4.0), (60.0, 10.0), (30.0, 60.0)], true),
                _ => box_path(kind, (4.0, 4.0), (60.0, 60.0)),
            };
            let style = ShapeStyle { outline: Some(Color32::BLACK), fill: Some(Color32::RED), width: 3.0, open: matches!(kind, ShapeKind::Line | ShapeKind::Curve) };
            render(&mut pix, &path, style);
            assert!(pix.data().chunks_exact(4).any(|p| p[3] > 0), "{kind:?} drew nothing");
        }
    }
}
