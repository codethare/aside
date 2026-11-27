//! The cat: where its window lands, where it comes from, what pose it is in, and how it
//! is drawn. Nothing here touches a surface, so the seams it computes are testable
//! without a compositor.

// The stage's surface is what calls all of this; it lands in a later task.
#![cfg_attr(not(test), allow(dead_code, reason = "wired to the stage's surface"))]

use crate::render;

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
