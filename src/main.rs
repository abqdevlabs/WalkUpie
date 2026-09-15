#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;

use eframe::egui;
use walkupie::settings;

fn main() -> eframe::Result {
    #[cfg(debug_assertions)]
    env_logger::init();

    let settings = settings::load();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("WalkUpie")
            .with_inner_size([176.0, 200.0])
            .with_min_inner_size([176.0, 200.0])
            .with_decorations(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_window_level(egui::WindowLevel::AlwaysOnTop),
        ..Default::default()
    };

    eframe::run_native(
        "WalkUpie",
        options,
        Box::new(move |cc| Ok(Box::new(app::WalkUpie::new(cc, settings)))),
    )
}