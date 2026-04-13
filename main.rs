mod app;
mod mkv_merge;
mod parser;
mod subtitle_api;
mod ui;
mod utils;

use app::SubMergeApp;
use eframe::egui;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([700.0, 800.0])
            .with_min_inner_size([600.0, 700.0])
            .with_decorations(true)
            .with_transparent(false),
        ..Default::default()
    };

    eframe::run_native(
        "SubMerge",
        options,
        Box::new(|cc| {
            ui::theme::setup_custom_fonts(&cc.egui_ctx);
            ui::theme::setup_style(&cc.egui_ctx);
            Ok(Box::new(SubMergeApp::new(cc)))
        }),
    )
}
