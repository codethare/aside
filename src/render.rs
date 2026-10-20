//! Software renderer: draws the panel into an ARGB8888 (BGRA byte order) shm buffer.
//! Pure CPU, no Wayland dependency, so it is unit-testable headless.

use ab_glyph_rasterizer::{Point, Rasterizer, point};
use skrifa::outline::{DrawSettings, OutlinePen, pen::PathStyle};
use skrifa::prelude::*;

pub type Bgra = [u8; 4];

pub const fn bgra(r: u8, g: u8, b: u8, a: u8) -> Bgra {
    [b, g, r, a]
}

/// Spotlight panel corner radius (logical px), clamped to half the panel size.
pub const CORNER_RADIUS: u32 = 12;

/// Space reserved around the panel for its drop shadow (logical px). The buffer
/// and the layer surface grow by it; the panel keeps its size and position.
pub const SHADOW: u32 = 24;

/// Shadow layers as (blur radius, alpha at the panel edge, vertical offset),
/// logical px and 0-255. A tight contact shadow under a wide ambient one; the
/// offset is positive because the light comes from above.
const SHADOW_TIGHT: (u32, u32, u32) = (7, 170, 2);
const SHADOW_WIDE: (u32, u32, u32) = (17, 130, 6);
// The margin must fit the widest layer, or its falloff is cut off at the buffer edge.
const _: () = assert!(SHADOW >= SHADOW_WIDE.0 + SHADOW_WIDE.2);

/// Hairline on the panel's outermost pixel — the seam between the translucent
/// body and the shadow. Lit along the top, dark underneath, like window chrome.
pub(crate) const BORDER_LIGHT_ALPHA: u32 = 34;
pub(crate) const BORDER_DARK_ALPHA: u32 = 60;

// Eye-comfort rules, checked at compile time (clippy: assertions on constants
// belong here): the content inset must clear the corner radius so text never
// grazes the curve, and the panel must float off the screen edge rather than
// hug it.
const _: () = assert!(crate::PAD >= CORNER_RADIUS);

/// Fully transparent pixel: what the shadow margin holds every frame.
const CLEAR: Bgra = [0, 0, 0, 0];

/// Fixed gap between the prompt label and the typed input (8px grid).
pub(crate) const PROMPT_GAP: f32 = 8.0;
/// Prompt label rail: vertical accent strip on the bar's left edge.
/// Vertical padding defines its height (text line + vpad both sides) —
/// the same rhythm the capsule used.
pub(crate) const PILL_VPAD: u32 = 2;
/// Width of the label rail.
pub(crate) const LABEL_RAIL_W: u32 = 4;
/// A logical-pixel divider at the input/list boundary.
pub(crate) const DIVIDER_H: u32 = 1;
pub(crate) const DIVIDER_ALPHA: u32 = 64;

/// Translucent glass body: the alpha is what the compositor blends with the
/// wallpaper, so the buffer is premultiplied (`panel::draw` does that).
pub const BG_NORMAL: Bgra = bgra(0x1e, 0x1e, 0x1e, 0xd4);
pub const FG_NORMAL: Bgra = bgra(0xbb, 0xbb, 0xbb, 0xff);
/// Input bar: a subtly lighter tone than the list, so bar and list read as
/// separate pieces of one panel.
pub const BG_PROMPT: Bgra = bgra(0x2c, 0x2c, 0x2c, 0xdc);
pub const FG_PROMPT: Bgra = bgra(0xee, 0xee, 0xee, 0xff);
/// Prompt label accent: a 4px rail on the input bar's left edge (option F); a
/// deep slate blue that carries the emphasis while staying harmonious with the
/// gray bar and apart from the selection blue.
pub const LABEL_ACCENT: Bgra = bgra(0x2e, 0x4a, 0x5c, 0xff);
/// Selected row. The background is darker than the old `0f7ca6` so white text
/// clears WCAG AA: `#ffffff` on `#0b6285` is 6.77:1 (the previous pair was
/// 4.06:1 — the weakest contrast in the whole UI, on the row that matters most).
pub const BG_SEL: Bgra = bgra(0x0b, 0x62, 0x85, 0xee);
pub const FG_SEL: Bgra = bgra(0xff, 0xff, 0xff, 0xff);

