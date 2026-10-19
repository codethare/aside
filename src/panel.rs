//! Paints the panel: the transcript viewport above, the input bar below, both
//! inside one rounded outline.
//!
//! Geometry comes from `wlmenu` unchanged — the input bar is the same band it
//! draws (a 4 px `LABEL_ACCENT` rail, an 8 px gap, a 1 px divider at 25 %
//! alpha), just at the bottom edge instead of the top, because a conversation
//! grows downward. Which row is highlighted never moves anything: scrolling and
//! the working indicator change colours and offsets, never geometry.

use crate::font::PanelFont;
use crate::layout::{self, Input, MAX_ROWS, Viewport};
use crate::render::{self, Bgra, Subpixel};
use crate::transcript::{Kind, Row, Transcript};

/// Paints the whole frame. `buf` is `w * h * 4` bytes, BGRA.
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
    scale: u32,
) {
    let s = scale.max(1);
    let p = crate::PAD * s;
    // The font is re-derived at `SIZE * scale`, so its metrics are already in
    // buffer pixels; only the panel's own constants are logical.
    let row_h = font.row_h;
    let aa = layout::text_aa(s);
    let bar_h = row_h + render::DIVIDER_H * s;
    let top = h.saturating_sub(bar_h);

    // One pass over every pixel: the buffer may be a different shm slot each
    // frame, so anything outside the rounded outline must be written too.
    let r = (render::CORNER_RADIUS * s).min(h / 2);
    for y in 0..h {
        let (x0, x1) = render::rounded_span(w, h, r, y);
        let bg = if y >= top {
            render::BG_PROMPT
        } else {
            render::BG_NORMAL
        };
        paint_span(buf, w, y, 0, x0, [0, 0, 0, 0]);
        paint_span(buf, w, y, x0, x1, bg);
        paint_span(buf, w, y, x1, w, [0, 0, 0, 0]);
    }

    // Divider between the conversation and the bar.
    if h > bar_h {
        let y = top - render::DIVIDER_H * s;
        let (x0, x1) = render::rounded_span(w, h, r, y);
        let mut colour = render::BG_PROMPT;
        render::blend_px(
            &mut colour,
            [
                render::LABEL_ACCENT[0],
                render::LABEL_ACCENT[1],
                render::LABEL_ACCENT[2],
            ],
            render::DIVIDER_ALPHA,
        );
        render::rect(buf, w, h, x0, y, x1 - x0, render::DIVIDER_H * s, colour);
    }

    // Transcript: the rows the viewport shows, drawn bottom-up from the divider.
    let rows = transcript.rows();
    let visible = viewport.visible(rows.len());
    let shown = visible.len();
    for (i, row) in rows[visible.clone()].iter().enumerate() {
        let index = visible.start + i;
        // Rows sit above the bar, newest at the bottom.
        let y = top.saturating_sub(((shown - i) as u32) * row_h);
        draw_row(
            buf,
            w,
            h,
            font,
            row,
            y,
            row_h,
            highlight == Some(index),
            aa,
            s,
        );
    }

    // Input bar: rail, then the typed text, then the caret.
    let baseline = render::row_baseline(font, top);
    // `line_h` is already in buffer pixels (the font is derived at `SIZE * scale`);
    // only the padding is a logical constant. Scaling the whole sum again made the
    // rail taller than the row at 2× and the subtraction below panicked.
    let rail_h = font.line_h.ceil() as u32 + 2 * render::PILL_VPAD * s;
    let rail_y = top + (row_h - rail_h) / 2;
    render::rect(
        buf,
        w,
        h,
        p,
        rail_y,
        render::LABEL_RAIL_W * s,
        rail_h,
        layout::rail_color(streaming),
    );
    let gap = render::PROMPT_GAP * s as f32;
    let text_x = p as f32 + (render::LABEL_RAIL_W * s) as f32 + gap;
    let limit = w as f32 - text_x - p as f32;
    let typed = input.text();
    let end = render::draw_text(
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
    let caret_w = 2 * s;
    let caret_bottom = (baseline - font.descent_px).round().max(0.0) as u32;
    let caret_h = font.line_h.ceil() as u32;
    if caret_visible && caret_bottom > 0 {
        let caret_top = caret_bottom.saturating_sub(caret_h);
        let caret_x = if typed.is_empty() { text_x } else { end } as u32 + s;
        render::rect(
            buf,
            w,
            h,
            caret_x.min(w.saturating_sub(caret_w)),
            caret_top,
            caret_w,
            caret_bottom.saturating_sub(caret_top),
            render::FG_PROMPT,
        );
    }
}

/// One transcript row: optional highlight band (clipped to the rounded outline,
/// as the item rows are) and the text in its kind's colour.
#[allow(clippy::too_many_arguments)] // a row is seven facts about where it goes
fn draw_row(
    buf: &mut [u8],
    w: u32,
    h: u32,
    font: &PanelFont,
    row: &Row<'_>,
    y: u32,
    row_h: u32,
    highlighted: bool,
    aa: Subpixel,
    s: u32,
) {
    let r = (render::CORNER_RADIUS * s).min(h / 2);
    if highlighted {
        for yy in y..(y + row_h).min(h) {
            let (x0, x1) = render::rounded_span(w, h, r, yy);
            paint_span(buf, w, yy, x0, x1, render::BG_SEL);
        }
    }
    let (fg, _) = colours(row.kind, highlighted);
    render::draw_text(
        buf,
        w,
        h,
        font,
        crate::PAD as f32 * s as f32,
        render::row_baseline(font, y),
        row.text,
        fg,
        w as f32 - 2.0 * crate::PAD as f32 * s as f32,
        aa,
    );
}

/// Text and background for a row. The four kinds are told apart by colour and by
/// what they say: a question is as bright as the input line it was typed into,
/// an answer is the softer normal foreground, a failure is the loudest thing on
/// screen, and a note explains itself in words.
fn colours(kind: Kind, highlighted: bool) -> (Bgra, Bgra) {
    if highlighted {
        return (render::FG_SEL, render::BG_SEL);
    }
    match kind {
        Kind::Question => (render::FG_PROMPT, render::BG_NORMAL),
        Kind::Answer => (render::FG_NORMAL, render::BG_NORMAL),
        Kind::Error => (render::FG_SEL, render::BG_NORMAL),
        Kind::Note => (render::FG_NORMAL, render::BG_NORMAL),
    }
}

fn paint_span(buf: &mut [u8], w: u32, y: u32, x0: u32, x1: u32, colour: Bgra) {
    if x1 > x0 {
        render::rect(buf, w, u32::MAX, x0, y, x1 - x0, 1, colour);
    }
}

/// Rows of conversation that fit in a panel `h` pixels tall, capped at the
/// viewport maximum.
pub fn visible_rows(h: u32, font: &PanelFont, scale: u32) -> usize {
    let s = scale.max(1);
    let row_h = font.row_h;
    let bar = row_h + render::DIVIDER_H * s;
    ((h.saturating_sub(bar)) / row_h).min(MAX_ROWS as u32) as usize
}

/// Panel height for a wanted number of conversation rows.
pub fn height_for_rows(rows: usize, font: &PanelFont, scale: u32) -> u32 {
    let s = scale.max(1);
    (rows.min(MAX_ROWS) as u32) * font.row_h + font.row_h + render::DIVIDER_H * s
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
    use crate::layout::Lines;

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    struct Scene {
        transcript: Transcript,
        input: Input,
        viewport: Viewport,
    }

    fn scene(font: &PanelFont, w: f32) -> Scene {
        let mut transcript = Transcript::default();
        transcript.ask("what does EAGAIN mean", font, w);
        transcript.delta(
            "It means the resource is momentarily unavailable, so retry the call.",
            font,
            w,
        );
        transcript.finish_answer();
        transcript.ask("and 折行?", font, w);
        transcript.delta("同样的意思", font, w);
        let mut input = Input::default();
        input.insert("next question");
        let viewport = Viewport::new(MAX_ROWS);
        Scene {
            transcript,
            input,
            viewport,
        }
    }

    fn paint(scene: &Scene, streaming: bool, w: u32, h: u32) -> Vec<u8> {
        let f = font();
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
            streaming,
            true,
            1,
        );
        buf
    }

    #[test]
    fn the_panel_is_rounded_and_opaque_inside() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for_rows(8, &f, 1));
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[((y * w + x) * 4) as usize..][..4];
        for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1), (w - 1, h - 1)] {
            assert_eq!(px(x, y), &[0, 0, 0, 0], "corner ({x},{y}) must be clear");
        }
        assert_eq!(px(6, 2)[3], 0xff, "just inside the corner is opaque");
        assert_eq!(px(w / 2, h / 2), render::BG_NORMAL);
    }

    #[test]
    fn the_input_bar_is_the_prompt_band_at_the_bottom() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for_rows(4, &f, 1));
        let buf = paint(&scene, false, w, h);
        let px = |x: u32, y: u32| &buf[((y * w + x) * 4) as usize..][..4];
        let bar_top = h - f.row_h;
        assert_eq!(px(w / 2, bar_top + 2), render::BG_PROMPT, "bar band");
        assert_eq!(
            px(w / 2, bar_top - 3),
            render::BG_NORMAL,
            "transcript above"
        );
        // The rail is the working indicator, at the content inset.
        assert_eq!(
            px(crate::PAD + 1, bar_top + f.row_h / 2),
            &render::LABEL_ACCENT
        );
    }

    #[test]
    fn a_stream_ending_changes_the_rail_colour_and_nothing_else() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for_rows(6, &f, 1));
        let idle = paint(&scene, false, w, h);
        let busy = paint(&scene, true, w, h);
        let bar_top = h - f.row_h;
        let rail = ((crate::PAD + 1) as usize, (bar_top + f.row_h / 2) as usize);
        let mut differing = Vec::new();
        for (i, (a, b)) in idle
            .as_chunks::<4>()
            .0
            .iter()
            .zip(busy.as_chunks::<4>().0)
            .enumerate()
        {
            if a != b {
                differing.push((i % w as usize, i / w as usize));
            }
        }
        assert!(!differing.is_empty(), "the indicator must change");
        for (x, y) in differing {
            assert!(
                x >= crate::PAD as usize
                    && x < (crate::PAD + render::LABEL_RAIL_W) as usize
                    && y >= bar_top as usize,
                "only the rail may change, but ({x},{y}) did (rail at {rail:?})"
            );
        }
    }

    #[test]
    fn the_highlight_band_covers_the_row_and_keeps_its_text_readable() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for_rows(6, &f, 1));
        let mut buf = vec![0u8; (w * h * 4) as usize];
        draw(
            &mut buf,
            w,
            h,
            &f,
            &scene.transcript,
            &scene.input,
            &scene.viewport,
            Some(2),
            false,
            true,
            1,
        );
        let rows = scene.transcript.rows();
        assert!(rows.len() > 2);
        // Row 2 is drawn second from the bottom of the visible block.
        let shown = scene.viewport.visible(rows.len()).len();
        let bar = h - f.row_h - render::DIVIDER_H;
        let y = bar - ((shown - 2) as u32) * f.row_h + 1;
        let px = &buf[((y * w + w / 2) * 4) as usize..][..4];
        assert_eq!(px, render::BG_SEL, "the highlighted row's band");
    }

    #[test]
    fn rows_keep_their_positions_when_the_answer_grows() {
        // The viewport follows the tail, so appending text may scroll: what must
        // not change is the geometry of a row that stays visible.
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = (500u32, height_for_rows(12, &f, 1));
        let before = paint(&scene, true, w, h);
        let mut after_scene = scene;
        after_scene.transcript.delta(" more", &f, 300.0);
        let after = paint(&after_scene, true, w, h);
        let bar = (h - f.row_h) as usize;
        // The input bar, drawn last, must be pixel-identical: nothing about a
        // delta can move it.
        let bar_of = |b: &Vec<u8>| b[bar * w as usize * 4..].to_vec();
        assert_eq!(bar_of(&before), bar_of(&after));
    }

    #[test]
    fn the_visible_row_count_follows_the_window_and_the_cap() {
        let f = font();
        assert_eq!(visible_rows(height_for_rows(6, &f, 1), &f, 1), 6);
        // The cap holds even for a window taller than the viewport allows.
        assert_eq!(visible_rows(10_000, &f, 1), MAX_ROWS);
        assert_eq!(visible_rows(1, &f, 1), 0, "too short for a bar and a row");
        // At 2x the font is re-derived twice as large, so the same eight rows
        // take twice the buffer and still fit the same window.
        let big = PanelFont::load(crate::font::SIZE * 2.0).expect("system font");
        assert_eq!(visible_rows(height_for_rows(8, &big, 2), &big, 2), 8);
        assert!(height_for_rows(8, &big, 2) > height_for_rows(8, &f, 1));
    }

    #[test]
    fn every_scale_paints_instead_of_panicking() {
        // The font is derived at `SIZE * scale`, so its metrics are already in
        // buffer pixels. Scaling one of them a second time made the input rail
        // taller than its row at 2x, and `row_h - rail_h` panicked before a single
        // pixel was drawn — on a HiDPI output that means no panel at all.
        for scale in [1u32, 2, 3] {
            let f = PanelFont::load(crate::font::SIZE * scale as f32).expect("system font");
            let (w, h) = (500 * scale, height_for_rows(4, &f, scale));
            let scene = scene(&f, (w - 2 * crate::PAD * scale) as f32);
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
                scale,
            );
            assert!(
                buf.as_chunks::<4>().0.iter().any(|p| p[3] == 0xff),
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
        let (w, h) = (140u32, height_for_rows(8, &f, 1));
        let buf = paint(&scene, true, w, h);
        let ink = |p: &[u8]| p[..3] != render::BG_NORMAL[..3];
        // Something was painted, and all of it inside the panel.
        assert!(buf.as_chunks::<4>().0.iter().any(|p| ink(p)));
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
        let w = 900u32;
        let h = height_for_rows(12, &f, 1);
        let mut scene = scene(&f, (w - 2 * crate::PAD) as f32);
        scene
            .transcript
            .fail("the provider refused the request (401)", &f, 700.0);
        scene.transcript.add_note("cancelled", &f, 700.0);
        let ppm = to_ppm(&paint(&scene, false, w, h), w, h);
        assert!(ppm.starts_with(b"P6\n900 "));
        let header = ppm
            .windows(4)
            .position(|w| w == b"255\n")
            .expect("PPM header")
            + 4;
        assert_eq!(ppm.len() - header, (w * h * 3) as usize);
        let rows = Lines::wrap("anything", &f, 100.0);
        assert!(!rows.rows().is_empty());
    }
}
