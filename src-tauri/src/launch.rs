//! Starting a capture from the LAUNCHER, with the mouse.
//!
//! One command, and it exists because the home screen's headline tile was drawn
//! in its built state while reaching nothing at all: until 6 September 2026
//! `veil::perform_capture` had exactly one caller outside the benchmark, the
//! global-shortcut handler in `shortcut.rs`. A button that looks pressable and
//! is not is a defect, not a detail.
//!
//! # It calls `veil::perform_capture`, and NOTHING else
//!
//! Not a variant of it, not a copy of it with one line changed. The whole point
//! of `perform_capture` living in `veil.rs` rather than inside the shortcut
//! closure is that the benchmark and the shortcut measure THE SAME code; a
//! third entry point that quietly did something else would end that, and the
//! difference would show up as a latency figure nobody could attribute.
//!
//! So there is no `hide()` of the main window here, no `set_focus`, no delay,
//! no second thought. What that costs is written down rather than worked
//! around - see the section below.
//!
//! # WHAT THIS PATH DOES THAT THE SHORTCUT NEVER DOES, and what nobody has
//! # measured yet
//!
//! `perform_capture` photographs the screen BEFORE it hands anything to the
//! veil page (the ORDER comment in `veil.rs` has the reasoning), which is what
//! stops the veil from photographing itself. It says nothing about the MAIN
//! window, because until this file existed the main window was never in front
//! when a capture started.
//!
//! It is now. A click on the tile is, by definition, a click on a focused
//! Cliche window, and the frame frozen a few milliseconds later contains that
//! window - glass, title bar, the tile still lit by the pointer, and whatever
//! the user actually wanted to capture hidden behind all of it. Nothing here
//! prevents that, deliberately: hiding the window would change the very code
//! path this command exists to share, and WHICH remedy to apply - hide the
//! window, capture the desktop behind it, or accept it - is a product decision.
//!
//! NOT MEASURED, and that word is exact: this is READ off the order of the
//! statements in `perform_capture`, never observed. What to observe, so that
//! the next reader can settle it in one run rather than re-deriving it:
//!
//! 1. Put a recognisable window under Cliche, click the tile, and drag over the
//!    part of the screen Cliche was covering. EXPECTED IF THIS IS RIGHT: the
//!    frozen image shows Cliche's own window there, and the clipboard receives
//!    a picture of Cliche. Press Ctrl + Maj + 2 from that other window instead
//!    and the same drag yields the window, which is the difference.
//! 2. In the TERMINAL the two are told apart by one line: `shortcut.rs` prints
//!    `[cliche] shortcut: press N` before calling, and this path prints
//!    nothing. In the REPORT they are not told apart at all - both file into
//!    the same `Timings`, and a median over a mixed session cannot be read per
//!    source. Use ONE of the two from a fresh launch before reading figures.
//! 3. `press N` and `run N` STOP AGREEING once this path is used, because the
//!    press counter lives in the shortcut closure and the run number lives in
//!    `Veil`. Only the bench could do that before, and only when asked for.
//!    Correlate on the RUN number, which is what the veil lines carry.
//!
//! A THIRD difference, in this path's favour and worth knowing before anybody
//! "fixes" it: `veil_decoded` calls `set_focus` on the veil, and Windows only
//! lets a process raise a window when it is already the foreground process. On
//! this path Cliche IS the foreground process, because the user just clicked
//! it. The shortcut path is the one that has to fight for that, not this one.
//! Also read, not measured.
//!
//! # It is not guarded by its distance from anything
//!
//! Unlike `shortcut.rs`, which declares no command and is unreachable from a
//! webview, this module opens an `invoke` frontier onto the capture pipeline.
//! Two things stand on it and both are deliberate: `capabilities/default.json`
//! grants `allow-capture-region` to the `main` window alone, and the check
//! below refuses any other label with a line that names both windows. `ipc.rs`
//! explains why one is not enough.

use tauri::{AppHandle, Webview};

use crate::ipc;
use crate::veil;

/// Starts a region capture, exactly as Ctrl + Maj + 2 does.
///
/// Returns as soon as the pipeline has been started, not when the user has
/// drawn anything: `perform_capture` hands the frozen frame to the veil page
/// and returns, and the capture ends later through `veil_selected` or
/// `veil_dismissed`. So an `Ok(())` here means "a capture was started", never
/// "a screenshot was taken" - the page that calls this must not promise the
/// second.
///
/// A failure INSIDE the pipeline is not reported here either, for the reason
/// `perform_capture` takes no `Result` at all: every failure it meets abandons
/// the run and prints, because a failed run counted as a fast one would poison
/// the median. The only error this can return is the refusal below.
///
/// **Main window only.** The veil never starts a capture - it is what a capture
/// puts on screen, and a `capture_region` from inside it would restart the
/// pipeline while the screen is already frozen.
#[tauri::command]
pub fn capture_region(app: AppHandle, webview: Webview) -> Result<(), String> {
    ipc::ensure_from(webview.label(), ipc::MAIN_WINDOW_LABEL, "capture_region")?;

    // On a webview IPC thread here, on the plugin's hotkey thread from the
    // shortcut. Both are worker threads and neither may panic; `perform_capture`
    // is written for exactly that constraint and needs nothing added here.
    veil::perform_capture(&app);

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
}
