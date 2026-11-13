mod app;
mod theme;
mod pages;
mod widgets;
mod logger;
mod updater;
mod tray;
mod autostart;

use std::sync::{Arc, Mutex};
use std::collections::VecDeque;
use eframe::egui;
use crate::app::MyApp;
use simple_login::smp;

fn load_icon() -> egui::IconData {
    let icon_bytes = include_bytes!("../assets/app.png");
    let img = image::load_from_memory(icon_bytes).expect("无法加载png文件");
    let img = img.to_rgba8();
    let (width, height) = img.dimensions();
    egui::IconData {
        rgba: img.into_raw(),
        width,
        height,
    }
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_icon(Arc::new(load_icon()))
            .with_inner_size([545.0, 355.0])
            .with_maximize_button(false)
            .with_visible(false),
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    let mut fonts = egui::FontDefinitions::default();
    let log_buffer = Arc::new(Mutex::new(VecDeque::new()));
    let _guard = logger::init_logging(log_buffer.clone());
    fonts.font_data.insert(
        "Noto Sans SC".to_string(),
        Arc::new(egui::FontData::from_static(include_bytes!(".././assets/NotoSansSC-Regular.ttf"))),
    );
    fonts.families
        .get_mut(&egui::FontFamily::Proportional).unwrap()
        .insert(0, "Noto Sans SC".to_string());

    let tray_app = tray::TrayApp::new().expect("无法创建系统托盘");

    eframe::run_native(
        "SimpleLogin",
        options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc, fonts, log_buffer, tray_app))))
    )
}