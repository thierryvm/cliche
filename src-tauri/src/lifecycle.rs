//! When Cliche ends, and which window a second launch brings back.
//!
//! # THE GHOST PROCESS OF 7 SEPTEMBER 2026
//!
//! Cliche v0.1.0, installed and run: the user closed the main window, launched
//! Cliche again, and was told by a red banner that Ctrl + Maj + 2 was held by
//! another application. It was - by Cliche. Two `cliche.exe` were running. The
//! first had no window on screen and still owned the combination; the second was
//! the one being looked at.
//!
//! The cause was read in the vendored runtime the day it was diagnosed,
//! `tauri-runtime-wry-2.11.4/src/lib.rs:4310-4325`: on a
//! `TaoWindowEvent::Destroyed` the runtime takes the window out of its map and
//! then emits `RunEvent::ExitRequested` - and sets `ControlFlow::Exit` - ONLY
//! when `windows.0.borrow().is_empty()`. The veil is built at startup and left
//! hidden (`veil::create`), and a hidden window is a window like any other in
//! that map. Closing `main` therefore never empties it, no exit is ever
//! requested, and the process goes on living with nothing to show for itself.
//!
//! That version is the one this build really uses: `Cargo.lock` pins
//! `tauri-runtime-wry 2.11.4` under `tauri 2.11.5`. The READING of those lines is
//! second-hand here - it was done at diagnosis time, in the cargo registry, which
//! is not reachable from where this file was written - so treat the line numbers
//! as a pointer to check rather than as something re-verified today.
//!
//! Two rules come out of that, and they are the whole of this module:
//!
//! 1. **Closing the main window ends the application.** The veil is not a reason
//!    to stay alive: it exists to serve the main window, it is never what the
//!    user is looking at when they decide they are done, and it holds no state
//!    anybody would come back for.
//! 2. **A second launch hands the screen back to the first process.** Which is
//!    the other half of the same defect: without it, the answer to an invisible
//!    Cliche is a second Cliche that cannot have the shortcut.
//!
//! # Why the rules are HERE and not in the closures that apply them
//!
//! `on_window_event` and the single-instance callback both need a running event
//! loop, so neither can be exercised by `cargo test` - the same constraint that
//! put `shortcut::change` behind a `Registrar` trait and `ipc::ensure_from`
//! behind a `&str` instead of a `Webview`. What `lib.rs` holds is therefore the
//! WIRING; what this file holds is the decision, and the decision is what the
//! tests below can turn red.

use tauri::{AppHandle, Manager, Runtime, WebviewWindow};

/// Label of the main window, as declared in `tauri.conf.json`.
///
/// MOVED HERE FROM `ipc.rs` on 7 September 2026, which re-exports it so that
/// every existing caller goes on reading ONE value. It stopped being an IPC fact
/// that day: the same label now also decides which window's closing ends the
/// process and which window a second launch brings back, and a second copy of it
/// would be a second chance to drift away from the configuration.
///
/// `the_main_window_label_is_the_one_the_configuration_declares` is what holds
/// it against the file that really creates the window.
pub const MAIN_WINDOW_LABEL: &str = "main";

/// Whether closing the window with this label must end the application.
///
/// # It asks about the LABEL, deliberately, and not about how many are left
///
/// "Was that the last window?" is precisely the question the runtime already
/// answers wrongly on our behalf - the veil is a window it can count and the
/// user can never see, so the answer is always no. Re-asking it here in another
/// form would reproduce the defect rather than fix it. What ends Cliche is the
/// window the user closes, and that is a name.
///
/// An exact comparison, for the reason [`crate::ipc::is_from`] gives: labels are
/// chosen by this application, and a loose match is how a rule like this one
/// quietly starts firing on the wrong window.
pub fn closing_ends_the_application(label: &str) -> bool {
    label == MAIN_WINDOW_LABEL
}

/// The window a second launch brings back in front of the user.
///
/// A function rather than a second constant, so that what the test pins is an
/// ANSWER: an alias of [`MAIN_WINDOW_LABEL`] under another name would be true by
/// definition and could never be caught being wrong.
pub fn window_a_second_instance_raises() -> &'static str {
    MAIN_WINDOW_LABEL
}

