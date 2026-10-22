//! `zwp_text_input_v3`: what makes the panel usable for anyone who types
//! through an input method.
//!
//! `smithay-client-toolkit` 0.21 has no client-side text input (only the
//! input-method side), so the protocol is spoken directly here. The state
//! machine is pure and tested; the protocol calls around it are thin.
//!
//! The rule when typing: with a text-input object the compositor delivers text
//! as `commit_string`, and raw keysyms are only used when no text-input manager
//! was advertised at all. Both paths exist because a compositor with the
//! protocol but no running input method sends nothing through it — verified
//! behaviour of wlroots, and the reason the fallback cannot simply be dropped.

use wayland_client::{
    Connection, Dispatch, Proxy, QueueHandle,
    globals::GlobalList,
    protocol::{wl_seat::WlSeat, wl_surface::WlSurface},
};
use wayland_protocols::wp::text_input::zv3::client::{
    zwp_text_input_manager_v3::ZwpTextInputManagerV3,
    zwp_text_input_v3::{Event as TextInputEvent, ZwpTextInputV3},
};

/// What the input line should do with a protocol event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action {
    /// Replace the pre-edit with this text (shown, not committed).
    Preedit(String),
    /// The user committed this text: it goes into the input line.
    Commit(String),
    /// Delete up to this many characters before (and after) the caret.
    DeleteSurrounding { before: u32, after: u32 },
    /// End of a batch: the panel acknowledges and repaints.
    RoundTrip,
    /// Nothing to do.
    Ignore,
}

/// The text-input state: the pre-edit string being composed, if any.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Ime {
    /// Text the input method is currently composing. Raw keysyms are ignored
    /// while this is non-empty, because the input method is handling them.
    pub preedit: String,
    /// Whether an input method has spoken on this text input at all.
    live: bool,
    /// Text keys seen since the input method last said anything. Held for the rest
    /// of the event round: if the input method speaks in it, the keys were part of
    /// its composition; if it stays silent, they are the panel's own text.
    held: String,
}

impl Ime {
    /// Whether typing should come from raw keysyms rather than the input method.
    ///
    /// A text-input manager is not an input method: sway binds
    /// `zwp_text_input_v3` on every seat and still sends nothing when no
    /// fcitx/ibus is connected, so only a commit, a pre-edit or a done proves
    /// there is something on the other end to type for us.
    pub fn wants_raw_keys(&self) -> bool {
        !self.live
    }

    /// Hold a text key that the input method might claim. Only meaningful once an
    /// input method has been heard from at all; before that the keys are typed
    /// straight away.
    pub fn hold(&mut self, text: &str) {
        self.held.push_str(text);
    }

    /// What the keys held in this round amount to: `None` while the panel is
    /// waiting for the input method, `Some` when it has stayed silent and the text
    /// is the panel's to insert.
    pub fn flush(&mut self) -> Option<String> {
        (!self.held.is_empty()).then(|| std::mem::take(&mut self.held))
    }

    /// What to do about one protocol event.
    pub fn handle(&mut self, event: TextInputEvent) -> Action {
        // Whatever the input method says, the keys held so far are its business:
        // it is composing them, or it has just committed what they became.
        self.held.clear();
        self.handle_event(event)
    }

    fn handle_event(&mut self, event: TextInputEvent) -> Action {
        match event {
            TextInputEvent::PreeditString { text, .. } => {
                self.live = true;
                self.preedit = text.clone().unwrap_or_default();
                match text {
                    Some(t) => Action::Preedit(t),
                    None => Action::Ignore,
                }
            }
            TextInputEvent::CommitString { text } => {
                self.live = true;
                self.preedit.clear();
                Action::Commit(text.unwrap_or_default())
            }
            TextInputEvent::DeleteSurroundingText {
                before_length,
                after_length,
            } => {
                self.live = true;
                Action::DeleteSurrounding {
                    before: before_length,
                    after: after_length,
                }
            }
            TextInputEvent::Done { .. } => {
                self.live = true;
                Action::RoundTrip
            }
            _ => Action::Ignore,
        }
    }
}

/// Binds the manager, if the compositor has one. `None` means the panel types
/// with raw keysyms and says so, rather than appearing to accept input it cannot
/// finish without an input method.
pub fn bind(globals: &GlobalList, qh: &QueueHandle<crate::State>) -> Option<ZwpTextInputManagerV3> {
    // Version 1 is enough: v2 adds actions and a language event the panel does
    // not use, and asking for less than the compositor offers is always legal.
    globals
        .bind::<ZwpTextInputManagerV3, crate::State, ()>(qh, 1..=1, ())
        .ok()
}

/// Create the panel's text-input object for `surface` on `seat`.
///
/// Deliberately does not enable it: enabling is what a focused surface does, and
/// river answers an enable from an unfocused client with "client requested to
/// enable text input without focus, ignoring request" (and then rejects the
/// commit that followed it). The focus handler enables it instead.
pub fn attach(
    manager: &ZwpTextInputManagerV3,
    seat: &WlSeat,
    surface: &WlSurface,
    qh: &QueueHandle<crate::State>,
) -> ZwpTextInputV3 {
    // The object is not tied to a surface: `enter`/`leave` say which surface
    // it is typing into, which is what the panel wants when it is unminimised.
    let _ = surface;
    manager.get_text_input(seat, qh, ())
}

/// The panel's `Dispatch` side of the protocol: events go to the input line.
impl Dispatch<ZwpTextInputManagerV3, ()> for crate::State {
    fn event(
        _state: &mut Self,
        _proxy: &ZwpTextInputManagerV3,
        _event: <ZwpTextInputManagerV3 as Proxy>::Event,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        // The manager has no events.
    }
}

