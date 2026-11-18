//! Low level pixel utilities operating on `tiny_skia::Pixmap`.
//!
//! The main canvas is always fully opaque, which means its premultiplied
//! RGBA data is identical to straight RGBA. Floating selections may carry
//! transparency and are handled with the premultiplied helpers below.

use egui::Color32;
use tiny_skia::{BlendMode, FilterQuality, PathBuilder, Pixmap, PixmapPaint, Transform};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct IRect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl IRect {
    pub const fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self { x, y, w, h }
    }

    /// Rectangle spanned by two pixel coordinates (start of drag to end of drag).
    pub fn from_points(a: (i32, i32), b: (i32, i32)) -> Self {
        let x = a.0.min(b.0);
        let y = a.1.min(b.1);
        Self::new(x, y, (a.0 - b.0).abs(), (a.1 - b.1).abs())
    }

    pub fn right(&self) -> i32 {
        self.x + self.w
    }

    pub fn bottom(&self) -> i32 {
        self.y + self.h
    }

    pub fn is_empty(&self) -> bool {
        self.w <= 0 || self.h <= 0
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x as f32 && y >= self.y as f32 && x < self.right() as f32 && y < self.bottom() as f32
    }

    pub fn intersect(&self, o: &IRect) -> Option<IRect> {
        let x0 = self.x.max(o.x);
        let y0 = self.y.max(o.y);
        let x1 = self.right().min(o.right());
        let y1 = self.bottom().min(o.bottom());
        if x1 > x0 && y1 > y0 {
            Some(IRect::new(x0, y0, x1 - x0, y1 - y0))
        } else {
            None
        }
    }
}

pub fn rgba(c: Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), 255]
}

pub fn sk_color(c: Color32) -> tiny_skia::Color {
    tiny_skia::Color::from_rgba8(c.r(), c.g(), c.b(), c.a())
}

pub fn new_canvas(w: u32, h: u32, fill: Color32) -> Pixmap {
    let mut p = Pixmap::new(w.max(1), h.max(1)).expect("valid canvas size");
    p.fill(sk_color(fill));
    p
}

pub fn pixel(p: &Pixmap, x: i32, y: i32) -> Option<[u8; 4]> {
    if x < 0 || y < 0 || x >= p.width() as i32 || y >= p.height() as i32 {
        return None;
    }
    let i = ((y as u32 * p.width() + x as u32) * 4) as usize;
    let d = p.data();
    Some([d[i], d[i + 1], d[i + 2], d[i + 3]])
}

/// Scan-line flood fill with exact color matching (classic paint bucket).
pub fn flood_fill(p: &mut Pixmap, x: i32, y: i32, color: [u8; 4]) -> bool {
    let (w, h) = (p.width() as i32, p.height() as i32);
    if x < 0 || y < 0 || x >= w || y >= h {
        return false;
    }
    let data = p.data_mut();
    let idx = |x: i32, y: i32| ((y * w + x) * 4) as usize;
    let s = idx(x, y);
    let target = [data[s], data[s + 1], data[s + 2], data[s + 3]];
    if target == color {
        return false;
    }
    let matches = |d: &[u8], i: usize| d[i] == target[0] && d[i + 1] == target[1] && d[i + 2] == target[2] && d[i + 3] == target[3];
    let mut stack = vec![(x, y)];
    while let Some((sx, sy)) = stack.pop() {
        if !matches(data, idx(sx, sy)) {
            continue;
        }
        let mut lx = sx;
        while lx > 0 && matches(data, idx(lx - 1, sy)) {
            lx -= 1;
        }
        let mut rx = sx;
        while rx + 1 < w && matches(data, idx(rx + 1, sy)) {
            rx += 1;
        }
        for xx in lx..=rx {
            let i = idx(xx, sy);
            data[i..i + 4].copy_from_slice(&color);
        }
        for ny in [sy - 1, sy + 1] {
            if ny < 0 || ny >= h {
                continue;
            }
            let mut xx = lx;
            while xx <= rx {
                if matches(data, idx(xx, ny)) {
                    stack.push((xx, ny));
                    while xx <= rx && matches(data, idx(xx, ny)) {
                        xx += 1;
                    }
                } else {
                    xx += 1;
                }
            }
        }
    }
    true
}

/// Copy a rectangle out of `p`. Areas outside of `p` are transparent.
pub fn crop(p: &Pixmap, r: IRect) -> Pixmap {
    let mut out = Pixmap::new(r.w.max(1) as u32, r.h.max(1) as u32).unwrap();
    copy_raw(&mut out, p, -r.x, -r.y);
    out
}

