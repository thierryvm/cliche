//! The veil: a preheated, hidden, full-screen window that paints the frozen
//! screen, and the two transports that carry the pixels to it.
//!
//! This module exists to answer ONE question, honestly: from the moment the
//! shortcut handler is entered, to the moment a full-screen overlay showing the
//! frozen screen is painted - is the median under 150 ms?
//!
//! # The window is built at startup, never at shortcut time
//!
//! [`create`] is called from `setup()`. Creating a window means creating a
//! WebView2 instance, loading a document and running its script: hundreds of
//! milliseconds, once. Doing that inside the shortcut handler would put it
//! inside the budget, and hiding it outside the budget would be worse - it
//! would be a measurement of something the user never experiences. So the
//! window exists, hidden, from the first second of the process, and a capture
//! only fills it and shows it.
//!
//! Since 4 September 2026 the shortcut handler does not show it either: it hands
//! the frame to the page and returns, and the window is shown by
//! [`veil_decoded`] once the page says the image is ready to draw. The ORDER
//! comment in [`perform_capture`] has the reasoning and the objection it routes
//! around.
//!
//! # The five steps, and what each one really covers
//!
//! | label | from | to | window |
//! | --- | --- | --- | --- |
//! | `capture` | handler entry | the RGBA frame is in hand | hidden |
//! | `transport` | there | the payload is built and staged | hidden |
//! | `decoded` | there | `eval` -> fetch -> `decode()` -> acknowledgement | HIDDEN |
//! | `shown` | there | `show()` and `set_focus()` have returned | becomes visible |
//! | `painted` | there | one animation frame, acknowledged | visible |
//!
//! `transport` deliberately covers EVERYTHING between having the frame and
//! having something the page can load - for transport B that includes the PNG
//! encode and the base64. Splitting them would make the two transports
//! incomparable, which is the only thing this measurement is for.
//!
//! # THE MEASUREMENT CONTRACT - both ends of the total are where they were
//!
//! The pipeline was reordered on 4 September 2026 (see the ORDER comment in
//! [`perform_capture`]), and a reordering that also moved the goalposts would
//! make every before/after comparison worthless. So the TOTAL still starts at
//! the entry of the shortcut handler and still ends when the page has painted a
//! VISIBLE window. Only the inside changed.
//!
//! ## The prediction, written down before the measurement, so it can be wrong
//!
//! Against the 18 clean runs of 4 September 2026 on 868ba0d - capture 23.4/25.4,
//! transport 1.4/1.6, shown 0.0/0.2, painted 91.3/94.3, TOTAL 115.3/121.7
//! (median/p95, ms) - this reordering predicts:
//!
//! - `decoded` lands near the OLD `painted` minus one animation frame: it covers
//!   the same fetch and decode, without the rAF that used to precede the
//!   acknowledgement.
//! - `shown` stays near zero. It covers `show()` and `set_focus()`, exactly what
//!   it covered before.
//! - `painted` collapses to one animation frame plus one IPC round trip.
//! - **The TOTAL RISES**, by roughly one extra IPC round trip: the page now
//!   reports twice where it used to report once.
//!
//! **A TOTAL that falls sharply is a DEFECT, not a win.** It would mean a step
//! stopped being counted - most likely a run being filed before it was really
//! painted. The two ends of the total are fixed; nothing inside can make the
//! whole trip shorter than it was.
//!
//! # `painted` is an approximation, and its two errors point OPPOSITE ways
//!
//! The page acknowledges from inside a `requestAnimationFrame` callback taken
//! AFTER `HTMLImageElement.decode()` resolves - scheduled in the same turn as
//! the `veil_decoded` call that asks Rust to show the window, so the frame it
//! runs in is one of a window that is visible or about to be. Rust timestamps
//! when the acknowledgement arrives. That number is:
//!
//! - **too large** by the return trip of the acknowledgement itself (webview ->
//!   IPC -> command handler), and
//! - **too small** because a `requestAnimationFrame` callback runs BEFORE the
//!   compositor presents the frame it belongs to. Nothing available to a page
//!   proves presentation.
//!
//! So it is NOT a clean upper bound, and calling it one would be the flattering
//! version. The first error is very likely the larger of the two - an IPC round
//! trip against one compositor frame - but that has not been measured here, and
//! "likely" is not "measured". Read the total as an estimate with a known
//! over-count and a known under-count, both named.
//!
//! # Nothing here may panic
//!
//! Everything below runs either on the global-shortcut thread, on a webview IPC
//! thread, on the benchmark thread, or - since 4 September 2026 - on one of the
//! short-lived fallback threads [`arm_show_fallback`] spawns. A panic on any of
//! them takes the application down, and losing the app to a diagnostic is a bad
//! trade.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{
    AppHandle, Manager, PhysicalPosition, PhysicalSize, Webview, WebviewUrl, WebviewWindowBuilder,
};

use crate::capture::{self, Frame, MARK_CAPTURE};
use crate::clipboard;
use crate::geometry::{self, CssRect};
use crate::ipc;
use crate::timing::Timings;

/// Label of the veil window. Used by `create`, by the pipeline, and by the
/// commands; one constant so the three cannot drift apart.
pub const VEIL_WINDOW_LABEL: &str = "veil";

/// Name of the custom URI scheme Rust serves the frozen frame on.
pub const VEIL_SCHEME: &str = "cliche";

/// The origin WebView2 turns [`VEIL_SCHEME`] into ON WINDOWS.
///
/// Tauri serves custom schemes over `http://<scheme>.localhost/...` on Windows
/// and Android; on macOS and Linux the same scheme appears as
/// `cliche://localhost/...`. Cliché bundles for Windows only (`"targets":
/// ["nsis"]`), so this constant is the Windows form - and this comment is here
/// so that a future macOS port fails on a grep rather than on a blank veil.
///
/// This exact string is what has to appear in `img-src` in `tauri.conf.json`,
/// and it is the ONLY custom-scheme origin that directive allows. The `asset:`
/// and `http://asset.localhost` entries that used to sit beside it were removed
/// on 4 September 2026: `assetProtocol` is not configured, `tauri` is built with
/// `features = []` so `protocol-asset` is not compiled in, and nothing in the
/// repository calls `convertFileSrc`. They allowed a source for a protocol that
/// does not answer.
pub const VEIL_ORIGIN: &str = "http://cliche.localhost";

/// Timing label for building and staging the payload.
pub const MARK_TRANSPORT: &str = "transport";

/// Timing label for the page reporting the frame DECODED, window still hidden.
///
/// New on 4 September 2026, and the step the reordering exists for: it covers
/// `eval` -> fetch -> `HTMLImageElement.decode()` -> the acknowledgement's trip,
/// all of it behind a window nobody can see yet.
pub const MARK_DECODED: &str = "decoded";

/// Timing label for the window becoming visible.
pub const MARK_SHOWN: &str = "shown";

/// Timing label for the webview's paint acknowledgement.
pub const MARK_PAINTED: &str = "painted";

/// Environment variable choosing the transport, read once at startup.
pub const TRANSPORT_ENV: &str = "CLICHE_TRANSPORT";

/// Environment variable asking for N automated runs, read once at startup.
pub const BENCH_ENV: &str = "CLICHE_BENCH";

/// How the frozen frame reaches the page.
///
/// Both are compiled in, always. The choice is made at startup from an
/// environment variable so that twenty runs of one and twenty of the other need
/// a restart, not a rebuild.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// **A** - Rust serves a BMP from memory on the `cliche:` scheme, the page
    /// loads it with an `<img>`. See `capture::encode_bmp` for why BMP.
    CustomProtocolBmp,
    /// **B** - PNG, base64, pushed into the page as a `data:` URL. Known to
    /// start 69.6 ms in the red (measured 3 September 2026); implemented so
    /// that the figure is CONSTATED rather than assumed.
    DataUrlPng,
}

impl Transport {
    /// Value accepted for [`Transport::CustomProtocolBmp`].
    pub const NAME_BMP: &'static str = "bmp";
    /// Value accepted for [`Transport::DataUrlPng`].
    pub const NAME_PNG: &'static str = "png";

    /// Reads the transport out of a raw environment value.
    ///
    /// Pure, so the parsing is under test without touching the process
    /// environment. Returns the transport AND, when the value was not
    /// understood, the line to print: silently falling back to the default
    /// would mean measuring transport A while believing you measured B, which
    /// is the one mistake that would invalidate the whole comparison.
    pub fn parse(raw: Option<&str>) -> (Self, Option<String>) {
        let Some(raw) = raw else {
            return (Self::CustomProtocolBmp, None);
        };

        let trimmed = raw.trim();
        if trimmed.eq_ignore_ascii_case(Self::NAME_BMP) {
            (Self::CustomProtocolBmp, None)
        } else if trimmed.eq_ignore_ascii_case(Self::NAME_PNG) {
            (Self::DataUrlPng, None)
        } else {
            (
                Self::CustomProtocolBmp,
                Some(format!(
                    "[cliche] veil: {TRANSPORT_ENV}=\"{raw}\" is not understood; expected \
                     \"{bmp}\" or \"{png}\". FALLING BACK to \"{bmp}\" - the figures below are \
                     transport A, whatever you meant to measure.",
                    bmp = Self::NAME_BMP,
                    png = Self::NAME_PNG,
                )),
            )
        }
    }

    /// How the transport names itself in the report header.
    pub fn describe(self) -> &'static str {
        match self {
            Self::CustomProtocolBmp => "A - custom protocol, BMP (header + memcpy)",
            Self::DataUrlPng => "B - data URL, PNG + base64",
        }
    }
}

