#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on windows in release mode!

mod launcher;
use launcher::LauncherApp;

fn main() {
    let options = eframe::NativeOptions {
        resizable: false,
        initial_window_size: Some(egui::vec2(480.0, 240.0)),
        ..Default::default()
    };
    eframe::run_native(
        "AstralEra Launcher",
        options,
        Box::new(|_cc| Box::new(LauncherApp::default())),
    );
}