/// Horizontal pixel span `[x0, x1)` of the rounded panel at row `y`
/// (x0 inclusive, x1 exclusive). Corner pixels fall outside the span;
/// middle rows span the full width. Radius is clamped to half a dimension.
pub fn rounded_span(w: u32, h: u32, r: u32, y: u32) -> (u32, u32) {
    let r = r.min(w / 2).min(h / 2);
    if r == 0 {
        return if y < h { (0, w) } else { (0, 0) };
    }
    let rr = (r * r) as f64;
    let py = y as f64 + 0.5;
    let top = r as f64;
    let bottom = h as f64 - r as f64;
    if py >= top && py <= bottom {
        return (0, w);
    }
    let dy = if py < top { top - py } else { py - bottom };
    let inset = (r as f64 - (rr - dy * dy).max(0.0).sqrt()).ceil() as u32;
    (inset.min(w), w.saturating_sub(inset))
}

/// Fractional horizontal inset of the rounded outline at row `y`. Rounding it
/// up gives the whole-pixel span; the fraction left over is the coverage of the
/// pixel just outside that span, which is what antialiases the edge.
pub(crate) fn rounded_inset(w: u32, h: u32, r: u32, y: u32) -> f64 {
    let r = r.min(w / 2).min(h / 2);
    if r == 0 || y >= h {
        return 0.0;
    }
    let rr = (r * r) as f64;
    let py = y as f64 + 0.5;
    let top = r as f64;
    let bottom = h as f64 - r as f64;
    if py >= top && py <= bottom {
        return 0.0;
    }
    let dy = if py < top { top - py } else { py - bottom };
    r as f64 - (rr - dy * dy).max(0.0).sqrt()
}

/// The panel inside the shadow margin: origin and size in buffer px, plus the
/// rounded outline that the body, its antialiased edge, the hairline and the
/// shadow are all derived from. One outline for all four is what keeps the
/// border and the shadow from leaving a seam between them.
#[derive(Clone, Copy)]
pub(crate) struct Panel {
    pub x: u32,
    pub y: u32,
    pub w: u32,
    pub h: u32,
    pub r: u32,
}

impl Panel {
    /// Whole-pixel span `[x0, x1)` of panel-relative row `rel` in buffer coords.
    pub(crate) fn span(&self, rel: u32) -> (u32, u32) {
        let (x0, x1) = rounded_span(self.w, self.h, self.r, rel);
        (self.x + x0, self.x + x1)
    }
}

/// Premultiply a straight colour: wl_shm ARGB8888 is premultiplied, so a
/// translucent body has to be written as `rgb * a`, not as `rgb` next to an
/// alpha byte (the compositor would blend a too-bright panel).
pub(crate) fn premul(c: Bgra) -> Bgra {
    let a = c[3] as u32;
    [
        (c[0] as u32 * a / 255) as u8,
        (c[1] as u32 * a / 255) as u8,
        (c[2] as u32 * a / 255) as u8,
        c[3],
    ]
}

/// Composite an already premultiplied source over a premultiplied destination
/// with `cov` (0-255) attenuating it — edge coverage, or a border layer.
#[inline]
pub(crate) fn over_pm(dst: &mut [u8], src: Bgra, cov: u32) {
    let sa = src[3] as u32 * cov / 255;
    let inv = 255 - sa;
    for c in 0..3 {
        dst[c] = ((src[c] as u32 * cov / 255) + dst[c] as u32 * inv / 255) as u8;
    }
    dst[3] = (sa + dst[3] as u32 * inv / 255) as u8;
}

pub(crate) fn set_span(buf: &mut [u8], w: u32, y: u32, x0: u32, x1: u32, color: Bgra) {
    let row = &mut buf[(y * w + x0) as usize * 4..(y * w + x1) as usize * 4];
    for px in row.as_chunks_mut::<4>().0 {
        *px = color;
    }
}

