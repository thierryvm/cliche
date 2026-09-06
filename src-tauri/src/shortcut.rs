//! The global shortcut: the entry point of the capture pipeline.
//!
//! Registered from RUST, in `setup`, and deliberately never from JavaScript.
//! The handler has to run before the webview is involved at all, and an IPC
//! round trip would land inside the very budget this lot measures. That is also
//! why no `@tauri-apps/plugin-global-shortcut` package exists in `package.json`.
//!
//! Nothing here may panic. The closure below runs on the plugin's hotkey
//! thread; a panic there takes the application down, and losing the app to a
//! diagnostic is a bad trade. Hence `try_state` rather than `state`, saturating
//! arithmetic, and every error turned into a printed line.
//!
//! The COMBINATION is not written here. It is a row of `shortcuts::REGISTRY`
//! (`shortcuts.rs`, plural - one letter away, and a different file), which is
//! also what the help page reads. This module looks its entry up by id and
//! binds it; the two files are kept apart because that one declares a command
//! and this one deliberately declares none, as the next section explains.
//!
//! # SINCE 6 SEPTEMBER 2026 THE COMBINATION CAN ALSO COME FROM THE USER
//!
//! `settings.rs` holds what they chose, and [`startup_choice`] decides which of
//! the two this launch offers the system. The registry row stopped being the
//! answer and became the FALLBACK - the combination used on a first launch, and
//! the one used again whenever the file is missing, unreadable or holds
//! something no longer registrable.
//!
//! What that changes here, and it is the part worth reading before touching this
//! file: the handler is bound more than once per process. [`change`] releases
//! the live combination, offers the new one, and PUTS THE OLD ONE BACK when the
//! system refuses - because a user who tries a combination another program owns
//! must not be left with no capture shortcut at all. That sequence is pure and
//! takes its registrar as an argument, so the rollback is a test rather than a
//! hope; `on_shortcut` needs an event loop and could never be exercised here.
//!
//! # What guards this module, and what does not - stated because it is easy to
//! # assume the wrong one
//!
//! Nothing in this file is reachable from a webview: `install` is called from
//! `setup`, and the handler is a Rust closure the plugin invokes. No command is
//! declared here, so there is no `invoke` frontier for an ACL to sit on, and no
//! capability of this application grants anything to the global-shortcut plugin
//! - correctly, since no page needs it.
//!
//! What that does NOT mean, and what a reader of the paragraph above could
//! reasonably infer: that the ACL is what protects the pipeline this shortcut
//! starts. `crate::veil::perform_capture` is called from Rust, here, with no
//! capability involved anywhere. The commands that END a capture -
//! `veil_painted`, `veil_selected`, `veil_dismissed` - ARE invoked from a
//! webview, and since 4 September 2026 the ACL does check them: this application
//! declares its own manifest, and `capabilities/veil.json` grants those three to
//! the veil window and to no other. They are guarded twice - there, and by the
//! webview-label check in `ipc.rs`, which explains why both are kept. By neither
//! this module's distance from the frontend, nor by the absent plugin permission
//! described just above.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

use serde::Serialize;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, ShortcutState,
};

use crate::settings::{self, SettingsRead};
use crate::shortcuts::{self, Combination, ShortcutEntry};

/// How many finished runs before the report prints itself, with no further
/// action from whoever is measuring.
///
/// 20 and not 10: `timing.rs` computes p95 by nearest rank, and on 10 samples
/// `ceil(0.95 * 10) = 10` makes the p95 the maximum - a number that carries no
/// more information than "the worst of ten".
const RUNS_PER_REPORT: usize = 20;

/// The one registry entry this module binds.
///
/// The combination itself no longer lives here: it is a row of
/// `shortcuts::REGISTRY`, which is also what the main window reads through
/// `describe_shortcuts`. Looked up by id rather than by position, so reordering
/// the table cannot silently change which shortcut starts a capture.
///
/// Returns an error instead of panicking on a missing entry, for the reason
/// this whole module is written the way it is: an application with no capture
/// shortcut and one loud line in the terminal is worth more than no application.
fn capture_entry() -> Result<&'static ShortcutEntry, String> {
    shortcuts::entry(shortcuts::CAPTURE_REGION).ok_or_else(|| {
        format!(
            "[cliche] shortcut: the registry has no `{}` entry, so NOTHING starts a capture. \
             This is a programming error in src-tauri/src/shortcuts.rs, not a machine problem.",
            shortcuts::CAPTURE_REGION
        )
    })
}

/// How many times the capture combination has been pressed since this process
/// started.
///
/// A module-level `static` since 6 September 2026, and it used to be a local
/// owned by the handler's closure. The combination is settable now, so the
/// handler is bound AGAIN every time it changes - and a counter living in the
/// closure would restart at zero with each new combination, which would make
/// "press 1" in the terminal mean "the first press since you last changed a
/// setting". One process, one count.
///
/// `Relaxed` for the reason it always was: this counter is only ever compared
/// with itself, and each `fetch_add` hands back a distinct value, which is all
/// a press number needs.
static PRESSES: AtomicUsize = AtomicUsize::new(0);

/// Whether the run just filed completes a batch worth reporting on.
///
/// `pub(crate)` because the caller moved in 1d: a run is now FILED when the
/// webview acknowledges the paint, not when the handler returns, so `veil.rs`
/// is where the batch is counted. The rule itself stays here, next to the
/// constant it reads and the tests that pin it.
pub(crate) fn report_due(run_number: usize) -> bool {
    run_number > 0 && run_number % RUNS_PER_REPORT == 0
}

