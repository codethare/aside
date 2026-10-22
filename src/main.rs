//! aside - a keyboard-summoned AI chat panel for Wayland that leaves nothing
//! behind. Behaviour is specified in `openspec/changes/add-aside-chat-panel/specs/`.

mod chat;
mod clipboard;
mod control;
mod feed;
mod font;
mod ime;
mod layout;
mod panel;
mod provider;
mod render;
mod sandbox;
mod sse;
mod transcript;

use std::process::exit;
use std::time::Duration;

use smithay_client_toolkit::activation::{ActivationHandler, ActivationState, RequestData};
use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::reexports::calloop::PostAction;
use smithay_client_toolkit::reexports::calloop::generic::Generic;
use smithay_client_toolkit::reexports::calloop::{EventLoop, Interest};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
};
use smithay_client_toolkit::seat::pointer::{
    AxisScroll, PointerEvent, PointerEventKind, PointerHandler,
};
use smithay_client_toolkit::seat::{Capability, SeatHandler, SeatState};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shm::Shm;
use smithay_client_toolkit::shm::slot::{Buffer, SlotPool};
use wayland_client::globals::registry_queue_init;
use wayland_client::protocol::{wl_keyboard, wl_output, wl_pointer, wl_seat, wl_shm, wl_surface};
use wayland_client::{Connection, QueueHandle};
use wayland_protocols::wp::text_input::zv3::client::zwp_text_input_v3::{
    ContentHint, ContentPurpose, Event as TextInputEvent, ZwpTextInputV3,
};

use chat::{Close, Request};
use font::PanelFont;
use layout::{Input, Viewport};
use transcript::Transcript;

/// Content inset inside the panel (logical px).
pub const PAD: u32 = 16;
/// `wlmenu`'s bar width, so the two read as one product.
const WIDTH: u32 = 640;
/// How far up the screen the panel's bottom edge sits, as a share of the output
/// height: just below the vertical centre, where the eye already is, instead of
/// hugging the bottom edge. The panel hangs from there and grows upwards.
const EDGE_SHARE: i32 = 38;
/// How tall the replies may be, as a multiple of that same gap: the field's own
/// distance to the screen's bottom is the yardstick the panel grows by.
const REPLY_SHARE: u32 = 1;
/// How long the replies card takes to grow out of the bar.
const REVEAL_MS: u128 = 260;
/// How long a shape change takes at its shortest. The morph is the point, so it is
/// long enough to be seen rather than only to be over.
const MORPH_IN_MS: u128 = 360;
/// How long the bar takes to grow into the answering panel, a little longer
/// because the card travels further.
const MORPH_OUT_MS: u128 = 460;
/// How long the panel takes to open: a line spreading out of a point, then the
/// card unfolding upward out of that line, one after the other.
const ENTER_MS: u128 = 520;
/// How long the panel takes to close, which is the opening run backwards: the card
/// rolls up into a line, then the line pulls in from both ends to a point.
const CLOSE_MS: u128 = 460;
/// How much of an opening or closing the first track takes: the line gets its
/// width first, and the closing run gives up its height first.
const TRACK_SHARE: u128 = 55;
/// How long the left button is held before moving the panel starts: a click on
/// the panel must not be a one-pixel jump.
const DRAG_HOLD_MS: u64 = 250;
/// Left button, from `linux/input-event-codes.h`.
const BTN_LEFT: u32 = 0x110;
/// Output height assumed for the height cap until the real one is known.
const FALLBACK_OUTPUT_H: u32 = 1080;
const APP_ID: &str = "aside";

const USAGE: &str = "\
usage: aside [--model MODEL] [--base-url URL] [--proxy URL] [--preview FILE] [--help] [--version]

  --model MODEL     model to ask (default deepseek/deepseek-v4.1-flash)
  --base-url URL    provider base URL (default https://api.commandcode.ai/provider/v1)
  --proxy URL       proxy for the request, any scheme curl accepts
                    (e.g. socks5h://127.0.0.1:1080); default is a direct connection
  --preview FILE    render a scripted conversation to a PPM file and exit
";

/// What a key press means. Kept pure, so the key table in the spec can be
/// checked without a compositor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Key {
    Send,
    Cancel,
    Prefix,
    Up,
    Down,
    PageUp,
    PageDown,
    Copy,
    Backspace,
    Delete,
    Left,
    Right,
    Home,
    End,
    WordLeft,
    WordRight,
    Text,
    Ignore,
}

/// The key table: one meaning per key, and `Alt+C` is the only mapping that
/// reaches the clipboard.
fn key_action(keysym: Keysym, alt: bool, ctrl: bool, has_text: bool) -> Key {
    let plain = !alt && !ctrl;
    match keysym {
        Keysym::Return | Keysym::KP_Enter => Key::Send,
        Keysym::Escape => Key::Cancel,
        Keysym::Up if plain => Key::Up,
        Keysym::Down if plain => Key::Down,
        Keysym::Prior if plain => Key::PageUp,
        Keysym::Next if plain => Key::PageDown,
        Keysym::BackSpace if plain => Key::Backspace,
        Keysym::Delete if plain => Key::Delete,
        Keysym::Left if ctrl => Key::WordLeft,
        Keysym::Right if ctrl => Key::WordRight,
        Keysym::Left if plain => Key::Left,
        Keysym::Right if plain => Key::Right,
        Keysym::Home => Key::Home,
        Keysym::End => Key::End,
        Keysym::c if alt && !ctrl => Key::Copy,
        Keysym::t if ctrl && !alt => Key::Prefix,
        _ if has_text && plain => Key::Text,
        _ => Key::Ignore,
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        print!("{USAGE}");
        return;
    }
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("aside {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    let flag = |name: &str| -> Option<String> {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if let Some(path) = flag("--preview") {
        if let Err(e) = preview(std::path::Path::new(&path)) {
            eprintln!("aside: {e}");
            exit(1);
        }
        return;
    }

    // Before anything is created: the credential and the configuration. A missing
    // key is one line on stderr and a non-zero exit, not an empty panel.
    let key = match provider::api_key() {
        Some(k) => k,
        None => {
            eprintln!(
                "aside: {} is not set: export it, then run aside again",
                provider::KEY_VAR
            );
            exit(2);
        }
    };
    let config = match provider::resolve(
        flag("--model").as_deref(),
        flag("--base-url").as_deref(),
        flag("--proxy").as_deref(),
    ) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("aside: {e}");
            exit(2);
        }
    };

    // A second launch asks this one to come forward, and exits without even
    // connecting to the compositor.
    let listener = match control::claim() {
        control::Claim::Owner(listener) => Some(listener),
        control::Claim::Raised => return,
        control::Claim::Disabled => None,
    };

    if let Err(e) = run(config, key, listener) {
        eprintln!("aside: {e}");
        exit(1);
    }
}

/// `--preview FILE`: the panel's real layout and paint code, offscreen. This is
/// how the look is checked on a machine with no compositor.
fn preview(path: &std::path::Path) -> Result<(), String> {
    let font = PanelFont::load(font::SIZE).map_err(|e| format!("no usable font: {e}"))?;
    let max_w = chat::answer_width(WIDTH, 1);
    let mut transcript = Transcript::default();
    transcript.ask(
        "what does EAGAIN mean when a socket read returns it?",
        &font,
    );
    transcript.delta(
        "It means the call would block: the resource is momentarily unavailable rather than \
         broken. Retry it, and read a repeated EAGAIN under load as backpressure. \
         同一句话换成中文时走的是 CJK 字体回退。",
        &font,
        max_w,
    );
    transcript.finish_answer(&font);
    transcript.ask("and if it never clears?", &font);
    transcript.delta("Then the peer is gone", &font, max_w);
    transcript.cancel("cancelled", &font, max_w);
    transcript.fail("the provider refused the request (401)", &font, max_w);
    // The answering panel is a fixed size: the bar, the gap and the box.
    let (pw, ph) = (
        WIDTH,
        panel::height_for(room_for_replies(Some(FALLBACK_OUTPUT_H)), &font, 1),
    );
    let (w, h) = surface_size((pw, ph));
    let mut input = Input::default();
    input.insert("next question 下一个问题");
    let viewport = Viewport::new(panel::transcript_px(ph, &font, 1));
    let mut buf = vec![0u8; (w * h * 4) as usize];
    panel::draw(
        &mut buf,
        w,
        h,
        &font,
        &transcript,
        &input,
        &viewport,
        Some(0),
        true,
        "",
        "",
        "",
        1.0,
        panel::Look::settled(panel::State::Replies),
        None,
        1,
    );
    std::fs::write(path, panel::to_ppm(&buf, w, h)).map_err(|e| format!("{}: {e}", path.display()))
}

/// How long to wait for the first configure before calling the compositor's
/// silence a failure. Compositors that configure at all do it in milliseconds;
/// one that never does leaves a client with nothing to draw and no way to be seen.
pub const CONFIGURE_TIMEOUT_S: u64 = 5;

