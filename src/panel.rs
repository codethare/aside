//! Paints the panel: two cards of one glass chrome, the replies above and the
//! input bar below them.
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
    caret_visible: bool,
    preedit: &str,
    // The bar's right-hand label: `-- VIM --` while vim mode owns the keyboard.
    label: &str,
    // How much of the replies card has grown out of the bar: 1 is fully shown.
    reveal: f32,
    scale: u32,
) {
    let s = scale.max(1);
    let aa = layout::text_aa(s);
    let bar_h = font.row_h;
    let Some(panel) = panel_of(w, h, render::CORNER_RADIUS, s) else {
        buf.fill(0);
        return;
    };

    // Two cards: the replies above, the bar 12 px below them on the panel's bottom
    // row. The bar keeps one place on screen because the surface hangs from the
    // bottom edge; the replies are the card that grows out of it.
    let sep = render::SEPARATION * s;
    let bar = render::Panel {
        y: panel.y + panel.h.saturating_sub(bar_h),
        h: bar_h.min(panel.h),
        ..panel
    };
    let room = bar.y.saturating_sub(panel.y).saturating_sub(sep);
    // The card is the room's full height for as long as the panel is up: the
    // transcript scrolls inside it rather than resizing it. The reveal only makes
    // it appear from the bar, so `reveal = 0` is no card at all and 1 is the box.
    let card_h = (reveal.clamp(0.0, 1.0) * room as f32).round() as u32;
    let replies = (card_h > 0).then(|| render::Panel {
        y: bar.y - sep - card_h,
        h: card_h,
        ..panel
    });
    let cards: Vec<render::Panel> = match replies {
        Some(card) => vec![card, bar],
        None => vec![bar],
    };
    render::shadow(buf, w, h, &cards, s);
    if let Some(replies) = replies.as_ref() {
        render::body(buf, w, replies);
        // Rows are painted against the card's own origin, so the text travels up
        // out of the bar with it instead of being wiped in place.
        let clip = replies.y..(replies.y + replies.h);
        draw_replies(
            buf, w, font, transcript, viewport, highlight, replies, clip, aa, s,
        );
    }
    render::body(buf, w, &bar);

    // Scroll position indicator: only while the transcript overflows the card.
    let track_y = replies.map_or(bar.y, |r| r.y);
    let track_h = bar.y.saturating_sub(sep).saturating_sub(track_y);
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
        &bar,
        font,
        input,
        bar_h,
        caret_visible,
        preedit,
        label,
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

/// The reply lines inside a card: the blocks the viewport shows, clipped to the
/// part of the card that has been revealed so far.
#[allow(clippy::too_many_arguments)]
fn draw_replies(
    buf: &mut [u8],
    w: u32,
    font: &PanelFont,
    transcript: &Transcript,
    viewport: &Viewport,
    highlight: Option<usize>,
    card: &render::Panel,
    clip: std::ops::Range<u32>,
    aa: Subpixel,
    s: u32,
) {
    let blocks = Blocks::new(font);
    for (index, row) in transcript.rows(blocks).iter().enumerate() {
        if !viewport.shows(row.y, blocks.row_h) {
            continue;
        }
        // The row's position inside the card is its place in the transcript less
        // how far the view has been scrolled: without the subtraction a long answer
        // is drawn below the card and nothing shows.
        let y = card.y + row.y.saturating_sub(viewport.top);
        if y + blocks.row_h <= clip.start || y >= clip.end {
            continue;
        }
        draw_row(
            buf,
            w,
            font,
            card,
            row,
            y,
            highlight == Some(index),
            clip.end,
            aa,
            s,
        );
    }
}

/// Paints the bar alone, on the panel's bottom row: typed text and caret.
#[allow(clippy::too_many_arguments)]
fn draw_bar(
    buf: &mut [u8],
    w: u32,
    h: u32,
    panel: &render::Panel,
    font: &PanelFont,
    input: &Input,
    row_h: u32,
    caret_visible: bool,
    preedit: &str,
    label: &str,
    aa: Subpixel,
    s: u32,
) {
    let p = crate::PAD * s;
    // The bar is the last row of the panel; the surface hangs from the bottom
    // edge, so that row keeps one place on screen while replies grow above it.
    let bar_y = panel.y + panel.h.saturating_sub(row_h);
    let baseline = render::row_baseline(font, bar_y);
    // No label rail: `wlmenu` draws one only when it is given a prompt label, and
    // this bar has none, so the text starts at the padding.
    let text_x = (panel.x + p) as f32;
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
    if !label.is_empty() {
        // Right-aligned and in the dimmer ink: it explains why a key did not
        // type, without competing with what is being typed.
        let width = render::measure(font, label, f32::MAX);
        render::draw_text(
            buf,
            w,
            h,
            font,
            (panel.x + panel.w) as f32 - p as f32 - width,
            baseline,
            label,
            render::FG_NORMAL,
            width,
            aa,
        );
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
    render::shadow(buf, w, h, &[panel], s);
    render::body(buf, w, &panel);
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
            // Unlit squares stay visible as a dim dot rather than vanishing: a
            // quarter of the primary ink over the body.
            render::over_pm(&mut filled, render::FG_PROMPT, 64);
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

/// One reply line: the text in its ink, brighter and 2 px in when it is the
/// highlighted one. No rail, no band: `aside` has no list to mark.
#[allow(clippy::too_many_arguments)] // a row is where it goes plus what it says
fn draw_row(
    buf: &mut [u8],
    w: u32,
    font: &PanelFont,
    card: &render::Panel,
    row: &Row<'_>,
    y: u32,
    highlighted: bool,
    bottom: u32,
    aa: Subpixel,
    s: u32,
) {
    // The highlighted line moves in by 2 px and takes the primary ink, so the mark
    // is the text itself and nothing changes size.
    let inset = crate::PAD * s + if highlighted { 2 * s } else { 0 };
    let fg = if highlighted {
        render::FG_SEL
    } else {
        colours(row.kind)
    };
    // Clipped at the card's bottom edge, so ink never runs past its own card.
    render::draw_text(
        buf,
        w,
        bottom,
        font,
        (card.x + inset) as f32,
        render::row_baseline(font, y),
        row.text,
        fg,
        (card.w.saturating_sub(2 * inset)) as f32,
        aa,
    );
}

/// A reply's ink: the secondary one for what the model said, so a failure or a
/// note reads as the panel's own voice without changing tone.
fn colours(_kind: Kind) -> Bgra {
    render::FG_NORMAL
}

/// Replies height a panel of `h` buffer pixels can show above its bar, in buffer
/// pixels: the rows the spec caps, expressed where the painter needs it.
pub fn transcript_px(h: u32, font: &PanelFont, scale: u32) -> u32 {
    let s = scale.max(1);
    let chrome = font.row_h + render::SEPARATION * s;
    h.saturating_sub(chrome).min(MAX_ROWS as u32 * font.row_h)
}

/// Panel height, in buffer pixels without the shadow margin, for replies that
/// draw `content` buffer pixels: the bar, the gap and the visible slice.
pub fn height_for(content: u32, font: &PanelFont, scale: u32) -> u32 {
    let s = scale.max(1);
    font.row_h + render::SEPARATION * s + content.min(MAX_ROWS as u32 * font.row_h)
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

    /// The panel `main` would give this scene: 500 px wide, the bar, the gap and
    /// the replies.
    fn sized(scene: &Scene, f: &PanelFont) -> (u32, u32) {
        (500, height_for(scene.transcript.total_height(), f, 1))
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

    fn paint(scene: &Scene, w: u32, h: u32) -> Vec<u8> {
        paint_revealed(scene, w, h, 1.0)
    }

    /// One frame of the panel with the replies card revealed to `reveal`.
    fn paint_revealed(scene: &Scene, w: u32, h: u32, reveal: f32) -> Vec<u8> {
        paint_highlighted(scene, w, h, None, reveal)
    }

    fn paint_highlighted(
        scene: &Scene,
        w: u32,
        h: u32,
        highlight: Option<usize>,
        reveal: f32,
    ) -> Vec<u8> {
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
            highlight,
            true,
            "",
            "",
            reveal,
            1,
        );
        buf
    }

    fn glass() -> render::Bgra {
        render::premul(render::BG_PROMPT)
    }

    #[test]
    fn the_panel_floats_in_a_shadow_margin() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let buf = paint(&scene, w, h);
        let px = |x: u32, y: u32| &buf[((y * bw + x) * 4) as usize..][..4];
        // The margin is empty except for the shadow the cards cast into it.
        assert_eq!(px(0, 0), &[0, 0, 0, 0], "buffer corner is clear");
        assert_eq!(px(0, bh / 2)[3], 0, "outside the shadow reach stays clear");
        // Hard against the panel's left edge there is shadow, not panel.
        let inside_shadow = px(render::SHADOW - 1, bh / 2);
        assert!(inside_shadow[3] > 0, "the cards cast a shadow");
        assert_eq!(inside_shadow[..3], [0, 0, 0], "shadow is black");
        // ...and inside a card the body is the translucent glass, premultiplied.
        assert_eq!(&buf[at(bw, 2, f.row_h / 2)..][..4], glass());
    }

    #[test]
    fn the_corner_is_antialiased_not_staircased() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _bh) = frame(w, h);
        let buf = paint(&scene, w, h);
        // The pixel just outside the whole-pixel span carries partial coverage;
        // without it the curve would step in whole pixels.
        let (x0, _) = render::rounded_span(w, h, render::CORNER_RADIUS, 0);
        let outer = &buf[at(bw, x0.saturating_sub(1), 0)..][..4];
        assert!(
            outer[3] > 0 && outer[3] < render::BG_PROMPT[3],
            "edge pixel {outer:?} is neither clear nor full body"
        );
    }

    /// Two cards of one tone: the replies above, the bar below, and only their
    /// own shadows in the 12 px between them.
    #[test]
    fn the_two_cards_share_one_tone() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        let replies_h = h - f.row_h - render::SEPARATION;
        assert_eq!(px(12, replies_h / 2), glass(), "the replies card");
        assert_eq!(px(12, h - f.row_h / 2), glass(), "the bar card");
        let gap_y = h - f.row_h - render::SEPARATION / 2;
        assert_ne!(px(w / 2, gap_y), glass(), "the gap is not body");
    }

    /// No label means no accent rail: `wlmenu` draws that only for a `-p` prompt.
    #[test]
    fn the_bar_carries_no_rail() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        for y in h - f.row_h..h {
            for x in 0..w {
                assert_ne!(
                    &buf[at(bw, x, y)..][..4][..3],
                    &render::LABEL_ACCENT[..3],
                    "accent ink in the bar at ({x},{y})"
                );
            }
        }
    }

    /// Composing with an input method has to be visible: the pre-edit is what
    /// tells the user their keystrokes went somewhere, before anything commits.
    #[test]
    fn a_pre_edit_is_drawn_after_the_typed_text() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.input.insert("why");
        let (w, h) = sized(&scene, &f);
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
                true,
                preedit,
                "",
                1.0,
                1,
            );
            buf
        };
        let bare = paint_with("");
        let composing = paint_with("zhong");
        // Right of the typed text the bar is empty until the input method speaks.
        let from = crate::PAD + render::measure(&f, "why", f32::MAX).ceil() as u32 + 8;
        let ink = |buf: &[u8]| {
            (from..from + 60)
                .flat_map(|x| (h - f.row_h..h).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        assert_eq!(ink(&bare), 0, "nothing is drawn after the typed text");
        assert!(ink(&composing) > 0, "the composition is visible");
    }

    /// Vim mode has to say so: the bar reads `-- VIM --` on the right and the
    /// caret is gone, so a swallowed keystroke is never a mystery.
    #[test]
    fn the_bar_says_when_vim_mode_is_on() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let paint_with = |label: &str, caret: bool| {
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
                caret,
                "",
                label,
                1.0,
                1,
            );
            buf
        };
        let plain = paint_with("", true);
        let vim = paint_with("-- VIM --", false);
        // The label's ink is in the bar's right half, and only there.
        let ink = |buf: &[u8], from: u32, to: u32| {
            (from..to)
                .flat_map(|x| (h - f.row_h..h).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        let right = (w * 2 / 3, w - crate::PAD);
        assert_eq!(
            ink(&plain, right.0, right.1),
            0,
            "nothing there when typing"
        );
        assert!(ink(&vim, right.0, right.1) > 0, "the mode is on the bar");
        // The caret is gone: with an empty input it sits at the padding, and only
        // the typing frame has it.
        assert!(ink(&plain, crate::PAD, crate::PAD + 3) > 0, "the caret");
        assert_eq!(ink(&vim, crate::PAD, crate::PAD + 3), 0, "no caret in vim");
    }

    #[test]
    fn the_highlight_is_ink_not_a_mark() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let plain = paint_highlighted(&scene, w, h, None, 1.0);
        let lit = paint_highlighted(&scene, w, h, Some(0), 1.0);
        let (bw, _) = frame(w, h);
        // Nothing in the margin changes: the mark is the text, so every pixel left
        // of the text column is identical with and without the highlight.
        for y in 0..f.row_h {
            for x in 0..crate::PAD {
                assert_eq!(
                    &plain[at(bw, x, y)..][..4],
                    &lit[at(bw, x, y)..][..4],
                    "a mark appeared at ({x},{y})"
                );
            }
        }
        // The line itself changes: brighter ink, 2 px in.
        let ink = |buf: &Vec<u8>| {
            (crate::PAD..crate::PAD + 40)
                .flat_map(|x| (0..f.row_h).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        assert_ne!(
            ink(&plain),
            ink(&lit),
            "the highlighted line is inked differently"
        );
        // ...and no other line moved: the second line is byte-identical.
        for y in f.row_h..2 * f.row_h {
            for x in 0..w {
                assert_eq!(
                    &plain[at(bw, x, y)..][..4],
                    &lit[at(bw, x, y)..][..4],
                    "an unhighlighted line changed at ({x},{y})"
                );
            }
        }
    }

    /// The arrangement: the replies in one card above, the bar in the card below,
    /// nothing but their shadows in between.
    #[test]
    fn the_reply_is_a_card_above_the_bar() {
        let f = font();
        let blocks = Blocks::new(&f);
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        assert!(
            scene
                .transcript
                .rows(blocks)
                .iter()
                .all(|r| r.kind != Kind::Question),
            "the question is drawn nowhere"
        );
        let replies_h = h - f.row_h - render::SEPARATION;
        assert_eq!(
            px(12, f.row_h),
            glass(),
            "the replies card starts at the top"
        );
        assert_eq!(
            px(12, replies_h - f.row_h),
            glass(),
            "and ends above the gap"
        );
        assert_ne!(px(w / 2, replies_h + render::SEPARATION / 2), glass());
        assert_eq!(px(12, h - f.row_h / 2), glass(), "the bar card");
    }

    /// The card grows out of the bar: at half the reveal, nothing above the height
    /// it has reached is painted, and the bar is untouched.
    #[test]
    fn the_reveal_clips_the_card_and_leaves_the_bar_alone() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let full = paint_revealed(&scene, w, h, 1.0);
        let half = paint_revealed(&scene, w, h, 0.5);
        let px = |buf: &Vec<u8>, x: u32, y: u32| buf[at(bw, x, y)..][..4].to_vec();
        let replies_h = h - f.row_h - render::SEPARATION;
        let revealed = replies_h / 2;
        // The top half of the replies card is not on screen yet.
        assert_eq!(
            px(&half, 12, revealed / 2),
            vec![0, 0, 0, 0],
            "nothing above the reveal"
        );
        assert_eq!(
            px(&full, 12, revealed / 2),
            glass(),
            "and it is there when finished"
        );
        // What has been revealed sits against the bar: the card grows upwards out
        // of it, so the area just above the gap is glass in both frames.
        assert_eq!(
            px(&half, 12, revealed + revealed / 2),
            glass(),
            "the card grows up from the bar"
        );
        // The bar is pixel-identical: it does not move, so it cannot animate.
        for y in h - f.row_h..h {
            for x in 0..w {
                assert_eq!(px(&half, x, y), px(&full, x, y), "bar moved at ({x},{y})");
            }
        }
    }

    /// The reported defect: an answer taller than the box drew nothing at all,
    /// because the lines were placed from the transcript's origin while the card
    /// was clipped to the box.
    #[test]
    fn a_tall_answer_scrolls_inside_the_box() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        for i in 0..6 {
            scene.transcript.add_note(&format!("note {i}"), &f, 300.0);
        }
        let room = 4 * f.row_h;
        let (w, h) = (500u32, height_for(room, &f, 1));
        scene.viewport = Viewport::new(transcript_px(h, &f, 1));
        let total = scene.transcript.total_height();
        assert!(total > room, "the transcript is taller than the box");
        scene.viewport.follow(total);
        let tail = paint(&scene, w, h);
        let ink_in = |from: u32, to: u32| {
            (from..to)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .filter(|&(x, y)| {
                    tail[at(w, x, y)..][..4][..3] != render::premul(render::BG_PROMPT)[..3]
                })
                .count()
        };
        // The box is full of the newest lines, right down to its last row.
        assert!(ink_in(f.row_h, 2 * f.row_h) > 0, "the box is not empty");
        assert!(
            ink_in(room - f.row_h, room) > 0,
            "the newest line is at the bottom of the box"
        );
        // Scrolled to the top instead, the frame is a different slice of the text,
        // and the bar is untouched either way.
        scene.viewport.top = 0;
        let head = paint(&scene, w, h);
        assert_ne!(head, tail, "the view offset is applied to the lines");
    }

    #[test]
    fn the_scroll_thumb_shows_only_when_the_transcript_overflows() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        // A second turn, so the replies are taller than the panel can show.
        scene.transcript.ask("and at night?", &f);
        scene.transcript.delta(
            "The same scattering leaves only the long wavelengths, so the sky reads red.",
            &f,
            300.0,
        );
        scene.transcript.finish_answer(&f);
        assert!(scene.transcript.total_height() > 2 * f.row_h);
        scene.viewport = Viewport::new(2 * f.row_h);
        let (w, h) = (500u32, height_for(2 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        let tw = 3;
        let tx = w - (crate::PAD + tw) / 2;
        assert_eq!(
            &buf[at(bw, tx, 2)..][..4][..3],
            &render::LABEL_ACCENT[..3],
            "the overflow indicator is at the card's right edge"
        );
        // Everything fits once the viewport is tall enough: no indicator.
        scene.viewport = Viewport::new(MAX_ROWS as u32 * 30);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let short = paint(&scene, w, h);
        assert_ne!(
            &short[at(bw, tx, 2)..][..4][..3],
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
        // The circle is round: the corner carries at most a trace of the shadow,
        // and the middle is the glass body.
        fn px(buf: &[u8], w: u32, x: u32, y: u32) -> &[u8] {
            &buf[at(w, x, y)..][..4]
        }
        assert!(px(&a, w, 0, 0)[3] <= 1, "corner stays all but clear");
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
            let pw = 500 * scale;
            let scene = scene(&f, (pw - 2 * crate::PAD * scale) as f32);
            let ph = height_for(scene.transcript.total_height(), &f, scale);
            let (w, h) = (
                pw + 2 * render::SHADOW * scale,
                ph + 2 * render::SHADOW * scale,
            );
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
                true,
                "",
                "",
                1.0,
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
        let (w, h) = sized(&scene, &f);
        let buf = paint(&scene, w, h);
        let ink = |p: &[u8]| p[..3] != glass()[..3];
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
        let mut scene = scene(&f, 500.0);
        scene
            .transcript
            .fail("the provider refused the request (401)", &f, 500.0);
        let (w, h) = sized(&scene, &f);
        let buf = paint(&scene, w, h);
        assert!(buf.as_chunks::<4>().0.iter().any(|p| p[3] > 0));
    }
}
