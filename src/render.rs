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
/// `plush`'s current margin.
pub const SHADOW: u32 = 26;

/// Shadow layers as (blur radius, alpha at the panel edge, vertical offset),
/// logical px and 0-255. A tight contact shadow under a wide ambient one; the
/// offset is positive because the light comes from above.
const SHADOW_TIGHT: (u32, u32, u32) = (9, 120, 2);
const SHADOW_WIDE: (u32, u32, u32) = (20, 95, 6);
// The margin must fit the widest layer, or its falloff is cut off at the buffer edge.
const _: () = assert!(SHADOW >= SHADOW_WIDE.0 + SHADOW_WIDE.2);

/// How far the shadow reaches above a card, in logical px: the widest layer's blur
/// less the downward offset that keeps it out of the card itself. The gap between
/// the two cards has to clear it, or one card's shadow smears onto the other.
pub const SHADOW_REACH_UP: u32 = SHADOW_WIDE.0 - SHADOW_WIDE.2;

/// The rim on a card's outermost pixel — the seam between the translucent body and
/// the shadow — and the dark band one pixel inside it, which is what makes the lit
/// edge read as glass with thickness rather than as a line drawn on the card.
/// (The profiles and the inks are `plush`'s, so both panels share one chrome.)
pub(crate) const BORDER_LIGHT_ALPHA: u32 = 90;
pub(crate) const BORDER_DARK_ALPHA: u32 = 90;
/// The band's shape at the bottom edge, where the body shadows its own lower wall;
/// absent at the top, where there is no wall below the edge.
const BORDER_INNER: [(f32, f32); 5] =
    [(0.0, 0.0), (0.3, 0.05), (0.6, 0.3), (0.85, 0.8), (1.0, 1.0)];
const BORDER_INNER_ALPHA: u32 = 130;
/// The lit half holds its strength down the top corner arcs and then dies quickly,
/// so it reads as a specular on glass rather than a wash over the upper half.
const BORDER_LIT: [(f32, f32); 5] = [
    (0.0, 1.0),
    (0.16, 0.9),
    (0.42, 0.55),
    (0.78, 0.0),
    (1.0, 0.0),
];
/// The dark half stays out of the way until the lower third and is whole at the
/// bottom edge.
const BORDER_DARK: [(f32, f32); 5] = [
    (0.0, 0.08),
    (0.45, 0.15),
    (0.72, 0.4),
    (0.9, 0.85),
    (1.0, 1.0),
];

/// Share down a piecewise-linear profile at `t` (0 = top of the card, 1 = bottom).
fn profile(table: &[(f32, f32)], t: f32) -> f32 {
    let mut prev = table[0];
    for &(at, share) in &table[1..] {
        if t <= at {
            let span = at - prev.0;
            let k = if span <= 0.0 {
                1.0
            } else {
                (t - prev.0) / span
            };
            return prev.1 + (share - prev.1) * k;
        }
        prev = (at, share);
    }
    table[table.len() - 1].1
}

// Eye-comfort rules, checked at compile time (clippy: assertions on constants
// belong here): the content inset must clear the corner radius so text never
// grazes the curve, and the panel must float off the screen edge rather than
// hug it.
const _: () = assert!(crate::PAD >= CORNER_RADIUS);

/// Fully transparent pixel: what the shadow margin holds every frame.
const CLEAR: Bgra = [0, 0, 0, 0];

/// Fixed gap between the prompt label and the typed input (8px grid).
pub(crate) const PROMPT_GAP: f32 = 8.0;