fn run(
    config: provider::Config,
    key: String,
    listener: Option<std::os::unix::net::UnixListener>,
) -> Result<(), String> {
    let conn = Connection::connect_to_env().map_err(|e| format!("no Wayland session: {e}"))?;
    let (globals, event_queue) =
        registry_queue_init(&conn).map_err(|e| format!("cannot list globals: {e}"))?;
    let qh = event_queue.handle();
    let mut event_loop: EventLoop<State> =
        EventLoop::try_new().map_err(|e| format!("cannot create the event loop: {e}"))?;
    let loop_handle = event_loop.handle();
    smithay_client_toolkit::reexports::calloop_wayland_source::WaylandSource::new(
        conn.clone(),
        event_queue,
    )
    .insert(loop_handle.clone())
    .map_err(|e| format!("cannot watch the Wayland socket: {e}"))?;

    // Fonts load on a thread while the handshake happens: the first frame must
    // not wait for a font file.
    let load = font::spawn_load(font::SIZE);

    let compositor = CompositorState::bind(&globals, &qh)
        .map_err(|e| format!("wl_compositor is missing: {e}"))?;
    // A layer surface, not an ordinary window: the compositor places the panel by
    // anchor and margin, and workspaces or tiling never move it.
    let shell =
        LayerShell::bind(&globals, &qh).map_err(|e| format!("wlr-layer-shell is missing: {e}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm is missing: {e}"))?;
    let activation = ActivationState::bind(&globals, &qh).ok();
    let ime_manager = ime::bind(&globals, &qh);

    let surface = compositor.create_surface(&qh);
    surface.set_buffer_scale(1);
    let layer = shell.create_layer_surface(&qh, surface, Layer::Overlay, Some(APP_ID), None);
    // Anchored bottom and left: a layer-shell margin only applies to an anchored
    // edge, and the panel is moved on both axes. With no right anchor the surface
    // keeps its own width, and `left_margin` centres it.
    layer.set_anchor(Anchor::BOTTOM | Anchor::LEFT);
    layer.set_margin(0, 0, bottom_margin(None, 0), 0);
    layer.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    // Collapsed to the bar. The first commit is buffer-less: it asks for the
    // configure, and nothing may be attached before that one arrives.
    let (sw, sh) = surface_size((WIDTH, panel::bar_height()));
    layer.set_size(sw, sh);
    layer.commit();
    let pool_size = surface_size((WIDTH, FALLBACK_OUTPUT_H));
    let pool = SlotPool::new((pool_size.0 * pool_size.1 * 4) as usize, &shm)
        .map_err(|e| format!("cannot create the shm pool: {e}"))?;

    let mut state = State {
        registry_state: RegistryState::new(&globals),
        seat_state: SeatState::new(&globals, &qh),
        output_state: OutputState::new(&globals, &qh),
        shm,
        activation,
        qh,
        pool,
        buffer: None,
        width: WIDTH,
        height: panel::bar_height(),
        scale: 1,
        phase: Phase::Idle,
        mode: Mode::Typing,
        drag: Drag::default(),
        press: None,
        pointer_at: (0.0, 0.0),
        dragging: None,
        pointer: None,
        morph: None,
        enter_start: None,
        close_start: None,
        reveal_pending: false,
        reveal_start: None,
        revealed: false,
        phase_start: std::time::Instant::now(),
        focused: false,
        output_w: None,
        output_h: None,
        first_frame: true,
        configured: false,
        configure_wait: Some(std::time::Instant::now()),
        painted: false,
        dirty: true,
        exit: false,
        layer,
        keyboard: None,
        modifiers: Modifiers::default(),
        font: None,
        load: Some(load),
        transcript: Transcript::default(),
        input: Input::default(),
        highlight: 0,
        viewport: Viewport::new(0),
        request: None,
        ime: ime::Ime::default(),
        seat: None,
        ime_manager,
        text_input: None,
        config,
        key,
    };
    state.adopt_font()?;

    // The raise channel: level-triggered, and accept() never blocks the loop.
    if let Some(listener) = listener {
        loop_handle
            .insert_source(
                Generic::new(
                    listener,
                    Interest::READ,
                    smithay_client_toolkit::reexports::calloop::Mode::Level,
                ),
                |_, listener, state: &mut State| {
                    use std::io::Read;
                    while let Ok((mut stream, _)) = listener.accept() {
                        let mut byte = [0u8; 1];
                        let _ = stream.read(&mut byte);
                    }
                    state.raise();
                    Ok(PostAction::Continue)
                },
            )
            .map_err(|e| format!("cannot watch the control socket: {e}"))?;
    }

    while !state.exit {
        // While a request is open, a byte can arrive at any moment; when idle
        // there is nothing to poll for, so the loop sleeps.
        if state.holding() {
            // Waiting out the hold before a drag begins.
            state.check_hold();
        }
        let timeout = if state.holding() {
            Duration::from_millis(30)
        } else if state.morphing() || state.rising() || state.revealing() {
            // The card is growing: a frame every step of the ease.
            Duration::from_millis(16)
        } else if state.phase == Phase::Thinking {
            // The working light advances by the clock; a frame every step of its
            // lap keeps it smooth.
            Duration::from_millis(16)
        } else if state.request.is_some() {
            Duration::from_millis(16)
        } else {
            Duration::from_millis(250)
        };
        if let Err(e) = event_loop.dispatch(timeout, &mut state) {
            let protocol = conn.protocol_error().map(|e| (e.code, e.message));
            match fate(protocol, state.painted, compositor_gone(&e)) {
                Fate::Report(message) => return Err(message),
                Fate::Quiet => return Ok(()),
                Fate::Unrelated => return Err(format!("the compositor left: {e}")),
            }
        }
        if let Some(error) = conn.protocol_error() {
            return Err(format!(
                "the compositor reported {}: {}",
                error.code, error.message
            ));
        }
        state.tick()?;
    }
    Ok(())
}

/// What a failed dispatch means.
#[derive(Debug, PartialEq, Eq)]
enum Fate {
    /// The compositor's own reason, which outranks anything inferred from the
    /// socket: it is what a rejected request looks like from here.
    Report(String),
    /// The session ended after the panel had been on screen: nothing to say.
    Quiet,
    /// Something else; the caller reports it in its own words.
    Unrelated,
}

/// Decide a failed dispatch, apart from the event loop so each outcome can be
/// pinned without a compositor.
fn fate(protocol: Option<(u32, String)>, painted: bool, gone: bool) -> Fate {
    if let Some((code, message)) = protocol {
        return Fate::Report(format!("the compositor reported {code}: {message}"));
    }
    if gone && !painted {
        // Nothing was ever drawn, so this is not a session ending: the panel was
        // refused before it appeared, and silence would leave no trace at all.
        return Fate::Report(
            "the compositor closed the connection before the first frame".to_string(),
        );
    }
    if gone {
        return Fate::Quiet;
    }
    Fate::Unrelated
}

/// Whether a dispatch failure is the compositor closing the connection — the
/// session ending — rather than something the panel can report on. The socket's
/// error kind sits behind two wrappers and the outer one does not expose it
/// through `source()`, so each wrapper is opened by type.
fn compositor_gone(err: &(dyn std::error::Error + 'static)) -> bool {
    use smithay_client_toolkit::reexports::calloop;
    use smithay_client_toolkit::reexports::client::backend::WaylandError;

    // `wayland-backend`'s error type implements only the deprecated `cause()` and
    // leaves `source()` at its default, so walking the chain misses the I/O error
    // underneath. The two wrappers this program can receive are matched by type
    // instead, which does not depend on that link existing.
    if let Some(e) = err.downcast_ref::<calloop::Error>() {
        return match e {
            calloop::Error::IoError(io) => is_closed(io.kind()),
            calloop::Error::OtherError(inner) => compositor_gone(&**inner),
            calloop::Error::InvalidToken => false,
        };
    }
    if let Some(e) = err.downcast_ref::<WaylandError>() {
        return match e {
            WaylandError::Io(io) => is_closed(io.kind()),
            WaylandError::Protocol(_) => false,
        };
    }
    if let Some(e) = err.downcast_ref::<std::io::Error>() {
        return is_closed(e.kind());
    }
    false
}

/// The socket being gone, as opposed to an error worth reporting.
fn is_closed(kind: std::io::ErrorKind) -> bool {
    matches!(
        kind,
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::NotConnected
            | std::io::ErrorKind::UnexpectedEof
    )
}

/// The width text is laid out at, in buffer pixels: the answering panel's, whatever
/// the panel's own width is at the moment. Laying the transcript out at a smaller
/// width and back would re-wrap it twice per send, and a panel narrower than one
/// CJK character once split it mid-byte.
fn layout_width(scale: u32) -> f32 {
    let s = scale.max(1);
    chat::answer_width(WIDTH * s, s)
}

/// The layer surface carries the panel plus its shadow margin on both sides.
fn surface_size(panel: (u32, u32)) -> (u32, u32) {
    (panel.0 + 2 * render::SHADOW, panel.1 + 2 * render::SHADOW)
}

/// How far the panel has been dragged from its anchored place, in logical px.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Drag {
    left: i32,
    bottom: i32,
}

/// The visible gap under the panel, in logical px: a share of the output height,
/// plus whatever the user dragged it by. Negative drags are clamped by the caller.
fn edge_gap(output_h: Option<u32>, drag: i32) -> u32 {
    (base_gap(output_h) + drag).max(render::SHADOW as i32) as u32
}

/// The layer surface's bottom margin: the visible gap minus the shadow margin it
/// already carries.
fn bottom_margin(output_h: Option<u32>, drag: i32) -> i32 {
    edge_gap(output_h, drag) as i32 - render::SHADOW as i32
}

/// Where a drag leaves the surface horizontally: the centred place plus the drag,
/// kept on the output so the panel cannot be pushed off either side.
fn dragged_left(output_w: Option<u32>, sw: u32, drag: i32) -> i32 {
    let room = output_w
        .map(|ow| (ow as i32 - sw as i32).max(0))
        .unwrap_or(i32::MAX / 2);
    (left_margin(output_w, sw) + drag).clamp(0, room)
}

/// The bottom margin a drag asks for, clamped so the whole surface stays on the
/// output: it cannot go under the bar's own shadow, nor above the top edge.
fn dragged_bottom(output_h: Option<u32>, sh: u32, drag: i32) -> i32 {
    let room = output_h
        .map(|oh| (oh as i32 - sh as i32).max(0))
        .unwrap_or(i32::MAX / 2);
    bottom_margin(output_h, drag).clamp(0, room)
}

/// The drag offset that produces a bottom margin: the inverse of `dragged_bottom`,
/// so `edge_gap` — and the cap derived from it — matches what is on screen.
fn drag_for_bottom(output_h: Option<u32>, bottom: i32) -> i32 {
    bottom + render::SHADOW as i32 - base_gap(output_h)
}

/// The gap the anchor asks for, before any drag.
fn base_gap(output_h: Option<u32>) -> i32 {
    output_h.unwrap_or(FALLBACK_OUTPUT_H) as i32 * EDGE_SHARE / 100
}

/// Whether a pointer at `y` in surface coordinates is over the replies card rather
/// than the bar: the wheel scrolls there, and does nothing over the field.
fn over_replies(panel_h: u32, y: f64) -> bool {
    let top = render::SHADOW as f64;
    let bar = top + (panel_h.saturating_sub(panel::bar_height())) as f64;
    y >= top && y < bar
}

/// The tallest the replies may get: the gap below the bar, or the room left above
/// it when the panel has been dragged so high that the gap is most of the screen.
fn reply_cap(output_h: Option<u32>, gap: u32) -> u32 {
    let h = output_h.unwrap_or(FALLBACK_OUTPUT_H);
    // What is left above the bar once the bar and the gap between the cards are
    // on screen: the panel must never leave the output.
    let chrome = panel::bar_height() + render::SEPARATION;
    let room = h.saturating_sub(gap + render::SHADOW + chrome);
    (gap * REPLY_SHARE).min(room)
}

