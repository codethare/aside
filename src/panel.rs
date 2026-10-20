//! Paints the panel: the input bar as the top row, the transcript below it,
//! both inside one rounded outline.
//!
//! Frame and geometry come from `wlmenu` unchanged: the panel floats inside a
//! 24 px shadow margin, its body is premultiplied translucent glass with a
//! hairline border, and the bar is the same band it draws (a 4 px
//! `LABEL_ACCENT` rail, an 8 px gap, a 1 px divider at 25 % alpha). The bar is
//! the first row, because a top-anchored layer surface can only grow downwards.
//! The thinking state is a third drawing path, a circle with four blinking
//! squares.

use crate::font::PanelFont;
use crate::layout::{self, Blocks, Input, MAX_ROWS, Viewport};
use crate::render::{self, Bgra, Subpixel};
use crate::transcript::{Kind, Row, Transcript};

/// Paints the bar and the transcript. `buf` is `w * h * 4` bytes, BGRA, and
/// holds the panel plus its shadow margin on every side.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    buf: &mut [u8],
    w: u32,
    h: u32,
    font: &PanelFont,
    transcript: &Transcript,
    input: &Input,
    viewport: &Viewport,
    highlight: Option<usize>,
    streaming: bool,
    caret_visible: bool,
    preedit: &str,
    scale: u32,
) {
    let s = scale.max(1);
    let aa = layout::text_aa(s);
    let bar_h = font.row_h;
    let Some(panel) = panel_of(w, h, render::CORNER_RADIUS, s) else {
        buf.fill(0);
        return;
    };

    // The bar is the panel's bottom row: the replies grow upwards out of it, and
    // the bar keeps one place on screen because the surface hangs from the bottom
    // edge.
    let divider_h = render::DIVIDER_H * s;
    let bar_top = panel.h.saturating_sub(bar_h);
    let divider_top = bar_top.saturating_sub(divider_h);

    render::shadow(buf, w, h, &panel, s);
    body(buf, w, &panel, bar_top, panel.h - bar_top);

    if panel.h > bar_h {
        // Inset by the content padding: the hairline separates the replies from
        // the bar without touching the border ring.
        let (x0, x1) = panel.span(divider_top);
        let mut divider = render::premul(render::BG_PROMPT);
        render::over_pm(&mut divider, render::LABEL_ACCENT, render::DIVIDER_ALPHA);
        let (dx0, dx1) = (x0 + crate::PAD * s, x1.saturating_sub(crate::PAD * s));
        if dx0 < dx1 {
            render::set_span(buf, w, panel.y + divider_top, dx0, dx1, divider);
        }
    }

    // Transcript: the blocks the viewport shows, from the panel's top edge down to
    // the bar. A card is filled once, when its first visible row comes up; every
    // row then draws its own text over it.
    let blocks = Blocks::new(font);
    let rows = transcript.rows(blocks);
    let card = card_color();
    let clip = panel.y..(panel.y + divider_top);
    let mut filled: Option<u32> = None;
    for (index, row) in rows
        .iter()
        .enumerate()
        .filter(|(_, row)| viewport.shows(row.y, blocks.row_h))
    {
        let y = panel.y + row.y;
        if row.kind.is_card() && filled != Some(row.block.0) {
            filled = Some(row.block.0);
            render::rounded_rect(
                buf,
                w,
                panel.x + crate::PAD * s,
                panel.y + row.block.0,
                panel.w - 2 * crate::PAD * s,
                row.block.1,
                render::CORNER_RADIUS * s,
                card,
                clip.clone(),
            );
        }
        draw_row(
            buf,
            w,
            &panel,
            font,
            row,
            y,
            blocks,
            highlight == Some(index),
            aa,
            s,
        );
    }

    // Scroll position indicator: only while the transcript overflows the viewport.
    let track_y = panel.y;
    let track_h = divider_top;
    let total = transcript.total_height();
    if let Some((ty, th)) =
        render::scroll_thumb(track_y, track_h, viewport.height, total, viewport.top)
    {
        let tw = 3 * s;
        let tx = panel.x + panel.w - (crate::PAD * s + tw) / 2;
        render::rect(buf, w, h, tx, ty, tw, th, render::LABEL_ACCENT);
    }

    draw_bar(
        buf,
        w,
        h,
        &panel,
        font,
        input,
        bar_h,
        streaming,
        caret_visible,
        preedit,
        aa,
        s,
    );
}

