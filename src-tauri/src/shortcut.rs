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

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, ShortcutState,
};

use crate::shortcuts::{self, ShortcutEntry};

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
        accelerator: &'static str,
    },
    /// The operating system refused the combination, with the reason it gave.
    ///
    /// The reason is the OS's own words, in English, and it is meant for the
    /// terminal and the developer console - never for the screen. What the
    /// screen says is decided in `src/shortcut-hint.ts`.
    #[serde(rename = "refused-by-system")]
    RefusedBySystem {
        /// The combination that was refused. Named, so a user can free it.
        accelerator: &'static str,
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

/// Loads the plugin and binds the capture shortcut to the timing handler.
///
/// Returns WHAT HAPPENED rather than whether it worked. The three states of
/// [`ShortcutStatus`] are the three answers this function can come back with,
/// and every one of them ends up on the screen through
/// `shortcuts::describe_shortcut_status`: this return value is no longer a line
/// for the terminal, it is the fact the launcher draws.
///
/// Nothing is printed here, and nothing panics: the caller prints
/// `status.terminal_line()` and puts the status into managed state.
pub fn install(app: &AppHandle) -> ShortcutStatus {
    let entry = match capture_entry() {
        Ok(entry) => entry,
        Err(reason) => return ShortcutStatus::NotAttempted { reason },
    };

    // Every branch below that gives up before the operating system is asked is
    // `NotAttempted`: the combination was never offered, so nothing external
    // refused it. Only `on_shortcut` can produce a refusal.
    let shortcut = match shortcuts::parse(entry) {
        Ok(shortcut) => shortcut,
        Err(reason) => return ShortcutStatus::NotAttempted { reason },
    };

    // The plugin is loaded here, next to its only use, rather than in the
    // builder chain: `install` then either wires the shortcut completely or
    // says in one value why it did not, and `lib.rs` has a single line to read.
    if let Err(error) = app.plugin(GlobalShortcutBuilder::new().build()) {
        return ShortcutStatus::NotAttempted {
            reason: format!("the global-shortcut plugin failed to load: {error}"),
        };
    }

    // Owned by the closure, which is `Fn`: an atomic is what lets it count
    // without `&mut`. `Relaxed` because this counter is only ever compared with
    // itself - each `fetch_add` hands back a distinct value, which is all a run
    // number needs.
    let runs = AtomicUsize::new(0);

    app.global_shortcut()
        .on_shortcut(shortcut, move |app, _shortcut, event| {
            // The plugin reports both edges. Only the press starts a capture;
            // measuring the release would fold in how long the user held the
            // keys down, which is not our latency.
            if !matches!(event.state(), ShortcutState::Pressed) {
                return;
            }

            // `saturating_add` for the same reason as everything else on this
            // path: a debug overflow panic here would kill the application over
            // a counter. Printed BEFORE the capture, so that a press which then
            // fails somewhere still leaves a trace in the terminal.
            let press = runs.fetch_add(1, Ordering::Relaxed).saturating_add(1);
            println!("[cliche] shortcut: press {press}");

            // t0 IS THE FIRST LINE OF `perform_capture` - and that is a
            // limitation, not a design choice.
            //
            // "shortcut pressed -> handler entered" is NOT measurable from
            // inside this process. Nothing gives us the instant the key went
            // down: the event carries a hotkey id and a state, no timestamp.
            // Our first possible clock reading is the entry of this handler, so
            // the whole trip "physical key -> Windows low-level hook ->
            // global-hotkey thread -> this closure" lies OUTSIDE every figure
            // the instrument prints.
            //
            // Read the 150 ms budget accordingly: it is counted from HERE, not
            // from the user's finger. The unmeasured part is unknown, and
            // unknown is not the same as zero.
            //
            // The whole pipeline lives in `veil::perform_capture` rather than
            // in this closure for one reason: the automated benchmark has to
            // call THE SAME code, or it would be measuring a different program.
            // The press counter above is not the measurement - a run is filed
            // when the webview acknowledges the paint, and `veil.rs` prints the
            // report every twenty of those.
            crate::veil::perform_capture(app);
        })
        .map_or_else(
            |error| ShortcutStatus::RefusedBySystem {
                accelerator: entry.accelerator,
                reason: error.to_string(),
            },
            |()| ShortcutStatus::Accepted {
                accelerator: entry.accelerator,
            },
        )
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
        let shortcut = shortcuts::parse(entry).expect("the capture shortcut must parse");

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
            accelerator: entry.accelerator,
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
                accelerator: "Ctrl+Shift+Digit2",
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
                accelerator: "Ctrl+Shift+Digit2",
            },
            ShortcutStatus::RefusedBySystem {
                accelerator: "Ctrl+Shift+Digit2",
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
}
