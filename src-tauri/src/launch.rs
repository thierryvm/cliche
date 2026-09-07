//! Starting a capture from the LAUNCHER, with the mouse.
//!
//! One command, and it exists because the home screen's headline tile was drawn
//! in its built state while reaching nothing at all: until 6 September 2026
//! `veil::perform_capture` had exactly one caller outside the benchmark, the
//! global-shortcut handler in `shortcut.rs`. A button that looks pressable and
//! is not is a defect, not a detail.
//!
//! # It calls `veil::perform_capture`, and NOTHING else PHOTOGRAPHS
//!
//! Not a variant of it, not a copy of it with one line changed. The whole point
//! of `perform_capture` living in `veil.rs` rather than inside the shortcut
//! closure is that the benchmark and the shortcut measure THE SAME code; a
//! third entry point that quietly did something else would end that, and the
//! difference would show up as a latency figure nobody could attribute.
//!
//! What this module now does BEFORE that call - hide the main window, wait,
//! capture, and put the window back afterwards - is the subject of the next
//! section, and it is deliberately all on THIS side of the call. Not one line
//! of `perform_capture` changed shape for it.
//!
//! # THE WINDOW IS HIDDEN FIRST, AND ONLY ON THIS PATH - decided 6 September 2026
//!
//! `perform_capture` photographs the screen BEFORE it hands anything to the
//! veil page (the ORDER comment in `veil.rs` has the reasoning), which is what
//! stops the veil from photographing itself. It says nothing about the MAIN
//! window, because until this file existed the main window was never in front
//! when a capture started.
//!
//! It is now. A click on the tile is, by definition, a click on a focused
//! Cliche window, and the frame frozen a few milliseconds later would contain
//! that window - glass, title bar, the tile still lit by the pointer, and
//! whatever the user actually wanted to capture hidden behind all of it. This
//! header used to say that WHICH remedy to apply was a product decision and not
//! the code's to take. Thierry took it on 6 September 2026: hide the window.
//!
//! HIDING IT WAS NOT ENOUGH, and the user said so twice. Windows animates a
//! window on its way out, so a screen photographed 120 ms later could still hold
//! Cliche, half transparent, over the desktop. Since 7 September 2026 this path
//! goes through `compositor.rs`, which takes the fade away and then turns the
//! wait below from a blind sleep into a DEADLINE - real presentations first,
//! then only what is left of [`RECOMPOSE_SETTLE`]. The cost of a click did not
//! change. That module's header is where the reasoning, the portability rule it
//! obeys, and the list of what is NOT measured all live.
//!
//! **The global-shortcut path is untouched, and that is not an oversight.** It
//! is the reference path of the 150 ms budget, it does not have this problem -
//! the user is in another window when they press the keys - and putting a
//! `hide()` plus a wait in front of it would move the goalposts of every figure
//! `docs/MESURES.md` holds. So `shortcut.rs` was not opened for this lot.
//!
//! Two consequences of the split, said here rather than discovered:
//!
//! 1. The two paths no longer take the same time to reach the frozen frame.
//!    This one is slower by [`RECOMPOSE_SETTLE`] plus whatever `hide()` costs,
//!    and it PRINTS that figure - see [`capture_region`]. The shortcut path
//!    prints nothing of the sort, which is one more way to tell the two apart
//!    in a terminal.
//! 2. `Timings` still cannot tell them apart: both file into the same instrument,
//!    from `perform_capture` onwards. Use ONE of the two from a fresh launch
//!    before reading a median.
//!
//! A THIRD difference, in this path's favour and worth knowing before anybody
//! "fixes" it: `veil_decoded` calls `set_focus` on the veil, and Windows only
//! lets a process raise a window when it is already the foreground process. On
//! this path Cliche IS the foreground process, because the user just clicked
//! it. The shortcut path is the one that has to fight for that, not this one.
//! Read, not measured - and note that the `hide()` above may well have given
//! that foreground status away, which is one more thing nobody has observed.
//!
//! # It is not guarded by its distance from anything
//!
//! Unlike `shortcut.rs`, which declares no command and is unreachable from a
//! webview, this module opens an `invoke` frontier onto the capture pipeline.
//! Two things stand on it and both are deliberate: `capabilities/default.json`
//! grants `allow-capture-region` to the `main` window alone, and the check
//! below refuses any other label with a line that names both windows. `ipc.rs`
//! explains why one is not enough.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, Webview};