/// Translucent glass body: the alpha is what the compositor blends with the
/// wallpaper, so the buffer is premultiplied (`panel::draw` does that). One tone
/// for the whole panel: the bar card and the replies card are the same glass.
pub const BG_PROMPT: Bgra = bgra(0x2c, 0x2c, 0x2c, 0xdc);
/// The secondary ink (`wlmenu`'s rows) and the primary one (its input line): the
/// replies are the first, the field and the highlighted line the second.
pub const FG_NORMAL: Bgra = bgra(0xeb, 0xeb, 0xf5, 0x9b);
pub const FG_PROMPT: Bgra = bgra(0xff, 0xff, 0xff, 0xf5);
/// Gap between the replies card and the bar card (logical px): they are two cards,
/// the way `plush` floats its own apart, and the gap has to clear the shadow one
/// card throws up onto the other.
pub const SEPARATION: u32 = 16;
const _: () = assert!(SEPARATION > SHADOW_REACH_UP);
/// The bar while the prefix mode waits: the field's tone pushed towards the accent,
/// so a mode that swallows the next key is visible before it is read.
pub const BG_MODE: Bgra = bgra(0x2e, 0x4a, 0x5c, 0xdc);
/// Prompt label accent: a 4px rail on the input bar's left edge (option F); a
/// deep slate blue that carries the emphasis while staying harmonious with the
/// gray bar and apart from the selection blue.
pub const LABEL_ACCENT: Bgra = bgra(0x2e, 0x4a, 0x5c, 0xff);
/// The highlighted line's ink: opaque white, so the line the panel would copy is
/// the brightest one on screen without a mark in the margin.
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

/// Both drop-shadow layers for every card in one pass. The falloff is measured
/// from the card's *outline* — a true signed distance to the rounded rectangle — so
/// it wraps the corners instead of being cut off by them the way a per-row span is.
/// A pixel takes the strongest card touching it, and nothing is painted under a
/// translucent body. (Ported from `wlmenu`, which is where the numbers come from.)
pub(crate) fn shadow(buf: &mut [u8], w: u32, h: u32, panels: &[Panel], scale: u32) {
    if panels.is_empty() {
        buf.fill(0);
        return;
    }
    let s = scale.max(1);
    // (blur, vertical offset, alpha by squared distance)
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
    let drop = layers.iter().map(|l| l.1.max(0) as u32).max().unwrap_or(0);
    let shapes: Vec<Shape> = panels.iter().map(Shape::new).collect();
    // The area any layer can touch: the cards' outlines grown by the blur (and by
    // the downward offset), clipped to the buffer.
    let left = shapes.iter().map(|p| p.x0()).min().unwrap_or(0);
    let right = shapes.iter().map(|p| p.x1()).max().unwrap_or(w);
    let x0 = left.saturating_sub(reach);
    let x1 = (right + reach).min(w);
    let y0 = panels
        .iter()
        .map(|p| p.y)
        .min()
        .unwrap_or(0)
        .saturating_sub(reach);
    let y1 = panels
        .iter()
        .map(|p| p.y + p.h)
        .max()
        .unwrap_or(0)
        .saturating_add(reach + drop)
        .min(h);

    // Alpha by horizontal position, rebuilt only when a row's geometry changes:
    // every row level with the straight part of a card shares one profile.
    let mut band = vec![0u8; (x1 - x0) as usize];
    let mut inked = 0..0;
    let mut cached = [u32::MAX; 4];

    for y in y0..h.min(y1.max(y0)) {
        let py = y as f32 + 0.5;
        // This row's distance to every layer of every card, measured from where its
        // corner arcs begin (`qy`), plus the same for the cards' own outlines.
        let mut qys = [[0.0f32; 2]; 2];
        let mut body_qy = [0.0f32; 2];
        let mut key = [u32::MAX; 4];
        let mut live = 0u32;
        for (m, shape) in shapes.iter().enumerate().take(2) {
            // Only the negative side collapses: the body contour needs the real
            // value, or a row just outside the card reads as if it were under it.
            body_qy[m] = shape.qy(py, 0).max(0.0);
            for (k, (blur, offset, _)) in layers.iter().enumerate() {
                // Rows level with the straight part all see the same falloff, so the
                // negative side collapses to one key; past the blur the layer is gone.
                let qy = shape.qy(py, *offset);
                let reach = *blur as f32 + shape.r;
                qys[m][k] = qy.max(0.0).min(reach);
                if qy < reach {
                    key[m * 2 + k] = qys[m][k].to_bits();
                    live |= 1 << (m * 2 + k);
                }
            }
        }
        let live_bits = live;
        if key != cached {
            cached = key;
            let mut first = band.len();
            let mut last = 0;
            for (i, out) in band.iter_mut().enumerate() {
                let px = x0 as f32 + i as f32 + 0.5;
                let mut a = 0u32;
                for (m, shape) in shapes.iter().enumerate().take(2) {
                    // No layer of this card reaches the row at all.
                    if live_bits & (0b11 << (m * 2)) == 0 {
                        continue;
                    }
                    let qx = shape.qx(px);
                    // Under the card itself: painted by its own body, never by the shadow.
                    if outline_gap(qx, body_qy[m], shape.r) <= 0.0 {
                        continue;
                    }
                    let mut pa = 0u32;
                    for (k, (blur, _, lut)) in layers.iter().enumerate() {
                        if live_bits & (1 << (m * 2 + k)) == 0 {
                            continue;
                        }
                        let d = outline_gap(qx, qys[m][k], shape.r).max(0.0);
                        let la = lut[((d * d) as u32).min(blur * blur) as usize];
                        pa = la + div255(pa * (255 - la));
                    }
                    a = a.max(pa);
                }
                if a > 0 {
                    first = first.min(i);
                    last = i;
                }
                *out = a as u8;
            }
            inked = if last >= first { first..last + 1 } else { 0..0 };
        }
        let from = (x0 + inked.start as u32).min(w);
        let to = (x0 + inked.end as u32).min(w);
        // Outside the stretch this row paints, a reused buffer slot still holds the
        // previous frame.
        if from > 0 {
            set_span(buf, w, y, 0, from, CLEAR);
        }
        if to < w {
            set_span(buf, w, y, to, w, CLEAR);
        }
        // The card standing on this row paints its own body, so the shadow goes in
        // around it instead of under it: two stretches, not the whole width.
        let body = panels
            .iter()
            .find(|p| y >= p.y && y < p.y + p.h)
            .map(|p| p.span(y - p.y));
        let (bl, br) = match body {
            Some((bx0, bx1)) => (bx0.max(from).min(to), bx1.min(to).max(from)),
            None => (to, to),
        };
        for (seg_from, seg_to) in [(from, bl), (br, to)] {
            for (pixel, a) in buf[((y * w + seg_from) * 4) as usize..]
                .as_chunks_mut::<4>()
                .0
                .iter_mut()
                .take((seg_to.saturating_sub(seg_from)) as usize)
                .zip(&band[(seg_from - x0) as usize..(seg_to - x0) as usize])
            {
                pixel.copy_from_slice(&[0, 0, 0, *a]);
            }
        }
    }
    // Rows the layers cannot reach at all.
    for y in (0..y0).chain(y1.min(h)..h) {
        set_span(buf, w, y, 0, w, CLEAR);
    }
}

