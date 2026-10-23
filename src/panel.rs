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

/// The panel's three shapes, as shapes rather than as the state machine's phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The input bar alone, which is also the shape a running request keeps.
    Bar,
    /// The replies above, the bar below.
    Replies,
}

/// Which shape is being painted, where it is coming from, and how far it has got:
/// `t = 1` and `rise = 1` is settled, anything less is a transition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Look {
    pub state: State,
    /// The shape being left; ignored once `t` is 1.
    pub from: State,
    /// Progress of the shape change, 0 at the shape being left.
    pub t: f32,
    /// How far the card has risen out of the bottom edge: 1 when in place.
    pub rise: f32,
    /// How far the card has spread across from its centre: 1 when full width.
    pub spread: f32,
}

impl Look {
    /// A settled frame: the state, painted by its own painter.
    pub fn settled(state: State) -> Look {
        Look {
            state,
            from: state,
            t: 1.0,
            rise: 1.0,
            spread: 1.0,
        }
    }

    /// A frame on its way: a shape change, a rise out of the bottom edge, a spread
    /// from a point, or any two of them at once.
    pub fn moving(state: State, from: State, t: f32, rise: f32, spread: f32) -> Look {
        Look {
            state,
            from,
            t: t.clamp(0.0, 1.0),
            rise: rise.clamp(0.0, 1.0),
            spread: spread.clamp(0.0, 1.0),
        }
    }

    /// Whether this frame is animated rather than a settled shape.
    fn animated(&self) -> bool {
        self.t < 1.0 || self.rise < 1.0 || self.spread < 1.0
    }
}

/// A card at `spread` of its width, centred where it will be: how the panel opens
/// as a line growing both ways out of a point. A card of no width is not painted.
fn spread_card(card: render::Panel, spread: f32, min_w: u32) -> render::Panel {
    let w = ((card.w as f32) * spread.clamp(0.0, 1.0)).round() as u32;
    let w = w.max(if spread > 0.0 { min_w } else { 0 });
    render::Panel {
        x: card.x + (card.w - w) / 2,
        w,
        ..card
    }
}

/// A card between `min_h` and its own height, anchored on its bottom edge: how the
/// panel unfolds upward from a line, and how it rolls back down into one.
fn risen(card: render::Panel, rise: f32, min_h: u32) -> render::Panel {
    let rise = rise.clamp(0.0, 1.0);
    let min_h = min_h.min(card.h).max(1);
    let h = (min_h as f32 + (card.h as f32 - min_h as f32) * rise).round() as u32;
    render::Panel {
        y: card.y + card.h - h.max(1),
        h: h.max(1),
        ..card
    }
}

/// One shape's content, drawn on `card` at `fade` of its alpha: what a morph
/// cross-fades between, and what a settled frame draws at 1.
#[allow(clippy::too_many_arguments)] // the content of a panel is everything it holds
fn draw_content(
    buf: &mut [u8],
    w: u32,
    h: u32,
    font: &PanelFont,
    transcript: &Transcript,
    input: &Input,
    viewport: &Viewport,
    // The rows the panel marks: a pointer's selection, or one line from the
    // keyboard. Nothing when there is nothing to mark.
    marked: Option<(usize, usize)>,
    caret_visible: bool,
    preedit: &str,
    hint: &str,
    label: &str,
    state: State,
    card: &render::Panel,
    head: Option<f32>,
    fade: f32,
    aa: Subpixel,
    s: u32,
) {
    if fade <= 0.0 {
        return;
    }
    match state {
        State::Bar => {
            draw_bar(
                buf,
                w,
                h,
                card,
                font,
                input,
                card.h,
                caret_visible,
                preedit,
                hint,
                label,
                fade,
                aa,
                s,
            );
            // The working light runs over the rim the body has just painted, so it
            // needs the card's whole rim intact: only the bar's own state draws it.
            if let Some(head) = head.filter(|_| fade > 0.99) {
                render::orbit(buf, w, h, card, head, s);
            }
        }
        State::Replies => draw_replies(
            buf,
            w,
            font,
            transcript,
            viewport,
            marked,
            card,
            card.y..(card.y + card.h),
            fade,
            aa,
            s,
        ),
    }
}

