//! Cliché - local screenshot utility.
//!
//! The application logic lives in this library rather than in `main.rs` so it
//! can be exercised by `cargo test` without starting an event loop.

mod displays;
pub mod timing;

pub use displays::{collect_displays, describe_displays, summarize, DisplayInfo};

use displays::print_displays;

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
