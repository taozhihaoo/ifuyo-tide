// GUI subsystem for release builds: double-clicking the exe must open
// the app window, not a console. Debug builds (tauri dev) keep the
// console so lifecycle logs stay visible during development.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ifuyo_tide_lib::run()
}