/// The line one failed step of a raise produces.
///
/// Pure, so its wording is under test. It says WHICH path failed and what the
/// runtime answered: this line is printed by the process the user cannot see,
/// in the terminal of the process they can, and a failure nobody can attribute
/// is a failure nobody can act on.
fn raise_failure(step: &str, reason: &str) -> String {
    format!(
        "[cliche] lifecycle: a second launch could not {step} ({reason}); the \
         `{MAIN_WINDOW_LABEL}` window may still be out of sight"
    )
}

/// What bringing a window back in front of the user is made of.
///
/// A trait, for the reason [`crate::shortcut::Registrar`] is one: the SEQUENCE
/// is the decision - which of the steps runs, in which order, and what happens
/// when one of them is refused - and none of it can be exercised through a real
/// window, since building one needs a running event loop. So the sequence takes
/// its window as an argument, and a test hands it one that records every call.
///
/// `&mut self` throughout, although the real window needs none of it: it is what
/// lets the fake keep its log in a plain `Vec`, and the same trade
/// [`crate::shortcut::Registrar`] makes.
pub trait Front {
    /// Whether the window is minimised right now.
    fn is_minimized(&mut self) -> Result<bool, String>;
    /// Takes the window out of the taskbar and back onto the screen.
    fn unminimize(&mut self) -> Result<(), String>;
    /// Makes a hidden window visible.
    fn show(&mut self) -> Result<(), String>;
    /// Gives the window the keyboard focus.
    fn set_focus(&mut self) -> Result<(), String>;
}

/// Brings a window back in front of the user, and says what did not go through.
///
/// # The order, and why the first step is conditional
///
/// 1. MINIMISED is asked first, and the window is restored only when the answer
///    is yes. Unconditional, that restore would also run on a MAXIMISED window,
///    and restoring one of those is what takes its maximisation away: a second
///    launch that resized the user's window would be a new defect put in place
///    of the old one. REASONED, NOT MEASURED - nothing here has observed what
///    Windows does with a restore on a maximised window; what is certain is that
///    asking first costs one call and removes the question.
/// 2. SHOWN, because the window may be hidden rather than minimised.
///    `launch::hide_main_window` takes it off the screen for the length of a
///    tile capture, and a `show()` is the only way back.
/// 3. FOCUSED last. Focus given to a window that is still minimised or still
///    hidden lands on nothing.
///
/// Every step is attempted whatever the ones before it did. A window that could
/// not be un-minimised is still worth showing, and stopping at the first refusal
/// would leave the user with the ghost this module exists to end - a Cliche that
/// is running and invisible.
///
/// Returns one line per failed step, in the order they were attempted. Nothing
/// is printed here: this is the decision, and its wording is under test.
pub fn bring_to_front(window: &mut impl Front) -> Vec<String> {
    let mut failures = Vec::new();

    // A question that could not be answered is taken as YES, and that is a
    // decision rather than a default. The whole point of this path is a user who
    // cannot see Cliche; leaving a minimised window minimised is the symptom
    // itself, while un-maximising one that was not minimised is an annoyance.
    // The cheaper of the two wrongs is chosen, out loud.
    let minimized = match window.is_minimized() {
        Ok(minimized) => minimized,
        Err(reason) => {
            failures.push(raise_failure(
                "find out whether the window was minimised",
                &reason,
            ));
            true
        }
    };

    if minimized {
        if let Err(reason) = window.unminimize() {
            failures.push(raise_failure("take the window out of the taskbar", &reason));
        }
    }

    if let Err(reason) = window.show() {
        failures.push(raise_failure("make the window visible", &reason));
    }

    if let Err(reason) = window.set_focus() {
        failures.push(raise_failure("give the window the focus", &reason));
    }

    failures
}

/// The real window, behind the trait above.
///
/// Deliberately thin: everything it could get wrong is a decision, and the
/// decisions are in [`bring_to_front`], where they are tested. What is left here
/// is one call per step and one error turned into a string.
impl<R: Runtime> Front for WebviewWindow<R> {
    fn is_minimized(&mut self) -> Result<bool, String> {
        WebviewWindow::is_minimized(self).map_err(|error| error.to_string())
    }