/// The panel inside the shadow margin, or `None` when the buffer is too small
/// to hold one.
fn panel_of(w: u32, h: u32, radius: u32, scale: u32) -> Option<render::Panel> {
    let mo = render::SHADOW * scale.max(1);
    if w <= 2 * mo || h <= 2 * mo {
        return None;
    }
    Some(render::Panel {
        x: mo,
        y: mo,
        w: w - 2 * mo,
        h: h - 2 * mo,
        r: radius * scale.max(1),
    })
}

/// The translucent body, its antialiased edge and its hairline border. `band_top`
/// and `band_h` place the `BG_PROMPT` band: the bar's rows at the bottom of the
/// panel, or the whole panel for the ball.
fn body(buf: &mut [u8], w: u32, panel: &render::Panel, band_top: u32, band_h: u32) {
    let (bg_normal, bg_prompt) = (
        render::premul(render::BG_NORMAL),
        render::premul(render::BG_PROMPT),
    );
    for rel in 0..panel.h {
        let by = panel.y + rel;
        let inset = render::rounded_inset(panel.w, panel.h, panel.r, rel);
        let (x0, x1) = panel.span(rel);
        let in_band = rel >= band_top && rel < band_top + band_h;
        let bg = if in_band { bg_prompt } else { bg_normal };
        render::set_span(buf, w, by, x0, x1, bg);
        // What is left of the fractional corner inset is the coverage of the
        // pixel just outside the span, which is what antialiases the edge.
        let cov = ((inset.ceil() - inset) * 255.0).round() as u32;
        if cov > 0 && x0 > 0 {
            render::over_pm(&mut buf[((by * w + x0 - 1) * 4) as usize..][..4], bg, cov);
        }
        if cov > 0 && x1 < w {
            render::over_pm(&mut buf[((by * w + x1) * 4) as usize..][..4], bg, cov);
        }
        // Hairline on the outermost pixel: lit along the top, dark underneath,
        // so the shadow reads as growing out of the edge.
        let t = 1.0 - rel as f32 / panel.h as f32;
        let light = (render::BORDER_LIGHT_ALPHA as f32 * t * t).round() as u32;
        if x0 < x1 {
            for x in [x0, x1 - 1] {
                let dst = &mut buf[((by * w + x) * 4) as usize..][..4];
                render::over_pm(dst, [0, 0, 0, 0xff], render::BORDER_DARK_ALPHA);
                render::over_pm(dst, [0xff, 0xff, 0xff, 0xff], light);
            }
        }
    }
}

