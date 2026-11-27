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
    let want = ((STAGE * size).round().max(0.0) as u32).min(room);
    // An even side keeps the window's centre on a whole buffer pixel at every integer
    // scale, which is what makes the paw, the pad's rim and the bubble's tail one point
    // rather than three that are a pixel apart.
    let side = want - want % 2;
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

/// The window part way through the hop: the straight line from `from` to `to`, eased at
/// both ends, with the middle lifted out of it, so the cat springs rather than sliding.
pub fn hop(from: Stage, to: Stage, t: f32) -> Stage {
    let t = t.clamp(0.0, 1.0);
    let e = t * t * (3.0 - 2.0 * t);
    let lerp = |a: i32, b: i32| (a as f32 + (b - a) as f32 * e).round() as i32;
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
/// The waiting cat's blink: open at first, then closed for 260 ms every five seconds.
pub const BLINK_CYCLE_MS: u128 = 5_000;
pub const BLINK_MS: u128 = 260;
const BLINK_AT_MS: u128 = 1_200;

/// 1 while the eyes are shut. Two frames per cycle at the idle tick, not an idle loop.
pub fn blink(since: Duration) -> f32 {
    let phase = since.as_millis() % BLINK_CYCLE_MS;
    if (BLINK_AT_MS..BLINK_AT_MS + BLINK_MS).contains(&phase) {
        1.0
    } else {
        0.0
    }
}

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

/// The mascot's materials: flat cream paper inside a warm near-black line, with one
/// coral accent. No gradient, no cast shadow, no underlay.
const INK: Bgra = render::bgra(0x2b, 0x25, 0x20, 0xff);
const PAPER: Bgra = render::bgra(0xf3, 0xec, 0xdf, 0xff);
const EYE: Bgra = render::bgra(0xff, 0xfd, 0xf8, 0xff);
const ACCENT: Bgra = render::bgra(0xef, 0x83, 0x6d, 0xff);
const BLUSH: Bgra = render::bgra(0xef, 0x83, 0x6d, 0x55);

/// The three line roles, as fractions of the stage's side: silhouette, part, detail.
fn weights(w: f32) -> (f32, f32, f32) {
    let role = |k: f32, floor: f32| (w * k).max(floor);
    (role(0.0115, 1.6), role(0.0080, 1.2), role(0.0055, 0.9))
}

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
    (0.65 * w, 0.34 * h, 0.16 * w)
}

/// The mouth: on the head's outline at its lower front, so the bubble's card-coloured
/// tail ends where the cat's line is rather than crossing a transparent head.
fn mouth(w: f32, h: f32) -> (f32, f32) {
    let (hx, hy, r) = head(w, h);
    (hx + 0.906 * r, hy + 0.423 * r)
}

/// The mouth in a square window's own px, for a caller that has to place the bubble's
/// growth origin in another surface's coordinates.
pub fn mouth_at(size: u32) -> (f32, f32) {
    mouth(size as f32, size as f32)
}

/// What one frame of the cat is doing, as numbers the shapes are placed from.
#[derive(Debug, Clone, Copy)]
struct Act {
    /// How far the arm has reached: 0 at the shoulder, 1 on the pad.
    reach: f32,
    /// How far the head has dipped toward the arm, in stage heights.
    dip: f32,
    /// The head's turn toward the panel, radians. The ears turn about its centre; the
    /// face turns about the mouth, so the bubble's point never moves.
    tilt: f32,
    /// How open the mouth is.
    mouth: f32,
    /// How far the tongue is out.
    tongue: f32,
    /// The tongue's sideways wag, -1..1.
    tongue_sway: f32,
    /// Whether the tail is up, as it is mid-leap.
    tail_up: bool,
    /// The tail tip's sideways sway, in stage widths.
    tail_sway: f32,
    /// One ear's extra turn, radians.
    ear: f32,
    /// 1 for closed eyes.
    blink: f32,
    /// The pupils' sideways look, in head radii: positive toward the panel.
    gaze: f32,
}

