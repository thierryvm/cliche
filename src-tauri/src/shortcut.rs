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

use std::str::FromStr;
use std::sync::atomic::{AtomicUsize, Ordering};

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{
    Builder as GlobalShortcutBuilder, GlobalShortcutExt, Shortcut, ShortcutState,
};

use crate::timing::Timings;

/// The capture shortcut, in the plugin's own syntax.
///
/// ONE place on purpose. The same combination is already announced to the user
/// in `src/design/Showcase.tsx` ("Ctrl + Maj + 2 decoupe une zone"), and lot 2
/// turns this constant into the registry the in-app help derives from. Two
/// values for one fact always drift apart.
///
/// `Digit2` rather than `2`: the parser accepts both, but a W3C code names a
/// PHYSICAL key rather than the character it produces. On the Belgian AZERTY
/// keyboard this project is used on, that key types `e` with an accent
/// unshifted and `2` with Shift - so `Ctrl+Shift+Digit2` is exactly the
/// "Ctrl + Maj + 2" the interface promises, and stays that key whatever the
/// active layout.
pub const CAPTURE_SHORTCUT: &str = "Ctrl+Shift+Digit2";

/// How many finished runs before the report prints itself, with no further
/// action from whoever is measuring.
///
/// 20 and not 10: `timing.rs` computes p95 by nearest rank, and on 10 samples
/// `ceil(0.95 * 10) = 10` makes the p95 the maximum - a number that carries no
/// more information than "the worst of ten".
const RUNS_PER_REPORT: usize = 20;

/// Parses [`CAPTURE_SHORTCUT`].
///
/// Split out because it is the one part of this module that needs no event
/// loop: a unit test can hold the constant against the plugin's real parser.
pub fn capture_shortcut() -> Result<Shortcut, String> {
    Shortcut::from_str(CAPTURE_SHORTCUT)
        .map_err(|error| format!("`{CAPTURE_SHORTCUT}` is not a valid shortcut: {error}"))
}

/// Whether the run just filed completes a batch worth reporting on.
fn report_due(run_number: usize) -> bool {
    run_number > 0 && run_number % RUNS_PER_REPORT == 0
}

/// The line printed when the shortcut could not be taken.
///
/// Pure, so its wording is under test. This message is the only thing standing
/// between "another program already owns Ctrl+Shift+2" and an application that
/// looks perfectly fine and does nothing at all - the worst of both worlds.
fn registration_failure(reason: &str) -> String {
    format!(
        "[cliche] shortcut: FAILED to take {CAPTURE_SHORTCUT} ({reason}). \
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
    let shortcut = capture_shortcut().map_err(|reason| registration_failure(&reason))?;

    // The plugin is loaded here, next to its only use, rather than in the
    // builder chain: `install` then either wires the shortcut completely or
    // fails with one message, and `lib.rs` has a single line to read.
    app.plugin(GlobalShortcutBuilder::new().build())
        .map_err(|error| registration_failure(&format!("plugin failed to load: {error}")))?;

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

            // `try_state`, never `state`: `state` panics when the type was
            // never managed, and a panic on this thread ends the application.
            // A missing instrument is a diagnostic, not a crash.
            let Some(timings) = app.try_state::<Timings>() else {
                eprintln!(
                    "[cliche] shortcut: no timing instrument is managed; this press was not measured"
                );
                return;
            };

            // t0 IS THIS LINE - and that is a limitation, not a design choice.
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
            timings.begin_run();
            timings.mark("handler");
            timings.finish_run();

            // `saturating_add` for the same reason as everything else on this
            // path: a debug overflow panic here would kill the application over
            // a counter.
            let run = runs.fetch_add(1, Ordering::Relaxed).saturating_add(1);
            println!("[cliche] shortcut: run {run}");

            if report_due(run) {
                // The point of the exercise: press 20 times, read the terminal.
                // No command to type, no window to open.
                for line in timings.report().lines() {
                    println!("[cliche] {line}");
                }
            }
        })
        .map_err(|error| registration_failure(&error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri_plugin_global_shortcut::{Code, Modifiers};

    #[test]
    fn the_announced_shortcut_is_what_the_plugin_parser_actually_accepts() {
        // The value the interface promises the user, held against the parser
        // that will have to register it. A typo in the constant fails here
        // rather than at run time, in a message nobody is watching for.
        let shortcut = capture_shortcut().expect("the capture shortcut must parse");

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
        let message = registration_failure("HotKey already registered");

        assert!(
            message.contains(CAPTURE_SHORTCUT),
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
