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

use std::time::Duration;

use crate::font::PanelFont;
use crate::layout::{self, Blocks, Input, MAX_ROWS, Style, Viewport};
use crate::render::{self, Bgra, Subpixel};
use crate::transcript::{Kind, Row, Transcript};
use crate::{MARQUEE_HOLD, MARQUEE_SPEED};

/// A question that is already out. The bar shows it instead of the input line, at a
/// scroll that depends only on how long it has been out, so nothing has to be kept
/// between frames.
#[derive(Debug, Clone, Copy)]
pub struct Pending<'a> {
    pub text: &'a str,
    pub elapsed: Duration,
    /// Set once the request ends: how long into being drawn out of the bar this frame
    /// is. The line keeps its heading and leaves the edge it was moving towards.
    pub leaving: Option<Duration>,
}

/// The panel's three shapes, as shapes rather than as the state machine's phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    /// The input bar alone, which is also the shape a running request keeps.
    Bar,
    /// The replies above, the bar below.
    Replies,
}

/// A pointer's selection, in characters: where it was anchored and where it has been
/// dragged to. Either end may be the earlier one, so a drag upwards selects the
/// same text as a drag downwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Selection {
    pub anchor: (usize, usize),
    pub cursor: (usize, usize),
}

impl Selection {
    /// The selection's two ends, earlier first.
    pub fn ends(&self) -> ((usize, usize), (usize, usize)) {
        if self.anchor <= self.cursor {
            (self.anchor, self.cursor)
        } else {
            (self.cursor, self.anchor)
        }
    }

    /// The characters of `row` this selection covers: whole rows between the two
    /// ends, and the anchored and dragged rows only in part.
    pub fn columns(&self, row: usize, len: usize) -> Option<(usize, usize)> {
        let ((r0, c0), (r1, c1)) = self.ends();
        if row < r0 || row > r1 {
            return None;
        }
        let from = if row == r0 { c0.min(len) } else { 0 };
        let to = if row == r1 { c1.min(len) } else { len };
        (from < to).then_some((from, to))
    }
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
    // The question that is out while its request runs: shown in the bar, not typed into.
    pending: Option<Pending<'_>>,
    viewport: &Viewport,
    // A pointer's selection, in characters; `None` when the pointer has not chosen.
    selection: Option<Selection>,
    // The line the keyboard names.
    highlight: Option<usize>,
    // The copy-mode cursor: the line and character it is on.
    cursor: Option<(usize, usize)>,
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
                pending,
                card.y..(card.y + card.h),
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
            selection,
            highlight,
            cursor,
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
    // The question that is out while its request runs: shown in the bar, not typed into.
    pending: Option<Pending<'_>>,
    viewport: &Viewport,
    // A pointer's selection, in characters; `None` when the pointer has not chosen.
    selection: Option<Selection>,
    // The line the keyboard names.
    highlight: Option<usize>,
    // The copy-mode cursor: the line and character it is on.
    cursor: Option<(usize, usize)>,
    caret_visible: bool,
    preedit: &str,
    // What the bar says instead of the input line: the prefix mode's instruction.
    hint: &str,
    // The bar's right-hand label: `-- COPY --` while copy mode owns the keyboard.
    label: &str,
    // Which shape this frame is, and how far it has morphed into it.
    look: Look,
    // Where the working light is on the bar's rim, 0..1 round the perimeter; the
    // light is only painted while a request is open and slow enough for one.
    head: Option<f32>,
    // How strongly the scroll indicator is drawn: 0 leaves it off entirely.
    thumb: f32,
    // The burst's multiplier and the face it is set in: above one, the indicator
    // carries the number on the strip.
    burst: Option<Burst<'_>>,
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
            pending,
            viewport,
            selection,
            highlight,
            cursor,
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
            pending,
            viewport,
            selection,
            highlight,
            cursor,
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
            buf, w, font, transcript, viewport, selection, highlight, cursor, replies, clip, 1.0,
            aa, s,
        );
    }
    render::body(buf, w, &bar, bar_tone);

    // Scroll position indicator: only while the view has recently moved, and only
    // while the transcript overflows the card.
    let total = transcript.total_height();
    if thumb > 0.0
        && let Some((track_y, track_h)) = scroll_track(h, bar_h, s)
        && let Some((ty, th)) =
            render::scroll_thumb(track_y, track_h, viewport.height, total, viewport.top)
    {
        let ink = render::dim(render::LABEL_ACCENT, thumb);
        let tw = 3 * s;
        let tx = panel.x + panel.w - (crate::PAD * s + tw) / 2;
        // Where the number goes while a burst accelerates, and the rows it takes: the
        // strip leaves those rows empty rather than being drawn through the digits.
        let label = burst.filter(|b| b.factor > 1.0).map(|burst| {
            let text = burst_label(burst.factor);
            let w = render::measure(burst.font, &text, panel.w as f32).ceil() as u32;
            let row = burst.font.row_h;
            // Centred on the strip's own column, but never past the card's right edge
            // and never off its rows: a mark on the right edge stays on the card.
            let x = (tx + tw / 2)
                .saturating_sub(w / 2)
                .min((panel.x + panel.w).saturating_sub(2 * s + w))
                .max(panel.x);
            let y = (ty + th / 2)
                .saturating_sub(row / 2)
                .clamp(track_y, (track_y + track_h).saturating_sub(row));
            Label {
                text,
                w,
                x,
                y,
                row,
                font: burst.font,
            }
        });
        let gap = label.as_ref().map(|l| (l.y, l.y + l.row));
        for (top, bottom) in strip_segments(ty, th, gap) {
            render::rect(buf, w, h, tx, top, tw, bottom - top, ink);
        }
        if let Some(label) = label {
            render::draw_text(
                buf,
                w,
                h,
                label.font,
                label.x as f32,
                render::row_baseline(label.font, label.y),
                &label.text,
                render::dim(render::FG_NORMAL, thumb),
                label.w as f32 + 1.0,
                label.y..label.y + label.row,
                aa,
            );
        }
    }

    draw_bar(
        buf,
        w,
        h,
        &bar,
        font,
        input,
        pending,
        bar.y..(bar.y + bar.h),
        caret_visible,
        preedit,
        hint,
        label,
        1.0,
        aa,
        s,
    );
    // The working light, over the rim the bar has just painted: a request in flight
    // is shown by the field's own edge, not by taking the field away. It runs while
    // the box stays up for a follow-up, so the box is never without a sign.
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

/// How much of a row at `y` survives the card's bottom edge, over a ramp of `ramp`
/// pixels: whole at or above the ramp, nothing at the edge itself.
fn edge_fade(y: u32, bottom: u32, ramp: u32) -> f32 {
    (bottom.saturating_sub(y) as f32 / ramp.max(1) as f32).clamp(0.0, 1.0)
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
    selection: Option<Selection>,
    highlight: Option<usize>,
    cursor: Option<(usize, usize)>,
    card: &render::Panel,
    clip: std::ops::Range<u32>,
    fade: f32,
    aa: Subpixel,
    s: u32,
) {
    let blocks = Blocks::new(font);
    let rows = transcript.rows(blocks);
    // A fenced block is one tint behind its rows, with the card's own corner shape: a
    // tint per row would have square corners wherever a block begins and ends.
    let pad = crate::PAD * s;
    for (top, bottom) in code_runs(&rows, card, viewport, blocks) {
        // The card's own rim and its inner band are chrome: a block that starts at the
        // card's very top edge stops under them rather than painting them over. The
        // tint keeps the block's own geometry, so its corners stay its own wherever
        // the card cuts it.
        let paint_from = top.max(clip.start).max(card.y + 2 * s);
        let paint_to = bottom
            .min(clip.end)
            .min(card.y + card.h.saturating_sub(2 * s));
        if paint_to <= paint_from {
            continue;
        }
        let tint = render::Panel {
            x: card.x + pad,
            y: top,
            w: (card.x + card.w)
                .saturating_sub(pad)
                .saturating_sub(card.x + pad),
            h: bottom - top,
            r: render::CORNER_RADIUS * s,
        };
        render::fill(
            buf,
            w,
            &tint,
            render::dim(render::BG_CODE, fade),
            paint_from..paint_to,
        );
    }
    for (index, row) in rows.iter().enumerate() {
        if !viewport.shows(row.y, blocks.row_h) {
            continue;
        }
        // The row's position inside the card is its place in the transcript less
        // how far the view has been scrolled: without the subtraction a long answer
        // is drawn below the card and nothing shows.
        // Signed on purpose: `saturating_sub` would clamp a partly scrolled row to
        // the card's top and paint it over the row below, which is exactly the
        // squashed pair of lines a scrolled view showed.
        let y = card.y as i64 + row.y as i64 - viewport.top as i64;
        if y < clip.start as i64 {
            continue;
        }
        let y = y as u32;
        if y >= clip.end {
            continue;
        }
        // A row is either inside a pointer's selection — a band behind these
        // characters — or it is the keyboard's line, inked. A pointer that has
        // selected text wins, because that is what the copy would take.
        let len = row.text.chars().count();
        let columns = selection.and_then(|sel| sel.columns(index, len));
        let mark = match columns {
            Some(cols) => RowMark::Band(cols),
            None if highlight == Some(index) => RowMark::Ink,
            None => RowMark::None,
        };
        // The copy cursor is its own mark: a bar at the character it is on, so it can
        // sit inside a selection without changing what is selected.
        let on_row = cursor.filter(|(row, _)| *row == index).map(|(_, ch)| ch);
        // The card's last row fades out rather than ending: a line half under the
        // edge leaves, and a line that fits sits whole above the ramp.
        let edge = edge_fade(y, clip.end, blocks.row_h);
        draw_row(
            buf,
            w,
            font,
            card,
            row,
            y,
            mark,
            on_row,
            clip.end,
            fade * edge,
            aa,
            s,
        );
    }
}