impl Act {
    const REST: Act = Act {
        reach: 1.0,
        dip: 0.0,
        tilt: 0.10,
        mouth: 0.0,
        tongue: 0.0,
        tongue_sway: 0.0,
        tail_up: false,
        tail_sway: 0.0,
        ear: 0.0,
        blink: 0.0,
        gaze: 0.04,
    };
}

/// The stage's buffer as a place to stroke lines. Every stroke carries its own edge
/// coverage: a line of hard-edged segments reads as a polyline, not a drawing.
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

    /// A Catmull-Rom curve through `p`, stroked segment by segment with round joins, and
    /// a hand-drawn waver that is zero at every control point. No allocation.
    fn curve(&mut self, p: &[(f32, f32)], width: f32, c: Bgra) {
        let amp = self.w as f32 * 0.0035;
        for i in 0..p.len().saturating_sub(1) {
            let p0 = p[i.saturating_sub(1)];
            let p1 = p[i];
            let p2 = p[i + 1];
            let p3 = p.get(i + 2).copied().unwrap_or(p2);
            let (dx, dy) = (p2.0 - p1.0, p2.1 - p1.1);
            let len = (dx * dx + dy * dy).sqrt().max(1e-6);
            let (nx, ny) = (-dy / len, dx / len);
            let mut from = p1;
            for k in 1..=16 {
                let t = k as f32 / 16.0;
                let mut to = catmull(p0, p1, p2, p3, t);
                // The window makes the waver vanish at both ends, so a join stays a point.
                let waver = amp
                    * (std::f32::consts::PI * t).sin()
                    * (i as f32 * 2.399 + t * std::f32::consts::TAU).sin();
                to.0 += nx * waver;
                to.1 += ny * waver;
                self.capsule(from, to, width / 2.0, c);
                from = to;
            }
        }
    }

    /// One-weight rings: the head and the paw.
    fn ring(&mut self, cx: f32, cy: f32, r: f32, width: f32, c: Bgra) {
        let (outer, inner) = (r + width / 2.0, (r - width / 2.0).max(0.0));
        self.fill(cx - outer, cx + outer, cy - outer, cy + outer, c, |x, y| {
            let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
            (outer + 0.5 - d).clamp(0.0, 1.0) * (d - inner + 0.5).clamp(0.0, 1.0)
        });
    }

    /// A flat elliptical ring: the paw's mitten.
    fn ellipse_ring(&mut self, cx: f32, cy: f32, rx: f32, ry: f32, width: f32, c: Bgra) {
        let (rx, ry) = (rx.max(0.5), ry.max(0.5));
        let small = rx.min(ry);
        self.fill(
            cx - rx - width,
            cx + rx + width,
            cy - ry - width,
            cy + ry + width,
            c,
            |x, y| {
                let d = (((x - cx) / rx).powi(2) + ((y - cy) / ry).powi(2)).sqrt();
                // The normalized radius is converted back to px on the nearest axis, so the
                // band is the same weight the rest of the drawing uses.
                (width / 2.0 + 0.5 - (d - 1.0).abs() * small).clamp(0.0, 1.0)
            },
        );
    }

    /// A filled triangle, four samples to the pixel: the nose and the bubble's tail.
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

    /// A closed polygon, scanline-filled: the body. The ink outline covers the hard edge.
    fn polygon(&mut self, p: &[(f32, f32)], c: Bgra) {
        let (mut y0, mut y1) = (f32::MAX, f32::MIN);
        for q in p {
            y0 = y0.min(q.1);
            y1 = y1.max(q.1);
        }
        let mut xs = [0.0f32; 64];
        let first = (y0.ceil().max(0.0)) as i32;
        let last = (y1.floor().min(self.h as f32 - 1.0)) as i32;
        for y in first..=last {
            let yc = y as f32 + 0.5;
            let mut n = 0;
            for i in 0..p.len() {
                let a = p[i];
                let b = p[(i + 1) % p.len()];
                if (a.1 <= yc) != (b.1 <= yc) && n < xs.len() {
                    xs[n] = a.0 + (yc - a.1) * (b.0 - a.0) / (b.1 - a.1);
                    n += 1;
                }
            }
            xs[..n].sort_unstable_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            for pair in xs[..n].as_chunks::<2>().0 {
                let x0 = (pair[0].ceil().max(0.0)) as i32;
                let x1 = (pair[1].floor().min(self.w as f32 - 1.0)) as i32;
                for x in x0..=x1 {
                    self.px(x, y, c, 1.0);
                }
            }
        }
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
    // The bubble's tail goes under the cat: its point ends on the head's line, at the
    // mouth, so it reads as coming out of the mouth rather than stopping beside it.
    if bubble {
        bubble_tail(&mut ink, fw, fh, s, bar_h);
    }
    let act = match pose {
        Pose::Peek => {
            peek_pose(&mut ink, fw, fh, edge, t / PEEK_MS as f32);
            return;
        }
        Pose::Hop => {
            let t = (t / HOP_MS as f32).clamp(0.0, 1.0);
            let arc = (std::f32::consts::PI * t).sin();
            Act {
                reach: 0.0,
                tail_up: true,
                dip: -0.02 + 0.02 * arc,
                tilt: 0.06 + 0.05 * arc,
                tail_sway: 0.005 * (2.2 * std::f32::consts::PI * t).sin(),
                ear: 0.05 * (3.0 * std::f32::consts::PI * t).sin(),
                gaze: 0.02,
                ..Act::REST
            }
        }
        Pose::Arm => {
            let t = t / ARM_MS as f32;
            Act {
                reach: ease_out_back(t),
                tilt: 0.10 + 0.04 * (std::f32::consts::PI * t).sin(),
                tail_sway: 0.006 * (2.0 * std::f32::consts::PI * t).sin(),
                ear: 0.06 * (2.5 * std::f32::consts::PI * t).sin(),
                gaze: 0.08,
                ..Act::REST
            }
        }
        Pose::Waiting => Act {
            blink: blink(since),
            ..Act::REST
        },
        Pose::Lick => {
            // A flick and a pause, not a metronome: one tongue motion per 620 ms.
            let t = since.as_secs_f32();
            let phase = (t / 0.62) % 1.0;
            let tongue = if phase < 0.5 {
                (phase / 0.5 * std::f32::consts::PI).sin()
            } else {
                0.0
            };
            Act {
                dip: 0.018 * tongue + 0.008 * (t * 2.3).sin().abs(),
                tilt: 0.07 + 0.035 * (t * 3.1).sin(),
                tongue,
                tongue_sway: (t * 11.0).sin() * tongue,
                tail_sway: 0.006 * (t * 1.7).sin(),
                ear: 0.06 * (t * 1.3).sin().max(0.0),
                gaze: -0.02 - 0.10 * tongue,
                ..Act::REST
            }
        }
        Pose::Speak => {
            let t = (t / SPEAK_MS as f32).clamp(0.0, 1.0);
            let main = (std::f32::consts::PI * t).sin().max(0.0).powf(0.7);
            let second = if t > 0.55 {
                ((t - 0.55) / 0.45 * std::f32::consts::PI).sin() * 0.55
            } else {
                0.0
            };
            Act {
                mouth: (main + second).min(1.0),
                dip: -0.008 * (2.0 * std::f32::consts::PI * t).sin(),
                tilt: 0.10 + 0.02 * (2.0 * std::f32::consts::PI * t).sin(),
                tail_sway: 0.005 * (2.0 * std::f32::consts::PI * t).sin(),
                gaze: 0.05,
                ..Act::REST
            }
        }
        Pose::Extend => {
            let t = (t / EXTEND_MS as f32).clamp(0.0, 1.0);
            Act {
                reach: 1.0 - 0.30 * (std::f32::consts::PI * t).sin()
                    + 0.045 * (3.0 * std::f32::consts::PI * t).sin(),
                tilt: 0.10 + 0.04 * (std::f32::consts::PI * t).sin(),
                tail_sway: 0.006 * (2.0 * std::f32::consts::PI * t).sin(),
                ear: 0.05 * (2.0 * std::f32::consts::PI * t).sin(),
                gaze: 0.08,
                ..Act::REST
            }
        }
    };
    cat(&mut ink, fw, fh, act);
}