/// Both drop-shadow layers in one pass. They share the panel outline, so each
/// pixel gets its alpha from both LUTs and is written once: no read-modify-write
/// between layers, and the band needs no clear (which is why the frame does not
/// start with a full-buffer memset).
pub(crate) fn shadow(buf: &mut [u8], w: u32, h: u32, panel: &Panel, scale: u32) {
    // (blur, vertical offset, alpha by squared distance)
    let s = scale.max(1);
    let layers: [(u32, i64, Vec<u32>); 2] =
        [SHADOW_WIDE, SHADOW_TIGHT].map(|(blur, a0, offset)| {
            let b = (blur * s).max(1);
            let offset = (offset * s) as i64;
            let lut = (0..=b * b)
                .map(|d2| {
                    // A blurred edge starts at half the peak and eases out: a ramp
                    // from the full value leaves a visible cusp where it begins.
                    let t = (d2 as f64).sqrt() / b as f64;
                    (a0 as f64 * 0.5 * (1.0 + (std::f64::consts::PI * t).cos())).round() as u32
                })
                .collect();
            (b, offset, lut)
        });
    let reach = layers.iter().map(|l| l.0).max().unwrap_or(1);
    // Alpha by horizontal distance, rebuilt per row: the two LUTs are combined
    // once per column distance instead of once per shadow pixel.
    let mut band = vec![0u8; reach as usize + 1];
    // Rows inside the panel all share dv = 0, so the band is usually built once.
    let mut cached = [u32::MAX; 2];
    for by in 0..h {
        let y = by as i64;
        // Per layer: squared row distance from the shifted body, clamped to the
        // layer's blur (the offset is what hangs the shadow below the panel).
        let dv: [u32; 2] = [0, 1].map(|k| {
            let (b, offset, _) = &layers[k];
            let top = panel.y as i64 + *offset;
            let bottom = top + panel.h as i64;
            let d = if y < top {
                top - y
            } else if y >= bottom {
                y - bottom + 1
            } else {
                0
            };
            let d = d.min(*b as i64) as u32;
            d * d
        });
        // The outline row comes from the body itself, not from the offset shape:
        // both layers must see the same span, or their falloffs leave a stripe
        // where one starts further out than the other.
        let rel = (y - panel.y as i64).clamp(0, panel.h as i64 - 1) as u32;
        let (x0, x1) = panel.span(rel);
        let start = x0.saturating_sub(reach);
        let end = (x1 + reach).min(w);
        // Everything the band does not cover is emptied: a reused buffer slot
        // still holds the previous frame.
        if start > 0 {
            set_span(buf, w, by, 0, start, CLEAR);
        }
        if end < w {
            set_span(buf, w, by, end, w, CLEAR);
        }
        let visible = (0..2).any(|k| dv[k] < layers[k].0 * layers[k].0);
        if !visible {
            if start < end {
                set_span(buf, w, by, start, end, CLEAR);
            }
            continue;
        }
        if dv != cached {
            cached = dv;
            for (dx, out) in band.iter_mut().enumerate() {
                let dx2 = (dx * dx) as u32;
                let mut a = 0u32;
                for (k, (b, _, lut)) in layers.iter().enumerate() {
                    let la = lut[(dx2 + dv[k]).min(b * b) as usize];
                    a = la + div255(a * (255 - la));
                }
                *out = a as u8;
            }
        }
        // Two bands outside the outline; the body itself is painted later, so
        // it is stepped over rather than tested per pixel.
        if by >= panel.y && by < panel.y + panel.h {
            put_band(buf, w, by, start..x0.min(end), (x0, x1), Side::Left, &band);
            put_band(buf, w, by, x1.max(start)..end, (x0, x1), Side::Right, &band);
        } else {
            put_band(buf, w, by, start..end, (x0, x1), Side::Row, &band);
        }
    }
}

/// Which part of an outline row a shadow band covers.
#[derive(Clone, Copy)]
enum Side {
    Left,
    Right,
    /// The whole row: the body is not there, so the middle is shadow too.
    Row,
}

/// Writes one row's shadow band: premultiplied black, so only the alpha byte.
fn put_band(
    buf: &mut [u8],
    w: u32,
    by: u32,
    xs: std::ops::Range<u32>,
    (x0, x1): (u32, u32),
    side: Side,
    band: &[u8],
) {
    for x in xs {
        let dx = match side {
            Side::Left => x0 - 1 - x,
            Side::Right => x - x1,
            Side::Row if x < x0 => x0 - 1 - x,
            Side::Row if x >= x1 => x - x1,
            Side::Row => 0,
        };
        let a = band[(dx as usize).min(band.len() - 1)];
        let dst = &mut buf[((by * w + x) * 4) as usize..][..4];
        dst.copy_from_slice(&[0, 0, 0, a]);
    }
}

/// `x / 255` without the hardware divide: this runs once per shadow pixel per
/// layer, where the compiler's `/ 255` costs more than the rest of the pixel.
#[inline]
fn div255(x: u32) -> u32 {
    (x + 1 + (x >> 8)) >> 8
}

