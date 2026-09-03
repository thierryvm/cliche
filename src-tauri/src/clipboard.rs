//! The cut region, into the system clipboard, from Rust.
//!
//! This module is the end of the chain 1e stopped one step short of: 1e proved
//! that a rectangle drawn in CSS pixels becomes an exact rectangle of the
//! capture's own bytes, then dropped it. Here those bytes leave the process.
//!
//! # No encoding. None.
//!
//! The clipboard takes `tauri::image::Image`, which is RGBA plus its two
//! dimensions - exactly what `capture::Frame` already holds. Read in the
//! `tauri` 2.11.5 source: `Image::new` is a `const fn` that wraps a borrowed
//! slice, and the plugin's `write_image` hands `image.rgba()` straight to
//! `arboard::ImageData`. So the whole conversion is a borrow.
//!
//! Encoding to PNG here would cost 69.6 ms (measured 3 September 2026, see
//! `capture::encode_bmp`) to produce a format nothing on this path asks for.
//!
//! # This step is OUTSIDE the 150 ms budget - read this before comparing figures
//!
//! The budget of lots 1b-1e runs from the shortcut handler being entered to the
//! veil being painted, and it is closed by `veil_painted`, which calls
//! `Timings::finish_run`. The clipboard write happens LATER, after the user has
//! finished dragging - seconds later, at human speed. It cannot be part of that
//! total, and it must not be counted in it.
//!
//! That is why this module carries its OWN instrument ([`Meter`]) instead of
//! marking a step on the pipeline's `Timings`. The reason is mechanical, not
//! stylistic:
//!
//! - by the time `veil_selected` runs, the pipeline's run is already closed, so
//!   `Timings::mark` would drop the mark and count it as ignored - a silent
//!   nothing, plus a counter that says "a step was measured under the wrong
//!   name";
//! - opening a fresh run on the pipeline's instrument would be worse: `aggregate`
//!   pushes every run's last offset into the TOTAL series, so each copy would
//!   drop a human-scale number into `total_median` - the one figure the 150 ms
//!   verdict is read off.
//!
//! Two instruments, two reports, and [`Meter::report_lines`] says on its first
//! line that this one is not comparable to 1d's.
//!
//! # Nothing here may panic
//!
//! Same rule as `veil.rs`, same reason: this runs on a webview IPC thread and a
//! panic there takes the application down. In particular the plugin's own
//! `ClipboardExt::clipboard()` helper is NOT used - it is `state::<Clipboard<R>>()`,
//! which panics when the plugin was never registered. [`copy_selection`] uses
//! `try_state` and returns an error the page can catch.
//!
//! One panic remains, and it is not ours to remove: the plugin's `write_image`
//! does `.lock().unwrap().as_mut().unwrap()` internally, so it panics on a
//! poisoned lock, or if called after `RunEvent::Exit` has taken the clipboard
//! away. Stated rather than hidden; both cases need the application to be on its
//! way out already.
//!
//! # Threads, and the Linux caveat
//!
//! The plugin's source warns that `read_text` and `read_image` must not run on
//! the main thread - the underlying library can deadlock on Linux. `write_image`
//! carries no such warning, and nothing in this module is scheduled onto the
//! main thread: `veil_selected` is a synchronous command, so it runs on a
//! webview IPC thread, and the ignored round-trip test below runs on a test
//! thread. Cliche ships Windows only, but the portability constraint of
//! 3 September 2026 says no door may be closed, and this one is not.
//!
//! **Not verified here:** that Tauri never promotes a synchronous command onto
//! the main thread on some platform. It is read from `veil.rs`'s own header and
//! from the absence of a `run_on_main_thread` anywhere on this path, not
//! measured at runtime.
//!
//! # What is NOT proven here - the round trip is MISSING, and why
//!
//! Every test below stops at this module's own boundary. **Nothing in the suite
//! proves the bytes really reach the Windows clipboard.**
//!
//! That test was written, and it does not compile in this project. It needs a
//! `Clipboard<R>`, which only the plugin hands out through managed state, so it
//! needs an `App` - and building a real one needs the main thread and an event
//! loop, which a test binary has neither of. The way round is
//! `tauri::test::mock_builder`, gated behind tauri's `test` feature.
//!
//! Measured on 4 September 2026, not assumed: adding
//! `tauri = { version = "2.11.5", features = ["test"] }` to `[dev-dependencies]`
//! compiles cleanly and then the test binary REFUSES TO START -
//! `STATUS_ENTRYPOINT_NOT_FOUND` (0xc0000139), before a single test runs. The
//! whole suite dies with it. Isolated by reverting that one line: the same tree
//! then ran 97 tests. The cause is that feature and nothing else in this lot -
//! in particular it is NOT the clipboard plugin's own linkage, which is present
//! in the passing build.
//!
//! Diagnosing an entry point missing from a DLL needs a shell, which the agent
//! that wrote this did not have. So the round trip is left UNCOVERED and said so
//! here, rather than replaced by a test that would pass without touching a
//! clipboard. Until somebody runs the application and pastes, "it reaches the
//! clipboard" is an untested claim.
//!
//! # Capabilities: nothing was added, and here is why - both halves
//!
//! **The half that is true, and was the right call.**
//! `capabilities/default.json` still grants `core:default` only. The plugin's
//! ACL permissions (`permissions/autogenerated/commands/write_image.toml`, read
//! on 4 September 2026) are generated per COMMAND: they gate
//! `invoke("plugin:clipboard-manager|write_image")` coming from the webview.
//! That gate really does hold - a `plugin:` command takes the
//! `plugin_command.is_some()` branch of `webview/mod.rs:1823` and is refused
//! without a capability. So no page of this application can ask the plugin to
//! write the clipboard, and adding a capability would have handed that ability
//! to `main` for no reason. This module never crosses that frontier anyway: it
//! calls the plain Rust method on the plugin's managed state.
//!
//! **The half that was missing, and read as the opposite of the truth.**
//! None of that says anything about THIS crate's own `#[tauri::command]`s -
//! `veil_selected` in particular, which is what calls
//! [`copy_selection`]. Those carry no `plugin:` prefix, and this application
//! declares no ACL manifest (verified 4 September 2026: `src-tauri/permissions/`
//! does not exist and `gen/schemas/acl-manifests.json` has no application key).
//! From a local origin the ACL check therefore never runs on them. Until
//! 4 September 2026 nothing guarded them; the guard is now `ipc.rs`, checking
//! the calling webview's label in Rust, and it is what stops `main` from
//! reaching the clipboard THROUGH `veil_selected` rather than through the
//! plugin.
//!
//! Read from the plugin's source, its permission files and the vendored
//! `tauri` 2.11.5 source; NOT confirmed by a run of the application.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_clipboard_manager::Clipboard;

