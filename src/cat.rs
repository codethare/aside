//! The cat: where its window lands, where it comes from, what pose it is in, and how it
//! is drawn. Nothing here touches a surface, so the seams it computes are testable
//! without a compositor.

// The stage's surface is what calls all of this; it lands in a later task.
#![cfg_attr(not(test), allow(dead_code, reason = "wired to the stage's surface"))]

use std::time::Duration;

use crate::render;
use crate::render::Bgra;

/// Which screen edge the cat comes out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge {
    Top,
    Bottom,
    Left,
    Right,
}

/// The panel's surface on the output, in logical px, with its input card's height: what
/// the stage is placed against.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panel {
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub bar_h: u32,
}

/// The cat's window: a square of logical px at a place on the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Stage {
    pub x: i32,
    pub y: i32,
    pub size: u32,
}

/// The stage's side at `size` 1. It shrinks to the room the panel leaves, and stops being
/// a cat under `STAGE_FLOOR`: a cat squeezed into 40 px is worse than no cat.
pub const STAGE: f32 = 260.0;
pub const STAGE_FLOOR: u32 = 180;

/// The input card's left edge on the output: the panel's surface plus its shadow margin.
fn card_left(panel: Panel) -> i32 {
    panel.x + render::SHADOW as i32
}

/// The input card's vertical centre, which is where the paw and the stage's centre go.
fn bar_centre(panel: Panel) -> i32 {
    panel.y + panel.h as i32 - render::SHADOW as i32 - panel.bar_h as i32 / 2
}

/// The stage's side: what the size asks for, or the room the panel leaves, whichever is
/// smaller, and nothing at all under the floor.
pub fn stage_size(size: f32, room: u32) -> Option<u32> {
    let side = ((STAGE * size).round().max(0.0) as u32).min(room);
    (side >= STAGE_FLOOR).then_some(side)
}

/// Where the cat settles. The right edge on the input card's left edge and the centre on
/// the card's centre: that one identity puts the paw's tip on the card's left rim and the
/// reply card's bottom-left corner inside the stage.
pub fn landing(panel: Panel, size: f32) -> Option<Stage> {
    let side = stage_size(size, card_left(panel).max(0) as u32)?;
    Some(Stage {
        x: card_left(panel) - side as i32,
        y: bar_centre(panel) - side as i32 / 2,
        size: side,
    })
}

/// The window at `edge`, flush with it and centred on the input card's axis. The pose is
/// drawn cropped by the window's edge, so the cat is cut exactly by the screen.
pub fn peek(edge: Edge, output: (u32, u32), panel: Panel, size: f32) -> Option<Stage> {
    let side = landing(panel, size)?.size;
    let s = side as i32;
    let (x, y) = match edge {
        Edge::Bottom => (panel.x + panel.w as i32 / 2 - s / 2, output.1 as i32 - s),
        Edge::Top => (panel.x + panel.w as i32 / 2 - s / 2, 0),
        Edge::Left => (0, bar_centre(panel) - s / 2),
        Edge::Right => (output.0 as i32 - s, bar_centre(panel) - s / 2),
    };
    Some(Stage { x, y, size: side })
}

/// The window part way through the hop: the straight line from `from` to `to`, with the
/// middle lifted out of it, so the cat arcs rather than sliding.
pub fn hop(from: Stage, to: Stage, t: f32) -> Stage {
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: i32, b: i32| (a as f32 + (b - a) as f32 * t).round() as i32;
    let lift = (std::f32::consts::PI * t).sin() * 0.35 * from.size as f32;
    Stage {
        x: lerp(from.x, to.x),
        y: lerp(from.y, to.y) - lift.round() as i32,
        size: from.size,
    }
}

/// What the panel is doing, as far as the cat is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mood {
    Idle,
    Thinking,
    Answering,
}

/// What the cat is doing, which is all the painter needs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pose {
    Peek,
    Hop,
    Arm,
    Waiting,
    Lick,
    Speak,
    Extend,
}