/// Horizontal margin that centres the surface on the output.
fn left_margin(output_w: Option<u32>, surface_w: u32) -> i32 {
    output_w
        .map(|ow| ((ow as i64 - surface_w as i64) / 2).max(0) as i32)
        .unwrap_or(0)
}

/// The logical size a phase wants: idle and thinking are the bar, because a running
/// request keeps the field it was typed in, and answering is the bar plus the
/// replies' fixed box. These are the panel's sizes; `surface_size` adds the shadow
/// margin, and the answers' own height never enters: the box is fixed. Pure, so the
/// spec's numbers are testable without a surface.
fn size_for(phase: Phase, output_h: Option<u32>) -> (u32, u32) {
    let bar = panel::bar_height();
    match phase {
        Phase::Idle | Phase::Thinking => (WIDTH, bar),
        // Fixed: the room the replies are given, and nothing about how much text
        // they hold, so the panel does not resize as an answer arrives or as it is
        // moved. The replies scroll inside it.
        Phase::Answering => (WIDTH, bar + render::SEPARATION + room_for_replies(output_h)),
    }
}

/// The size a shape change is painted at: the larger of its two shapes, so the
/// surface is resized once per transition and never per frame.
fn morph_size(from: Phase, to: Phase, output_h: Option<u32>) -> (u32, u32) {
    let (fw, fh) = size_for(from, output_h);
    let (tw, th) = size_for(to, output_h);
    (fw.max(tw), fh.max(th))
}

/// How tall the replies card always is: the anchored gap, clamped by the room left
/// above the bar and by the 64 rows the frame may draw.
fn room_for_replies(output_h: Option<u32>) -> u32 {
    let gap = edge_gap(output_h, 0);
    let rows = layout::MAX_ROWS as u32 * panel::bar_height();
    reply_cap(output_h, gap).min(rows)
}

/// How much of the replies card has grown out of the bar after `elapsed_ms`: an
/// ease-out, so it leaves fast and settles instead of stopping dead. Read off the
/// clock like the working light, so a late frame lands at the right height.
pub fn reveal_at(elapsed_ms: u128) -> f32 {
    let t = (elapsed_ms as f32 / REVEAL_MS as f32).clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

/// An ease-out over one track of an opening or closing.
fn track_at(elapsed_ms: u128, over: u128) -> f32 {
    let t = (elapsed_ms as f32 / over.max(1) as f32).clamp(0.0, 1.0);
    1.0 - (1.0 - t) * (1.0 - t) * (1.0 - t)
}

/// The two tracks of the opening, `(spread, rise)`, `elapsed_ms` after it began: a
/// line grows both ways out of a point, and once it has its width the card unfolds
/// upward out of it.
pub fn enter_tracks(elapsed_ms: u128) -> (f32, f32) {
    let split = ENTER_MS * TRACK_SHARE / 100;
    let spread = track_at(elapsed_ms, split);
    // The rise only starts once the line has most of its width, so the two read as
    // one movement instead of two.
    let rise = track_at(
        elapsed_ms.saturating_sub(split * 4 / 5),
        ENTER_MS - split * 4 / 5,
    );
    (spread, rise)
}

/// The two tracks of the closing, `(spread, rise)`, which is the opening run
/// backwards: the card rolls up into a line, then the line pulls in to a point.
pub fn close_tracks(elapsed_ms: u128) -> (f32, f32) {
    let split = CLOSE_MS * TRACK_SHARE / 100;
    let rise = 1.0 - track_at(elapsed_ms, split);
    let spread = 1.0 - track_at(elapsed_ms.saturating_sub(split), CLOSE_MS - split);
    (spread, rise)
}

/// How far a shape change has got after `elapsed_ms` of `over`: a damped spring,
/// normalised so it lands on exactly 1 at the deadline. That way a late frame is at
/// the right shape and the frame after the last one is the settled one.
pub fn morph_at(elapsed_ms: u128, over: u128) -> f32 {
    let t = (elapsed_ms as f32 / over.max(1) as f32).clamp(0.0, 1.0);
    // Under-damped: one visible overshoot of a few percent, then rest.
    const DAMPING: f32 = 5.0;
    const FREQ: f32 = 1.6 * std::f32::consts::PI;
    let spring = 1.0 - (-DAMPING * t).exp() * (FREQ * t).cos();
    // Normalise the spring's own landing value to 1, so the deadline is exact.
    let end = 1.0 - (-DAMPING).exp() * FREQ.cos();
    (spring / end).clamp(0.0, 1.0)
}

/// The shape the panel paints itself as, for a phase.
fn state_of(phase: Phase) -> panel::State {
    match phase {
        Phase::Idle | Phase::Thinking => panel::State::Bar,
        Phase::Answering => panel::State::Replies,
    }
}

/// A shape change in progress: where it started and when.
#[derive(Debug, Clone, Copy)]
struct Morph {
    from: Phase,
    to: Phase,
    start: std::time::Instant,
}

impl Morph {
    /// How long this change runs for, which depends on how far the shape travels.
    fn duration_ms(self) -> u128 {
        match (self.from, self.to) {
            (Phase::Thinking, Phase::Answering) => MORPH_OUT_MS,
            _ => MORPH_IN_MS,
        }
    }
}

/// How long one lap of the working light takes.
const LAP_MS: u128 = 1_400;

/// Where the working light is on the bar's rim, as a fraction of its perimeter,
/// read from elapsed milliseconds rather than from a frame count, so a late frame
/// lands where the time says.
pub fn orbit_at(elapsed_ms: u128) -> f32 {
    (elapsed_ms % LAP_MS) as f32 / LAP_MS as f32
}

/// Who owns the keyboard. `Prefix` is one key wide, the way a terminal
/// multiplexer's prefix is; `Vim` keeps it until `Esc`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Typing goes to the input line, and the key table applies.
    Typing,
    /// `Ctrl+T` was pressed: the next key is a settings command.
    Prefix,
    /// The highlight moves and nothing is typed.
    Vim,
}

/// What the one key after the prefix means. The table is the place later settings
/// keys are added, rather than taking more keys from the input line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefix {
    /// `[`: enter vim mode.
    Vim,
    /// Anything else: leave the prefix and do nothing.
    None,
}

fn prefix_action(keysym: Keysym) -> Prefix {
    match keysym {
        Keysym::bracketleft => Prefix::Vim,
        _ => Prefix::None,
    }
}

/// Where a vim-mode key wants the highlight to go.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Move {
    /// That many rows from where it is.
    Lines(i64),
    /// The first or the last line of the transcript.
    First,
    Last,
}

/// The movement a vim-mode key asks for, if it asks for one. `page` is one
/// screenful, in the unit the highlight counts in.
fn vim_move(keysym: Keysym, ctrl: bool, page: i64) -> Option<Move> {
    match (keysym, ctrl) {
        // `n`/`p` page; `Ctrl+n`/`Ctrl+p` are the same movement one line at a time,
        // which is where `j`/`k` used to be before they moved the window.
        (Keysym::n, false) => Some(Move::Lines(page)),
        (Keysym::p, false) => Some(Move::Lines(-page)),
        (Keysym::n, true) => Some(Move::Lines(1)),
        (Keysym::p, true) => Some(Move::Lines(-1)),
        // The ends of the transcript, vim's own keys for them.
        (Keysym::g, false) => Some(Move::First),
        (Keysym::G, false) => Some(Move::Last),
        _ => None,
    }
}

/// How far one vim-mode keypress moves the panel, in logical px: one padding step,
/// and ten of them when `Ctrl` is held, so the keyboard can cross the screen.
const WINDOW_STEP: i32 = 16;

/// The panel movement a vim-mode key asks for, in logical px: `dx` right, `dy` up.
/// `hjkl` are spatial here, which is why the transcript's own keys moved.
fn window_move(keysym: Keysym, ctrl: bool) -> Option<(i32, i32)> {
    let step = if ctrl { WINDOW_STEP * 10 } else { WINDOW_STEP };
    match keysym {
        Keysym::h => Some((-step, 0)),
        Keysym::l => Some((step, 0)),
        Keysym::j => Some((0, -step)),
        Keysym::k => Some((0, step)),
        _ => None,
    }
}

/// The panel's three states. Their sizes are fixed by the spec; this is the
/// state machine behind them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The bar alone: nothing asked yet.
    Idle,
    /// A question is out and nothing has come back: the bar, with its rim lit.
    Thinking,
    /// An answer, an error or a note is on screen.
    Answering,
}

struct State {
    registry_state: RegistryState,
    seat_state: SeatState,
    output_state: OutputState,
    shm: Shm,
    activation: Option<ActivationState>,
    qh: QueueHandle<State>,
    pool: SlotPool,
    buffer: Option<Buffer>,
    /// Logical size, as the compositor configured it.
    width: u32,
    height: u32,
    scale: u32,
    first_frame: bool,
    /// Whether the compositor has configured the surface yet. A buffer may not be
    /// attached before that: the first commit is buffer-less and its only job is
    /// to ask for the configure, and a strict compositor kills a client that
    /// commits pixels ahead of it ("xdg_surface has never been configured").
    configured: bool,
    /// When the wait for that first configure started, so a compositor that never
    /// sends one is reported instead of leaving the panel invisible forever.
    configure_wait: Option<std::time::Instant>,
    /// Whether any frame has reached the compositor, which is what tells a
    /// refused startup from a session that ended after the panel was on screen.
    painted: bool,
    dirty: bool,
    exit: bool,
    layer: LayerSurface,
    /// Which of the three states the panel is in.
    phase: Phase,
    /// Who owns the keyboard: the input line, the prefix, or vim mode.
    mode: Mode,
    /// Moved by dragging, in logical px from the anchored place.
    drag: Drag,
    /// The left button is down, and since when.
    press: Option<std::time::Instant>,
    /// Where the pointer is, latest first.
    pointer_at: (f64, f64),
    /// The pointer position a drag started from, once the hold has elapsed.
    dragging: Option<(f64, f64)>,
    pointer: Option<wl_pointer::WlPointer>,
    /// The shape change in progress, if any.
    morph: Option<Morph>,
    /// When the panel started opening, once per run.
    enter_start: Option<std::time::Instant>,
    /// When the panel started closing, which is what makes it exit.
    close_start: Option<std::time::Instant>,
    /// A reply arrived while the panel was still morphing: the card's own
    /// reveal waits for the shape change to end instead of running on top of it.
    reveal_pending: bool,
    /// When the replies card started growing out of the bar, once per run.
    reveal_start: Option<std::time::Instant>,
    /// Whether this run has already revealed its first card.
    revealed: bool,
    /// When the current phase began: the working light and the reveal are read off
    /// this clock.
    phase_start: std::time::Instant,
    /// Whether the compositor has handed this surface the keyboard.
    focused: bool,
    /// Logical size of the output the panel is on, once it has reported one.
    output_w: Option<u32>,
    output_h: Option<u32>,
    keyboard: Option<wl_keyboard::WlKeyboard>,
    modifiers: Modifiers,
    font: Option<PanelFont>,
    load: Option<font::FontLoad>,
    transcript: Transcript,
    input: Input,
    highlight: usize,
    viewport: Viewport,
    request: Option<Request>,
    ime: ime::Ime,
    /// The seat the text-input object was created against, once one appears.
    seat: Option<wl_seat::WlSeat>,
    ime_manager: Option<wayland_protocols::wp::text_input::zv3::client::zwp_text_input_manager_v3::ZwpTextInputManagerV3>,
    text_input: Option<ZwpTextInputV3>,
    config: provider::Config,
    key: String,
}