use crate::capture::Frame;
use crate::timing::Timings;

/// Timing label for the clipboard write.
///
/// Deliberately NOT one of the four pipeline labels of `veil.rs`: it belongs to
/// another instrument and another report. A test below holds that apart.
pub const MARK_CLIPBOARD: &str = "clipboard";

/// Bytes per pixel in RGBA8, as `Image` and `arboard` both read it.
const BYTES_PER_PIXEL: u64 = 4;

/// The smallest selection worth putting on the clipboard, in PHYSICAL pixels of
/// area: **64**, an 8 x 8 square.
///
/// # Why a threshold exists at all
///
/// `docs/PRD.md`, case 6, requires that getting the selection wrong leaves
/// "ni fichier orphelin, ni presse-papier ecrase par une image vide, ni ligne
/// parasite dans l'historique". A click that the user did not mean as a drag
/// must therefore NOT touch the clipboard. The cost of the two mistakes is
/// wildly asymmetric, and that asymmetry is what sets the rule:
///
/// - refusing a real selection: a message, and the user drags again;
/// - accepting an accidental click: whatever they had copied is gone, silently,
///   and nothing can bring it back.
///
/// So the threshold errs high.
///
/// # Why 64, and not a number that felt right
///
/// It sits at the geometric middle of the two quantities that bracket it, four
/// times away from each:
///
/// - **16 px is the biggest plausible accident.** A click is a press and a
///   release at what the user believes is one point; the pointer drifts by a few
///   pixels of hand jitter in between. Taking a generous 4 px per axis gives a
///   4 x 4 = 16 px rectangle. (Windows has its own constant for this exact
///   question, `SM_CXDRAG`/`SM_CYDRAG`, and 4 is the conventional value - but it
///   has NOT been read from this machine, and it is deliberately not read at
///   runtime: a threshold that moved with a system setting would make the same
///   gesture behave differently on two machines.)
/// - **256 px is the smallest deliberate capture.** The smallest thing anyone
///   sets out to screenshot is a glyph or an icon, and an icon is 16 x 16.
///
/// `sqrt(16 * 256) = 64`. Any threshold in that gap works; 64 is the one that is
/// as far from both edges as the pair allows, so neither a jittery click nor a
/// tiny icon lands near it.
///
/// # Why AREA, and not a minimum on each side
///
/// A minimum per side would refuse a deliberate thin strip - 200 x 1 physical px
/// is a gesture somebody makes on purpose, and its area of 200 says so. An
/// accidental click is small in BOTH axes at once, so its area collapses far
/// faster than either side does. Area is the measure that separates the two.
pub const MIN_COPYABLE_AREA_PX: u64 = 64;