/// Shared state of the veil. Managed by Tauri, borrowed as `State<Veil>`.
///
/// Deliberately NOT `Debug`, for the reason `capture::Frame` spells out: a
/// derived one would let a stray `{:?}` print 8.29 MB of screenshot into
/// whatever log it landed in.
pub struct Veil {
    transport: Transport,
    /// The payload the custom protocol will hand over, with the run number it
    /// belongs to. `None` once served: a frame is fetched exactly once, and
    /// taking it is what makes serving it a MOVE rather than an 8.29 MB copy.
    pending: Mutex<Option<(u64, Vec<u8>)>>,
    /// The frozen frame currently on screen, with the run it belongs to. Kept
    /// so that a selection can be cut out of it; the run number is what lets a
    /// selection drawn on a stale image be refused instead of cutting the wrong
    /// screenshot.
    ///
    /// Unlike `pending`, this is NOT taken when read: the user may draw a
    /// second rectangle. Where it is written, and what that costs the budget,
    /// is in [`perform_capture`].
    frame: Mutex<Option<(u64, Frame)>>,
    /// Run number, and the cache-buster in the URL. WebView2 would happily
    /// re-serve a previous response for an identical URL, and a `painted` that
    /// measured a cache hit would be a fabricated 2 ms.
    generation: AtomicU64,
    /// The highest run that has already been made VISIBLE, or zero.
    ///
    /// Since 4 September 2026 two paths reach `show()` - the page's
    /// `veil_decoded` and [`arm_show_fallback`]'s timer - and they race by
    /// design. This is what makes the second arrival a no-op. The rule is
    /// [`may_show`], the atomic application of it is [`Veil::claim_show`], and
    /// both are under test.
    shown: AtomicU64,
    /// How many runs reached `painted`. The benchmark waits on this, and the
    /// report is printed every twenty.
    painted: AtomicUsize,
}

impl Veil {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport,
            pending: Mutex::new(None),
            frame: Mutex::new(None),
            generation: AtomicU64::new(0),
            shown: AtomicU64::new(0),
            painted: AtomicUsize::new(0),
        }
    }

    pub fn transport(&self) -> Transport {
        self.transport
    }

    /// Opens the next run number.
    ///
    /// Extracted from [`perform_capture`] so that the show claim below can be
    /// raced in a test: a `Veil` is a plain value, an `AppHandle` is not.
    /// `saturating_add` for the reason everything on this path saturates - a
    /// debug overflow panic on the hotkey thread would end the application.
    fn next_run(&self) -> u64 {
        self.generation
            .fetch_add(1, Ordering::Relaxed)
            .saturating_add(1)
    }

    /// Claims the right to make the veil visible for `run`.
    ///
    /// `true` for exactly ONE caller per run number, and never for a run the
    /// pipeline has moved past. This is the whole of property D.
    ///
    /// `fetch_max` is the atomic half of the `run > already_shown` rule: it
    /// hands back the PREVIOUS value, so of the two callers that race here -
    /// the page's acknowledgement and the fallback timer - exactly one can
    /// observe a value below `run`. `Relaxed` is enough because nothing else is
    /// published through this location: the only thing being decided is which
    /// caller proceeds, and a read-modify-write on one atomic is totally
    /// ordered whatever the ordering asked for.
    ///
    /// **The staleness half is a plain load, so it is best effort, and saying
    /// so matters.** A second shortcut press can bump `generation` between that
    /// load and the `fetch_max`, in which case a run about to go stale shows the
    /// veil a moment before the newer one shows it again. `Timings::begin_run`
    /// already discards the run that press interrupted, so nothing is measured
    /// twice. What this guard promises is one `show()` per RUN NUMBER, not one
    /// `show()` per window.
    fn claim_show(&self, run: u64) -> bool {
        if !may_show(
            self.generation.load(Ordering::Relaxed),
            self.shown.load(Ordering::Relaxed),
            run,
        ) {
            return false;
        }

        self.shown.fetch_max(run, Ordering::Relaxed) < run
    }

    /// Takes the lock, recovering a poisoned one. Same reasoning as
    /// `Timings::state`: the guarded value is a byte buffer with no invariant,
    /// and a panic elsewhere must not turn every later capture into a crash.
    fn pending(&self) -> std::sync::MutexGuard<'_, Option<(u64, Vec<u8>)>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Same recovery, same reason, for the retained frame.
    fn frame(&self) -> std::sync::MutexGuard<'_, Option<(u64, Frame)>> {
        self.frame
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Drops everything this run is holding: the frozen frame AND the payload
    /// staged for the custom protocol.
    ///
    /// **Both, always.** `pending` used to be emptied by `serve` and by nothing
    /// else, so Escape - or a `show()` that failed, or an `eval()` that failed -
    /// left 8.29 MB of screen on the heap until the next capture, in a process
    /// that stays open for days. The user believes nothing was kept; their
    /// password manager is still in a buffer that a minidump, the page file, or
    /// any process of the same session can read. Whatever ends a capture must
    /// come through here.
    ///
    /// It ALSO burns the show claim of the run in flight, and that line arrived
    /// on 4 September 2026 with the fallback timer. Escape at 200 ms leaves a
    /// timer counting down to 250 ms; without this the veil the user just
    /// dismissed would come back up over whatever they turned to - and empty,
    /// since the frame has just been dropped two lines above. The page cannot
    /// ordinarily receive Escape while the window is hidden, so this closes a
    /// race rather than a bug that was observed; it costs one atomic on a path
    /// that has already ended a capture.
    ///
    /// The claim burnt is the one of `run`, NOT of whatever `generation` reads
    /// at that instant, and that distinction was a real defect until the review
    /// of 4 September 2026 found it. Reading `generation` here meant a caller
    /// closing run 1 could burn the claim of run 2: a second shortcut press
    /// landing between a caller's `claim_show(1)` and its `release()` moved
    /// `generation` to 2, and `fetch_max(2)` then made run 2 unshowable
    /// FOR EVER - the "nothing happens" defect this whole reorder exists to
    /// prevent. Narrow, microseconds wide, and the kind of thing that is
    /// impossible to diagnose from the symptom.
    ///
    /// Callers that know the run they are closing pass it. `veil_dismissed` has
    /// none of its own - the page does not send one - so it passes
    /// [`Veil::current_run`], which is what "the capture on screen" means
    /// there. Giving that command a run number is a separate improvement.
    ///
    /// Split out of `veil_dismissed` so it can be tested: building an
    /// `AppHandle` needs an event loop a test binary has not, but a `Veil` is
    /// a plain value.
    fn release(&self, run: u64) {
        *self.pending() = None;
        *self.frame() = None;
        self.shown.fetch_max(run, Ordering::Relaxed);
    }

    /// The run the pipeline is on, for the one caller that has no run of its
    /// own. See [`Veil::release`].
    fn current_run(&self) -> u64 {
        self.generation.load(Ordering::Relaxed)
    }
}

/// Whether a run may be made visible, given the run the pipeline is on and the
/// highest run already shown.
///
/// Pure, and deliberately apart from the atomics that apply it, for the reason
/// `ipc::is_from` is apart from `Webview`: a decision over three integers can be
/// put to every row of its own table, and a decision that needs an event loop
/// cannot be tested at all. Both halves of the rule earn their place:
///
/// - `run == current_run` - an acknowledgement belonging to a capture the user
///   has already finished with must not raise the window back up.
/// - `run > already_shown` - whichever of the two paths gets here first shows
///   the veil; the second does nothing at all.
///
/// `run != 0` because zero is what the page holds when nothing is on screen,
/// and what `generation` reads before the first capture.
fn may_show(current_run: u64, already_shown: u64, run: u64) -> bool {
    run != 0 && run == current_run && run > already_shown
}

/// Builds the veil window, HIDDEN, sized to the primary monitor.
///
/// Called from `setup()`. Returns the line to print on failure rather than
/// printing it: the caller decides whether a veil-less application keeps going.
pub fn create(app: &AppHandle) -> Result<(), String> {
    let window =
        WebviewWindowBuilder::new(app, VEIL_WINDOW_LABEL, WebviewUrl::App("veil.html".into()))
            .title("Cliche veil")
            // Not a window in the usual sense: no chrome, no taskbar entry, no
            // shadow, nothing the user can grab. It is a sheet of glass over the
            // screen.
            .decorations(false)
            .resizable(false)
            .skip_taskbar(true)
            .shadow(false)
            .always_on_top(true)
            // THE line that makes this lot honest. The window is constructed now,
            // at startup, and stays invisible until a shortcut shows it.
            .visible(false)
            .build()
            .map_err(|error| format!("[cliche] veil: could not create the veil window: {error}"))?;

    // Sized here, once, and not at shortcut time: `set_size` crosses to the
    // main thread and would land inside the budget. Physical pixels, because
    // that is what the capture is in - a logical-pixel rectangle is wrong by
    // the scale factor on any screen above 100 %, and invisibly right at 100 %.
    match app.primary_monitor() {
        Ok(Some(monitor)) => {
            let position = *monitor.position();
            let size = *monitor.size();
            if let Err(error) = window.set_position(PhysicalPosition::new(position.x, position.y)) {
                eprintln!("[cliche] veil: could not place the veil window: {error}");
            }
            if let Err(error) = window.set_size(PhysicalSize::new(size.width, size.height)) {
                eprintln!("[cliche] veil: could not size the veil window: {error}");
            }
            println!(
                "[cliche] veil: preheated, hidden, {}x{} physical px at ({}, {})",
                size.width, size.height, position.x, position.y
            );
        }
        Ok(None) => {
            eprintln!("[cliche] veil: no primary monitor reported; the veil keeps its default size")
        }
        Err(error) => eprintln!("[cliche] veil: could not read the primary monitor: {error}"),
    }

    Ok(())
}