/// The card a state is painted as, inside `panel`: the bar's band on the bottom
/// row, the ball's square centred on it, or the whole panel for the two-card pane.
pub fn card_of(state: State, panel: &render::Panel, bar_h: u32) -> render::Panel {
    match state {
        State::Bar => render::Panel {
            y: panel.y + panel.h.saturating_sub(bar_h),
            h: bar_h.min(panel.h),
            ..*panel
        },
        State::Replies => *panel,
    }
}

/// A card at a fraction of the way from `a` to `b`: the shape change the morphs
/// are made of.
pub fn lerp_card(a: &render::Panel, b: &render::Panel, t: f32) -> render::Panel {
    let t = t.clamp(0.0, 1.0);
    let at = |a: u32, b: u32| (a as f32 + (b as f32 - a as f32) * t).round() as u32;
    render::Panel {
        x: at(a.x, b.x),
        y: at(a.y, b.y),
        w: at(a.w, b.w),
        h: at(a.h, b.h),
        r: at(a.r, b.r),
    }
    .clamped()
}

impl render::Panel {
    /// A lerped card can be a pixel wider than its own height allows; the outline
    /// maths clamps the radius anyway, so this only keeps the size sane.
    fn clamped(self) -> render::Panel {
        render::Panel {
            h: self.h.max(1),
            w: self.w.max(1),
            ..self
        }
    }
}

