// Prevents additional console window on Windows in release; on macOS just runs.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    oxiline_app_lib::run();
}