/// The three answers this application can get about its capture shortcut, and
/// the only three it will ever report.
///
/// # Why a type, and why THREE states
///
/// Until 6 September 2026 [`install`] returned `Result<(), String>` and `lib.rs`
/// printed the error and dropped it. Nothing else in the process could ever
/// learn what Windows had answered, so the launcher said "the shortcut registry
/// could not be read" - a sentence that is false in every one of the cases
/// below. What is on the screen has to be what happened.
///
/// Two states would not do either, and the difference is not academic: it
/// decides what the user is TOLD. A combination Windows refused is a combination
/// another program is holding, and the user can free it. A combination that was
/// never offered - an unreadable accelerator, an entry missing from the
/// registry, a plugin that failed to load - is a fault of this application, and
/// telling that user to close another program would send them hunting for
/// something that does not exist.
///
/// # The wire shape
///
/// Serialised internally tagged, so the frontend switches on one field. Each
/// variant NAMES its tag rather than leaning on `rename_all`, so that the
/// strings crossing to TypeScript are readable in this file and can be held
/// against `src/shortcuts.ts` - which
/// `the_three_wire_tags_are_the_ones_serde_is_told_to_emit_and_the_frontend_reads`
/// does.
///
/// `Clone` because the command hands a copy to the webview; the value is
/// managed state and there is exactly one of it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "status")]
pub enum ShortcutStatus {
    /// The operating system took the combination. The shortcut works.
    #[serde(rename = "accepted")]
    Accepted {
        /// The combination that was taken, in the plugin's syntax.
        ///
        /// `String` and no longer `&'static str`, since 6 September 2026: the
        /// combination can be chosen by the user, and a value read out of a file
        /// at run time has no static lifetime to borrow from.
        accelerator: String,
    },
    /// The operating system refused the combination, with the reason it gave.
    ///
    /// The reason is the OS's own words, in English, and it is meant for the
    /// terminal and the developer console - never for the screen. What the
    /// screen says is decided in `src/shortcut-hint.ts`.
    #[serde(rename = "refused-by-system")]
    RefusedBySystem {
        /// The combination that was refused. Named, so a user can free it.
        accelerator: String,
        /// What the operating system answered, verbatim.
        reason: String,
    },
    /// The combination was never offered to the operating system, and why.
    ///
    /// No `accelerator`: in the worst of these cases there is no entry to read
    /// one from, and a field that is sometimes a guess is worse than no field.
    #[serde(rename = "not-attempted")]
    NotAttempted {
        /// What stopped this application from even asking.
        reason: String,
    },
}

impl ShortcutStatus {
    /// The line the terminal gets, or `None` when there is nothing to report.
    ///
    /// Pure, so its wording is under test, and returned rather than printed for
    /// the reason [`install`] used to return one: the CALLER is what decides the
    /// application keeps going, so the caller is where that decision should be
    /// readable.
    pub fn terminal_line(&self) -> Option<String> {
        match self {
            Self::Accepted { .. } => None,
            Self::RefusedBySystem {
                accelerator,
                reason,
            } => Some(registration_failure(accelerator, reason)),
            Self::NotAttempted { reason } => Some(never_offered(reason)),
        }
    }
}

/// The line printed when the operating system REFUSED the combination.
///
/// Pure, so its wording is under test. This message is the only thing standing
/// between "another program already owns Ctrl+Shift+2" and an application that
/// looks perfectly fine and does nothing at all - the worst of both worlds.
///
/// The combination is passed in rather than read from a constant here: it is
/// the registry entry's, and taking it from anywhere else would let the message
/// name a combination other than the one that was actually refused.
fn registration_failure(accelerator: &str, reason: &str) -> String {
    format!(
        "[cliche] shortcut: FAILED to take {accelerator} ({reason}). \
         Cliche is running WITHOUT its capture shortcut - another program is \
         most likely holding that combination."
    )
}

/// The line printed when the combination was never offered at all.
///
/// A SEPARATE line from the one above, and that is the point of it. Until
/// 6 September 2026 an unreadable accelerator and a plugin that failed to load
/// both printed "another program is most likely holding that combination" -
/// which sent whoever read it looking for a program that was not there. These
/// three causes are ours; the message says so.
fn never_offered(reason: &str) -> String {
    format!(
        "[cliche] shortcut: the capture shortcut was NEVER OFFERED to the system \
         ({reason}). Cliche is running WITHOUT its capture shortcut - this one is a \
         fault of the application itself, not a combination another program is holding."
    )
}

/// The line printed when the saved combination could not be used.
///
/// Pure, so its wording is under test. The FALLBACK is named as well as the
/// fault: a user who set a shortcut, restarted, and found another one in its
/// place needs to be told both which one is live and why theirs is not.
fn saved_choice_unusable(why: &str, fallback: &str) -> String {
    format!(
        "[cliche] settings: the saved capture shortcut could not be used ({why}). \
         Falling back to {fallback}, the combination this application ships with. The file is \
         left exactly as it is - setting a shortcut again is what replaces it."
    )
}

/// Which combination this launch will offer the operating system.
///
/// Pure, and this is the function the requirement "a settings file that is
/// missing, unreadable or incoherent must not stop the application" actually
/// lives in. It cannot fail: every input has an answer, and the worst of them
/// answers with the combination this application ships with plus one line for
/// the terminal.
///
/// A file that is ABSENT gets no line at all. It is the ordinary state of a
/// first launch, and a warning printed on every one of those would train
/// whoever reads that terminal to skip the lines that matter.
///
/// The saved value is put through `shortcuts::accept` here rather than trusted:
/// a file is an input like any other. A combination that no longer parses -
/// written by an older version, or edited by hand - is a fallback, not a crash
/// and not a shortcut nobody can press.
pub fn startup_choice(
    saved: &SettingsRead,
    fallback: &Combination,
) -> (Combination, Option<String>) {
    let why = match saved {
        SettingsRead::Absent => return (fallback.clone(), None),
        SettingsRead::Unusable(why) => why.clone(),
        SettingsRead::Chosen(value) => match shortcuts::accept(value) {
            Ok(chosen) => return (chosen, None),
            Err(why) => why,
        },
    };

    (
        fallback.clone(),
        Some(saved_choice_unusable(&why, &fallback.accelerator)),
    )
}

/// What one attempt at taking or releasing a combination can do.
///
/// A trait, and the whole reason is the paragraph below [`change`]: the ORDER in
/// which a shortcut is released, replaced and put back is the part of this
/// module a user notices when it is wrong, and it cannot be exercised through a
/// real plugin - `on_shortcut` needs an event loop. So the sequence takes its
/// registrar as an argument and a test hands it one that records every call.
pub trait Registrar {
    /// Gives a combination back to the operating system.
    fn unregister(&mut self, accelerator: &str) -> Result<(), String>;
    /// Takes a combination, wiring it to the capture handler.
    fn register(&mut self, accelerator: &str) -> Result<(), String>;
}

/// What became of a request to change the capture combination.
///
/// Named by the RESULT and not by the mechanism, deliberately. What a user has
/// to be told is which combination is live now; whether that took a rollback, or
/// whether the old one was never released in the first place, is a detail of how
/// - and both leave the machine in the same state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The new combination is registered.
    Took(Combination),
    /// The new one was refused. The PREVIOUS one is registered.
    Kept {
        /// What the system would not take.
        refused: Combination,
        /// What is live instead - the combination that was there before.
        active: Combination,
        /// What the system answered about `refused`, verbatim.
        reason: String,
    },
    /// The new one was refused and no combination is registered at all.
    ///
    /// The state this module exists to make rare: it is reached only when the
    /// previous combination could not be taken back either, or when there was
    /// no previous combination to fall back on.
    Stranded {
        refused: Combination,
        reason: String,
    },
}

