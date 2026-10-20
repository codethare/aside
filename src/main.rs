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
use smithay_client_toolkit::reexports::calloop::{EventLoop, Interest, Mode};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::registry_handlers;
use smithay_client_toolkit::seat::keyboard::{
    KeyEvent, KeyboardHandler, Keysym, Modifiers, RawModifiers,
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
use wayland_client::protocol::{wl_keyboard, wl_output, wl_seat, wl_shm, wl_surface};
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
/// Gap between the top edge and the panel, and the room kept below it.
const TOP_MARGIN: i32 = 32;
const BOTTOM_ROOM: i32 = 32;
// The shadow margin hangs into the screen edge, so the surface margin is the
// panel's visible gap minus that margin.
const _: () = assert!(TOP_MARGIN >= render::SHADOW as i32);
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
        max_w,
    );
    transcript.delta(
        "It means the call would block: the resource is momentarily unavailable rather than \
         broken. Retry it, and read a repeated EAGAIN under load as backpressure. \
         同一句话换成中文时走的是 CJK 字体回退。",
        &font,
        max_w,
    );
    transcript.finish_answer(&font);
    transcript.ask("and if it never clears?", &font, max_w);
    transcript.delta("Then the peer is gone", &font, max_w);
    transcript.cancel("cancelled", &font, max_w);
    transcript.fail("the provider refused the request (401)", &font, max_w);
    // The panel is as tall as its blocks, so the transcript comes first.
    let (pw, ph) = (
        WIDTH,
        panel::height_for(transcript.total_height(), &font, 1),
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
        false,
        true,
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
    layer.set_anchor(Anchor::TOP);
    layer.set_margin(shadow_margin(), 0, 0, 0);
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
                Generic::new(listener, Interest::READ, Mode::Level),
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
        let timeout = if state.phase == Phase::Thinking {
            // The blink advances every 200 ms; wake often enough to show it.
            Duration::from_millis(50)
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

/// The layer surface carries the panel plus its shadow margin on both sides.
fn surface_size(panel: (u32, u32)) -> (u32, u32) {
    (panel.0 + 2 * render::SHADOW, panel.1 + 2 * render::SHADOW)
}

/// The top margin leaves the panel `TOP_MARGIN` below the screen edge: the
/// shadow margin already occupies part of that gap.
const fn shadow_margin() -> i32 {
    TOP_MARGIN - render::SHADOW as i32
}

/// Horizontal margin that centres the surface on the output.
fn left_margin(output_w: Option<u32>, surface_w: u32) -> i32 {
    output_w
        .map(|ow| ((ow as i64 - surface_w as i64) / 2).max(0) as i32)
        .unwrap_or(0)
}

/// The logical size a phase wants: idle is the bar, thinking is a ball twice the
/// bar tall, answering is the bar plus the transcript's own height, capped so the
/// panel never leaves the output. `content` is the transcript's height in logical
/// pixels — its blocks are laid out in buffer pixels, so the caller divides by the
/// scale. These are the panel's sizes; `surface_size` adds the shadow margin.
/// Pure, so the spec's numbers are testable without a surface.
fn size_for(phase: Phase, content: u32, output_h: Option<u32>) -> (u32, u32) {
    let bar = panel::bar_height();
    match phase {
        Phase::Idle => (WIDTH, bar),
        Phase::Thinking => (2 * bar, 2 * bar),
        Phase::Answering => {
            let cap = output_h
                .unwrap_or(FALLBACK_OUTPUT_H)
                .saturating_sub((TOP_MARGIN + BOTTOM_ROOM) as u32);
            let rows = layout::MAX_ROWS as u32 * bar;
            let want = bar + 1 + content.min(rows);
            (WIDTH, want.min(cap).max(bar))
        }
    }
}

/// Which of the four squares is lit, from elapsed milliseconds rather than from
/// a frame count, so a late frame lands on the right square.
fn blink_at(elapsed_ms: u128) -> usize {
    const STEP_MS: u128 = 200;
    ((elapsed_ms / STEP_MS) % 4) as usize
}

/// The panel's three states. Their sizes are fixed by the spec; this is the
/// state machine behind them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    /// The bar alone: nothing asked yet.
    Idle,
    /// A question is out and nothing has come back: the ball.
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
    /// When the current phase began: the ball's blink is read off this clock.
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
    /// Join the font thread once. Until then there is no text, and no frame.
    fn target_size(&self) -> (u32, u32) {
        // The blocks are buffer pixels; the panel's size is logical.
        size_for(
            self.phase,
            self.transcript.total_height().div_ceil(self.scale.max(1)),
            self.output_h,
        )
    }

    fn blink_index(&self) -> usize {
        blink_at(self.phase_start.elapsed().as_millis())
    }

    /// Give the surface the size its phase wants, centred on the output, and
    /// re-wrap once if the width changed.
    fn apply_size(&mut self) {
        let (w, h) = self.target_size();
        let (sw, sh) = surface_size((w, h));
        self.layer
            .set_margin(shadow_margin(), 0, 0, left_margin(self.output_w, sw));
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
        chat::answer_width(self.width * self.scale, self.scale)
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
        transcript.ask(&question, font, max_w);
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
        self.phase = if started {
            Phase::Thinking
        } else {
            Phase::Answering
        };
        self.phase_start = std::time::Instant::now();
        self.apply_size();
        self.dirty = true;
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
            self.phase = Phase::Answering;
            self.apply_size();
            self.dirty = true;
        } else {
            self.exit = true;
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
            // The first text ends the ball; every later row may grow the panel.
            self.phase = Phase::Answering;
            self.apply_size();
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
        let lit = if self.phase == Phase::Thinking {
            Some(self.blink_index())
        } else {
            None
        };
        let stride = w as i32 * 4;
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
        let streaming = self.request.is_some();
        match lit {
            Some(lit) => panel::draw_ball(canvas, w, h, lit, self.scale),
            None => panel::draw(
                canvas,
                w,
                h,
                self.font.as_ref().expect("checked above"),
                &self.transcript,
                &self.input,
                &self.viewport,
                Some(self.highlight),
                streaming,
                true,
                self.scale,
            ),
        }
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
        let text = event.utf8.clone();
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
        // With an input method, text arrives through the protocol; without one,
        // the keysyms are the only text there is.
        if action == Key::Text && self.ime.wants_raw_keys() && self.ime.preedit.is_empty() {
            if let Some(text) = text {
                self.input.insert(&text);
                self.after_typing();
            }
            return;
        }
        match action {
            Key::Send => self.send(),
            Key::Cancel => self.cancel(),
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
        let x = (mo + PAD * s + render::LABEL_RAIL_W * s) as f32
            + render::PROMPT_GAP * s as f32
            + render::measure(font, before, max_w);
        // Both offsets are byte offsets into the surrounding text.
        let caret = self.input.caret() as i32;
        // The bar is the first row, inside the shadow margin.
        let bar_top = mo;
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
        assert_eq!(size_for(Phase::Idle, 0, Some(1080)), (640, bar));
        assert_eq!(size_for(Phase::Thinking, 0, Some(1080)), (60, 60));
        // Answering: the bar plus the transcript's own height, which is blocks
        // and gaps rather than a count of rows.
        assert_eq!(
            size_for(Phase::Answering, 4 * bar, Some(1080)),
            (640, bar + 1 + 4 * bar)
        );
        // A card's padding counts: 8 px above and below make the panel taller
        // than its text rows alone.
        assert_eq!(
            size_for(Phase::Answering, 3 * bar + 16, Some(1080)),
            (640, bar + 1 + 3 * bar + 16)
        );
        // The cap is the output less the margin above and the room below.
        assert_eq!(
            size_for(Phase::Answering, 1000 * bar, Some(1080)),
            (640, 1080 - 64)
        );
        // Before the output reports, a 1080-pixel one is assumed.
        assert_eq!(
            size_for(Phase::Answering, 1000 * bar, None),
            (640, 1080 - 64)
        );
        // An empty transcript still leaves the bar plus its divider.
        assert_eq!(size_for(Phase::Answering, 0, Some(1080)).1, bar + 1);
        // A tiny output cannot make the panel taller than its bar or negative.
        assert_eq!(size_for(Phase::Answering, 10 * bar, Some(40)).1, bar);
    }

    #[test]
    fn the_blink_comes_from_the_clock() {
        // 200 ms a step, 800 ms a cycle, and a late frame lands where the clock
        // says rather than where the frame count would.
        assert_eq!(blink_at(0), 0);
        assert_eq!(blink_at(199), 0);
        assert_eq!(blink_at(200), 1);
        assert_eq!(blink_at(450), 2);
        assert_eq!(blink_at(799), 3);
        assert_eq!(blink_at(800), 0);
        assert_eq!(blink_at(5_000), 1, "25 steps, wrapped");
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