use crate::compositor;
use crate::ipc;
use crate::veil;

/// How long Windows is given to repaint the desktop WITHOUT Cliche in it,
/// between `hide()` returning and the screen being photographed.
///
/// # THIS VALUE IS NOT MEASURED, and that word is exact
///
/// Nobody has observed how long the desktop window manager takes to present a
/// frame without this window on this machine. What follows is the REASONING
/// that produced 120 ms, so that the next reader can argue with it rather than
/// re-derive it:
///
/// - `hide()` returns when the window has been asked to go, not when the
///   screen behind it has been redrawn. Everything after that is the
///   compositor's, and nothing in this process is told when it is done.
/// - One presentation is 16.7 ms at 60 Hz and 8.3 ms at 120 Hz. 120 ms is
///   therefore about seven frames on the slower of the two - generous on
///   purpose.
/// - **The two ways of being wrong do not cost the same.** Too SHORT and the
///   frozen frame catches the window half erased: a picture that is wrong in a
///   way the user reads as a bug, on a path where they were expecting a
///   screenshot. Too LONG and a mouse-driven capture takes a tenth of a second
///   longer, on the one path that carries no budget - the 150 ms of
///   `docs/MESURES.md` is the SHORTCUT's, and this path is deliberately not in
///   it. An asymmetric risk is bought off on the expensive side.
///
/// # What the terminal line does and does NOT settle
///
/// [`capture_region`] prints what this path really cost. That is the PRICE, and
/// it is a measurement. It says nothing about whether the wait is long ENOUGH:
/// sufficiency is read off the IMAGE - put a recognisable window under Cliche,
/// click the tile, drag over the part of the screen Cliche was covering, and
/// look at what lands on the clipboard. A ghost of the title bar in it means
/// this number is too small.
///
/// # SINCE 7 SEPTEMBER 2026 IT IS A FLOOR, and the value did not move
///
/// [`capture_region`] no longer sleeps this long; it hands the number to
/// `compositor::settle_after_hide` as a DEADLINE. That function waits for real
/// presentations first and then sleeps only what is LEFT of the 120 ms, so a
/// click on the tile costs exactly what it cost before and the evidence is
/// bought for nothing.
///
/// Which is also why the number stays where it is. The compositor's answer is
/// EVIDENCE, not proof: `compositor.rs`'s header quotes Microsoft's own page to
/// show that a flush speaks for what THIS process queued in DirectX, and our
/// `hide()` is not that. Nothing about waiting for presentations licenses
/// lowering this figure, and nothing has measured it either.
const RECOMPOSE_SETTLE: Duration = Duration::from_millis(120);

/// One presentation at 60 Hz, the slower of the two refresh rates
/// [`RECOMPOSE_SETTLE`] is argued against.
///
/// `cfg(test)` for the reason `veil::DECODE_P95_MEASURED` carries it: this is a
/// number the JUSTIFICATION uses, not one the application reads, and compiled
/// into the binary `clippy -D warnings` would rightly call it dead.
#[cfg(test)]
const FRAME_60HZ: Duration = Duration::from_micros(16_667);

/// What this application knows about the main window when a capture ends.
///
/// Pure data, and deliberately apart from the atomic that carries it, for the
/// reason `veil::may_show` is apart from the atomics that apply it: a decision
/// over one value can be put to every row of its own table, and a decision that
/// needs an event loop cannot be tested at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MainWindow {
    /// THIS path hid it, before the capture, and nothing has put it back yet.
    HiddenForCapture,
    /// Nobody here touched it. Either the capture came from the global
    /// shortcut, which hides nothing, or the restore for a tile capture has
    /// already run. Whatever state the window is in, it is the USER's.
    LeftAlone,
}

/// Whether the end of a capture must put the main window back up.
///
/// # The `LeftAlone` row is the whole reason this is a function
///
/// A capture started from the global shortcut hid nothing. The user may have
/// minimised Cliche themselves, hours ago, and be working in another
/// application - which is exactly the case Ctrl + Maj + 2 exists for. Raising
/// the window over their work at the end of that capture would be this lot
/// breaking the path it promised not to touch.
///
/// So the rule is not "show the main window when a capture ends". It is "undo
/// what this file did, and nothing else".
pub fn should_restore(main: MainWindow) -> bool {
    matches!(main, MainWindow::HiddenForCapture)
}