/// Scrollbar thumb `(y, h)` for a `visible`-row viewport over `total` items at
/// scroll offset `top`, inside a track starting at `track_y` with height
/// `track_h`. `None` when everything fits (or there is no track). Length tracks
/// the visible/total ratio so the indicator reads as "list continues".
pub(crate) fn scroll_thumb(
    track_y: u32,
    track_h: u32,
    visible: usize,
    total: usize,
    top: usize,
) -> Option<(u32, u32)> {
    if visible == 0 || total <= visible || track_h == 0 {
        return None;
    }
    let th = ((track_h as u64 * visible as u64) / total as u64).clamp(1, track_h as u64) as u32;
    let max_top = (total - visible) as u64;
    let span = track_h.saturating_sub(th) as u64;
    let ty = track_y + ((span * top.min(max_top as usize) as u64) / max_top) as u32;
    Some((ty, th))
}

#[allow(clippy::too_many_arguments)] // a rectangle is six numbers plus the buffer
pub fn rect(buf: &mut [u8], w: u32, h: u32, x: u32, y: u32, rw: u32, rh: u32, color: Bgra) {
    let x0 = (x as i64).clamp(0, w as i64) as u32;
    let y0 = (y as i64).clamp(0, h as i64) as u32;
    let x1 = ((x + rw) as i64).clamp(0, w as i64) as u32;
    let y1 = ((y + rh) as i64).clamp(0, h as i64) as u32;
    for py in y0..y1 {
        let row = &mut buf[(py * w + x0) as usize * 4..(py * w + x1) as usize * 4];
        for px in row.as_chunks_mut::<4>().0 {
            // Composite, not overwrite: the panel under it may be translucent.
            blend_px(px, [color[0], color[1], color[2]], color[3] as u32);
        }
    }
}

/// Antialiasing mode for text. LCD subpixel modes sample coverage in three
/// horizontal sub-columns and feed them to R, G and B separately, which is
/// noticeably sharper on 1× displays; `Gray` is neutral (no colour fringing)
/// and is what `Bgr` exists for — panels wired blue-green-red need the mirror.
/// `Gray`/`Bgr` are only reachable by changing `main::TEXT_AA`, but both are
/// covered by tests (and are the documented escape hatches), so they are not
/// dead code.
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subpixel {
    Gray,
    Rgb,
    Bgr,
}

/// Subpixel sample offsets, in pixels, inside one pixel column.
const SUBPIXEL_SAMPLES: f32 = 3.0;
/// BGRA byte index for the red, green and blue channels.
const RGB_CHAN: [usize; 3] = [2, 1, 0];
const BGR_CHAN: [usize; 3] = [0, 1, 2];

/// Subpixel buckets per pixel: glyphs are rasterized per bucket, so pen
/// positioning stays subpixel-accurate and still cacheable.
const BUCKETS_PER_PX: f32 = 4.0;
/// Glyph cache cap; a script with thousands of distinct glyphs resets it
/// (costing one frame) instead of growing without bound.
const GLYPH_CACHE_CAP: usize = 8192;

/// Draw `s` starting at (x, baseline), clipped to `max_w` px wide.
/// Returns the x position just past the last drawn advance.
#[allow(clippy::too_many_arguments)]
pub fn draw_text(
    buf: &mut [u8],
    w: u32,
    h: u32,
    font: &crate::font::PanelFont,
    x: f32,
    baseline: f32,
    s: &str,
    color: Bgra,
    max_w: f32,
    subpixel: Subpixel,
) -> f32 {
    let mut cx = x;
    let end = x + max_w;
    let mode = match subpixel {
        Subpixel::Gray => 0,
        Subpixel::Rgb => 1,
        Subpixel::Bgr => 2,
    };
    let bucket = |v: f32| ((v.fract() * BUCKETS_PER_PX) as u8).min(BUCKETS_PER_PX as u8 - 1);
    let canonical = |v: f32| (v.fract() * BUCKETS_PER_PX).floor() / BUCKETS_PER_PX;
    'ch: for ch in s.chars() {
        // The chain answers in order (primary first), so CJK and anything else
        // the primary lacks falls back to a system font.
        let Some((face_idx, gid, advance)) = font.glyph(ch) else {
            continue; // no face in the chain has this glyph; skip it silently
        };
        if cx + advance > end {
            break 'ch;
        }
        let key: crate::font::GlyphKey = (
            face_idx as u8,
            gid.to_u32() as u16,
            bucket(cx),
            bucket(baseline),
            mode,
        );
        if !font.glyphs.borrow().contains_key(&key) {
            if font.glyphs.borrow().len() >= GLYPH_CACHE_CAP {
                font.glyphs.borrow_mut().clear();
            }
            // Rasterize at the canonical pen (subpixel part only), then blit
            // at the whole-pixel part — the two differ by an integer shift.
            let planes = match subpixel {
                // One neutral plane, used for every channel.
                Subpixel::Gray => vec![plane(
                    font,
                    face_idx,
                    gid,
                    point(canonical(cx), canonical(baseline)),
                )],
                // One plane per subpixel column: each carries one channel.
                _ => (0..3)
                    .map(|k| {
                        let off = k as f32 / SUBPIXEL_SAMPLES;
                        plane(
                            font,
                            face_idx,
                            gid,
                            point(canonical(cx) + off, canonical(baseline)),
                        )
                    })
                    .collect(),
            };
            font.glyphs
                .borrow_mut()
                .insert(key, crate::font::GlyphBitmap { advance, planes });
        }
        let cache = font.glyphs.borrow();
        let bmp = cache.get(&key).expect("inserted above");
        let dx = cx.floor() as i32;
        let dy = baseline.floor() as i32;
        match subpixel {
            Subpixel::Gray => blit_plane(buf, w, h, &bmp.planes[0], dx, dy, color, None),
            Subpixel::Rgb => {
                for (k, plane) in bmp.planes.iter().enumerate() {
                    blit_plane(buf, w, h, plane, dx, dy, color, Some(RGB_CHAN[k]));
                }
            }
            Subpixel::Bgr => {
                for (k, plane) in bmp.planes.iter().enumerate() {
                    blit_plane(buf, w, h, plane, dx, dy, color, Some(BGR_CHAN[k]));
                }
            }
        }
        cx += bmp.advance;
    }
    cx
}