/// The entrance, in the order it is drawn: the head over the edge, the hop, the paw.
pub const PEEK_MS: u128 = 420;
pub const HOP_MS: u128 = 400;
pub const ARM_MS: u128 = 300;
/// When the entrance is over and the cat is waiting with its paw out.
pub const ENTER_MS: u128 = PEEK_MS + HOP_MS + ARM_MS;
/// How long a request runs before the cat licks the paw it just extended.
pub const LICK_AFTER_MS: u128 = 300;
/// The mouth's one open and close, at the first text.
pub const SPEAK_MS: u128 = 200;
/// The paw's second reach, when the answer ends.
pub const EXTEND_MS: u128 = 260;

/// What the cat is doing now. `entrance` is the time since its first frame, `since_mood`
/// the time since the panel entered that mood, and `since_finish` the time since the
/// answer ended.
pub fn pose(
    entrance: Duration,
    mood: Mood,
    since_mood: Duration,
    since_finish: Option<Duration>,
    animate: bool,
) -> Pose {
    let ms = |d: Duration| d.as_millis();
    // Sending the question ends the entrance where it is: a cat in mid-air has no mouth
    // to hang a bubble from, and a second clock for a one-second case is not worth it.
    if !animate || mood != Mood::Idle {
        return match mood {
            Mood::Idle => Pose::Waiting,
            Mood::Thinking if ms(since_mood) >= LICK_AFTER_MS => Pose::Lick,
            Mood::Thinking => Pose::Waiting,
            Mood::Answering if since_finish.is_some_and(|d| ms(d) < EXTEND_MS) => Pose::Extend,
            Mood::Answering if ms(since_mood) < SPEAK_MS => Pose::Speak,
            Mood::Answering => Pose::Waiting,
        };
    }
    let t = ms(entrance);
    if t < PEEK_MS {
        Pose::Peek
    } else if t < PEEK_MS + HOP_MS {
        Pose::Hop
    } else if t < ENTER_MS {
        Pose::Arm
    } else {
        Pose::Waiting
    }
}

/// The clock and the process: no dependency, and a summon is a fresh roll because the
/// process exits with the panel.
pub fn seed() -> u64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    nanos ^ (std::process::id() as u64).rotate_left(32)
}

/// One of four, from a seed. A splitmix step, so neighbouring seeds — the clock's
/// nanoseconds — do not walk the edges in order.
pub fn roll_edge(seed: u64) -> Edge {
    let mut x = seed.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^= x >> 31;
    match x % 4 {
        0 => Edge::Bottom,
        1 => Edge::Top,
        2 => Edge::Left,
        _ => Edge::Right,
    }
}

// ---- the drawing ----

/// The cat's inks: a near-black body, a rim that is the panel's own ink dimmed, and the
/// three things that are not the silhouette — the eyes, the mouth and the tongue.
const BODY: Bgra = render::bgra(0x0e, 0x0e, 0x13, 0xff);
const RIM: Bgra = render::bgra(0xa8, 0xa8, 0xc0, 0xc0);
const EYE: Bgra = render::bgra(0xf4, 0xf4, 0xff, 0xff);
const TONGUE: Bgra = render::bgra(0xd0, 0x6a, 0x84, 0xff);
const MOUTH: Bgra = render::bgra(0x3a, 0x16, 0x20, 0xff);

/// Where the paw meets the pad: the stage's right edge at its vertical centre, which is
/// the input card's left rim at that card's vertical centre.
pub fn paw(w: u32, h: u32) -> (u32, u32) {
    (w, h / 2)
}

/// Where the bubble's tail starts: the reply card's bottom-left corner, on the stage's
/// right edge, `bar_h / 2 + SEPARATION` above the stage's centre.
pub fn tail_start(w: u32, h: u32, s: u32, bar_h: u32) -> (u32, u32) {
    let gap = (bar_h as f32 / 2.0 + render::SEPARATION as f32) * s as f32;
    (w, (h as f32 / 2.0 - gap).max(0.0).round() as u32)
}

/// The head's centre and radius, which the ears, the eyes, the mouth and the tongue are
/// all placed from. One head, so a pose that moves it moves all of them.
fn head(w: f32, h: f32) -> (f32, f32, f32) {
    (0.63 * w, 0.40 * h, 0.155 * w)
}

/// The mouth: the head's front, low on its face, where a cat's is.
fn mouth(w: f32, h: f32) -> (f32, f32) {
    let (hx, hy, r) = head(w, h);
    (hx + 0.66 * r, hy + 0.62 * r)
}