impl State {
    /// The size the surface needs. The replies' box is fixed, so this does not read
    /// the transcript at all: an answer arriving never resizes the panel. A shape
    /// change is painted inside the larger of the two shapes, so the surface is
    /// resized once per transition rather than per frame.
    fn target_size(&self) -> (u32, u32) {
        match self.morph.filter(|_| self.morphing()) {
            Some(m) => morph_size(m.from, m.to, self.output_h),
            None => size_for(self.phase, self.output_h),
        }
    }

    /// How far the shape change has got, if one is running.
    fn morph_progress(&self) -> f32 {
        match self.morph {
            Some(m) => morph_at(m.start.elapsed().as_millis(), m.duration_ms()),
            None => 1.0,
        }
    }

    /// Whether a frame is needed for a shape change or the rise into place.
    fn morphing(&self) -> bool {
        match self.morph {
            Some(m) => m.start.elapsed().as_millis() < m.duration_ms(),
            None => false,
        }
    }

    /// How far the panel has spread across and risen, `(spread, rise)`: one while
    /// it is settled, and part-way through an opening or a closing.
    fn tracks(&self) -> (f32, f32) {
        if let Some(start) = self.close_start {
            return close_tracks(start.elapsed().as_millis());
        }
        match self.enter_start {
            Some(start) => enter_tracks(start.elapsed().as_millis()),
            None => (1.0, 1.0),
        }
    }

    /// Whether the panel is opening or closing, which needs frames of its own.
    fn rising(&self) -> bool {
        let (spread, rise) = self.tracks();
        spread < 1.0 || rise < 1.0
    }

    /// Whether a closing has run its course, which is when the panel goes.
    fn close_finished(&self) -> bool {
        match self.close_start {
            Some(start) => start.elapsed().as_millis() >= CLOSE_MS,
            None => false,
        }
    }

    /// What the panel paints itself as in this frame.
    fn look(&self) -> panel::Look {
        let (spread, rise) = self.tracks();
        match self.morph.filter(|_| self.morphing()) {
            Some(m) => panel::Look::moving(
                state_of(m.to),
                state_of(m.from),
                self.morph_progress(),
                rise,
                spread,
            ),
            None => panel::Look {
                rise,
                spread,
                ..panel::Look::settled(state_of(self.phase))
            },
        }
    }

    /// Move to another phase, morphing into it: the surface takes the larger of the
    /// two shapes for the duration, and settles into the new one when it ends.
    fn set_phase(&mut self, to: Phase) {
        if self.phase == to {
            return;
        }
        // Only a change of shape is a morph: the bar is the bar, whether or not a
        // request is running, so sending a question moves nothing.
        self.morph = (state_of(self.phase) != state_of(to)).then(|| Morph {
            from: self.phase,
            to,
            start: std::time::Instant::now(),
        });
        self.phase = to;
        self.phase_start = std::time::Instant::now();
        self.apply_size();
        self.dirty = true;
    }

    /// A shape change that has run its course is dropped, and the surface takes the
    /// size of the shape it became: one resize, on the frame after the last one.
    fn settle_morph(&mut self) {
        if self.morph.is_some() && !self.morphing() {
            self.morph = None;
            self.apply_size();
            self.dirty = true;
            // The replies card grows out of the bar once the panel is the panel,
            // so a reply is seen as one motion and then as its own content.
            if self.reveal_pending {
                self.reveal_pending = false;
                self.revealed = true;
                self.reveal_start = Some(std::time::Instant::now());
            }
        }
    }

    /// Where the light is on the rim, while a request is open.
    fn head(&self) -> Option<f32> {
        (self.phase == Phase::Thinking).then(|| orbit_at(self.phase_start.elapsed().as_millis()))
    }

    /// How much of the replies card is on screen: 1 unless the reveal is running.
    fn reveal(&self) -> f32 {
        match self.reveal_start {
            Some(start) => reveal_at(start.elapsed().as_millis()),
            None => 1.0,
        }
    }

    /// Whether the reveal needs a frame: it drives its own wake-ups while it runs.
    fn revealing(&self) -> bool {
        self.reveal() < 1.0
    }

    /// Give the surface the size its phase wants, centred on the output, and
    /// re-wrap once if the width changed.
    fn apply_size(&mut self) {
        let (w, h) = self.target_size();
        let (sw, sh) = surface_size((w, h));
        let (left, bottom) = self.margins(sw, sh);
        self.layer.set_margin(0, 0, bottom, left);
        self.layer.set_size(sw, sh);
        let width_changed = w != self.width;
        if width_changed || h != self.height {
            self.width = w;
            self.height = h;
            if width_changed {
                // A different width means different wrapping: once per resize.
                self.rewrap();
            }
            self.buffer = None;
            if let Some(font) = self.font.as_ref() {
                self.viewport.height = panel::transcript_px(h * self.scale, font, self.scale);
            }
            self.viewport.clamp(self.transcript.total_height());
            self.dirty = true;
        }
    }

    /// Where the surface goes: centred plus the drag, kept on the output.
    fn margins(&self, sw: u32, sh: u32) -> (i32, i32) {
        let left = dragged_left(self.output_w, sw, self.drag.left);
        let bottom = dragged_bottom(self.output_h, sh, self.drag.bottom);
        (left, bottom)
    }

    /// Remember the output's logical size: it centres the panel and caps its height.
    fn note_output(&mut self, output: &wl_output::WlOutput) {
        let Some(info) = self.output_state.info(output) else {
            return;
        };
        let Some((w, h)) = info.logical_size else {
            return;
        };
        if w <= 0 || h <= 0 {
            return;
        }
        self.output_w = Some(w as u32);
        self.output_h = Some(h as u32);
        self.apply_size();
    }

    fn adopt_font(&mut self) -> Result<(), String> {
        if self.font.is_some() {
            return Ok(());
        }
        let Some(load) = self.load.take() else {
            return Ok(());
        };
        let font = load
            .join()
            .map_err(|_| "the font thread died".to_string())??;
        self.viewport = Viewport::new(panel::transcript_px(
            self.height * self.scale,
            &font,
            self.scale,
        ));
        self.font = Some(font);
        self.rewrap();
        self.dirty = true;
        Ok(())
    }

    fn buffer_height(&self) -> u32 {
        // The surface carries the shadow margin: the panel is inside it.
        surface_size((self.width, self.height)).1 * self.scale
    }

    fn buffer_width(&self) -> u32 {
        surface_size((self.width, self.height)).0 * self.scale
    }

    /// The width text is laid out at, in buffer pixels. The panel's own width,
    /// not the surface's: the shadow margin is not content.
    fn text_width(&self) -> f32 {
        layout_width(self.scale)
    }

    /// Lay the transcript out again after a width or scale change: once per
    /// resize, never per frame.
    fn rewrap(&mut self) {
        let max_w = self.text_width();
        let State {
            transcript, font, ..
        } = self;
        if let Some(font) = font.as_ref() {
            transcript.rewrap(font, max_w);
        }
    }

    /// A second launch asked us to come forward.
    fn raise(&mut self) {
        if let Some(activation) = self.activation.as_ref() {
            activation.request_token(
                &self.qh,
                RequestData {
                    seat_and_serial: None,
                    surface: Some(self.layer.wl_surface().clone()),
                    app_id: Some(APP_ID.to_string()),
                    udata: (),
                },
            );
        }
        self.dirty = true;
    }

    fn rows(&self) -> usize {
        self.transcript.total_rows()
    }

    /// Rows a `PageUp`/`PageDown` moves: one screenful, in the same unit the
    /// highlight counts in.
    fn page_rows(&self) -> i64 {
        let Some(font) = self.font.as_ref() else {
            return 1;
        };
        (self.viewport.height / font.row_h).max(1) as i64
    }

    fn send(&mut self) {
        if self.request.is_some() || self.input.is_blank() {
            return;
        }
        let question = self.input.take();
        let max_w = self.text_width();
        let State {
            transcript,
            font,
            config,
            key,
            request,
            viewport,
            highlight,
            ..
        } = self;
        let Some(font) = font.as_ref() else { return };
        transcript.ask(&question, font);
        let outcome = Request::start(config, key, &transcript.request_turns());
        let started = match outcome {
            Ok(started) => {
                *request = Some(started);
                true
            }
            Err(e) => {
                transcript.fail(&e, font, max_w);
                false
            }
        };
        let rows = transcript.total_rows();
        viewport.follow(transcript.total_height());
        *highlight = rows.saturating_sub(1);
        self.morph = None;
        self.set_phase(if started {
            Phase::Thinking
        } else {
            Phase::Answering
        });
    }

    fn cancel(&mut self) {
        if let Some(request) = self.request.as_mut() {
            request.cancel();
            self.request = None;
            let max_w = self.text_width();
            let State {
                transcript, font, ..
            } = self;
            if let Some(font) = font.as_ref() {
                transcript.cancel("cancelled", font, max_w);
            }
            viewport_follow(&mut self.viewport, transcript.total_height());
            // Cancelling settles the panel now: no one waits for a morph to watch a
            // stream that has already stopped.
            self.morph = None;
            self.reveal_pending = false;
            if !self.revealed {
                self.revealed = true;
                self.reveal_start = None;
            }
            self.set_phase(Phase::Answering);
        } else if self.close_start.is_none() {
            // Nothing to cancel: the panel closes, rolling up and then pulling in
            // from both ends, and the process goes when that has run.
            self.close_start = Some(std::time::Instant::now());
            self.dirty = true;
        }
    }