/// Whether a selection of this size is big enough to be worth the clipboard.
///
/// The comparison is `>=`: [`MIN_COPYABLE_AREA_PX`] itself is accepted. Computed
/// in `u64` because `u32 * u32` overflows a `u32`, and an overflow here would
/// turn a huge selection into a tiny one and refuse it.
pub fn is_worth_copying(width: u32, height: u32) -> bool {
    u64::from(width) * u64::from(height) >= MIN_COPYABLE_AREA_PX
}

/// The refusal, worded once so it is under test.
///
/// It names the size, the area and the threshold: a user who meant to drag has
/// to be able to tell "I clicked" from "the tool is broken".
pub fn too_small_line(width: u32, height: u32) -> String {
    format!(
        "a selection of {width}x{height} physical px covers {area} px, below the {MIN_COPYABLE_AREA_PX} \
         px this tool treats as a deliberate drag; the clipboard was NOT touched",
        area = u64::from(width) * u64::from(height),
    )
}

/// Wraps RGBA bytes in the image type the clipboard takes, checking what
/// `Image::new` does not.
///
/// `Image::new` is a `const fn` that stores the slice and the two dimensions on
/// trust - read in the `tauri` 2.11.5 source. The plugin then hands the slice to
/// `arboard`, which reads `width * height * 4` bytes out of it. `Frame` already
/// makes that impossible upstream; this check is the seam with a foreign API,
/// where one comparison is cheaper than finding out what a Win32 DIB does with a
/// buffer that is too short.
///
/// A zero dimension is refused explicitly: it passes the length check trivially
/// (0 == 0), which is exactly why it needs its own branch.
pub fn to_image(width: u32, height: u32, rgba: &[u8]) -> Result<Image<'_>, String> {
    if width == 0 || height == 0 {
        return Err(format!(
            "an image of {width}x{height} has no pixels; it cannot go on the clipboard"
        ));
    }

    let expected = u64::from(width) * u64::from(height) * BYTES_PER_PIXEL;
    let actual = rgba.len() as u64;

    if actual != expected {
        return Err(format!(
            "{actual} byte(s) cannot describe a {width}x{height} RGBA image, which needs {expected}"
        ));
    }

    Ok(Image::new(rgba, width, height))
}

/// What a successful copy is worth reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Copied {
    /// How long the write itself took. See the module header: OUTSIDE the
    /// 150 ms budget.
    pub elapsed: Duration,
    /// How many copies this process has filed, `0` when no [`Meter`] is managed.
    /// The caller uses it to decide when a batch report is due.
    pub copies: usize,
}