/// Paints the bar alone, on the panel's bottom row: rail, typed text, caret.
#[allow(clippy::too_many_arguments)]
fn draw_bar(
    buf: &mut [u8],
    w: u32,
    h: u32,
    panel: &render::Panel,
    font: &PanelFont,
    input: &Input,
    row_h: u32,
    streaming: bool,
    caret_visible: bool,
    preedit: &str,
    aa: Subpixel,
    s: u32,
) {
    let p = crate::PAD * s;
    // The bar is the last row of the panel; the surface hangs from the bottom
    // edge, so that row keeps one place on screen while replies grow above it.
    let bar_y = panel.y + panel.h.saturating_sub(row_h);
    let baseline = render::row_baseline(font, bar_y);
    // `line_h` is already in buffer pixels (the font is derived at `SIZE * scale`);
    // only the padding is a logical constant. Scaling the whole sum again made the
    // rail taller than the row at 2× and the subtraction below panicked.
    let rail_h = font.line_h.ceil() as u32 + 2 * render::PILL_VPAD * s;
    let rail_y = bar_y + row_h.saturating_sub(rail_h) / 2;
    render::rect(
        buf,
        w,
        h,
        panel.x + p,
        rail_y,
        render::LABEL_RAIL_W * s,
        rail_h,
        layout::rail_color(streaming),
    );
    let gap = render::PROMPT_GAP * s as f32;
    let text_x = (panel.x + p) as f32 + (render::LABEL_RAIL_W * s) as f32 + gap;
    let limit = (panel.x + panel.w) as f32 - p as f32 - text_x;
    let typed = input.text();
    let mut end = render::draw_text(
        buf,
        w,
        h,
        font,
        text_x,
        baseline,
        typed,
        render::FG_PROMPT,
        limit.max(0.0),
        aa,
    );
    // A composition in flight is not committed text: it is drawn where it will
    // land, underlined, so the line shows what the input method still owns. With
    // no pre-edit the bar is exactly what it was.
    if !preedit.is_empty() {
        let end_x = render::draw_text(
            buf,
            w,
            h,
            font,
            end,
            baseline,
            preedit,
            render::FG_PROMPT,
            (text_x + limit.max(0.0) - end).max(0.0),
            aa,
        );
        let underline = (baseline - font.descent_px).round().max(0.0) as u32;
        render::rect(
            buf,
            w,
            h,
            end as u32,
            underline,
            (end_x - end).max(1.0) as u32,
            s,
            render::FG_PROMPT,
        );
        end = end_x;
    }
    let caret_w = 2 * s;
    let caret_bottom = (baseline - font.descent_px).round().max(0.0) as u32;
    let caret_h = font.line_h.ceil() as u32;
    if caret_visible && caret_bottom > 0 {
        let caret_top = caret_bottom.saturating_sub(caret_h);
        let caret_x = if typed.is_empty() && preedit.is_empty() {
            text_x
        } else {
            end
        } as u32
            + s;
        let right = (panel.x + panel.w).saturating_sub(caret_w);
        render::rect(
            buf,
            w,
            h,
            caret_x.min(right),
            caret_top,
            caret_w,
            caret_bottom.saturating_sub(caret_top),
            render::FG_PROMPT,
        );
    }
}

/// The thinking ball: a filled circle on the cleared surface, with four `FG_PROMPT`
/// squares in a 2×2 grid, one lit. `lit` comes from the clock, never from a frame
/// counter, so a late frame lands on the right square.
pub fn draw_ball(buf: &mut [u8], w: u32, h: u32, lit: usize, scale: u32) {
    let s = scale.max(1);
    let Some(mut panel) = panel_of(w, h, render::CORNER_RADIUS, s) else {
        buf.fill(0);
        return;
    };
    // A radius of half the panel makes the outline a circle; `rounded_span`
    // clamps it to half of whichever side is shorter.
    panel.r = panel.w.min(panel.h);
    // The band is the whole panel, so the body takes the prompt tone throughout.
    render::shadow(buf, w, h, &panel, s);
    body(buf, w, &panel, 0, panel.h);
    // 4 px squares, 2 px gaps: a 2×2 grid centred in the ball.
    let cell = 4 * s;
    let gap = 2 * s;
    let grid = cell * 2 + gap;
    let left = panel.x + (panel.w - grid) / 2;
    let top = panel.y + (panel.h - grid) / 2;
    for i in 0..4 {
        let (col, row) = (i as u32 % 2, i as u32 / 2);
        let mut filled = render::premul(render::BG_PROMPT);
        if i == lit {
            filled = render::FG_PROMPT;
        } else {
            // Unlit squares stay visible as a dim dot rather than vanishing.
            render::over_pm(&mut filled, render::LABEL_ACCENT, render::DIVIDER_ALPHA);
        }
        render::rect(
            buf,
            w,
            h,
            left + col * (cell + gap),
            top + row * (cell + gap),
            cell,
            cell,
            filled,
        );
    }
}