    /// The left button is down: once it has been held long enough, the panel
    /// follows the pointer. Checked by the loop rather than by a timer of its own,
    /// so a press that is released early never starts anything.
    fn check_hold(&mut self) {
        let Some(at) = self.press else { return };
        if self.dragging.is_some() || at.elapsed() < Duration::from_millis(DRAG_HOLD_MS) {
            return;
        }
        self.dragging = Some(self.pointer_at);
    }

    /// Whether a frame is needed for the hold deadline.
    fn holding(&self) -> bool {
        self.press.is_some() && self.dragging.is_none()
    }

    /// Move the panel by the pointer's delta since the drag began. The margins are
    /// the only way to move a layer surface; the drag is an offset from the
    /// anchored place, clamped to the output by `margins`.
    fn drag_to(&mut self, position: (f64, f64)) {
        let Some(from) = self.dragging else { return };
        // Up on screen is further from the bottom edge, so the bottom margin grows
        // as the pointer's y shrinks.
        let (dx, dy) = (position.0 - from.0, from.1 - position.1);
        self.move_panel(dx.round() as i32, dy.round() as i32);
        self.dragging = Some(position);
    }

    /// Move the panel by `(dx, dy)` logical px, clamped to the output. The one path
    /// for both the pointer and the keyboard: the clamped margins are turned back
    /// into an offset, so the gap the cap is derived from is the gap on screen.
    fn move_panel(&mut self, dx: i32, dy: i32) {
        let (sw, sh) = surface_size((self.width, self.height));
        let centred = left_margin(self.output_w, sw);
        let left = dragged_left(self.output_w, sw, self.drag.left.saturating_add(dx));
        self.drag.left = left - centred;
        let bottom = dragged_bottom(self.output_h, sh, self.drag.bottom.saturating_add(dy));
        self.drag.bottom = drag_for_bottom(self.output_h, bottom);
        // The cap is a share of the gap, so a move can change how tall the panel may
        // grow: size it again before the margins go out.
        self.apply_size();
        self.dirty = true;
    }

    /// Scroll the replies from a pointer axis event. A notch is three lines; a
    /// touchpad's continuous delta is already pixels, which is what `discrete`
    /// being zero means.
    fn wheel(&mut self, vertical: &AxisScroll) {
        let Some(font) = self.font.as_ref() else {
            return;
        };
        let delta = if vertical.discrete != 0 {
            vertical.discrete as i64 * 3 * font.row_h as i64
        } else {
            (vertical.absolute * self.scale.max(1) as f64).round() as i64
        };
        if delta == 0 {
            return;
        }
        let total = self.transcript.total_height();
        self.viewport.scroll_by(delta, total);
        self.dirty = true;
    }

    /// What a key means while vim mode owns the keyboard. Navigation and the two
    /// keys that already had a meaning in every mode; anything else is swallowed,
    /// because the input line must not change while the mode is on.
    fn on_vim_key(&mut self, event: &KeyEvent) {
        let (alt, ctrl) = (self.modifiers.alt, self.modifiers.ctrl);
        match event.keysym {
            Keysym::Escape => self.leave_vim(),
            Keysym::Return | Keysym::KP_Enter => self.send(),
            Keysym::c if alt && !ctrl => self.copy_highlight(),
            keysym => {
                // The spatial pair moves the panel; the transcript keeps its own
                // keys, one step further out of the way.
                if let Some((dx, dy)) = window_move(keysym, ctrl) {
                    self.move_panel(dx, dy);
                    return;
                }
                match vim_move(keysym, ctrl, self.page_rows()) {
                    Some(Move::Lines(delta)) => self.move_highlight(delta),
                    Some(Move::First) => self.move_to(0),
                    Some(Move::Last) => self.move_to(usize::MAX),
                    None => {}
                }
            }
        }
    }

    /// Vim mode takes the keyboard from the input line and from the input method:
    /// otherwise `j` is composed by fcitx instead of moving the highlight.
    fn enter_vim(&mut self) {
        self.mode = Mode::Vim;
        self.ime.preedit.clear();
        if let Some(text_input) = self.text_input.as_ref() {
            text_input.disable();
            text_input.commit();
        }
        self.dirty = true;
    }

    /// Back to typing, with the input method listening again.
    fn leave_vim(&mut self) {
        self.mode = Mode::Typing;
        if self.focused
            && let Some(text_input) = self.text_input.as_ref()
        {
            text_input.enable();
            text_input.set_content_type(ContentHint::empty(), ContentPurpose::Normal);
            text_input.commit();
        }
        self.report_cursor();
        self.dirty = true;
    }

    /// Put the highlight on a line of the transcript, clamped to what exists.
    fn move_to(&mut self, index: usize) {
        let rows = self.rows();
        if rows == 0 {
            return;
        }
        let next = index.min(rows - 1);
        self.move_highlight(next as i64 - self.highlight as i64);
    }