/// Advance width of `s` under the same face-chain rules as `draw_text`, capped
/// at `max_w`. The layout uses it to decide where a row must break.
pub(crate) fn measure(font: &crate::font::PanelFont, s: &str, max_w: f32) -> f32 {
    let mut cx = 0.0;
    for ch in s.chars() {
        let Some((_, _, adv)) = font.glyph(ch) else {
            continue;
        };
        if cx + adv > max_w {
            break;
        }
        cx += adv;
    }
    cx
}

pub(crate) fn row_baseline(font: &crate::font::PanelFont, row_y: u32) -> f32 {
    let top = row_y as f32 + (font.row_h as f32 - font.line_h) / 2.0;
    top + font.ascent_px
}

/// Rasterize one coverage plane of `gid` for chain face `face_idx` at `pos`.
fn plane(
    font: &crate::font::PanelFont,
    face_idx: usize,
    gid: GlyphId,
    pos: Point,
) -> crate::font::GlyphPlane {
    // `PanelFont::glyph` just answered from this face, so it is loaded.
    let Some(face) = font.face(face_idx) else {
        return crate::font::GlyphPlane::default();
    };
    let Some(face_font) = face.font() else {
        return crate::font::GlyphPlane::default();
    };
    raster(&face_font, gid, face.px_per_unit(font.size), pos)
}

/// Rasterize one coverage plane of `gid` at `pos`.
///
/// The outline comes from skrifa in font units; the bounds, the subpixel
/// rounding and the rasterizer are the ones `ab_glyph` used, because those are
/// what put a glyph where the layout measured it. The bounds are the pen's point
/// bounds, including control points — `ttf-parser` computed the same thing (it
/// skips the glyph's declared bbox, which can be a pixel tighter and would move
/// ink).
fn raster(
    font: &FontRef<'_>,
    gid: GlyphId,
    px_per_unit: f32,
    pos: Point,
) -> crate::font::GlyphPlane {
    let mut pen = CurvePen::default();
    let settings = DrawSettings::unhinted(Size::unscaled(), LocationRef::default())
        .with_path_style(PathStyle::FreeType);
    let drawn = font
        .outline_glyphs()
        .get(gid)
        .is_some_and(|glyph| glyph.draw(settings, &mut pen).is_ok());
    if !drawn {
        return crate::font::GlyphPlane::default();
    }
    pen.close();
    let Some((min, max)) = pen.bounds else {
        return crate::font::GlyphPlane::default();
    };
    // The subpixel fraction takes part in the floor/ceil so identical subpixel
    // positions round identically; the whole part is added afterwards.
    let (x_trunc, x_fract) = (pos.x.trunc(), pos.x.fract());
    let (y_trunc, y_fract) = (pos.y.trunc(), pos.y.fract());
    let min_x = (min.x * px_per_unit + x_fract).floor() + x_trunc;
    let max_x = (max.x * px_per_unit + x_fract).ceil() + x_trunc;
    // Font y grows upwards, screen y downwards.
    let min_y = (max.y * -px_per_unit + y_fract).floor() + y_trunc;
    let max_y = (min.y * -px_per_unit + y_fract).ceil() + y_trunc;
    let (w, h) = ((max_x - min_x) as u32, (max_y - min_y) as u32);
    if w == 0 || h == 0 {
        return crate::font::GlyphPlane::default();
    }
    let offset = point(pos.x - min_x, pos.y - min_y);
    let mut rasterizer = Rasterizer::new(w as usize, h as usize);
    for curve in &pen.curves {
        // The scale and the y flip `ab_glyph` applied before rasterizing.
        let at = |p: Point| point(p.x * px_per_unit, p.y * -px_per_unit) + offset;
        match *curve {
            Curve::Line(p0, p1) => rasterizer.draw_line(at(p0), at(p1)),
            Curve::Quad(p0, p1, p2) => rasterizer.draw_quad(at(p0), at(p1), at(p2)),
            Curve::Cubic(p0, p1, p2, p3) => rasterizer.draw_cubic(at(p0), at(p1), at(p2), at(p3)),
        }
    }
    let mut coverage = Vec::with_capacity(w as usize * h as usize);
    rasterizer.for_each_pixel_2d(|_, _, cov| coverage.push((cov * 255.0).round() as u8));
    crate::font::GlyphPlane {
        min_x: min_x as i32,
        min_y: min_y as i32,
        w,
        h,
        coverage,
    }
}

