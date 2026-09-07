//! Cliché - local screenshot utility.
//!
//! The application logic lives in this library rather than in `main.rs` so it
//! can be exercised by `cargo test` without starting an event loop.

pub mod capture;
pub mod clipboard;
mod compositor;
mod displays;
pub mod geometry;
pub mod ipc;
mod launch;
mod lifecycle;
mod settings;
mod shortcut;
mod shortcuts;
pub mod timing;
pub mod veil;

pub use displays::{collect_displays, describe_displays, summarize, DisplayInfo};
pub use launch::capture_region;
pub use shortcut::{CaptureShortcut, ShortcutChange, ShortcutStatus};
pub use shortcuts::{
    describe_shortcut_status, describe_shortcuts, set_capture_shortcut, Combination,
    ShortcutCategory, ShortcutEntry, ShortcutRow, REGISTRY,
};

use displays::print_displays;
use tauri::{Manager, WindowEvent};
use timing::Timings;

/// Builds and runs the application. Returns only when the app exits.
pub fn run() {
    let result = tauri::Builder::default()
        // FIRST, and that is a requirement rather than a preference: "The Single
        // Instance plugin must be the first one to be registered to work well.
        // This assures that it runs before other plugins can interfere"
        // (v2.tauri.app/plugin/single-instance, read on 7 September 2026 - the
        // documentation page, NOT the plugin's own source, which could not be
        // opened here). `lifecycle.rs` keeps it in this position.
        //
        // WHAT IT BUYS is the second half of the ghost process of 7 September
        // 2026. A user whose first Cliche is invisible does the one thing left
        // to them - launch it again - and until today the second process went
        // and asked Windows for Ctrl + Maj + 2, which the first one was still
        // holding. The red banner it then showed was true and useless: the other
        // application WAS Cliche. Now the second process hands the screen back
        // and stops.
        //
        // The callback runs in the FIRST process. Nothing of this plugin is
        // reachable from a webview - it declares no command and ships no
        // JavaScript API - so no capability grants it anything and
        // `src-tauri/permissions/` gains no file; `Cargo.toml` has the reading
        // that settles it, and `ipc.rs`'s sentinel finds nothing new in this
        // crate's own handler list.
        .plugin(tauri_plugin_single_instance::init(
            |app, _arguments, _directory| {
                // The RULE is `lifecycle::raise_main_window`, deliberately not
                // this closure: a callback that needs a second process to exist
                // can never be run by `cargo test`.
                lifecycle::raise_main_window(app);
            },
        ))
        // CLOSING THE MAIN WINDOW ENDS CLICHE, and until 7 September 2026 no
        // handler was installed here at all - which is how the process came to
        // outlive its own window. `lifecycle.rs` holds the reading of
        // `tauri-runtime-wry` that explains why nothing else was ever going to
        // request an exit, and both halves of the rule are under test there:
        // `main` ends it, `veil` does not.
        //
        // `exit(0)` and not `std::process::exit`: it goes through
        // `RunEvent::ExitRequested` and `RunEvent::Exit` (the documented
        // behaviour of `AppHandle::exit`, read on docs.rs/tauri/2.11.5 on
        // 7 September 2026). A raw process exit would skip both; what the
        // loaded plugins do in them has not been read here, and skipping a
        // teardown to save nothing is not a trade worth making when the resource
        // at stake is the global hotkey this whole fix is about.
        //
        // The event is not prevented and the veil is not touched: it is a window
        // of this process, and it goes when the process goes.
        .on_window_event(|window, event| {
            if matches!(event, WindowEvent::CloseRequested { .. })
                && lifecycle::closing_ends_the_application(window.label())
            {
                window.app_handle().exit(0);
            }
        })
        // Registered so that `clipboard::copy_selection` finds a `Clipboard` in
        // managed state. This adds NO capability: the plugin's commands DO go
        // through the ACL (`plugin_command.is_some()` in
        // `webview/mod.rs:1823`), no capability grants
        // `clipboard-manager:allow-write-image`, and nothing here reaches the
        // plugin through the webview anyway - see the header of `clipboard.rs`.
        //
        // Since 4 September 2026 the commands below are checked too, by the
        // same `if`: this application declares its own ACL manifest, so each of
        // them is granted by name to ONE window in `capabilities/`. `ipc.rs`
        // has the full reading, and the reasons the Rust guard stays alongside.
        .plugin(tauri_plugin_clipboard_manager::init())
        // The frozen frame is served from MEMORY on this scheme; nothing is
        // written to disk and nothing leaves the process. On Windows the
        // webview reaches it at `http://cliche.localhost/frame/<n>.bmp`, which
        // is the origin `img-src` has to allow in `tauri.conf.json`.
        //
        // The handler only ever hands back the ONE buffer the current run
        // staged, for the exact run number in the path, and takes it as it
        // does so. Any other path gets a 404: this scheme is not a file server.
        //
        // A scheme registered here is served to EVERY webview of the process,
        // `main` included - so the calling webview's label is passed on and
        // `serve` refuses anything that is not the veil. Without it `main`
        // could fetch `/frame/<n>.bmp` in a loop: `serve` TAKES the buffer, so
        // the veil would get a 404, never acknowledge, and Cliche would stop
        // capturing with no message at all.
        //
        // The ACL does NOT help here, and this is the one place where that is
        // still true after the manifest was armed: a URI scheme is not `invoke`,
        // so no capability is ever consulted for it. That label check is the
        // only protection this route has.
        .register_uri_scheme_protocol(veil::VEIL_SCHEME, |ctx, request| {
            veil::serve(ctx.app_handle(), ctx.webview_label(), request.uri().path())
        })
        .invoke_handler(tauri::generate_handler![
            displays::describe_displays,
            shortcuts::describe_shortcuts,
            shortcuts::describe_shortcut_status,
            shortcuts::set_capture_shortcut,
            launch::capture_region,
            veil::veil_ready,
            veil::veil_decoded,
            veil::veil_painted,
            veil::veil_selected,
            veil::veil_confirmed,
            veil::veil_dismissed,
        ])
        .setup(|app| {
            // THE FIRST INSTRUCTION, and the order is the whole of the fix of
            // 7 September 2026.
            //
            // THE WINDOWS ARE ALREADY BUILT WHEN THIS CLOSURE STARTS. Tauri's
            // own `setup` loops over `app.config().app.windows` and builds each
            // one that asks for it, and only then does
            // `if let Some(setup) = app.setup.take() { (setup)(app) }` run
            // (`tauri-2.11.5/src/app.rs:2521-2535`, read in the vendored source
            // on 7 September 2026). So the `main` webview is loading its
            // document, booting React and free to `invoke` while every line
            // below is still running - and the lines below take real time:
            // `collect_displays` enumerates the monitors through xcap, and
            // `veil::create` builds an entire WebView2, "hundreds of
            // milliseconds" by its own comment further down.
            //
            // Until today the capture status was managed on the LAST line of
            // this closure, so `describe_shortcut_status` had nothing to answer
            // with during that whole interval and returned an error. The
            // launcher drew it as a red "the shortcut registry could not be
            // read" - over a shortcut that then worked perfectly, which is how
            // the defect was found. The state exists from here on and says
            // `Starting` until `install` has answered.
            //
            // WHAT IS DELIBERATELY NOT MOVED UP WITH IT: `shortcut::install`.
            // A global hotkey that is live before `Timings`, `MainWindowClaim`,
            // `Veil` and the veil window exist is a press that lands in a
            // half-built application. The STATE moves early; the ANSWER stays
            // where it always was, further down.
            app.manage(shortcut::CaptureShortcut::starting());

            // Logged from the backend, before the webview has had a chance to
            // render. If the window comes up blank - a CSP mistake, a dev
            // server that never started - the terminal still shows whether the
            // Rust side saw the monitors. That distinction is the whole point
            // of logging here as well as in the command.
            match collect_displays(app.handle()) {
                Ok(found) => print_displays("startup", &found),
                Err(error) => eprintln!("[cliche] startup: {error}"),
            }

            // Managed BEFORE the shortcut is bound: the handler looks the
            // instrument up on every press, and the first press can land the
            // instant registration succeeds.
            app.manage(Timings::new());

            // The debt the launcher tile takes on when it hides the window
            // before a capture. Managed BEFORE the veil exists and before the
            // shortcut is bound, for the same reason as the instrument above:
            // `launch::hide_main_window` refuses to hide anything it cannot
            // record, so an unmanaged claim would silently turn the tile back
            // into a capture with Cliche in the frame.
            app.manage(launch::MainWindowClaim::new());

            // THE MAIN WINDOW STOPS FADING, once, here - and once is the point,
            // exactly as it is for the veil below: this asks the compositor to
            // change a property of a window, not to do a piece of work, and
            // asking again on every capture would put a system call inside the
            // path the user is waiting through.
            //
            // WHY IT IS DONE AT ALL: a capture started from the tile hides this
            // window first and photographs the screen 120 ms later. Windows
            // animates a window on its way out, and the user has twice come back
            // with a screenshot containing Cliche itself, half transparent, over
            // their desktop. A window that leaves in one step has no half-erased
            // state to be caught in. `compositor.rs` holds the mechanics, the
            // portability rule that shaped it, and - in plain words - what about
            // all this is NOT measured.
            //
            // The line is PRINTED whatever the answer, success included, because
            // it is the only evidence that exists about a mechanism no test in
            // this repository can reach. The wording of all four outcomes lives
            // in `compositor.rs`, where it is under test.
            let silencing = match app.get_webview_window(ipc::MAIN_WINDOW_LABEL) {
                Some(window) => compositor::silence_transitions(&window),
                None => compositor::Silencing::WindowMissing,
            };
            println!("{}", silencing.terminal_line(ipc::MAIN_WINDOW_LABEL));

            // The clipboard step's own instrument, and a SEPARATE type on
            // purpose: Tauri manages state by type, and these figures must never
            // be aggregated with the pipeline's - the write happens after the
            // user's drag, long outside the 150 ms budget. `clipboard.rs`'s
            // header has the mechanics.
            app.manage(clipboard::Meter::new());

            // The transport is read ONCE, here, and never again: switching
            // between the two candidate routes is a restart with a different
            // environment variable, not a rebuild. A value that is not
            // understood is announced rather than swallowed - measuring
            // transport A while believing you measured B is the one mistake
            // that would invalidate the whole comparison.
            let (transport, transport_warning) =
                veil::Transport::parse(std::env::var(veil::TRANSPORT_ENV).ok().as_deref());
            if let Some(warning) = transport_warning {
                eprintln!("{warning}");
            }
            println!("[cliche] veil: transport {}", transport.describe());
            app.manage(veil::Veil::new(transport));

            // Built HERE, at startup, hidden. Creating a window means creating
            // a WebView2 instance and loading a document: hundreds of
            // milliseconds, once. Doing it inside the shortcut handler would
            // put that cost inside the 150 ms budget, and it is exactly the
            // shortcut this lot exists to refuse.
            if let Err(error) = veil::create(app.handle()) {
                eprintln!("{error}");
            }

            // Deliberately not `return Err(...)` on a failure: a combination the
            // OS refuses must not stop the application. But it must not pass in
            // silence either - Cliche would look perfectly fine and do nothing.
            //
            // Two readers now, and that is the change of 6 September 2026. The
            // terminal gets the line, as it always did. The STATUS is recorded,
            // so `describe_shortcut_status` can hand the launcher what actually
            // happened instead of leaving it to say the registry could not be
            // read - a sentence that was false in every one of the three cases.
            //
            // RECORDED rather than managed, since 7 September 2026: the state
            // was managed on the first instruction of this closure, hundreds of
            // milliseconds ago, and the launcher has most likely already read
            // `Starting` out of it. This call is what replaces that with what
            // the operating system answered.
            //
            // WHICH combination is offered is read from disk first. The same
            // rule applies one level up and it is the reason `settings::read`
            // returns an answer rather than a `Result`: a settings file that is
            // missing, unreadable or incoherent must not stop the application
            // either. It falls back to the registry's own combination and says
            // so, on the line `installed.note` carries.
            let saved = settings::read(app.handle());
            let installed = shortcut::install(app.handle(), &saved);
            if let Some(note) = &installed.note {
                eprintln!("{note}");
            }
            if let Some(line) = installed.status.terminal_line() {
                eprintln!("{line}");
            }
            // `try_state` and not `state`: the latter PANICS when the type was
            // never managed. It IS managed - the first instruction of this
            // closure does it - so the other branch is reachable only if that
            // line is removed, and a printed line answers that far better than
            // bringing the application down over it.
            match app.try_state::<shortcut::CaptureShortcut>() {
                Some(state) => state.record(installed),
                None => eprintln!(
                    "[cliche] shortcut: no capture state is managed, so what the system answered \
                     cannot reach the launcher. The first instruction of `setup` in \
                     src-tauri/src/lib.rs is what manages it, and it has gone."
                ),
            }

            // Measuring without touching the keyboard. `CLICHE_BENCH=20` runs
            // the very same `perform_capture` the shortcut calls; read
            // `veil::spawn_bench` for the three ways it is nevertheless
            // gentler than a real press.
            let (bench_runs, bench_warning) =
                veil::parse_bench(std::env::var(veil::BENCH_ENV).ok().as_deref());
            if let Some(warning) = bench_warning {
                eprintln!("{warning}");
            }
            if let Some(runs) = bench_runs {
                veil::spawn_bench(app.handle(), runs);
            }

            Ok(())
        })
        .run(tauri::generate_context!());

    // No `unwrap`/`expect` on a real execution path: a failed start must say
    // what went wrong and exit non-zero, not print a backtrace.
    if let Err(error) = result {
        eprintln!("[cliche] fatal: could not start the application: {error}");
        std::process::exit(1);
    }
}