impl Outcome {
    /// The value the webview receives, once the caller knows whether the choice
    /// was written down.
    ///
    /// `saved` is meaningless for anything but [`Outcome::Took`]: nothing is
    /// written when the combination was not taken, and writing a refused one
    /// would hand the same failure to every launch afterwards.
    fn into_change(self, saved: bool) -> ShortcutChange {
        match self {
            Self::Took(active) => ShortcutChange::Changed { active, saved },
            Self::Kept {
                refused,
                active,
                reason,
            } => ShortcutChange::Kept {
                refused,
                active,
                reason,
            },
            Self::Stranded { refused, reason } => ShortcutChange::Stranded { refused, reason },
        }
    }
}

/// [`Outcome`], as it crosses to the webview.
///
/// Internally tagged on `outcome`, and each variant NAMES its tag rather than
/// leaning on `rename_all`, for the same reason [`ShortcutStatus`] does: the
/// strings crossing to TypeScript are readable in this file and can be held
/// against `src/shortcuts.ts` - which
/// `the_three_change_tags_are_the_ones_serde_emits_and_the_frontend_reads` does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "outcome")]
pub enum ShortcutChange {
    /// The combination asked for is the one that is live.
    #[serde(rename = "changed")]
    Changed {
        active: Combination,
        /// Whether the choice will survive a restart. `false` means the shortcut
        /// WORKS and the settings file could not be written - which the screen
        /// has to say, or the user finds the old combination back tomorrow with
        /// nothing having warned them.
        saved: bool,
    },
    /// The combination asked for was refused; the previous one is still live.
    #[serde(rename = "kept")]
    Kept {
        refused: Combination,
        active: Combination,
        reason: String,
    },
    /// The combination asked for was refused and nothing is live.
    #[serde(rename = "stranded")]
    Stranded {
        refused: Combination,
        reason: String,
    },
}

/// Swaps one capture combination for another, and PUTS THE OLD ONE BACK when
/// the new one is refused.
///
/// # The order, which is the whole of this function
///
/// 1. The wanted combination is already the live one: nothing is touched. Asking
///    the operating system for a combination it has already given us is refused
///    by Windows, and that refusal would read as "your setting did not work".
/// 2. The live combination is RELEASED. Until it is, the new one may collide
///    with it - `Ctrl+Shift+Digit2` cannot be registered twice - and the user
///    would be told another application is holding a combination this one holds.
/// 3. The wanted combination is offered. Taken: done.
/// 4. Refused: the previous one is registered AGAIN, immediately. This is the
///    step the whole lot is about. Without it, a user who tries a combination
///    another program owns is left with NO capture shortcut at all, having
///    changed a setting that was working.
/// 5. The rollback itself is refused: [`Outcome::Stranded`], and the reason
///    carries both refusals, because at that point neither combination works and
///    the terminal is the only place that can say why.
///
/// A failure at step 2 stops there and reports [`Outcome::Kept`]: the previous
/// combination was never released, so it is still live, which is exactly what
/// that answer means.
///
/// Nothing is printed and nothing is written to disk: this is the DECISION, and
/// the caller is where its consequences are carried out.
pub fn change<R: Registrar>(
    registrar: &mut R,
    active: Option<&Combination>,
    wanted: &Combination,
) -> Outcome {
    if let Some(current) = active {
        if current.accelerator == wanted.accelerator {
            return Outcome::Took(current.clone());
        }

        if let Err(reason) = registrar.unregister(&current.accelerator) {
            return Outcome::Kept {
                refused: wanted.clone(),
                active: current.clone(),
                reason,
            };
        }
    }

    let Err(reason) = registrar.register(&wanted.accelerator) else {
        return Outcome::Took(wanted.clone());
    };

    let Some(current) = active else {
        return Outcome::Stranded {
            refused: wanted.clone(),
            reason,
        };
    };

    match registrar.register(&current.accelerator) {
        Ok(()) => Outcome::Kept {
            refused: wanted.clone(),
            active: current.clone(),
            reason,
        },
        Err(second) => Outcome::Stranded {
            refused: wanted.clone(),
            reason: format!(
                "{reason}; and {} could not be taken back either: {second}",
                current.accelerator
            ),
        },
    }
}

/// What this application currently knows about its capture shortcut.
///
/// Managed by Tauri, and behind a mutex since the combination became settable:
/// `describe_shortcut_status` and `describe_shortcuts` both read it, and
/// `set_capture_shortcut` writes it, on whatever thread the IPC call landed on.
#[derive(Debug)]
pub struct CaptureShortcut {
    /// Whether the global-shortcut plugin is loaded in this process at all.
    ///
    /// NOT a nicety, and not derivable from the status without a chain of
    /// reasoning that would break the day somebody adds a case: `global_shortcut()`
    /// is `self.state::<GlobalShortcut<R>>().inner()`
    /// (`tauri-plugin-global-shortcut-2.3.2/src/lib.rs:249-251`), and
    /// `Manager::state` PANICS when the type was never managed. Nothing that
    /// runs on an IPC thread may panic, so [`change_capture_shortcut`] refuses
    /// outright rather than reaching a plugin that is not there.
    plugin_loaded: bool,
    inner: Mutex<CaptureState>,
}

#[derive(Debug, Clone)]
struct CaptureState {
    status: ShortcutStatus,
    /// The combination the status is ABOUT, with the caps to draw it.
    ///
    /// `None` only for [`ShortcutStatus::NotAttempted`], where there may be no
    /// entry to read a combination from at all - which is exactly why that
    /// variant carries no accelerator either.
    attempted: Option<Combination>,
}

impl CaptureShortcut {
    pub fn new(
        status: ShortcutStatus,
        attempted: Option<Combination>,
        plugin_loaded: bool,
    ) -> Self {
        Self {
            plugin_loaded,
            inner: Mutex::new(CaptureState { status, attempted }),
        }
    }

    /// Runs something against the state, whatever happened on another thread.
    ///
    /// `PoisonError::into_inner` and NOT `unwrap`, for the reason the whole of
    /// this module is written the way it is: a poisoned mutex means some other
    /// thread panicked while holding it, and taking the application down over
    /// that would lose the user their session for a diagnostic. Nothing here
    /// panics while the lock is held, so the value behind it is intact.
    fn with<T>(&self, action: impl FnOnce(&mut CaptureState) -> T) -> T {
        let mut state = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        action(&mut state)
    }

    /// What the operating system last answered.
    pub fn status(&self) -> ShortcutStatus {
        self.with(|state| state.status.clone())
    }

    /// The combination that was last OFFERED, whether or not it was taken.
    ///
    /// This and not [`Self::active`] is what `describe_shortcuts` publishes: a
    /// combination Windows refused is still the one this application asked for,
    /// and the launcher's refusal note has to name it.
    pub fn attempted(&self) -> Option<Combination> {
        self.with(|state| state.attempted.clone())
    }