/// The debt this path owes the user: one `show()` for every `hide()`.
///
/// Managed by Tauri, from `setup()`. A single flag rather than a run number,
/// and the reason is that `perform_capture` deliberately hands none back - it
/// takes no `Result` at all, because a failed run counted as a fast one would
/// poison the median. What this flag means is therefore not "run N is in
/// flight" but "a window of ours is off the screen and has not been put back",
/// which is the thing that actually has to be true again before the user is
/// left alone.
///
/// That is also why a second capture starting while the flag is set is not a
/// problem: the debt is the same debt, and whichever capture ends last pays it.
#[derive(Debug, Default)]
pub struct MainWindowClaim {
    hidden: AtomicBool,
}

impl MainWindowClaim {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that the main window has just been hidden by this path.
    ///
    /// Called only after a `hide()` that RETURNED `Ok`: a claim taken for a
    /// hide that never happened would make the next end-of-capture raise a
    /// window the user had put away themselves.
    fn owe_a_restore(&self) {
        self.hidden.store(true, Ordering::Relaxed);
    }

    /// Takes the debt, if there is one, and leaves none behind.
    ///
    /// `swap` and not a load-then-store: several paths end a capture, and two
    /// of them can arrive - a `veil_decoded` that failed to show and the
    /// fallback timer behind it. Exactly one may come away with
    /// [`MainWindow::HiddenForCapture`]. `Relaxed` is enough for the same
    /// reason `Veil::claim_show` gives: nothing is published through this
    /// location, and a read-modify-write on one atomic is totally ordered
    /// whatever ordering is asked for.
    fn take(&self) -> MainWindow {
        if self.hidden.swap(false, Ordering::Relaxed) {
            MainWindow::HiddenForCapture
        } else {
            MainWindow::LeftAlone
        }
    }
}

/// Takes the main window off the screen. `true` when it really went.
///
/// The claim is read BEFORE anything is hidden, and that order is the point: a
/// window hidden with nothing to record the debt is a window nobody will ever
/// put back, and the user would be left with an application that has vanished.
/// Without the instrument, this path does the OLD thing - it captures with
/// Cliche in the frame - and says so.
fn hide_main_window(app: &AppHandle) -> bool {
    let Some(claim) = app.try_state::<MainWindowClaim>() else {
        eprintln!(
            "[cliche] launch: no window claim is managed, so nothing could put the window back \
             afterwards; it was NOT hidden and this capture will contain Cliche itself"
        );
        return false;
    };

    let Some(window) = app.get_webview_window(ipc::MAIN_WINDOW_LABEL) else {
        eprintln!(
            "[cliche] launch: the `{}` window does not exist; this capture will contain whatever \
             is on screen",
            ipc::MAIN_WINDOW_LABEL
        );
        return false;
    };

    if let Err(error) = window.hide() {
        eprintln!(
            "[cliche] launch: could not hide the window before capturing: {error}. The capture \
             goes ahead and will contain Cliche itself"
        );
        return false;
    }

    claim.owe_a_restore();
    true
}

/// Puts the main window back - if, and only if, this file took it away.
///
/// # Where this is called from, and why it is spread over so many places
///
/// Every place a capture ENDS, in `veil.rs`: `hide_veil` (which is the single
/// door every hide of the veil goes through, so it covers the confirmation,
/// Escape, and an `eval` that never reached the page), plus the failure
/// branches that abandon a run without ever putting a veil on screen - a
/// capture that failed, an encode that failed, a `show()` that failed, and the
/// fallback timer's own two. Those last ones are exactly the case a single
/// tidy call site would have missed: the window is off the screen, there is no
/// veil, and nothing else will ever run.
///
/// A capture that FAILED at the clipboard is deliberately not in that list. The
/// veil stays up on purpose there (`veil::AfterSelection`) so the selection can
/// be corrected; the capture is not over, and Escape is what ends it.
///
/// # `show()` and not `set_focus()`
///
/// The window comes back, it does not come back IN FRONT. After a successful
/// copy the user's next move is a paste, into something that is not Cliche, and
/// stealing the foreground at that exact moment would be this application
/// interrupting the thing it just helped with. Visibility is what was taken
/// away; visibility is what is given back.
///
/// Nothing here may panic: it runs on whatever thread ended the capture,
/// including the global-shortcut thread and the fallback timer's.
pub fn restore_main_window(app: &AppHandle) {
    let Some(claim) = app.try_state::<MainWindowClaim>() else {
        return;
    };

    // Taken unconditionally, decided separately. The atomic answers "was there
    // a debt"; `should_restore` is the rule, and it is under test.
    if !should_restore(claim.take()) {
        return;
    }

    let Some(window) = app.get_webview_window(ipc::MAIN_WINDOW_LABEL) else {
        eprintln!(
            "[cliche] launch: the `{}` window no longer exists; it cannot be put back",
            ipc::MAIN_WINDOW_LABEL
        );
        return;
    };

    if let Err(error) = window.show() {
        eprintln!(
            "[cliche] launch: could not put the window back after the capture: {error}. It is \
             hidden with no other way back than restarting Cliche"
        );
        return;
    }

    println!("[cliche] launch: the window is back");
}

