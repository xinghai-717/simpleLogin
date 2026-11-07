mod app;
mod theme;
mod pages;

use std::sync::Arc;
use eframe::egui;
use crate::app::MyApp;
use simple_login::smp;

fn main() -> eframe::Result {
    let mut options = eframe::NativeOptions::default();
    let mut fonts = egui::FontDefinitions::default();

    options.renderer = eframe::Renderer::Glow;
    options.viewport = egui::ViewportBuilder::default()
        .with_visible(false);
    fonts.font_data.insert(
        "Noto Sans SC".to_string(),
        Arc::new(egui::FontData::from_static(include_bytes!(".././assets/NotoSansSC-Regular.ttf"))),
    );
    fonts.families
        .get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Noto Sans SC".to_string());


    eframe::run_native(
        "SimpleLogin",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc,fonts))))
    )
}