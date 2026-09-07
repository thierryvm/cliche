//! Making the desktop compositor finish what it was asked, before a capture.
//!
//! # THE DEFECT, seen twice by the user
//!
//! A capture started from the launcher TILE came back with Cliche itself in the
//! frame, half transparent, over the desktop. `launch::capture_region` hides the
//! main window first and waits `RECOMPOSE_SETTLE` - 120 ms - before the screen
//! is photographed, and that constant's own comment says in as many words that
//! 120 ms is a REASONED number and not a measured one. A window caught half
//! erased is that bet being lost.
//!
//! Two things are done about it here, and NEITHER replaces the wait:
//!
//! 1. **The window stops fading at all.** Windows animates a window on its way
//!    out, and DWM can be told not to, per window, once. A window that leaves
//!    the screen in one step has no half-erased state to be caught in.
//! 2. **The wait stops being blind.** Before sleeping, the capture thread asks
//!    the compositor to block until frames have really been presented. Whatever
//!    that costs is then DEDUCTED from the 120 ms rather than added to it - see
//!    [`settle_after_hide`] - so a click on the tile costs exactly what it cost
//!    before, and the picture can only get better.
//!
//! # ONE INTERFACE, ONE ENGINE PER SYSTEM - and that is not optional here
//!
//! `docs/STACK.md`, under "Portabilite", records the constraint Thierry set and
//! dates it to 3 September 2026: Cliche must one day run on Windows, macOS and
//! Linux, and nothing may close that door. Until today this crate's `src/` held
//! NO platform-specific code at all - only `build.rs` had a `#[cfg(windows)]`,
//! for the manifest. This module is the first, so it is shaped the way that rule
//! requires:
//!
//! * the two gestures below say nothing about any operating system, and their
//!   callers - `lib.rs` and `launch.rs` - never learn which engine ran;
//! * `engine` is `#[cfg(windows)]` and speaks DWM;
//! * the `#[cfg(not(windows))]` engine does NOTHING NATIVE and says so, rather
//!   than quietly looking like an implementation. macOS and Linux each have
//!   mechanisms of their own for both questions, and NEITHER has been looked at;
//! * the dependency sits under `[target.'cfg(windows)'.dependencies]` in
//!   `Cargo.toml`, so on macOS it is not so much as resolved.
//!
//! On a system with no engine the floor is the whole of the protection - which
//! is exactly what the tile path had on Windows until 7 September 2026. The
//! fallback is therefore a degradation, never a breakage.
//!
//! # WHAT IS NOT MEASURED, said here before anybody leans on it
//!
//! **Nobody has observed that a presentation guarantees our `hide()` has been
//! composed.** That is the whole reason the floor stays, and it is not timidity.
//! Microsoft's own reference for `DwmFlush` says it "waits for any queued
//! DirectX changes that were queued by the calling application to be drawn to
//! the screen before returning. It does not flush the entire session rendering
//! batch" (learn.microsoft.com, dwmapi.h, read on 7 September 2026). Our
//! `hide()` is not a DirectX change this process queued; it is a window-manager
//! operation the compositor picks up on its own. So a flush is EVIDENCE that the
//! compositor has presented, and it is NOT proof that what it presented was a
//! desktop with Cliche gone from it.
//!
//! Nothing else here is measured either, and the list is short on purpose:
//!
//! * that turning transitions off removes the fade this user saw has been READ
//!   from the attribute's documentation, never observed on this machine;
//! * how long two flushes take is unknown. It may be nothing. It may be two
//!   frames. That is why `launch::capture_region` prints BOTH figures - the
//!   presentations and the sleep left after them - on every tile capture: it is
//!   the only way anybody will ever learn which of the two mechanisms is doing
//!   the work.

use std::time::{Duration, Instant};

use tauri::{Runtime, WebviewWindow};

/// What taking one window's transitions away came to.
///
/// Pure data, and its terminal line is under test, for the reason
/// `shortcut::ShortcutStatus` is built the same way: this line is the ONLY thing
/// a user can report back about a mechanism no test in this repository can
/// reach, so its wording is a behaviour rather than a detail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Silencing {
    /// DWM accepted. The window now leaves the screen in one step.
    ///
    /// Only an engine constructs this, so on a build with none it is dead - and
    /// `clippy -D warnings` is right to say so. The exemption is written per
    /// variant and per platform rather than over the enum: if `Applied` ever
    /// stopped being constructed ON WINDOWS, that is a real defect and the
    /// warning must still arrive.
    #[cfg_attr(not(windows), allow(dead_code))]
    Applied,
    /// Refused - by the compositor, or by Tauri when the window handle could
    /// not be had. The application starts anyway; only the fade comes back.
    #[cfg_attr(not(windows), allow(dead_code))]
    Refused(String),
    /// This build has no engine for the system it is running on.
    ///
    /// The mirror image of the two above: on Windows nothing constructs it, and
    /// the day something does the engine has gone missing.
    #[cfg_attr(windows, allow(dead_code))]
    NoEngineHere,
    /// There was no window to silence. Constructed by the CALLER and not by an
    /// engine, deliberately: which window to look for is the application's
    /// business, and `lib.rs` is the only place that can find none.
    WindowMissing,
}