/// The cat in one act, back to front: tail, arm, body, ears, head, paw, face. Every fill is
/// flat paper; every edge is one of the three ink weights.
fn cat(ink: &mut Ink, w: f32, h: f32, act: Act) {
    let (outline, part, detail) = weights(w);
    let dip = act.dip;
    let (hx, hy, r) = head(w, h);
    let hy = hy + dip * h;
    let (mx, my) = mouth(w, h);
    let my = my + dip * h;

    // The tail behind the body: a paper stroke inside its ink border.
    let root = (0.16 * w, 0.74 * h);
    let sway = act.tail_sway * w;
    let tail_width = 0.045 * w;
    if act.tail_up {
        let p = [
            root,
            (0.13 * w + sway * 0.4, 0.58 * h),
            (0.11 * w + sway * 0.8, 0.40 * h),
            (0.16 * w + sway * 1.2, 0.29 * h),
        ];
        stroke_pair(ink, &p, tail_width, outline);
    } else {
        let p = [
            root,
            (0.055 * w + sway * 0.3, 0.68 * h),
            (0.035 * w + sway * 0.6, 0.55 * h),
            (0.08 * w + sway, 0.44 * h),
            (0.16 * w + sway * 1.3, 0.45 * h),
            (0.17 * w + sway * 1.5, 0.55 * h),
            (0.10 * w + sway * 1.6, 0.54 * h),
        ];
        stroke_pair(ink, &p, tail_width, outline);
    }

    // The front leg, behind the body so the body fill hides the shoulder.
    let (rx, ry) = (0.052 * w, 0.040 * w);
    let shoulder = (0.64 * w, 0.76 * h);
    let rest = (w - rx, 0.5 * h);
    let bend = (0.79 * w, 0.64 * h);
    let at = |q: (f32, f32)| {
        (
            shoulder.0 + (q.0 - shoulder.0) * act.reach,
            shoulder.1 + (q.1 - shoulder.1) * act.reach,
        )
    };
    let tip = at(rest);
    if act.reach > 0.01 {
        let wrist = (tip.0 - rx, tip.1);
        stroke_pair(ink, &[shoulder, at(bend), wrist], 0.05 * w, outline);
    }

    // The body: one filled contour closed under the head, then its open ink outline.
    let body = body_points(w, h, dip);
    let mut filled = [(0.0, 0.0); 128];
    let n = sampled(&body, &mut filled);
    ink.polygon(&filled[..n], PAPER);
    ink.curve(&body, outline, INK);
    // The sitting legs and the grounded paw; tucked while the cat is mid-leap.
    if !act.tail_up {
        // The grounded front leg, from the chest to its paw.
        ink.curve(
            &[
                (0.66 * w, 0.70 * h),
                (0.63 * w, 0.80 * h),
                (0.60 * w, 0.885 * h),
            ],
            detail,
            INK,
        );
        // The folded hind leg, one detail line inside the body.
        ink.curve(
            &[
                (0.24 * w, 0.78 * h),
                (0.30 * w, 0.83 * h),
                (0.33 * w, 0.89 * h),
            ],
            detail,
            INK,
        );
        // The other front paw, on the ground.
        let (gx, gy) = (0.60 * w, 0.885 * h);
        let (grx, gry) = (0.055 * w, 0.030 * h);
        ink.ellipse(gx, gy, grx, gry, PAPER);
        ink.ellipse_ring(gx, gy, grx, gry, outline, INK);
        for side in [-1.0, 1.0] {
            let x = gx + side * 0.22 * grx;
            ink.capsule(
                (x, gy + 0.10 * gry),
                (x, gy + 0.68 * gry),
                detail * 0.5,
                INK,
            );
        }
    }

    // The ears behind the head; the head fill hides their bases.
    ear(ink, (hx, hy), r, act.tilt, (205.0, 225.0, 245.0), part);
    ear(
        ink,
        (hx, hy),
        r,
        act.tilt + act.ear,
        (295.0, 315.0, 335.0),
        part,
    );

    // The head in front of the body and the ears.
    ink.disc(hx, hy, r, PAPER);
    ink.ring(hx, hy, r, outline, INK);

    // The mitten paw in front, its right edge at the stage's edge.
    if act.reach > 0.01 {
        ink.ellipse(tip.0, tip.1, rx, ry, PAPER);
        ink.ellipse_ring(tip.0, tip.1, rx, ry, outline, INK);
        for side in [-1.0, 1.0] {
            let x = tip.0 + side * 0.22 * rx;
            ink.capsule(
                (x, tip.1 - 0.90 * ry),
                (x, tip.1 - 0.52 * ry),
                detail * 0.5,
                INK,
            );
        }
    }

    // The face, and the mouth it answers from.
    face(
        ink,
        (hx, hy),
        r,
        act.tilt,
        act.blink,
        act.gaze,
        part,
        detail,
        (mx, my),
    );
    if act.mouth > 0.01 {
        let (cx, cy) = spin((mx - 0.12 * r, my + 0.02 * r), (mx, my), act.tilt);
        let (ex, ey) = (
            0.09 * r * (0.5 + act.mouth),
            0.05 * r + 0.13 * r * act.mouth,
        );
        ink.ellipse(cx, cy, ex, ey, INK);
        ink.ellipse(cx, cy + 0.04 * r * act.mouth, ex * 0.55, ey * 0.45, ACCENT);
        ink.ellipse_ring(cx, cy, ex, ey, part * 0.6, INK);
    }
    if act.tongue > 0.01 {
        let tongue = (
            mx + (0.06 + 0.12 * act.tongue + 0.05 * act.tongue_sway) * r,
            my + (0.20 + 0.36 * act.tongue) * r,
        );
        ink.capsule((mx, my), tongue, 0.06 * r + detail, INK);
        ink.capsule((mx, my), tongue, 0.06 * r, ACCENT);
    }
}