    /// What the bar says while the prefix waits for its one key.
    fn mode_hint(&self) -> &'static str {
        match self.mode {
            Mode::Prefix => "press [ to enter vim mode",
            Mode::Typing | Mode::Vim => "",
        }
    }

    /// The bar's right-hand label, which is how a swallowed keystroke explains
    /// itself.
    fn mode_label(&self) -> &'static str {
        match self.mode {
            Mode::Vim => "-- VIM --",
            Mode::Typing | Mode::Prefix => "",
        }
    }

    /// `Alt+C`: the only path to the clipboard, and it copies one line.
    fn copy_highlight(&mut self) {
        let Some(font) = self.font.as_ref() else {
            return;
        };
        let text = self
            .transcript
            .rows(layout::Blocks::new(font))
            .get(self.highlight)
            .map(|row| row.text.to_string());
        let Some(text) = text else { return };
        if let Err(e) = clipboard::copy(&text) {
            let max_w = self.text_width();
            let State {
                transcript, font, ..
            } = self;
            if let Some(font) = font.as_ref() {
                transcript.fail(&e, font, max_w);
            }
            self.dirty = true;
        }
    }

    /// Drain whatever the request has produced and finish it when the child
    /// exits. Runs once per event-loop turn, so the UI never blocks on a pipe.
    fn tick(&mut self) -> Result<(), String> {
        if let Some(start) = self.configure_wait
            && start.elapsed() >= Duration::from_secs(CONFIGURE_TIMEOUT_S)
        {
            return Err(format!(
                "the compositor did not configure the window within {CONFIGURE_TIMEOUT_S}s"
            ));
        }
        if self.phase == Phase::Thinking {
            // The blink is the only thing moving, so keep asking for frames.
            self.dirty = true;
        }
        if self.close_finished() {
            // The closing animation has run: the panel goes, and the process with it.
            self.exit = true;
            return Ok(());
        }
        if self.morphing() || self.rising() {
            // A shape change or the rise into place: every frame has to be painted.
            self.dirty = true;
        }
        if self.revealing() {
            // The card is growing: every frame of the ease has to be painted.
            self.dirty = true;
        }
        // A finished shape change hands the surface its new size, once.
        self.settle_morph();
        // The round is over: keys the input method never claimed are the panel's own
        // text. This is what keeps letters and digits working once the input method
        // is switched off or gone. Never while a mode owns the keyboard: there, a
        // letter is a command, and vim mode must not be typed into.
        if self.mode == Mode::Typing
            && let Some(text) = self.ime.flush()
        {
            self.input.insert(&text);
            self.after_typing();
        }
        let max_w = self.text_width();
        let mut text_arrived = false;
        if let Some(open) = self.request.as_mut() {
            let State {
                transcript,
                font,
                viewport,
                ..
            } = self;
            if let Some(font) = font.as_ref() {
                if open.poll(transcript, font, max_w) {
                    text_arrived = true;
                    if viewport.following {
                        viewport.clamp(transcript.total_height());
                    }
                    self.dirty = true;
                }
                if !open.running() {
                    let close = open.finish();
                    self.request = None;
                    match close {
                        Close::Complete => transcript.finish_answer(font),
                        Close::Truncated(note) => transcript.cancel(&note, font, max_w),
                        Close::Failed(message) => transcript.fail(&message, font, max_w),
                    }
                    viewport.follow(transcript.total_height());
                    self.dirty = true;
                    text_arrived = true;
                }
            }
        }
        if text_arrived {
            // The first text ends the wait: the bar grows into the panel, and the
            // replies card's own reveal waits until that shape change is over, so a
            // reply is seen as one motion and then as its content.
            if !self.revealed && self.transcript.total_height() > 0 {
                self.reveal_pending = true;
            }
            self.set_phase(Phase::Answering);
        }
        if self.dirty {
            self.dirty = false;
            self.draw();
        }
        Ok(())
    }

    fn draw(&mut self) {
        if !self.configured {
            // Nothing may be attached before the first configure; the configure
            // handler draws the first frame.
            return;
        }
        let (w, h) = (self.buffer_width(), self.buffer_height());
        if w == 0 || h == 0 {
            return;
        }
        let _ = self.adopt_font();
        if self.font.is_none() {
            return; // no font yet: nothing to draw and nothing to wait for
        }
        // The shape this frame paints, and how far into it the panel has morphed.
        let look = self.look();
        let head = self.head();
        let stride = w as i32 * 4;
        // Read before the canvas borrows the pool: the reveal moves every frame.
        let reveal = self.reveal();
        let label = self.mode_label();
        let hint = self.mode_hint();
        let typing = self.mode == Mode::Typing;
        let buffer = match self.buffer.take() {
            Some(buffer) => buffer,
            None => {
                match self
                    .pool
                    .create_buffer(w as i32, h as i32, stride, wl_shm::Format::Argb8888)
                {
                    Ok((buffer, _)) => buffer,
                    Err(e) => {
                        eprintln!("aside: cannot allocate a frame buffer: {e}");
                        self.exit = true;
                        return;
                    }
                }
            }
        };
        let Some(canvas) = self.pool.canvas(&buffer) else {
            // The compositor still holds the previous buffer; try again later.
            self.buffer = Some(buffer);
            self.dirty = true;
            return;
        };
        // Every pixel is written: the shm slots rotate, so a partial paint would
        // show the previous frame through.
        canvas.fill(0);
        panel::draw(
            canvas,
            w,
            h,
            self.font.as_ref().expect("checked above"),
            &self.transcript,
            &self.input,
            &self.viewport,
            Some(self.highlight),
            typing,
            &self.ime.preedit,
            hint,
            label,
            reveal,
            look,
            head,
            self.scale,
        );
        self.layer
            .wl_surface()
            .damage_buffer(0, 0, w as i32, h as i32);
        if let Err(e) = buffer.attach_to(self.layer.wl_surface()) {
            eprintln!("aside: cannot attach the frame: {e}");
            self.exit = true;
            return;
        }
        self.layer.commit();
        self.painted = true;
        self.buffer = Some(buffer);
    }

    fn on_key(&mut self, event: &KeyEvent) {
        // The panel is on its way out: no key has a meaning any more.
        if self.close_start.is_some() {
            return;
        }
        let text = event.utf8.clone();
        // The prefix takes exactly one key, whatever it is: `[` goes to vim mode
        // and everything else is swallowed, so a settings key never types.
        if self.mode == Mode::Prefix {
            self.mode = Mode::Typing;
            if prefix_action(event.keysym) == Prefix::Vim {
                self.enter_vim();
            }
            self.dirty = true;
            return;
        }
        if self.mode == Mode::Vim {
            self.on_vim_key(event);
            return;
        }
        // While the ball spins the question is already sent: only Esc means
        // anything. The check sits before the key table so no text key slips in.
        if self.phase == Phase::Thinking && event.keysym != Keysym::Escape {
            return;
        }
        let action = key_action(
            event.keysym,
            self.modifiers.alt,
            self.modifiers.ctrl,
            text.is_some(),
        );
        // No input method has been heard from: the keysyms are the only text there
        // is, so they are typed straight away.
        if action == Key::Text && self.ime.wants_raw_keys() {
            if let Some(text) = text {
                self.input.insert(&text);
                self.after_typing();
            }
            return;
        }
        // An input method is in play. A pre-edit means it owns the keyboard; with
        // none, the key is held for the rest of this event round, and typed only if
        // the input method stays silent about it (see `tick`).
        if action == Key::Text {
            if self.ime.preedit.is_empty()
                && let Some(text) = text
            {
                self.ime.hold(&text);
            }
            return;
        }
        match action {
            Key::Send => self.send(),
            Key::Cancel => self.cancel(),
            Key::Prefix => {
                self.mode = Mode::Prefix;
                self.dirty = true;
            }
            Key::Up => self.move_highlight(-1),
            Key::Down => self.move_highlight(1),
            Key::PageUp => self.move_highlight(-self.page_rows()),
            Key::PageDown => self.move_highlight(self.page_rows()),
            Key::Copy => self.copy_highlight(),
            Key::Backspace => {
                self.input.backspace();
                self.after_typing();
            }
            Key::Delete => {
                self.input.delete();
                self.after_typing();
            }
            Key::Left => {
                self.input.left();
                self.after_typing();
            }
            Key::Right => {
                self.input.right();
                self.after_typing();
            }
            Key::Home => {
                self.input.home();
                self.after_typing();
            }
            Key::End => {
                self.input.end();
                self.after_typing();
            }
            Key::WordLeft => {
                self.input.word_left();
                self.after_typing();
            }
            Key::WordRight => {
                self.input.word_right();
                self.after_typing();
            }
            Key::Text | Key::Ignore => {}
        }
    }

    fn after_typing(&mut self) {
        self.dirty = true;
        self.report_cursor();
    }

    /// Tell the input method where the caret is, and what text surrounds it, so
    /// a candidate window lands against the text rather than in the corner.
    fn report_cursor(&mut self) {
        // A commit while the text input is disabled is a client bug (river says
        // so by name): the compositor only applies state while it has focus.
        if !self.focused {
            return;
        }
        let (Some(text_input), Some(font)) = (self.text_input.as_ref(), self.font.as_ref()) else {
            return;
        };
        let s = self.scale;
        let before = self.input.text();
        let max_w = self.text_width();
        let mo = render::SHADOW * s;
        // No rail in the bar: the text starts at the padding.
        let x = (mo + PAD * s) as f32 + render::measure(font, before, max_w);
        // Both offsets are byte offsets into the surrounding text.
        let caret = self.input.caret() as i32;
        // The bar is the last row of the panel, inside the shadow margin.
        let bar_top = mo + (self.height - panel::bar_height()) * s;
        text_input.set_surrounding_text(before.to_string(), caret, caret);
        text_input.set_cursor_rectangle(
            x as i32,
            bar_top as i32,
            (2 * s) as i32,
            font.row_h as i32,
        );
        text_input.commit();
    }

    fn move_highlight(&mut self, delta: i64) {
        let rows = self.rows();
        if rows == 0 {
            return;
        }
        let next = (self.highlight as i64 + delta).clamp(0, rows as i64 - 1) as usize;
        self.highlight = next;
        let State {
            transcript,
            font,
            viewport,
            ..
        } = self;
        if let Some(font) = font.as_ref() {
            let blocks = layout::Blocks::new(font);
            if let Some(y) = transcript.row_y(next, blocks) {
                viewport.ensure_visible(y, blocks.row_h, transcript.total_height());
            }
        }
        self.dirty = true;
    }

    /// What the compositor's text-input events mean for the input line.
    fn on_text_input(&mut self, event: TextInputEvent) {
        // Second line of defence: the text input is disabled in vim mode, but a
        // commit already in flight must not append to a line nobody is typing on.
        if self.mode != Mode::Typing {
            return;
        }
        // The bar draws the pre-edit, so any change to it is a change on screen:
        // it appearing, growing, or being cancelled by `Esc` in the input method.
        let composed = self.ime.preedit.clone();
        match self.ime.handle(event) {
            ime::Action::Commit(text) => {
                // The question is already sent while the ball spins.
                if self.phase != Phase::Thinking {
                    self.input.insert(&text);
                    self.after_typing();
                }
            }
            ime::Action::Preedit(_) | ime::Action::Ignore | ime::Action::RoundTrip => {}
            // The input method asked for characters around the caret to go.
            ime::Action::DeleteSurrounding { before, .. } => {
                for _ in 0..before {
                    self.input.backspace();
                }
                if let Some(text_input) = self.text_input.as_ref() {
                    // Acknowledging the batch is what makes the compositor apply
                    // the surrounding-text change.
                    text_input.commit();
                }
                self.dirty = true;
            }
        }
        if self.ime.preedit != composed {
            self.dirty = true;
        }
    }
}

/// `Viewport::follow` through a reference, so callers holding split borrows can
/// still re-pin the view to the tail.
fn viewport_follow(viewport: &mut Viewport, total: u32) {
    viewport.follow(total);
}

impl CompositorHandler for State {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        let scale = new_factor.max(1) as u32;
        if scale == self.scale {
            return;
        }
        self.scale = scale;
        self.layer.wl_surface().set_buffer_scale(scale as i32);
        // A different density means different glyphs and different wrapping.
        if let Ok(font) = PanelFont::load(font::SIZE * scale as f32) {
            self.font = Some(font);
        }
        let max_w = self.text_width();
        let buffer_h = self.height * self.scale;
        let State {
            transcript,
            font,
            viewport,
            ..
        } = self;
        if let Some(font) = font.as_ref() {
            transcript.rewrap(font, max_w);
            viewport.height = panel::transcript_px(buffer_h, font, scale);
            viewport.clamp(transcript.total_height());
        }
        self.buffer = None;
        self.dirty = true;
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _transform: wl_output::Transform,
    ) {
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _time: u32,
    ) {
        self.draw();
        if self.dirty {
            self.dirty = false;
            self.draw();
        }
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
    }
}

impl LayerShellHandler for State {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        // The compositor closed us: exit rather than live with no way to be seen.
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        _configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        self.configured = true;
        self.configure_wait = None;
        // The size is ours: the configure is the permission to attach, and the
        // panel's own sizing decides what that size is.
        self.apply_size();
        if self.first_frame {
            self.first_frame = false;
            // The panel rises out of the bottom edge on its first frame.
            self.enter_start = Some(std::time::Instant::now());
            self.draw();
        } else if self.dirty {
            self.dirty = false;
            self.draw();
        }
    }
}

impl OutputHandler for State {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        self.note_output(&output);
    }

    fn update_output(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        self.note_output(&output);
    }

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}
}

impl SeatHandler for State {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            if let Ok(pointer) = self.seat_state.get_pointer(qh, &seat) {
                self.pointer = Some(pointer);
            }
            return;
        }
        if capability != Capability::Keyboard || self.keyboard.is_some() {
            return;
        }
        if let Ok(keyboard) = self.seat_state.get_keyboard(qh, &seat, None) {
            self.keyboard = Some(keyboard);
        }
        // The text-input object belongs to a seat, so it is created when one
        // turns up rather than at startup.
        if let Some(manager) = self.ime_manager.clone() {
            let surface = self.layer.wl_surface().clone();
            self.text_input = Some(ime::attach(&manager, &seat, &surface, qh));
            self.report_cursor();
        }
        self.seat = Some(seat);
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Keyboard
            && let Some(keyboard) = self.keyboard.take()
        {
            keyboard.release();
        }
        if capability == Capability::Pointer
            && let Some(pointer) = self.pointer.take()
        {
            pointer.release();
            // A device that went away mid-drag leaves no press behind.
            self.press = None;
            self.dragging = None;
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl KeyboardHandler for State {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        _keysyms: &[Keysym],
    ) {
        if self.layer.wl_surface() == surface
            && let Some(text_input) = self.text_input.as_ref()
        {
            self.focused = true;
            text_input.enable();
            text_input.set_content_type(ContentHint::empty(), ContentPurpose::Normal);
            text_input.commit();
        }
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        if self.layer.wl_surface() == surface {
            self.focused = false;
            // Stop composing, or an input method keeps a pre-edit open while the
            // panel is not focused.
            self.ime.preedit.clear();
            if let Some(text_input) = self.text_input.as_ref() {
                text_input.disable();
                text_input.commit();
            }
        }
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.on_key(&event);
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: KeyEvent,
    ) {
        self.on_key(&event);
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        _event: KeyEvent,
    ) {
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _keyboard: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: Modifiers,
        _raw: RawModifiers,
        _layout: u32,
    ) {
        self.modifiers = modifiers;
    }
}

impl smithay_client_toolkit::shm::ShmHandler for State {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl ActivationHandler for State {
    type RequestUdata = ();

    fn new_token(&mut self, token: String, _data: &RequestData<()>) {
        if let Some(activation) = self.activation.as_ref() {
            activation.activate::<State>(self.layer.wl_surface(), token);
        }
    }
}

smithay_client_toolkit::delegate_registry!(State);

impl ProvidesRegistryState for State {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState, SeatState,];
}

smithay_client_toolkit::delegate_dispatch2!(State);

