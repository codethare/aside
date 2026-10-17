//! Software renderer: draws the panel into an ARGB8888 (BGRA byte order) shm buffer.
//! Pure CPU, no Wayland dependency, so it is unit-testable headless.

use ab_glyph::{Font, Glyph, GlyphId, PxScale, ScaleFont, point};

pub type Bgra = [u8; 4];

pub const fn bgra(r: u8, g: u8, b: u8, a: u8) -> Bgra {
    [b, g, r, a]
}

/// Spotlight panel corner radius (logical px), clamped to half the panel size.
pub const CORNER_RADIUS: u32 = 12;

// Eye-comfort rules, checked at compile time (clippy: assertions on constants
// belong here): the content inset must clear the corner radius so text never
// grazes the curve, and the panel must float off the screen edge rather than
// hug it.
const _: () = assert!(crate::PAD >= CORNER_RADIUS);

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

pub const BG_NORMAL: Bgra = bgra(0x22, 0x22, 0x22, 0xff);
pub const FG_NORMAL: Bgra = bgra(0xbb, 0xbb, 0xbb, 0xff);
/// Input bar: a subtly lighter tone than the list, so bar and list read as
/// separate pieces of one panel.
pub const BG_PROMPT: Bgra = bgra(0x2e, 0x2e, 0x2e, 0xff);
pub const FG_PROMPT: Bgra = bgra(0xee, 0xee, 0xee, 0xff);
/// Prompt label accent: a 4px rail on the input bar's left edge (option F); a
/// deep slate blue that carries the emphasis while staying harmonious with the
/// gray bar and apart from the selection blue.
pub const LABEL_ACCENT: Bgra = bgra(0x2e, 0x4a, 0x5c, 0xff);
/// Selected row. The background is darker than the old `0f7ca6` so white text
/// clears WCAG AA: `#ffffff` on `#0b6285` is 6.77:1 (the previous pair was
/// 4.06:1 — the weakest contrast in the whole UI, on the row that matters most).
pub const BG_SEL: Bgra = bgra(0x0b, 0x62, 0x85, 0xff);
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

#[allow(clippy::too_many_arguments)] // a rectangle is six numbers plus the buffer
pub fn rect(buf: &mut [u8], w: u32, h: u32, x: u32, y: u32, rw: u32, rh: u32, color: Bgra) {
    let x0 = (x as i64).clamp(0, w as i64) as u32;
    let y0 = (y as i64).clamp(0, h as i64) as u32;
    let x1 = ((x + rw) as i64).clamp(0, w as i64) as u32;
    let y1 = ((y + rh) as i64).clamp(0, h as i64) as u32;
    for py in y0..y1 {
        let row = &mut buf[(py * w + x0) as usize * 4..(py * w + x1) as usize * 4];
        for px in row.as_chunks_mut::<4>().0 {
            *px = color;
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
    let scale = || PxScale::from(font.size);
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
        // Walk the face chain (primary first); draw with the first face that
        // has this glyph so CJK etc. fall back to a system font. A face beyond
        // the primary is read on first use here.
        for face_idx in 0..font.face_count() {
            let Some(face) = font.face(face_idx) else {
                continue; // unreadable face, try the next
            };
            let scaled = face.as_scaled(scale());
            let gid = scaled.glyph_id(ch);
            if gid == GlyphId(0) {
                continue; // no glyph in this face, try the next
            }
            let advance = scaled.h_advance(gid);
            if cx + advance > end {
                break 'ch;
            }
            let key: crate::font::GlyphKey =
                (face_idx as u8, gid.0, bucket(cx), bucket(baseline), mode);
            if !font.glyphs.borrow().contains_key(&key) {
                if font.glyphs.borrow().len() >= GLYPH_CACHE_CAP {
                    font.glyphs.borrow_mut().clear();
                }
                // Rasterize at the canonical pen (subpixel part only), then blit
                // at the whole-pixel part — the two differ by an integer shift.
                let planes = match subpixel {
                    // One neutral plane, used for every channel.
                    Subpixel::Gray => {
                        vec![raster(
                            &face,
                            gid,
                            scale(),
                            point(canonical(cx), canonical(baseline)),
                        )]
                    }
                    // One plane per subpixel column: each carries one channel.
                    _ => (0..3)
                        .map(|k| {
                            let off = k as f32 / SUBPIXEL_SAMPLES;
                            raster(
                                &face,
                                gid,
                                scale(),
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
            continue 'ch;
        }
        // No face in the chain has this glyph; skip it silently.
    }
    cx
}

/// Advance width of `s` under the same face-chain rules as `draw_text`, capped
/// at `max_w`. The layout uses it to decide where a row must break.
pub(crate) fn measure(font: &crate::font::PanelFont, s: &str, max_w: f32) -> f32 {
    let scale = || PxScale::from(font.size);
    let mut cx = 0.0;
    'ch: for ch in s.chars() {
        for face_idx in 0..font.face_count() {
            let Some(face) = font.face(face_idx) else {
                continue;
            };
            let scaled = face.as_scaled(scale());
            let gid = scaled.glyph_id(ch);
            if gid == GlyphId(0) {
                continue;
            }
            let adv = scaled.h_advance(gid);
            if cx + adv > max_w {
                break 'ch;
            }
            cx += adv;
            continue 'ch;
        }
    }
    cx
}

pub(crate) fn row_baseline(font: &crate::font::PanelFont, row_y: u32) -> f32 {
    let top = row_y as f32 + (font.row_h as f32 - font.line_h) / 2.0;
    top + font.ascent_px
}

/// Rasterize one coverage plane of `gid` at `pos`.
fn raster(
    face: &ab_glyph::FontVec,
    gid: GlyphId,
    scale: PxScale,
    pos: ab_glyph::Point,
) -> crate::font::GlyphPlane {
    let Some(outline) = face.outline_glyph(Glyph {
        id: gid,
        scale,
        position: pos,
    }) else {
        return crate::font::GlyphPlane::default();
    };
    let b = outline.px_bounds();
    // px_bounds is an f32 rect and `draw` iterates width()×height() from `min`,
    // so both must stay in the same units.
    let (w, h) = (b.width() as u32, b.height() as u32);
    let mut coverage = Vec::with_capacity((w * h) as usize);
    outline.draw(|_, _, cov| coverage.push((cov * 255.0).round() as u8));
    crate::font::GlyphPlane {
        min_x: b.min.x as i32,
        min_y: b.min.y as i32,
        w,
        h,
        coverage,
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
    // dst alpha stays 255 (fully opaque menu)
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
}