/// How long after the page was handed the frame the veil is shown ANYWAY.
///
/// **This is not a tuning knob, it is the thing that keeps a missing
/// acknowledgement from turning the shortcut into "nothing happens".** That
/// defect was called blocking on 3 September 2026, and a missing acknowledgement
/// is not hypothetical: in the 18-run session of 4 September 2026 on 868ba0d,
/// runs 1 and 2 never acknowledged inside [`BENCH_RUN_TIMEOUT`]. Under the old
/// order they still showed a veil, because `show()` came first; under this one
/// they would show nothing at all.
///
/// A run rescued this way is ABANDONED, never measured: it is a run whose
/// timings are unknown by definition, and letting it into a median would flatter
/// the median with the very failure it is there to report.
const SHOW_FALLBACK: Duration = Duration::from_millis(250);

/// The `painted` p95 [`SHOW_FALLBACK`] is chosen against.
///
/// PROVENANCE, because a threshold without one is a taste: 94.3 ms is the
/// `painted` p95 of the session Thierry ran on 4 September 2026 against commit
/// 868ba0d - 18 clean runs, median 91.3 ms. Under that pipeline `painted`
/// covered fetch + decode + one `requestAnimationFrame` + the acknowledgement's
/// own trip, which is within one animation frame of what [`MARK_DECODED`] covers
/// now.
///
/// Held against `SHOW_FALLBACK` by a test, which is the only place these two
/// numbers ever meet - hence `cfg(test)`. It is a MEASUREMENT the threshold is
/// justified by, not a value the application reads; compiled into the binary it
/// would be dead weight, and `clippy -D warnings` would say so.
#[cfg(test)]
const DECODE_P95_MEASURED: Duration = Duration::from_micros(94_300);

/// One capture, start to finish. THE function under measurement.
///
/// Called by the global shortcut handler and by the benchmark, so that the two
/// paths differ only in what precedes this call - see [`spawn_bench`].
///
/// Takes no `Result`: every failure is printed and abandons the run, because a
/// failed run that was counted as a fast one would poison the median.
pub fn perform_capture(app: &AppHandle) {
    // `try_state`, never `state`: `state` panics when the type was never
    // managed, and a panic on the hotkey thread ends the application.
    let Some(timings) = app.try_state::<Timings>() else {
        eprintln!("[cliche] veil: no timing instrument is managed; this run was not measured");
        return;
    };
    let Some(veil) = app.try_state::<Veil>() else {
        eprintln!("[cliche] veil: no veil state is managed; this run was not measured");
        return;
    };
    let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) else {
        eprintln!("[cliche] veil: the veil window does not exist; this run was not measured");
        return;
    };

    // t0. The handler was entered; everything after this line is inside the
    // budget. What happened BEFORE - the physical key, the Windows low-level
    // hook, the global-hotkey thread - is not observable from this process and
    // is therefore outside every figure printed. Unknown is not zero.
    timings.begin_run();

    let frame = match capture::capture_primary() {
        Ok(frame) => frame,
        Err(error) => {
            eprintln!("[cliche] veil: capture failed: {error}");
            timings.abandon_run();
            return;
        }
    };
    timings.mark(MARK_CAPTURE);

    let run = veil.next_run();

    let source = match veil.transport {
        Transport::CustomProtocolBmp => match capture::encode_bmp(&frame) {
            Ok(bmp) => {
                *veil.pending() = Some((run, bmp));
                // Built from a constant and an integer, so it contains no
                // character that would need escaping in the JavaScript string
                // literal below. Safe BY CONSTRUCTION, not by inspection.
                format!("{VEIL_ORIGIN}/frame/{run}.bmp")
            }
            Err(error) => {
                eprintln!("[cliche] veil: BMP failed: {error}");
                timings.abandon_run();
                return;
            }
        },
        Transport::DataUrlPng => match capture::encode_png(&frame) {
            Ok(png) => {
                let mut url =
                    String::with_capacity(DATA_URL_PREFIX.len() + png.len().div_ceil(3) * 4);
                url.push_str(DATA_URL_PREFIX);
                push_base64(&png, &mut url);
                url
            }
            Err(error) => {
                eprintln!("[cliche] veil: PNG failed: {error}");
                timings.abandon_run();
                return;
            }
        },
    };
    timings.mark(MARK_TRANSPORT);

    // ORDER: hand the image over, and show NOTHING here.
    //
    // THIS IS THE REVERSE OF WHAT STOOD HERE UNTIL 4 SEPTEMBER 2026, and the
    // comment it replaces argued against the change. That argument is preserved
    // rather than deleted, because it is still sound: **WebView2 throttles (and
    // may stop) `requestAnimationFrame` in a window that is not visible**, so an
    // acknowledgement scheduled from a hidden page could arrive late, or never,
    // for a reason having nothing to do with the transport. It set one
    // condition for revisiting: that somebody MEASURE what a hidden WebView2
    // actually does.
    //
    // THAT MEASUREMENT HAS STILL NOT BEEN TAKEN. Nobody has observed a hidden
    // WebView2 on this machine, and this lot did not change that. What changed
    // is that the objection no longer applies to the hidden stretch, because
    // there is no `requestAnimationFrame` in it: the page answers with
    // `veil_decoded` from the `HTMLImageElement.decode()` promise, which
    // resolves on the image-decoding pipeline and is not a callback the
    // compositor schedules. The rAF has moved AFTER the window is visible,
    // where a throttle cannot reach it. The objection was routed around, not
    // refuted.
    //
    // What the old order cost, measured: `show()` returned, the window became
    // visible, and the page then spent the whole fetch-and-decode - median
    // 91.3 ms, p95 94.3 ms over 18 clean runs on 868ba0d, 4 September 2026 -
    // displaying whatever it held from the previous capture. That stretch is
    // the flashing.
    //
    // The hypothesis this now rests on is written down so it can be falsified:
    // IF a hidden WebView2 also stalls `decode()`, `veil_decoded` never
    // arrives. `arm_show_fallback` below is what stops that from turning the
    // shortcut into "nothing happens", and it is why that fallback is not
    // optional.
    //
    // `eval` is `ExecuteScriptAsync` on Windows: it returns before the script
    // has run, which is exactly right - the rest of the trip is the page's, and
    // the page is what reports `decoded`.
    if let Err(error) = window.eval(format!("window.__clicheShow(\"{source}\",{run})")) {
        eprintln!("[cliche] veil: could not hand the frame to the page: {error}");
        timings.abandon_run();
        // Kept from the old order even though this function no longer shows
        // anything: the window can still be up from a capture this press
        // interrupted, and that capture is now dead too. Leaving it would show
        // a stale frozen screen with no run behind it.
        let _ = window.hide();
        // The page was never told the URL, so the staged payload has no reader
        // and must not survive the run - 8.29 MB of the user's screen, in a
        // process that stays open for days.
        veil.release(run);
        return;
    }

    // Armed AFTER the `eval`, so the countdown starts when the page was handed
    // its work. Nothing between here and the acknowledgement is marked, so the
    // cost of spawning this thread inflates no step of the report - but it is
    // not free, and it runs on the shortcut thread, so it is stated rather than
    // implied.
    arm_show_fallback(app, run);

    // The frame is kept so the selection can be cut out of it (`veil_selected`),
    // and it is kept HERE - after the page has been handed the image - on
    // purpose. Read this before moving it somewhere that reads more naturally.
    //
    // What it costs:
    //
    // - Moving a `Frame` is a 24-byte move. The 8.29 MB `Vec` travels by
    //   pointer; nothing is copied.
    // - Replacing the slot drops whatever was in it. In the ORDINARY path that
    //   is `None` and costs nothing: both ways a capture ends - `veil_selected`
    //   and `veil_dismissed` - empty THIS slot. (Only this one. Until
    //   4 September 2026 `pending` was emptied by `serve` alone, so a
    //   dismissed or failed run kept its staged BMP; `Veil::release` is what
    //   now closes both, and every path that ends a run calls it.)
    //   The only case that frees 8.29 MB
    //   here is the shortcut pressed twice without the veil being closed in
    //   between, which is the run `Timings::begin_run` already discards.
    //   It is still why this line sits after `eval` rather than next to the
    //   encode where it would read more naturally: it is off the stretch
    //   between the shortcut and the image reaching the page.
    //
    //   WHAT IT NO LONGER IS, since the reorder of 4 September 2026, and the
    //   sentence that stood here said otherwise: this is not "past the last
    //   mark". `decoded`, `shown` and `painted` are all taken AFTER it now, on
    //   the webview's IPC thread. So in the double-press case this drop runs
    //   CONCURRENTLY with the decode it is no longer in front of, and could in
    //   principle contend for the allocator with the thread being measured.
    //   That contention has NOT been measured, and it only exists on a run
    //   `begin_run` has already discarded.
    // - Peak memory rises to three buffers for a moment on transport A - the
    //   previous frame, this frame, and its BMP copy - about 25 MB at
    //   1920 x 1080. Stated rather than discovered from a task manager.
    *veil.frame() = Some((run, frame));
}