/// What one frame of the cat is doing, as numbers the shapes are placed from.
#[derive(Debug, Clone, Copy)]
struct Act {
    /// How far the arm has reached: 0 at the shoulder, 1 on the pad.
    reach: f32,
    /// How far the head has dipped toward the arm, in stage heights.
    dip: f32,
    /// How open the mouth is.
    mouth: f32,
    /// How far the tongue is out.
    tongue: f32,
    /// Whether the tail is up, as it is mid-leap.
    tail_up: bool,
    /// The body's stretch along its length, 1 for the resting cat.
    stretch: f32,
}

impl Act {
    const REST: Act = Act {
        reach: 1.0,
        dip: 0.0,
        mouth: 0.0,
        tongue: 0.0,
        tail_up: false,
        stretch: 1.0,
    };
}

/// The stage's buffer as a place to put filled shapes. Every shape carries its own edge
/// coverage: a cat of hard-edged discs reads as a robot.
struct Ink<'a> {
    buf: &'a mut [u8],
    w: u32,
    h: u32,
}

impl Ink<'_> {
    fn px(&mut self, x: i32, y: i32, c: Bgra, cov: f32) {
        if x < 0 || y < 0 || x >= self.w as i32 || y >= self.h as i32 {
            return;
        }
        let cov = (cov.clamp(0.0, 1.0) * 255.0).round() as u32;
        if cov == 0 {
            return;
        }
        let i = (y as u32 * self.w + x as u32) as usize * 4;
        render::over_pm(&mut self.buf[i..][..4], render::premul(c), cov);
    }

    /// Every pixel of a shape's bounding box, with the coverage `f` gives it.
    fn fill(&mut self, x0: f32, x1: f32, y0: f32, y1: f32, c: Bgra, f: impl Fn(f32, f32) -> f32) {
        for y in (y0 - 1.0).floor() as i32..=(y1 + 1.0).ceil() as i32 {
            for x in (x0 - 1.0).floor() as i32..=(x1 + 1.0).ceil() as i32 {
                let cov = f(x as f32 + 0.5, y as f32 + 0.5);
                self.px(x, y, c, cov);
            }
        }
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Bgra) {
        self.fill(cx - r, cx + r, cy - r, cy + r, c, |x, y| {
            r + 0.5 - ((x - cx).powi(2) + (y - cy).powi(2)).sqrt()
        });
    }

    fn ellipse(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, c: Bgra) {
        let (rx, ry) = (rx.max(0.5), ry.max(0.5));
        let edge = (rx + ry) / 2.0;
        self.fill(cx - rx, cx + rx, cy - ry, cy + ry, c, |x, y| {
            let d = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
            (1.0 - d) * edge + 0.5
        });
    }

    /// A thick line with round ends: the arm, the tail, the tongue.
    fn capsule(&mut self, a: (f32, f32), b: (f32, f32), r: f32, c: Bgra) {
        let (vx, vy) = (b.0 - a.0, b.1 - a.1);
        let len2 = (vx * vx + vy * vy).max(1e-6);
        self.fill(
            a.0.min(b.0) - r,
            a.0.max(b.0) + r,
            a.1.min(b.1) - r,
            a.1.max(b.1) + r,
            c,
            |x, y| {
                let t = (((x - a.0) * vx + (y - a.1) * vy) / len2).clamp(0.0, 1.0);
                let (dx, dy) = (x - a.0 - vx * t, y - a.1 - vy * t);
                r + 0.5 - (dx * dx + dy * dy).sqrt()
            },
        );
    }

    /// A filled triangle, four samples to the pixel: the ears and the bubble's tail.
    fn triangle(&mut self, p: [(f32, f32); 3], c: Bgra) {
        let x0 = p.iter().map(|q| q.0).fold(f32::MAX, f32::min);
        let x1 = p.iter().map(|q| q.0).fold(f32::MIN, f32::max);
        let y0 = p.iter().map(|q| q.1).fold(f32::MAX, f32::min);
        let y1 = p.iter().map(|q| q.1).fold(f32::MIN, f32::max);
        let inside = |x: f32, y: f32| {
            let side =
                |a: (f32, f32), b: (f32, f32)| (b.0 - a.0) * (y - a.1) - (b.1 - a.1) * (x - a.0);
            let (s0, s1, s2) = (side(p[0], p[1]), side(p[1], p[2]), side(p[2], p[0]));
            (s0 >= 0.0 && s1 >= 0.0 && s2 >= 0.0) || (s0 <= 0.0 && s1 <= 0.0 && s2 <= 0.0)
        };
        self.fill(x0, x1, y0, y1, c, |x, y| {
            let mut hits = 0.0;
            for (dx, dy) in [(-0.25, -0.25), (0.25, -0.25), (-0.25, 0.25), (0.25, 0.25)] {
                if inside(x + dx, y + dy) {
                    hits += 0.25;
                }
            }
            hits
        });
    }
}