    fn unminimize(&mut self) -> Result<(), String> {
        WebviewWindow::unminimize(self).map_err(|error| error.to_string())
    }

    fn show(&mut self) -> Result<(), String> {
        WebviewWindow::show(self).map_err(|error| error.to_string())
    }

    fn set_focus(&mut self) -> Result<(), String> {
        WebviewWindow::set_focus(self).map_err(|error| error.to_string())
    }
}

/// Hands the screen back to this process, when a SECOND Cliche is launched.
///
/// Called from the single-instance plugin's callback, which runs in the FIRST
/// process while the second one stops. Nothing here may panic: it runs on
/// whatever thread that plugin listens on, and a panic there would take down the
/// only process that still has the user's shortcut - turning a duplicate launch
/// into a lost application, which is worse than the defect being fixed.
///
/// That is also why a missing window is a printed line rather than an error
/// returned to nobody: there is no caller left to report to.
///
/// # ONE INTERACTION IS KNOWN AND DELIBERATELY NOT HANDLED
///
/// `launch::hide_main_window` takes the main window off the screen for the
/// length of a tile capture, and this function would put it straight back. A
/// second launch landing inside that window - `RECOMPOSE_SETTLE`, 120 ms - would
/// therefore leave Cliche in the frozen frame. It is left alone on purpose:
/// suppressing the raise would mean a second launch that answers with nothing at
/// all, which is the defect this whole module exists to end, and the debt
/// `MainWindowClaim` holds still pays itself afterwards. NOT OBSERVED - the race
/// is read from the two code paths, and nobody has provoked it.
pub fn raise_main_window<R: Runtime>(app: &AppHandle<R>) {
    let label = window_a_second_instance_raises();

    let Some(mut window) = app.get_webview_window(label) else {
        eprintln!(
            "[cliche] lifecycle: a second launch found no `{label}` window to bring back. This \
             process is running with nothing to show for itself - the state the window-close rule \
             in this module exists to prevent."
        );
        return;
    };

    for line in bring_to_front(&mut window) {
        eprintln!("{line}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::veil::VEIL_WINDOW_LABEL;

    /// The window labels `tauri.conf.json` declares, read literally.
    ///
    /// NOT a JSON parser, for the reason `veil.rs`'s `declares_window` gives:
    /// this crate does not depend on `serde_json` and a lot about window
    /// lifetimes is not the place to buy one. It walks every `"label"` key and
    /// keeps what sits between the next pair of quotes.
    ///
    /// Its blind spots fail in the SAFE direction: a shape it cannot read yields
    /// no label at all, which turns the test below red rather than letting it
    /// pass quietly. The fixtures test pins that.
    fn labels_declared_in_the_configuration(config: &str) -> Vec<String> {
        config
            .split("\"label\"")
            .skip(1)
            .filter_map(|after_key| {
                let (_, from_value) = after_key.split_once('"')?;
                let (value, _) = from_value.split_once('"')?;
                Some(value.to_owned())
            })
            .collect()
    }

    /// A window that answers from a script and REMEMBERS every call.
    ///
    /// The log is the point, exactly as it is for `shortcut::FakeRegistrar`:
    /// what this module promises is not only that a second launch reports its
    /// failures, it is the ORDER of the three steps and the fact that one
    /// refusal does not cancel the others. An assertion on the returned lines
    /// alone would pass over an implementation that never showed anything.
    struct FakeWindow {
        minimized: Result<bool, String>,
        refuse: Vec<&'static str>,
        calls: Vec<String>,
    }

    impl FakeWindow {
        fn new(minimized: bool) -> Self {
            Self {
                minimized: Ok(minimized),
                refuse: Vec::new(),
                calls: Vec::new(),
            }
        }

        /// A window whose state cannot be read at all.
        fn unreadable() -> Self {
            Self {
                minimized: Err("the window is gone".to_owned()),
                refuse: Vec::new(),
                calls: Vec::new(),
            }
        }

        fn refusing(mut self, step: &'static str) -> Self {
            self.refuse.push(step);
            self
        }

        fn answer(&mut self, step: &'static str) -> Result<(), String> {
            self.calls.push(step.to_owned());

            if self.refuse.contains(&step) {
                Err(format!("the runtime would not {step}"))
            } else {
                Ok(())
            }
        }
    }

    impl Front for FakeWindow {
        fn is_minimized(&mut self) -> Result<bool, String> {
            self.calls.push("is_minimized".to_owned());
            self.minimized.clone()
        }

        fn unminimize(&mut self) -> Result<(), String> {
            self.answer("unminimize")
        }

        fn show(&mut self) -> Result<(), String> {
            self.answer("show")
        }

        fn set_focus(&mut self) -> Result<(), String> {
            self.answer("set_focus")
        }
    }

    // === which window's closing ends the application =========================

    #[test]
    fn closing_the_main_window_ends_the_application() {
        // THE defect of 7 September 2026, in one line. Without this rule the
        // process outlives its last visible window and keeps holding the global
        // shortcut, so the next launch is refused the combination by itself.
        assert!(
            closing_ends_the_application(MAIN_WINDOW_LABEL),
            "closing the `{MAIN_WINDOW_LABEL}` window must end the process: the hidden veil is \
             still in the runtime's window map, so nothing else will ever request an exit"
        );
    }

    #[test]
    fn closing_the_veil_leaves_the_application_running() {
        // The other half, and the one a blunt fix would get wrong. The veil is
        // raised and taken down on every capture; ending the application with
        // it would turn each screenshot into a quit.
        assert!(
            !closing_ends_the_application(VEIL_WINDOW_LABEL),
            "the `{VEIL_WINDOW_LABEL}` window comes and goes with every capture; ending Cliche \
             when it goes would make one screenshot the last thing the user does"
        );
    }

    #[test]
    fn the_label_that_ends_the_application_is_matched_exactly() {
        // Each of these would pass a `starts_with`, a `contains` or a
        // case-insensitive comparison. None of them is the main window, and a
        // loose match here is a process that exits on the wrong window.
        for impostor in ["main2", "Main", "main ", " main", "main-decoy", ""] {
            assert!(
                !closing_ends_the_application(impostor),
                "`{impostor}` must not pass for the main window"
            );
        }
    }

    #[test]
    fn the_window_that_ends_the_application_is_not_the_one_that_is_always_open() {
        // `ipc.rs` holds the same inequality for a different reason - its guard
        // would become a no-op. Here the cost is larger and immediate: one label
        // for both windows makes every rule in this file fire on the wrong one,
        // so a capture would quit Cliche and closing the launcher would not.
        assert_ne!(
            MAIN_WINDOW_LABEL, VEIL_WINDOW_LABEL,
            "the two windows must be distinguishable, or `closing_ends_the_application` answers \
             the same thing for both and a second launch raises a full-screen sheet"
        );
    }

    #[test]
    fn the_main_window_label_is_the_one_the_configuration_declares() {
        // The constant held against the file that really creates the window. A
        // `label` renamed in tauri.conf.json and not here would leave every rule
        // in this module pointing at a window that does not exist: closing the
        // launcher would end nothing, and a second launch would find nothing to
        // raise - the exact defect of 7 September 2026, back with a new cause.
        //
        // `include_str!` rather than `fs::read_to_string`, unlike `ipc.rs`'s ACL
        // sentinel: that one reads a GENERATED file, which has to be read at run
        // time, while this is a source file of the repository and reading it at
        // compile time makes editing it re-run this test.
        const CONFIG: &str = include_str!("../tauri.conf.json");

        let labels = labels_declared_in_the_configuration(CONFIG);

        assert!(
            !labels.is_empty(),
            "no window label was read out of tauri.conf.json. This READER is what is broken, not \
             the application - and a reader that finds nothing would keep this test green whatever \
             the configuration declares"
        );
        assert!(
            labels.iter().any(|label| label == MAIN_WINDOW_LABEL),
            "tauri.conf.json declares {labels:?} and MAIN_WINDOW_LABEL is `{MAIN_WINDOW_LABEL}`. \
             The constant and the configuration have drifted apart"
        );
    }

    #[test]
    fn the_configuration_reader_finds_labels_and_refuses_a_near_miss() {
        // Without this, the test above could be green over a reader that sees
        // nothing, or one that sees a label in any word beginning with `label`.
        assert_eq!(
            labels_declared_in_the_configuration(r#"{ "windows": [{ "label": "main" }] }"#),
            vec!["main".to_owned()]
        );
        assert_eq!(
            labels_declared_in_the_configuration(r#"[{ "label": "main" }, { "label": "second" }]"#),
            vec!["main".to_owned(), "second".to_owned()],
            "a configuration that declares two windows must yield two labels, or the test above \
             would stop noticing the second"
        );

        assert!(labels_declared_in_the_configuration("{}").is_empty());
        assert!(
            labels_declared_in_the_configuration(r#"{ "labelled": "main" }"#).is_empty(),
            "`labelled` is not `label`, and reading one into the other would let a key nobody \
             wrote decide which window ends this application"
        );
    }

    // === what a second launch does ===========================================

    #[test]
    fn a_second_launch_raises_the_main_window() {
        assert_eq!(
            window_a_second_instance_raises(),
            MAIN_WINDOW_LABEL,
            "a user who launches Cliche again is asking to SEE it; the launcher is the window \
             that answers that"
        );
        assert_ne!(
            window_a_second_instance_raises(),
            VEIL_WINDOW_LABEL,
            "raising the veil would put a full-screen, always-on-top sheet over the desktop with \
             no capture behind it and nothing but Escape to take it down"
        );
    }

    #[test]
    fn a_minimised_window_is_taken_out_of_the_taskbar_before_it_is_shown() {
        // THE case the whole path exists for: Cliche is running, minimised, and
        // the user launches it again because they cannot see it. A `show()` on a
        // minimised window leaves it minimised, so without the first step the
        // second launch would do nothing visible at all - which is
        // indistinguishable, to the user, from the defect being unfixed.
        let mut window = FakeWindow::new(true);

        let failures = bring_to_front(&mut window);

        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(
            window.calls,
            vec![
                "is_minimized".to_owned(),
                "unminimize".to_owned(),
                "show".to_owned(),
                "set_focus".to_owned(),
            ],
            "the focus must come LAST: given to a window that is still minimised or still hidden \
             it lands on nothing"
        );
    }

    #[test]
    fn a_window_that_is_not_minimised_is_never_unminimised() {
        // The step is conditional on purpose. Restoring a window that is not
        // minimised is what un-maximises a maximised one, and a second launch
        // that resized the user's window would be a new defect put in place of
        // the old one.
        let mut window = FakeWindow::new(false);

        let failures = bring_to_front(&mut window);

        assert!(failures.is_empty(), "{failures:?}");
        assert_eq!(
            window.calls,
            vec![
                "is_minimized".to_owned(),
                "show".to_owned(),
                "set_focus".to_owned(),
            ],
            "a window that is not minimised must not be restored: on a maximised one that is what \
             takes the maximisation away"
        );
    }

    #[test]
    fn a_window_whose_state_cannot_be_read_is_taken_out_of_the_taskbar_anyway() {
        // The unanswerable question, decided towards the user's problem: they
        // launched Cliche again because they cannot see it. Leaving a minimised
        // window minimised is the symptom itself; un-maximising a window that
        // was not minimised is an annoyance. The cheaper wrong is chosen, and
        // the failure is still reported.
        let mut window = FakeWindow::unreadable();

        let failures = bring_to_front(&mut window);

        assert_eq!(
            window.calls,
            vec![
                "is_minimized".to_owned(),
                "unminimize".to_owned(),
                "show".to_owned(),
                "set_focus".to_owned(),
            ],
            "a state that could not be read must be treated as minimised, or the one case this \
             path exists for is the one it gives up on"
        );
        assert_eq!(failures.len(), 1, "{failures:?}");
        assert!(
            failures[0].contains("the window is gone"),
            "what the runtime answered must survive into the line: {}",
            failures[0]
        );
    }

    #[test]
    fn every_step_is_attempted_even_after_one_of_them_is_refused() {
        // A window that could not be un-minimised is still worth showing, and a
        // window that could not be shown is still worth focusing. Stopping at
        // the first refusal would leave the user with the ghost this module
        // exists to end - a Cliche that is running and invisible.
        let mut window = FakeWindow::new(true)
            .refusing("unminimize")
            .refusing("show")
            .refusing("set_focus");

        let failures = bring_to_front(&mut window);

        assert_eq!(
            window.calls,
            vec![
                "is_minimized".to_owned(),
                "unminimize".to_owned(),
                "show".to_owned(),
                "set_focus".to_owned(),
            ],
            "one refused step must not cancel the others"
        );
        assert_eq!(
            failures.len(),
            3,
            "each refusal is a separate thing that did not happen: {failures:?}"
        );

        for line in &failures {
            assert!(
                line.contains("second launch"),
                "the line must say WHICH path failed - this one runs in the process the user \
                 cannot see, next to lines from the one they can: {line}"
            );
            assert!(
                line.contains(MAIN_WINDOW_LABEL),
                "the line must name the window: {line}"
            );
            assert!(
                line.contains("the runtime would not"),
                "the reason must survive into the line, or nobody can act on it: {line}"
            );
        }
    }

    #[test]
    fn a_window_that_came_back_has_nothing_to_report() {
        // The silence is the report, the same rule `ShortcutStatus::Accepted`
        // obeys: a line on the happy path would train whoever reads that
        // terminal to skip the ones that matter.
        let mut window = FakeWindow::new(true);

        assert!(bring_to_front(&mut window).is_empty());
    }

    // === the wiring, read from `lib.rs` itself ===============================

    #[test]
    fn the_single_instance_plugin_is_the_first_one_registered() {
        // "The Single Instance plugin must be the first one to be registered to
        // work well. This assures that it runs before other plugins can
        // interfere." - the Tauri v2 documentation, v2.tauri.app/plugin/
        // single-instance, read on 7 September 2026.
        //
        // READ, NOT MEASURED, and the distinction matters here: that sentence
        // was taken from Tauri's own documentation page and NOT confirmed
        // against the plugin's source, which could not be opened from where this
        // was written. Nothing in this repository has observed what registering
        // it second would cost.
        //
        // What is certain is the direction of the risk. This plugin is what
        // stops the second process before it asks Windows for Ctrl + Maj + 2;
        // every plugin that runs before it is a plugin the doomed process has
        // already started.
        let source = include_str!("lib.rs");

        let first = source
            .find(".plugin(")
            .expect("lib.rs must register at least one plugin");

        assert!(
            source[first..].starts_with(".plugin(tauri_plugin_single_instance::init"),
            "the first plugin registered in lib.rs is not the single-instance one. Move it back \
             to the top of the builder chain: what follows here is `{}`",
            &source[first..(first + 60).min(source.len())]
        );
    }

    #[test]
    fn the_builder_asks_this_module_rather_than_deciding_for_itself() {
        // The rules above are worth exactly as much as their wiring. Until
        // 7 September 2026 `lib.rs` installed no window-event handler at all,
        // which is why the process survived its own window - so the thing to
        // guard is not only WHAT the rule says but that somebody still asks it.
        //
        // A source-reading test, like `ipc.rs`'s command sentinel, and with the
        // same honest limit: it proves the call is written, never that it ran.
        // Nothing in this suite can run it - that needs an event loop.
        let source = include_str!("lib.rs");

        for call in [
            ".on_window_event(",
            "lifecycle::closing_ends_the_application(",
            "tauri_plugin_single_instance::init(",
            "lifecycle::raise_main_window(",
        ] {
            assert!(
                source.contains(call),
                "`{call}` is nowhere in lib.rs. The decision this module holds is then wired to \
                 nothing, and every other test in this file is green over an application that \
                 still leaves a ghost process behind"
            );
        }

        assert!(
            !source.contains(&format!("== \"{MAIN_WINDOW_LABEL}\"")),
            "lib.rs compares a window label literally. The rule belongs here, where it is tested; \
             a copy of it in a closure no test can reach is how the two drift apart"
        );
    }
}