/// Shows the veil [`SHOW_FALLBACK`] from now, unless the page got there first.
///
/// # Why this is not optional
///
/// Since the reordering, the ONLY thing that makes the veil appear on a healthy
/// run is `veil_decoded` coming back from the page. An acknowledgement that
/// never arrives therefore turns Ctrl+Shift+2 into "nothing happens" - the
/// defect Thierry called blocking on 3 September 2026 - where the old order
/// would at least have shown a window. And a missing acknowledgement is not a
/// thought experiment: in the 18-run session of 4 September 2026 on 868ba0d,
/// runs 1 and 2 never acknowledged within [`BENCH_RUN_TIMEOUT`].
///
/// # The run it rescues is thrown away, deliberately
///
/// `abandon_run`, never `finish_run`. A run shown by this path has no `decoded`
/// and no `shown` mark, so its latency is unknown by construction; letting it
/// into a median would flatter the median with the very failure the fallback
/// exists to report. The terminal line is what a human reads instead, and the
/// report's discarded-run count is where it shows up as a figure.
///
/// # What the user sees when it fires
///
/// A BLACK veil that fills in when the decode eventually finishes - never the
/// previous capture. `__clicheShow` sets `frame.hidden = true` before it touches
/// `src`, and that line matters more now than it ever did: it used to guard
/// against a stale flash of one frame, and it is now what makes this fallback
/// safe to fire at all.
///
/// # A thread, and what that costs
///
/// One thread per capture, alive for 250 ms. Not a timer plugin, because there
/// is none in this dependency set and adding one for a sleep is not a trade
/// worth making. The spawn happens after the last mark this function takes, so
/// it inflates no step of the report; it is not free all the same, and it runs
/// on the global-shortcut thread. Nothing in the closure may panic, for the
/// reason the module header gives.
fn arm_show_fallback(app: &AppHandle, run: u64) {
    let app = app.clone();

    std::thread::spawn(move || {
        std::thread::sleep(SHOW_FALLBACK);

        let Some(veil) = app.try_state::<Veil>() else {
            return;
        };

        // The claim IS the race. If the page acknowledged - or if the capture
        // was dismissed, which burns the claim through `Veil::release` - this
        // returns false and the timer was simply never needed.
        if !veil.claim_show(run) {
            return;
        }

        let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) else {
            eprintln!(
                "[cliche] veil: run {run} was not acknowledged and the veil window does not \
                 exist; nothing can be shown"
            );
            if let Some(timings) = app.try_state::<Timings>() {
                timings.abandon_run();
            }
            veil.release(run);
            return;
        };

        if let Err(error) = window.show() {
            eprintln!("[cliche] veil: fallback could not show the veil: {error}");
            if let Some(timings) = app.try_state::<Timings>() {
                timings.abandon_run();
            }
            veil.release(run);
            return;
        }
        // Same reason as on the acknowledged path: without focus, Escape never
        // reaches the veil - and a veil shown by the fallback is exactly the one
        // a user is most likely to want to be rid of.
        if let Err(error) = window.set_focus() {
            eprintln!("[cliche] veil: fallback could not focus the veil: {error}");
        }

        if let Some(timings) = app.try_state::<Timings>() {
            timings.abandon_run();
        }

        // Neither `decoded` nor `shown` is marked: this run is not a
        // measurement, and half a pipeline filed under those labels would make
        // the report worse, not better.
        eprintln!(
            "[cliche] veil: run {run} did NOT acknowledge its decode within {SHOW_FALLBACK:?}; \
             the veil was shown by the FALLBACK and this run is NOT measured"
        );
    });
}

/// Prefix of a transport-B data URL. Its bytes, plus the base64 alphabet, are
/// the only characters that can appear in the URL - none of them needs escaping
/// inside a double-quoted JavaScript string.
const DATA_URL_PREFIX: &str = "data:image/png;base64,";

/// The 64 characters of standard base64, plus `=` for padding. Fixed, so that
/// [`push_base64`] provably cannot emit a quote, a backslash or a newline.
const BASE64_ALPHABET: [u8; 64] =
    *b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Appends `bytes` to `out` as standard, padded base64.
