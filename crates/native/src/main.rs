//! Native sshcli terminal binary.
//!
//! Launches a single `eframe` window with a dockable terminal tab wired to a
//! real local PTY shell. This is the seed for the native app; later steps will
//! add SSH sessions and connection management.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

use sshcli_native::NativeApp;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("sshcli native")
            .with_inner_size([1000.0, 700.0])
            .with_min_inner_size([480.0, 320.0]),
        ..Default::default()
    };
    eframe::run_native(
        "sshcli native",
        options,
        Box::new(|cc| Ok(Box::new(NativeApp::new(cc)))),
    )
}