/// Rotate `q` about `pivot`: the ears about the head, the face about the mouth.
fn spin(q: (f32, f32), pivot: (f32, f32), angle: f32) -> (f32, f32) {
    let (s, c) = angle.sin_cos();
    let (x, y) = (q.0 - pivot.0, q.1 - pivot.1);
    (pivot.0 + x * c - y * s, pivot.1 + x * s + y * c)
}

/// A point on the head's ring, `at` degrees round it, `reach` radii from the centre, then
/// turned by `tilt` about that centre so an ear stays on the ring when the head moves.
fn on_head(w: f32, h: f32, dip: f32, tilt: f32, at: f32, reach: f32) -> (f32, f32) {
    let (hx, hy, r) = head(w, h);
    let hy = hy + dip * h;
    let a = at.to_radians();
    spin(
        (hx + a.cos() * r * reach, hy + a.sin() * r * reach),
        (hx, hy),
        tilt,
    )
}

/// A Catmull-Rom point between `p1` and `p2`, with `p0` and `p3` as its tangents.
fn catmull(p0: (f32, f32), p1: (f32, f32), p2: (f32, f32), p3: (f32, f32), t: f32) -> (f32, f32) {
    let (t2, t3) = (t * t, t * t * t);
    let at = |a: f32, b: f32, c: f32, d: f32| {
        0.5 * (2.0 * b
            + (c - a) * t
            + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2
            + (3.0 * b - a - 3.0 * c + d) * t3)
    };
    (at(p0.0, p1.0, p2.0, p3.0), at(p0.1, p1.1, p2.1, p3.1))
}