    /// The combination that is REGISTERED right now, if there is one.
    pub fn active(&self) -> Option<Combination> {
        self.with(|state| match state.status {
            ShortcutStatus::Accepted { .. } => state.attempted.clone(),
            ShortcutStatus::RefusedBySystem { .. } | ShortcutStatus::NotAttempted { .. } => None,
        })
    }

    fn set(&self, status: ShortcutStatus, attempted: Option<Combination>) {
        self.with(|state| {
            state.status = status;
            state.attempted = attempted;
        });
    }
}

/// What a press of the capture combination does.
///
/// A free function since the handler is bound more than once: the same body has
/// to run whichever combination is live, and a closure written twice is two
/// capture pipelines one edit apart.
fn on_press(app: &AppHandle, event_state: ShortcutState) {
    // The plugin reports both edges. Only the press starts a capture; measuring
    // the release would fold in how long the user held the keys down, which is
    // not our latency.
    if !matches!(event_state, ShortcutState::Pressed) {
        return;
    }

    // `saturating_add` for the same reason as everything else on this path: a
    // debug overflow panic here would kill the application over a counter.
    // Printed BEFORE the capture, so that a press which then fails somewhere
    // still leaves a trace in the terminal.
    let press = PRESSES.fetch_add(1, Ordering::Relaxed).saturating_add(1);
    println!("[cliche] shortcut: press {press}");

    // t0 IS THE FIRST LINE OF `perform_capture` - and that is a limitation, not
    // a design choice.
    //
    // "shortcut pressed -> handler entered" is NOT measurable from inside this
    // process. Nothing gives us the instant the key went down: the event carries
    // a hotkey id and a state, no timestamp. Our first possible clock reading is
    // the entry of this handler, so the whole trip "physical key -> Windows
    // low-level hook -> global-hotkey thread -> this closure" lies OUTSIDE every
    // figure the instrument prints.
    //
    // Read the 150 ms budget accordingly: it is counted from HERE, not from the
    // user's finger. The unmeasured part is unknown, and unknown is not the same
    // as zero.
    //
    // The whole pipeline lives in `veil::perform_capture` rather than in this
    // handler for one reason: the automated benchmark has to call THE SAME code,
    // or it would be measuring a different program. The press counter above is
    // not the measurement - a run is filed when the webview acknowledges the
    // paint, and `veil.rs` prints the report every twenty of those.
    crate::veil::perform_capture(app);
}

/// The registrar that really talks to the plugin.
///
/// Deliberately thin: everything it could get wrong is a decision, and the
/// decisions are in [`change`], where they are tested. What is left here is one
/// parse and one plugin call per operation, and neither may panic.
struct PluginRegistrar<'a> {
    app: &'a AppHandle,
}

impl Registrar for PluginRegistrar<'_> {
    fn unregister(&mut self, accelerator: &str) -> Result<(), String> {
        let shortcut = shortcuts::parse(accelerator)?;

        self.app
            .global_shortcut()
            .unregister(shortcut)
            .map_err(|error| error.to_string())
    }

    fn register(&mut self, accelerator: &str) -> Result<(), String> {
        let shortcut = shortcuts::parse(accelerator)?;

        self.app
            .global_shortcut()
            .on_shortcut(shortcut, |app, _shortcut, event| {
                on_press(app, event.state());
            })
            .map_err(|error| error.to_string())
    }
}

/// Everything `setup` learns from one attempt at installing the shortcut.
///
/// The note is separate from the status because the two say different things:
/// the status is what the OPERATING SYSTEM answered about the combination that
/// was offered, and the note is why that was not the combination the user had
/// chosen. A launch can have either, both or neither.
pub struct Installed {
    pub status: ShortcutStatus,
    pub attempted: Option<Combination>,
    pub note: Option<String>,
    /// Whether the global-shortcut plugin was loaded. See [`CaptureShortcut`].
    pub plugin_loaded: bool,
}

/// Loads the plugin and binds the capture shortcut to the timing handler.
///
/// Returns WHAT HAPPENED rather than whether it worked. The three states of
/// [`ShortcutStatus`] are the three answers this function can come back with,
/// and every one of them ends up on the screen through
/// `shortcuts::describe_shortcut_status`: this return value is no longer a line
/// for the terminal, it is the fact the launcher draws.
///
/// The COMBINATION it binds is [`startup_choice`]'s, which is the user's when
/// there is a usable one on disk and the registry's otherwise. A settings file
/// that is missing, unreadable or incoherent produces a `note` and never an
/// early return: an application that refused to start over a preference would
/// be trading its whole purpose for one.
///
/// Nothing is printed here, and nothing panics: the caller prints the note and
/// `status.terminal_line()`, and puts the rest into managed state.
pub fn install(app: &AppHandle, saved: &SettingsRead) -> Installed {
    let entry = match capture_entry() {
        Ok(entry) => entry,
        Err(reason) => {
            return Installed {
                status: ShortcutStatus::NotAttempted { reason },
                attempted: None,
                note: None,
                plugin_loaded: false,
            }
        }
    };

    // The registry's own row, with its HAND-WRITTEN caps rather than derived
    // ones: for this combination, and this one alone, there is a statement
    // somebody wrote and a test holds it - see
    // `the_displayed_keys_are_the_combination_that_is_actually_registered`.
    let fallback = Combination {
        accelerator: entry.accelerator.to_owned(),
        keys: entry.keys.iter().map(|key| (*key).to_owned()).collect(),
    };

    let (wanted, note) = startup_choice(saved, &fallback);

    // The plugin is loaded here, next to its only use, rather than in the
    // builder chain: `install` then either wires the shortcut completely or
    // says in one value why it did not, and `lib.rs` has a single line to read.
    //
    // Every branch that gives up before the operating system is asked is
    // `NotAttempted`: the combination was never offered, so nothing external
    // refused it.
    if let Err(error) = app.plugin(GlobalShortcutBuilder::new().build()) {
        return Installed {
            status: ShortcutStatus::NotAttempted {
                reason: format!("the global-shortcut plugin failed to load: {error}"),
            },
            attempted: None,
            note,
            plugin_loaded: false,
        };
    }

    let mut registrar = PluginRegistrar { app };

    let status = match registrar.register(&wanted.accelerator) {
        Ok(()) => ShortcutStatus::Accepted {
            accelerator: wanted.accelerator.clone(),
        },
        Err(reason) => ShortcutStatus::RefusedBySystem {
            accelerator: wanted.accelerator.clone(),
            reason,
        },
    };

    Installed {
        status,
        attempted: Some(wanted),
        note,
        plugin_loaded: true,
    }
}