///
/// **This function is compiled WITHOUT optimisation.** `Cargo.toml` optimises
/// dependencies only, on purpose, so that this crate stays debuggable. A byte
/// loop over a ~1.5 MB PNG therefore costs several times what a release build
/// would. Transport B's `transport` figure is inflated by that amount, and
/// must be read as a PESSIMISTIC number - which is acceptable here only because
/// transport B already spends 46 % of the budget on the PNG encode alone, a
/// figure that comes from an OPTIMISED dependency and is not inflated at all.
///
/// Every index into [`BASE64_ALPHABET`] is masked to six bits, so it is in
/// 0..=63 by construction and cannot be out of bounds.
fn push_base64(bytes: &[u8], out: &mut String) {
    let mut chunks = bytes.chunks_exact(3);

    for chunk in chunks.by_ref() {
        if let [first, second, third] = *chunk {
            let group = (u32::from(first) << 16) | (u32::from(second) << 8) | u32::from(third);
            out.push(char::from(BASE64_ALPHABET[(group >> 18) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[(group >> 12) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[(group >> 6) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[group as usize & 0x3F]));
        }
    }

    match *chunks.remainder() {
        [first] => {
            let group = u32::from(first) << 16;
            out.push(char::from(BASE64_ALPHABET[(group >> 18) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[(group >> 12) as usize & 0x3F]));
            out.push_str("==");
        }
        [first, second] => {
            let group = (u32::from(first) << 16) | (u32::from(second) << 8);
            out.push(char::from(BASE64_ALPHABET[(group >> 18) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[(group >> 12) as usize & 0x3F]));
            out.push(char::from(BASE64_ALPHABET[(group >> 6) as usize & 0x3F]));
            out.push('=');
        }
        _ => {}
    }
}

/// Serves the staged frame on the `cliche:` scheme.
///
/// The bytes are TAKEN, not cloned: an 8.29 MB copy here would be a cost this
/// transport is specifically trying not to pay, and it would land inside the
/// budget. The consequence is deliberate and has to be said: a second request
/// for the same URL gets 404, the page never acknowledges, and the run is
/// discarded - visible in the report as a discarded run rather than as a fast
/// one.
///
/// **`caller` is the webview label, and it is checked first.** A scheme
/// registered with `register_uri_scheme_protocol` is served to every webview of
/// the process, `main` included. Because the buffer is TAKEN, a fetch from
/// `main` is not merely a read of the user's screen: it is a denial of service.
/// `main` polling `/frame/<n>.bmp` empties the slot, the veil then gets a 404,
/// never acknowledges, and Cliche stops capturing without a single message the
/// user would see. Tauri fills this label in from the webview that made the
/// request; it is not a value the page chooses.
pub fn serve(app: &AppHandle, caller: &str, path: &str) -> tauri::http::Response<Vec<u8>> {
    if let Err(refused) = ipc::ensure_from(caller, VEIL_WINDOW_LABEL, VEIL_SCHEME) {
        eprintln!("[cliche] veil: {refused}");
        return response(
            403,
            "text/plain",
            b"this scheme is not served here".to_vec(),
        );
    }

    let requested = parse_frame_path(path);

    let body = app.try_state::<Veil>().and_then(|veil| {
        let mut pending = veil.pending();
        match (requested, pending.as_ref()) {
            (Some(wanted), Some((staged, _))) if wanted == *staged => {
                pending.take().map(|(_, bytes)| bytes)
            }
            _ => None,
        }
    });

    match body {
        Some(bytes) => response(200, "image/bmp", bytes),
        None => {
            eprintln!("[cliche] veil: nothing staged for `{path}`; serving 404");
            response(404, "text/plain", b"no frame staged for this URL".to_vec())
        }
    }
}

/// Reads the run number out of `/frame/<n>.bmp`. `None` for anything else, so
/// an unexpected path is refused rather than answered with whatever is staged.
fn parse_frame_path(path: &str) -> Option<u64> {
    path.strip_prefix("/frame/")?
        .strip_suffix(".bmp")?
        .parse()
        .ok()
}

/// Builds a response without ever unwrapping the builder.
///
/// `Response::builder().body()` returns a `Result`; the only way it fails is an
/// invalid header set by us. Rather than `unwrap` on a webview thread, a
/// failure degrades to an empty 500 - the page then shows nothing, the run is
/// discarded, and the report says so.
///
/// # The ABSENCE of `Access-Control-Allow-Origin` is a security decision
///
/// Written down because it is a protection that exists only as a missing line,
/// and a missing line is the easiest thing in a file to add back "for
/// convenience". `<img src>` is not subject to CORS, so the veil displays the
/// frame without one - but `fetch()` and `XMLHttpRequest` ARE, and without that
/// header a script cannot READ these bytes. That is what keeps the frozen
/// screen from being turned into a `Blob`, a canvas readback or a POST body by
/// any script running in the veil document.
///
/// So: no `Access-Control-Allow-Origin` here, ever, and no wildcard "to make
/// debugging easier". Anything that needs the pixels reads them in Rust, where
/// they already are.
///
/// **Not measured, read.** This is the CORS rule as specified, not something
/// this project has observed WebView2 enforce. The webview-label check in
/// [`serve`] is the guard that does not depend on it.
fn response(status: u16, content_type: &str, body: Vec<u8>) -> tauri::http::Response<Vec<u8>> {
    tauri::http::Response::builder()
        .status(status)
        .header("Content-Type", content_type)
        // The veil must never be served from cache: a cached response would
        // make `painted` measure a memory read instead of a decode.
        .header("Cache-Control", "no-store")
        .body(body)
        .unwrap_or_else(|_| {
            let mut fallback = tauri::http::Response::new(Vec::new());
            *fallback.status_mut() = tauri::http::StatusCode::INTERNAL_SERVER_ERROR;
            fallback
        })
}

/// The page's acknowledgement that the frozen frame is DECODED - and the call
/// that makes the veil window visible.
///
/// This is the hinge of the reordering of 4 September 2026. Until then
/// `perform_capture` showed the window and then handed the image over, so the
/// window was visible for the whole decode - median 91.3 ms, p95 94.3 ms over
/// 18 clean runs on 868ba0d - showing whatever the page held before. That
/// stretch is the two flashes the veil was reported to have. Now the window is
/// shown HERE, when there is something in it.
///
/// The three marks are laid out so the report still reads as a pipeline:
/// `decoded` closes the hidden stretch, `shown` covers `show()` and
/// `set_focus()` alone, and `painted` is the animation frame that follows on a
/// window that is genuinely visible.
///
/// `set_focus` is not decoration. An always-on-top window Windows never
/// activated receives no key events, so without it Escape would not reach the
/// veil's own document and the only way out of a capture would be gone.
///
/// **Veil window only**, by capability AND in Rust, and this is the second most
/// dangerous of the four to leave open. `main` runs React and its dependency
/// tree; a `veil_decoded` from there would raise a full-screen, always-on-top,
/// undecorated window over the user's desktop with no capture behind it. Same
/// choice as `veil_painted` on the refusal - a printed line, because the veil's
/// devtools cannot be opened (see [`ipc`]).
#[tauri::command]
pub fn veil_decoded(app: AppHandle, webview: Webview, run: u64) {
    if let Err(refused) = ipc::ensure_from(webview.label(), VEIL_WINDOW_LABEL, "veil_decoded") {
        eprintln!("[cliche] veil: {refused}");
        return;
    }

    let Some(veil) = app.try_state::<Veil>() else {
        eprintln!("[cliche] veil: no veil state is managed; run {run} cannot be shown");
        return;
    };

    // CLAIMED FIRST, before a single mark is taken. Two things are refused here
    // and both matter: a stale acknowledgement, which would raise a window over
    // a capture the user has finished with, and the loser of the race against
    // the fallback timer, which would file a `decoded` into a run the fallback
    // has already abandoned.
    if !veil.claim_show(run) {
        return;
    }

    let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) else {
        eprintln!("[cliche] veil: the veil window does not exist; run {run} cannot be shown");
        // Both of these were missing until the review of 4 September 2026, and
        // the twin path in `arm_show_fallback` already did them: the claim was
        // taken just above, so nothing else will ever end this run. Without the
        // release, 8.29 MB of the user's screen lives on in a process that
        // stays open for days; without the abandon, the run stays open in the
        // instrument and the next `begin_run` reports it as interrupted.
        if let Some(timings) = app.try_state::<Timings>() {
            timings.abandon_run();
        }
        veil.release(run);
        return;
    };

    // Bound once rather than looked up three times, and NOT with a `let Some
    // ... else { return }`: showing the veil is what the user asked for, and an
    // unmanaged instrument must cost the measurement, never the capture.
    let timings = app.try_state::<Timings>();

    if let Some(timings) = &timings {
        timings.mark(MARK_DECODED);
    }

    if let Err(error) = window.show() {
        eprintln!("[cliche] veil: could not show the veil: {error}");
        if let Some(timings) = &timings {
            timings.abandon_run();
        }
        // Same reasoning as the `eval` failure in `perform_capture`: the staged
        // payload has no reader now, and 8.29 MB of the user's screen must not
        // outlive the capture that took it.
        veil.release(run);
        return;
    }
    if let Err(error) = window.set_focus() {
        eprintln!("[cliche] veil: could not focus the veil: {error}");
    }

    if let Some(timings) = &timings {
        timings.mark(MARK_SHOWN);
    }
}

/// The page's acknowledgement that the frozen image is on screen.
///
/// Read the module header before trusting the number this closes: the mark is
/// taken HERE, on arrival, which over-counts by the acknowledgement's own trip
/// and under-counts by one compositor frame.
///
/// **Veil window only**, by capability AND in Rust ([`ipc`] explains why both
/// are kept). A `veil_painted` from `main` would close a run that was never
/// painted and file a fabricated latency into the median this whole lot is
/// about. Refused with a printed line rather than a `Result`, and the reason is
/// where the refusal would LAND: the page does catch it
/// (`src/veil/main.ts:415`), but only into `console.error` - and the veil window
/// holds no `core:webview` permission, so its devtools cannot be opened. A
/// terminal line is the only one a human will ever read, which is also why the
/// ACL's own refusal, which goes back to the page, would not be enough alone.
#[tauri::command]
pub fn veil_painted(app: AppHandle, webview: Webview, run: u64) {
    if let Err(refused) = ipc::ensure_from(webview.label(), VEIL_WINDOW_LABEL, "veil_painted") {
        eprintln!("[cliche] veil: {refused}");
        return;
    }

    let Some(timings) = app.try_state::<Timings>() else {
        return;
    };
    timings.mark(MARK_PAINTED);
    timings.finish_run();

    let Some(veil) = app.try_state::<Veil>() else {
        return;
    };
    let painted = veil
        .painted
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);
    println!("[cliche] veil: run {run} painted ({painted} measured)");

    if crate::shortcut::report_due(painted) {
        println!("[cliche] veil: transport {}", veil.transport().describe());
        for line in timings.report().lines() {
            println!("[cliche] {line}");
        }
    }
}

/// Cuts the user's selection out of the frozen frame.
///
/// The page sends the two corners of its drag, in CSS pixels, exactly as it
/// measured them - it applies no scale of its own. Turning them into physical
/// pixels is `geometry::to_physical` and nothing else; that is the only place
/// in this application where a scale factor is multiplied, and the only place
/// where the rounding rule is decided.
///
/// **The scale comes from the WINDOW, not from the monitor.** These coordinates
/// were measured by this webview, and Tauri reports a scale factor per window;
/// on a mixed-DPI desktop, or if the veil ever failed to be sized to the
/// primary monitor at startup, the two numbers can differ and the window's is
/// the one that describes these coordinates.
///
/// # What happens to the cut - 1f
///
/// It is measured, its size is reported, and then it goes on the system
/// clipboard. The printed selection line is still the proof of the chain -
/// a rectangle drawn in CSS pixels becomes an exact rectangle of the capture's
/// own bytes - and `clipboard::success_line` is the proof it left the process.
///
/// **A click is refused before anything is cut.** The area rule lives in
/// `clipboard::is_worth_copying`, with the reasoning behind its threshold; it is
/// applied here, before the crop, so a mis-click consumes no frame, closes no
/// veil and never reaches the clipboard. That is `docs/PRD.md` case 6.
///
/// **Everything the clipboard costs is OUTSIDE the 150 ms budget.** That budget
/// ends at `painted`, before the user has even started dragging. See
/// `clipboard::Meter` for the instrument that keeps those figures apart.
///
/// Returns `Result` so that a refusal reaches the page's `catch` instead of
/// vanishing: a selection that was rejected must not look like one that
/// succeeded.
/// **Veil window only, and this is the most important of the four guards.**
/// `main` runs React and its dependency tree, and one compromised package there
/// could call this with a full-screen rectangle and put the frozen screen on the
/// system clipboard without the user drawing anything at all. Two things refuse
/// that now: `capabilities/veil.json` grants `veil_selected` to the veil window
/// alone, so Tauri's ACL rejects the call before this function is entered, and
/// the check below rejects it again with a line naming both windows. Nothing
/// enforced it before 4 September 2026 - with no application manifest, the ACL
/// simply did not look at this crate's commands; `ipc.rs` has that reading of
/// `webview/mod.rs:1823` and the reasons the Rust check is kept alongside.
#[tauri::command]
pub fn veil_selected(
    app: AppHandle,
    webview: Webview,
    run: u64,
    x0: f64,
    y0: f64,
    x1: f64,
    y1: f64,
) -> Result<(), String> {
    // FIRST, before the frame is read, before anything is cut, before the
    // clipboard is touched.
    ipc::ensure_from(webview.label(), VEIL_WINDOW_LABEL, "veil_selected")?;

    // `try_state`, never `state`, for the reason `perform_capture` gives: this
    // runs on a webview IPC thread and a panic there ends the application.
    let veil = app
        .try_state::<Veil>()
        .ok_or_else(|| "no veil state is managed; the selection cannot be cut".to_owned())?;
    let window = app
        .get_webview_window(VEIL_WINDOW_LABEL)
        .ok_or_else(|| "the veil window does not exist".to_owned())?;

    let scale = window
        .scale_factor()
        .map_err(|error| format!("could not read the veil window's scale factor: {error}"))?;

    let rectangle = geometry::to_physical(CssRect::from_corners(x0, y0, x1, y1)?, scale)?;

    // A click is not a drag. Refused HERE, before the frame is cut and before
    // the frame is released, so that a mis-click leaves everything as it was:
    // no crop, no clipboard write, and the veil still up to drag again. Doing it
    // after the cut would work too, but it would have thrown the frozen frame
    // away - and then the retry would have nothing to cut.
    if !clipboard::is_worth_copying(rectangle.width(), rectangle.height()) {
        return Err(clipboard::too_small_line(
            rectangle.width(),
            rectangle.height(),
        ));
    }

    // Scoped, so the lock is released before anything is printed or any window
    // is touched. Holding it across a call into Tauri would be a deadlock
    // waiting for the day the two threads meet.
    let cut = {
        // Not `mut` any more: this guard is only read now. The frame used to be
        // emptied here, and is emptied after the copy instead.
        let held = veil.frame();

        let cut = {
            let (staged, frame) = held.as_ref().ok_or_else(|| {
                "no frozen frame is being shown; there is nothing to cut".to_owned()
            })?;

            // A selection drawn on run 3 must never be cut out of run 4's
            // image. Without this the user would get a rectangle of the right
            // shape taken from the wrong screen - the one failure that produces
            // a plausible file and no error at all.
            if *staged != run {
                return Err(format!(
                    "the selection belongs to run {run}, but run {staged} is the frame on \
                     screen; it was not cut"
                ));
            }

            capture::crop(frame, rectangle)?
        };

        // The frozen frame is NOT released here, and the reason arrived with
        // the fix below. Making the clipboard refusal visible is worth nothing
        // if the user cannot act on it: dropping the frame now would leave them
        // looking at an error over a veil that has nothing left to cut, so the
        // retry it invites would fail with a different message. It is released
        // after the copy succeeds - see the end of this function.
        cut
    };

    println!(
        "[cliche] veil: run {run} selection {width}x{height} physical px at ({x}, {y}) - \
         {bytes} byte(s), from a CSS rectangle at scale {scale:.2}",
        width = cut.width(),
        height = cut.height(),
        x = rectangle.x(),
        y = rectangle.y(),
        bytes = cut.pixels().len(),
    );

    // THE COPY COMES FIRST, AND THE VEIL IS HIDDEN ONLY ONCE IT SUCCEEDS.
    //
    // It used to be the other way round, with this reasoning: "should it fail,
    // the user is left with their screen back rather than with a frozen overlay
    // they have to press Escape to be rid of". That reasoning was wrong, and the
    // review bot on PR #5 found why. The refusal travels back to the page's
    // `catch`, which paints it in the veil - so hiding first wrote the error
    // message into a window nobody could see. The user got their desktop back,
    // no image on the clipboard, and NO MESSAGE: a silent failure on the one
    // action this product exists to perform.
    //
    // Hiding afterwards costs the ~10 ms the copy takes, all of it outside the
    // 150 ms budget, which ends at `painted` and long before the user's drag.
    // And it makes this path agree with the too-small refusal, which already
    // leaves the veil up so the selection can be corrected (see the area check
    // above, refused before the crop).
    //
    // `?`: a refusal must reach the page's `catch`. A capture that did not make
    // it to the clipboard has failed, and it must not look like one that worked.
    let copied = clipboard::copy_selection(&app, &cut)?;

    // The capture really is over now, so the frozen frame goes. Holding 8.29 MB
    // for a window nobody is looking at is a cost with no purpose in a process
    // that stays open for days - but it is only pointless ONCE the copy has
    // worked. Until then it is what a retry needs.
    *veil.frame() = None;

    if let Err(error) = window.hide() {
        eprintln!("[cliche] veil: could not hide the veil after the selection: {error}");
    }

    println!(
        "{}",
        clipboard::success_line(
            run,
            cut.width(),
            cut.height(),
            cut.pixels().len(),
            copied.elapsed,
        )
    );

    // Same batch rule as the paint report, so a measuring session reads the same
    // way. The header line of this one says why its total is NOT comparable to
    // the one 1d prints.
    if crate::shortcut::report_due(copied.copies) {
        if let Some(meter) = app.try_state::<clipboard::Meter>() {
            for line in meter.report_lines() {
                println!("[cliche] {line}");
            }
        }
    }

    // `cut` is dropped here, its bytes now owned by the clipboard.
    Ok(())
}

/// Escape: close the veil and throw the run away.
///
/// `abandon_run` and not `finish_run`. A cancelled capture has no latency to
/// report, and filing it would let a failed run count as a successful one -
/// the exact way a median gets flattered.
///
/// **Veil window only.** Checked BEFORE the run is abandoned: `main` calling
/// this in a loop would cancel every capture the user starts, and Cliche would
/// simply appear broken. Same choice as `veil_painted` on the refusal - a
/// printed line rather than a `Result`. The page catches this one too
/// (`src/veil/main.ts:712`), into a console nobody can open while a veil covers
/// the screen.
#[tauri::command]
pub fn veil_dismissed(app: AppHandle, webview: Webview) {
    if let Err(refused) = ipc::ensure_from(webview.label(), VEIL_WINDOW_LABEL, "veil_dismissed") {
        eprintln!("[cliche] veil: {refused}");
        return;
    }

    if let Some(timings) = app.try_state::<Timings>() {
        timings.abandon_run();
    }
    if let Some(veil) = app.try_state::<Veil>() {
        // Released, not kept: a cancelled capture has nothing left to cut, and
        // 8.29 MB held for a hidden window is a cost with no purpose. It also
        // means a selection arriving after Escape finds nothing and says so,
        // rather than cutting the screen the user just dismissed.
        //
        // `current_run` because this command carries no run number: the page
        // sends none, so "the capture on screen" is the only thing Escape can
        // mean here. Every OTHER caller of `release` names its own run - see
        // the method for why that difference matters.
        veil.release(veil.current_run());
    }
    if let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) {
        if let Err(error) = window.hide() {
            eprintln!("[cliche] veil: could not hide the veil: {error}");
        }
    }
    println!("[cliche] veil: dismissed");
}

/// How long a benchmark run waits for its acknowledgement before giving up.
const BENCH_RUN_TIMEOUT: Duration = Duration::from_secs(3);

/// How long the veil stays hidden between benchmark runs.
///
/// Not cosmetic. Without it the window is already visible when the next run
/// starts, `show()` becomes a no-op, and `shown` measures nothing - a fake
/// figure that would look like an excellent result.
const BENCH_SETTLE: Duration = Duration::from_millis(400);

/// How long the benchmark waits after startup before its first run, so the
/// veil's document has certainly finished loading. A first run measured
/// against a page that is still parsing would be a slow outlier with no
/// meaning.
const BENCH_WARMUP: Duration = Duration::from_secs(3);

/// Starts N automated runs when [`BENCH_ENV`] asks for them.
///
/// # Why this exists, and how it differs from a real key press
///
/// Injecting keystrokes into the user's session is not allowed here, and it
/// would buy nothing: `t0` is the ENTRY OF THE HANDLER, and everything before
/// it (key, hook, hotkey thread) is already outside every figure the instrument
/// prints. So a programmatic call to [`perform_capture`] measures exactly the
/// same interval as a real press.
///
/// The differences that DO exist, stated rather than glossed over:
///
/// - **Thread.** A real press runs on the global-hotkey plugin's thread; this
///   runs on a thread spawned here. Both are non-main threads that reach the
///   main thread through the same Tauri dispatch, so `show()` pays the same
///   crossing - but they are not the same thread.
/// - **Machine state.** A benchmark run is preceded by [`BENCH_SETTLE`] of
///   quiet; a real press is preceded by whatever the user was doing. The
///   benchmark therefore measures a machine that is calmer than reality.
/// - **Cadence.** Runs follow each other every few hundred milliseconds. Page
///   and allocator caches are warmer than they would be on a press once an hour.
///
/// All three flatter the result. Treat the benchmark as a floor and the
/// keyboard as the arbiter.
pub fn spawn_bench(app: &AppHandle, runs: usize) {
    let app = app.clone();

    std::thread::spawn(move || {
        println!("[cliche] bench: {runs} run(s) requested; warming up for {BENCH_WARMUP:?}");
        std::thread::sleep(BENCH_WARMUP);

        for run in 1..=runs {
            if let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) {
                if let Err(error) = window.hide() {
                    eprintln!("[cliche] bench: could not hide the veil: {error}");
                }
            }
            std::thread::sleep(BENCH_SETTLE);

            let Some(veil) = app.try_state::<Veil>() else {
                eprintln!("[cliche] bench: no veil state; stopping");
                return;
            };
            let before = veil.painted.load(Ordering::Relaxed);

            perform_capture(&app);

            // Polling, not a condition variable: the poller must not be able to
            // influence the measurement, and `painted` is timestamped in the
            // command handler, never here.
            // `elapsed()` against a start, not `Instant::now() + timeout`:
            // adding a `Duration` to an `Instant` panics on overflow, and
            // nothing on any thread of this application may panic.
            let waiting_since = Instant::now();
            while veil.painted.load(Ordering::Relaxed) == before {
                if waiting_since.elapsed() >= BENCH_RUN_TIMEOUT {
                    eprintln!(
                        "[cliche] bench: run {run} of {runs} never acknowledged within \
                         {BENCH_RUN_TIMEOUT:?}; abandoning it"
                    );
                    if let Some(timings) = app.try_state::<Timings>() {
                        timings.abandon_run();
                    }
                    break;
                }
                std::thread::sleep(Duration::from_millis(2));
            }
        }

        if let Some(window) = app.get_webview_window(VEIL_WINDOW_LABEL) {
            let _ = window.hide();
        }

        println!("[cliche] bench: finished");
        if let (Some(timings), Some(veil)) = (app.try_state::<Timings>(), app.try_state::<Veil>()) {
            println!("[cliche] bench: transport {}", veil.transport().describe());
            for line in timings.report().lines() {
                println!("[cliche] {line}");
            }
        }
    });
}

/// Reads [`BENCH_ENV`]. Pure, so the parsing is under test.
///
/// A value that is not a positive number is reported rather than ignored: a
/// typo that silently ran zero benchmark runs would look exactly like an
/// application that simply did not start the benchmark.
pub fn parse_bench(raw: Option<&str>) -> (Option<usize>, Option<String>) {
    let Some(raw) = raw else {
        return (None, None);
    };

    match raw.trim().parse::<usize>() {
        Ok(0) => (
            None,
            Some(format!(
                "[cliche] bench: {BENCH_ENV}=0 asks for no run; nothing scheduled"
            )),
        ),
        Ok(runs) => (Some(runs), None),
        Err(error) => (
            None,
            Some(format!(
                "[cliche] bench: {BENCH_ENV}=\"{raw}\" is not a run count ({error}); \
                 NO benchmark was scheduled"
            )),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Whether a capability file declares a window, read literally.
    ///
    /// NOT a JSON parser. This crate does not depend on `serde_json` and lot 2
    /// is not the place to add a dependency for four lines. It finds the
    /// `"windows"` key, takes the array behind it and compares each entry with
    /// the quoted label.
    ///
    /// Its blind spots all fail in the SAFE direction: a shape it cannot read
    /// makes it answer "not declared", which turns the guard below red rather
    /// than letting it pass quietly. The fixtures test pins that.
    fn declares_window(capability: &str, label: &str) -> bool {
        let Some((_, after_key)) = capability.split_once("\"windows\"") else {
            return false;
        };
        let Some(open) = after_key.find('[') else {
            return false;
        };
        let Some(close) = after_key[open..].find(']') else {
            return false;
        };

        let quoted = format!("\"{label}\"");
        after_key[open + 1..open + close]
            .split(',')
            .any(|entry| entry.trim() == quoted)
    }

    /// The verdict itself, kept away from the filesystem so that BOTH its rows
    /// can be tested.
    ///
    /// The real tree only ever exercises ONE of the four combinations at a
    /// time: today `(true, true)`, and before 4 September 2026 `(false, false)`.
    /// The other three would each need a tree that does not exist, so the
    /// reading of the tree and the rule are two functions, and the rule is
    /// exercised on all four.
    fn acl_would_lock_the_veil_out(has_permissions_dir: bool, veil_is_listed: bool) -> bool {
        has_permissions_dir && !veil_is_listed
    }

    #[test]
    fn the_acl_rule_fires_on_exactly_one_of_its_four_combinations() {
        // How this tree stood until 4 September 2026: no manifest, and no
        // capability naming the veil. Harmless, because no check ran.
        assert!(!acl_would_lock_the_veil_out(false, false));
        assert!(!acl_would_lock_the_veil_out(false, true));
        // Today's row: the manifest is armed AND the veil has its capability.
        assert!(!acl_would_lock_the_veil_out(true, true));
        // The bomb: a manifest exists and the veil is in no capability.
        assert!(
            acl_would_lock_the_veil_out(true, false),
            "this is the one case the build must go red on"
        );
    }

    #[test]
    fn the_capability_reader_recognises_a_window_and_refuses_a_near_miss() {
        // Without this, the guard below could be green because the reader
        // cannot see anything at all.
        let listing = r#"{ "windows": ["main", "veil"], "permissions": [] }"#;
        assert!(declares_window(listing, "main"));
        assert!(declares_window(listing, VEIL_WINDOW_LABEL));

        let only_main = r#"{ "windows": ["main"], "permissions": ["core:default"] }"#;
        assert!(declares_window(only_main, "main"));
        assert!(
            !declares_window(only_main, VEIL_WINDOW_LABEL),
            "this is the shape of default.json, which names `main` and only `main`; reading \
             `veil` into it would disarm the guard"
        );

        // A description mentioning the veil is not a declaration.
        let prose = r#"{ "description": "not for the veil window", "windows": ["main"] }"#;
        assert!(!declares_window(prose, VEIL_WINDOW_LABEL));

        // Near misses that a `contains` would wave through.
        let impostors = r#"{ "windows": ["veil2", "veil-decoy", "Veil"] }"#;
        assert!(!declares_window(impostors, VEIL_WINDOW_LABEL));

        // Shapes it cannot read must answer "not declared".
        assert!(!declares_window("{}", VEIL_WINDOW_LABEL));
        assert!(!declares_window(
            r#"{ "windows": "veil" }"#,
            VEIL_WINDOW_LABEL
        ));
    }

    #[test]
    fn the_veil_window_is_capable_the_day_this_application_gets_an_acl_manifest() {
        // THE DAY IS 4 SEPTEMBER 2026. This test was written against a bomb
        // with a long fuse and now guards the state that defused it.
        //
        // `src-tauri/permissions/` exists, so tauri-build generates an
        // application ACL manifest and the ACL check runs on this crate's own
        // commands (`ipc.rs` has the reading of `webview/mod.rs:1823`). The
        // veil window therefore HAS to be named in a capability: were it not,
        // every `veil_*` call would be refused at once, for a reason nothing in
        // the code would name. `capabilities/veil.json` is what names it, and
        // this test is what keeps the two facts from drifting apart - a
        // capability file deleted or renamed turns it red.
        //
        // The name is left as it was written, in the future tense, so that the
        // fuse and the day it burned out stay one story in the history.
        let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let capabilities = crate_root.join("capabilities");

        let mut files = 0;
        let mut listed_main = false;
        let mut listed_veil = false;

        for entry in fs::read_dir(&capabilities)
            .unwrap_or_else(|error| panic!("{} must be readable: {error}", capabilities.display()))
        {
            let path = entry.expect("a directory entry must be readable").path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("{} must be readable: {error}", path.display()));
            files += 1;
            listed_main |= declares_window(&text, crate::ipc::MAIN_WINDOW_LABEL);
            listed_veil |= declares_window(&text, VEIL_WINDOW_LABEL);
        }

        // The two checks that stop this test from passing for the wrong reason:
        // a wrong path, or a reader that sees nothing in a real file.
        assert!(
            files > 0,
            "no capability file was found in {}; this test would then pass whatever the \
             application declares",
            capabilities.display()
        );
        assert!(
            listed_main,
            "`main` is declared in capabilities/default.json and the scan did not find it, so \
             the scan - not the application - is what is broken"
        );

        assert!(
            !acl_would_lock_the_veil_out(crate_root.join("permissions").is_dir(), listed_veil),
            "src-tauri/permissions/ now exists, so tauri-build generates an application ACL \
             manifest and the ACL check starts running on this crate's OWN commands (ipc.rs \
             quotes the condition). The `{VEIL_WINDOW_LABEL}` window is listed in no capability \
             file under {}, so veil_painted, veil_selected and veil_dismissed would all be \
             refused and captures would stop working. Add `{VEIL_WINDOW_LABEL}` to the \
             `windows` array of a capability granting what the veil needs.",
            capabilities.display()
        );
    }

    #[test]
    fn the_five_labels_are_the_ones_the_report_has_to_show_in_order() {
        // The report lists steps in the order they were first marked, so these
        // five names ARE the comparison between the two transports. A rename
        // would produce two reports that cannot be held side by side.
        //
        // FIVE since 4 September 2026, and this is a change of pipeline rather
        // than a relaxed assertion. `decoded` is a step that did not exist, and
        // neither `shown` nor `painted` still covers what it used to: the window
        // is now made visible BETWEEN them. A report printed before that day and
        // one printed after are therefore NOT comparable step by step - only
        // their TOTAL is, and keeping the two ends of that total where they were
        // is the whole point of the contract in the module header.
        let pipeline = [
            MARK_CAPTURE,
            MARK_TRANSPORT,
            MARK_DECODED,
            MARK_SHOWN,
            MARK_PAINTED,
        ];

        assert_eq!(
            pipeline,
            ["capture", "transport", "decoded", "shown", "painted"]
        );

        // `Timings::mark` drops a duplicate label and counts it as ignored, so
        // any collision would silently lose a step.
        for (index, label) in pipeline.iter().enumerate() {
            assert!(
                !pipeline[index + 1..].contains(label),
                "`{label}` appears twice in the pipeline"
            );
        }
    }

    #[test]
    fn the_fallback_outlasts_a_slow_decode_and_still_fires_before_the_bench_gives_up() {
        // PROVENANCE OF THE FIGURE IT IS HELD AGAINST. `DECODE_P95_MEASURED` is
        // the `painted` p95 of the session Thierry ran on 4 September 2026
        // against commit 868ba0d: 18 clean runs, median 91.3 ms, p95 94.3 ms.
        // Under THAT pipeline `painted` covered fetch + decode + one rAF + the
        // acknowledgement's own trip, which is within one animation frame of
        // what `decoded` covers now - so it is the best measurement available
        // for how long a HEALTHY run may take before it starts to look dead.
        //
        // Twice it, and not once: 94.3 ms is a p95, not a maximum. A fallback
        // firing on the tail of a healthy distribution would show the veil early
        // AND call `abandon_run`, trading a rare flash for a pipeline that
        // routinely refuses to be measured.
        assert!(
            SHOW_FALLBACK > DECODE_P95_MEASURED * 2,
            "{SHOW_FALLBACK:?} is not more than twice the measured decode p95 \
             ({DECODE_P95_MEASURED:?}): healthy runs would be shown by the fallback and \
             then discarded, and the report would blame the pipeline"
        );

        // And under the benchmark's own patience, or the bench would abandon
        // every unacknowledged run before the fallback ever repaired one - the
        // report would show the discard and never the repair.
        assert!(
            SHOW_FALLBACK < BENCH_RUN_TIMEOUT,
            "a fallback the benchmark outlives is a fallback the benchmark hides"
        );
    }

    #[test]
    fn the_show_rule_is_a_pure_decision_over_three_numbers() {
        // Kept apart from the atomics for the reason `ipc::is_from` is kept
        // apart from `Webview`: a rule that needs no event loop can be put to
        // every row of its own table.
        //                    current, already shown, the run asking
        assert!(may_show(4, 3, 4), "the current run, not yet shown");
        assert!(!may_show(4, 4, 4), "this run has already been shown");
        assert!(!may_show(4, 5, 4), "a newer run is already on screen");
        assert!(
            !may_show(4, 3, 3),
            "stale: run 3 acknowledged while run 4 is in flight"
        );
        assert!(!may_show(4, 3, 5), "a run that was never started");
        assert!(
            !may_show(0, 0, 0),
            "before the first capture there is nothing to show"
        );
    }

    #[test]
    fn exactly_one_of_the_two_paths_may_show_a_run_and_a_stale_one_may_not() {
        // PROPERTY D, the risk this change carries. Since 4 September 2026 TWO
        // paths reach `show()` - the page's `veil_decoded` and the fallback
        // timer - and they race by design. A `Veil` is a plain value, so the
        // claim can be raced here with no window and no event loop.
        let veil = Veil::new(Transport::CustomProtocolBmp);
        let run = veil.next_run();

        assert!(veil.claim_show(run), "the first arrival shows the veil");
        assert!(
            !veil.claim_show(run),
            "the second arrival must do NOTHING: a second `show()` raises the window \
             again over whatever the user has moved on to"
        );

        assert!(
            !veil.claim_show(run + 1),
            "no such run has been started; an acknowledgement naming it is not one"
        );
        assert!(
            !veil.claim_show(0),
            "0 is what the page holds when nothing is on screen"
        );

        // A stale acknowledgement, arriving after the next shortcut press.
        let next = veil.next_run();
        assert!(
            !veil.claim_show(run),
            "run {run} is over; its late acknowledgement must not wake the window"
        );
        assert!(veil.claim_show(next), "the run in flight is still showable");
    }

    #[test]
    fn releasing_a_capture_stops_a_fallback_from_raising_the_veil_afterwards() {
        // Escape at 200 ms; the fallback armed by `perform_capture` is still
        // counting down. Without this line in `Veil::release`, the veil the user
        // just dismissed comes back up 50 ms later, over whatever they turned
        // to - and `veil_dismissed` has already dropped the frame, so it comes
        // back EMPTY.
        //
        // The page cannot ordinarily receive Escape while the window is hidden,
        // so this is a guard against a race rather than a repair of an observed
        // bug. It costs one atomic on a path that ends a capture.
        let veil = Veil::new(Transport::CustomProtocolBmp);
        let run = veil.next_run();

        veil.release(run);

        assert!(
            !veil.claim_show(run),
            "a released capture must be showable by neither path"
        );
    }

    #[test]
    fn closing_one_run_never_burns_the_claim_of_the_run_that_replaced_it() {
        // THE defect the review of 4 September 2026 found, and the reason
        // `release` takes a run instead of reading `generation`.
        //
        // The sequence, and it is only microseconds wide: a caller claims the
        // show for run 1, the user presses the shortcut again - `generation`
        // becomes 2 - and only then does that caller fail and release. Reading
        // `generation` at THAT moment burnt run 2's claim before run 2 had ever
        // been shown, and neither its acknowledgement nor its fallback could
        // raise the veil again. The user would press the shortcut and see
        // nothing at all, for ever, which is precisely the failure the whole
        // reorder exists to make impossible.
        let veil = Veil::new(Transport::CustomProtocolBmp);

        let first = veil.next_run();
        assert!(veil.claim_show(first), "run 1 must be claimable");

        let second = veil.next_run();

        // The late caller of run 1 gives up, naming ITS OWN run.
        veil.release(first);

        assert!(
            veil.claim_show(second),
            "run {second} was never shown, and closing run {first} must not have taken its \
             claim: the shortcut would do nothing from here on"
        );
    }

    #[test]
    fn a_fresh_veil_holds_no_frame_so_a_selection_before_a_capture_is_refused() {
        // `veil_selected` answers "there is nothing to cut" from this being
        // `None`. A `Veil` that started with a frame - or kept one across a
        // dismissal - would cut a screen the user is no longer looking at.
        let veil = Veil::new(Transport::CustomProtocolBmp);

        assert!(veil.frame().is_none());
    }

    #[test]
    fn ending_a_capture_releases_the_staged_payload_and_not_only_the_frame() {
        // The scenario: shortcut pressed over a password manager, the page has
        // not fetched the image yet, Escape. `pending` still holds the whole
        // 8.29 MB screen, in a process that stays open for days - reachable
        // from a minidump, the page file, or any process of the same session.
        let veil = Veil::new(Transport::CustomProtocolBmp);
        *veil.pending() = Some((1, vec![0xAB; 32]));
        *veil.frame() = None;

        veil.release(1);

        assert!(
            veil.pending().is_none(),
            "the staged screen must not outlive the capture that staged it"
        );
        assert!(veil.frame().is_none());
    }

    #[test]
    fn the_default_transport_is_the_one_that_does_not_encode() {
        assert_eq!(Transport::parse(None).0, Transport::CustomProtocolBmp);
        assert_eq!(Transport::parse(None).1, None, "a default is not a warning");
    }

    #[test]
    fn both_transports_can_be_selected_by_name_whatever_the_case() {
        assert_eq!(
            Transport::parse(Some("bmp")).0,
            Transport::CustomProtocolBmp
        );
        assert_eq!(Transport::parse(Some("png")).0, Transport::DataUrlPng);
        assert_eq!(Transport::parse(Some("  PNG  ")).0, Transport::DataUrlPng);
        assert_eq!(
            Transport::parse(Some("Bmp")).0,
            Transport::CustomProtocolBmp
        );
    }

    #[test]
    fn an_unknown_transport_falls_back_but_says_so_loudly() {
        // The failure this guards: measuring A while believing you measured B.
        let (transport, warning) = Transport::parse(Some("data-url"));

        assert_eq!(transport, Transport::CustomProtocolBmp);
        let warning = warning.expect("an unknown value must produce a warning");
        assert!(
            warning.contains("data-url"),
            "the bad value must be quoted: {warning}"
        );
        assert!(
            warning.contains("bmp") && warning.contains("png"),
            "{warning}"
        );
        assert!(
            warning.contains("FALLING BACK"),
            "the message must say the measurement is not the one asked for: {warning}"
        );
    }

    #[test]
    fn a_frame_path_yields_its_run_number_and_nothing_else_does() {
        assert_eq!(parse_frame_path("/frame/1.bmp"), Some(1));
        assert_eq!(parse_frame_path("/frame/4096.bmp"), Some(4096));

        // Anything unexpected must be refused rather than answered with
        // whatever happens to be staged.
        assert_eq!(parse_frame_path("/frame/1.png"), None);
        assert_eq!(parse_frame_path("/frame/.bmp"), None);
        assert_eq!(parse_frame_path("/frame/-1.bmp"), None);
        assert_eq!(parse_frame_path("/"), None);
        assert_eq!(parse_frame_path("/../frame/1.bmp"), None);
    }

    #[test]
    fn the_windows_origin_matches_what_the_csp_has_to_allow() {
        // This string and the `img-src` entry in tauri.conf.json are the same
        // fact written twice; if this test is edited, that file must be too.
        assert_eq!(VEIL_ORIGIN, "http://cliche.localhost");
        assert!(
            VEIL_ORIGIN.ends_with(&format!("{VEIL_SCHEME}.localhost")),
            "Tauri serves a custom scheme at http://<scheme>.localhost on Windows"
        );
    }

    #[test]
    fn the_csp_allows_the_veil_origin_and_no_protocol_this_build_does_not_serve() {
        // Reads the real configuration rather than a copy of it: a test that
        // asserted against a string literal here would keep passing after
        // somebody edited the file.
        const CONFIG: &str = include_str!("../tauri.conf.json");

        assert!(
            CONFIG.matches(VEIL_ORIGIN).count() == 2,
            "both `csp` and `devCsp` have to allow {VEIL_ORIGIN}"
        );
        assert!(
            !CONFIG.contains("asset"),
            "`asset:` / `http://asset.localhost` allow a source for a protocol \
             this build does not serve: no `assetProtocol` in the config, \
             `tauri` built with `features = []`, no `convertFileSrc` in the \
             repository. Removed 4 September 2026 - if the asset protocol is \
             ever genuinely enabled, this assertion is the place to say so."
        );
    }

    #[test]
    fn base64_matches_the_worked_examples_from_rfc_4648() {
        // Hand-computable vectors, including both padding lengths - the two
        // cases a naive encoder gets wrong.
        let cases = [
            (&b""[..], ""),
            (&b"f"[..], "Zg=="),
            (&b"fo"[..], "Zm8="),
            (&b"foo"[..], "Zm9v"),
            (&b"foob"[..], "Zm9vYg=="),
            (&b"fooba"[..], "Zm9vYmE="),
            (&b"foobar"[..], "Zm9vYmFy"),
        ];

        for (input, expected) in cases {
            let mut encoded = String::new();
            push_base64(input, &mut encoded);
            assert_eq!(encoded, expected, "encoding {input:?}");
        }
    }

    #[test]
    fn base64_output_can_never_break_out_of_a_javascript_string_literal() {
        // The reason no escaping happens before `eval`: the alphabet makes it
        // impossible. Every byte value is fed in, so the whole output space is
        // covered.
        let every_byte: Vec<u8> = (0..=255u8).collect();
        let mut encoded = String::new();
        push_base64(&every_byte, &mut encoded);

        assert!(
            encoded.bytes().all(|byte| byte.is_ascii_alphanumeric()
                || byte == b'+'
                || byte == b'/'
                || byte == b'='),
            "base64 emitted a character outside its alphabet"
        );
        assert!(!encoded.contains('"') && !encoded.contains('\\') && !encoded.contains('\n'));
        assert!(!DATA_URL_PREFIX.contains('"') && !DATA_URL_PREFIX.contains('\\'));
    }

    #[test]
    fn base64_grows_the_payload_by_a_third_which_is_transport_b_s_extra_cost() {
        // Pins the arithmetic the report has to be read against: a ~1.5 MB PNG
        // becomes ~2 MB of JavaScript source, on top of the 69.6 ms encode.
        let mut encoded = String::new();
        push_base64(&vec![0u8; 3_000], &mut encoded);

        assert_eq!(encoded.len(), 4_000);
    }

    #[test]
    fn no_bench_is_scheduled_unless_a_positive_count_is_asked_for() {
        assert_eq!(parse_bench(None), (None, None));
        assert_eq!(parse_bench(Some("20")).0, Some(20));
        assert_eq!(parse_bench(Some(" 20 ")).0, Some(20));
    }

    #[test]
    fn a_bench_count_that_is_not_a_count_is_reported_not_ignored() {
        let (runs, warning) = parse_bench(Some("twenty"));

        assert_eq!(runs, None);
        let warning = warning.expect("a bad count must be reported");
        assert!(warning.contains("twenty"), "{warning}");
        assert!(
            warning.contains("NO benchmark"),
            "silence here looks exactly like a benchmark that was never asked for: {warning}"
        );

        assert_eq!(parse_bench(Some("0")).0, None);
        assert!(
            parse_bench(Some("0")).1.is_some(),
            "zero runs must be explained"
        );
        assert_eq!(parse_bench(Some("-3")).0, None);
    }

    #[test]
    fn the_benchmark_hides_the_veil_between_runs_for_long_enough_to_mean_something() {
        // If this were zero, `show()` on the next run would be a no-op and
        // `shown` would read ~0 ms - a fabricated success.
        assert!(
            BENCH_SETTLE >= Duration::from_millis(100),
            "too short a settle makes `shown` measure nothing"
        );
        assert!(
            BENCH_RUN_TIMEOUT > BENCH_SETTLE,
            "a run must be allowed more time than the pause before it"
        );
    }
}