impl Dispatch<ZwpTextInputV3, ()> for crate::State {
    fn event(
        state: &mut Self,
        _proxy: &ZwpTextInputV3,
        event: TextInputEvent,
        _data: &(),
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        state.on_text_input(event);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(e: TextInputEvent) -> TextInputEvent {
        e
    }

    /// The reported defect: after Chinese, switching the input method off left the
    /// panel unable to type letters or digits, because "an input method exists" was
    /// latched once and never revisited.
    #[test]
    fn a_key_the_input_method_ignores_is_typed() {
        let mut ime = Ime::default();
        // An input method has spoken before, so keys are held rather than typed.
        ime.handle(event(TextInputEvent::CommitString {
            text: Some("你好".into()),
        }));
        assert!(!ime.wants_raw_keys());

        // Now it says nothing about these keys: they are the panel's text.
        ime.hold("n");
        ime.hold("7");
        assert_eq!(ime.flush().as_deref(), Some("n7"));
        assert_eq!(ime.flush(), None, "a flush happens once per round");
    }

    #[test]
    fn held_keys_are_dropped_when_the_input_method_answers() {
        let mut ime = Ime::default();
        ime.handle(event(TextInputEvent::Done { serial: 1 }));
        ime.hold("n");
        // A pre-edit in the same round: the key was the input method's.
        let action = ime.handle(event(TextInputEvent::PreeditString {
            text: Some("n".into()),
            cursor_begin: 1,
            cursor_end: 1,
        }));
        assert_eq!(action, Action::Preedit("n".into()));
        assert_eq!(ime.flush(), None, "the key must not be typed as well");

        // Same for a commit: only the committed text lands.
        ime.hold("i");
        assert_eq!(
            ime.handle(event(TextInputEvent::CommitString {
                text: Some("你".into())
            })),
            Action::Commit("你".into())
        );
        assert_eq!(ime.flush(), None);
    }

    #[test]
    fn a_commit_replaces_the_preedit() {
        let mut ime = Ime::default();
        assert_eq!(
            ime.handle(event(TextInputEvent::PreeditString {
                text: Some("zhong".into()),
                cursor_begin: 5,
                cursor_end: 5,
            })),
            Action::Preedit("zhong".into())
        );
        assert_eq!(ime.preedit, "zhong");
        assert_eq!(
            ime.handle(event(TextInputEvent::CommitString {
                text: Some("中".into())
            })),
            Action::Commit("中".into())
        );
        assert!(ime.preedit.is_empty(), "a commit ends the composition");
    }

    #[test]
    fn a_long_cjk_commit_arrives_whole() {
        // A composed Chinese sentence, committed in one event: nothing may be
        // dropped, and the input buffer holds exactly what was composed.
        let sentence =
            "这是一段用输入法合成的问题，需要完整地进入输入行，不能丢字，也不能被截断。".repeat(6);
        assert!(sentence.chars().count() > 200);
        let mut ime = Ime::default();
        let mut input = crate::layout::Input::default();
        assert_eq!(
            ime.handle(event(TextInputEvent::PreeditString {
                text: Some("zhe shi yi duan".into()),
                cursor_begin: 15,
                cursor_end: 15,
            })),
            Action::Preedit("zhe shi yi duan".into())
        );
        let action = ime.handle(event(TextInputEvent::CommitString {
            text: Some(sentence.clone()),
        }));
        if let Action::Commit(text) = action {
            input.insert(&text);
        }
        assert_eq!(input.text(), sentence);
        assert!(ime.preedit.is_empty());
    }

    #[test]
    fn a_null_commit_commits_nothing() {
        let mut ime = Ime::default();
        assert_eq!(
            ime.handle(event(TextInputEvent::CommitString { text: None })),
            Action::Commit(String::new())
        );
    }

    #[test]
    fn a_done_event_is_the_end_of_a_batch() {
        let mut ime = Ime::default();
        assert_eq!(
            ime.handle(event(TextInputEvent::Done { serial: 7 })),
            Action::RoundTrip
        );
    }

    #[test]
    fn surrounding_deletion_is_reported_as_lengths() {
        let mut ime = Ime::default();
        assert_eq!(
            ime.handle(event(TextInputEvent::DeleteSurroundingText {
                before_length: 2,
                after_length: 1
            })),
            Action::DeleteSurrounding {
                before: 2,
                after: 1
            }
        );
    }

    #[test]
    fn a_bound_but_silent_text_input_still_types_raw_keys() {
        // A compositor that advertises text-input but has no input method
        // connected answers nothing, so it must not be trusted with the keys.
        let ime = Ime::default();
        assert!(ime.wants_raw_keys(), "no input method means raw keysyms");
    }

    #[test]
    fn an_input_method_that_speaks_takes_the_keys() {
        let mut ime = Ime::default();
        ime.handle(event(TextInputEvent::Done { serial: 1 }));
        assert!(!ime.wants_raw_keys(), "a done means something is typing");

        // A commit alone is just as good a proof.
        let mut ime = Ime::default();
        ime.handle(event(TextInputEvent::CommitString {
            text: Some("中".into()),
        }));
        assert!(!ime.wants_raw_keys());
    }

    #[test]
    fn a_v2_event_the_panel_does_not_use_is_ignored() {
        let mut ime = Ime::default();
        // Version 2 events can still turn up from a compositor that offers more
        // than the panel asked for; they are not guessed at.
        assert_eq!(
            ime.handle(event(TextInputEvent::Language {
                language: "en".into()
            })),
            Action::Ignore
        );
    }
}