/// One frame of the cat, in stage-local buffer px. `s` is the buffer scale, `bar_h` the
/// input card's height in logical px, `edge` the edge the cat came out of, and `since`
/// the time since its pose began.
#[allow(clippy::too_many_arguments)] // a window, a pose and the two numbers it is placed by
pub fn draw(
    buf: &mut [u8],
    w: u32,
    h: u32,
    s: u32,
    bar_h: u32,
    edge: Edge,
    pose: Pose,
    since: Duration,
    bubble: bool,
) {
    let (fw, fh) = (w as f32, h as f32);
    let t = since.as_millis() as f32;
    let mut ink = Ink { buf, w, h };
    // The bubble's tail goes under the cat: the head covers its point, so it reads as
    // coming out of the mouth rather than stopping at it.
    if bubble {
        bubble_tail(&mut ink, fw, fh, s, bar_h);
    }
    let act = match pose {
        Pose::Peek => {
            peek_pose(&mut ink, fw, fh, edge, t / PEEK_MS as f32);
            return;
        }
        Pose::Hop => Act {
            reach: 0.0,
            tail_up: true,
            stretch: 1.08,
            dip: -0.02,
            ..Act::REST
        },
        Pose::Arm => Act {
            reach: ease_out(t / ARM_MS as f32),
            ..Act::REST
        },
        Pose::Waiting => Act::REST,
        Pose::Lick => {
            // Two flicks a second, the tongue on the paw it just extended.
            let phase = (t / 450.0) % 1.0;
            Act {
                dip: 0.03 * (phase * std::f32::consts::PI).sin().abs(),
                tongue: (phase * std::f32::consts::PI).sin().max(0.0),
                ..Act::REST
            }
        }
        Pose::Speak => Act {
            mouth: (t / SPEAK_MS as f32 * std::f32::consts::PI).sin().max(0.0),
            ..Act::REST
        },
        Pose::Extend => Act {
            reach: 1.0 - 0.35 * (t / EXTEND_MS as f32 * std::f32::consts::PI).sin(),
            ..Act::REST
        },
    };
    cat(&mut ink, fw, fh, act);
}

/// The cat in one act: the rim pass first, offset up and left, then the body over it, so
/// the rim survives as the one-pixel light along the edges a light would catch.
fn cat(ink: &mut Ink, w: f32, h: f32, act: Act) {
    for (offset, colour) in [((-1.1, -1.1), RIM), ((0.0, 0.0), BODY)] {
        silhouette(ink, w, h, act, offset, colour);
    }
    details(ink, w, h, act);
}