/// The screen-space span of every run of fenced-block rows: consecutive rows are one
/// block, and a row that is not code ends it.
fn code_runs(
    rows: &[Row<'_>],
    card: &render::Panel,
    viewport: &Viewport,
    blocks: Blocks,
) -> Vec<(u32, u32)> {
    let mut runs: Vec<(u32, u32)> = Vec::new();
    let mut open: Option<(u32, u32)> = None;
    for row in rows {
        if !row.code {
            if let Some(run) = open.take() {
                runs.push(run);
            }
            continue;
        }
        // Signed, like the row: a run can begin above the card and be clipped.
        let y = card.y as i64 + row.y as i64 - viewport.top as i64;
        let top = y.max(0) as u32;
        let bottom = (y + blocks.row_h as i64).max(0) as u32;
        match &mut open {
            Some((_, b)) if *b == top => *b = bottom,
            _ => {
                if let Some(run) = open.take() {
                    runs.push(run);
                }
                open = Some((top, bottom));
            }
        }
    }
    if let Some(run) = open {
        runs.push(run);
    }
    runs
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
    pending: Option<Pending<'_>>,
    clip: std::ops::Range<u32>,
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
    // The bar is one row tall, anchored to the card's bottom edge whatever the card
    // is doing mid-morph: the line keeps the place it will settle in, and only the
    // rows the card has unfolded are painted on, so it forms as the bar rises.
    let bar_y = (panel.y + panel.h).saturating_sub(font.row_h);
    let baseline = render::row_baseline(font, bar_y);
    // No label rail: `wlmenu` draws one only when it is given a prompt label, and
    // this bar has none, so the text starts at the padding.
    let text_x = (panel.x + p) as f32;
    let limit = (panel.x + panel.w) as f32 - p as f32 - text_x;
    // A question that is already out is not a line anyone can type into: the bar keeps
    // it, the only thing drawn there, and it scrolls by its age. There is no caret
    // because there is nowhere to put one, and no pre-edit because nothing composes it.
    if let Some(p) = pending {
        let travel = (render::advance(font, p.text) - limit).max(0.0);
        let (scroll, solid) = match p.leaving {
            Some(since) => leaving_offset(p.elapsed, since, travel, limit, s),
            None => (marquee(p.elapsed, travel, s), 1.0),
        };
        let (skip, shift) = scroll_to(font, p.text, scroll);
        render::draw_text(
            buf,
            w,
            h,
            font,
            text_x - shift,
            baseline,
            &p.text[skip..],
            render::dim(ink, solid),
            limit + shift,
            clip.clone(),
            aa,
        );
        return;
    }
    // While the prefix waits, the bar carries the instruction and no input: the
    // caret and the pre-edit belong to a line nobody is typing on.
    let composing = hint.is_empty();
    let preedit = if composing { preedit } else { "" };
    let typed = input.text();
    let caret_byte = input.caret().min(typed.len());
    let (before, after) = if composing {
        (&typed[..caret_byte], &typed[caret_byte..])
    } else {
        ("", hint)
    };
    let caret_w = 2 * s;
    // A question longer than the bar scrolls: the caret stays inside the field, so
    // a word typed wrong three sentences back can still be corrected.
    let before_w = render::advance(font, before);
    let scroll = if composing {
        caret_scroll(font, input, limit, preedit, s)
    } else {
        0.0
    };
    // Whole characters only: the line is drawn from a character boundary, shifted
    // by the bit of a glyph still hanging off the left edge.
    let (skip, shift) = if scroll > 0.0 {
        scroll_to(font, before, scroll)
    } else {
        (0, 0.0)
    };
    let mut end = render::draw_text(
        buf,
        w,
        h,
        font,
        text_x - shift,
        baseline,
        &before[skip..],
        ink,
        limit + shift,
        clip.clone(),
        aa,
    );
    // The caret sits between what is typed and what is still being composed, at the
    // byte index the input holds — which is what makes `Left` and a mid-word
    // backspace visible at all.
    if caret_visible && composing {
        let caret_bottom = (baseline - font.descent_px).round().max(0.0) as u32;
        let caret_top = caret_bottom.saturating_sub(font.line_h.ceil() as u32);
        let x = (text_x + before_w - scroll).max(text_x) as u32 + s;
        let right = (panel.x + panel.w).saturating_sub(caret_w);
        band(
            buf,
            w,
            h,
            clip.clone(),
            x.min(right),
            caret_top,
            caret_w,
            caret_bottom.saturating_sub(caret_top),
            ink,
        );
    }
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
            (text_x + limit - end).max(0.0),
            clip.clone(),
            aa,
        );
        let underline = (baseline - font.descent_px).round().max(0.0) as u32;
        band(
            buf,
            w,
            h,
            clip.clone(),
            end as u32,
            underline,
            (end_x - end).max(1.0) as u32,
            s,
            ink,
        );
        end = end_x;
    }
    if !after.is_empty() {
        render::draw_text(
            buf,
            w,
            h,
            font,
            end,
            baseline,
            after,
            ink,
            (text_x + limit - end).max(0.0),
            clip.clone(),
            aa,
        );
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
            clip,
            aa,
        );
    }
}

/// A filled band, clipped to the rows a card has unfolded.
#[allow(clippy::too_many_arguments)]
fn band(
    buf: &mut [u8],
    w: u32,
    h: u32,
    clip: std::ops::Range<u32>,
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    color: render::Bgra,
) {
    let top = y.max(clip.start);
    let bottom = (y + height).min(clip.end);
    if bottom > top {
        render::rect(buf, w, h, x, top, width, bottom - top, color);
    }
}

/// How far the input line is scrolled left, in buffer px, to keep the caret inside a
/// bar `limit` px wide. The painter and the input method's cursor rectangle both
/// need the same answer, so both ask here.
pub fn caret_scroll(font: &PanelFont, input: &Input, limit: f32, preedit: &str, s: u32) -> f32 {
    let at = input.caret().min(input.text().len());
    let before = render::advance(font, &input.text()[..at]);
    (before + render::advance(font, preedit) + (2 * s) as f32 - limit).max(0.0)
}

/// The byte to start drawing at, and the sub-character shift, that put `scroll`
/// pixels off the left of a line: `scroll` is how far the caret would otherwise
/// sit past the bar's right edge.
/// Which way the line is travelling at `elapsed`: true while it runs right to left.
///
/// The holds belong to the leg that follows them: a line paused at its far end is
/// about to come back, and one paused at its start is about to run out.
fn marquee_left(elapsed: Duration, travel: f32, s: u32) -> bool {
    if travel <= 0.0 {
        return true;
    }
    let speed = MARQUEE_SPEED * s as f32;
    let run = travel / speed;
    let leg = run + MARQUEE_HOLD.as_secs_f32();
    let t = elapsed.as_secs_f32() % (2.0 * leg);
    t < run || t >= leg + run
}

/// Where the line is and how solid it is once the request has ended: it carries on the
/// way it was going, off the edge it was heading for, fading as it goes.
fn leaving_offset(
    elapsed: Duration,
    since: Duration,
    travel: f32,
    limit: f32,
    s: u32,
) -> (f32, f32) {
    let start = marquee(elapsed, travel, s);
    // A line that never moved has no heading; it leaves to the left, the way a line
    // read to its end would go.
    let to = if marquee_left(elapsed, travel, s) {
        travel + limit
    } else {
        -limit
    };
    let p = (since.as_secs_f32() / crate::SUCK.as_secs_f32()).clamp(0.0, 1.0);
    // The square makes it leave at the speed it had and accelerate out.
    (start + (to - start) * p * p, 1.0 - p)
}

/// How far into a question that is already out the bar has scrolled, in pixels.
///
/// Right to left at `MARQUEE_SPEED`, a hold at the far end, back at the same speed, a
/// hold at the start, and round again — for as long as the request is unfinished. It is
/// a function of the age alone, so a frame that is dropped or late changes nothing.
fn marquee(elapsed: Duration, travel: f32, s: u32) -> f32 {
    if travel <= 0.0 {
        return 0.0;
    }
    let speed = MARQUEE_SPEED * s as f32;
    let run = travel / speed;
    let hold = MARQUEE_HOLD.as_secs_f32();
    let leg = run + hold;
    let t = elapsed.as_secs_f32() % (2.0 * leg);
    if t < run {
        t * speed
    } else if t < leg {
        travel
    } else if t < leg + run {
        travel - (t - leg) * speed
    } else {
        0.0
    }
}

