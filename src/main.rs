//! Tattile: a native macOS app to practice clear, correct English writing.

mod ai;
mod analysis;
mod app;
mod model;
mod prompts;
mod secrets;
mod storage;
mod theme;
mod ui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Tattile")
            .with_inner_size([1360.0, 880.0])
            .with_min_inner_size([1040.0, 660.0]),
        ..Default::default()
    };
    eframe::run_native("Tattile", options, Box::new(|cc| Ok(Box::new(app::TattileApp::new(cc)))))
}