/// The clipboard step's own instrument. Managed by Tauri, borrowed as
/// `State<Meter>`.
///
/// A distinct type from `Timings` on purpose: Tauri manages state BY TYPE, and
/// these figures must never be aggregated with the pipeline's. See the module
/// header for what folding them together would do to `total_median`.
#[derive(Debug, Default)]
pub struct Meter {
    timings: Timings,
    copies: AtomicUsize,
}

impl Meter {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the step. Called as late as possible - everything before it is
    /// validation, and measuring validation would blur the figure.
    fn begin(&self) {
        self.timings.begin_run();
    }

    /// Files a successful write and returns how many have been filed.
    fn filed(&self) -> usize {
        self.timings.mark(MARK_CLIPBOARD);
        self.timings.finish_run();
        self.copies
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    /// Throws a failed write away. A write that did not happen has no latency,
    /// and filing it would let a failure count as a fast success - the same rule
    /// the capture pipeline follows.
    fn abandoned(&self) {
        self.timings.abandon_run();
    }

    /// The report, headed by the one line that stops it being read as 1d's.
    pub fn report_lines(&self) -> Vec<String> {
        let mut lines = vec![format!(
            "clipboard: {copies} copy/copies - this step is OUTSIDE the 150 ms budget, which ends \
             at `painted`. It happens after the user's drag, at human speed; do NOT add it to the \
             pipeline total.",
            copies = self.copies.load(Ordering::Relaxed),
        )];
        lines.extend(self.timings.report().lines());
        lines
    }
}

/// The line printed when a copy succeeds. Pure, so its wording is under test -
/// it is the line the manual procedure tells a human to look for.
pub fn success_line(run: u64, width: u32, height: u32, bytes: usize, elapsed: Duration) -> String {
    format!(
        "[cliche] clipboard: run {run} copied {width}x{height} physical px ({bytes} RGBA byte(s)) \
         in {millis:.1} ms - OUTSIDE the 150 ms budget, which ends at `painted`",
        millis = elapsed.as_secs_f64() * 1000.0,
    )
}

/// Puts a cut region on the system clipboard.
///
/// Order of the three refusals, and it matters: the size is judged FIRST, so a
/// click costs neither a lookup nor an allocation, and above all so that a
/// selection this tool refuses never reaches the clipboard at all - the PRD's
/// case 6 is about what the clipboard still holds afterwards.
///
/// Returns `Result` so a refusal reaches the page's `catch` instead of vanishing.
pub fn copy_selection<R: Runtime>(app: &AppHandle<R>, cut: &Frame) -> Result<Copied, String> {
    if !is_worth_copying(cut.width(), cut.height()) {
        return Err(too_small_line(cut.width(), cut.height()));
    }

    let image = to_image(cut.width(), cut.height(), cut.pixels())?;

    // `try_state`, never the plugin's `clipboard()` helper: that one is
    // `state::<Clipboard<R>>()`, which PANICS when the plugin was not
    // registered, on a webview IPC thread.
    let clipboard = app.try_state::<Clipboard<R>>().ok_or_else(|| {
        "the clipboard plugin is not registered; nothing can be copied".to_owned()
    })?;

    let meter = app.try_state::<Meter>();
    if let Some(meter) = meter.as_ref() {
        meter.begin();
    }

    // Measured here as well as by the meter: the meter aggregates, this is the
    // number the success line shows for THIS copy. Two clock reads, tens of
    // nanoseconds, against a write measured in milliseconds.
    let started = Instant::now();
    let written = clipboard.write_image(&image);
    let elapsed = started.elapsed();

    if let Err(error) = written {
        if let Some(meter) = meter.as_ref() {
            meter.abandoned();
        }
        return Err(format!("the clipboard refused the image: {error}"));
    }

    let copies = meter.as_ref().map_or(0, |meter| meter.filed());

    Ok(Copied { elapsed, copies })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 8 x 8 - exactly [`MIN_COPYABLE_AREA_PX`], so the smallest thing this
    /// module accepts is also what the real round trip is run on. Every pixel
    /// differs from every other, so a transposition or a channel swap fails.
    /// Alpha is 255 everywhere: see the round-trip test for why.
    fn known_cut() -> Frame {
        let mut pixels = Vec::with_capacity(8 * 8 * 4);
        for row in 0..8u8 {
            for column in 0..8u8 {
                pixels.push(10 * (column + 1));
                pixels.push(100 + row);
                pixels.push(row * 8 + column);
                pixels.push(255);
            }
        }

        Frame::new(8, 8, pixels).expect("the hand-written cut must match its dimensions")
    }

    // ---------------------------------------------------------------------
    // The area rule. No screen, no clipboard: these run in CI.
    // ---------------------------------------------------------------------

    #[test]
    fn the_threshold_is_the_one_the_doc_comment_argues_for() {
        // The number is load-bearing - the reasoning in `MIN_COPYABLE_AREA_PX`
        // is about 16 and 256 specifically. Changing it silently would leave
        // that argument describing a rule that no longer exists.
        assert_eq!(MIN_COPYABLE_AREA_PX, 64);
    }

    #[test]
    fn a_selection_of_exactly_the_threshold_is_copied_and_one_pixel_less_is_not() {
        // The boundary, in both directions, at the exact values.
        assert!(
            is_worth_copying(8, 8),
            "64 px is the threshold itself and must be accepted, or the rule is off by one"
        );
        assert!(!is_worth_copying(7, 9), "63 px is below the threshold");
        assert!(is_worth_copying(65, 1), "65 px is above it");
        assert!(!is_worth_copying(63, 1), "63 px again, as a strip");
    }

    #[test]
    fn a_click_never_reaches_the_clipboard() {
        // The PRD's case 6, as values. A click is small in BOTH axes at once,
        // which is what makes an area rule the right one.
        for (width, height) in [(1, 1), (2, 2), (4, 4), (1, 10), (3, 5)] {
            assert!(
                !is_worth_copying(width, height),
                "{width}x{height} is a click; it must not touch the clipboard"
            );
        }
    }

    #[test]
    fn a_deliberate_thin_strip_is_copied_even_though_one_side_is_one_pixel() {
        // What an area rule buys over a minimum per side: 200 x 1 is a gesture
        // somebody makes on purpose.
        assert!(is_worth_copying(200, 1));
        assert!(is_worth_copying(1, 64));
    }

    #[test]
    fn a_huge_selection_does_not_wrap_into_a_refusal() {
        // `u32 * u32` overflows a `u32`. In debug that panics on an IPC thread;
        // in release it would silently refuse a full-screen selection.
        assert!(is_worth_copying(u32::MAX, u32::MAX));
        assert!(is_worth_copying(65_536, 65_536));
    }

    #[test]
    fn the_refusal_says_what_was_measured_and_what_was_required() {
        let message = too_small_line(3, 5);

        assert!(message.contains("3x5"), "unexpected: {message}");
        assert!(message.contains("15"), "the area must be named: {message}");
        assert!(
            message.contains("64"),
            "the threshold must be named: {message}"
        );
        assert!(
            message.contains("NOT touched"),
            "the user has to learn their clipboard is intact: {message}"
        );
    }

    // ---------------------------------------------------------------------
    // The conversion to `tauri::image::Image`.
    // ---------------------------------------------------------------------

    #[test]
    fn a_cut_becomes_an_image_of_the_same_dimensions_and_the_same_bytes() {
        let cut = known_cut();

        let image = to_image(cut.width(), cut.height(), cut.pixels()).expect("a Frame is coherent");

        assert_eq!(
            (image.width(), image.height()),
            (8, 8),
            "width and height must not be swapped on the way in"
        );
        assert_eq!(
            image.rgba(),
            cut.pixels(),
            "the bytes must be handed over untouched - no encode, no channel swap"
        );
    }

    #[test]
    fn a_buffer_that_contradicts_its_dimensions_is_refused_before_arboard_sees_it() {
        // `Image::new` checks NOTHING (read in the tauri 2.11.5 source), and
        // `arboard` reads width * height * 4 bytes out of whatever it is given.
        // Bound to locals: `to_image` BORROWS its buffer, so a temporary would
        // not outlive the assertion.
        let one_short = vec![0u8; 8 * 8 * 4 - 1];
        let one_long = vec![0u8; 8 * 8 * 4 + 1];
        let exact = vec![0u8; 8 * 8 * 4];
        let far_too_short = vec![0u8; 3];

        assert!(
            to_image(8, 8, &one_short).is_err(),
            "one byte short must be refused"
        );
        assert!(
            to_image(8, 8, &one_long).is_err(),
            "one byte too many means the dimensions lie"
        );
        assert!(
            to_image(8, 8, &exact).is_ok(),
            "the exact length must pass, or the check refuses everything"
        );

        let message = to_image(8, 8, &far_too_short).expect_err("3 bytes is not a 8x8 image");
        assert!(
            message.contains("3") && message.contains("256"),
            "both lengths must be named to be actionable: {message}"
        );
    }

    #[test]
    fn a_zero_dimension_is_refused_even_though_its_length_check_would_pass() {
        // 0 * 0 * 4 == 0 == the length of an empty buffer, so the arithmetic
        // alone would wave this through.
        assert!(to_image(0, 0, &[]).is_err());
        assert!(to_image(0, 8, &[]).is_err());
        assert!(to_image(8, 0, &[]).is_err());
    }

    // ---------------------------------------------------------------------
    // The lines a human reads.
    // ---------------------------------------------------------------------

    #[test]
    fn the_success_line_names_the_size_the_time_and_the_budget_it_is_outside_of() {
        let line = success_line(3, 640, 360, 921_600, Duration::from_micros(4_240));

        assert!(line.contains("run 3"), "unexpected: {line}");
        assert!(line.contains("640x360"), "unexpected: {line}");
        assert!(line.contains("921600"), "unexpected: {line}");
        assert!(
            line.contains("4.2 ms"),
            "one decimal, like every other figure: {line}"
        );
        assert!(
            line.contains("OUTSIDE the 150 ms budget"),
            "a reader must not be able to add this to 1d's total: {line}"
        );
    }

    #[test]
    fn the_meter_report_says_on_its_first_line_that_it_is_not_the_pipeline_report() {
        let meter = Meter::new();

        let lines = meter.report_lines();

        let first = lines.first().expect("a report always has a header");
        assert!(
            first.contains("OUTSIDE the 150 ms budget"),
            "unexpected header: {first}"
        );
        assert!(
            lines.len() > 1,
            "the instrument's own lines must follow the header"
        );
    }

    #[test]
    fn the_clipboard_label_is_not_one_of_the_pipeline_labels() {
        // If it ever were, and someone folded the two instruments together, the
        // clipboard's human-scale duration would land in `total_median` - the
        // figure the 150 ms verdict is read off.
        use crate::capture::MARK_CAPTURE;
        use crate::veil::{MARK_PAINTED, MARK_SHOWN, MARK_TRANSPORT};

        assert_eq!(MARK_CLIPBOARD, "clipboard");
        for label in [MARK_CAPTURE, MARK_TRANSPORT, MARK_SHOWN, MARK_PAINTED] {
            assert_ne!(MARK_CLIPBOARD, label);
        }
    }

    #[test]
    fn the_cut_the_round_trip_would_use_is_exactly_at_the_threshold() {
        // `known_cut` is written for a round trip through a REAL clipboard (see
        // "What is NOT proven here" in the module header). It is 8 x 8 = 64 px,
        // the threshold itself, so that test - the day it can exist - also
        // exercises the boundary in the real path. If someone shrinks the
        // fixture, it stops being copyable at all.
        let cut = known_cut();

        assert_eq!((cut.width(), cut.height()), (8, 8));
        assert!(is_worth_copying(cut.width(), cut.height()));
        assert_eq!(
            u64::from(cut.width()) * u64::from(cut.height()),
            MIN_COPYABLE_AREA_PX
        );
    }
}