/// One transcript row: the rail when it is highlighted, and the text in its
/// kind's colour, inset into its card when it is inside one.
#[allow(clippy::too_many_arguments)] // a row is seven facts about where it goes
fn draw_row(
    buf: &mut [u8],
    w: u32,
    panel: &render::Panel,
    font: &PanelFont,
    row: &Row<'_>,
    y: u32,
    blocks: Blocks,
    highlighted: bool,
    aa: Subpixel,
    s: u32,
) {
    let pad = if row.kind.is_card() { blocks.pad } else { 0 };
    if highlighted {
        // A rail in the margin, not a band across the row: the transcript is a
        // stack of blocks, not a list. It clears a card's edge and a question's
        // first glyph by a pixel either way.
        let bottom = panel.y + panel.h;
        render::rect(
            buf,
            w,
            bottom,
            panel.x + crate::PAD * s - 3 * s,
            y,
            2 * s,
            (y + blocks.row_h).min(bottom) - y,
            render::BG_SEL,
        );
    }
    let (fg, _) = colours(row.kind, highlighted);
    // Clipped at the panel's bottom edge, so ink never runs into the shadow.
    render::draw_text(
        buf,
        w,
        panel.y + panel.h,
        font,
        (panel.x + crate::PAD * s + pad) as f32,
        render::row_baseline(font, y),
        row.text,
        fg,
        (panel.w - 2 * (crate::PAD * s + pad)) as f32,
        aa,
    );
}

/// The card's fill, composited once over the transcript's body: the card is
/// translucent, and every card pixel sits on exactly that body.
fn card_color() -> Bgra {
    let mut px = render::premul(render::BG_NORMAL);
    render::over_pm(&mut px, render::premul(render::BG_CARD), 255);
    px
}

fn colours(kind: Kind, highlighted: bool) -> (Bgra, Bgra) {
    if highlighted {
        return (render::FG_SEL, render::BG_SEL);
    }
    match kind {
        Kind::Question => (render::FG_PROMPT, render::BG_NORMAL),
        Kind::Answer | Kind::Note | Kind::Error => (render::FG_NORMAL, render::BG_NORMAL),
    }
}

/// Transcript height a panel of `h` buffer pixels can show under its bar, in
/// buffer pixels: the rows the spec caps, expressed where the painter needs it.
pub fn transcript_px(h: u32, font: &PanelFont, scale: u32) -> u32 {
    let s = scale.max(1);
    let bar = font.row_h + render::DIVIDER_H * s;
    h.saturating_sub(bar).min(MAX_ROWS as u32 * font.row_h)
}

/// Panel height, in buffer pixels without the shadow margin, for a transcript
/// that draws `content` buffer pixels: the bar, the divider and the visible
/// slice of the blocks.
pub fn height_for(content: u32, font: &PanelFont, scale: u32) -> u32 {
    let s = scale.max(1);
    font.row_h + render::DIVIDER_H * s + content.min(MAX_ROWS as u32 * font.row_h)
}

/// The bar's height in logical pixels. The font's own metrics are in buffer
/// pixels (it is derived at `SIZE * scale`), so this is the one number the layer
/// surface, which is sized logically, needs.
pub fn bar_height() -> u32 {
    crate::font::row_h_for(crate::font::SIZE)
}