/// Sample a Catmull-Rom contour into `out` without allocating. The caller supplies the
/// fixed array: the body is filled from this polyline and the ink line is stroked over it.
fn sampled(points: &[(f32, f32)], out: &mut [(f32, f32)]) -> usize {
    let mut n = 0;
    for i in 0..points.len().saturating_sub(1) {
        let p0 = points[i.saturating_sub(1)];
        let p1 = points[i];
        let p2 = points[i + 1];
        let p3 = points.get(i + 2).copied().unwrap_or(p2);
        for k in 0..10 {
            if n < out.len() {
                out[n] = catmull(p0, p1, p2, p3, k as f32 / 10.0);
                n += 1;
            }
        }
    }
    if let Some(last) = points.last()
        && n < out.len()
    {
        out[n] = *last;
        n += 1;
    }
    n
}

/// The body's contour: taller on the chest than the rump, open where the head's fill
/// closes it.
fn body_points(w: f32, h: f32, dip: f32) -> [(f32, f32); 11] {
    [
        on_head(w, h, dip, 0.0, 155.0, 1.0),
        (0.44 * w, 0.43 * h),
        (0.31 * w, 0.47 * h),
        (0.21 * w, 0.58 * h),
        (0.16 * w, 0.74 * h),
        (0.19 * w, 0.86 * h),
        (0.35 * w, 0.91 * h),
        (0.54 * w, 0.90 * h),
        (0.66 * w, 0.82 * h),
        (0.68 * w, 0.68 * h),
        on_head(w, h, dip, 0.0, 50.0, 1.0),
    ]
}