impl Silencing {
    /// The one line this leaves in the terminal, whatever happened.
    ///
    /// Printed on every start, success included - unlike `lifecycle`'s
    /// `bring_to_front`, whose silence is its report. The difference is that
    /// nothing else can ever speak for this mechanism: a user whose captures
    /// still show a ghost of Cliche has no other way to tell us whether the
    /// attribute was set, refused, or never attempted.
    ///
    /// It names the WINDOW as well as the outcome, because this application has
    /// two of them and only one is ever silenced.
    pub fn terminal_line(&self, label: &str) -> String {
        match self {
            Silencing::Applied => format!(
                "[cliche] compositor: system transitions are OFF for the `{label}` window, so \
                 hiding it before a capture takes effect in one step instead of a fade"
            ),
            Silencing::Refused(reason) => format!(
                "[cliche] compositor: system transitions could NOT be turned off for the \
                 `{label}` window ({reason}); it still fades out, and a capture started from the \
                 tile may catch it half erased"
            ),
            Silencing::NoEngineHere => format!(
                "[cliche] compositor: no engine on this system, so the `{label}` window keeps its \
                 system transitions; the fixed wait before a capture is all the protection there is"
            ),
            Silencing::WindowMissing => format!(
                "[cliche] compositor: there is no `{label}` window to take the system transitions \
                 away from, so nothing was attempted; it will fade out when a capture hides it"
            ),
        }
    }
}

/// Takes a window's system transitions away, once and for good.
///
/// Called at startup from `lib.rs`, for the reason the veil is built there: it
/// is a one-off cost, and paying it inside a capture would put it in front of
/// the user.
///
/// # A failure here changes nothing but the fade
///
/// The outcome is handed back rather than acted on, and no path in this
/// application treats it as fatal. A DWM that refuses leaves Cliche exactly
/// where it was this morning - a window that fades out, and a tile capture that
/// may catch it half erased - and an application that would not START because a
/// compositor said no would be a far larger defect than the one being fixed.
pub fn silence_transitions<R: Runtime>(window: &WebviewWindow<R>) -> Silencing {
    engine::silence_transitions(window)
}

/// What one settle after a hide really cost, in its two halves.
///
/// Both are handed back because both are printed: on the evidence available
/// today nobody can say which of the two mechanisms is doing the work, and one
/// total would hide exactly the number that settles it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settled {
    /// Time spent inside the compositor, waiting for real presentations.
    pub presented: Duration,
    /// Time spent asleep afterwards, to reach the floor.
    pub slept: Duration,
}

impl Settled {
    /// What a capture that hid nothing waited for: nothing at all.
    pub const NOTHING: Self = Self {
        presented: Duration::ZERO,
        slept: Duration::ZERO,
    };
}

/// Waits for the desktop to be redrawn without the window that was just hidden.
///
/// # It is a DEADLINE, not a sleep, and that is the whole design
///
/// The presentations are waited for FIRST, and what they cost is then taken OFF
/// the floor. A settle therefore ends `floor` after it started, whether the
/// compositor answered in one microsecond or in forty milliseconds - so the
/// evidence is bought for nothing and the click costs what it always cost.
///
/// The floor is passed in rather than known here: it belongs to the path that
/// pays it, `launch::RECOMPOSE_SETTLE`, next to the reasoning that produced it.
///
/// **The floor is the protection; the presentations are only evidence.** The
/// module header says why - no presentation observed from this process proves
/// anything about our `hide()` having been composed. Nothing here may be read as
/// licence to lower that number.
pub fn settle_after_hide(floor: Duration) -> Settled {
    let started = Instant::now();

    engine::wait_for_presentations();

    let presented = started.elapsed();
    let slept = remaining_wait(presented, floor);

    // Guarded rather than called with zero: `thread::sleep(ZERO)` still enters
    // the operating system and gives up the timeslice, and this is precisely the
    // branch where the presentations have already cost more than the floor - the
    // one case in which every remaining microsecond is one the user waits out.
    if !slept.is_zero() {
        std::thread::sleep(slept);
    }

    Settled { presented, slept }
}