/// Starts a region capture, the way the tile does.
///
/// # It no longer runs on the calling thread, and that is deliberate
///
/// Everything below happens on a thread of its own, so `Ok(())` here means "a
/// capture was SCHEDULED", not even "a capture was started". Two reasons, and
/// the second is the one that decided it:
///
/// 1. [`RECOMPOSE_SETTLE`] is a sleep, and sleeping in a command blocks the
///    thread Tauri chose to run it on for a tenth of a second.
/// 2. Tauri runs a SYNCHRONOUS command on the main thread unless it is declared
///    `async` or `#[tauri::command(async)]`. A sleep there would block the
///    event loop the `hide()` and the veil's `show()` both have to go through.
///    That reading has not been confirmed against this version's source, which
///    is precisely why the sleep is not left where it could matter: a thread of
///    our own is correct under either answer.
///
/// A failure INSIDE the pipeline is not reported here either, for the reason
/// `perform_capture` takes no `Result` at all: every failure it meets abandons
/// the run and prints, because a failed run counted as a fast one would poison
/// the median. The only error this can return is the refusal below - which is
/// still returned SYNCHRONOUSLY, before anything is spawned, so a call from the
/// wrong window never reaches the screen.
///
/// # The line it prints
///
/// One line per tile capture, prefixed `[cliche] launch:`, carrying the time
/// really spent between the start of the hide and `perform_capture` returning.
/// It exists because [`RECOMPOSE_SETTLE`] is a cost this path pays on every
/// click and a reasoned number rather than a measured one; the terminal is
/// where it stops being an assumption.
///
/// **Main window only.** The veil never starts a capture - it is what a capture
/// puts on screen, and a `capture_region` from inside it would restart the
/// pipeline while the screen is already frozen.
#[tauri::command]
pub fn capture_region(app: AppHandle, webview: Webview) -> Result<(), String> {
    ipc::ensure_from(webview.label(), ipc::MAIN_WINDOW_LABEL, "capture_region")?;

    // Nothing in this closure may panic, for the reason `veil.rs`'s header
    // gives: a panic on a worker thread takes the application down, and losing
    // the app to a screenshot is a bad trade. `perform_capture` is written for
    // exactly that constraint and needs nothing added here.
    std::thread::spawn(move || {
        let started = Instant::now();

        let settled = if hide_main_window(&app) {
            // A DEADLINE, not a sleep. `settle_after_hide` waits for real
            // presentations and then sleeps only the rest of RECOMPOSE_SETTLE,
            // so this branch still ends a flat 120 ms after it began - see that
            // constant's own comment, which the change of 7 September 2026 did
            // not move by a millisecond.
            compositor::settle_after_hide(RECOMPOSE_SETTLE)
        } else {
            // Nothing was hidden, so there is nothing to wait for. The capture
            // still happens - with Cliche in it, as it did before this lot -
            // and `hide_main_window` has already said why on the terminal.
            compositor::Settled::NOTHING
        };

        veil::perform_capture(&app);

        // BOTH halves of the wait, and that is the point of the line. Nobody
        // knows which of the two mechanisms is doing the work here: the
        // presentations may cost nothing, in which case this is the blind sleep
        // of yesterday, or they may cost most of the floor. One total would hide
        // exactly the figure that settles it.
        println!(
            "[cliche] launch: tile capture, {total:.1} ms from hiding the window to \
             perform_capture returning; the wait for Windows to recompose the desktop without \
             Cliche in it was {presented:.1} ms of real presentations plus {slept:.1} ms of \
             sleep to reach the floor",
            total = started.elapsed().as_secs_f64() * 1_000.0,
            presented = settled.presented.as_secs_f64() * 1_000.0,
            slept = settled.slept.as_secs_f64() * 1_000.0,
        );
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::veil::VEIL_WINDOW_LABEL;

    #[test]
    fn the_veil_window_cannot_start_a_capture() {
        // The refusal, held at the only place a test can reach it: constructing
        // a `Webview` needs a running event loop, a label does not. The ACL says
        // the same thing a second time, and `ipc.rs` puts THAT question to the
        // shipped `RuntimeAuthority`.
        let refusal = ipc::ensure_from(VEIL_WINDOW_LABEL, ipc::MAIN_WINDOW_LABEL, "capture_region")
            .expect_err("the veil must not be able to restart the pipeline it is the result of");

        assert!(refusal.contains("capture_region"), "{refusal}");
        assert!(refusal.contains(VEIL_WINDOW_LABEL), "{refusal}");
        assert!(refusal.contains(ipc::MAIN_WINDOW_LABEL), "{refusal}");
        assert!(
            refusal.contains("REFUSED"),
            "the line must say the capture did not start: {refusal}"
        );
    }

    #[test]
    fn the_launcher_may_start_one() {
        assert!(
            ipc::ensure_from(
                ipc::MAIN_WINDOW_LABEL,
                ipc::MAIN_WINDOW_LABEL,
                "capture_region"
            )
            .is_ok(),
            "the tile this command exists for is in the main window; refusing it there \
             would leave the button as mute as it was before"
        );
    }

    #[test]
    fn a_capture_that_hid_nothing_never_raises_a_window() {
        // THE row this rule exists for. A capture started with Ctrl + Maj + 2
        // hides nothing, so the flag is clear when it ends - and the user may
        // have minimised Cliche themselves, hours ago, to work in the very
        // window they are now capturing. Raising it over their work would be
        // this lot breaking the path it promised not to touch.
        assert!(
            !should_restore(MainWindow::LeftAlone),
            "a capture that hid nothing must put nothing back: the window's state is the \
             user's, not this file's"
        );
    }

    #[test]
    fn a_capture_that_hid_the_window_puts_it_back() {
        // The other row, and the one that makes the tile usable at all: without
        // it a single click would make Cliche disappear for good.
        assert!(
            should_restore(MainWindow::HiddenForCapture),
            "the tile path hid the window; whatever ends the capture owes the user a show()"
        );
    }

    #[test]
    fn the_debt_is_paid_once_and_only_once() {
        // Several paths end one capture, and two of them can arrive together -
        // a `veil_decoded` whose `show()` failed, and the fallback timer behind
        // it. The second must find nothing owed, or a window the user has since
        // minimised would be raised again from under them.
        let claim = MainWindowClaim::new();

        assert_eq!(
            claim.take(),
            MainWindow::LeftAlone,
            "a claim nobody has taken owes nothing; a fresh application must not raise \
             windows at the end of a shortcut capture"
        );

        claim.owe_a_restore();
        assert_eq!(claim.take(), MainWindow::HiddenForCapture);
        assert_eq!(
            claim.take(),
            MainWindow::LeftAlone,
            "the debt was paid on the previous line; the second path to end this capture \
             must come away with nothing"
        );
    }

    #[test]
    fn the_recompose_wait_covers_several_presentations_and_stays_a_tenth_of_a_second() {
        // The two halves of the argument in RECOMPOSE_SETTLE's own comment,
        // pinned so that lowering it to "make the click feel snappier" fails
        // here rather than in a screenshot with half a title bar in it.
        //
        // It is a BOUND on a reasoned number, not a measurement, and no test in
        // this repository can turn it into one: what a compositor takes to
        // present a frame is not observable from this process.
        assert!(
            RECOMPOSE_SETTLE >= FRAME_60HZ * 4,
            "{RECOMPOSE_SETTLE:?} is under four presentations at 60 Hz ({:?}). The frame would \
             risk catching the window half erased, which is worse than leaving it in",
            FRAME_60HZ * 4
        );
        assert!(
            RECOMPOSE_SETTLE <= Duration::from_millis(200),
            "{RECOMPOSE_SETTLE:?} is a delay the user feels on every click of the tile. Past a \
             fifth of a second the remedy costs more than the defect"
        );
    }
}