fn scroll_to(font: &PanelFont, text: &str, scroll: f32) -> (usize, f32) {
    let mut gone = 0.0;
    for (at, ch) in text.char_indices() {
        let step = render::advance(font, ch.encode_utf8(&mut [0; 4]));
        // Keep one character back when the next would pass the scroll: a glyph
        // that is only half off the edge is drawn, shifted, not dropped.
        if gone + step > scroll {
            return (at, scroll - gone);
        }
        gone += step;
    }
    (text.len(), (scroll - gone).max(0.0))
}

/// How one reply line is marked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RowMark {
    /// Nothing: the line is drawn like its neighbours.
    None,
    /// The keyboard's line: brighter ink, two pixels in.
    Ink,
    /// A pointer's selection: these characters, on a band of their own.
    Band((usize, usize)),
}

/// One reply line: the text in its ink, marked as the keyboard or the pointer
/// asked. The band is measured from the characters themselves, so a selection ends
/// where the pointer said rather than at the end of the line.
#[allow(clippy::too_many_arguments)] // a row is where it goes plus what it says
fn draw_row(
    buf: &mut [u8],
    w: u32,
    font: &PanelFont,
    card: &render::Panel,
    row: &Row<'_>,
    y: u32,
    mark: RowMark,
    cursor: Option<usize>,
    bottom: u32,
    fade: f32,
    aa: Subpixel,
    s: u32,
) {
    // The keyboard's line moves in by 2 px and takes the primary ink, so that mark
    // is the text itself and nothing changes size. A pointer's selection is a band
    // under exactly the characters it covers, with the text in the primary ink on
    // top of it. A row's own indentation comes first: a list hangs, a quote steps in.
    let pad = crate::PAD * s;
    let inset = pad + row.indent + if mark == RowMark::Ink { 2 * s } else { 0 };
    let left = card.x + inset;
    let right = (card.x + card.w).saturating_sub(pad);
    let body = (y + font.row_h).min(bottom) - y.min(bottom);
    if row.quote && body > 0 {
        // A quote is marked by a rail at its own left edge, in the ink the panel uses
        // for labels.
        render::rect(
            buf,
            w,
            bottom,
            left.saturating_sub(pad / 2 * 3),
            y,
            s.max(1),
            body,
            render::dim(render::LABEL_ACCENT, fade),
        );
    }
    let mono_ref = font.mono();
    let mono = mono_ref.as_deref();
    if let RowMark::Band((from, to)) = mark {
        let text = row.text;
        let start = left + row_advance(font, row, byte_at(text, from), mono) as u32;
        let end = left + row_advance(font, row, byte_at(text, to), mono) as u32;
        if end > start {
            render::rect(
                buf,
                w,
                bottom,
                start,
                y,
                end - start,
                body,
                render::dim(render::BG_SEL, fade),
            );
        }
    }
    if let Some(ch) = cursor {
        // One pixel wide and as tall as the line: a caret in the text, not a box
        // around it.
        let x = left + row_advance(font, row, byte_at(row.text, ch), mono) as u32;
        render::rect(
            buf,
            w,
            bottom,
            x,
            y,
            s.max(1),
            body,
            render::dim(render::FG_SEL, fade),
        );
    }
    // A marked row is one ink, so a selection reads as one piece; otherwise each span
    // has its own, and a code span is measured and drawn in the code face. The bytes
    // between spans are drawn too, in the row's plain ink: a span is an overlay, not
    // the row.
    let marked = mark != RowMark::None;
    let base = render::row_baseline(font, y);
    let rows = y..(y + font.row_h).min(bottom);
    let mut x = left as f32;
    let limit = right as f32;
    let mut at = 0;
    for span in row.spans {
        let start = span.range.start.clamp(at, row.text.len());
        let end = span.range.end.clamp(start, row.text.len());
        if start > at {
            x = render::draw_text(
                buf,
                w,
                bottom,
                font,
                x,
                base,
                &row.text[at..start],
                render::dim(ink_of(Style::Plain, row.kind, marked), fade),
                (limit - x).max(0.0),
                rows.clone(),
                aa,
            );
        }
        if end > start {
            let face = face_of(font, mono, span.style);
            x = render::draw_text(
                buf,
                w,
                bottom,
                face,
                x,
                base,
                &row.text[start..end],
                render::dim(ink_of(span.style, row.kind, marked), fade),
                (limit - x).max(0.0),
                rows.clone(),
                aa,
            );
        }
        at = end;
    }
    if at < row.text.len() {
        render::draw_text(
            buf,
            w,
            bottom,
            font,
            x,
            base,
            &row.text[at..],
            render::dim(ink_of(Style::Plain, row.kind, marked), fade),
            (limit - x).max(0.0),
            rows,
            aa,
        );
    }
}

/// The face a span is drawn in: a code span takes the monospace chain, and falls back
/// to the text face when the system has none rather than drawing nothing. The code
/// face is resolved once per row, not once per span.
fn face_of<'a>(font: &'a PanelFont, mono: Option<&'a PanelFont>, style: Style) -> &'a PanelFont {
    match style {
        Style::Code => mono.unwrap_or(font),
        _ => font,
    }
}

/// A span's ink. `marked` is the keyboard's line or a pointer's selection, which take
/// one ink so the mark reads as one piece.
fn ink_of(style: Style, kind: Kind, marked: bool) -> Bgra {
    if marked {
        return render::FG_SEL;
    }
    match style {
        Style::Plain | Style::Em | Style::Link => colours(kind),
        // Strong and code are the brighter ink; emphasis keeps its own weight because
        // the chain has no slanted face to give it.
        Style::Strong | Style::Code => render::FG_PROMPT,
    }
}

/// Advance of the first `byte` bytes of a row, each span measured in its own face, so
/// a selection's band ends exactly where its text does even when that text is code.
fn row_advance(font: &PanelFont, row: &Row<'_>, byte: usize, mono: Option<&PanelFont>) -> f32 {
    let byte = byte.min(row.text.len());
    let mut at = 0;
    let mut width = 0.0;
    for span in row.spans {
        if at < byte && at < span.range.start {
            let end = span.range.start.min(byte);
            width += render::advance(font, &row.text[at..end]);
            at = end;
        }
        if at < byte {
            let end = span.range.end.min(byte);
            width += render::advance(face_of(font, mono, span.style), &row.text[at..end]);
            at = end;
        }
        if at >= byte {
            break;
        }
    }
    if at < byte {
        width += render::advance(font, &row.text[at..byte]);
    }
    width
}

/// The byte offset of character `index` in `text`: the selection counts characters,
/// the painter counts bytes.
fn byte_at(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

/// A reply's ink: the secondary one for what the model said, so a failure or a
/// note reads as the panel's own voice without changing tone.
fn colours(_kind: Kind) -> Bgra {
    render::FG_NORMAL
}

/// The rows the scroll indicator's track spans, in buffer pixels, inside a panel `h`
/// tall whose bar is `bar_h`: the replies card, less the gap that keeps the bar
/// still. The painter puts the strip here and the pointer grabs it here, so the
/// strip drawn is the strip grabbed. `None` when there is no card to draw it on.
pub fn scroll_track(h: u32, bar_h: u32, scale: u32) -> Option<(u32, u32)> {
    let s = scale.max(1);
    let top = render::SHADOW * s;
    let track = h.saturating_sub(2 * top + bar_h + render::SEPARATION * s);
    (track > 0).then_some((top, track))
}

/// The column a press grabs the indicator from, in buffer pixels, and the card's own
/// right edge: the replies' `PAD`, which is where the 3 px strip is painted and
/// where no reply text is, because rows wrap at the same `PAD`.
pub fn scroll_column(w: u32, scale: u32) -> (u32, u32) {
    let s = scale.max(1);
    let right = w.saturating_sub(render::SHADOW * s);
    (right.saturating_sub(crate::PAD * s), right)
}

/// The multiplier a capsule shows: one decimal only when it is not whole, so the two
/// halves of a burst's ramp read as `1.5x` and the whole ones as `4x`.
pub fn burst_label(factor: f32) -> String {
    if (factor - factor.round()).abs() < 0.05 {
        format!("{}x", factor.round() as i32)
    } else {
        format!("{factor:.1}x")
    }
}

/// The burst's multiplier as the indicator carries it: the factor, and the smaller
/// face it is set in so a number on the right edge cannot read as a line of the
/// answer.
#[derive(Clone, Copy)]
pub struct Burst<'a> {
    pub factor: f32,
    pub font: &'a PanelFont,
}

/// The number the strip carries while a burst accelerates: its text, the width it
/// measured, and the box it takes on the card. The strip leaves that box empty.
struct Label<'a> {
    text: String,
    w: u32,
    x: u32,
    y: u32,
    row: u32,
    font: &'a PanelFont,
}

