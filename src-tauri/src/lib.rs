//! Cliché - local screenshot utility.
//!
//! The application logic lives in this library rather than in `main.rs` so it
//! can be exercised by `cargo test` without starting an event loop.

pub mod capture;
pub mod clipboard;
mod displays;
pub mod geometry;
pub mod ipc;
mod shortcut;
pub mod timing;
pub mod veil;

pub use displays::{collect_displays, describe_displays, summarize, DisplayInfo};

use displays::print_displays;
use tauri::Manager;
use timing::Timings;

/// Builds and runs the application. Returns only when the app exits.
pub fn run() {
    let result = tauri::Builder::default()
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
            veil::veil_ready,
            veil::veil_decoded,
            veil::veil_painted,
            veil::veil_selected,
            veil::veil_dismissed,
        ])
        .setup(|app| {
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

            if let Err(error) = shortcut::install(app.handle()) {
                // Deliberately not `return Err(...)`: a combination the OS
                // refuses must not stop the application. But it must not pass
                // in silence either - Cliche would look perfectly fine and do
                // nothing. The message says which shortcut, and why.
                eprintln!("{error}");
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
