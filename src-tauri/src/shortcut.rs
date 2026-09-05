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

/// The line printed when the shortcut could not be taken.
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

/// Loads the plugin and binds the capture shortcut to the timing handler.
///
/// Returns the ready-to-print failure line rather than printing it here: the
/// caller is the one that decides the application keeps going, so the caller is
/// where that decision should be readable.
pub fn install(app: &AppHandle) -> Result<(), String> {
    let entry = capture_entry()?;

    // Bound once so that all three ways this can fail name the SAME
    // combination - the entry's, not a constant that could have moved.
    let refused = |reason: &str| registration_failure(entry.accelerator, reason);

    let shortcut = shortcuts::parse(entry).map_err(|reason| refused(&reason))?;

    // The plugin is loaded here, next to its only use, rather than in the
    // builder chain: `install` then either wires the shortcut completely or
    // fails with one message, and `lib.rs` has a single line to read.
    app.plugin(GlobalShortcutBuilder::new().build())
        .map_err(|error| refused(&format!("plugin failed to load: {error}")))?;

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
        .map_err(|error| refused(&error.to_string()))
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
        let entry = capture_entry().expect("the registry must hold the capture entry");
        let message = registration_failure(entry.accelerator, "HotKey already registered");

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
}