/// A card's outline for the shadow: centre, half extents, and the corner radius
/// clamped to them, so the distances below stay meaningful on a degenerate card.
struct Shape {
    cx: f32,
    cy: f32,
    half_w: f32,
    half_h: f32,
    r: f32,
    x: u32,
    w: u32,
}

impl Shape {
    fn new(panel: &Panel) -> Self {
        let half_w = panel.w as f32 / 2.0;
        let half_h = panel.h as f32 / 2.0;
        Self {
            cx: panel.x as f32 + half_w,
            cy: panel.y as f32 + half_h,
            half_w,
            half_h,
            r: (panel.r as f32).min(half_w).min(half_h),
            x: panel.x,
            w: panel.w,
        }
    }

    /// Horizontal gap from where the corner arcs begin; `|qx| > r` is the straight side.
    fn qx(&self, px: f32) -> f32 {
        (px - self.cx).abs() - (self.half_w - self.r)
    }

    /// The same for a row, with the layer's downward offset applied.
    fn qy(&self, py: f32, offset: i64) -> f32 {
        (py - self.cy - offset as f32).abs() - (self.half_h - self.r)
    }

    fn x0(&self) -> u32 {
        self.x
    }

    fn x1(&self) -> u32 {
        self.x + self.w
    }
}

/// Signed distance from a rounded rectangle's outline, given the horizontal and
/// vertical gaps to where its corner arcs begin: negative inside.
fn outline_gap(qx: f32, qy: f32, radius: f32) -> f32 {
    if qy <= 0.0 {
        qx - radius
    } else if qx <= 0.0 {
        qy - radius
    } else {
        (qx * qx + qy * qy).sqrt() - radius
    }
}