/// One path command, in font units, as skrifa emits it.
#[derive(Clone, Copy)]
enum Curve {
    Line(Point, Point),
    Quad(Point, Point, Point),
    Cubic(Point, Point, Point, Point),
}

/// Collects skrifa's path commands and tracks the bounds of every point it saw,
/// control points included — the same bookkeeping `ttf-parser`'s outline builder
/// did for `ab_glyph`, and what the pixel bounds are derived from.
#[derive(Default)]
struct CurvePen {
    curves: Vec<Curve>,
    bounds: Option<(Point, Point)>,
    last: Point,
    last_move: Option<Point>,
}

impl CurvePen {
    fn include(&mut self, p: Point) {
        let (min, max) = self.bounds.unwrap_or((p, p));
        self.bounds = Some((
            point(min.x.min(p.x), min.y.min(p.y)),
            point(max.x.max(p.x), max.y.max(p.y)),
        ));
    }
}

impl OutlinePen for CurvePen {
    fn move_to(&mut self, x: f32, y: f32) {
        let p = point(x, y);
        self.last = p;
        self.last_move = Some(p);
        self.include(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = point(x, y);
        self.curves.push(Curve::Line(self.last, p));
        self.last = p;
        self.include(p);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (c, p) = (point(cx, cy), point(x, y));
        self.curves.push(Curve::Quad(self.last, c, p));
        self.last = p;
        self.include(c);
        self.include(p);
    }

    fn curve_to(&mut self, cx1: f32, cy1: f32, cx2: f32, cy2: f32, x: f32, y: f32) {
        let (c1, c2, p) = (point(cx1, cy1), point(cx2, cy2), point(x, y));
        self.curves.push(Curve::Cubic(self.last, c1, c2, p));
        self.last = p;
        self.include(c1);
        self.include(c2);
        self.include(p);
    }

    fn close(&mut self) {
        // A last contour can end without one (Cantarell-VF.otf does), which is
        // why this is called once more before the curves are rasterized.
        if let Some(m) = self.last_move.take() {
            self.curves.push(Curve::Line(self.last, m));
        }
    }
}

/// Blend one coverage plane: across all channels (grayscale AA, `chan = None`)
/// or into a single channel (LCD subpixel AA). Subpixel planes touch disjoint
/// channels, so blending them in sequence is exact.
#[allow(clippy::too_many_arguments)]
fn blit_plane(
    buf: &mut [u8],
    w: u32,
    h: u32,
    plane: &crate::font::GlyphPlane,
    dx: i32,
    dy: i32,
    color: Bgra,
    chan: Option<usize>,
) {
    for gy in 0..plane.h {
        for gx in 0..plane.w {
            let cov = plane.coverage[(gy * plane.w + gx) as usize];
            if cov == 0 {
                continue;
            }
            let px = dx + plane.min_x + gx as i32;
            let py = dy + plane.min_y + gy as i32;
            if px < 0 || py < 0 || px >= w as i32 || py >= h as i32 {
                continue;
            }
            let a = (cov as u32 * color[3] as u32) / 255;
            if a == 0 {
                continue;
            }
            let dst = &mut buf[((py as u32 * w + px as u32) * 4) as usize..][..4];
            match chan {
                None => blend_px(dst, [color[0], color[1], color[2]], a),
                Some(c) => dst[c] = ((color[c] as u32 * a + dst[c] as u32 * (255 - a)) / 255) as u8,
            }
        }
    }
}

/// `-P` password mode: one asterisk per char; otherwise return the query unchanged.
pub(crate) fn blend_px(dst: &mut [u8], src: [u8; 3], a: u32) {
    let inv = 255 - a;
    dst[0] = ((src[0] as u32 * a + dst[0] as u32 * inv) / 255) as u8;
    dst[1] = ((src[1] as u32 * a + dst[1] as u32 * inv) / 255) as u8;
    dst[2] = ((src[2] as u32 * a + dst[2] as u32 * inv) / 255) as u8;
    // src is opaque here (its alpha is already folded into `a`), so "src over"
    // in premultiplied space also moves the destination alpha towards opaque —
    // without this, ink over a translucent panel would stay translucent.
    dst[3] = (a + dst[3] as u32 * inv / 255) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::font::PanelFont;

    /// Background first, then glyphs at the row's baseline: the order the panel
    /// paints a row in.
    fn paint_text(
        buf: &mut [u8],
        w: u32,
        h: u32,
        font: &PanelFont,
        text: &str,
        aa: Subpixel,
    ) -> f32 {
        rect(buf, w, h, 0, 0, w, h, BG_PROMPT);
        draw_text(
            buf,
            w,
            h,
            font,
            crate::PAD as f32,
            row_baseline(font, 0),
            text,
            FG_PROMPT,
            10_000.0,
            aa,
        )
    }

    fn blank(w: u32, h: u32) -> Vec<u8> {
        vec![0u8; (w * h * 4) as usize]
    }

    fn has_ink(buf: &[u8], bg: Bgra) -> bool {
        buf.as_chunks::<4>().0.iter().any(|p| *p != bg)
    }

    #[test]
    fn rounded_span_outline_geometry() {
        let (w, h, r) = (200u32, 40u32, 10u32);
        // Top corner row starts inset from the left edge (curve), shrinking to full width.
        let (x0, _) = rounded_span(w, h, r, 0);
        assert!(x0 > 0 && x0 < r);
        // Middle rows are full width.
        assert_eq!(rounded_span(w, h, r, 20), (0, w));
        // Bottom corner row ends before the right edge.
        let (_, x1) = rounded_span(w, h, r, h - 1);
        assert!(x1 > w - r && x1 < w);
        // Zero radius = full rect.
        assert_eq!(rounded_span(w, h, 0, 0), (0, w));
        // Radius clamped to half the height: nearly full-width spans, not deep corners.
        let (x0, x1) = rounded_span(w, 2, 10, 0);
        assert!(x0 <= 1 && x1 >= w - 1);
    }

    #[test]
    fn subpixel_aa_is_per_channel_and_order_symmetric() {
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (w, h) = (240u32, font.row_h);
        let render = |aa: Subpixel| {
            let mut buf = blank(w, h);
            paint_text(&mut buf, w, h, &font, "firim", aa);
            buf
        };
        let gray = render(Subpixel::Gray);
        let rgb = render(Subpixel::Rgb);
        let bgr = render(Subpixel::Bgr);

        assert_ne!(gray, rgb, "subpixel AA must change the rendering");
        assert!(
            gray.as_chunks::<4>().0.iter().all(|p| p[0] == p[2]),
            "grayscale AA must stay colour-neutral"
        );
        assert!(
            rgb.as_chunks::<4>().0.iter().any(|p| p[0] != p[2]),
            "RGB subpixel AA must produce channel differences"
        );
        // BGR is RGB with red and blue swapped, green untouched.
        for (a, b) in rgb.as_chunks::<4>().0.iter().zip(bgr.as_chunks::<4>().0) {
            assert_eq!(a[0], b[2], "blue and red are mirrored");
            assert_eq!(a[2], b[0]);
            assert_eq!(a[1], b[1], "green is shared");
        }
    }

    #[test]
    fn glyph_cache_is_reused_across_frames() {
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (w, h) = (320u32, font.row_h);
        let mut a = blank(w, h);
        paint_text(&mut a, w, h, &font, "Firefox 火狐", Subpixel::Gray);
        let cached = font.glyphs.borrow().len();
        assert!(cached > 0, "the first frame must populate the cache");

        let mut b = blank(w, h);
        paint_text(&mut b, w, h, &font, "Firefox 火狐", Subpixel::Gray);
        assert_eq!(a, b, "cached glyphs must render exactly as the first frame");
        assert_eq!(font.glyphs.borrow().len(), cached, "reuse adds no entries");
    }

    #[test]
    fn draw_text_clips_at_max_width_without_overflow() {
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (w, h) = (400u32, font.row_h);
        let mut buf = blank(w, h);
        let (start, max_w) = (crate::PAD as f32, 60.0f32);
        let limit = (start + max_w).ceil() as u32;
        let long = "W".repeat(200);

        let drawn = draw_text(
            &mut buf,
            w,
            h,
            &font,
            start,
            row_baseline(&font, 0),
            &long,
            FG_PROMPT,
            max_w,
            Subpixel::Gray,
        );
        assert!(drawn <= start + max_w, "advance {drawn} ran past the limit");
        let ink = |px: &[u8]| px[..3] != [0, 0, 0];
        // Something was drawn inside the limit...
        let inside = (0..h)
            .flat_map(|y| (0..limit).map(move |x| (x, y)))
            .any(|(x, y)| ink(&buf[((y * w + x) * 4) as usize..][..4]));
        assert!(
            inside,
            "the clipped text should still draw inside the limit"
        );
        // ...and nothing at or past it.
        for y in 0..h {
            for x in limit..w {
                let px = &buf[((y * w + x) * 4) as usize..][..4];
                assert!(!ink(px), "ink past the clip limit at ({x},{y})");
            }
        }
    }

    #[test]
    fn cjk_renders_through_the_fallback_chain() {
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (w, h) = (320u32, font.row_h);
        let mut buf = blank(w, h);
        paint_text(&mut buf, w, h, &font, "火狐", Subpixel::Gray);
        assert!(has_ink(&buf, BG_PROMPT), "CJK glyphs must reach the buffer");
    }

    #[test]
    fn measure_matches_the_advance_draw_text_reports() {
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (w, h) = (400u32, font.row_h);
        let mut buf = blank(w, h);
        let end = draw_text(
            &mut buf,
            w,
            h,
            &font,
            0.0,
            row_baseline(&font, 0),
            "wrapping 折行",
            FG_PROMPT,
            10_000.0,
            Subpixel::Gray,
        );
        assert!(
            (measure(&font, "wrapping 折行", f32::MAX) - end).abs() < 0.01,
            "the wrapper measures what the painter advances"
        );
    }

    fn luminance(px: Bgra) -> f64 {
        let chan = |c: u8| {
            let c = c as f64 / 255.0;
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * chan(px[2]) + 0.7152 * chan(px[1]) + 0.0722 * chan(px[0])
    }

    fn contrast(a: Bgra, b: Bgra) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        let (hi, lo) = if x > y { (x, y) } else { (y, x) };
        (hi + 0.05) / (lo + 0.05)
    }

    #[test]
    fn default_palette_meets_wcag_aa_for_text() {
        for (name, fg, bg) in [
            ("normal", FG_NORMAL, BG_NORMAL),
            ("prompt", FG_PROMPT, BG_PROMPT),
            ("selected", FG_SEL, BG_SEL),
        ] {
            let r = contrast(fg, bg);
            assert!(
                r >= 4.5,
                "{name} text contrast {r:.2}:1 is below WCAG AA (4.5:1)"
            );
        }
    }

    #[test]
    fn a_whole_pixel_shift_does_not_change_the_plane() {
        // The painter rasterizes at the canonical pen and blits at the whole
        // pixel, so a plane has to be pure translation: same coverage, same
        // canvas, one pixel over. The glyph cache is keyed on that being true.
        let font = PanelFont::load(crate::font::SIZE).expect("system font available");
        let (face_idx, gid, _) = font.glyph('W').expect("the primary face has W");
        let pen = point(0.5, 0.25);
        let here = plane(&font, face_idx, gid, pen);
        let moved = plane(&font, face_idx, gid, point(pen.x + 1.0, pen.y + 1.0));
        assert!(
            here.coverage.iter().any(|c| *c > 0),
            "W must cover something"
        );
        assert_eq!((here.w, here.h), (moved.w, moved.h), "same canvas size");
        assert_eq!(
            here.coverage, moved.coverage,
            "same coverage, pixel for pixel"
        );
        assert_eq!(
            (here.min_x + 1, here.min_y + 1),
            (moved.min_x, moved.min_y),
            "the canvas moved with the pen"
        );
    }
}