/// How long is left to sleep, once `already_spent` has gone by.
///
/// THE decision of this module, and the only part of it a test can reach: the
/// Win32 calls need a compositor and a live window, and no `cargo test` in this
/// repository has either.
///
/// `saturating_sub` and not `-`: a `Duration` subtraction that would go negative
/// PANICS, and this runs on the capture thread, where a panic takes the whole
/// application down with it. Presentations slower than the floor are not a
/// far-fetched case either - they are what a busy machine looks like, and they
/// are the exact input that would produce one.
fn remaining_wait(already_spent: Duration, floor: Duration) -> Duration {
    floor.saturating_sub(already_spent)
}

// === the Windows engine ======================================================

#[cfg(windows)]
mod engine {
    use std::mem::size_of_val;
    use std::sync::atomic::{AtomicBool, Ordering};

    use tauri::{Runtime, WebviewWindow};
    use windows::Win32::Graphics::Dwm::{
        DwmFlush, DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
    };

    use super::Silencing;

    /// How many presentations are waited for.
    ///
    /// TWO, and the reason is the FIRST one: `DwmFlush` returns at the next
    /// present, which may be a frame the compositor had already begun before our
    /// `hide()` ever reached it. The second is therefore the first that CAN have
    /// been composed afterwards.
    ///
    /// NOT MEASURED, and the module header is blunt about why that argument is
    /// weaker than it looks. Two is the cheapest number that is not one, and it
    /// is free: [`super::settle_after_hide`] deducts whatever it costs from the
    /// floor instead of adding to it.
    const PRESENTATIONS: usize = 2;

    /// Whether a refused flush has already been reported.
    ///
    /// Once per process, never once per capture: a machine whose DWM is absent
    /// or refusing would otherwise print a line on every click, in the very
    /// terminal where the capture figures are read.
    static FLUSH_REFUSAL_REPORTED: AtomicBool = AtomicBool::new(false);

    /// Asks DWM to stop animating this window.
    pub(super) fn silence_transitions<R: Runtime>(window: &WebviewWindow<R>) -> Silencing {
        let handle = match window.hwnd() {
            Ok(handle) => handle,
            Err(error) => return Silencing::Refused(error.to_string()),
        };

        // DWM reads this attribute as a `BOOL`, TRUE meaning "disable
        // transitions" - the DWMWINDOWATTRIBUTE reference on learn.microsoft.com,
        // read on 7 September 2026. A `BOOL` is a 32-bit integer at the ABI,
        // which is what this local is, and the SIZE below is taken from the value
        // itself rather than written as `4` so that the two cannot disagree.
        let disable_transitions: i32 = 1;

        // SAFETY: `handle` was just returned by Tauri for a window this process
        // owns, and `window` stays borrowed for the whole call, so the window
        // cannot be destroyed underneath it. `pvAttribute` and `cbAttribute` are
        // both `[in]` parameters and the call is synchronous (the
        // DwmSetWindowAttribute reference, read on 7 September 2026), so DWM
        // reads the four bytes of a local that outlives the statement and keeps
        // nothing that could be read after it is gone.
        let asked = unsafe {
            DwmSetWindowAttribute(
                handle,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                (&disable_transitions as *const i32).cast(),
                size_of_val(&disable_transitions) as u32,
            )
        };

        match asked {
            Ok(()) => Silencing::Applied,
            Err(error) => Silencing::Refused(error.to_string()),
        }
    }

    /// Blocks until the compositor has presented, twice.
    pub(super) fn wait_for_presentations() {
        for _ in 0..PRESENTATIONS {
            // SAFETY: `DwmFlush` takes no argument and is handed no pointer of
            // ours; the only thing it can do to this process is block the
            // calling thread. That thread is the one `launch::capture_region`
            // spawned for the capture, never the event loop - blocking the event
            // loop here would stall the very `hide()` being waited for.
            if let Err(error) = unsafe { DwmFlush() } {
                report_refusal(&error);
                // Handed back AT ONCE, with no second attempt: a DWM that
                // refuses once will refuse again, and a capture that waited
                // through two refusals would pay for them on every click. The
                // floor in `super::settle_after_hide` still runs, which is what
                // this path had before there was an engine at all.
                return;
            }
        }
    }