/// One card: rounded body fill, 1 px coverage antialiasing on both sides, and the
/// hairline rim that makes its edge read as glass against the shadow. Every card
/// takes the same tone except the bar in the prefix mode.
pub(crate) fn body(buf: &mut [u8], w: u32, panel: &Panel, tone: Bgra) {
    let bg = premul(tone);
    for by in panel.y..panel.y + panel.h {
        let rel = by - panel.y;
        let (x0, x1) = panel.span(rel);
        set_span(buf, w, by, x0, x1, bg);
        // What is left of the fractional corner inset is the coverage of the pixel
        // just outside the span, which is what antialiases the edge.
        let inset = rounded_inset(panel.w, panel.h, panel.r, rel);
        let cov = ((inset.ceil() - inset) * 255.0).round() as u32;
        if cov > 0 && x0 > 0 {
            over_pm(&mut buf[(by * w + x0 - 1) as usize * 4..][..4], bg, cov);
        }
        if cov > 0 && x1 < w {
            over_pm(&mut buf[(by * w + x1) as usize * 4..][..4], bg, cov);
        }
        // The rim on the outermost pixel: the seam where the body meets the shadow,
        // dark underneath and lit on top.
        let t = rel as f32 / panel.h.max(1) as f32;
        let dark = (BORDER_DARK_ALPHA as f32 * profile(&BORDER_DARK, t)).round() as u32;
        let light = (BORDER_LIGHT_ALPHA as f32 * profile(&BORDER_LIT, t)).round() as u32;
        if x0 < x1 {
            for x in [x0, x1 - 1] {
                let dst = &mut buf[(by * w + x) as usize * 4..][..4];
                if dark > 0 {
                    over_pm(dst, [0, 0, 0, 0xff], dark);
                }
                if light > 0 {
                    over_pm(dst, [0xff, 0xff, 0xff, 0xff], light);
                }
            }
        }
        // The band one pixel inside it, on the two columns and, on the rows where the
        // band's own edges lie — one below the top edge, one above the bottom — across
        // the whole inner span.
        let band = (BORDER_INNER_ALPHA as f32 * profile(&BORDER_INNER, t)).round() as u32;
        if band > 0 && x1 > x0 + 2 {
            let edge_row = rel == 1 || rel + 2 == panel.h;
            for x in (x0 + 1)..(x1 - 1) {
                if !edge_row && x != x0 + 1 && x != x1 - 2 {
                    continue;
                }
                over_pm(
                    &mut buf[(by * w + x) as usize * 4..][..4],
                    [0, 0, 0, 0xff],
                    band,
                );
            }
        }
    }
}

/// `x / 255` without the hardware divide: this runs once per shadow pixel per
/// layer, where the compiler's `/ 255` costs more than the rest of the pixel.
#[inline]
fn div255(x: u32) -> u32 {
    (x + 1 + (x >> 8)) >> 8
}