/// One paper stroke inside one ink border: the tail and the arm.
fn stroke_pair(ink: &mut Ink, p: &[(f32, f32)], width: f32, outline: f32) {
    ink.curve(p, width + 2.0 * outline, INK);
    ink.curve(p, width, PAPER);
}

/// A flat paper ear with a coral inner and two ink edges. The head fill hides its base.
fn ear(ink: &mut Ink, centre: (f32, f32), r: f32, tilt: f32, angles: (f32, f32, f32), part: f32) {
    let at = |deg: f32, reach: f32| {
        let a = deg.to_radians();
        spin(
            (
                centre.0 + a.cos() * r * reach,
                centre.1 + a.sin() * r * reach,
            ),
            centre,
            tilt,
        )
    };
    let a = at(angles.0, 1.0);
    let b = at(angles.1, 1.5);
    let d = at(angles.2, 1.0);
    ink.triangle([a, b, d], PAPER);
    let c = ((a.0 + b.0 + d.0) / 3.0, (a.1 + b.1 + d.1) / 3.0);
    let inner = |q: (f32, f32)| (c.0 + (q.0 - c.0) * 0.52, c.1 + (q.1 - c.1) * 0.52);
    ink.triangle([inner(a), inner(b), inner(d)], ACCENT);
    ink.capsule(a, b, part / 2.0, INK);
    ink.capsule(b, d, part / 2.0, INK);
}

/// The face, turned about the mouth: big eye whites, pupils that look with the pose, a
/// coral nose and blush, an ink smile that ends at the mouth, and whiskers.
#[allow(clippy::too_many_arguments)] // the head, its three line roles and the mouth point
fn face(
    ink: &mut Ink,
    (hx, hy): (f32, f32),
    r: f32,
    tilt: f32,
    blink: f32,
    gaze: f32,
    part: f32,
    detail: f32,
    mouth: (f32, f32),
) {
    let f = |q: (f32, f32)| spin(q, mouth, tilt);
    for dx in [-0.38, 0.38] {
        let (ex, ey) = (hx + dx * r, hy - 0.12 * r);
        if blink > 0.5 {
            let a = f((ex - 0.22 * r, ey));
            let b = f((ex, ey + 0.10 * r));
            let d = f((ex + 0.22 * r, ey));
            ink.curve(&[a, b, d], part, INK);
        } else {
            let c = f((ex, ey));
            ink.ellipse(c.0, c.1, 0.22 * r, 0.27 * r, EYE);
            ink.ellipse_ring(c.0, c.1, 0.22 * r, 0.27 * r, part, INK);
            let (px, py) = (c.0 + gaze * r, c.1 + 0.02 * r);
            ink.ellipse(px, py, 0.08 * r, 0.16 * r, INK);
            ink.disc(px - 0.025 * r, py - 0.06 * r, 0.03 * r, EYE);
        }
    }
    // The blush sits behind the nose and the whiskers.
    for dx in [-0.56, 0.56] {
        let b = f((hx + dx * r, hy + 0.42 * r));
        ink.ellipse(b.0, b.1, 0.15 * r, 0.08 * r, BLUSH);
    }
    // The nose, a small coral triangle with a hairline ink edge.
    let nose = [
        f((hx + 0.36 * r, hy + 0.20 * r)),
        f((hx + 0.56 * r, hy + 0.20 * r)),
        f((hx + 0.46 * r, hy + 0.34 * r)),
    ];
    ink.triangle(nose, ACCENT);
    for (a, b) in [(nose[0], nose[1]), (nose[1], nose[2]), (nose[2], nose[0])] {
        ink.capsule(a, b, detail * 0.5, INK);
    }
    // The smile runs into the mouth's corner, so the bubble's tail continues the line.
    ink.curve(
        &[
            f((hx + 0.46 * r, hy + 0.34 * r)),
            f((hx + 0.58 * r, hy + 0.42 * r)),
            mouth,
        ],
        detail,
        INK,
    );
    // Two whiskers on the panel side; the far side is left clean.
    for (from_x, from_y, to_x, to_y) in [(0.72, 0.28, 1.38, 0.08), (0.70, 0.38, 1.34, 0.52)] {
        ink.capsule(
            f((hx + from_x * r, hy + from_y * r)),
            f((hx + to_x * r, hy + to_y * r)),
            detail,
            INK,
        );
    }
}