/// Paints the panel in one of its shapes. `buf` is `w * h * 4` bytes, BGRA, and
/// holds the panel plus its shadow margin on every side. `look` says which shape
/// and how far into it the panel has morphed; a settled look is painted exactly as
/// that shape always was.
#[allow(clippy::too_many_arguments)]
pub fn draw(
    buf: &mut [u8],
    w: u32,
    h: u32,
    font: &PanelFont,
    transcript: &Transcript,
    input: &Input,
    viewport: &Viewport,
    // The rows the panel marks: a pointer's selection, or one line from the
    // keyboard. Nothing when there is nothing to mark.
    marked: Option<(usize, usize)>,
    caret_visible: bool,
    preedit: &str,
    // What the bar says instead of the input line: the prefix mode's instruction.
    hint: &str,
    // The bar's right-hand label: `-- VIM --` while vim mode owns the keyboard.
    label: &str,
    // Which shape this frame is, and how far it has morphed into it.
    look: Look,
    // Where the working light is on the bar's rim, 0..1 round the perimeter; the
    // light is only painted while a request is open.
    head: Option<f32>,
    scale: u32,
) {
    let s = scale.max(1);
    let aa = layout::text_aa(s);
    let bar_h = font.row_h;
    let Some(panel) = panel_of(w, h, render::CORNER_RADIUS, s) else {
        buf.fill(0);
        return;
    };
    if look.animated() {
        let from = card_of(look.from, &panel, bar_h);
        let to = card_of(look.state, &panel, bar_h);
        // A thin line at nothing, the full card at one; a point of no width at
        // nothing, the full width at one. Either can be part-way on its own.
        let card = spread_card(
            risen(lerp_card(&from, &to, look.t), look.rise, 2 * s),
            look.spread,
            2 * s,
        );
        if card.w == 0 || card.h == 0 {
            return;
        }
        // Nothing is drawn on a line or on a point: the content follows both tracks.
        let window = look.rise * look.spread;
        render::shadow(buf, w, h, &[card], s);
        // One card, two contents, faded in two halves: the shape being left goes
        // first, and the shape arriving follows once the card is wide enough.
        draw_content(
            buf,
            w,
            h,
            font,
            transcript,
            input,
            viewport,
            marked,
            caret_visible,
            preedit,
            hint,
            label,
            look.from,
            &card,
            head,
            // What is leaving is all but gone when what arrives begins, so the
            // card is never seen empty; and nothing is drawn on a line or a point.
            (1.0 - look.t * 1.8).clamp(0.0, 1.0) * window,
            aa,
            s,
        );
        render::body(buf, w, &card, render::BG_PROMPT);
        draw_content(
            buf,
            w,
            h,
            font,
            transcript,
            input,
            viewport,
            marked,
            caret_visible,
            preedit,
            hint,
            label,
            look.state,
            &card,
            head,
            // What arrives does so once there is a card wide enough to hold it.
            ((look.t - 0.45) * 1.8).clamp(0.0, 1.0) * window,
            aa,
            s,
        );
        return;
    }

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
    // The card is the room itself: the panel grew into it on its way here, so it is
    // not grown a second time, and the transcript scrolls inside it.
    let replies = (room > 0).then(|| render::Panel {
        y: bar.y - sep - room,
        h: room,
        ..panel
    });
    let cards: Vec<render::Panel> = match replies {
        Some(card) => vec![card, bar],
        None => vec![bar],
    };
    render::shadow(buf, w, h, &cards, s);
    // The bar takes the mode tone while the prefix waits, so a key that will be
    // swallowed is visible before it is pressed.
    let bar_tone = if hint.is_empty() {
        render::BG_PROMPT
    } else {
        render::BG_MODE
    };
    if let Some(replies) = replies.as_ref() {
        render::body(buf, w, replies, render::BG_PROMPT);
        // Rows are painted against the card's own origin, so the text travels up
        // out of the bar with it instead of being wiped in place.
        let clip = replies.y..(replies.y + replies.h);
        draw_replies(
            buf, w, font, transcript, viewport, marked, replies, clip, 1.0, aa, s,
        );
    }
    render::body(buf, w, &bar, bar_tone);

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
        hint,
        label,
        1.0,
        aa,
        s,
    );
    // The working light, over the rim the bar has just painted: a request in flight
    // is shown by the field's own edge, not by taking the field away.
    if let Some(head) = head {
        render::orbit(buf, w, h, &bar, head, s);
    }
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
    marked: Option<(usize, usize)>,
    card: &render::Panel,
    clip: std::ops::Range<u32>,
    fade: f32,
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
            marked.is_some_and(|(from, to)| (from..=to).contains(&index)),
            clip.end,
            fade,
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
    hint: &str,
    label: &str,
    fade: f32,
    aa: Subpixel,
    s: u32,
) {
    let p = crate::PAD * s;
    // A morph fades the whole line; a settled frame draws it whole.
    let ink = render::dim(render::FG_PROMPT, fade);
    // The bar is the last row of the panel; the surface hangs from the bottom
    // edge, so that row keeps one place on screen while replies grow above it.
    let bar_y = panel.y + panel.h.saturating_sub(row_h);
    let baseline = render::row_baseline(font, bar_y);
    // No label rail: `wlmenu` draws one only when it is given a prompt label, and
    // this bar has none, so the text starts at the padding.
    let text_x = (panel.x + p) as f32;
    let limit = (panel.x + panel.w) as f32 - p as f32 - text_x;
    // While the prefix waits, the bar carries the instruction and no input: the
    // caret and the pre-edit belong to a line nobody is typing on.
    let composing = hint.is_empty();
    let typed = if composing { input.text() } else { hint };
    let preedit = if composing { preedit } else { "" };
    let mut end = render::draw_text(
        buf,
        w,
        h,
        font,
        text_x,
        baseline,
        typed,
        ink,
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
            ink,
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
            ink,
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
            render::dim(render::FG_NORMAL, fade),
            width,
            aa,
        );
    }
    let caret_w = 2 * s;
    let caret_bottom = (baseline - font.descent_px).round().max(0.0) as u32;
    let caret_h = font.line_h.ceil() as u32;
    if caret_visible && composing && caret_bottom > 0 {
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
            ink,
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
    fade: f32,
    aa: Subpixel,
    s: u32,
) {
    // The highlighted line moves in by 2 px and takes the primary ink, so the mark
    // is the text itself and nothing changes size.
    let inset = crate::PAD * s + if highlighted { 2 * s } else { 0 };
    let fg = render::dim(
        if highlighted {
            render::FG_SEL
        } else {
            colours(row.kind)
        },
        fade,
    );
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
    crate::font::row_h_for(crate::font::size())
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
        paint_highlighted(scene, w, h, None)
    }

    fn paint_highlighted(scene: &Scene, w: u32, h: u32, marked: Option<(usize, usize)>) -> Vec<u8> {
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
            marked,
            true,
            "",
            "",
            "",
            Look::settled(State::Replies),
            None,
            1,
        );
        buf
    }

    fn glass() -> render::Bgra {
        render::premul(render::BG_PROMPT)
    }

    /// Each state is one card of the same panel: the bar's band on the bottom row,
    /// the ball's square centred on it, and the whole panel for the pane.
    #[test]
    fn each_state_is_one_card() {
        let f = font();
        let h = 3 * f.row_h;
        let panel = render::Panel {
            x: 20,
            y: 26,
            w: 640,
            h,
            r: render::CORNER_RADIUS,
        };
        let bar = card_of(State::Bar, &panel, f.row_h);
        assert_eq!(
            (bar.x, bar.y, bar.w, bar.h),
            (20, 26 + h - f.row_h, 640, f.row_h)
        );
        let pane = card_of(State::Replies, &panel, f.row_h);
        assert_eq!((pane.y, pane.h), (26, h), "the whole panel");
        // A shape change is the lerp of the two, exact at both ends.
        assert_eq!(lerp_card(&bar, &pane, 0.0), bar);
        assert_eq!(lerp_card(&bar, &pane, 1.0), pane);
    }

    /// The opening's first half: a line spreading out of a point, which is the
    /// card with no width and no height yet.
    #[test]
    fn a_frame_can_be_a_line_between_a_point_and_the_bar() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        let (w, h) = (500u32, f.row_h);
        let (bw, bh) = frame(w, h);
        let paint_at = |spread: f32, rise: f32| {
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
                "",
                "",
                "",
                Look::moving(State::Bar, State::Bar, 1.0, rise, spread),
                None,
                1,
            );
            buf
        };
        let glass = glass();
        // A point: nothing at all is painted.
        assert!(
            paint_at(0.0, 0.0)
                .as_chunks::<4>()
                .0
                .iter()
                .all(|p| p[3] == 0),
            "a point is nothing"
        );
        // Half-spread: the middle of the panel has the line, its ends do not.
        let half = paint_at(0.5, 0.0);
        assert_ne!(&half[at(bw, w / 2, 0)..][..4][..3], &glass[..3], "centre");
        assert_eq!(
            &half[at(bw, 4, 0)..][..4],
            &[0, 0, 0, 0],
            "the line has not reached the edge"
        );
        // Full spread, still a line: it sits on the bottom row and is thin.
        let line = paint_at(1.0, 0.0);
        // The rim colours the outermost row, so the probe is the row inside it.
        assert_ne!(
            &line[at(bw, 4, h - 1)..][..4][..3],
            &[0, 0, 0][..],
            "the line reaches the edge"
        );
        assert_eq!(
            &line[at(bw, w / 2, h - 1)..][..4][..3],
            &[32, 32, 32][..],
            "and runs across the middle, in the rim's tone"
        );
        // A line is not the bar: above it there is only the shadow it casts.
        let above = &line[at(bw, w / 2, h - 8)..][..4];
        assert_ne!(above[..3], glass[..3], "not the body");
        assert_eq!(above[..3], [0, 0, 0], "shadow, and no more");
        // Both tracks done: the idle bar.
        assert_eq!(paint_at(1.0, 1.0), paint(&scene, w, h));
    }

    /// A settled frame is the shape's own painter, whatever shape it came from: the
    /// animation cannot leak into the frame that ends it.
    #[test]
    fn a_settled_look_ignores_where_it_came_from() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let paint_look = |look: Look| {
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
                "",
                "",
                "",
                look,
                None,
                1,
            );
            buf
        };
        let settled = paint_look(Look::settled(State::Replies));
        // The bar is the shape it can come from; the frame at rest is the same one
        // whatever the shape it left.
        assert_eq!(
            paint_look(Look {
                from: State::Bar,
                ..Look::settled(State::Replies)
            }),
            settled,
            "a frame at rest is the same frame"
        );
    }

    /// The entrance is the card growing up out of the bottom edge: at nothing it is
    /// not there, and at the end it is the idle bar, pixel for pixel.
    #[test]
    fn the_panel_rises_out_of_the_bottom_edge() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        let (w, h) = (500u32, f.row_h);
        let (bw, _) = frame(w, h);
        let (_, _) = (bw, h);
        let paint_rise = |rise: f32| {
            let (bw2, bh2) = frame(w, h);
            let mut buf = vec![0u8; (bw2 * bh2 * 4) as usize];
            draw(
                &mut buf,
                bw2,
                bh2,
                &f,
                &scene.transcript,
                &scene.input,
                &scene.viewport,
                None,
                true,
                "",
                "",
                "",
                Look::moving(State::Bar, State::Bar, 1.0, rise, 1.0),
                None,
                1,
            );
            buf
        };
        let glass = glass();
        let down = paint_rise(0.0);
        let up = paint_rise(1.0);
        let settled = paint(&scene, w, h);
        // At nothing the bar's band is not painted: a row inside the panel is not
        // the glass, and at the end it is. (Row 4, because the first rows are the
        // card's own rim.)
        assert_ne!(&down[at(bw, w / 2, 4)..][..4], glass, "not there yet");
        assert_eq!(&up[at(bw, w / 2, 4)..][..4], glass, "there at the end");
        assert_eq!(up, settled, "the last frame is the idle bar");
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
                "",
                Look::settled(State::Replies),
                None,
                1,
            );
            buf
        };
        let bare = paint_with("");
        let composing = paint_with("zhong");
        // Right of the typed text the bar is empty until the input method speaks.
        let from = crate::PAD + render::measure(&f, "why", f32::MAX).ceil() as u32 + 8;
        // Inside the rim's bands: the bar's own text rows.
        let rows = (h - f.row_h + 2)..(h - 2);
        let ink = |buf: &[u8]| {
            (from..from + 60)
                .flat_map(|x| rows.clone().map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        assert_eq!(ink(&bare), 0, "nothing is drawn after the typed text");
        assert!(ink(&composing) > 0, "the composition is visible");
    }

    /// The prefix mode is visible before its key is pressed: the bar's body takes
    /// the mode tone and the line carries the instruction, with no caret and none
    /// of the input.
    #[test]
    fn the_prefix_mode_turns_the_bar_and_shows_the_hint() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.input.insert("half typed");
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let paint_with = |hint: &str, caret: bool| {
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
                hint,
                "",
                Look::settled(State::Replies),
                None,
                1,
            );
            buf
        };
        let typing = paint_with("", true);
        let prefix = paint_with("press [ to enter vim mode", false);
        let px = |buf: &Vec<u8>, x: u32, y: u32| buf[at(bw, x, y)..][..4].to_vec();
        // The bar's body is the mode tone, not the glass.
        let mid = h - f.row_h / 2;
        assert_eq!(px(&typing, 12, mid), render::premul(render::BG_PROMPT));
        assert_eq!(px(&prefix, 12, mid), render::premul(render::BG_MODE));
        // The hint is inked where the input used to be, and the typed text is not.
        let ink_row = |buf: &Vec<u8>| {
            (crate::PAD..w - crate::PAD)
                .flat_map(|x| (h - f.row_h..h).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    buf[at(bw, x, y)..][..4][..3] != render::premul(render::BG_MODE)[..3]
                })
                .count()
        };
        assert!(ink_row(&prefix) > 0, "the hint is drawn");
        // And with the prefix over, the bar is exactly what it was.
        let back = paint_with("", true);
        assert_eq!(back, typing, "the bar comes back byte for byte");
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
                "",
                label,
                Look::settled(State::Replies),
                None,
                1,
            );
            buf
        };
        let plain = paint_with("", true);
        let vim = paint_with("-- VIM --", false);
        // The label's ink is in the bar's right half, and only there.
        // Inside the rim's bands: the bar's own text rows.
        let rows = (h - f.row_h + 2)..(h - 2);
        let ink = |buf: &[u8], from: u32, to: u32| {
            (from..to)
                .flat_map(|x| rows.clone().map(move |y| (x, y)))
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
        let plain = paint_highlighted(&scene, w, h, None);
        let lit = paint_highlighted(&scene, w, h, Some((0, 0)));
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

    /// The reported defect: the panel opened, and then the card inside it grew a
    /// second time. The growth is the morph and nothing else, so once the panel is
    /// the panel the card fills it in that same frame.
    #[test]
    fn the_card_grows_once_while_the_panel_does() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let px = |buf: &Vec<u8>, x: u32, y: u32| buf[at(bw, x, y)..][..4].to_vec();
        let paint_look = |look: Look| {
            let (bw2, bh2) = frame(w, h);
            let mut buf = vec![0u8; (bw2 * bh2 * 4) as usize];
            draw(
                &mut buf,
                bw2,
                bh2,
                &f,
                &scene.transcript,
                &scene.input,
                &scene.viewport,
                None,
                true,
                "",
                "",
                "",
                look,
                None,
                1,
            );
            buf
        };
        // Half way through the panel's growth: the card's top is still below the
        // panel's own top, so nothing is painted there.
        let growing = paint_look(Look::moving(State::Replies, State::Bar, 0.5, 1.0, 1.0));
        assert_ne!(
            px(&growing, 12, 2),
            glass(),
            "the card has not reached the top yet"
        );
        // Settled: the same frame that ends the morph fills the panel, with no
        // further animation to come.
        let settled = paint_look(Look::settled(State::Replies));
        assert_eq!(px(&settled, 12, 2), glass(), "the card fills the panel");
        // Its last rows belong to the rim, so the probe is a couple of rows above
        // the gap: the card, not its edge.
        assert_eq!(
            px(&settled, 30, h - f.row_h - render::SEPARATION - 4),
            glass(),
            "right down to the gap above the bar"
        );
        // The card is anchored on the bottom row in both frames: the bar does not
        // travel while the panel grows above it.
        for frame in [&growing, &settled] {
            assert_eq!(
                px(frame, 30, h - f.row_h / 2),
                glass(),
                "the bar's row is the card's bottom, mid-growth and settled"
            );
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

    /// The gap between the two cards clears the shadow one throws up onto the other:
    /// `SEPARATION` is 16, and the wide layer reaches 14 px above a card.
    #[test]
    fn the_card_gap_clears_the_shadow() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        let glass = glass();
        let gap_top = h - f.row_h - render::SEPARATION;
        // Every row of the gap is shadow, never a card's body...
        for y in gap_top..(h - f.row_h) {
            for x in crate::PAD..(w - crate::PAD) {
                assert_ne!(px(x, y), glass, "body inside the gap at ({x},{y})");
            }
        }
        // ...and the gap really is the separation, not one pixel more or less.
        assert_eq!(
            px(w / 2, gap_top - 1),
            glass,
            "the replies end above the gap"
        );
        assert_eq!(px(w / 2, h - f.row_h), glass, "the bar starts below it");
    }

    /// A selected range is marked row by row, and the rows outside it are not: the
    /// pointer's way of naming lines paints exactly what the keyboard's does.
    #[test]
    fn every_selected_row_is_inked() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        for i in 0..4 {
            scene.transcript.add_note(&format!("note {i}"), &f, 300.0);
        }
        let (w, h) = (500u32, height_for(6 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let plain = paint_highlighted(&scene, w, h, None);
        let picked = paint_highlighted(&scene, w, h, Some((1, 3)));
        // The lower half of the band: a descender from the row above can reach into
        // its top pixels, and that ink belongs to the row above, not to this one.
        let ink_in = |buf: &Vec<u8>, row: u32| {
            let y0 = row * f.row_h + f.row_h / 2;
            let y1 = row * f.row_h + f.row_h - 2;
            (crate::PAD..crate::PAD + 60)
                .flat_map(|x| (y0..y1).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        // Rows 1..=3 are inked by the selection; row 0 and row 4 are untouched.
        for row in 1..=3 {
            assert_ne!(
                ink_in(&picked, row),
                ink_in(&plain, row),
                "row {row} is marked"
            );
        }
        assert_eq!(ink_in(&picked, 0), ink_in(&plain, 0), "row 0 is not");
        // Nothing moved: the two frames differ only in ink, never in where a line is.
        let changed: Vec<u32> = (0..8)
            .filter(|row| ink_in(&picked, *row) != ink_in(&plain, *row))
            .collect();
        assert_eq!(changed, [1, 2, 3], "exactly the three selected rows");
    }

    #[test]
    fn the_scroll_thumb_shows_only_when_the_transcript_overflows() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        // One answer taller than the box can show: the box is fixed, so this is
        // what makes it scroll. A new turn, so it is the one on screen.
        scene.transcript.ask("and at dusk?", &f);
        scene.transcript.delta(
            " The same scattering leaves only the long wavelengths at dusk, which is\
             why the sky reads red, and why the sea reads grey, and why a fire reads\
             orange from a mile away.",
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

    /// The working light runs the bar's rim: the lit stretch of the edge is the one
    /// the head has just passed, and the bar's own text is still drawn under it.
    #[test]
    fn the_working_light_runs_round_the_bar() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        scene.input.insert("typing 打字");
        let (w, h) = (500u32, f.row_h);
        let (bw, bh) = frame(w, h);
        let paint_head = |head: Option<f32>| {
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
                "",
                "",
                "",
                Look::settled(State::Bar),
                head,
                1,
            );
            buf
        };
        let idle = paint_head(None);
        let dark = paint_head(Some(0.0));
        assert_ne!(idle, dark, "a request in flight shows on the rim");

        // A point a tenth of the way round the outline, probed with the head just
        // past it and with the head half a lap away.
        let card = render::Panel {
            x: render::SHADOW,
            y: render::SHADOW,
            w,
            h,
            r: render::CORNER_RADIUS,
        };
        let points = render::outline(&card);
        let on_rim = |share: f32| points[(share * points.len() as f32) as usize % points.len()];
        let (px_x, px_y) = on_rim(0.10);
        // `at` takes panel-relative coordinates; the outline is in buffer ones.
        let probe =
            |buf: &[u8]| buf[at(bw, px_x - render::SHADOW, px_y - render::SHADOW)..][..4][0];
        let behind = probe(&paint_head(Some(0.12)));
        let away = probe(&paint_head(Some(0.60)));
        assert!(
            behind > away,
            "the stretch behind the head is lit: {behind} against {away}"
        );
        // The bar keeps its text: the light did not take the field away.
        let ink = |buf: &[u8]| {
            (crate::PAD..w / 2)
                .flat_map(|x| (h / 2..h - 2).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        assert!(ink(&dark) > 0, "the typed text is drawn under the light");
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
                "",
                Look::settled(State::Replies),
                None,
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