/// Changes the live capture combination, writes the choice down, and records
/// what happened.
///
/// The decision is [`change`]'s; this is where its three answers are carried
/// out. Read in that order:
///
/// 1. The swap, with its rollback.
/// 2. The FILE, and only when the new combination was really taken. Persisting a
///    combination the system refused would hand the same failure to every launch
///    afterwards, and the only way out would be finding the file.
/// 3. The managed state, whatever happened - so that what
///    `describe_shortcut_status` and `describe_shortcuts` answer next is what
///    the system now holds and not what it held at startup.
///
/// A write that fails does NOT undo the change: the shortcut works, it simply
/// will not survive a restart, and `saved: false` is how the screen is told to
/// say so.
pub fn change_capture_shortcut(
    app: &AppHandle,
    wanted: &Combination,
) -> Result<ShortcutChange, String> {
    let state = app.try_state::<CaptureShortcut>().ok_or_else(|| {
        "no shortcut state is managed, so nothing here knows what is registered. `setup` in \
         src-tauri/src/lib.rs is what puts it there."
            .to_owned()
    })?;

    // Refused BEFORE anything reaches the plugin, and this is not defensive
    // programming for its own sake: `global_shortcut()` panics when the plugin
    // was never loaded, and a panic on an IPC thread takes the application down.
    // An error string crosses back to the page; a panic would not.
    if !state.plugin_loaded {
        return Err(
            "the global-shortcut plugin is not loaded in this process, so no combination can be \
             registered at all. The line `setup` printed at startup says why."
                .to_owned(),
        );
    }

    let mut registrar = PluginRegistrar { app };
    let outcome = change(&mut registrar, state.active().as_ref(), wanted);

    let saved = match &outcome {
        Outcome::Took(taken) => match settings::write(app, &taken.accelerator) {
            Ok(path) => {
                println!(
                    "[cliche] settings: capture shortcut {} written to {}",
                    taken.accelerator,
                    path.display()
                );
                true
            }
            Err(error) => {
                eprintln!(
                    "[cliche] settings: {} is REGISTERED but could not be written down ({error}). \
                     It works now and will not survive a restart.",
                    taken.accelerator
                );
                false
            }
        },
        Outcome::Kept { .. } | Outcome::Stranded { .. } => false,
    };

    match &outcome {
        Outcome::Took(taken) => state.set(
            ShortcutStatus::Accepted {
                accelerator: taken.accelerator.clone(),
            },
            Some(taken.clone()),
        ),
        Outcome::Kept { active, .. } => state.set(
            ShortcutStatus::Accepted {
                accelerator: active.accelerator.clone(),
            },
            Some(active.clone()),
        ),
        Outcome::Stranded { refused, reason } => state.set(
            ShortcutStatus::RefusedBySystem {
                accelerator: refused.accelerator.clone(),
                reason: reason.clone(),
            },
            Some(refused.clone()),
        ),
    }

    if let Some(line) = state.status().terminal_line() {
        eprintln!("{line}");
    }

    Ok(outcome.into_change(saved))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut};

    #[test]
    fn the_announced_shortcut_is_what_the_plugin_parser_actually_accepts() {
        // The value the interface promises the user, held against the parser
        // that will have to register it. A typo in the constant fails here
        // rather than at run time, in a message nobody is watching for.
        //
        // ADAPTED on 5 September 2026: the combination moved out of a constant
        // in this file and into `shortcuts::REGISTRY`, so the test reads it
        // from the entry `install` actually binds. The assertions themselves
        // are untouched - and they are the point. `shortcuts.rs` checks that
        // EVERY entry parses; this one checks that the entry starting a capture
        // is still Ctrl + Maj + 2, which is what the interface says out loud.
        let entry = capture_entry().expect("the registry must hold the capture entry");
        let shortcut =
            shortcuts::parse(entry.accelerator).expect("the capture shortcut must parse");

        assert_eq!(shortcut.key, Code::Digit2);
        assert_eq!(
            shortcut.mods,
            Modifiers::CONTROL | Modifiers::SHIFT,
            "Ctrl and Shift, and nothing else: an extra modifier would be a \
             different combination from the one the interface announces"
        );
    }

    #[test]
    fn the_parser_rejects_nonsense_so_the_test_above_is_not_vacuous() {
        // Without this, "it parsed" could mean "the parser accepts anything".
        assert!(Shortcut::from_str("Ctrl+Shift+NotAKey").is_err());
        assert!(Shortcut::from_str("").is_err());
    }

    #[test]
    fn the_report_is_due_every_twenty_runs_and_never_before() {
        assert!(!report_due(0), "no run at all is not a completed batch");
        assert!(!report_due(1));
        assert!(!report_due(RUNS_PER_REPORT - 1));
        assert!(report_due(RUNS_PER_REPORT));
        assert!(!report_due(RUNS_PER_REPORT + 1));
        assert!(
            report_due(RUNS_PER_REPORT * 2),
            "a measuring session does not stop at the first batch"
        );
    }

    #[test]
    fn a_batch_is_twenty_runs_because_a_ten_run_p95_is_only_the_maximum() {
        // Pinned deliberately: `timing.rs` documents that nearest-rank p95 on
        // 10 samples collapses onto the maximum. Lowering this constant would
        // silently turn the printed p95 into something else.
        assert_eq!(RUNS_PER_REPORT, 20);
    }

    #[test]
    fn a_refused_shortcut_names_the_combination_and_the_reason() {
        // ADAPTED on 5 September 2026: `registration_failure` now takes the
        // combination rather than reading a constant, so the test passes the
        // entry's own accelerator - and asserts against that same value rather
        // than against a second copy typed here. The three assertions are
        // unchanged.
        //
        // ADAPTED AGAIN on 6 September 2026: the line is now reached through
        // the status that carries it, so this exercises what `lib.rs` prints
        // rather than a function nothing calls.
        let entry = capture_entry().expect("the registry must hold the capture entry");
        let message = ShortcutStatus::RefusedBySystem {
            accelerator: entry.accelerator.to_owned(),
            reason: "HotKey already registered".to_owned(),
        }
        .terminal_line()
        .expect("a refusal by the system has something to say");

        assert!(
            message.contains(entry.accelerator),
            "a user cannot free a combination the message does not name: {message}"
        );
        assert!(
            message.contains("HotKey already registered"),
            "the reason given by the OS must survive into the message: {message}"
        );
        assert!(
            message.contains("WITHOUT"),
            "the message must say the app is running without its shortcut, not \
             just that something failed: {message}"
        );
    }

    #[test]
    fn an_accepted_shortcut_has_nothing_to_print() {
        // The silence is the report. A line on the happy path would train
        // whoever runs this to skip the lines that matter.
        assert_eq!(
            ShortcutStatus::Accepted {
                accelerator: "Ctrl+Shift+Digit2".to_owned(),
            }
            .terminal_line(),
            None
        );
    }

    #[test]
    fn a_shortcut_that_was_never_offered_does_not_blame_another_program() {
        // THE reason this state exists. Until 6 September 2026 an unreadable
        // accelerator and a plugin that failed to load both printed "another
        // program is most likely holding that combination", and whoever read it
        // went looking for a program that was not there.
        let message = ShortcutStatus::NotAttempted {
            reason: "the global-shortcut plugin failed to load: no event loop".to_owned(),
        }
        .terminal_line()
        .expect("a shortcut that was never offered has something to say");

        assert!(
            message.contains("no event loop"),
            "the cause must survive into the message, or nobody can act on it: {message}"
        );
        assert!(
            message.contains("NEVER OFFERED"),
            "the message must say the system was never asked, which is what \
             separates this state from a refusal: {message}"
        );
        assert!(
            message.contains("WITHOUT"),
            "the message must say the app is running without its shortcut: {message}"
        );
        assert!(
            !message.contains("another program is most likely"),
            "nothing external refused this combination; sending the user after \
             another program is the defect this state was split out to end: {message}"
        );
    }

    /// The tag serde is told to emit for a status, one arm per variant.
    ///
    /// Exhaustive on purpose: a fourth variant added to [`ShortcutStatus`] stops
    /// this file compiling, which is the loudest way to be told that the
    /// TypeScript union has to grow an arm too.
    fn wire_tag(status: &ShortcutStatus) -> &'static str {
        match status {
            ShortcutStatus::Accepted { .. } => "accepted",
            ShortcutStatus::RefusedBySystem { .. } => "refused-by-system",
            ShortcutStatus::NotAttempted { .. } => "not-attempted",
        }
    }

    #[test]
    fn the_three_wire_tags_are_the_ones_serde_is_told_to_emit_and_the_frontend_reads() {
        // WHAT THIS CHECKS, AND WHAT IT DOES NOT, because the difference is the
        // whole honesty of it: it holds the `#[serde(rename = "…")]` attribute
        // in THIS file against the literals `src/shortcuts.ts` switches on. It
        // does NOT observe the JSON serde actually produces - this crate has no
        // `serde_json` and lot 3 is not the place to buy one. What is verified
        // is that the two sides name the same three strings; what is taken on
        // serde's word is that `#[serde(tag = "status")]` puts them under the
        // key `status`.
        //
        // Both files are read at compile time, so editing either one re-runs
        // this test.
        let rust = include_str!("shortcut.rs");
        let typescript = include_str!("../../src/shortcuts.ts");

        let statuses = [
            ShortcutStatus::Accepted {
                accelerator: "Ctrl+Shift+Digit2".to_owned(),
            },
            ShortcutStatus::RefusedBySystem {
                accelerator: "Ctrl+Shift+Digit2".to_owned(),
                reason: String::new(),
            },
            ShortcutStatus::NotAttempted {
                reason: String::new(),
            },
        ];

        for status in &statuses {
            let tag = wire_tag(status);

            assert!(
                rust.contains(&format!("#[serde(rename = \"{tag}\")]")),
                "no variant of ShortcutStatus is renamed to `{tag}`, so serde will not emit it"
            );
            assert!(
                typescript.contains(&format!("'{tag}'")),
                "src/shortcuts.ts does not switch on `{tag}`. Rust would send a status the \
                 launcher cannot read, and TypeScript would not say so: the value crosses IPC \
                 at run time"
            );
        }

        // Without this, both searches above could be green because they match
        // anything at all.
        for absent in ["accepted-maybe", "refused", "not-attempted-yet"] {
            assert!(
                !rust.contains(&format!("#[serde(rename = \"{absent}\")]")),
                "the search over this file's source matched `{absent}`, which no variant is \
                 named: every assertion above is worthless"
            );
        }
    }

    // === the settable combination ==========================================

    /// A combination shaped as `shortcuts::accept` hands one over.
    fn combination(accelerator: &str) -> Combination {
        shortcuts::accept(accelerator)
            .unwrap_or_else(|reason| panic!("`{accelerator}` must be acceptable: {reason}"))
    }

    /// A registrar that answers from a script and REMEMBERS every call.
    ///
    /// The log is the point. What this lot promises is not only that a refused
    /// combination leaves the previous one live - it is the ORDER that gets
    /// there: release, offer, and put back. An assertion on the outcome alone
    /// would pass over an implementation that never released anything.
    struct FakeRegistrar {
        /// Accelerators `register` must refuse, with what the system "said".
        refuse: Vec<(String, String)>,
        /// Accelerators `unregister` must refuse.
        refuse_release: Vec<String>,
        calls: Vec<String>,
    }

    impl FakeRegistrar {
        fn new() -> Self {
            Self {
                refuse: Vec::new(),
                refuse_release: Vec::new(),
                calls: Vec::new(),
            }
        }

        fn refusing(mut self, accelerator: &str, reason: &str) -> Self {
            self.refuse
                .push((accelerator.to_owned(), reason.to_owned()));
            self
        }

        fn refusing_release(mut self, accelerator: &str) -> Self {
            self.refuse_release.push(accelerator.to_owned());
            self
        }
    }

    impl Registrar for FakeRegistrar {
        fn unregister(&mut self, accelerator: &str) -> Result<(), String> {
            self.calls.push(format!("unregister {accelerator}"));

            if self.refuse_release.iter().any(|held| held == accelerator) {
                return Err("the plugin would not release it".to_owned());
            }
            Ok(())
        }

        fn register(&mut self, accelerator: &str) -> Result<(), String> {
            self.calls.push(format!("register {accelerator}"));

            self.refuse
                .iter()
                .find(|(held, _)| held == accelerator)
                .map_or(Ok(()), |(_, reason)| Err(reason.clone()))
        }
    }

    #[test]
    fn a_combination_the_system_takes_replaces_the_one_before_it() {
        // The ordinary path, and it is here to keep the rollback test below from
        // being green over a `change` that never changes anything.
        let current = combination("Ctrl+Shift+Digit2");
        let wanted = combination("Ctrl+Shift+KeyA");
        let mut registrar = FakeRegistrar::new();

        let outcome = change(&mut registrar, Some(&current), &wanted);

        assert_eq!(outcome, Outcome::Took(wanted.clone()));
        assert_eq!(
            registrar.calls,
            vec![
                "unregister Ctrl+Shift+Digit2".to_owned(),
                "register Ctrl+Shift+KeyA".to_owned(),
            ],
            "the previous combination must be RELEASED before the new one is offered: a \
             combination cannot be registered twice, and the collision would be reported to the \
             user as another program holding it"
        );
    }

    #[test]
    fn a_refused_combination_puts_the_previous_one_back_and_says_so() {
        // THE test of this lot. A user tries a combination another program owns;
        // what they must NOT end up with is an application that has lost the
        // shortcut it had before they touched anything.
        let current = combination("Ctrl+Shift+Digit2");
        let wanted = combination("Ctrl+Shift+KeyA");
        let mut registrar =
            FakeRegistrar::new().refusing("Ctrl+Shift+KeyA", "HotKey already registered");

        let outcome = change(&mut registrar, Some(&current), &wanted);

        assert_eq!(
            registrar.calls,
            vec![
                "unregister Ctrl+Shift+Digit2".to_owned(),
                "register Ctrl+Shift+KeyA".to_owned(),
                "register Ctrl+Shift+Digit2".to_owned(),
            ],
            "the released combination must be taken back, and IMMEDIATELY: any other order \
             leaves a window in which this application holds no capture shortcut"
        );

        let Outcome::Kept {
            refused,
            active,
            reason,
        } = outcome
        else {
            panic!("a refused combination must leave the previous one live: {outcome:?}");
        };

        assert_eq!(
            active, current,
            "the combination reported as live is not the one that was put back"
        );
        assert_eq!(
            refused, wanted,
            "the refusal must name what the user asked for, or they cannot tell which of the two \
             failed"
        );
        assert_eq!(
            reason, "HotKey already registered",
            "what the system answered must survive into the outcome"
        );
    }

    #[test]
    fn a_previous_combination_that_cannot_be_taken_back_is_reported_as_such() {
        // The honest end of the rollback. It must not be told as « your old
        // shortcut is back »: nothing is registered, and a user who believed the
        // reassuring version would press keys that do nothing.
        let current = combination("Ctrl+Shift+Digit2");
        let wanted = combination("Ctrl+Shift+KeyA");
        let mut registrar = FakeRegistrar::new()
            .refusing("Ctrl+Shift+KeyA", "HotKey already registered")
            .refusing("Ctrl+Shift+Digit2", "the hotkey thread is gone");

        let outcome = change(&mut registrar, Some(&current), &wanted);

        let Outcome::Stranded { refused, reason } = outcome else {
            panic!("with neither combination registered, nothing may claim one is: {outcome:?}");
        };

        assert_eq!(refused, wanted);
        assert!(
            reason.contains("HotKey already registered") && reason.contains("the hotkey thread"),
            "both refusals must reach the terminal: this is the one state where nothing works, \
             and the reason is all anybody has: {reason}"
        );
        assert!(
            reason.contains(&current.accelerator),
            "the combination that could not be taken back must be named: {reason}"
        );
    }

    #[test]
    fn a_combination_that_cannot_be_released_leaves_the_new_one_unoffered() {
        // If the live combination will not go, offering the new one would either
        // collide with it or leave two shortcuts registered. Neither is
        // acceptable, and the previous one IS still live - which is exactly what
        // `Kept` means.
        let current = combination("Ctrl+Shift+Digit2");
        let wanted = combination("Ctrl+Shift+KeyA");
        let mut registrar = FakeRegistrar::new().refusing_release("Ctrl+Shift+Digit2");

        let outcome = change(&mut registrar, Some(&current), &wanted);

        assert_eq!(
            registrar.calls,
            vec!["unregister Ctrl+Shift+Digit2".to_owned()],
            "nothing may be offered to the system once the live combination refused to go"
        );
        assert!(
            matches!(outcome, Outcome::Kept { ref active, .. } if *active == current),
            "the previous combination was never released, so it is still the live one: {outcome:?}"
        );
    }

    #[test]
    fn asking_for_the_combination_that_is_already_live_touches_nothing() {
        // Windows refuses a combination it has already handed over. Releasing
        // and re-taking would work, but the failure mode if anything went wrong
        // in between is losing a shortcut over a setting that changed NOTHING.
        let current = combination("Ctrl+Shift+Digit2");
        let mut registrar = FakeRegistrar::new();

        let outcome = change(&mut registrar, Some(&current), &current);

        assert_eq!(outcome, Outcome::Took(current));
        assert!(
            registrar.calls.is_empty(),
            "the operating system was disturbed for a setting that did not change: {:?}",
            registrar.calls
        );
    }

    #[test]
    fn a_launch_with_no_live_combination_can_still_take_one() {
        // The state a user reaches after a refusal at startup: nothing is
        // registered, and the settings screen is the way out of it. There is
        // nothing to release and nothing to fall back on.
        let wanted = combination("Ctrl+Shift+KeyA");
        let mut registrar = FakeRegistrar::new();

        assert_eq!(
            change(&mut registrar, None, &wanted),
            Outcome::Took(wanted.clone())
        );
        assert_eq!(registrar.calls, vec!["register Ctrl+Shift+KeyA".to_owned()]);

        let mut refusing = FakeRegistrar::new().refusing("Ctrl+Shift+KeyA", "held elsewhere");
        assert!(
            matches!(
                change(&mut refusing, None, &wanted),
                Outcome::Stranded { .. }
            ),
            "with no previous combination there is nothing to fall back on, and saying otherwise \
             would promise a shortcut that does not exist"
        );
    }

    #[test]
    fn the_settings_file_is_only_written_for_a_combination_that_was_taken() {
        // Held on the mapping rather than on the disk, because writing needs an
        // AppHandle. `saved` is meaningful for ONE outcome, and a screen that
        // read it on the others would tell the user their refused combination
        // had been remembered.
        let refused = combination("Ctrl+Shift+KeyA");
        let active = combination("Ctrl+Shift+Digit2");

        assert!(matches!(
            Outcome::Took(active.clone()).into_change(true),
            ShortcutChange::Changed { saved: true, .. }
        ));
        assert!(matches!(
            Outcome::Took(active.clone()).into_change(false),
            ShortcutChange::Changed { saved: false, .. }
        ));

        // Whatever the caller passes, neither of these carries the field: there
        // is nothing to save, so there is nothing to claim about saving.
        assert!(matches!(
            Outcome::Kept {
                refused: refused.clone(),
                active: active.clone(),
                reason: String::new(),
            }
            .into_change(true),
            ShortcutChange::Kept { .. }
        ));
        assert!(matches!(
            Outcome::Stranded {
                refused,
                reason: String::new(),
            }
            .into_change(true),
            ShortcutChange::Stranded { .. }
        ));
    }

    /// The tag serde emits for a change, one arm per variant.
    ///
    /// Exhaustive on purpose, like [`wire_tag`]: a fourth variant added to
    /// [`ShortcutChange`] stops this file compiling.
    fn change_tag(change: &ShortcutChange) -> &'static str {
        match change {
            ShortcutChange::Changed { .. } => "changed",
            ShortcutChange::Kept { .. } => "kept",
            ShortcutChange::Stranded { .. } => "stranded",
        }
    }

    #[test]
    fn the_three_change_tags_are_the_ones_serde_emits_and_the_frontend_reads() {
        // Same instrument, and the same honest limit, as its neighbour above:
        // it holds the `#[serde(rename = "…")]` attributes in THIS file against
        // the literals `src/shortcuts.ts` switches on. It does not observe the
        // JSON serde produces.
        let rust = include_str!("shortcut.rs");
        let typescript = include_str!("../../src/shortcuts.ts");

        let active = combination("Ctrl+Shift+Digit2");
        let changes = [
            ShortcutChange::Changed {
                active: active.clone(),
                saved: true,
            },
            ShortcutChange::Kept {
                refused: active.clone(),
                active: active.clone(),
                reason: String::new(),
            },
            ShortcutChange::Stranded {
                refused: active,
                reason: String::new(),
            },
        ];

        for change in &changes {
            let tag = change_tag(change);

            assert!(
                rust.contains(&format!("#[serde(rename = \"{tag}\")]")),
                "no variant of ShortcutChange is renamed to `{tag}`, so serde will not emit it"
            );
            assert!(
                typescript.contains(&format!("'{tag}'")),
                "src/shortcuts.ts does not switch on `{tag}`. The settings screen would receive \
                 an answer it cannot read, at run time, with nothing in either language to say so"
            );
        }

        for absent in ["changed-maybe", "rolled-back", "lost"] {
            assert!(
                !rust.contains(&format!("#[serde(rename = \"{absent}\")]")),
                "the search over this file's source matched `{absent}`, which no variant is \
                 named: every assertion above is worthless"
            );
        }
    }

    // --- what a launch does with the settings file --------------------------

    /// The registry's own combination, as `install` builds its fallback.
    fn registry_fallback() -> Combination {
        let entry = capture_entry().expect("the registry must hold the capture entry");

        Combination {
            accelerator: entry.accelerator.to_owned(),
            keys: entry.keys.iter().map(|key| (*key).to_owned()).collect(),
        }
    }

    #[test]
    fn a_first_launch_uses_the_registry_and_says_nothing_about_it() {
        // No file is the ORDINARY state, not a fault. A line printed on every
        // first launch would train whoever reads that terminal to skip the ones
        // that matter.
        let fallback = registry_fallback();
        let (chosen, note) = startup_choice(&SettingsRead::Absent, &fallback);

        assert_eq!(chosen, fallback);
        assert_eq!(note, None, "a missing settings file is not worth a warning");
    }

    #[test]
    fn a_saved_combination_is_the_one_that_gets_offered() {
        // Without this, every assertion below could be green over a
        // `startup_choice` that ignores the file entirely - and a settings
        // screen that never survives a restart.
        let fallback = registry_fallback();
        let (chosen, note) =
            startup_choice(&SettingsRead::Chosen("Ctrl+Alt+KeyQ".to_owned()), &fallback);

        assert_eq!(chosen.accelerator, "Ctrl+Alt+KeyQ");
        assert_eq!(chosen.keys, vec!["Ctrl", "Alt", "Q"]);
        assert_eq!(note, None, "an ordinary saved choice is not a warning");
    }

    #[test]
    fn a_damaged_settings_file_falls_back_and_names_both_the_fault_and_the_fallback() {
        // THE requirement: a file that is unreadable, empty or incoherent must
        // not stop the application. Both rows below reach the same answer by
        // different roads - one the FILE could not be read, the other its
        // contents are no longer a combination this application will register.
        let fallback = registry_fallback();

        for (saved, fault) in [
            (
                SettingsRead::Unusable("settings.json is unusable: it holds no key".to_owned()),
                "it holds no key",
            ),
            (
                SettingsRead::Chosen("Ctrl+Shift+NotAKey".to_owned()),
                "NotAKey",
            ),
            (SettingsRead::Chosen("Digit2".to_owned()), "modifier"),
        ] {
            let (chosen, note) = startup_choice(&saved, &fallback);

            assert_eq!(
                chosen, fallback,
                "the application must start with the combination it ships with rather than with \
                 none: {saved:?}"
            );

            let note = note.expect("a fallback the user did not ask for must be announced");

            assert!(
                note.contains(fault),
                "the note says nothing a reader could act on: {note}"
            );
            assert!(
                note.contains(&fallback.accelerator),
                "the note must name the combination that IS live, or the user cannot tell what \
                 to press: {note}"
            );
            assert!(
                note.contains("left exactly as it is"),
                "the note must say the file was not overwritten: a user whose choice vanished \
                 needs to know it is still on disk to be looked at: {note}"
            );
        }
    }

    #[test]
    fn what_is_registered_and_what_was_merely_offered_are_two_questions() {
        // `describe_shortcuts` publishes the ATTEMPTED combination, so a
        // refusal note can name it; `change` needs the ACTIVE one, so it knows
        // what to release. Confusing the two would make a refused combination
        // look live - and `change` would try to unregister something the system
        // never gave us.
        let combination = combination("Ctrl+Shift+Digit2");

        let accepted = CaptureShortcut::new(
            ShortcutStatus::Accepted {
                accelerator: combination.accelerator.clone(),
            },
            Some(combination.clone()),
            true,
        );
        assert_eq!(accepted.active().as_ref(), Some(&combination));
        assert_eq!(accepted.attempted().as_ref(), Some(&combination));

        let refused = CaptureShortcut::new(
            ShortcutStatus::RefusedBySystem {
                accelerator: combination.accelerator.clone(),
                reason: "HotKey already registered".to_owned(),
            },
            Some(combination.clone()),
            true,
        );
        assert_eq!(
            refused.active(),
            None,
            "a combination the system refused is not registered, and treating it as live would \
             have `change` release a shortcut nobody holds"
        );
        assert_eq!(
            refused.attempted().as_ref(),
            Some(&combination),
            "it is still the combination this application asked for, and the launcher's refusal \
             note names it"
        );

        let never = CaptureShortcut::new(
            ShortcutStatus::NotAttempted {
                reason: "the plugin failed to load".to_owned(),
            },
            None,
            false,
        );
        assert_eq!(never.active(), None);
        assert_eq!(never.attempted(), None);
        assert!(
            !never.plugin_loaded,
            "a launch that never loaded the plugin must be marked as such: `global_shortcut()` \
             PANICS on an unloaded plugin, and `change_capture_shortcut` reads this flag to \
             refuse rather than take the application down from an IPC thread"
        );
    }
}