/// The head over the edge, and the face with it: the cat has not arrived yet.
fn peek_pose(ink: &mut Ink, w: f32, h: f32, edge: Edge, t: f32) {
    let (outline, part, detail) = weights(w);
    let rise = ease_out_back(t.clamp(0.0, 1.0));
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
    let tilt = 0.10 * rise;
    let twitch = 0.05 * (t * 3.0 * std::f32::consts::PI).sin();
    ear(ink, (cx, cy), r, tilt, (205.0, 225.0, 245.0), part);
    ear(ink, (cx, cy), r, tilt + twitch, (295.0, 315.0, 335.0), part);
    ink.disc(cx, cy, r, PAPER);
    ink.ring(cx, cy, r, outline, INK);
    let mouth = (cx + 0.906 * r, cy + 0.423 * r);
    face(ink, (cx, cy), r, tilt, 0.0, 0.10, part, detail, mouth);
}

/// The bubble's tail: the card's corner to the mouth, in the card's own fill.
fn bubble_tail(ink: &mut Ink, w: f32, h: f32, s: u32, bar_h: u32) {
    let (cx, cy) = tail_start(w as u32, h as u32, s, bar_h);
    let (cx, cy) = (cx as f32, cy as f32);
    let (mx, my) = mouth(w, h);
    ink.triangle([(cx, cy), (cx, cy + 0.10 * w), (mx, my)], render::BG_PROMPT);
}