fn silhouette(ink: &mut Ink, w: f32, h: f32, act: Act, o: (f32, f32), c: Bgra) {
    let (ox, oy) = o;
    // The body: a loaf, wider than tall, with the head sitting on its front.
    ink.ellipse(
        0.36 * w + ox,
        0.68 * h + oy,
        0.28 * w * act.stretch,
        0.205 * h,
        c,
    );
    let (hx, hy, r) = head(w, h);
    let hy = hy + act.dip * h;
    ink.disc(hx + ox, hy + oy, r, c);
    // The ears: two triangles on the crown, tilted out.
    ink.triangle(
        [
            (hx - 0.92 * r + ox, hy - 0.52 * r + oy),
            (hx - 0.66 * r + ox, hy - 1.34 * r + oy),
            (hx - 0.24 * r + ox, hy - 0.70 * r + oy),
        ],
        c,
    );
    ink.triangle(
        [
            (hx + 0.24 * r + ox, hy - 0.70 * r + oy),
            (hx + 0.66 * r + ox, hy - 1.34 * r + oy),
            (hx + 0.92 * r + ox, hy - 0.52 * r + oy),
        ],
        c,
    );
    // The tail: out of the back, down and round, or straight up mid-leap.
    let root = (0.16 * w + ox, 0.72 * h + oy);
    if act.tail_up {
        ink.capsule(root, (0.10 * w + ox, 0.30 * h + oy), 0.040 * w, c);
    } else {
        let bend = (0.04 * w + ox, 0.62 * h + oy);
        ink.capsule(root, bend, 0.042 * w, c);
        ink.capsule(bend, (0.12 * w + ox, 0.48 * h + oy), 0.036 * w, c);
    }
    // The arm: shoulder to wherever the reach has got to, the paw ending on the pad.
    let (px, py) = paw(w as u32, h as u32);
    let shoulder = (0.54 * w + ox, 0.74 * h + oy);
    let tip = (
        shoulder.0 + (px as f32 - 0.05 * w - shoulder.0) * act.reach,
        shoulder.1 + (py as f32 - shoulder.1) * act.reach,
    );
    ink.capsule(shoulder, tip, 0.032 * w, c);
    ink.disc(tip.0, tip.1, 0.048 * w, c);
}

fn details(ink: &mut Ink, w: f32, h: f32, act: Act) {
    let (hx, hy, r) = head(w, h);
    let hy = hy + act.dip * h;
    // The eyes: two bright dots, which is the whole face a silhouette can carry.
    ink.disc(hx - 0.42 * r, hy - 0.08 * r, 0.115 * r, EYE);
    ink.disc(hx + 0.42 * r, hy - 0.08 * r, 0.115 * r, EYE);
    let (mx, my) = mouth(w, h);
    let my = my + act.dip * h;
    if act.mouth > 0.01 {
        ink.ellipse(
            mx,
            my + 0.18 * r,
            0.22 * r + 0.18 * r * act.mouth,
            0.07 * r + 0.30 * r * act.mouth,
            MOUTH,
        );
        // The tongue inside the open mouth, which is what makes it a mouth and not a hole.
        ink.ellipse(
            mx,
            my + 0.18 * r + 0.22 * r * act.mouth,
            0.14 * r * act.mouth,
            0.08 * r * act.mouth,
            TONGUE,
        );
    }
    if act.tongue > 0.01 {
        // Down onto the arm it just extended, which passes under the head.
        let tip = (mx + 0.12 * r, my + (0.30 + 0.60 * act.tongue) * r);
        ink.capsule((mx, my), tip, 0.085 * r, TONGUE);
    }
}

/// The head over the edge, and one paw with it: the cat has not arrived yet.
fn peek_pose(ink: &mut Ink, w: f32, h: f32, edge: Edge, t: f32) {
    let rise = ease_out(t.clamp(0.0, 1.0));
    let (hx, _, r) = head(w, h);
    let out = 0.20 * w;
    let (from, to) = match edge {
        Edge::Bottom => ((hx, h + r + out), (hx, 0.72 * h)),
        Edge::Top => ((hx, -r - out), (hx, 0.28 * h)),
        Edge::Left => ((-r - out, 0.5 * h), (0.30 * w, 0.5 * h)),
        Edge::Right => ((w + r + out, 0.5 * h), (0.70 * w, 0.5 * h)),
    };
    let (cx, cy) = (
        from.0 + (to.0 - from.0) * rise,
        from.1 + (to.1 - from.1) * rise,
    );
    for (offset, colour) in [((-1.1, -1.1), RIM), ((0.0, 0.0), BODY)] {
        let (ox, oy) = offset;
        ink.disc(cx + ox, cy + oy, r, colour);
        ink.triangle(
            [
                (cx - 0.95 * r + ox, cy - 0.50 * r + oy),
                (cx - 0.62 * r + ox, cy - 1.55 * r + oy),
                (cx - 0.18 * r + ox, cy - 0.66 * r + oy),
            ],
            colour,
        );
        ink.triangle(
            [
                (cx + 0.18 * r + ox, cy - 0.66 * r + oy),
                (cx + 0.62 * r + ox, cy - 1.55 * r + oy),
                (cx + 0.95 * r + ox, cy - 0.50 * r + oy),
            ],
            colour,
        );
        // The paw on the edge, reaching over it.
        let (ax, ay) = match edge {
            Edge::Bottom => (cx, cy + 0.85 * r),
            Edge::Top => (cx, cy - 0.85 * r),
            Edge::Left => (cx - 0.85 * r, cy),
            Edge::Right => (cx + 0.85 * r, cy),
        };
        ink.disc(ax + ox, ay + oy, 0.30 * r, colour);
    }
    ink.disc(cx - 0.42 * r, cy - 0.08 * r, 0.115 * r, EYE);
    ink.disc(cx + 0.42 * r, cy - 0.08 * r, 0.115 * r, EYE);
}

