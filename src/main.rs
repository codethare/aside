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
use smithay_client_toolkit::shell::xdg::XdgShell;
use smithay_client_toolkit::shell::xdg::window::{
    Window, WindowConfigure, WindowDecorations, WindowHandler,
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
const BOX_SHARE: u32 = 38;
/// How tall the replies may be, as a multiple of that same gap: the field's own
/// distance to the screen's bottom is the yardstick the panel grows by.
/// How long `copied N lines` stays in the bar.
const COPY_NOTE_MS: u64 = 1_200;
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
    Paste,
    /// `Alt+hjkl`: move the panel itself.
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
        Keysym::c if ctrl && !alt => Key::Copy,
        Keysym::v if ctrl && !alt => Key::Paste,
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
    // The configuration and the font, before anything is drawn or previewed: a bad
    // value is one line naming its line, and the preview then shows the panel the
    // next real run would show.
    let file = match provider::entries() {
        Ok(entries) => entries,
        Err(e) => {
            eprintln!("aside: {e}");
            exit(2);
        }
    };
    let font_size = file
        .iter()
        .find(|(k, _)| k == "font_size")
        .and_then(|(_, v)| v.parse().ok())
        .unwrap_or(font::SIZE);
    let font_family = file
        .iter()
        .find(|(k, _)| k == "font_family")
        .map(|(_, v)| v.clone())
        .filter(|v| !v.trim().is_empty());
    font::configure(font_size, font_family);

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
        &file,
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
    let font = PanelFont::load(font::size()).map_err(|e| format!("no usable font: {e}"))?;
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
        None,
        Some(0),
        None,
        true,
        "",
        "",
        "",
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
    let load = font::spawn_load(font::size());

    let compositor = CompositorState::bind(&globals, &qh)
        .map_err(|e| format!("wl_compositor is missing: {e}"))?;
    // An ordinary window, not a layer surface: only a toplevel can be focused and
    // placed by the compositor and its window rules, which is what a keyboard-summoned
    // panel needs to be typed into without a click.
    let shell =
        XdgShell::bind(&globals, &qh).map_err(|e| format!("xdg_wm_base is missing: {e}"))?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm is missing: {e}"))?;
    let activation = ActivationState::bind(&globals, &qh).ok();
    let ime_manager = ime::bind(&globals, &qh);

    let surface = compositor.create_surface(&qh);
    surface.set_buffer_scale(1);
    // No decorations: the panel is its own chrome, and a titlebar would sit on top of
    // the shadow margin it draws for itself.
    let window = shell.create_window(surface, WindowDecorations::None, &qh);
    window.set_app_id(APP_ID);
    window.set_title(APP_ID);
    // Collapsed to the bar. The first commit is buffer-less: it asks for the
    // configure, and nothing may be attached before that one arrives.
    window.commit();
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
        shape: panel::State::Bar,
        mode: Mode::Typing,
        press: None,
        pointer_at: (0.0, 0.0),
        selecting: false,
        selection: None,
        copy_cursor: (0, 0),
        copied: None,
        pointer: None,
        morph: None,
        enter_start: None,
        close_start: None,
        phase_start: std::time::Instant::now(),
        focused: false,
        output_h: None,
        first_frame: true,
        configured: false,
        configure_wait: Some(std::time::Instant::now()),
        painted: false,
        dirty: true,
        exit: false,
        window,
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
        let timeout = if state.morphing() || state.rising() {
            // The card is growing: a frame every step of the ease.
            Duration::from_millis(16)
        } else if state.copy_note_up() {
            // The note is only up for a moment; a frame every step of that.
            Duration::from_millis(100)
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

/// The byte offset of character `index` in `text`: a selection counts characters,
/// a slice counts bytes.
fn byte_index(text: &str, index: usize) -> usize {
    text.char_indices()
        .nth(index)
        .map_or(text.len(), |(byte, _)| byte)
}

/// How many lines a selection covers: the rows from its first end to its last.
fn copied_lines(sel: &panel::Selection, rows: &[crate::transcript::Row<'_>]) -> usize {
    let ((r0, _), (r1, _)) = sel.ends();
    rows.iter()
        .enumerate()
        .filter(|(i, row)| {
            (r0..=r1).contains(i) && sel.columns(*i, row.text.chars().count()).is_some()
        })
        .count()
        .max(1)
}

/// What a pointer motion means. The button has to be down for any of it: a moving
/// pointer over the panel is a hover, and a hover selects nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Motion {
    /// No button down: the pointer is just passing over.
    Hover,
    /// Extend the selection the press started.
    Extend,
}

fn motion_with(press: Option<(f64, f64)>) -> Motion {
    match press {
        None => Motion::Hover,
        Some(_) => Motion::Extend,
    }
}

/// A pointer's logical y turned into an offset in the replies card, in buffer
/// pixels, plus how far the view is scrolled. The card's top is the panel's top,
/// which is the shadow margin in from the surface's.
fn transcript_y(pointer_y: f64, scale: u32, viewport_top: u32) -> u32 {
    let s = scale.max(1) as f64;
    let y = (pointer_y * s - render::SHADOW as f64 * s).max(0.0);
    y.round() as u32 + viewport_top
}

/// Whether a pointer at `y` in surface coordinates is over the replies card rather
/// than the bar: the wheel scrolls there, and does nothing over the field.
fn over_replies(panel_h: u32, y: f64) -> bool {
    let top = render::SHADOW as f64;
    let bar = top + (panel_h.saturating_sub(panel::bar_height())) as f64;
    y >= top && y < bar
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

/// How tall the replies card always is: a share of the output, capped by the 64 rows
/// one frame may draw. It is a size rather than a margin, because the compositor
/// decides where the window goes and the box has to fit wherever that is.
fn room_for_replies(output_h: Option<u32>) -> u32 {
    let h = output_h.unwrap_or(FALLBACK_OUTPUT_H);
    let rows = layout::MAX_ROWS as u32 * panel::bar_height();
    (h * BOX_SHARE / 100).min(rows)
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

/// The shape a phase is drawn as, given the shape already on screen. A request in
/// flight keeps the box it was asked from: shrinking to the bar and growing back on
/// every follow-up reads as the panel folding itself shut, and the answer that is
/// coming will fill the same box anyway. The working light says the request is out.
fn shape_after(current: panel::State, to: Phase) -> panel::State {
    match (current, to) {
        (panel::State::Replies, Phase::Thinking) => panel::State::Replies,
        _ => state_of(to),
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
/// multiplexer's prefix is; `Copy` keeps it until it is left.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// Typing goes to the input line, and the key table applies.
    Typing,
    /// `Ctrl+T` was pressed: the next key is a settings command.
    Prefix,
    /// A cursor walks the replies and can take a selection out with it.
    Copy,
}

/// What the one key after the prefix means. The table is the place later settings
/// keys are added, rather than taking more keys from the input line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Prefix {
    /// `[`: enter copy mode.
    Copy,
    /// Anything else: leave the prefix and do nothing.
    None,
}

fn prefix_action(keysym: Keysym) -> Prefix {
    match keysym {
        Keysym::bracketleft => Prefix::Copy,
        _ => Prefix::None,
    }
}

/// What a key means in copy mode, tmux's table cut down to what this panel has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CopyKey {
    /// One character left or right.
    Left,
    Right,
    /// The start or the end of the line.
    LineStart,
    LineEnd,
    /// That many lines down (negative up).
    Row(i64),
    /// The next or previous word.
    WordForward,
    WordBack,
    First,
    Last,
    /// Start a selection, or end the one there is.
    Select,
    /// Copy what is selected and leave.
    Copy,
    /// Leave without copying.
    Quit,
    /// Clear the selection, or leave when there is none.
    Cancel,
}

/// The copy-mode table. `page` is one screenful in rows, so the keys that move by a
/// page need no idea of the viewport's own arithmetic.
fn copy_key(keysym: Keysym, ctrl: bool, page: i64) -> Option<CopyKey> {
    let half = (page / 2).max(1);
    match (keysym, ctrl) {
        (Keysym::h, false) => Some(CopyKey::Left),
        (Keysym::l, false) => Some(CopyKey::Right),
        (Keysym::_0, false) => Some(CopyKey::LineStart),
        (Keysym::dollar, false) => Some(CopyKey::LineEnd),
        (Keysym::j, false) => Some(CopyKey::Row(1)),
        (Keysym::k, false) => Some(CopyKey::Row(-1)),
        (Keysym::w, false) => Some(CopyKey::WordForward),
        (Keysym::b, false) => Some(CopyKey::WordBack),
        (Keysym::d, true) => Some(CopyKey::Row(half)),
        (Keysym::u, true) => Some(CopyKey::Row(-half)),
        (Keysym::f, true) | (Keysym::Next, _) => Some(CopyKey::Row(page)),
        (Keysym::b, true) | (Keysym::Prior, _) => Some(CopyKey::Row(-page)),
        (Keysym::g, false) => Some(CopyKey::First),
        (Keysym::G, false) => Some(CopyKey::Last),
        (Keysym::space, false) | (Keysym::v, false) => Some(CopyKey::Select),
        (Keysym::Return, _) | (Keysym::KP_Enter, _) | (Keysym::y, false) => Some(CopyKey::Copy),
        (Keysym::q, false) => Some(CopyKey::Quit),
        (Keysym::Escape, _) => Some(CopyKey::Cancel),
        _ => None,
    }
}

/// How far one `Alt+hjkl` press moves the panel, in logical px: one padding step,
/// and ten of them with `Ctrl` held, so the keyboard can cross the screen.
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
    window: Window,
    /// Which of the three states the panel is in.
    phase: Phase,
    /// The shape it is drawn as, which is not always the phase's own: a request
    /// keeps the box a follow-up was asked from.
    shape: panel::State,
    /// Who owns the keyboard: the input line, the prefix, or copy mode.
    mode: Mode,
    /// Where the left button came down, while it is down: the drag's own origin, so
    /// the panel follows from the first motion rather than from the second.
    press: Option<(f64, f64)>,
    /// Where the pointer is, latest first.
    pointer_at: (f64, f64),
    /// Whether the press that is down started a selection.
    selecting: bool,
    /// What a pointer selected, in characters: an anchor and where it has been
    /// dragged to, either end first.
    selection: Option<panel::Selection>,
    /// Where the copy-mode cursor is: a line and a character of it.
    copy_cursor: (usize, usize),
    /// When the last copy happened, and how many lines it took.
    copied: Option<(std::time::Instant, usize)>,
    pointer: Option<wl_pointer::WlPointer>,
    /// The shape change in progress, if any.
    morph: Option<Morph>,
    /// When the panel started opening, once per run.
    enter_start: Option<std::time::Instant>,
    /// When the panel started closing, which is what makes it exit.
    close_start: Option<std::time::Instant>,
    /// When the current phase began: the working light is read off this clock.
    phase_start: std::time::Instant,
    /// Whether the compositor has handed this surface the keyboard.
    focused: bool,
    /// Logical size of the output the panel is on, once it has reported one.
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
                self.shape,
                state_of(m.from),
                self.morph_progress(),
                rise,
                spread,
            ),
            None => panel::Look {
                rise,
                spread,
                ..panel::Look::settled(self.shape)
            },
        }
    }

    /// Move to another phase, morphing into it: the surface takes the larger of the
    /// two shapes for the duration, and settles into the new one when it ends.
    fn set_phase(&mut self, to: Phase) {
        if self.phase == to {
            return;
        }
        let shape = shape_after(self.shape, to);
        // Only a change of shape is a morph.
        self.morph = (self.shape != shape).then(|| Morph {
            from: self.phase,
            to,
            start: std::time::Instant::now(),
        });
        self.shape = shape;
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
        }
    }

    /// Where the light is on the rim, while a request is open.
    fn head(&self) -> Option<f32> {
        (self.phase == Phase::Thinking).then(|| orbit_at(self.phase_start.elapsed().as_millis()))
    }

    /// Give the window the size its phase wants, and re-wrap once if the width
    /// changed. The size is a buffer's size: where the window goes is not ours.
    fn apply_size(&mut self) {
        let (w, h) = self.target_size();
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

    /// Remember the output's logical height: the reply box is a share of it.
    fn note_output(&mut self, output: &wl_output::WlOutput) {
        let Some(info) = self.output_state.info(output) else {
            return;
        };
        let Some((_w, h)) = info.logical_size else {
            return;
        };
        if h <= 0 {
            return;
        }
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
                    surface: Some(self.window.wl_surface().clone()),
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
        // The lines a selection named are about to be gone.
        self.selection = None;
        self.selecting = false;
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
            self.set_phase(Phase::Answering);
        } else if self.selection.is_some() {
            // A selection is the first thing Escape takes: it cancels nothing and
            // closes nothing.
            self.selection = None;
            self.dirty = true;
        } else if self.close_start.is_none() {
            // Nothing to cancel: the panel closes, rolling up and then pulling in
            // from both ends, and the process goes when that has run.
            self.close_start = Some(std::time::Instant::now());
            self.dirty = true;
        }
    }

    /// What a press on the panel starts. On a reply line it anchors a selection;
    /// anywhere else it does nothing. The window is the compositor's to move.
    fn press_on(&mut self, position: (f64, f64)) {
        let spot = self.reply_spot(position);
        self.selecting = spot.is_some();
        self.selection = spot.map(|(row, ch)| panel::Selection {
            anchor: (row, ch),
            cursor: (row, ch),
        });
        self.dirty = true;
    }

    /// Which reply character a pointer position is over: the row, and how many
    /// characters of it are to the left of the pointer.
    fn reply_spot(&self, position: (f64, f64)) -> Option<(usize, usize)> {
        let row_index = self.reply_row(position)?;
        let font = self.font.as_ref()?;
        let blocks = layout::Blocks::new(font);
        let row = self
            .transcript
            .rows(blocks)
            .get(row_index)?
            .text
            .to_string();
        // The pointer's x, less the card's inset, is the text it is over.
        let inset = (render::SHADOW + crate::PAD) as f64;
        let x = position.0 - inset;
        Some((row_index, render::char_at(font, &row, x as f32)))
    }

    /// Which reply line a pointer position is on, if any: the same walk the
    /// transcript does for the keyboard, in the other direction.
    fn reply_row(&self, position: (f64, f64)) -> Option<usize> {
        if !over_replies(self.height, position.1) {
            return None;
        }
        let font = self.font.as_ref()?;
        let blocks = layout::Blocks::new(font);
        let y = transcript_y(position.1, self.scale, self.viewport.top);
        self.transcript.row_at(y, blocks)
    }

    /// A motion extends a selection — but only while the button is down. A pointer
    /// merely passing over the replies selects nothing, and nothing the panel does
    /// moves its window: that is the compositor's.
    fn motion_to(&mut self, position: (f64, f64)) {
        if motion_with(self.press) == Motion::Hover {
            return;
        }
        if let (Some((row, ch)), Some(sel)) = (self.reply_spot(position), self.selection) {
            self.selection = Some(panel::Selection {
                cursor: (row, ch),
                ..sel
            });
        }
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

    /// What a key means while copy mode owns the keyboard: tmux's table, a cursor
    /// in the replies, and a selection that is the same one the pointer writes.
    fn on_copy_key(&mut self, event: &KeyEvent) {
        let Some(font) = self.font.as_ref() else {
            return;
        };
        let blocks = layout::Blocks::new(font);
        let rows = self.transcript.rows(blocks);
        if rows.is_empty() {
            if copy_key(event.keysym, self.modifiers.ctrl, self.page_rows()) == Some(CopyKey::Quit)
            {
                self.leave_copy();
            }
            return;
        }
        let len = |row: usize| rows.get(row).map_or(0, |r| r.text.chars().count());
        let last = rows.len() - 1;
        let (mut row, mut ch) = self.copy_cursor;
        row = row.min(last);
        ch = ch.min(len(row));
        let page = self.page_rows();
        let Some(key) = copy_key(event.keysym, self.modifiers.ctrl, page) else {
            return;
        };
        match key {
            CopyKey::Left => ch = ch.saturating_sub(1),
            CopyKey::Right => ch = (ch + 1).min(len(row)),
            CopyKey::LineStart => ch = 0,
            CopyKey::LineEnd => ch = len(row),
            CopyKey::Row(delta) => {
                let next = (row as i64 + delta).clamp(0, last as i64) as usize;
                row = next;
                ch = ch.min(len(row));
            }
            CopyKey::WordForward => (row, ch) = self.transcript.word_forward((row, ch), blocks),
            CopyKey::WordBack => (row, ch) = self.transcript.word_back((row, ch), blocks),
            CopyKey::First => {
                row = 0;
                ch = ch.min(len(0));
            }
            CopyKey::Last => {
                row = last;
                ch = ch.min(len(row));
            }
            CopyKey::Select => {
                // Space starts a selection at the cursor, and ends the one there is.
                self.selection = match self.selection {
                    Some(sel) if sel.cursor == (row, ch) => None,
                    _ => Some(panel::Selection {
                        anchor: (row, ch),
                        cursor: (row, ch),
                    }),
                };
            }
            CopyKey::Copy => {
                // How many lines went out, for the note in the bar: the copy itself
                // reads the same rows again, and it is a keypress, not a frame.
                let lines = match self.selection {
                    Some(sel) => copied_lines(&sel, &rows),
                    None => 1,
                };
                self.copy_highlight();
                self.copied = Some((std::time::Instant::now(), lines));
                self.leave_copy();
                return;
            }
            CopyKey::Quit => {
                self.leave_copy();
                return;
            }
            CopyKey::Cancel => {
                if self.selection.take().is_some() {
                    self.selection = None;
                } else {
                    self.leave_copy();
                    return;
                }
            }
        }
        self.copy_cursor = (row, ch);
        if let Some(sel) = self.selection.as_mut() {
            // Moving with a selection extends it, as tmux does.
            sel.cursor = (row, ch);
        }
        self.highlight = row;
        let blocks = layout::Blocks::new(font);
        if let Some(y) = self.transcript.row_y(row, blocks) {
            let total = self.transcript.total_height();
            self.viewport.ensure_visible(y, blocks.row_h, total);
        }
        self.dirty = true;
    }

    /// Copy mode takes the keyboard from the input line and from the input method:
    /// otherwise `j` is composed by fcitx instead of moving the cursor.
    fn enter_copy(&mut self) {
        self.mode = Mode::Copy;
        self.copy_cursor = (self.highlight, 0);
        self.ime.preedit.clear();
        if let Some(text_input) = self.text_input.as_ref() {
            text_input.disable();
            text_input.commit();
        }
        self.dirty = true;
    }

    /// Back to typing, with the input method listening again.
    fn leave_copy(&mut self) {
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

    /// What the bar says while the prefix waits for its one key.
    fn mode_hint(&self) -> &'static str {
        match self.mode {
            Mode::Prefix => "press [ to enter copy mode",
            Mode::Typing | Mode::Copy => "",
        }
    }

    /// Whether `copied N lines` is still on the bar, which needs frames to clear it.
    fn copy_note_up(&self) -> bool {
        self.copied
            .is_some_and(|(at, _)| at.elapsed() < Duration::from_millis(COPY_NOTE_MS))
    }

    /// The bar's right-hand label, which is how a swallowed keystroke explains
    /// itself.
    fn mode_label(&self) -> &'static str {
        // The copy note outranks the mode label: it is what just happened, and it
        // is only up for a moment.
        if let Some((at, lines)) = self.copied
            && at.elapsed() < Duration::from_millis(COPY_NOTE_MS)
        {
            return if lines <= 1 {
                "copied 1 line"
            } else {
                "copied lines"
            };
        }
        match self.mode {
            Mode::Copy => "-- COPY --",
            Mode::Typing | Mode::Prefix => "",
        }
    }

    /// `Alt+C`: the only path to the clipboard, and it copies one line.
    /// `Ctrl+V`: the clipboard comes in once, at the caret, and stays only in the
    /// input line. A failure is shown in the transcript, not swallowed.
    fn paste_clipboard(&mut self) {
        match clipboard::paste() {
            Ok(text) if !text.is_empty() => {
                self.input.insert(&text);
                self.after_typing();
            }
            Ok(_) => {}
            Err(e) => {
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
    }

    /// Copy the marked lines, one per line. The selection is what a pointer chose;
    /// without one it is the highlighted line, which is what `Alt+C` has always
    /// meant.
    fn copy_highlight(&mut self) {
        let Some(font) = self.font.as_ref() else {
            return;
        };
        let rows = self.transcript.rows(layout::Blocks::new(font));
        // What the pointer chose, character for character; without a selection the
        // whole line the keyboard names, which is what Home/End then Alt+C did before
        // there was a pointer.
        let text = match self.selection {
            Some(sel) => {
                let mut lines = Vec::new();
                for (index, row) in rows.iter().enumerate() {
                    let Some((from, to)) = sel.columns(index, row.text.chars().count()) else {
                        continue;
                    };
                    lines.push(&row.text[byte_index(row.text, from)..byte_index(row.text, to)]);
                }
                lines.join("\n")
            }
            None => match rows.get(self.highlight) {
                Some(row) => row.text.to_string(),
                None => return,
            },
        };
        if text.is_empty() {
            return;
        }
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
        // A finished shape change hands the surface its new size, once.
        self.settle_morph();
        // The round is over: keys the input method never claimed are the panel's own
        // text. This is what keeps letters and digits working once the input method
        // is switched off or gone. Never while a mode owns the keyboard: there, a
        // letter is a command, and copy mode must not be typed into.
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
            // The first text ends the wait: the bar grows into the panel, and that
            // growth is the reply's entrance. Nothing follows it: a second
            // animation inside the finished panel reads as the panel opening twice.
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
        let selection = self.selection;
        let head = self.head();
        let stride = w as i32 * 4;
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
            selection,
            Some(self.highlight),
            (self.mode == Mode::Copy).then_some(self.copy_cursor),
            typing,
            &self.ime.preedit,
            hint,
            label,
            look,
            head,
            self.scale,
        );
        self.window
            .wl_surface()
            .damage_buffer(0, 0, w as i32, h as i32);
        if let Err(e) = buffer.attach_to(self.window.wl_surface()) {
            eprintln!("aside: cannot attach the frame: {e}");
            self.exit = true;
            return;
        }
        self.window.commit();
        self.painted = true;
        self.buffer = Some(buffer);
    }

    fn on_key(&mut self, event: &KeyEvent) {
        // The panel is on its way out: no key has a meaning any more.
        if self.close_start.is_some() {
            return;
        }
        let text = event.utf8.clone();
        // The prefix takes exactly one key, whatever it is: `[` goes to copy mode
        // and everything else is swallowed, so a settings key never types.
        if self.mode == Mode::Prefix {
            self.mode = Mode::Typing;
            if prefix_action(event.keysym) == Prefix::Copy {
                self.enter_copy();
            }
            self.dirty = true;
            return;
        }
        if self.mode == Mode::Copy {
            self.on_copy_key(event);
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
            Key::Paste => self.paste_clipboard(),
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
        let mo = render::SHADOW * s;
        // The caret's own place, not the end of the line: a scrolled bar would put the
        // candidate window past the panel otherwise.
        let at = self.input.caret().min(self.input.text().len());
        let limit = self.text_width();
        let scroll = panel::caret_scroll(font, &self.input, limit, "", s);
        // No rail in the bar: the text starts at the padding.
        let x =
            (mo + PAD * s) as f32 + render::measure(font, &self.input.text()[..at], limit) - scroll;
        // Both offsets are byte offsets into the surrounding text.
        let caret = self.input.caret() as i32;
        // The bar is the last row of the panel, inside the shadow margin.
        let bar_top = mo + (self.height - panel::bar_height()) * s;
        text_input.set_surrounding_text(self.input.text().to_string(), caret, caret);
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
        // Second line of defence: the text input is disabled in copy mode, but a
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
        self.window.wl_surface().set_buffer_scale(scale as i32);
        // A different density means different glyphs and different wrapping.
        if let Ok(font) = PanelFont::load(font::size() * scale as f32) {
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

impl WindowHandler for State {
    fn request_close(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window) {
        // The compositor asked us to close: exit rather than live with no way to be
        // seen.
        self.exit = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _window: &Window,
        _configure: WindowConfigure,
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
            let surface = self.window.wl_surface().clone();
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
        if self.window.wl_surface() == surface
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
        if self.window.wl_surface() == surface {
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
            activation.activate::<State>(self.window.wl_surface(), token);
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
            if &event.surface != self.window.wl_surface() {
                continue;
            }
            self.pointer_at = event.position;
            match event.kind {
                // Where the press lands decides the gesture: on a reply line it
                // selects text, anywhere else it will move the panel.
                PointerEventKind::Press { button, .. } if button == BTN_LEFT => {
                    self.press = Some(event.position);
                    self.press_on(event.position);
                }
                PointerEventKind::Release { button, .. } if button == BTN_LEFT => {
                    self.press = None;
                    // The selection stays where it was chosen; the gesture that made
                    // it is over, so a later hover cannot extend it.
                    self.selecting = false;
                }
                // A pointer that leaves with the button down leaves the gesture
                // behind too, or the next hover would carry it on.
                PointerEventKind::Leave { .. } => {
                    self.press = None;
                    self.selecting = false;
                }
                PointerEventKind::Motion { .. } => self.motion_to(event.position),
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
        // A tiny output asks for a tiny box, never a negative one.
        assert_eq!(room_for_replies(Some(10)), 3);
        // The row cap bounds the box as well as the viewport: past ~5000 px the share
        // alone would allow more rows than a frame may draw.
        let tall = Some(8000u32);
        assert_eq!(
            room_for_replies(tall),
            layout::MAX_ROWS as u32 * bar,
            "never more than the rows the frame may draw"
        );
    }

    /// The reply box is a share of the output, and the same share of a short output:
    /// the number is a size now, not a margin under an anchor.
    #[test]
    fn the_reply_box_is_a_share_of_the_output() {
        let bar = panel::bar_height();
        for (output, share) in [(1080u32, 410u32), (1440, 547), (600, 228)] {
            assert_eq!(room_for_replies(Some(output)), share, "38 % of {output}");
            // The window is the bar, the gap between the cards and that box.
            let (_, h) = size_for(Phase::Answering, Some(output));
            assert_eq!(
                h,
                bar + render::SEPARATION + share,
                "the panel is its bar, the gap between the cards and the replies"
            );
        }
        // The rows a frame may draw bound it on a tall screen, however tall it is.
        assert_eq!(
            room_for_replies(Some(8000)),
            layout::MAX_ROWS as u32 * bar,
            "never more than the rows the frame may draw"
        );
        // And a screen too short to hold a share still gets a box, not a panic.
        assert_eq!(room_for_replies(Some(0)), 0);
    }

    /// `Ctrl+T` opens the prefix, and the prefix is one key wide: `[` goes to copy
    /// mode, everything else leaves it and does nothing.
    #[test]
    fn the_prefix_takes_one_key_and_opens_copy_mode() {
        assert_eq!(
            key_action(Keysym::t, false, true, true),
            Key::Prefix,
            "Ctrl+T is the prefix"
        );
        assert_eq!(prefix_action(Keysym::bracketleft), Prefix::Copy);
        for keysym in [
            Keysym::z,
            Keysym::Escape,
            Keysym::Return,
            Keysym::bracketright,
        ] {
            assert_eq!(prefix_action(keysym), Prefix::None, "{keysym:?}");
        }
    }

    /// The copy-mode table, key by key: what moves, what selects, what leaves.
    #[test]
    fn the_copy_mode_table_is_tmuxs_where_this_panel_has_something_to_move() {
        let page = 8;
        assert_eq!(copy_key(Keysym::h, false, page), Some(CopyKey::Left));
        assert_eq!(copy_key(Keysym::l, false, page), Some(CopyKey::Right));
        assert_eq!(copy_key(Keysym::_0, false, page), Some(CopyKey::LineStart));
        assert_eq!(
            copy_key(Keysym::dollar, false, page),
            Some(CopyKey::LineEnd)
        );
        assert_eq!(copy_key(Keysym::j, false, page), Some(CopyKey::Row(1)));
        assert_eq!(copy_key(Keysym::k, false, page), Some(CopyKey::Row(-1)));
        assert_eq!(copy_key(Keysym::w, false, page), Some(CopyKey::WordForward));
        assert_eq!(copy_key(Keysym::b, false, page), Some(CopyKey::WordBack));
        assert_eq!(
            copy_key(Keysym::d, true, page),
            Some(CopyKey::Row(4)),
            "half a page"
        );
        assert_eq!(copy_key(Keysym::u, true, page), Some(CopyKey::Row(-4)));
        assert_eq!(copy_key(Keysym::f, true, page), Some(CopyKey::Row(8)));
        assert_eq!(copy_key(Keysym::b, true, page), Some(CopyKey::Row(-8)));
        assert_eq!(copy_key(Keysym::Next, false, page), Some(CopyKey::Row(8)));
        assert_eq!(copy_key(Keysym::Prior, false, page), Some(CopyKey::Row(-8)));
        assert_eq!(copy_key(Keysym::g, false, page), Some(CopyKey::First));
        assert_eq!(copy_key(Keysym::G, false, page), Some(CopyKey::Last));
        assert_eq!(copy_key(Keysym::space, false, page), Some(CopyKey::Select));
        assert_eq!(copy_key(Keysym::v, false, page), Some(CopyKey::Select));
        assert_eq!(copy_key(Keysym::Return, false, page), Some(CopyKey::Copy));
        assert_eq!(copy_key(Keysym::y, false, page), Some(CopyKey::Copy));
        assert_eq!(copy_key(Keysym::q, false, page), Some(CopyKey::Quit));
        assert_eq!(copy_key(Keysym::Escape, false, page), Some(CopyKey::Cancel));
        // A letter that is not in the table types nothing and moves nothing.
        for keysym in [Keysym::a, Keysym::x, Keysym::z, Keysym::slash] {
            assert_eq!(copy_key(keysym, false, page), None, "{keysym:?}");
        }
    }

    /// The reported bug: a pointer passing over the replies selected text, because
    /// the "I am selecting" flag outlived the press that set it. A motion only acts
    /// while the button is down, and a hover is no gesture at all.
    #[test]
    fn a_hover_selects_nothing() {
        let down = Some((10.0, 10.0));
        assert_eq!(motion_with(None), Motion::Hover, "a stale selecting flag");
        assert_eq!(motion_with(down), Motion::Extend, "the button is down");
    }

    /// The copy note counts the lines that went out, and a single line says so.
    #[test]
    fn the_copy_note_counts_the_lines_it_took() {
        let f = PanelFont::load(font::SIZE).expect("system font available");
        let blocks = layout::Blocks::new(&f);
        let mut t = crate::transcript::Transcript::default();
        t.ask("q", &f);
        t.delta("one\ntwo\nthree", &f, 400.0);
        t.finish_answer(&f);
        let rows = t.rows(blocks);
        let one = panel::Selection {
            anchor: (0, 0),
            cursor: (0, 2),
        };
        assert_eq!(copied_lines(&one, &rows), 1, "part of one line is one line");
        let three = panel::Selection {
            anchor: (0, 0),
            cursor: (2, 3),
        };
        assert_eq!(copied_lines(&three, &rows), 3, "three lines");
    }

    /// Nothing in the key table moves the window: the compositor's window rules and
    /// its own move binding are the only things that do. `Alt` is still the copy.
    #[test]
    fn no_key_moves_the_window() {
        for (alt, ctrl) in [(true, false), (true, true), (false, true)] {
            for keysym in [Keysym::h, Keysym::j, Keysym::k, Keysym::l] {
                let action = key_action(keysym, alt, ctrl, true);
                assert!(
                    action == Key::Ignore,
                    "{keysym:?} alt={alt} ctrl={ctrl} is {action:?}"
                );
            }
        }
        // Unmodified they type, which is what a letter is for.
        assert_eq!(key_action(Keysym::h, false, false, true), Key::Text);
        assert_eq!(key_action(Keysym::h, true, false, true), Key::Ignore);
        assert_eq!(key_action(Keysym::j, true, true, true), Key::Ignore);
        assert_eq!(key_action(Keysym::c, true, false, true), Key::Copy);
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
        // Both copies and the paste are deliberate chords, and nothing else.
        assert_eq!(key_action(Keysym::c, true, false, true), Key::Copy);
        assert_eq!(key_action(Keysym::c, false, true, true), Key::Copy);
        assert_eq!(key_action(Keysym::v, false, true, true), Key::Paste);
    }

    #[test]
    fn only_a_copy_chord_reaches_the_clipboard() {
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
                        assert_eq!(k, Keysym::c, "only a c copies");
                        assert!(alt ^ ctrl, "and only with one modifier: {alt} {ctrl}");
                    }
                    if key_action(k, alt, ctrl, true) == Key::Paste {
                        assert_eq!((k, alt, ctrl), (Keysym::v, false, true), "only Ctrl+V");
                    }
                }
            }
        }
    }

    /// A follow-up keeps the box it was asked from: the shape a phase is drawn as is
    /// the phase's own, except that a request keeps the answering panel up. Shrinking
    /// to the bar and growing back on every follow-up read as the panel folding shut.
    #[test]
    fn a_request_keeps_the_box_it_was_asked_from() {
        assert_eq!(
            shape_after(panel::State::Bar, Phase::Thinking),
            panel::State::Bar
        );
        assert_eq!(
            shape_after(panel::State::Replies, Phase::Thinking),
            panel::State::Replies,
            "a follow-up does not fold the panel shut"
        );
        assert_eq!(
            shape_after(panel::State::Replies, Phase::Answering),
            panel::State::Replies
        );
        assert_eq!(
            shape_after(panel::State::Bar, Phase::Answering),
            panel::State::Replies
        );
        assert_eq!(
            shape_after(panel::State::Bar, Phase::Idle),
            panel::State::Bar
        );
    }

    /// A selection is finer than a line: the first and last rows are cut at the
    /// characters the pointer named, and the rows between are whole.
    #[test]
    fn a_selection_is_cut_at_the_characters_it_names() {
        let sel = panel::Selection {
            anchor: (1, 4),
            cursor: (3, 2),
        };
        assert_eq!(sel.columns(0, 10), None, "above the selection");
        assert_eq!(
            sel.columns(1, 10),
            Some((4, 10)),
            "from the anchor to the end"
        );
        assert_eq!(sel.columns(2, 10), Some((0, 10)), "whole rows between");
        assert_eq!(sel.columns(3, 10), Some((0, 2)), "up to the cursor");
        assert_eq!(sel.columns(4, 10), None, "below the selection");
        // Dragged the other way, the same characters.
        let back = panel::Selection {
            anchor: (3, 2),
            cursor: (1, 4),
        };
        for row in 0..5 {
            assert_eq!(back.columns(row, 10), sel.columns(row, 10), "row {row}");
        }
        // A click selects nothing: an empty range is no selection at all.
        let click = panel::Selection {
            anchor: (2, 3),
            cursor: (2, 3),
        };
        assert_eq!(click.columns(2, 10), None);
    }

    /// A configured size reaches the panel's own geometry: the bar, the box under it
    /// and the row cap are all expressed in rows of the configured text.
    #[test]
    fn the_panel_follows_the_configured_font_size() {
        let size = font::size();
        assert_eq!(size, font::SIZE, "no configuration in a test run");
        let bar = panel::bar_height();
        assert_eq!(bar, font::row_h_for(size), "the bar is one row of the text");
        let (_, h) = size_for(Phase::Answering, Some(1080));
        assert_eq!(h, bar + render::SEPARATION + room_for_replies(Some(1080)));
        // The room is a share of the output clamped by the rows a frame may draw, so
        // a tall enough output stops at the row cap rather than at the share.
        assert_eq!(room_for_replies(Some(3000)), 1140, "38 % of 3000");
        assert_eq!(
            room_for_replies(Some(6000)),
            layout::MAX_ROWS as u32 * bar,
            "the row cap is expressed in rows of this size"
        );
        assert!(font::row_h_for(30.0) > bar, "a larger size is a taller row");
    }

    #[test]
    fn a_modified_key_does_not_type_text() {
        assert_eq!(key_action(Keysym::x, false, true, true), Key::Ignore);
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