/// The pieces of the strip a gap leaves: the rows of it above the gap and below it,
/// and no fragment shorter than two pixels, which reads as a speck rather than as a
/// bar. A gap that covers the strip leaves nothing — the number's own place then says
/// where the view is.
pub fn strip_segments(ty: u32, th: u32, gap: Option<(u32, u32)>) -> Vec<(u32, u32)> {
    let end = ty + th;
    let Some((gap_top, gap_bottom)) = gap else {
        return vec![(ty, end)];
    };
    let mut pieces = Vec::new();
    for (top, bottom) in [(ty, gap_top.min(end)), (gap_bottom.clamp(ty, end), end)] {
        if bottom.saturating_sub(top) >= 2 {
            pieces.push((top, bottom));
        }
    }
    pieces
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
    bar_height_at(crate::font::size())
}

/// The bar's height at a face size: with a zoom the bar grows with the glyphs, and
/// the panel's other rows are measured against it.
pub fn bar_height_at(size: f32) -> u32 {
    crate::font::row_h_for(size)
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

    /// The panel's own `draw` with no burst, so every frame here but the capsule's own
    /// test is the strip: a burst is one argument the rest of the suite does not name.
    #[allow(clippy::too_many_arguments)] // the same list the painter itself takes
    fn draw(
        buf: &mut [u8],
        w: u32,
        h: u32,
        font: &PanelFont,
        transcript: &Transcript,
        input: &Input,
        pending: Option<Pending<'_>>,
        viewport: &Viewport,
        selection: Option<Selection>,
        highlight: Option<usize>,
        cursor: Option<(usize, usize)>,
        caret_visible: bool,
        preedit: &str,
        hint: &str,
        label: &str,
        look: Look,
        head: Option<f32>,
        thumb: f32,
        scale: u32,
    ) {
        #[allow(clippy::too_many_arguments)]
        super::draw(
            buf,
            w,
            h,
            font,
            transcript,
            input,
            pending,
            viewport,
            selection,
            highlight,
            cursor,
            caret_visible,
            preedit,
            hint,
            label,
            look,
            head,
            thumb,
            None,
            scale,
        );
    }

    fn font() -> PanelFont {
        PanelFont::load(crate::font::SIZE).expect("system font available")
    }

    /// The face the indicator's multiplier is set in: the panel's size times the share
    /// the panel itself uses, so what a frame shows is what the session draws.
    fn small() -> PanelFont {
        PanelFont::load(crate::font::SIZE * crate::LABEL_SHARE).expect("system font available")
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

    /// One frame with a pointer selection.
    fn paint_selected(scene: &Scene, w: u32, h: u32, selection: Selection) -> Vec<u8> {
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
            None,
            &scene.viewport,
            Some(selection),
            None,
            None,
            true,
            "",
            "",
            "",
            Look::settled(State::Replies),
            None,
            0.0,
            1,
        );
        buf
    }

    fn paint_highlighted(scene: &Scene, w: u32, h: u32, marked: Option<usize>) -> Vec<u8> {
        paint_with(scene, w, h, marked, 0.0, None)
    }

    /// One frame with the scroll indicator at `thumb` strength and, when `burst` is
    /// above one, as the capsule that carries it. 0 is the indicator a panel at rest
    /// draws, and what every other frame here holds.
    #[allow(clippy::too_many_arguments)]
    fn paint_with(
        scene: &Scene,
        w: u32,
        h: u32,
        marked: Option<usize>,
        thumb: f32,
        burst: Option<f32>,
    ) -> Vec<u8> {
        let f = font();
        let label = small();
        let burst = burst.map(|factor| Burst {
            factor,
            font: &label,
        });
        let (bw, bh) = frame(w, h);
        let mut buf = vec![0u8; (bw * bh * 4) as usize];
        super::draw(
            &mut buf,
            bw,
            bh,
            &f,
            &scene.transcript,
            &scene.input,
            None,
            &scene.viewport,
            None,
            marked,
            None,
            true,
            "",
            "",
            "",
            Look::settled(State::Replies),
            None,
            thumb,
            burst,
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::moving(State::Bar, State::Bar, 1.0, rise, spread),
                None,
                0.0,
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                look,
                None,
                0.0,
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::moving(State::Bar, State::Bar, 1.0, rise, 1.0),
                None,
                0.0,
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

    /// The line forms as the bar unfolds: at a third of the rise the text is not
    /// floating above the card it belongs to, it is simply not painted yet.
    #[test]
    fn the_line_forms_with_the_bar() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        scene.input.insert("what the bar is typing");
        let (w, h) = (500u32, f.row_h);
        let (bw, bh) = frame(w, h);
        let paint_rise = |rise: f32| {
            let mut buf = vec![0u8; (bw * bh * 4) as usize];
            draw(
                &mut buf,
                bw,
                bh,
                &f,
                &scene.transcript,
                &scene.input,
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::moving(State::Bar, State::Bar, 1.0, rise, 1.0),
                None,
                0.0,
                1,
            );
            buf
        };
        // Text is much brighter than the card's fill, so any column with a lit pixel
        // is text or the caret.
        let luma = |px: &[u8]| px[0] as u32 + px[1] as u32 + px[2] as u32;
        let lit_rows = |buf: &[u8]| -> Vec<u32> {
            (0..h)
                .filter(|&y| (0..w).any(|x| luma(&buf[at(bw, x, y)..][..4]) > 400))
                .collect()
        };
        let quarter = lit_rows(&paint_rise(0.25));
        let half = lit_rows(&paint_rise(0.5));
        let full = lit_rows(&paint_rise(1.0));
        assert!(!full.is_empty(), "the line is on a settled bar");
        assert!(
            quarter.len() < half.len() && half.len() < full.len(),
            "the line grows with the bar: {quarter:?} {half:?} {full:?}"
        );
        // Nothing is painted above the card's top: the fade is already all but full
        // at these rises, so only the clip can explain a row being missing - which is
        // what stops the text hovering over a bar that has not unfolded yet. (Below
        // half the card is only the row's descender space, where there is no line to
        // cut.)
        let card_top = |rise: f32| -> u32 {
            let card = card_of(
                State::Bar,
                &render::Panel {
                    x: 0,
                    y: 0,
                    w,
                    h,
                    r: 0,
                },
                f.row_h,
            );
            let risen_h = 2.0 + (card.h as f32 - 2.0) * rise;
            (card.y + card.h) - risen_h.round() as u32
        };
        let three_quarters = lit_rows(&paint_rise(0.75));
        for (rise, rows) in [(0.5, &half), (0.75, &three_quarters)] {
            let top = *rows.first().expect("some of the line has unfolded");
            assert!(
                top >= card_top(rise),
                "text above the card at {rise}: {top} < {}",
                card_top(rise)
            );
        }
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                preedit,
                "",
                "",
                Look::settled(State::Replies),
                None,
                0.0,
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                caret,
                "",
                hint,
                "",
                Look::settled(State::Replies),
                None,
                0.0,
                1,
            );
            buf
        };
        let typing = paint_with("", true);
        let prefix = paint_with("press [ for copy mode, + / - for text size", false);
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

    /// A fenced block's tint takes the card's own corner shape: its corners show the
    /// card's tone, not the tint, and the middle of it is tinted.
    #[test]
    fn a_code_block_is_tinted_with_rounded_corners() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        // A paragraph first, so the block's own top corners are on screen and not
        // under the card's chrome.
        scene.transcript.add_note(
            "before the block\n\n```\nlet x = 1;\nlet y = 2;\nlet z = 3;\n```",
            &f,
            300.0,
        );
        let (w, h) = (500u32, height_for(6 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let buf = paint(&scene, w, h);
        let tint = render::premul(render::BG_CODE);
        let px = |x: u32, y: u32| &buf[at(bw, x, y)..][..4];
        // The tint's own box: every pixel of it, wherever the block was placed.
        let (mut x0, mut y0, mut x1, mut y1) = (u32::MAX, u32::MAX, 0, 0);
        for y in 0..h {
            for x in 0..w {
                if px(x, y) == &tint[..] {
                    x0 = x0.min(x);
                    y0 = y0.min(y);
                    x1 = x1.max(x);
                    y1 = y1.max(y);
                }
            }
        }
        assert!(x1 > x0 && y1 > y0, "the block is tinted at all");
        // Its four corners are the card's, so they are not the tint: that is the
        // rounding, and a square corner would leave all four filled.
        for (x, y) in [(x0, y0), (x1 - 1, y0), (x0, y1 - 1), (x1 - 1, y1 - 1)] {
            assert_ne!(
                px(x, y),
                &tint[..],
                "corner ({x},{y}) is square: the block's tint has no radius"
            );
        }
        // And its middle is tinted, so the corners are rounded rather than absent.
        assert_eq!(px((x0 + x1) / 2, (y0 + y1) / 2), &tint[..]);
        // The radius is the card's, not a token rounding.
        assert!(
            x1 - x0 > 2 * render::CORNER_RADIUS && y1 - y0 > 2 * render::CORNER_RADIUS,
            "the block is big enough for the radius to show: {}x{}",
            x1 - x0,
            y1 - y0
        );
    }

    /// The copy cursor is a bar at the character it is on, drawn inside a selection
    /// without changing it: both are visible in the same frame.
    #[test]
    fn the_copy_cursor_is_drawn_where_the_cursor_is() {
        let f = font();
        let mut scene = scene(&f, 60.0);
        scene.transcript = Transcript::default();
        scene
            .transcript
            .add_note("one two three four five six seven", &f, 60.0);
        let (w, h) = (500u32, height_for(4 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let paint_cursor = |cursor: Option<(usize, usize)>| {
            let mut buf = vec![0u8; (bw * (h + 2 * render::SHADOW) * 4) as usize];
            let (bw2, bh2) = frame(w, h);
            draw(
                &mut buf,
                bw2,
                bh2,
                &f,
                &scene.transcript,
                &scene.input,
                None,
                &scene.viewport,
                None,
                Some(0),
                cursor,
                true,
                "",
                "",
                "",
                Look::settled(State::Replies),
                None,
                0.0,
                1,
            );
            buf
        };
        let without = paint_cursor(None);
        let at_start = paint_cursor(Some((0, 0)));
        let later = paint_cursor(Some((0, 6)));
        assert_ne!(without, at_start, "the cursor is drawn");
        assert_ne!(at_start, later, "and it moves with the character");
        // It sits on the row, in the text's own column: the only pixels it adds are
        // the bar's, so the columns that changed are exactly the bar.
        let bar = |buf: &Vec<u8>| {
            let changed: Vec<u32> = (crate::PAD..crate::PAD + 160)
                .filter(|&x| {
                    (0..f.row_h).any(|y| buf[at(bw, x, y)..][..4] != without[at(bw, x, y)..][..4])
                })
                .collect();
            changed
        };
        // The bar sits where the row's text begins: the row's own padding, plus the
        // two pixels the highlighted line is inset by.
        let start = bar(&at_start);
        assert_eq!(start.len(), 1, "one column wide");
        assert!(
            (crate::PAD..crate::PAD + 6).contains(&start[0]),
            "at the start of the line: {start:?}"
        );
        let later_bar = bar(&later);
        assert_eq!(later_bar.len(), 1, "still one column wide");
        assert!(
            later_bar[0] > crate::PAD,
            "a later character is further along: {later_bar:?}"
        );
    }

    /// The caret is the input's own byte index, not the end of the line: without
    /// that, `Left` moved the model's caret and nothing on screen moved with it.
    #[test]
    fn the_caret_sits_where_the_input_caret_is() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        scene.input.insert("hello");
        let (w, h) = (500u32, height_for(2 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        // The caret's column in the bar: the pixel it paints that the line does not.
        let caret_column = |input: &Input| {
            let (mut with, without) = (vec![0u8; (bw * bh * 4) as usize], {
                let mut b = vec![0u8; (bw * bh * 4) as usize];
                draw(
                    &mut b,
                    bw,
                    bh,
                    &f,
                    &scene.transcript,
                    input,
                    None,
                    &scene.viewport,
                    None,
                    None,
                    None,
                    false,
                    "",
                    "",
                    "",
                    Look::settled(State::Bar),
                    None,
                    0.0,
                    1,
                );
                b
            });
            draw(
                &mut with,
                bw,
                bh,
                &f,
                &scene.transcript,
                input,
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::settled(State::Bar),
                None,
                0.0,
                1,
            );
            let bar_top = crate::PAD;
            (crate::PAD..w - crate::PAD)
                .filter(|&x| {
                    (bar_top..h).any(|y| with[at(bw, x, y)..][..4] != without[at(bw, x, y)..][..4])
                })
                .max()
                .expect("the caret paints")
        };
        let mut at_end = Input::default();
        at_end.insert("hello");
        let mut at_start = at_end.clone();
        at_start.home();
        let mut mid = at_end.clone();
        mid.home();
        for _ in 0..3 {
            mid.right();
        }
        let (start, middle, end) = (
            caret_column(&at_start),
            caret_column(&mid),
            caret_column(&at_end),
        );
        assert!(start < middle, "the caret follows left: {start} {middle}");
        assert!(middle < end, "and right: {middle} {end}");
    }

    /// A question longer than the bar scrolls instead of running off the card, and
    /// the caret stays inside the field where it can be seen.
    #[test]
    fn a_long_question_scrolls_to_keep_its_caret() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript = Transcript::default();
        let mut input = Input::default();
        input.insert(&"wide words keep coming ".repeat(8));
        let (w, h) = (500u32, height_for(2 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let paint = |input: &Input, caret: bool| {
            let mut buf = vec![0u8; (bw * bh * 4) as usize];
            draw(
                &mut buf,
                bw,
                bh,
                &f,
                &scene.transcript,
                input,
                None,
                &scene.viewport,
                None,
                None,
                None,
                caret,
                "",
                "",
                "",
                Look::settled(State::Bar),
                None,
                0.0,
                1,
            );
            buf
        };
        // A column carrying text: the ink is far brighter than the bar's own fill,
        // so anything over the fill's luma is a glyph (or the caret).
        let luma = |px: &[u8]| px[0] as u32 + px[1] as u32 + px[2] as u32;
        let lit =
            |buf: &[u8], x: u32| (crate::PAD..h).any(|y| luma(&buf[at(bw, x, y)..][..4]) > 400);
        let end_frame = paint(&input, false);
        // The line runs to the bar's right padding instead of stopping short: that
        // was the "the input only takes so many characters" of a line that simply
        // stopped being painted.
        assert!(
            (w - crate::PAD - 40..w - crate::PAD).any(|x| lit(&end_frame, x)),
            "the line fills the bar"
        );
        assert!(
            lit(&end_frame, crate::PAD + 1),
            "and starts at its left edge"
        );
        // The caret is inside the field with the caret at the very end of a long
        // line, which is the whole point of scrolling.
        let with_caret = paint(&input, true);
        let caret_column = (crate::PAD..w - crate::PAD)
            .filter(|&x| {
                (crate::PAD..h)
                    .any(|y| with_caret[at(bw, x, y)..][..4] != end_frame[at(bw, x, y)..][..4])
            })
            .max()
            .expect("the caret paints");
        assert!(
            caret_column <= w - crate::PAD,
            "the caret stays inside the bar: {caret_column}"
        );
        // Home scrolls back, so the line moves under the caret rather than being
        // cut off: the two frames are not the same picture.
        let mut home = input.clone();
        home.home();
        assert_ne!(end_frame, paint(&home, false), "the line scrolled");
        assert!(
            lit(&paint(&home, false), crate::PAD + 1),
            "still starts at the edge"
        );
    }

    /// The question stays in the bar while it is out, and it is the bar's whole line
    /// there: no caret, and a scroll that follows its age alone.
    #[test]
    fn a_question_that_is_out_stays_in_the_bar_and_scrolls_with_its_age() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let long = "a question long enough that the bar cannot hold all of it at once, \
                    and then a few words more";
        let paint =
            |elapsed: Duration, leaving: Option<Duration>, caret: bool, text: Option<&str>| {
                let mut buf = vec![0u8; (bw * bh * 4) as usize];
                draw(
                    &mut buf,
                    bw,
                    bh,
                    &f,
                    &scene.transcript,
                    &scene.input,
                    text.map(|text| Pending {
                        text,
                        elapsed,
                        leaving,
                    }),
                    &scene.viewport,
                    None,
                    None,
                    None,
                    caret,
                    "",
                    "",
                    "",
                    Look::settled(State::Replies),
                    None,
                    0.0,
                    1,
                );
                buf
            };
        let rows = (h - f.row_h + 2)..(h - 2);
        let glass = glass();
        let inked = |buf: &[u8]| {
            (crate::PAD..w - crate::PAD)
                .map(|x| {
                    rows.clone()
                        .filter(|&y| buf[at(bw, x, y)..][..4][..3] != glass[..3])
                        .count()
                })
                .filter(|&n| n > 0)
                .count()
        };

        let start = paint(Duration::ZERO, None, false, Some(long));
        assert!(inked(&start) > 0, "the question is drawn, at the left edge");
        assert_ne!(
            start,
            paint(Duration::from_secs(4), None, false, Some(long)),
            "four seconds in, it has scrolled"
        );
        // A question that fits has nowhere to go: the same frame at any age.
        let short = paint(Duration::ZERO, None, false, Some("hi"));
        assert_eq!(
            short,
            paint(Duration::from_secs(60), None, false, Some("hi")),
            "a question that fits does not move"
        );
        // The exit starts as the scroll and runs the line out of the bar: an inkless
        // bar within `SUCK` is the same frame as an empty one.
        assert_eq!(
            paint(
                Duration::from_secs(1),
                Some(Duration::ZERO),
                false,
                Some(long)
            ),
            paint(Duration::from_secs(1), None, false, Some(long)),
            "the exit begins where the scroll was"
        );
        assert_eq!(
            paint(Duration::from_secs(1), Some(crate::SUCK), false, Some(long)),
            paint(Duration::from_secs(1), Some(crate::SUCK), false, None),
            "and the bar is empty once it has left"
        );

        // There is no caret to draw on a line nobody can type into, whatever the
        // keyboard says.
        assert_eq!(
            paint(Duration::from_secs(2), None, true, Some(long)),
            paint(Duration::from_secs(2), None, false, Some(long)),
            "a question that is out takes the caret with it"
        );
    }

    /// The caret is the painter's argument, not its guess: with the flag off the bar
    /// draws the text and nothing at the caret's column, so a panel that has no keyboard
    /// can be painted as one. Without this the caret could be reinstated by a frame that
    /// ignores the flag and no test would notice.
    #[test]
    fn the_painter_draws_the_caret_only_when_it_is_told_to() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.input.insert("hi");
        let (w, h) = sized(&scene, &f);
        let (bw, bh) = frame(w, h);
        let paint_with = |caret: bool| {
            let mut buf = vec![0u8; (bw * bh * 4) as usize];
            draw(
                &mut buf,
                bw,
                bh,
                &f,
                &scene.transcript,
                &scene.input,
                None,
                &scene.viewport,
                None,
                None,
                None,
                caret,
                "",
                "",
                "",
                Look::settled(State::Replies),
                None,
                0.0,
                1,
            );
            buf
        };
        let with = paint_with(true);
        let without = paint_with(false);
        // The bar's own text rows, inside the rim's bands.
        let rows = (h - f.row_h + 2)..(h - 2);
        let glass = glass();
        let inked = |buf: &[u8], x: u32| {
            rows.clone()
                .any(|y| buf[at(bw, x, y)..][..4][..3] != glass[..3])
        };
        let differing: Vec<u32> = (crate::PAD..w - crate::PAD)
            .filter(|&x| inked(&with, x) != inked(&without, x))
            .collect();
        assert_eq!(
            differing.len(),
            2,
            "the caret is two pixels wide and it is the only difference: {differing:?}"
        );
        for x in &differing {
            assert!(
                inked(&with, *x) && !inked(&without, *x),
                "column {x}: the caret is there only when the painter is told to draw it"
            );
        }
        // And the text itself is in both frames: the flag removes the caret, not the line.
        assert!(
            (crate::PAD..w - crate::PAD).any(|x| inked(&without, x)),
            "the typed line is still drawn without the caret"
        );
    }

    /// The heading is what decides which edge the line leaves by, so it has to be right
    /// through the holds too: a line paused at its end is coming back, one paused at its
    /// start is about to run out.
    #[test]
    fn the_line_heads_for_the_edge_it_is_moving_towards() {
        let travel = 200.0;
        let s = 1;
        let run = travel / MARQUEE_SPEED;
        let hold = MARQUEE_HOLD.as_secs_f32();
        let leg = run + hold;
        let at = |secs: f32| marquee_left(Duration::from_secs_f32(secs), travel, s);

        assert!(at(0.0), "it starts by running out, to the left");
        assert!(at(run / 2.0), "and keeps running out");
        assert!(
            !at(run + hold / 2.0),
            "paused at its end: it will come back right"
        );
        assert!(!at(leg + run / 2.0), "coming back");
        assert!(
            at(leg + run + hold / 2.0),
            "paused at its start: it will run out again"
        );
        assert!(at(2.0 * leg + run / 2.0), "the second round is the first");
        // A line that fits never moved, so it leaves the way a read line would: left.
        assert!(marquee_left(Duration::from_secs(3), 0.0, s));
    }

    /// The exit is the scroll read one more way: where it was at that instant, then one
    /// accelerated run past the edge it was heading for, fading out.
    #[test]
    fn a_finished_question_is_drawn_off_the_edge_it_was_heading_for() {
        let (travel, limit, s) = (200.0, 300.0, 1);
        let run = travel / MARQUEE_SPEED;
        let hold = MARQUEE_HOLD.as_secs_f32();
        let leg = run + hold;

        // Outbound: the line is running left, so it leaves by the left edge.
        let out = Duration::from_secs_f32(run / 2.0);
        let (x, solid) = leaving_offset(out, Duration::ZERO, travel, limit, s);
        assert_eq!(x, marquee(out, travel, s), "the exit starts on the scroll");
        assert_eq!(solid, 1.0, "at full ink");
        let (x, solid) = leaving_offset(out, crate::SUCK, travel, limit, s);
        assert_eq!(x, travel + limit, "and ends past the left edge");
        assert_eq!(solid, 0.0, "with no ink left");

        // Coming back: it is running right, so it leaves by the right edge, which is a
        // negative offset: the line is pushed out past the right edge.
        let back = Duration::from_secs_f32(leg + run / 2.0);
        let (x, _) = leaving_offset(back, crate::SUCK, travel, limit, s);
        assert_eq!(x, -limit, "and ends past the right edge");

        // Both keep going the way they were going, and neither arrives before its time.
        let (early, solid) = leaving_offset(out, crate::SUCK / 2, travel, limit, s);
        assert!(early > marquee(out, travel, s) && early < travel + limit);
        assert!((0.0..1.0).contains(&solid), "half-way is half solid");
    }

    /// A question too long for the bar is read, not flicked past: it runs at
    /// `MARQUEE_SPEED`, holds at each end for `MARQUEE_HOLD`, and turns round — for as
    /// long as the request is out. The offset is a function of the age alone, so a
    /// dropped frame cannot make it drift.
    #[test]
    fn the_question_scrolls_as_far_as_it_has_to_and_turns_round() {
        let travel = 200.0;
        let s = 1;
        let run = travel / MARQUEE_SPEED;
        let hold = MARQUEE_HOLD.as_secs_f32();
        let leg = run + hold;
        let at = |secs: f32| marquee(Duration::from_secs_f32(secs), travel, s);

        assert_eq!(at(0.0), 0.0, "it starts at the first character");
        assert!(at(run / 2.0) > travel / 4.0, "it is moving a moment later");
        assert_eq!(at(run), travel, "it reaches its last character");
        assert_eq!(
            at(run + hold / 2.0),
            travel,
            "and holds there so the turn is visible"
        );
        assert!(at(leg + run / 2.0) < travel, "then it comes back");
        assert_eq!(at(2.0 * leg), 0.0, "all the way, one full round later");
        assert_eq!(
            at(2.0 * leg - hold / 2.0),
            0.0,
            "and holds at the start, too"
        );
        assert!(
            (at(2.0 * leg + run / 2.0) - at(run / 2.0)).abs() < 0.01,
            "and turns round again: the second round is the first"
        );

        // Nowhere in between is it off the line, and it never jumps.
        let step = 1.0 / 60.0;
        let mut last = 0.0;
        let mut t = 0.0;
        while t < 12.0 {
            let x = at(t);
            assert!((0.0..=travel).contains(&x), "{t}s: {x} is off the line");
            assert!(
                (x - last).abs() <= MARQUEE_SPEED * step + 0.01,
                "{t}s: {last} -> {x} is a jump"
            );
            last = x;
            t += step;
        }

        // A question that fits has nowhere to go.
        for t in [0.0, 1.0, 5.0, 60.0] {
            assert_eq!(
                marquee(Duration::from_secs_f32(t), 0.0, s),
                0.0,
                "a line that fits does not move"
            );
        }
    }

    /// Copy mode has to say so: the bar reads `-- COPY --` on the right and the
    /// caret is gone, so a swallowed keystroke is never a mystery.
    #[test]
    fn the_bar_says_when_copy_mode_is_on() {
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                caret,
                "",
                "",
                label,
                Look::settled(State::Replies),
                None,
                0.0,
                1,
            );
            buf
        };
        let plain = paint_with("", true);
        let copy = paint_with("-- COPY --", false);
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
        assert!(ink(&copy, right.0, right.1) > 0, "the mode is on the bar");
        // The caret is gone: with an empty input it sits at the padding, and only
        // the typing frame has it.
        assert!(ink(&plain, crate::PAD, crate::PAD + 3) > 0, "the caret");
        assert_eq!(
            ink(&copy, crate::PAD, crate::PAD + 3),
            0,
            "no caret in copy mode"
        );
    }

    #[test]
    fn the_highlight_is_ink_not_a_mark() {
        let f = font();
        let scene = scene(&f, 300.0);
        let (w, h) = sized(&scene, &f);
        let plain = paint_highlighted(&scene, w, h, None);
        let lit = paint_highlighted(&scene, w, h, Some(0));
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                look,
                None,
                0.0,
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

    /// The reported defect: a scrolled view drew the row that was half above the
    /// card, and its descenders landed on the row below — two lines squashed into
    /// one. A row that does not fit the card is dropped whole.
    #[test]
    fn a_half_scrolled_row_is_not_drawn() {
        let f = font();
        let mut scene = scene(&f, 60.0);
        scene.transcript = Transcript::default();
        scene.transcript.add_note(
            "one two three four five six seven eight nine ten eleven twelve",
            &f,
            60.0,
        );
        let room = 3 * f.row_h;
        let (w, h) = (500u32, height_for(room, &f, 1));
        let (bw, _) = frame(w, h);
        // Scrolled by half a row: the first row's band is above the card's top, and
        // the second starts a half row down, inside the card.
        scene.viewport = Viewport::new(transcript_px(h, &f, 1));
        scene.viewport.top = f.row_h / 2;
        let buf = paint(&scene, w, h);
        let ink = |from: u32, to: u32| {
            (crate::PAD..w - crate::PAD)
                .flat_map(|x| (from..to).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4][..3] != glass()[..3])
                .count()
        };
        // Nothing of the dropped row's lower half is left in the card's first half
        // row: that is the half line that used to sit squashed above the next one.
        assert_eq!(
            ink(0, f.row_h / 2),
            0,
            "the dropped row left nothing behind"
        );
        // The row below is drawn where its place says, a half row down.
        assert!(
            ink(f.row_h / 2, f.row_h + f.row_h / 2) > 0,
            "the next row is drawn"
        );
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

    /// A pointer's selection is a band under the characters it names: the rows
    /// between its ends whole, and the last one cut where the pointer stopped.
    #[test]
    fn a_selection_is_a_band_under_its_characters() {
        let f = font();
        // One block of many rows, so a row index is a row band with no gap in it.
        let mut scene = scene(&f, 60.0);
        scene.transcript = Transcript::default();
        scene.transcript.add_note(
            "one two three four five six seven eight nine ten eleven twelve",
            &f,
            60.0,
        );
        assert!(
            scene.transcript.total_rows() >= 6,
            "enough rows to select in"
        );
        let (w, h) = (500u32, height_for(6 * f.row_h, &f, 1));
        let (bw, _) = frame(w, h);
        let plain = paint_highlighted(&scene, w, h, None);
        let picked = paint_selected(
            &scene,
            w,
            h,
            Selection {
                anchor: (1, 0),
                cursor: (3, 2),
            },
        );
        // The band is translucent over the card, so what it looks like is the
        // composite, not the token.
        let mut band_px = render::premul(render::BG_PROMPT);
        render::blend_px(
            &mut band_px,
            [render::BG_SEL[0], render::BG_SEL[1], render::BG_SEL[2]],
            render::BG_SEL[3] as u32,
        );
        // Count the band itself, not the ink: the band is what a selection adds.
        let band = |buf: &Vec<u8>, row: u32, x0: u32, x1: u32| {
            (x0..x1)
                .flat_map(|x| (row * f.row_h..(row + 1) * f.row_h).map(move |y| (x, y)))
                .filter(|&(x, y)| buf[at(bw, x, y)..][..4] == band_px)
                .count()
        };
        let row_all = (crate::PAD, w - crate::PAD);
        assert_eq!(band(&picked, 0, row_all.0, row_all.1), 0, "row 0 is not");
        for row in 1..=3 {
            assert!(band(&picked, row, row_all.0, row_all.1) > 0, "row {row} is");
        }
        assert_eq!(band(&picked, 4, row_all.0, row_all.1), 0, "row 4 is not");
        // The last row's band stops after its second character.
        assert!(
            band(&picked, 3, crate::PAD, crate::PAD + 30) > 0,
            "it starts"
        );
        assert_eq!(
            band(&picked, 3, crate::PAD + 120, w - crate::PAD),
            0,
            "and ends there"
        );
        // Nothing moved: without a selection there is no band at all.
        assert_eq!(band(&plain, 2, row_all.0, row_all.1), 0);
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
        let buf = paint_with(&scene, w, h, None, 1.0, None);
        let tw = 3;
        let tx = w - (crate::PAD + tw) / 2;
        assert_eq!(
            &buf[at(bw, tx, 2)..][..4][..3],
            &render::LABEL_ACCENT[..3],
            "the overflow indicator is at the card's right edge"
        );
        // The same overflowing transcript nobody has scrolled: nothing on the chrome.
        let still = paint(&scene, w, h);
        assert_ne!(
            &still[at(bw, tx, 2)..][..4][..3],
            &render::LABEL_ACCENT[..3],
            "at rest the indicator is not drawn"
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

    /// The strip painted is the strip grabbed: every pixel of the indicator is inside
    /// the card's right padding column and inside the track, which are the two the
    /// pointer reads. A change to `PAD`, `SHADOW` or the gap moves both or neither.
    #[test]
    fn the_indicator_is_drawn_where_the_pointer_grabs_it() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript.ask("and at dusk?", &f);
        scene.transcript.delta(
            " The same scattering leaves only the long wavelengths at dusk, which is\
             why the sky reads red, and why the sea reads grey, and why a fire reads\
             orange from a mile away.",
            &f,
            300.0,
        );
        scene.transcript.finish_answer(&f);
        scene.viewport = Viewport::new(2 * f.row_h);
        let (w, h) = (500u32, height_for(2 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let buf = paint_with(&scene, w, h, None, 1.0, None);
        let (left, right) = scroll_column(bw, 1);
        let (top, tall) = scroll_track(bh, f.row_h, 1).expect("a replies card to draw on");
        // The column is the padding and nothing else: it starts where the rows wrap
        // and ends at the card's own edge.
        assert_eq!(left, bw - render::SHADOW - crate::PAD);
        assert_eq!(right, bw - render::SHADOW);
        assert!(
            scroll_track(f.row_h, f.row_h, 1).is_none(),
            "a bar with no room for replies has no track"
        );
        let (mut seen, mut xs) = (0, Vec::new());
        for y in 0..bh {
            for x in 0..bw {
                let px = &buf[((y * bw + x) * 4) as usize..][..3];
                if px != &render::LABEL_ACCENT[..3] {
                    continue;
                }
                seen += 1;
                assert!(
                    (left..right).contains(&x),
                    "the indicator at x {x} is outside the column {left}..{right}"
                );
                assert!(
                    (top..top + tall).contains(&y),
                    "the indicator at y {y} is outside the track {top}..{}",
                    top + tall
                );
                xs.push(x);
            }
        }
        xs.sort_unstable();
        xs.dedup();
        assert_eq!(xs.len(), 3, "the indicator is 3 px wide: {xs:?}");
        assert!(seen > 0, "nothing was drawn where the pointer grabs");
    }

    /// A burst writes the multiplier on the strip: the strip keeps its 3 px and its
    /// length, the number is set over it at the thumb's middle, and its glyphs are
    /// shorter than a row of the answer.
    #[test]
    fn a_burst_writes_the_multiplier_on_the_strip() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.transcript.ask("and at dusk?", &f);
        // Enough answer to overflow the box several times over, so the thumb is tall:
        // the number's rows are left empty, so the strip needs rows above and below it
        // to be interrupted in the middle.
        for _ in 0..6 {
            scene.transcript.delta(
                " The same scattering leaves only the long wavelengths at dusk, which is\
                 why the sky reads red, and why the sea reads grey.",
                &f,
                300.0,
            );
        }
        scene.transcript.finish_answer(&f);
        scene.viewport = Viewport::new(8 * f.row_h);
        let (w, h) = (500u32, height_for(8 * f.row_h, &f, 1));
        let (bw, bh) = frame(w, h);
        let strip = paint_with(&scene, w, h, None, 1.0, None);
        let burst = paint_with(&scene, w, h, None, 1.0, Some(2.5));
        // Nothing is drawn for a burst of one step: the strip alone, byte for byte.
        assert_eq!(
            strip,
            paint_with(&scene, w, h, None, 1.0, Some(1.0)),
            "one step is one step"
        );
        // The strip's own geometry is untouched by the number: the same three columns,
        // over the same rows.
        let accent = |buf: &Vec<u8>| -> Vec<(u32, u32)> {
            (0..bw)
                .flat_map(|x| (0..bh).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    buf[((y * bw + x) * 4) as usize..][..3] == render::LABEL_ACCENT[..3]
                })
                .collect()
        };
        let columns = |v: &Vec<(u32, u32)>| -> Vec<u32> {
            let mut xs: Vec<u32> = v.iter().map(|p| p.0).collect();
            xs.sort_unstable();
            xs.dedup();
            xs
        };
        let rows = |v: &Vec<(u32, u32)>| -> (u32, u32) {
            (
                v.iter().map(|p| p.1).min().expect("ink"),
                v.iter().map(|p| p.1).max().expect("ink"),
            )
        };
        assert_eq!(columns(&accent(&strip)).len(), 3, "a strip is 3 px wide");
        assert_eq!(
            columns(&accent(&burst)),
            columns(&accent(&strip)),
            "and a burst does not widen it"
        );
        assert_eq!(
            rows(&accent(&burst)),
            rows(&accent(&strip)),
            "nor lengthen it"
        );
        // What the burst adds is the number: pixels that were not there before, inside
        // the card's rows, one small row tall rather than a line of the answer.
        let changed: Vec<(u32, u32)> = (0..bw)
            .flat_map(|x| (0..bh).map(move |y| (x, y)))
            .filter(|&(x, y)| {
                strip[((y * bw + x) * 4) as usize..][..4]
                    != burst[((y * bw + x) * 4) as usize..][..4]
            })
            .collect();
        assert!(!changed.is_empty(), "the multiplier was not drawn");
        // What the burst added that is the number: pixels that changed where the strip
        // had no ink. The strip's own rows inside the gap changed too — they are gone —
        // so those are not the number's.
        let glyphs: Vec<(u32, u32)> = changed
            .iter()
            .copied()
            .filter(|&(x, y)| {
                strip[((y * bw + x) * 4) as usize..][..3] != render::LABEL_ACCENT[..3]
            })
            .collect();
        assert!(!glyphs.is_empty(), "the number was not drawn");
        let (x0, x1) = (
            glyphs.iter().map(|p| p.0).min().expect("ink"),
            glyphs.iter().map(|p| p.0).max().expect("ink"),
        );
        let (y0, y1) = (
            glyphs.iter().map(|p| p.1).min().expect("ink"),
            glyphs.iter().map(|p| p.1).max().expect("ink"),
        );
        let card_right = bw - render::SHADOW;
        assert!(x1 < card_right, "the number ran past the card's right edge");
        let (track_y, track_h) = scroll_track(bh, f.row_h, 1).expect("a replies card");
        assert!(
            y0 >= track_y && y1 < track_y + track_h,
            "the number left the card's rows"
        );
        assert!(
            y1 - y0 + 1 < f.row_h,
            "the number is {} px tall against a {} px row of the answer",
            y1 - y0 + 1,
            f.row_h
        );
        // And it is the small face, not the answer's at a lower place: its ink is no
        // taller than one line of that smaller face.
        assert!(
            y1 - y0 < small().line_h as u32,
            "the number is {} px of ink against a {} px line at its own size",
            y1 - y0 + 1,
            small().line_h as u32
        );
        // The strip is interrupted where the number is: not one of its pixels sits in
        // the number's rows, and it is still there above and below them.
        let (strip_y0, strip_y1) = rows(&accent(&burst));
        for y in y0..=y1 {
            for x in 0..bw {
                assert!(
                    burst[((y * bw + x) * 4) as usize..][..3] != render::LABEL_ACCENT[..3],
                    "the strip was drawn through the number at ({x}, {y})"
                );
            }
        }
        assert!(strip_y0 < y0, "no strip above the number");
        assert!(strip_y1 > y1, "no strip below it");
        // Centred on the strip's own column: its width straddles the three columns.
        let strip_x = &columns(&accent(&strip))[..];
        assert!(
            x0 < strip_x[0] && x1 > strip_x[2],
            "the number is centred on the strip, not pushed to one side"
        );
        // And it is measured, not assumed: `2.5x` needs more room than `4x`.
        let whole = paint_with(&scene, w, h, None, 1.0, Some(4.0));
        let wide = |buf: &Vec<u8>| -> u32 {
            let xs: Vec<u32> = (0..bw)
                .flat_map(|x| (0..bh).map(move |y| (x, y)))
                .filter(|&(x, y)| {
                    strip[((y * bw + x) * 4) as usize..][..4]
                        != buf[((y * bw + x) * 4) as usize..][..4]
                })
                .map(|p| p.0)
                .collect();
            xs.iter().max().expect("ink") - xs.iter().min().expect("ink") + 1
        };
        assert!(
            x1 - x0 + 1 > wide(&whole) + 4,
            "\"2.5x\" ({} px) is wider than \"4x\" ({} px): the label is measured",
            x1 - x0 + 1,
            wide(&whole)
        );
    }

    /// The strip is cut around the number rather than through it, and a fragment too
    /// short to read as a bar is dropped: the number's own place says where the view is.
    #[test]
    fn the_strip_leaves_the_numbers_rows_empty() {
        assert_eq!(strip_segments(100, 40, None), vec![(100, 140)], "no number");
        assert_eq!(
            strip_segments(100, 40, Some((112, 130))),
            vec![(100, 112), (130, 140)],
            "a number in the middle"
        );
        assert_eq!(
            strip_segments(100, 18, Some((98, 120))),
            vec![],
            "a number taller than the whole strip"
        );
        assert_eq!(
            strip_segments(100, 40, Some((90, 110))),
            vec![(110, 140)],
            "a number at the top"
        );
        assert_eq!(
            strip_segments(100, 40, Some((130, 200))),
            vec![(100, 130)],
            "a number at the bottom"
        );
        // A sliver under two pixels reads as dirt, not as a bar.
        assert_eq!(strip_segments(100, 40, Some((101, 110))), vec![(110, 140)]);
        assert_eq!(strip_segments(100, 40, Some((130, 139))), vec![(100, 130)]);
    }

    /// The number the strip carries: the multiplier, with one decimal only when it is not
    /// whole, so the halves of the burst's ramp read as halves.
    #[test]
    fn the_strip_says_the_multiplier() {
        assert_eq!(burst_label(1.0), "1x");
        assert_eq!(burst_label(1.5), "1.5x");
        assert_eq!(burst_label(2.0), "2x");
        assert_eq!(burst_label(2.5), "2.5x");
        assert_eq!(burst_label(3.5), "3.5x");
        assert_eq!(burst_label(4.0), "4x");
    }

    /// The working light runs the bar's rim: the lit stretch of the edge is the one
    /// the head has just passed, and the bar's own text is still drawn under it.
    /// The replies dissolve into the card's bottom edge: a row that fits is whole,
    /// a row partly under the edge keeps only the part above the ramp, and the ramp
    /// never dips on its way up.
    #[test]
    fn the_replies_dissolve_into_the_cards_bottom_edge() {
        let (bottom, ramp) = (200u32, 30u32);
        assert_eq!(edge_fade(bottom, bottom, ramp), 0.0, "nothing at the edge");
        assert_eq!(edge_fade(bottom + 10, bottom, ramp), 0.0, "nor below it");
        assert_eq!(
            edge_fade(bottom - ramp, bottom, ramp),
            1.0,
            "whole at the top of the ramp"
        );
        assert_eq!(edge_fade(0, bottom, ramp), 1.0, "and everywhere above it");
        let half = edge_fade(bottom - ramp / 2, bottom, ramp);
        assert!((half - 0.5).abs() < 1e-6, "half way into the ramp: {half}");
        let mut last = 0.0;
        for y in (bottom - 2 * ramp..=bottom + 5).rev() {
            let at = edge_fade(y, bottom, ramp);
            assert!(at >= last, "the ramp dips at y {y}");
            last = at;
        }
    }

    /// The dissolve is wired, not only computed: the same row, one card taller, keeps
    /// half its ink, and its pixels are where they were.
    #[test]
    fn a_row_at_the_cards_edge_is_drawn_dimmed() {
        let f = font();
        let mut scene = scene(&f, 300.0);
        scene.input = Input::default();
        // A new turn, so the long answer is the one on screen.
        scene.transcript.ask("and at dusk?", &f);
        scene.transcript.delta(
            " The same scattering leaves only the long wavelengths at dusk, which is\
             why the sky reads red, and why the sea reads grey, and why a fire reads\
             orange from a mile away, and why the sun reads white at noon and\
             red as it sets.",
            &f,
            300.0,
        );
        let blocks = Blocks::new(&f);
        let rows = scene.transcript.rows(blocks);
        assert!(
            rows.len() >= 3,
            "the answer wrapped into {} rows",
            rows.len()
        );
        // The third row, and a card that ends half a row under its top in one frame
        // and a whole row under it in the other: the ramp is the only difference.
        let y = rows[2].y;
        scene.viewport = Viewport::new(3 * f.row_h);
        let (w, _) = sized(&scene, &f);
        let ramped = paint(&scene, w, height_for(y + f.row_h / 2, &f, 1));
        let whole = paint(&scene, w, height_for(y + f.row_h, &f, 1));
        let (bw, _) = frame(w, y + f.row_h);
        // The part of that row both frames hold: its first ten pixels.
        let glass = render::premul(render::BG_PROMPT);
        let ink = |buf: &Vec<u8>| -> u64 {
            (crate::PAD..w - crate::PAD)
                .flat_map(|x| (y..y + 10).map(move |y| (x, y)))
                .map(|(x, y)| {
                    let px = &buf[at(bw, x, y)..][..4];
                    (0..3)
                        .map(|c| (px[c] as i32 - glass[c] as i32).unsigned_abs() as u64)
                        .sum::<u64>()
                })
                .sum()
        };
        let (dimmed, full) = (ink(&ramped), ink(&whole));
        assert!(full > 0, "the row painted nothing at all");
        assert!(dimmed > 0, "the row at the edge is gone, not dimmed");
        let ratio = dimmed as f32 / full as f32;
        assert!(
            (ratio - 0.5).abs() < 0.1,
            "the row at the edge keeps {ratio} of its ink"
        );
    }

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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::settled(State::Bar),
                head,
                0.0,
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
                None,
                &scene.viewport,
                None,
                None,
                None,
                true,
                "",
                "",
                "",
                Look::settled(State::Replies),
                None,
                0.0,
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