/// The bubble's tail: the card's corner to the mouth, in the card's own fill.
fn bubble_tail(ink: &mut Ink, w: f32, h: f32, s: u32, bar_h: u32) {
    let (cx, cy) = tail_start(w as u32, h as u32, s, bar_h);
    let (cx, cy) = (cx as f32, cy as f32);
    let (mx, my) = mouth(w, h);
    ink.triangle([(cx, cy), (cx, cy + 0.10 * w), (mx, my)], render::BG_PROMPT);
}

/// A shape's progress as the eye expects it: fast out of the gate, settling at the end.
fn ease_out(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn panel() -> Panel {
        Panel {
            x: 614,
            y: 400,
            w: 692,
            h: 300,
            bar_h: 30,
        }
    }

    #[test]
    fn the_stage_lands_with_its_right_edge_on_the_input_card() {
        let panel = panel();
        let stage = landing(panel, 1.0).expect("room to the left");
        assert_eq!(stage.size, 260);
        assert_eq!(
            stage.x + stage.size as i32,
            panel.x + render::SHADOW as i32,
            "the right edge is the card's left edge"
        );
        let bar_centre = panel.y + panel.h as i32 - render::SHADOW as i32 - panel.bar_h as i32 / 2;
        assert_eq!(stage.y + stage.size as i32 / 2, bar_centre);
    }

    #[test]
    fn no_room_means_no_cat() {
        // 160 px to the left of the card is under the 180 floor.
        let panel = Panel {
            x: 134,
            y: 400,
            w: 692,
            h: 300,
            bar_h: 30,
        };
        assert_eq!(landing(panel, 2.0), None);
    }

    #[test]
    fn the_stage_shrinks_to_the_room_before_it_disappears() {
        assert_eq!(stage_size(2.0, 320), Some(320));
        assert_eq!(stage_size(2.0, 179), None);
    }

    #[test]
    fn every_edge_is_rolled_and_each_peek_is_flush_with_its_edge() {
        let seen: std::collections::BTreeSet<_> = (0..64).map(roll_edge).collect();
        assert_eq!(seen.len(), 4, "{seen:?}");
        let panel = panel();
        let out = (1920, 1080);
        // Centred on the input card's axis: x 614 + 346 - 130, y 659 - 130.
        for (edge, want) in [
            (Edge::Bottom, (830, 1080 - 260)),
            (Edge::Top, (830, 0)),
            (Edge::Left, (0, 659 - 130)),
            (Edge::Right, (1920 - 260, 659 - 130)),
        ] {
            let stage = peek(edge, out, panel, 1.0).expect("room");
            assert_eq!((stage.x, stage.y), want, "{edge:?}");
        }
    }

    #[test]
    fn the_entrance_runs_in_order_and_stops_when_the_question_goes() {
        let ms = Duration::from_millis;
        assert_eq!(pose(ms(0), Mood::Idle, ms(0), None, true), Pose::Peek);
        assert_eq!(pose(ms(500), Mood::Idle, ms(0), None, true), Pose::Hop);
        assert_eq!(pose(ms(900), Mood::Idle, ms(0), None, true), Pose::Arm);
        assert_eq!(pose(ms(1200), Mood::Idle, ms(0), None, true), Pose::Waiting);
        // Sent mid-entrance: the cat is where it lands, at once.
        assert_eq!(
            pose(ms(500), Mood::Thinking, ms(0), None, true),
            Pose::Waiting
        );
    }

    #[test]
    fn the_lick_belongs_to_thinking_and_stops_at_the_first_text() {
        let ms = Duration::from_millis;
        assert_eq!(
            pose(ms(2000), Mood::Thinking, ms(100), None, true),
            Pose::Waiting
        );
        assert_eq!(
            pose(ms(2000), Mood::Thinking, ms(400), None, true),
            Pose::Lick
        );
        assert_eq!(
            pose(ms(2000), Mood::Answering, ms(0), None, true),
            Pose::Speak
        );
        assert_eq!(
            pose(ms(2000), Mood::Answering, ms(400), None, true),
            Pose::Waiting
        );
    }

    #[test]
    fn the_paw_comes_back_after_the_answer() {
        let ms = Duration::from_millis;
        assert_eq!(
            pose(ms(2000), Mood::Answering, ms(400), Some(ms(100)), true),
            Pose::Extend
        );
        assert_eq!(
            pose(ms(2000), Mood::Answering, ms(400), Some(ms(400)), true),
            Pose::Waiting
        );
    }

    #[test]
    fn animation_off_is_the_landed_cat_in_the_first_frame() {
        assert_eq!(
            pose(Duration::ZERO, Mood::Idle, Duration::ZERO, None, false),
            Pose::Waiting
        );
    }

    #[test]
    fn the_paw_and_the_bubble_tail_land_on_the_contacts() {
        let (w, h, bar_h) = (260u32, 260u32, 30u32);
        // The identity holds at every buffer scale: the seam is a point, not a range.
        for s in [1u32, 2, 3] {
            assert_eq!(paw(w * s, h * s), (w * s, h * s / 2));
            let corner = tail_start(w * s, h * s, s, bar_h);
            assert_eq!(corner.0, w * s);
            assert!(corner.1 > 0 && corner.1 < h * s / 2, "{corner:?} at s {s}");
        }
    }

    #[test]
    fn the_cat_never_paints_outside_its_window() {
        let (w, h) = (260u32, 260u32);
        for pose in [
            Pose::Peek,
            Pose::Hop,
            Pose::Arm,
            Pose::Waiting,
            Pose::Lick,
            Pose::Speak,
            Pose::Extend,
        ] {
            let mut buf = vec![0u8; (w * h * 4) as usize];
            draw(
                &mut buf,
                w,
                h,
                1,
                30,
                Edge::Bottom,
                pose,
                Duration::from_millis(100),
                false,
            );
            // The window's own corners are outside any pose's reach: the buffer is the
            // clip, so a pose that painted past the edge would land here.
            for (x, y) in [(0, 0), (w - 1, 0), (0, h - 1)] {
                let px = &buf[((y * w + x) * 4) as usize..][..4];
                assert_eq!(px[3], 0, "{pose:?} transparent at {x},{y}");
            }
        }
    }

    #[test]
    fn every_pose_paints_something_of_its_own() {
        let (w, h) = (260u32, 260u32);
        let paint = |pose, since| {
            let mut buf = vec![0u8; (w * h * 4) as usize];
            draw(&mut buf, w, h, 1, 30, Edge::Bottom, pose, since, false);
            buf
        };
        let lit = |buf: &[u8]| buf.as_chunks::<4>().0.iter().filter(|p| p[3] > 8).count();
        let ms = Duration::from_millis;
        let waiting = paint(Pose::Waiting, ms(0));
        assert!(lit(&waiting) > 2000, "the cat is drawn: {}", lit(&waiting));
        // The peek is a head over an edge, so it covers less of the window than the cat.
        assert!(lit(&paint(Pose::Peek, ms(380))) < lit(&waiting));
        // The lick's tongue is a pixel class of its own: the frames differ.
        assert_ne!(
            paint(Pose::Lick, ms(0)),
            paint(Pose::Lick, ms(220)),
            "the tongue moves"
        );
        assert_ne!(
            paint(Pose::Speak, ms(0)),
            paint(Pose::Speak, ms(100)),
            "the mouth opens"
        );
    }

    #[test]
    fn the_hop_starts_and_ends_on_its_ends_and_is_in_the_air_between() {
        let a = Stage {
            x: 0,
            y: 820,
            size: 260,
        };
        let b = Stage {
            x: 640,
            y: 529,
            size: 260,
        };
        assert_eq!(hop(a, b, 0.0), a);
        assert_eq!(hop(a, b, 1.0), b);
        let mid = hop(a, b, 0.5);
        assert!(
            mid.y < (a.y + b.y) / 2,
            "the middle of the hop is above the line: {mid:?}"
        );
    }
}