/// Scrollbar thumb `(y, h)` for a `visible`-pixel viewport over `total` pixels at
/// scroll offset `top`, inside a track starting at `track_y` with height
/// `track_h`. `None` when everything fits (or there is no track). Length tracks
/// the visible/total ratio so the indicator reads as "list continues".
pub(crate) fn scroll_thumb(
    track_y: u32,
    track_h: u32,
    visible: u32,
    total: u32,
    top: u32,
) -> Option<(u32, u32)> {
    if visible == 0 || total <= visible || track_h == 0 {
        return None;
    }
    let th = ((track_h as u64 * visible as u64) / total as u64).clamp(1, track_h as u64) as u32;
    let max_top = (total - visible) as u64;
    let span = track_h.saturating_sub(th) as u64;
    let ty = track_y + ((span * top.min(total - visible) as u64) / max_top) as u32;
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

/// Fill a rounded rectangle with a colour already composited over what lies
/// beneath (the body is), clipped to the `clip` rows so a card scrolled half out
/// of the viewport cannot paint over the bar. The outline and the edge coverage
/// are the panel body's, which is what makes a card's corner look like every
/// other corner in the product.
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

    /// The inks are translucent, so what a reader sees is the ink composited over
    /// the one glass: that pair is what has to clear AA.
    fn ink_over(fg: Bgra, bg: Bgra) -> Bgra {
        let mut px = bg;
        for c in 0..3 {
            let a = fg[3] as u32;
            px[c] = ((fg[c] as u32 * a + bg[c] as u32 * (255 - a)) / 255) as u8;
        }
        px[3] = 255;
        px
    }

    /// The band one pixel inside the rim is what makes the lit edge read as glass
    /// with thickness: under the bottom edge where the body shadows its lower wall,
    /// and absent at the top where there is no wall below it.
    #[test]
    fn a_dark_band_sits_inside_the_bottom_rim() {
        let (rw, rh, r) = (120u32, 90u32, 12u32);
        let (x, y, w) = (10u32, 10u32, 160u32);
        let mut buf = blank(w, 100);
        let panel = Panel {
            x,
            y,
            w: rw,
            h: rh,
            r,
        };
        body(&mut buf, w, &panel, BG_PROMPT);
        fn px(buf: &[u8], w: u32, x: u32, y: u32) -> &[u8] {
            &buf[((y * w + x) * 4) as usize..][..4]
        }
        // A column level with the straight part of the card, so the rim and the band
        // are the two outermost pixels of the row.
        let column = x + rw / 2;
        // One row above the bottom edge: the band's own edge, across the inner span.
        let band = px(&buf, w, column, y + rh - 2);
        let inside = px(&buf, w, column, y + rh - 6);
        assert!(
            band[0] < inside[0],
            "the band is darker than the body: {band:?} against {inside:?}"
        );
        // ...and the left column has it too, on every row.
        let band = px(&buf, w, x + 1, y + rh - 6);
        let inside = px(&buf, w, x + 3, y + rh - 6);
        assert!(band[0] < inside[0], "the band runs down the side too");
        // At the top there is no band: the pixel inside the rim is the body.
        let top_in = px(&buf, w, column, y + 2);
        let top_deeper = px(&buf, w, column, y + 6);
        assert_eq!(top_in, top_deeper, "no band under the top edge");
    }

    #[test]
    fn default_palette_meets_wcag_aa_for_text() {
        for (name, fg) in [
            ("normal", FG_NORMAL),
            ("prompt", FG_PROMPT),
            ("selected", FG_SEL),
            // The hint the bar shows while the prefix mode waits.
            ("mode", FG_PROMPT),
        ] {
            let bg = match name {
                "mode" => BG_MODE,
                _ => BG_PROMPT,
            };
            let seen = ink_over(fg, bg);
            let r = contrast(seen, bg);
            assert!(
                r >= 4.5,
                "{name} text contrast {r:.2}:1 is below WCAG AA (4.5:1)"
            );
        }
    }

    /// One card of the shared chrome: body inside, clear outside, and the hairline
    /// lit on the top edge and dark on the bottom one.
    #[test]
    fn a_card_is_glass_with_a_lit_top_and_a_dark_bottom() {
        let (rw, rh, r) = (120u32, 60u32, 12u32);
        let (x, y) = (10u32, 10u32);
        let (w, h) = (160u32, 90u32);
        let mut buf = blank(w, h);
        let panel = Panel {
            x,
            y,
            w: rw,
            h: rh,
            r,
        };
        body(&mut buf, w, &panel, BG_PROMPT);
        fn px(buf: &[u8], w: u32, x: u32, y: u32) -> &[u8] {
            &buf[((y * w + x) * 4) as usize..][..4]
        }
        let glass = premul(BG_PROMPT);
        assert_eq!(
            px(&buf, w, x + rw / 2, y + rh / 2),
            glass,
            "the middle is the body"
        );
        assert_eq!(px(&buf, w, x, y), &CLEAR, "a corner is outside the outline");
        let edge = px(&buf, w, x + rounded_span(rw, rh, r, 1).0 - 1, y + 1);
        assert!(
            edge[3] > 0 && edge[3] < glass[3],
            "the corner pixel carries partial coverage: {edge:?}"
        );
        // The rim: lighter than the inside at the top, darker at the bottom, and the
        // dark band one pixel in at the bottom edge.
        // The rim, against three pixels inside it: the band occupies the pixel next
        // to the rim and would pass for the body's own edge.
        let rim = |row: u32| (px(&buf, w, x, y + row), px(&buf, w, x + 3, y + row));
        // A row or two inside the flat band: within the corner radius the span is
        // inset, and the pixel at the panel edge is edge coverage, not the rim.
        let (top_rim, top_in) = rim(r + 2);
        assert!(top_rim[0] > top_in[0], "top rim is lit");
        let (bottom_rim, bottom_in) = rim(rh - r - 3);
        assert!(bottom_rim[0] < bottom_in[0], "bottom rim is dark");
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