/// Raw copy of `src` into `dst` at (x, y), replacing pixels (no blending).
pub fn copy_raw(dst: &mut Pixmap, src: &Pixmap, x: i32, y: i32) {
    let dw = dst.width() as i32;
    let dh = dst.height() as i32;
    let sw = src.width() as i32;
    let sh = src.height() as i32;
    let x0 = x.max(0);
    let y0 = y.max(0);
    let x1 = (x + sw).min(dw);
    let y1 = (y + sh).min(dh);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    let sdata = src.data();
    let ddata = dst.data_mut();
    let len = ((x1 - x0) * 4) as usize;
    for yy in y0..y1 {
        let si = (((yy - y) * sw + (x0 - x)) * 4) as usize;
        let di = ((yy * dw + x0) * 4) as usize;
        ddata[di..di + len].copy_from_slice(&sdata[si..si + len]);
    }
}

/// Source-over composite of `src` onto `dst` at integer offset.
pub fn draw_over(dst: &mut Pixmap, src: &Pixmap, x: i32, y: i32) {
    dst.draw_pixmap(x, y, src.as_ref(), &PixmapPaint::default(), Transform::identity(), None);
}

pub fn fill_rect(p: &mut Pixmap, r: IRect, color: [u8; 4]) {
    let bounds = IRect::new(0, 0, p.width() as i32, p.height() as i32);
    let Some(r) = r.intersect(&bounds) else { return };
    let w = p.width() as i32;
    let data = p.data_mut();
    for yy in r.y..r.bottom() {
        for xx in r.x..r.right() {
            let i = ((yy * w + xx) * 4) as usize;
            data[i..i + 4].copy_from_slice(&color);
        }
    }
}

/// Fill the pixels of rectangle `r` where `mask` (sized r.w * r.h) is set.
pub fn fill_masked(p: &mut Pixmap, r: IRect, mask: &[u8], color: [u8; 4]) {
    let w = p.width() as i32;
    let h = p.height() as i32;
    let data = p.data_mut();
    for my in 0..r.h {
        let yy = r.y + my;
        if yy < 0 || yy >= h {
            continue;
        }
        for mx in 0..r.w {
            let xx = r.x + mx;
            if xx < 0 || xx >= w {
                continue;
            }
            if mask[(my * r.w + mx) as usize] != 0 {
                let i = ((yy * w + xx) * 4) as usize;
                data[i..i + 4].copy_from_slice(&color);
            }
        }
    }
}