/// A PPM of the current frame: what `--preview` writes, so the layout can be
/// looked at on a machine with no compositor. PPM because it needs no encoder.
pub fn to_ppm(buf: &[u8], w: u32, h: u32) -> Vec<u8> {
    let mut out = format!("P6\n{w} {h}\n255\n").into_bytes();
    out.reserve((w * h * 3) as usize);
    for px in buf.as_chunks::<4>().0 {
        out.extend_from_slice(&[px[2], px[1], px[0]]); // BGRA -> RGB
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    fn scene(font: &PanelFont, w: f32) -> Scene {
        let mut transcript = Transcript::default();
        transcript.ask("why is the sky blue?", font);
        transcript.delta(
            "Rayleigh scattering: shorter wavelengths scatter more, so the sky reads blue.",
            font,
            w,
        );
        transcript.finish_answer(font);
        let input = Input::default();
        let viewport = Viewport::new(MAX_ROWS as u32 * 30);
        Scene {
            transcript,
            input,
            viewport,
        }
    }

    struct Scene {
        transcript: Transcript,
        input: Input,
        viewport: Viewport,
    }

    /// Buffer size for a panel of `w`×`h` buffer px: the panel plus its shadow
    /// margin on both sides, as `main` sizes the surface.
    fn frame(w: u32, h: u32) -> (u32, u32) {
        (w + 2 * render::SHADOW, h + 2 * render::SHADOW)
    }

    /// Byte index of panel-relative (x, y) in a buffer `bw` px wide.
    fn at(bw: u32, x: u32, y: u32) -> usize {
        (((y + render::SHADOW) * bw + x + render::SHADOW) * 4) as usize
    }

    fn paint(scene: &Scene, streaming: bool, w: u32, h: u32) -> Vec<u8> {
        let f = font();
        let (bw, bh) = frame(w, h);
        let mut buf = vec![0u8; (bw * bh * 4) as usize];
        draw(
            &mut buf,
            bw,
            bh,
            &f,
            &scene.transcript,
            &scene.input,
            &scene.viewport,
            None,
            streaming,
            true,
            "",
            1,
        );
        buf
    }

    #[test]
    fn the_panel_floats_in_a_shadow_margin() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(8 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[((y * bw + x) * 4) as usize..][..4];
        // The margin is empty except for the shadow the panel casts into it.
        assert_eq!(px(0, 0), &[0, 0, 0, 0], "buffer corner is clear");
        assert_eq!(px(0, bh / 2)[3], 0, "outside the shadow reach stays clear");
        // Hard against the panel's left edge there is shadow, not panel.
        let inside_shadow = px(render::SHADOW - 1, bh / 2);
        assert!(inside_shadow[3] > 0, "the panel casts a shadow");
        assert_eq!(inside_shadow[..3], [0, 0, 0], "shadow is black");
        // ...and the body is the translucent glass, premultiplied.
        // Left of the card's inset: the transcript area is the plain body.
        assert_eq!(
            &buf[at(bw, crate::PAD - 2, 2)..][..4],
            render::premul(render::BG_NORMAL),
            "the body beside the card"
        );
    }

    #[test]
    fn the_corner_is_antialiased_not_staircased() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, _bh) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        // The pixel just outside the whole-pixel span carries partial coverage;
        // without it the curve would step in whole pixels.
        let (x0, _) = render::rounded_span(w, h, render::CORNER_RADIUS, 0);
        let outer = &buf[at(bw, x0.saturating_sub(1), 0)..][..4];
        assert!(
            outer[3] > 0 && outer[3] < render::BG_PROMPT[3],
            "edge pixel {outer:?} is neither clear nor full body"
        );
    }

    #[test]
    fn the_input_bar_is_the_prompt_band_at_the_bottom() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        // The bar's band is the last row of the panel; the replies are above it.
        assert_eq!(
            px(w / 2, h - 4),
            render::premul(render::BG_PROMPT),
            "bar band at the bottom"
        );
        assert_eq!(
            px(crate::PAD - 2, 2),
            render::premul(render::BG_NORMAL),
            "the replies above the bar"
        );
    }

    #[test]
    fn the_label_rail_sits_in_the_bar() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        // The rail is the accent colour, just inside PAD, mid-bar on the left.
        let y = h - f.row_h / 2;
        let rail = &buf[at(bw, crate::PAD + 1, y)..][..4];
        assert_eq!(
            &rail[..3],
            &render::LABEL_ACCENT[..3],
            "rail at the bar's left"
        );
    }

    /// Composing with an input method has to be visible: the pre-edit is what
    /// tells the user their keystrokes went somewhere, before anything commits.
    #[test]
    fn a_pre_edit_is_drawn_after_the_typed_text() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.input.insert("why");
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let paint_with = |preedit: &str| {
            let mut buf = vec![0u8; (bw * bh * 4) as usize];
            draw(
                &mut buf,
                bw,
                bh,
                &f,
                &scene.transcript,
                &scene.input,
                &scene.viewport,
                None,
                false,
                true,
                preedit,
                1,
            );
            buf
        };
        let bare = paint_with("");
        let composing = paint_with("zhong");
        // Right of the typed text the bar is empty until the input method speaks.
        let text_x = crate::PAD + render::LABEL_RAIL_W + render::PROMPT_GAP as u32;
        let from = text_x + render::measure(&f, "why", f32::MAX).ceil() as u32 + 8;
        let ink = |buf: &[u8]| {
            let text_rows = h - f.row_h..h - render::DIVIDER_H;
            (from..from + 60)
                .flat_map(|x| text_rows.clone().map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    buf[at(bw, x, y)..][..4][..3] != render::premul(render::BG_PROMPT)[..3]
                })
                .count()
        };
        assert_eq!(ink(&bare), 0, "nothing is drawn after the typed text");
        assert!(ink(&composing) > 0, "the composition is visible");
    }

    #[test]
    fn the_divider_is_inset_by_the_padding() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        let y = h - f.row_h - render::DIVIDER_H;
        let band = px(w / 2, y - 1);
        let mut expected = render::premul(render::BG_PROMPT);
        render::over_pm(&mut expected, render::LABEL_ACCENT, render::DIVIDER_ALPHA);
        assert_eq!(px(w / 2, y), expected, "the divider is the tinted band");
        assert_eq!(px(crate::PAD - 1, y), band, "inset starts at PAD");
        assert_eq!(px(crate::PAD, y), expected, "first inset pixel");
        assert_eq!(px(w - crate::PAD - 1, y), expected, "last inset pixel");
        assert_eq!(px(w - crate::PAD, y), band, "and stops there");
    }

    #[test]
    fn the_highlight_is_a_rail_not_a_band() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let mut buf = vec![0u8; (bw * bh * 4) as usize];
        draw(
            &mut buf,
            bw,
            bh,
            &f,
            &scene.transcript,
            &scene.input,
            &scene.viewport,
            Some(0),
            false,
            true,
            "",
            1,
        );
        let y = f.row_h / 2;
        let px = |x: u32| &buf[at(bw, x, y)..][..4];
        // The rail is the selection colour, 2 px in the margin left of the text;
        // `rect` composites it over the body, so that is what it must match.
        let mut rail = render::premul(render::BG_NORMAL);
        render::blend_px(
            &mut rail,
            [render::BG_SEL[0], render::BG_SEL[1], render::BG_SEL[2]],
            render::BG_SEL[3] as u32,
        );
        assert_eq!(px(crate::PAD - 3), rail, "the rail sits in the margin");
        assert_eq!(px(crate::PAD - 2), rail, "two pixels wide");
        assert_ne!(
            px(crate::PAD - 1)[..3],
            rail[..3],
            "and stops before the text"
        );
        // Nothing else of the row carries the selection colour: there is no band.
        for x in crate::PAD..w - crate::PAD {
            assert_ne!(px(x)[..3], rail[..3], "the row is not banded at x={x}");
        }
    }

    /// The arrangement the redesign is for: the reply in a card above, the bar
    /// below it on the panel's bottom row, and no question anywhere.
    #[test]
    fn the_reply_is_a_card_above_the_bar() {
        let f = font();
        let blocks = Blocks::new(&f);
        let scene = scene(&f, 300.0);
        let content = scene.transcript.total_height();
        let (w, h) = (500u32, height_for(content, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        let body = render::premul(render::BG_NORMAL);
        let rows = scene.transcript.rows(blocks);
        let answer = rows
            .iter()
            .find(|r| r.kind == Kind::Answer)
            .expect("an answer");
        assert!(
            rows.iter().all(|r| r.kind != Kind::Question),
            "the question is drawn nowhere"
        );

        // The card starts at the panel's top edge, inset by PAD, its padding
        // filled.
        let card = card_color();
        let card_top = answer.block.0;
        let mid = card_top + answer.block.1 / 2;
        assert_eq!(card_top, 0, "the replies start at the panel's top");
        assert_eq!(px(crate::PAD + 2, mid), card, "the card is filled");
        assert_eq!(px(crate::PAD - 1, mid), body, "inset by PAD");
        // Its text is inset a further 8 px, so the left column of the card is
        // padding and not glyph.
        assert_eq!(
            px(crate::PAD + 5, card_top + blocks.pad + f.row_h / 2),
            card
        );

        // The bar is the last row, in the prompt band, and no card reaches it.
        let bar_y = h - f.row_h / 2;
        assert_eq!(
            px(w / 2, bar_y - f.row_h / 2 + 1),
            render::premul(render::BG_PROMPT),
            "the bar's band"
        );
        assert_ne!(px(w / 2, bar_y), card, "nothing draws a card over the bar");
    }

    #[test]
    fn the_scroll_thumb_shows_only_when_the_transcript_overflows() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.viewport = Viewport::new(2 * f.row_h);
        let (w, h) = (500u32, height_for(2 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, false, w, h);
        let tw = 3;
        let tx = w - (crate::PAD + tw) / 2;
        assert_eq!(
            &buf[at(bw, tx, f.row_h + render::DIVIDER_H)..][..4][..3],
            &render::LABEL_ACCENT[..3],
            "the overflow indicator is on the right edge"
        );
        // Everything fits once the viewport is tall enough: no indicator.
        scene.viewport = Viewport::new(MAX_ROWS as u32 * 30);
        let short = paint(&scene, false, w, h);
        assert_ne!(
            &short[at(bw, tx, f.row_h + render::DIVIDER_H)..][..4][..3],
            &render::LABEL_ACCENT[..3]
        );
    }

    #[test]
    fn the_ball_has_a_lit_square_that_moves() {
        let (panel, scale) = (48u32, 1u32);
        let (w, h) = frame(panel, panel);
        let frame_at = |lit: usize| {
            let mut buf = vec![0u8; (w * h * 4) as usize];
            draw_ball(&mut buf, w, h, lit, scale);
            buf
        };
        let a = frame_at(0);
        let b = frame_at(1);
        assert_ne!(a, b, "a later phase lights a different square");
        // Every density paints the same circle instead of panicking on the radius.
        for scale in [1u32, 2, 3] {
            let side = panel * scale;
            let mut buf = vec![0u8; ((side + 2 * render::SHADOW * scale).pow(2) * 4) as usize];
            let w = side + 2 * render::SHADOW * scale;
            draw_ball(&mut buf, w, w, 0, scale);
        }
        // The circle is round: corners stay clear, the middle is opaque.
        fn px(buf: &[u8], w: u32, x: u32, y: u32) -> &[u8] {
            &buf[at(w, x, y)..][..4]
        }
        assert_eq!(px(&a, w, 0, 0), &[0, 0, 0, 0], "corner stays clear");
        assert_eq!(
            px(&a, w, panel / 2, panel / 2)[3],
            render::BG_PROMPT[3],
            "centre is the glass body"
        );
    }

    #[test]
    fn every_scale_paints_instead_of_panicking() {
        // The font is derived at `SIZE * scale`, so its metrics are already in
        // buffer pixels. Scaling one of them a second time made the input rail
        // taller than its row at 2x, and `row_h - rail_h` panicked before a single
        // pixel was drawn — on a HiDPI output that means no panel at all.
        for scale in [1u32, 2, 3] {
            let f = PanelFont::load(crate::font::SIZE * scale as f32).expect("system font");
            let (pw, ph) = (500 * scale, height_for(4 * f.row_h, &f, scale));
            let (w, h) = (
                pw + 2 * render::SHADOW * scale,
                ph + 2 * render::SHADOW * scale,
            );
            let scene = scene(&f, (pw - 2 * crate::PAD * scale) as f32);
            let mut buf = vec![0u8; (w * h * 4) as usize];
            draw(
                &mut buf,
                w,
                h,
                &f,
                &scene.transcript,
                &scene.input,
                &scene.viewport,
                None,
                false,
                true,
                "",
                scale,
            );
            assert!(
                buf.as_chunks::<4>().0.iter().any(|p| p[3] > 0),
                "scale {scale} painted nothing"
            );
        }
    }

    #[test]
    fn cjk_rows_paint_without_panicking_at_the_edges() {
        let f = font();
        let mut scene = scene(&f, 120.0);
        scene.transcript.delta(
            "很长的一段中文回答，需要在很窄的面板里折行显示，不能越界",
            &f,
            120.0,
        );
        let (w, h) = (140u32, height_for(8 * f.row_h, &f, 1));
        let buf = paint(&scene, true, w, h);
        let ink = |p: &[u8]| p[..3] != render::premul(render::BG_NORMAL)[..3];
        // Something was painted, and all of it inside the panel.
        assert!(buf.as_chunks::<4>().0.iter().any(|p| ink(p)));
    }

    #[test]
    fn the_visible_transcript_height_follows_the_surface_and_the_cap() {
        let f = font();
        let content = |rows: u32| rows * f.row_h;
        assert_eq!(
            transcript_px(height_for(content(6), &f, 1), &f, 1),
            content(6)
        );
        // The cap holds even for a panel taller than the viewport allows.
        assert_eq!(transcript_px(10_000, &f, 1), MAX_ROWS as u32 * f.row_h);
        assert_eq!(transcript_px(1, &f, 1), 0, "too short for a bar and a row");
        // At 2x the font is re-derived twice as large, so the same eight rows
        // take twice the buffer and still fit the same panel.
        let big = PanelFont::load(crate::font::SIZE * 2.0).expect("system font");
        assert_eq!(
            transcript_px(height_for(8 * big.row_h, &big, 2), &big, 2),
            8 * big.row_h
        );
        assert!(height_for(8 * big.row_h, &big, 2) > height_for(8 * f.row_h, &f, 1));
    }

    #[test]
    fn the_preview_encoder_writes_a_ppm_in_rgb_order() {
        let buf = vec![0x11, 0x22, 0x33, 0xff];
        let ppm = to_ppm(&buf, 1, 1);
        assert_eq!(&ppm[..11], b"P6\n1 1\n255\n");
        assert_eq!(&ppm[11..], &[0x33, 0x22, 0x11], "BGRA becomes RGB");
    }

    #[test]
    fn a_preview_of_a_scripted_conversation_renders() {
        let f = font();
        let w = 640u32;
        let h = height_for(12 * f.row_h, &f, 1);
        let (bw, bh) = frame(w, h);
        let mut scene = scene(&f, (w - 2 * crate::PAD) as f32);
        scene.transcript.fail(
            "the provider refused the request (401)",
            &f,
            (w - 2 * crate::PAD) as f32,
        );
        let mut buf = vec![0u8; (bw * bh * 4) as usize];
        draw(
            &mut buf,
            bw,
            bh,
            &f,
            &scene.transcript,
            &scene.input,
            &scene.viewport,
            None,
            false,
            true,
            "",
            1,
        );
        assert!(buf.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
    }
}
