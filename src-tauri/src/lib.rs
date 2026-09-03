//! Cliché - local screenshot utility.
//!
//! The application logic lives in this library rather than in `main.rs` so it
//! can be exercised by `cargo test` without starting an event loop.

mod displays;
mod shortcut;
pub mod timing;

pub use displays::{collect_displays, describe_displays, summarize, DisplayInfo};

use displays::print_displays;
use tauri::Manager;
use timing::Timings;

/// Builds and runs the application. Returns only when the app exits.
pub fn run() {
    let result = tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![displays::describe_displays])
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

            if let Err(error) = shortcut::install(app.handle()) {
                // Deliberately not `return Err(...)`: a combination the OS
                // refuses must not stop the application. But it must not pass
                // in silence either - Cliche would look perfectly fine and do
                // nothing. The message says which shortcut, and why.
                eprintln!("{error}");
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