/// Clear every pixel whose mask value is zero. `mask` must match the pixmap size.
pub fn apply_mask(p: &mut Pixmap, mask: &[u8]) {
    for (px, m) in p.data_mut().chunks_exact_mut(4).zip(mask.iter()) {
        if *m == 0 {
            px.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
}

/// Make all pixels exactly matching `color` transparent (used for transparent selections).
pub fn key_out(p: &mut Pixmap, color: Color32) {
    let c = rgba(color);
    for px in p.data_mut().chunks_exact_mut(4) {
        if px[3] == 255 && px[0] == c[0] && px[1] == c[1] && px[2] == c[2] {
            px.copy_from_slice(&[0, 0, 0, 0]);
        }
    }
}

/// Flatten a (possibly transparent) pixmap onto a solid background.
pub fn flatten_onto(p: &Pixmap, bg: Color32) -> Pixmap {
    let mut out = new_canvas(p.width(), p.height(), bg);
    draw_over(&mut out, p, 0, 0);
    out
}

fn remap(p: &Pixmap, nw: u32, nh: u32, f: impl Fn(u32, u32) -> (u32, u32)) -> Pixmap {
    let mut out = Pixmap::new(nw, nh).unwrap();
    let sw = p.width();
    let src = p.data();
    let dst = out.data_mut();
    for dy in 0..nh {
        for dx in 0..nw {
            let (sx, sy) = f(dx, dy);
            let si = ((sy * sw + sx) * 4) as usize;
            let di = ((dy * nw + dx) * 4) as usize;
            dst[di..di + 4].copy_from_slice(&src[si..si + 4]);
        }
    }
    out
}

pub fn rotate_cw(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    remap(p, h, w, |dx, dy| (dy, h - 1 - dx))
}

pub fn rotate_ccw(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    remap(p, h, w, |dx, dy| (w - 1 - dy, dx))
}

pub fn rotate_180(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    remap(p, w, h, |dx, dy| (w - 1 - dx, h - 1 - dy))
}

pub fn flip_h(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    remap(p, w, h, |dx, dy| (w - 1 - dx, dy))
}

pub fn flip_v(p: &Pixmap) -> Pixmap {
    let (w, h) = (p.width(), p.height());
    remap(p, w, h, |dx, dy| (dx, h - 1 - dy))
}

pub fn scale(p: &Pixmap, nw: u32, nh: u32, smooth: bool) -> Pixmap {
    let (nw, nh) = (nw.max(1), nh.max(1));
    if nw == p.width() && nh == p.height() {
        return p.clone();
    }
    let mut out = Pixmap::new(nw, nh).unwrap();
    let paint = PixmapPaint {
        opacity: 1.0,
        blend_mode: BlendMode::Source,
        quality: if smooth { FilterQuality::Bilinear } else { FilterQuality::Nearest },
    };
    let t = Transform::from_scale(nw as f32 / p.width() as f32, nh as f32 / p.height() as f32);
    out.draw_pixmap(0, 0, p.as_ref(), &paint, t, None);
    out
}

/// Skew by the given angles in degrees. Uncovered areas are filled with `fill`
/// (or left transparent when `fill` is `None`).
pub fn skew(p: &Pixmap, deg_h: f32, deg_v: f32, fill: Option<Color32>) -> Pixmap {
    let kx = deg_h.to_radians().tan();
    let ky = deg_v.to_radians().tan();
    let (w, h) = (p.width() as f32, p.height() as f32);
    let corners = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].map(|(x, y)| (x + kx * y, y + ky * x));
    let minx = corners.iter().map(|c| c.0).fold(f32::MAX, f32::min);
    let maxx = corners.iter().map(|c| c.0).fold(f32::MIN, f32::max);
    let miny = corners.iter().map(|c| c.1).fold(f32::MAX, f32::min);
    let maxy = corners.iter().map(|c| c.1).fold(f32::MIN, f32::max);
    let nw = (maxx - minx).round().clamp(1.0, 30000.0) as u32;
    let nh = (maxy - miny).round().clamp(1.0, 30000.0) as u32;
    let mut out = match fill {
        Some(c) => new_canvas(nw, nh, c),
        None => Pixmap::new(nw, nh).unwrap(),
    };
    let t = Transform::from_row(1.0, ky, kx, 1.0, -minx, -miny);
    let paint = PixmapPaint { quality: FilterQuality::Bilinear, ..Default::default() };
    out.draw_pixmap(0, 0, p.as_ref(), &paint, t, None);
    out
}

/// Invert the colors of a premultiplied pixmap, keeping alpha.
pub fn invert(p: &mut Pixmap) {
    for px in p.data_mut().chunks_exact_mut(4) {
        let a = px[3];
        px[0] = a - px[0].min(a);
        px[1] = a - px[1].min(a);
        px[2] = a - px[2].min(a);
    }
}

pub fn to_color_image(p: &Pixmap) -> egui::ColorImage {
    egui::ColorImage::from_rgba_premultiplied([p.width() as usize, p.height() as usize], p.data())
}

/// Build a pixmap from straight RGBA8 data. If `flatten` is given the result
/// is composited onto that color and is fully opaque.
pub fn from_rgba8(w: u32, h: u32, data: &[u8], flatten: Option<Color32>) -> Pixmap {
    let mut p = Pixmap::new(w.max(1), h.max(1)).unwrap();
    for (dst, src) in p.data_mut().chunks_exact_mut(4).zip(data.chunks_exact(4)) {
        let a = src[3] as u32;
        match flatten {
            Some(bg) => {
                let bgc = [bg.r() as u32, bg.g() as u32, bg.b() as u32];
                for c in 0..3 {
                    dst[c] = ((src[c] as u32 * a + bgc[c] * (255 - a) + 127) / 255) as u8;
                }
                dst[3] = 255;
            }
            None => {
                for c in 0..3 {
                    dst[c] = ((src[c] as u32 * a + 127) / 255) as u8;
                }
                dst[3] = a as u8;
            }
        }
    }
    p
}

/// Straight (un-premultiplied) RGBA8 data of a pixmap.
pub fn to_rgba8(p: &Pixmap) -> Vec<u8> {
    let mut out = Vec::with_capacity(p.data().len());
    for px in p.pixels() {
        let c = px.demultiply();
        out.extend_from_slice(&[c.red(), c.green(), c.blue(), c.alpha()]);
    }
    out
}

/// Rasterize a polygon (in local coordinates) into a w*h mask of 0/255 values.
pub fn polygon_mask(w: i32, h: i32, pts: &[(f32, f32)]) -> Vec<u8> {
    let mut mask = vec![0u8; (w.max(1) * h.max(1)) as usize];
    if pts.len() < 3 {
        return vec![255u8; mask.len()];
    }
    let Some(mut tmp) = Pixmap::new(w.max(1) as u32, h.max(1) as u32) else { return mask };
    let mut pb = PathBuilder::new();
    pb.move_to(pts[0].0, pts[0].1);
    for p in &pts[1..] {
        pb.line_to(p.0, p.1);
    }
    pb.close();
    if let Some(path) = pb.finish() {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(255, 255, 255, 255);
        paint.anti_alias = false;
        tmp.fill_path(&path, &paint, tiny_skia::FillRule::Winding, Transform::identity(), None);
    }
    for (m, px) in mask.iter_mut().zip(tmp.data().chunks_exact(4)) {
        *m = if px[3] > 0 { 255 } else { 0 };
    }
    mask
}

/// Light gray checkerboard used behind transparent areas.
pub fn checkerboard(w: u32, h: u32) -> Pixmap {
    let mut p = Pixmap::new(w.max(1), h.max(1)).unwrap();
    let data = p.data_mut();
    for y in 0..h as usize {
        let row = (y / 8) % 2;
        for x in 0..w as usize {
            let v = if (x / 8) % 2 == row { 255 } else { 204 };
            let i = (y * w as usize + x) * 4;
            data[i] = v;
            data[i + 1] = v;
            data[i + 2] = v;
            data[i + 3] = 255;
        }
    }
    p
}

/// Simple xorshift RNG – good enough for brush textures and airbrush spray.
#[derive(Clone)]
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        (x >> 32) as u32
    }

    /// Uniform float in [0, 1).
    pub fn f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }
}

