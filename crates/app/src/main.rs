//! FretStudy — learn guitar, one building block at a time.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // no console window on Windows release builds
#![allow(clippy::needless_range_loop)] // strings are indexed 0..6 throughout

mod app;
mod lesson;
mod recorder;
mod screens;
mod sound;
mod theme;
mod widgets;

#[cfg(test)]
mod snapshot_tests;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("FretStudy")
            .with_inner_size([1280.0, 860.0])
            .with_min_inner_size([960.0, 640.0])
            .with_icon(
                eframe::icon_data::from_png_bytes(include_bytes!("../../../assets/icon-256.png"))
                    .unwrap_or_default(),
            ),
        ..Default::default()
    };
    eframe::run_native(
        "FretStudy",
        options,
        Box::new(|cc| Ok(Box::new(app::App::new(cc)))),
    )
}