impl PointerHandler for State {
    fn pointer_frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _pointer: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if &event.surface != self.layer.wl_surface() {
                continue;
            }
            self.pointer_at = event.position;
            match event.kind {
                // Held down and not yet moving: the loop starts the drag when the
                // hold elapses, so a click moves nothing.
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                    self.press = Some(std::time::Instant::now());
                }
                PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                    self.press = None;
                    self.dragging = None;
                }
                PointerEventKind::Motion { .. } => self.drag_to(event.position),
                PointerEventKind::Axis { vertical, .. }
                    if over_replies(self.height, event.position.1) =>
                {
                    self.wheel(&vertical);
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refused_panel_reports_why_instead_of_going_quiet() {
        // The reported failure: the panel never appeared and all that was left was
        // "Broken pipe". A compositor that rejects a request sends its own reason
        // and then closes, so the reason must win over the socket error.
        assert_eq!(
            fate(Some((2, "invalid buffer size".to_string())), false, true),
            Fate::Report("the compositor reported 2: invalid buffer size".to_string())
        );
        // Closed before anything was drawn: a startup failure, not a session end.
        assert_eq!(
            fate(None, false, true),
            Fate::Report("the compositor closed the connection before the first frame".to_string())
        );
        // Closed after the panel had been on screen: the session ended.
        assert_eq!(fate(None, true, true), Fate::Quiet);
        // Not a closed socket at all: the caller reports it.
        assert_eq!(fate(None, false, false), Fate::Unrelated);
        // A protocol error is reported even mid-session.
        assert_eq!(
            fate(Some((1, "bad surface".to_string())), true, true),
            Fate::Report("the compositor reported 1: bad surface".to_string())
        );
    }

    #[test]
    fn the_compositor_leaving_is_not_reported_as_a_failure() {
        use smithay_client_toolkit::reexports::calloop;
        // The shape measured when a compositor exits under a running panel:
        // calloop wraps the I/O error twice before it reaches us, so the kind is
        // only visible down the chain.
        let closed = calloop::Error::OtherError(Box::new(calloop::Error::IoError(
            std::io::Error::from(std::io::ErrorKind::BrokenPipe),
        )));
        assert!(
            compositor_gone(&closed),
            "broken pipe means the session ended"
        );
        for kind in [
            std::io::ErrorKind::ConnectionReset,
            std::io::ErrorKind::ConnectionAborted,
            std::io::ErrorKind::NotConnected,
            std::io::ErrorKind::UnexpectedEof,
        ] {
            let e = calloop::Error::IoError(std::io::Error::from(kind));
            assert!(compositor_gone(&e), "{kind:?} is the socket closing");
        }
        // A real failure, and an error with no I/O cause at all, must stay
        // visible: only the socket closing is the session ending.
        let denied =
            calloop::Error::IoError(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
        assert!(!compositor_gone(&denied));
        assert!(!compositor_gone(&calloop::Error::InvalidToken));
    }

    #[test]
    fn the_pipe_error_is_found_through_the_wrappers_production_uses() {
        use smithay_client_toolkit::reexports::calloop;
        use smithay_client_toolkit::reexports::client::backend::WaylandError;
        // Measured off a compositor killed under a running panel: calloop wraps a
        // `wayland-backend` I/O error, and that type implements only the deprecated
        // `cause()`, so a `source()` walk never reaches the kind. Matching the
        // wrappers by type is what makes this true.
        let real = calloop::Error::OtherError(Box::new(WaylandError::Io(std::io::Error::from(
            std::io::ErrorKind::BrokenPipe,
        ))));
        assert!(compositor_gone(&real), "the shape production produces");
        assert_eq!(
            fate(None, false, compositor_gone(&real)),
            Fate::Report("the compositor closed the connection before the first frame".to_string())
        );
        assert_eq!(fate(None, true, compositor_gone(&real)), Fate::Quiet);
        // A protocol error is the compositor objecting to us, never a closed socket.
        let protocol = calloop::Error::OtherError(Box::new(WaylandError::Protocol(
            smithay_client_toolkit::reexports::client::backend::protocol::ProtocolError {
                code: 3,
                object_id: 7,
                object_interface: "wl_surface".to_string(),
                message: "bad".to_string(),
            },
        )));
        assert!(!compositor_gone(&protocol));
    }

    #[test]
    fn the_three_states_have_the_spec_s_sizes() {
        let bar = panel::bar_height();
        assert_eq!(bar, 30, "row_h_for(20) is 30 at 1x");
        assert_eq!(size_for(Phase::Idle, Some(1080)), (640, bar));
        // Thinking is the bar: the field a question was typed in stays, and only
        // its rim lights up.
        assert_eq!(
            size_for(Phase::Thinking, Some(1080)),
            (640, bar),
            "a running request keeps the bar"
        );
        // Answering: the bar, the 16 px between the cards, and a fixed room for the
        // replies. No term for the text, so it is the same size for one line and for
        // a thousand, and a move does not change it.
        let room = room_for_replies(Some(1080));
        assert_eq!(room, 410, "38 % of 1080");
        let answering = (640, bar + render::SEPARATION + room);
        assert_eq!(size_for(Phase::Answering, Some(1080)), answering);
        // Before the output reports, a 1080-pixel one is assumed.
        assert_eq!(size_for(Phase::Answering, None), answering);
        // A tiny output cannot make the room negative.
        assert_eq!(
            size_for(Phase::Answering, Some(10)).1,
            bar + render::SEPARATION
        );
        // The row cap bounds the box as well as the viewport: past ~5000 px the gap
        // alone would allow more rows than a frame may draw.
        let tall = Some(8000u32);
        assert_eq!(
            room_for_replies(tall),
            layout::MAX_ROWS as u32 * bar,
            "never more than the rows the frame may draw"
        );
    }

    /// The bar sits a share of the output height above the bottom edge, and the
    /// height cap is derived from the same number.
    #[test]
    fn the_bar_sits_a_share_of_the_output_up_the_screen() {
        for (output, gap) in [(1080u32, 410u32), (1440, 547), (600, 228)] {
            assert_eq!(edge_gap(Some(output), 0), gap, "38 % of {output}");
            // The surface margin is the gap less the shadow margin it carries.
            assert_eq!(
                bottom_margin(Some(output), 0),
                (gap - render::SHADOW) as i32
            );
            // A drag moves the gap, and with it the cap.
            assert_eq!(edge_gap(Some(output), 100), gap + 100);
            // The box is the gap, whatever the transcript holds.
            let (_, h) = size_for(Phase::Answering, Some(output));
            assert_eq!(
                h,
                panel::bar_height() + render::SEPARATION + gap,
                "the panel is its bar, the gap between the cards and the replies"
            );
        }
        // Too small an output cannot make the margin negative.
        assert_eq!(edge_gap(Some(10), 0), render::SHADOW);
        assert_eq!(bottom_margin(Some(10), 0), 0);
        // Dragged so high that the gap is most of the screen: the room above the
        // bar clamps the cap instead, and the panel still fits.
        let gap = edge_gap(Some(1080), 500);
        let cap = reply_cap(Some(1080), gap);
        let chrome = panel::bar_height() + render::SEPARATION;
        assert_eq!(
            cap,
            1080 - gap - render::SHADOW - chrome,
            "clamped by the room above"
        );
        assert!(chrome + cap <= 1080 - gap);
    }

    /// `Ctrl+T` opens the prefix, and the prefix is one key wide: `[` goes to vim
    /// mode, everything else leaves it and does nothing.
    #[test]
    fn the_prefix_takes_one_key_and_opens_vim() {
        assert_eq!(
            key_action(Keysym::t, false, true, true),
            Key::Prefix,
            "Ctrl+T is the prefix"
        );
        assert_eq!(prefix_action(Keysym::bracketleft), Prefix::Vim);
        for keysym in [
            Keysym::z,
            Keysym::Escape,
            Keysym::Return,
            Keysym::bracketright,
        ] {
            assert_eq!(prefix_action(keysym), Prefix::None, "{keysym:?}");
        }
    }

    /// The vim tables: `hjkl` step the window, and the transcript keeps its own
    /// keys for the line, the page and the ends.
    #[test]
    fn vim_keys_move_the_window_and_the_highlight() {
        let page = 7;
        // The spatial pair moves the panel, `Ctrl` ten times as far.
        assert_eq!(window_move(Keysym::h, false), Some((-16, 0)));
        assert_eq!(window_move(Keysym::l, false), Some((16, 0)));
        assert_eq!(window_move(Keysym::j, false), Some((0, -16)));
        assert_eq!(window_move(Keysym::k, false), Some((0, 16)));
        assert_eq!(window_move(Keysym::h, true), Some((-160, 0)));
        assert_eq!(window_move(Keysym::k, true), Some((0, 160)));
        for keysym in [Keysym::n, Keysym::p, Keysym::g, Keysym::a, Keysym::Escape] {
            assert_eq!(window_move(keysym, false), None, "{keysym:?}");
        }

        // The transcript: `n`/`p` page, `Ctrl+n`/`Ctrl+p` a line, `g`/`G` the ends.
        assert_eq!(vim_move(Keysym::n, false, page), Some(Move::Lines(page)));
        assert_eq!(vim_move(Keysym::p, false, page), Some(Move::Lines(-page)));
        assert_eq!(vim_move(Keysym::n, true, page), Some(Move::Lines(1)));
        assert_eq!(vim_move(Keysym::p, true, page), Some(Move::Lines(-1)));
        assert_eq!(vim_move(Keysym::g, false, page), Some(Move::First));
        assert_eq!(vim_move(Keysym::G, false, page), Some(Move::Last));
        // `hjkl` no longer move the highlight, and a letter outside the table does
        // nothing at all.
        for keysym in [Keysym::h, Keysym::j, Keysym::k, Keysym::l, Keysym::a] {
            assert_eq!(vim_move(keysym, false, page), None, "{keysym:?}");
        }
    }

    /// A drag moves the surface, but never off the output.
    #[test]
    fn a_drag_stays_on_the_output() {
        let (output, surface) = (Some(1920u32), 688u32);
        let centred = dragged_left(output, surface, 0);
        assert_eq!(centred, (1920 - 688) / 2);
        assert_eq!(dragged_left(output, surface, 100), centred + 100);
        // Far past either edge: clamped to the output, not pushed off it.
        assert_eq!(dragged_left(output, surface, -10_000), 0);
        assert_eq!(
            dragged_left(output, surface, 10_000),
            (1920 - 688),
            "the right edge stays on screen"
        );
        // With no output size yet there is nothing to centre against, and the drag
        // still cannot go negative.
        assert_eq!(dragged_left(None, surface, 100), 100);
        assert_eq!(dragged_left(None, surface, -50), 0);
        // Vertically: the surface cannot be pushed off the top either.
        let sh = 200u32;
        assert_eq!(dragged_bottom(Some(1080), sh, 10_000), 1080 - sh as i32);
        assert_eq!(dragged_bottom(Some(1080), sh, -10_000), 0);
        // The offset and the painted margin always agree, clamped or not: the cap is
        // derived from the gap the user can see.
        for drag in [-500, -20, 0, 40, 400, 5000] {
            let bottom = dragged_bottom(Some(1080), sh, drag);
            let offset = drag_for_bottom(Some(1080), bottom);
            assert_eq!(
                bottom_margin(Some(1080), offset),
                bottom,
                "drag {drag} disagrees with its margin"
            );
        }
        // A drag up grows the gap, and with it the cap; a drag down shrinks both.
        let base = bottom_margin(Some(1080), 0);
        assert_eq!(bottom_margin(Some(1080), -100), base - 100);
        assert!(
            reply_cap(Some(1080), edge_gap(Some(1080), 100))
                > reply_cap(Some(1080), edge_gap(Some(1080), -100))
        );
    }

    /// The wheel scrolls over the replies and nowhere else.
    #[test]
    fn the_wheel_only_counts_over_the_replies() {
        let panel_h = 400; // bar 30 + gap 12 + replies 358
        let top = render::SHADOW as f64;
        assert!(!over_replies(panel_h, top - 1.0), "the shadow margin");
        assert!(over_replies(panel_h, top + 1.0), "the replies card");
        let bar_top = top + (panel_h - panel::bar_height()) as f64;
        assert!(!over_replies(panel_h, bar_top + 1.0), "the bar");
        assert!(
            over_replies(panel_h, bar_top - 0.5),
            "just above the bar is replies"
        );
    }

    /// The shape changes are springs that land on exactly 1 at their deadline, and
    /// the rise into place is an ease-out, so a late frame is at the right shape and
    /// the frame after the last one is the settled one.
    #[test]
    fn a_shape_change_is_read_off_the_clock() {
        for over in [MORPH_IN_MS, MORPH_OUT_MS] {
            assert_eq!(morph_at(0, over), 0.0);
            assert_eq!(morph_at(over, over), 1.0, "the deadline is exact");
            assert_eq!(morph_at(over * 10, over), 1.0, "a late frame is done");
            let mut previous = 0.0;
            for ms in 0..=over {
                let at = morph_at(ms, over);
                assert!(at >= previous - 1e-6, "the spring never goes backwards");
                assert!((0.0..=1.0).contains(&at), "and stays inside its ends");
                previous = at;
            }
            // A spring is not an ease: it overtakes its target on the way.
            assert!(
                (0..=over).any(|ms| morph_at(ms, over) > 0.98),
                "it arrives before the deadline"
            );
        }
    }

    /// The opening and the closing are one run and its reverse: a line spreads out
    /// of a point and the card then unfolds up out of it; closing rolls the card up
    /// into a line and pulls the line in to a point.
    #[test]
    fn the_panel_opens_as_a_line_and_closes_back_into_one() {
        let (spread, rise) = enter_tracks(0);
        assert_eq!((spread, rise), (0.0, 0.0), "nothing at the start");
        assert_eq!(enter_tracks(ENTER_MS), (1.0, 1.0), "the bar at the end");
        // The width comes first: the line has most of its spread before the card
        // has risen a quarter of the way.
        let (spread, rise) = enter_tracks(ENTER_MS * 45 / 100);
        assert!(spread > 0.9, "the line is out: {spread}");
        assert!(rise < 0.3, "and the card has barely unfolded: {rise}");
        // Then it unfolds, and it is finished no earlier than the deadline.
        let (spread, rise) = enter_tracks(ENTER_MS * 85 / 100);
        assert!(spread == 1.0 && rise > 0.5, "unfolding: {rise}");

        // Closing is that run backwards: height first, then the two ends.
        let (spread, rise) = close_tracks(0);
        assert_eq!(
            (spread, rise),
            (1.0, 1.0),
            "whole at the start of the close"
        );
        assert_eq!(close_tracks(CLOSE_MS), (0.0, 0.0), "gone at the end");
        let (spread, rise) = close_tracks(CLOSE_MS * 50 / 100);
        assert!(rise < 0.2, "rolled up into a line: {rise}");
        assert!(spread > 0.5, "and the line is still out: {spread}");
        let (spread, _) = close_tracks(CLOSE_MS * 90 / 100);
        assert!(spread < 0.2, "then it pulls in: {spread}");
        // Both tracks only ever move one way in their own run.
        let mut last = (0.0, 0.0);
        for ms in 0..=ENTER_MS {
            let (spread, rise) = enter_tracks(ms);
            assert!(
                spread >= last.0 && rise >= last.1,
                "opening goes back at {ms}"
            );
            last = (spread, rise);
        }
        last = (1.0, 1.0);
        for ms in 0..=CLOSE_MS {
            let (spread, rise) = close_tracks(ms);
            assert!(
                spread <= last.0 && rise <= last.1,
                "closing goes back at {ms}"
            );
            last = (spread, rise);
        }
    }

    /// A transition needs the larger of its two shapes, which is what the surface is
    /// given for its duration.
    #[test]
    fn a_transition_runs_on_the_larger_shape() {
        let (bw, bh) = size_for(Phase::Idle, Some(1080));
        let (_, th) = size_for(Phase::Thinking, Some(1080));
        let (aw, ah) = size_for(Phase::Answering, Some(1080));
        // Bar to ball: the ball is narrow but taller than the bar's band.
        assert_eq!(
            morph_size(Phase::Idle, Phase::Thinking, Some(1080)),
            (bw, th)
        );
        // Ball to panel: the panel is the larger one in both directions.
        assert_eq!(
            morph_size(Phase::Thinking, Phase::Answering, Some(1080)),
            (aw, ah)
        );
        // A no-op change is the shape itself.
        assert_eq!(morph_size(Phase::Idle, Phase::Idle, Some(1080)), (bw, bh));
    }

    /// The reveal is a deadline: two frames at the same elapsed time paint the
    /// same height, and it is done at the deadline.
    #[test]
    fn the_reveal_is_read_off_the_clock() {
        assert_eq!(reveal_at(0), 0.0);
        assert_eq!(reveal_at(REVEAL_MS), 1.0);
        assert_eq!(reveal_at(REVEAL_MS * 10), 1.0, "a late frame is done");
        // Monotonic, and busy in between: an ease, not a step.
        let mut previous = 0.0;
        for ms in 0..=REVEAL_MS {
            let at = reveal_at(ms);
            assert!(at >= previous, "the reveal never goes backwards");
            previous = at;
        }
        let half = reveal_at(REVEAL_MS / 2);
        assert!(half > 0.5 && half < 1.0, "ease-out: {half} at half time");
    }

    /// The light's position comes from the clock, so a late frame lands where the
    /// time says rather than where a frame count would.
    #[test]
    fn the_working_light_comes_from_the_clock() {
        assert_eq!(orbit_at(0), 0.0);
        assert!((orbit_at(LAP_MS / 4) - 0.25).abs() < 1e-6);
        assert!((orbit_at(LAP_MS / 2) - 0.5).abs() < 1e-6);
        assert_eq!(orbit_at(LAP_MS), 0.0, "one lap is back where it began");
        assert!((orbit_at(5_000) - (5_000 % LAP_MS) as f32 / LAP_MS as f32).abs() < 1e-6);
        // It only ever goes forward inside a lap.
        let mut last = -1.0;
        for ms in (0..LAP_MS).step_by(50) {
            let at = orbit_at(ms);
            assert!(at > last, "the light goes back at {ms}");
            last = at;
        }
    }

    #[test]
    fn the_key_table_maps_exactly_what_the_spec_says() {
        let plain = |k: Keysym| key_action(k, false, false, false);
        assert_eq!(plain(Keysym::Return), Key::Send);
        assert_eq!(plain(Keysym::KP_Enter), Key::Send);
        assert_eq!(plain(Keysym::Escape), Key::Cancel);
        assert_eq!(plain(Keysym::Up), Key::Up);
        assert_eq!(plain(Keysym::Down), Key::Down);
        assert_eq!(plain(Keysym::Prior), Key::PageUp);
        assert_eq!(plain(Keysym::Next), Key::PageDown);
        assert_eq!(plain(Keysym::BackSpace), Key::Backspace);
        assert_eq!(plain(Keysym::Delete), Key::Delete);
        assert_eq!(plain(Keysym::Left), Key::Left);
        assert_eq!(plain(Keysym::Right), Key::Right);
        assert_eq!(plain(Keysym::Home), Key::Home);
        assert_eq!(plain(Keysym::End), Key::End);
        assert_eq!(key_action(Keysym::Left, false, true, false), Key::WordLeft);
        assert_eq!(
            key_action(Keysym::Right, false, true, false),
            Key::WordRight
        );
        assert_eq!(key_action(Keysym::c, true, false, true), Key::Copy);
    }

    #[test]
    fn only_alt_c_reaches_the_clipboard() {
        // The clipboard is a deliberate action or it is nothing: no other key,
        // in any modifier combination, maps to Copy.
        for k in [
            Keysym::Return,
            Keysym::Escape,
            Keysym::Up,
            Keysym::Down,
            Keysym::Prior,
            Keysym::Next,
            Keysym::BackSpace,
            Keysym::Delete,
            Keysym::Left,
            Keysym::Right,
            Keysym::Home,
            Keysym::End,
            Keysym::a,
            Keysym::c,
            Keysym::v,
        ] {
            for alt in [false, true] {
                for ctrl in [false, true] {
                    if key_action(k, alt, ctrl, true) == Key::Copy {
                        assert_eq!(
                            (k, alt, ctrl),
                            (Keysym::c, true, false),
                            "only Alt+C copies"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_modified_key_does_not_type_text() {
        assert_eq!(key_action(Keysym::c, false, true, true), Key::Ignore);
        assert_eq!(key_action(Keysym::a, true, false, true), Key::Ignore);
    }

    #[test]
    fn printable_keys_are_text_only_when_they_carry_text() {
        assert_eq!(key_action(Keysym::a, false, false, true), Key::Text);
        assert_eq!(
            key_action(Keysym::a, false, false, false),
            Key::Ignore,
            "a dead key produces no text and must do nothing"
        );
    }
}