    /// Says once, and only once, that the compositor could not be waited for.
    ///
    /// Takes anything printable rather than the crate's error type: nothing here
    /// needs to know what a `windows` error IS, and not naming the type keeps one
    /// more foreign name out of a file that already carries three.
    fn report_refusal(error: &impl std::fmt::Display) {
        if FLUSH_REFUSAL_REPORTED.swap(true, Ordering::Relaxed) {
            return;
        }

        eprintln!(
            "[cliche] compositor: DwmFlush refused ({error}); captures started from the tile fall \
             back to the fixed wait alone. Said once per run, not once per capture"
        );
    }
}

// === every other system, for now =============================================

#[cfg(not(windows))]
mod engine {
    use tauri::{Runtime, WebviewWindow};

    use super::Silencing;

    /// NOTHING NATIVE HAPPENS HERE, and that is an honest answer rather than a
    /// placeholder somebody will fill in without noticing.
    ///
    /// macOS and Linux both have mechanisms of their own for the two questions
    /// this module asks - a window that leaves the screen without an animation,
    /// and a wait that ends on a real presentation - and NEITHER has been looked
    /// at. On macOS, ordering a window out and CoreAnimation's transactions are
    /// where the reading would start; on Linux it depends on the session's
    /// compositor and there is no single answer. Writing either belongs in THIS
    /// file, under another `cfg`, with the callers untouched.
    ///
    /// Until then the caller is told so on the terminal, and the floor in
    /// [`super::settle_after_hide`] is the whole of the protection - exactly
    /// what the tile path had on Windows until 7 September 2026.
    pub(super) fn silence_transitions<R: Runtime>(_window: &WebviewWindow<R>) -> Silencing {
        Silencing::NoEngineHere
    }

    /// See [`silence_transitions`]: nothing native, deliberately.
    ///
    /// It returns instantly, so [`super::settle_after_hide`] sleeps the whole
    /// floor - the behaviour every platform had before this module existed.
    pub(super) fn wait_for_presentations() {}
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The same number `launch::RECOMPOSE_SETTLE` holds, restated here rather
    /// than imported.
    ///
    /// [`remaining_wait`] takes its floor as an argument precisely so that it
    /// knows nothing about the tile path, and widening that constant's
    /// visibility to reach it would undo the separation. What holds the shipped
    /// value is `launch.rs`'s own
    /// `the_recompose_wait_covers_several_presentations_and_stays_a_tenth_of_a_second`.
    const FLOOR: Duration = Duration::from_millis(120);

    #[test]
    fn presentations_that_cost_nothing_leave_the_whole_floor_to_sleep() {
        // The row a compositor that answers instantly produces - and the one
        // that has to keep the old behaviour exactly. Before 7 September 2026
        // this path slept 120 ms flat; on a machine where the flushes are free,
        // it still must.
        assert_eq!(
            remaining_wait(Duration::ZERO, FLOOR),
            FLOOR,
            "presentations that cost nothing must leave the entire floor to sleep, or a fast \
             machine would photograph the desktop sooner than it did yesterday"
        );
    }

    #[test]
    fn presentations_that_cost_half_the_floor_leave_the_other_half() {
        // THE row this whole change is for. The evidence has to be FREE: added
        // to the floor rather than taken off it, a click on the tile would cost
        // 180 ms instead of 120 - a delay the user feels, bought for a guarantee
        // the module header says nobody has.
        assert_eq!(
            remaining_wait(FLOOR / 2, FLOOR),
            FLOOR / 2,
            "the time the compositor already took must come OFF the floor, never on top of it"
        );
    }

    #[test]
    fn presentations_that_outlast_the_floor_are_not_slept_on_top_of() {
        // A busy machine, and the input that would panic a plain subtraction:
        // `Duration` has no negatives. This runs on the capture thread, where a
        // panic takes the application down - losing Cliche to a screenshot.
        assert_eq!(
            remaining_wait(FLOOR * 2, FLOOR),
            Duration::ZERO,
            "a floor already passed leaves nothing to sleep; it must not wrap, panic, or start \
             the wait again"
        );
    }

