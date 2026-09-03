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
//! window exists, hidden, from the first second of the process, and the
//! shortcut only shows it.
//!
//! # The four steps, and what each one really covers
//!
//! | label | from | to |
//! | --- | --- | --- |
//! | `capture` | handler entry | the RGBA frame is in hand |
//! | `transport` | there | the payload is built and staged |
//! | `shown` | there | the veil window is visible and focused |
//! | `painted` | there | the webview said it drew the image |
//!
//! `transport` deliberately covers EVERYTHING between having the frame and
//! having something the page can load - for transport B that includes the PNG
//! encode and the base64. Splitting them would make the two transports
//! incomparable, which is the only thing this measurement is for.
//!
//! # `painted` is an approximation, and its two errors point OPPOSITE ways
//!
//! The page acknowledges from inside a `requestAnimationFrame` callback taken
//! AFTER `HTMLImageElement.decode()` resolves, and Rust timestamps when the
//! acknowledgement arrives. That number is:
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
//! thread, or on the benchmark thread. A panic on any of them takes the
//! application down, and losing the app to a diagnostic is a bad trade.

use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use tauri::{AppHandle, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

use crate::capture::{self, MARK_CAPTURE};
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
/// This exact string is what has to appear in `img-src` in `tauri.conf.json`.
/// The existing `http://asset.localhost` entry in that same directive is the
/// built-in `asset` scheme following the identical rule.
pub const VEIL_ORIGIN: &str = "http://cliche.localhost";

/// Timing label for building and staging the payload.
pub const MARK_TRANSPORT: &str = "transport";

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
    /// Run number, and the cache-buster in the URL. WebView2 would happily
    /// re-serve a previous response for an identical URL, and a `painted` that
    /// measured a cache hit would be a fabricated 2 ms.
    generation: AtomicU64,
    /// How many runs reached `painted`. The benchmark waits on this, and the
    /// report is printed every twenty.
    painted: AtomicUsize,
}

impl Veil {
    pub fn new(transport: Transport) -> Self {
        Self {
            transport,
            pending: Mutex::new(None),
            generation: AtomicU64::new(0),
            painted: AtomicUsize::new(0),
        }
    }

    pub fn transport(&self) -> Transport {
        self.transport
    }

    /// Takes the lock, recovering a poisoned one. Same reasoning as
    /// `Timings::state`: the guarded value is a byte buffer with no invariant,
    /// and a panic elsewhere must not turn every later capture into a crash.
    fn pending(&self) -> std::sync::MutexGuard<'_, Option<(u64, Vec<u8>)>> {
        self.pending
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
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

    let run = veil
        .generation
        .fetch_add(1, Ordering::Relaxed)
        .saturating_add(1);

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

    // ORDER: show FIRST, hand the image over second.
    //
    // The tempting order is the opposite - let the page start decoding while
    // the window is still hidden, so the two costs overlap. It is not used,
    // because WebView2 throttles (and may stop) `requestAnimationFrame` in a
    // window that is not visible: the acknowledgement would arrive late, or
    // not at all, for a reason that has nothing to do with the transport. A
    // measurement that can silently stall is worse than a slightly serialised
    // one. Revisiting this is 1e's business, once somebody has MEASURED what a
    // hidden WebView2 actually does.
    if let Err(error) = window.show() {
        eprintln!("[cliche] veil: could not show the veil: {error}");
        timings.abandon_run();
        return;
    }
    // Focus, so that the Escape key reaches the veil's own document. An
    // always-on-top window that Windows never activated receives no key events.
    if let Err(error) = window.set_focus() {
        eprintln!("[cliche] veil: could not focus the veil: {error}");
    }
    timings.mark(MARK_SHOWN);

    // `eval` is `ExecuteScriptAsync` on Windows: it returns before the script
    // has run, which is exactly right - the rest of the trip is the page's, and
    // the page is what reports `painted`.
    if let Err(error) = window.eval(format!("window.__clicheShow(\"{source}\",{run})")) {
        eprintln!("[cliche] veil: could not hand the frame to the page: {error}");
        timings.abandon_run();
        let _ = window.hide();
    }
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
pub fn serve(app: &AppHandle, path: &str) -> tauri::http::Response<Vec<u8>> {
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

/// The page's acknowledgement that the frozen image is on screen.
///
/// Read the module header before trusting the number this closes: the mark is
/// taken HERE, on arrival, which over-counts by the acknowledgement's own trip
/// and under-counts by one compositor frame.
#[tauri::command]
pub fn veil_painted(app: AppHandle, run: u64) {
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

/// Escape: close the veil and throw the run away.
///
/// `abandon_run` and not `finish_run`. A cancelled capture has no latency to
/// report, and filing it would let a failed run count as a successful one -
/// the exact way a median gets flattered.
#[tauri::command]
pub fn veil_dismissed(app: AppHandle) {
    if let Some(timings) = app.try_state::<Timings>() {
        timings.abandon_run();
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

    #[test]
    fn the_four_labels_are_the_ones_the_report_has_to_show_in_order() {
        // The report lists steps in the order they were first marked, so these
        // four names ARE the comparison between the two transports. A rename
        // would produce two reports that cannot be held side by side.
        let pipeline = [MARK_CAPTURE, MARK_TRANSPORT, MARK_SHOWN, MARK_PAINTED];

        assert_eq!(pipeline, ["capture", "transport", "shown", "painted"]);

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
