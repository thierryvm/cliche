// Hides the extra console window on Windows in release builds. Debug builds
// keep it: that console is where the startup display report is read.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    cliche_lib::run()
}