    #[test]
    fn the_presentations_and_the_sleep_together_come_to_the_floor_and_never_more() {
        // The invariant, over the whole range rather than at three points: the
        // settle ends `FLOOR` after it began, whatever the compositor took. It
        // is what makes "the picture can only get better, the click costs the
        // same" true rather than hoped for.
        for spent_ms in [0, 1, 8, 16, 40, 60, 119, 120] {
            let spent = Duration::from_millis(spent_ms);
            let slept = remaining_wait(spent, FLOOR);

            assert_eq!(
                spent + slept,
                FLOOR,
                "{spent:?} spent presenting then {slept:?} asleep does not land on {FLOOR:?}"
            );
            assert!(
                slept <= FLOOR,
                "{slept:?} of sleep is more than the whole floor after {spent:?} had already gone"
            );
        }

        // Past the floor there is nothing left to give back, and the total is
        // the presentations alone - larger than the floor because the time has
        // already been spent, never because anything was added to it.
        for spent_ms in [121, 200, 5_000] {
            let spent = Duration::from_millis(spent_ms);

            assert_eq!(
                remaining_wait(spent, FLOOR),
                Duration::ZERO,
                "{spent:?} is already past {FLOOR:?}; sleeping any longer would be this fix \
                 making the tile slower than the defect it replaces"
            );
        }
    }

    #[test]
    fn every_outcome_says_what_happened_and_which_window_it_happened_to() {
        // This line is the only evidence about a mechanism no test can reach.
        // An outcome that did not name the window would be useless in an
        // application with two of them, and two outcomes sharing a line would be
        // worse than no line at all - it would answer the question wrongly.
        let outcomes = [
            Silencing::Applied,
            Silencing::Refused("DWM_E_COMPOSITIONDISABLED".to_owned()),
            Silencing::NoEngineHere,
            Silencing::WindowMissing,
        ];

        let mut lines = Vec::new();

        for outcome in &outcomes {
            let line = outcome.terminal_line("main");

            assert!(
                line.starts_with("[cliche] compositor: "),
                "every line of this module must be attributable in a terminal shared with \
                 launch, veil and lifecycle: {line}"
            );
            assert!(
                line.contains("`main`"),
                "{outcome:?} does not name the window it is about: {line}"
            );

            lines.push(line);
        }

        for (index, line) in lines.iter().enumerate() {
            assert!(
                !lines[index + 1..].contains(line),
                "two different outcomes print the same line, so the terminal cannot tell them \
                 apart: {line}"
            );
        }
    }

    #[test]
    fn a_refusal_carries_the_reason_and_reads_as_a_refusal() {
        // A refusal nobody can attribute is a refusal nobody can act on - the
        // rule `lifecycle::raise_failure` obeys. And it must not read like a
        // success: this is the line that tells us the fade is still there.
        let line = Silencing::Refused("DWM_E_COMPOSITIONDISABLED".to_owned()).terminal_line("main");

        assert!(
            line.contains("DWM_E_COMPOSITIONDISABLED"),
            "what the compositor answered must survive into the line: {line}"
        );
        assert!(
            line.contains("NOT"),
            "the line must say the attribute was not set, and say it in a way that survives a \
             skim: {line}"
        );
    }

    #[test]
    fn the_engine_returns_and_a_floor_of_nothing_is_slept_for_nothing() {
        // The one thing about the native path this suite CAN observe: that the
        // engine links, returns, and does not hang. It is not proof that the
        // flush did anything - the module header says why nothing here could be.
        //
        // On Windows this really calls `DwmFlush` twice. In a session with no
        // compositor it returns an error, which is reported once and handed
        // back; on every other platform the fallback engine returns instantly.
        // Both are fast, and neither is allowed to block for ever.
        let settled = settle_after_hide(Duration::ZERO);

        assert_eq!(
            settled.slept,
            Duration::ZERO,
            "a floor of nothing leaves nothing to sleep, whatever the presentations cost"
        );
    }

    #[test]
    fn both_gestures_are_wired_into_the_application() {
        // The rules above are worth exactly as much as their wiring, and the
        // same net `lifecycle`'s `the_builder_asks_this_module_rather_than_
        // deciding_for_itself` casts: a source-reading test, with the same
        // honest limit - it proves the calls are WRITTEN, never that they ran.
        // Nothing in this suite can run them; that needs an event loop and a
        // compositor.
        let lib = include_str!("lib.rs");
        assert!(
            lib.contains("compositor::silence_transitions("),
            "lib.rs never takes the main window's transitions away, so the window still fades \
             out and every other test in this file is green over the defect it exists to fix"
        );

        let launch = include_str!("launch.rs");
        assert!(
            launch.contains("compositor::settle_after_hide("),
            "launch.rs does not go through this module, so the deadline above is wired to \
             nothing and the tile path is back to a blind sleep"
        );
        assert!(
            !launch.contains("sleep(RECOMPOSE_SETTLE)"),
            "launch.rs still sleeps the floor directly. The wait is a DEADLINE now: sleeping it \
             flat AND flushing would make a tile capture cost the sum of the two"
        );
    }
}