/// Ease out with a small overshoot that settles exactly on 1: a reach, not a slide.
fn ease_out_back(t: f32) -> f32 {
    let u = t.clamp(0.0, 1.0) - 1.0;
    1.0 + 1.9 * u * u * u + 0.9 * u * u
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

    /// An odd side puts the window's centre on half a buffer pixel, and the paw, the pad's
    /// rim and the bubble's tail stop being one point.
    #[test]
    fn the_stage_is_an_even_number_of_pixels() {
        assert_eq!(stage_size(1.0, 181), Some(180));
        assert_eq!(stage_size(1.0, 260), Some(260));
    }

    #[test]
    fn the_contacts_are_one_point_at_every_scale_and_bar() {
        for bar_h in [30u32, 31] {
            for s in [1i32, 2, 3] {
                let panel = Panel {
                    x: 614,
                    y: 400,
                    w: 692,
                    h: 300,
                    bar_h,
                };
                let stage = landing(panel, 1.0).expect("room to the left");
                let bar_centre =
                    panel.y + panel.h as i32 - render::SHADOW as i32 - bar_h as i32 / 2;
                assert_eq!(stage.y + stage.size as i32 / 2, bar_centre, "bar {bar_h}");
                // In the window's own buffer the paw is the row the card's centre is on.
                let paw_row = (stage.size as i32 * s) / 2;
                assert_eq!(
                    stage.y * s + paw_row,
                    bar_centre * s,
                    "bar {bar_h} at scale {s}"
                );
            }
        }
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
        assert!(
            (18_000..50_000).contains(&lit(&waiting)),
            "the cat is a filled mascot: {}",
            lit(&waiting)
        );
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

    /// The mascot is flat paper inside ink: interiors are paper, edges are ink, and the
    /// paw ring still reaches the stage's edge.
    #[test]
    fn the_cat_is_a_flat_paper_mascot() {
        let (w, h) = (260u32, 260u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        draw(
            &mut buf,
            w,
            h,
            1,
            30,
            Edge::Bottom,
            Pose::Waiting,
            Duration::ZERO,
            false,
        );
        let px = |x: u32, y: u32| &buf[((y * w + x) * 4) as usize..][..4];
        let paper = |p: &[u8]| p[3] == 255 && p[2] > 0xe0 && p[1] > 0xd0;
        let ink = |p: &[u8]| p[3] > 0 && p[2] < 0x60;
        assert!(paper(px(78, 182)), "the body is paper");
        assert!(paper(px(169, 88)), "the head is paper");
        assert!(ink(px(140, 234)), "the body's ground line is ink");
        assert!(ink(px(127, 88)), "the head's ring is ink");
        assert!(ink(px(259, 130)), "the paw's ring reaches the pad");
    }

    /// The palette is paper, ink and one coral accent, and nothing else.
    #[test]
    fn the_cat_uses_paper_ink_and_one_accent() {
        let (w, h) = (260u32, 260u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        draw(
            &mut buf,
            w,
            h,
            1,
            30,
            Edge::Bottom,
            Pose::Lick,
            Duration::from_millis(220),
            false,
        );
        // BGRA: paper #f3ecdf, ink #2b2520, accent #ef836d.
        let paper = buf
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] == 255 && p[2] > 0xe0 && p[1] > 0xd0);
        let ink = buf
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0 && p[2] < 0x60);
        let accent = buf
            .as_chunks::<4>()
            .0
            .iter()
            .any(|p| p[3] > 0 && p[0] < 0x90 && p[1] > 0x70 && p[2] > 0xd0);
        assert!(paper, "the body is paper");
        assert!(ink, "the outline is warm ink");
        assert!(accent, "the tongue is the coral accent");
    }

    /// The blink is a window on the waiting clock, open at the start.
    #[test]
    fn the_waiting_cat_blinks_on_its_own_clock() {
        let ms = Duration::from_millis;
        assert_eq!(blink(ms(0)), 0.0);
        assert_eq!(blink(ms(1_300)), 1.0);
        assert_eq!(blink(ms(2_000)), 0.0);
        assert_eq!(blink(ms(6_300)), 1.0, "the next cycle's blink");
    }

    /// The reach overshoots, but never by more than 3 %, and settles exactly on the pad.
    #[test]
    fn the_reach_settles_exactly_on_the_pad() {
        let peak = (0..=100)
            .map(|i| ease_out_back(i as f32 / 100.0))
            .fold(0.0, f32::max);
        assert!(peak > 1.0, "the reach overshoots a little: {peak}");
        assert!(peak <= 1.03, "the overshoot stays under 3 %: {peak}");
        assert_eq!(ease_out_back(1.0), 1.0);
    }

    /// The smile ends on the mouth, so the head's tilt turns the face about a fixed point.
    #[test]
    fn the_face_turns_about_the_mouth() {
        let (w, h) = (260u32, 260u32);
        let mut buf = vec![0u8; (w * h * 4) as usize];
        draw(
            &mut buf,
            w,
            h,
            1,
            30,
            Edge::Bottom,
            Pose::Waiting,
            Duration::from_millis(2_000),
            false,
        );
        let (mx, my) = mouth(w as f32, h as f32);
        let p = &buf[((my as u32 * w + mx as u32) * 4) as usize..][..4];
        assert!(
            p[3] > 0 && p[2] < 0x80,
            "the smile ends in ink on the mouth at {mx},{my}"
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