/// Deterministic per-pixel noise used for "paper grain" textures.
pub fn grain(x: i32, y: i32) -> u8 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    ((h ^ (h >> 16)) & 0xFF) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flood_fill_stays_inside_border() {
        let mut p = new_canvas(20, 20, Color32::WHITE);
        // black square outline from (5,5) to (14,14)
        for i in 5..15 {
            for (x, y) in [(i, 5), (i, 14), (5, i), (14, i)] {
                fill_rect(&mut p, IRect::new(x, y, 1, 1), [0, 0, 0, 255]);
            }
        }
        assert!(flood_fill(&mut p, 10, 10, [255, 0, 0, 255]));
        assert_eq!(pixel(&p, 10, 10), Some([255, 0, 0, 255]));
        assert_eq!(pixel(&p, 5, 10), Some([0, 0, 0, 255]));
        assert_eq!(pixel(&p, 2, 2), Some([255, 255, 255, 255]));
        // filling with the same color is a no-op
        assert!(!flood_fill(&mut p, 10, 10, [255, 0, 0, 255]));
    }

    #[test]
    fn rotations_round_trip() {
        let mut p = new_canvas(7, 3, Color32::WHITE);
        fill_rect(&mut p, IRect::new(0, 0, 1, 1), [255, 0, 0, 255]);
        let r = rotate_cw(&p);
        assert_eq!((r.width(), r.height()), (3, 7));
        assert_eq!(pixel(&r, 2, 0), Some([255, 0, 0, 255]));
        assert_eq!(rotate_ccw(&r).data(), p.data());
        assert_eq!(rotate_180(&rotate_180(&p)).data(), p.data());
        assert_eq!(flip_h(&flip_h(&p)).data(), p.data());
        assert_eq!(flip_v(&flip_v(&p)).data(), p.data());
    }

    #[test]
    fn rgba_round_trip_and_crop() {
        let data: Vec<u8> = (0..4 * 4).flat_map(|i| [i as u8 * 10, 20, 30, 255]).collect();
        let p = from_rgba8(4, 4, &data, None);
        assert_eq!(to_rgba8(&p), data);
        let c = crop(&p, IRect::new(2, 2, 4, 4));
        assert_eq!(pixel(&c, 0, 0).unwrap()[3], 255);
        assert_eq!(pixel(&c, 3, 3).unwrap()[3], 0, "outside the source is transparent");
    }

    #[test]
    fn polygon_mask_covers_triangle() {
        let m = polygon_mask(10, 10, &[(0.0, 0.0), (10.0, 0.0), (0.0, 10.0)]);
        let set = m.iter().filter(|v| **v != 0).count();
        assert!((40..=60).contains(&set), "about half the pixels, got {set}");
    }
}
